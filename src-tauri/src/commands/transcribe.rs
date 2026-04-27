//! Transkrypcja PCM → tekst polski przez binarkę whisper.cpp.
//!
//! Pipeline:
//!   1. weź `Pcm` zapisany przez `stop_recording` w `AppState.last_pcm`
//!   2. resample do 16 kHz (whisper.cpp wymaga właśnie tego)
//!   3. zapisz jako tymczasowy WAV 16-bit mono
//!   4. odpal `whisper-cli.exe -m <model> -f <wav> -l pl -otxt -of <stem>`
//!   5. przeczytaj `<stem>.txt` i zwróć jako transkrypt
//!
//! Dlaczego subprocess a nie linkowany `whisper-rs`?  Crate `whisper-rs` został
//! zarchiwizowany w lipcu 2025 i nie buduje się z nowym LLVM/bindgenem.  Odpalanie
//! gotowej binarki z releases whisper.cpp eliminuje cały build-chain cmake +
//! libclang + C++ i działa identycznie na Windows/macOS/Linux.
//!
//! Konfiguracja ścieżek (kolejność): env var > AppSettings w SQLCipher > placeholder.
//!   - `RPSTR_WHISPER_BIN`   — override dla developerów
//!   - `RPSTR_WHISPER_MODEL` — override dla developerów
//!
//! Produkcyjnie ścieżki wstawia kreator z `setup.rs` do `AppSettings.whisper_bin`
//! / `AppSettings.whisper_model` i tu je odczytujemy synchronicznie.  Jeśli obu
//! brak, zwracamy placeholder — UI pokaże tekst zastępczy i zaprosi do
//! konfiguracji.

use crate::audio::{resample_linear, Pcm};
use crate::commands::settings::load_settings;
use crate::error::{AppError, Result};
use crate::state::AppState;
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::State;

const WHISPER_SAMPLE_RATE: u32 = 16_000;

#[derive(Serialize)]
pub struct TranscribeResult {
    pub transcript: String,
}

#[tauri::command]
pub async fn transcribe(visit_id: String, state: State<'_, AppState>) -> Result<TranscribeResult> {
    let pcm = state
        .last_pcm
        .lock()
        .clone()
        .ok_or_else(|| AppError::Other("brak nagrania — wywołaj najpierw stop_recording".into()))?;

    eprintln!(
        "[rpstr/transcribe] visit={} samples={} rate={}Hz duration={}ms",
        visit_id,
        pcm.samples.len(),
        pcm.sample_rate,
        pcm.duration_ms
    );

    let transcript = match (whisper_bin(&state), whisper_model_path(&state)) {
        (Some(bin), Some(model)) => run_whisper_subprocess(&bin, &model, pcm)?,
        _ => placeholder(pcm),
    };
    Ok(TranscribeResult { transcript })
}

// ───────────────────────────── konfiguracja ścieżek ──────────────────────────

fn whisper_bin(state: &AppState) -> Option<PathBuf> {
    let raw = if let Some(p) = std::env::var_os("RPSTR_WHISPER_BIN") {
        Some(PathBuf::from(p))
    } else {
        load_settings(state)
            .ok()
            .and_then(|s| s.whisper_bin)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
    };
    raw.map(prefer_whisper_cli)
}

/// W ZIP-ach whisper.cpp v1.7+ obok `whisper-cli.exe` siedzi `main.exe` —
/// w nowych wersjach to tylko stub wypisujący "deprecated, use whisper-cli"
/// i kończący się kodem 1.  Jeśli kreator zapisał wskaźnik na `main(.exe)`
/// (bug w starszych buildach naszego rpstra), automatycznie podmieniamy na
/// `whisper-cli(.exe)` z tego samego katalogu — bez konieczności re-runu setupu.
fn prefer_whisper_cli(p: PathBuf) -> PathBuf {
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if stem != "main" {
        return p;
    }
    let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
    let new_name = if ext.is_empty() {
        "whisper-cli".to_string()
    } else {
        format!("whisper-cli.{ext}")
    };
    let candidate = p.with_file_name(new_name);
    if candidate.exists() {
        candidate
    } else {
        p
    }
}

fn whisper_model_path(state: &AppState) -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("RPSTR_WHISPER_MODEL") {
        return Some(PathBuf::from(p));
    }
    load_settings(state)
        .ok()
        .and_then(|s| s.whisper_model)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

fn placeholder(pcm: Pcm) -> String {
    format!(
        "[transkrypt niedostępny — uruchom kreator konfiguracji (/setup); \
         nagranie: {} próbek @ {} Hz]",
        pcm.samples.len(),
        pcm.sample_rate,
    )
}

// ───────────────────────────── subprocess whisper.cpp ────────────────────────

fn run_whisper_subprocess(bin: &Path, model: &Path, pcm: Pcm) -> Result<String> {
    // Pre-flight checks before spawning the subprocess.
    if !bin.exists() {
        return Err(AppError::Other(format!(
            "whisper-cli nie znaleziony: {}. Uruchom kreator konfiguracji.",
            bin.display()
        )));
    }
    let model_size = std::fs::metadata(model)
        .map(|m| m.len())
        .unwrap_or(0);
    if model_size < 1_000_000 {
        return Err(AppError::Other(format!(
            "Model Whisper niekompletny lub brakujący ({} bajtów): {}. \
             Wejdź w Ustawienia → Model Whisper i pobierz ponownie.",
            model_size,
            model.display()
        )));
    }

    let samples_16k = resample_linear(&pcm.samples, pcm.sample_rate, WHISPER_SAMPLE_RATE);
    if samples_16k.is_empty() {
        return Err(AppError::Other("pusty bufor audio".into()));
    }

    let stem = unique_stem();
    let tmp_dir = std::env::temp_dir();
    let wav_path = tmp_dir.join(format!("{stem}.wav"));
    let out_stem = tmp_dir.join(&stem);
    let txt_path = tmp_dir.join(format!("{stem}.txt"));

    write_wav_16k_mono(&wav_path, &samples_16k)
        .map_err(|e| AppError::Other(format!("zapis WAV: {e}")))?;

    eprintln!(
        "[rpstr/transcribe] bin={:?} model={:?} ({} MB) wav={:?} samples={}",
        bin, model, model_size / 1_000_000, wav_path, samples_16k.len()
    );

    let output = std::process::Command::new(bin)
        .arg("-m")
        .arg(model)
        .arg("-f")
        .arg(&wav_path)
        .arg("-l")
        .arg("pl")
        .arg("-otxt")
        .arg("-of")
        .arg(&out_stem)
        .output()
        .map_err(|e| AppError::Other(format!("spawn {}: {e}", bin.display())))?;

    let _ = std::fs::remove_file(&wav_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let _ = std::fs::remove_file(&txt_path);
        // whisper.cpp v1.8+ prints errors to stdout; include both streams.
        let msg = [stdout.trim(), stderr.trim()]
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ");
        return Err(AppError::Other(format!(
            "whisper.cpp zakończył się błędem ({}): {}",
            output.status,
            if msg.is_empty() { "brak komunikatu — sprawdź logi" } else { &msg }
        )));
    }

    let transcript = std::fs::read_to_string(&txt_path)
        .map_err(|e| AppError::Other(format!("odczyt transkryptu {}: {e}", txt_path.display())))?;
    let _ = std::fs::remove_file(&txt_path);

    Ok(transcript.trim().to_string())
}

fn unique_stem() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("rpstr_{pid}_{nanos}")
}

// ───────────────────────────── zapis WAV 16 kHz mono ─────────────────────────

/// Minimalny writer WAV RIFF/PCM 16-bit mono 16 kHz — dokładnie to co łyknie
/// whisper.cpp bez dodatkowego resampla po swojej stronie.
fn write_wav_16k_mono(path: &Path, samples: &[f32]) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;

    let num_samples = samples.len() as u32;
    let byte_size = num_samples.saturating_mul(2);
    let chunk_size = 36u32.saturating_add(byte_size);

    // RIFF
    file.write_all(b"RIFF")?;
    file.write_all(&chunk_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt  — PCM, mono, 16 kHz, 16-bit
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&WHISPER_SAMPLE_RATE.to_le_bytes())?;
    file.write_all(&(WHISPER_SAMPLE_RATE * 2).to_le_bytes())?;
    file.write_all(&2u16.to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?;

    // data
    file.write_all(b"data")?;
    file.write_all(&byte_size.to_le_bytes())?;

    let mut buf = Vec::with_capacity(samples.len() * 2);
    for s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let i = (clamped * i16::MAX as f32) as i16;
        buf.extend_from_slice(&i.to_le_bytes());
    }
    file.write_all(&buf)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_has_correct_size_fields() {
        let tmp = std::env::temp_dir().join("rpstr_test.wav");
        let samples = vec![0.0f32; 100];
        write_wav_16k_mono(&tmp, &samples).unwrap();
        let bytes = std::fs::read(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);

        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[12..16], b"fmt ");
        assert_eq!(&bytes[36..40], b"data");
        // 100 samples * 2 bytes = 200 bytes data
        let data_size = u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]);
        assert_eq!(data_size, 200);
    }
}
