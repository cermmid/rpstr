//! Transkrypcja PCM → tekst polski przez whisper.cpp.
//!
//! Pipeline:
//!   1. weź `Pcm` zapisany przez `stop_recording` w `AppState.last_pcm`
//!   2. resample do 16 kHz (whisper wymaga właśnie tego)
//!   3. załaduj model z `models_dir/whisper-{profile}.bin` (wybór w settings)
//!   4. uruchom `WhisperContext::full()` z `language = Some("pl")`
//!   5. skleć segmenty w jedno string
//!
//! Tryb bez feature `stt-whisper`: zwracamy placeholder, żeby UI dało się
//! przeklikać i testować pozostałe etapy (nagrywanie, LLM, zapis).

use crate::audio::{resample_linear, Pcm};
use crate::error::{AppError, Result};
use crate::state::AppState;
use serde::Serialize;
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

    let transcript = run_whisper(pcm)?;
    Ok(TranscribeResult { transcript })
}

// ─────────────────────────────── z whisper-rs ────────────────────────────────

#[cfg(feature = "stt-whisper")]
fn run_whisper(pcm: Pcm) -> Result<String> {
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    let samples_16k = resample_linear(&pcm.samples, pcm.sample_rate, WHISPER_SAMPLE_RATE);

    let model_path = whisper_model_path()?;
    let ctx = WhisperContext::new_with_params(
        &model_path.to_string_lossy(),
        WhisperContextParameters::default(),
    )
    .map_err(|e| AppError::Other(format!("whisper load: {e}")))?;

    let mut state = ctx
        .create_state()
        .map_err(|e| AppError::Other(format!("whisper state: {e}")))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("pl"));
    params.set_translate(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_special(false);
    params.set_print_timestamps(false);
    params.set_n_threads(num_cpus_hint() as i32);

    state
        .full(params, &samples_16k)
        .map_err(|e| AppError::Other(format!("whisper run: {e}")))?;

    let num = state
        .full_n_segments()
        .map_err(|e| AppError::Other(format!("whisper segments count: {e}")))?;

    let mut text = String::new();
    for i in 0..num {
        let seg = state
            .full_get_segment_text(i)
            .map_err(|e| AppError::Other(format!("whisper segment {i}: {e}")))?;
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(seg.trim());
    }
    Ok(text)
}

#[cfg(feature = "stt-whisper")]
fn whisper_model_path() -> Result<std::path::PathBuf> {
    // TODO(D5): wybór konkretnego pliku na podstawie profilu z settings
    // (whisper-small.bin / -medium.bin / -large-v3-turbo.bin).  Na razie bierzemy
    // ścieżkę z ENV albo domyślną w %APPDATA%/rpstr/models.
    if let Ok(p) = std::env::var("RPSTR_WHISPER_MODEL") {
        return Ok(p.into());
    }
    let home = dirs_home().unwrap_or_else(|| std::path::PathBuf::from("."));
    Ok(home.join("rpstr/models/whisper.bin"))
}

#[cfg(feature = "stt-whisper")]
fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from)
}

#[cfg(feature = "stt-whisper")]
fn num_cpus_hint() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(8)
}

// ─────────────────────────────── Stub bez feature ────────────────────────────

#[cfg(not(feature = "stt-whisper"))]
fn run_whisper(pcm: Pcm) -> Result<String> {
    let _ = resample_linear(&pcm.samples, pcm.sample_rate, WHISPER_SAMPLE_RATE);
    Ok(format!(
        "[transkrypt niedostępny — feature `stt-whisper` nie jest włączony; \
         nagranie: {} próbek @ {} Hz]",
        pcm.samples.len(),
        pcm.sample_rate,
    ))
}
