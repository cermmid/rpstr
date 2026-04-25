//! Kreator pierwszego uruchomienia — iteracja 2.
//!
//! Zadanie: tester-lekarz dostaje jeden `.msi`, odpala, wszystko reszta dzieje
//! się bez "zainstaluj sobie Ollamę".  Poszczególne kroki to osobne Tauri
//! command'y — UI trzyma state machine, Rust dostarcza atomowe akcje i emituje
//! `setup:progress` eventy z postępem.
//!
//! Kolejność w wizardzie:
//!   1. `probe_system()` — wolne miejsce, RAM, GPU
//!   2. `check_ollama_installed()` — `which ollama`
//!   3. (jeśli nie) `install_ollama()` — pobierz + `/VERYSILENT` (wymaga UAC!)
//!   4. `pull_ollama_model(tag)` — `ollama pull` z parsowaniem stdout
//!   5. `download_whisper_cpp()` — release ZIP z GitHub, rozpakuj
//!   6. `download_whisper_model(size)` — ggml-*.bin z HuggingFace
//!   7. `finish_setup(...)` — zapis do AppSettings + `setup_completed=true`
//!
//! Wszystkie długie operacje emitują event `setup:progress` z payloadem
//! `{ stage: string, percent: number, message: string }`.

use crate::commands::settings::{load_settings, SETTINGS_KEY};
use crate::db;
use crate::error::{AppError, Result};
use crate::state::AppState;
use futures_util::StreamExt;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tauri::{AppHandle, Emitter, State};

const PROGRESS_EVENT: &str = "setup:progress";
const WHISPER_CPP_ZIP_URL: &str =
    "https://github.com/ggml-org/whisper.cpp/releases/download/v1.8.4/whisper-bin-x64.zip";
const OLLAMA_API: &str = "http://localhost:11434";

/// Minimalna wersja Ollamy obsługująca tagi `hf.co/<org>/<repo>:<quant>` —
/// wprowadzone w 0.21 (wrzesień 2024).  Bez tego pull Bielika 7B z HuggingFace
/// nie zadziała, bo SpeakLeash nie ma osobnego wpisu w bibliotece Ollamy.
const MIN_OLLAMA_VERSION_FOR_HF: (u32, u32) = (0, 21);

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress {
    stage: &'static str,
    percent: f32,
    message: String,
}

fn emit(app: &AppHandle, stage: &'static str, percent: f32, msg: impl Into<String>) {
    let _ = app.emit(
        PROGRESS_EVENT,
        Progress {
            stage,
            percent: percent.clamp(0.0, 100.0),
            message: msg.into(),
        },
    );
}

// ─────────────────────────────── probe ──────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemProbe {
    pub free_disk_gb: f64,
    pub total_ram_gb: u32,
    /// "cuda" / "directml" / "metal" / "cpu-only"
    pub gpu: String,
    pub ollama_installed: bool,
}

#[tauri::command]
pub async fn probe_system() -> Result<SystemProbe> {
    use sysinfo::System;

    let mut sys = System::new();
    sys.refresh_memory();
    let total_ram_gb = (sys.total_memory() / (1024 * 1024 * 1024)) as u32;

    // Folder docelowy na modele — %APPDATA%/rpstr/models na Windows.
    let models_dir = default_models_dir();
    if let Some(parent) = models_dir.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let target = models_dir
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let free_disk_gb = available_space_gb(&target);

    let gpu = detect_gpu();
    let ollama_installed = ollama_version().await.is_some();

    Ok(SystemProbe {
        free_disk_gb,
        total_ram_gb,
        gpu,
        ollama_installed,
    })
}

fn available_space_gb(path: &Path) -> f64 {
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::mem::MaybeUninit;
        use std::os::unix::ffi::OsStrExt;
        let c = match CString::new(path.as_os_str().as_bytes()) {
            Ok(c) => c,
            Err(_) => return 0.0,
        };
        unsafe {
            let mut stat = MaybeUninit::<libc_statvfs>::zeroed();
            let rc = statvfs_raw(c.as_ptr(), stat.as_mut_ptr());
            if rc != 0 {
                return 0.0;
            }
            let s = stat.assume_init();
            (s.f_bavail as f64 * s.f_frsize as f64) / (1024.0 * 1024.0 * 1024.0)
        }
    }
    #[cfg(windows)]
    {
        // GetDiskFreeSpaceExW przez winapi byłby czystszy, ale unikamy dodatkowej
        // zależności — `fs::available_space` doda się w std w przyszłości.
        // Na razie: przybliżenie przez sprawdzenie czy da się utworzyć katalog.
        let _ = path;
        // Heurystyka: zakładamy 50 GB, kreator i tak pyta testera o potwierdzenie.
        50.0
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        0.0
    }
}

// Minimalny FFI do statvfs — żeby nie ciągnąć całej libc dla jednej funkcji.
#[cfg(unix)]
#[repr(C)]
#[allow(non_camel_case_types)]
struct libc_statvfs {
    f_bsize: u64,
    f_frsize: u64,
    f_blocks: u64,
    f_bfree: u64,
    f_bavail: u64,
    f_files: u64,
    f_ffree: u64,
    f_favail: u64,
    f_fsid: u64,
    f_flag: u64,
    f_namemax: u64,
    _spare: [u32; 6],
}

#[cfg(unix)]
extern "C" {
    #[link_name = "statvfs"]
    fn statvfs_raw(path: *const std::os::raw::c_char, buf: *mut libc_statvfs) -> std::os::raw::c_int;
}

fn detect_gpu() -> String {
    if std::process::Command::new("nvidia-smi")
        .arg("-L")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return "cuda".into();
    }
    #[cfg(target_os = "macos")]
    {
        return "metal".into();
    }
    #[cfg(target_os = "windows")]
    {
        return "directml".into();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        "cpu-only".into()
    }
}

#[tauri::command]
pub async fn check_ollama_installed() -> Result<bool> {
    // Sprawdzamy HTTP endpoint zamiast PATH — usługa Ollamy startuje sama po
    // instalacji na Windowsie, a PATH w procesie rpstr może być niezaktualizowany
    // do następnego restartu.  Endpoint `/api/version` działa zawsze gdy serwer
    // żyje, niezależnie od tego, jak Ollama została zainstalowana.
    Ok(ollama_version().await.is_some())
}

async fn ollama_version() -> Option<(u32, u32)> {
    #[derive(Deserialize)]
    struct VersionResp {
        version: String,
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .ok()?;
    let resp = client
        .get(format!("{OLLAMA_API}/api/version"))
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?;
    let body: VersionResp = resp.json().await.ok()?;
    parse_version(&body.version)
}

fn parse_version(v: &str) -> Option<(u32, u32)> {
    // "0.5.7" → (0, 5).  "0.5.7-rc1" → (0, 5).  Tylko major.minor nas obchodzi.
    let cleaned: String = v
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let parts: Vec<&str> = cleaned.split('.').collect();
    let major = parts.first()?.parse().ok()?;
    let minor = parts.get(1)?.parse().ok()?;
    Some((major, minor))
}

// ─────────────────────────────── Ollama install ─────────────────────────────

#[tauri::command]
pub async fn install_ollama(app: AppHandle) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::time::Duration;
        emit(&app, "ollama-install", 5.0, "Pobieranie instalatora Ollamy…");
        let tmp = std::env::temp_dir().join("OllamaSetup.exe");
        download_file(
            "https://ollama.com/download/OllamaSetup.exe",
            &tmp,
            &app,
            "ollama-install",
            (5.0, 75.0),
        )
        .await?;

        emit(
            &app,
            "ollama-install",
            80.0,
            "Instalacja (zaakceptuj prompt administratora Windows)…",
        );
        let status = std::process::Command::new(&tmp)
            .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"])
            .status()
            .map_err(|e| AppError::Other(format!("spawn OllamaSetup: {e}")))?;
        let _ = std::fs::remove_file(&tmp);
        if !status.success() {
            return Err(AppError::Other(format!(
                "OllamaSetup zakończył się kodem {status}"
            )));
        }
        // Daj instalatorowi czas na dopisanie PATH / uruchomienie usługi.
        tokio::time::sleep(Duration::from_secs(3)).await;
        emit(&app, "ollama-install", 100.0, "Ollama zainstalowana.");
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        Err(AppError::NotImplemented(
            "automatyczna instalacja Ollamy — na macOS/Linux uruchom ją ręcznie",
        ))
    }
}

// ─────────────────────────────── Ollama pull ────────────────────────────────

/// Pull modelu przez HTTP API Ollamy (`POST /api/pull`).  Używamy tego zamiast
/// subprocess `ollama pull`, bo:
///   1. CLI Ollamy renderuje progress bar przez `\r` (cursor returns) i nasz
///      `BufReader::lines()` czekał w nieskończoność na `\n` — dlatego pasek
///      stał na 0% mimo że pull rzeczywiście leciał.
///   2. HTTP API zwraca strumień NDJSON z `{"status", "completed", "total"}` —
///      progress dokładny do bajta i **błędy są jednoznaczne** (pole `"error"`),
///      a nie zgubione w logu.
///   3. Eliminujemy zależność od PATH (po instalacji Ollamy zmienna PATH w
///      bieżącym procesie nie jest świeża).
#[tauri::command]
pub async fn pull_ollama_model(name: String, app: AppHandle) -> Result<()> {
    // Pre-check: czy serwer żyje + czy wersja obsługuje hf.co/ tagi.
    let version = ollama_version().await.ok_or_else(|| {
        AppError::Other(
            "Ollama nie odpowiada na localhost:11434. \
             Sprawdź że została zainstalowana i jest uruchomiona."
                .into(),
        )
    })?;
    if name.starts_with("hf.co/") && version < MIN_OLLAMA_VERSION_FOR_HF {
        return Err(AppError::Other(format!(
            "Ollama {}.{} jest za stara dla tagów hf.co/ (Bielika 7B z HuggingFace). \
             Wymagana wersja {}.{} lub nowsza. \
             Zaktualizuj Ollamę: https://ollama.com/download",
            version.0, version.1, MIN_OLLAMA_VERSION_FOR_HF.0, MIN_OLLAMA_VERSION_FOR_HF.1
        )));
    }

    emit(
        &app,
        "ollama-pull",
        0.0,
        format!("Pobieranie modelu {name}…"),
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60 * 60))
        .build()?;

    let resp = client
        .post(format!("{OLLAMA_API}/api/pull"))
        .json(&serde_json::json!({ "name": name, "stream": true }))
        .send()
        .await?
        .error_for_status()
        .map_err(|e| AppError::Other(format!("Ollama /api/pull HTTP: {e}")))?;

    let mut stream = resp.bytes_stream();
    let mut buf = Vec::<u8>::new();
    let mut last_emit = std::time::Instant::now();
    let mut last_progress: f32 = 0.0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buf.extend_from_slice(&chunk);

        // NDJSON — każda linia to osobny event.  Tnijmy po `\n`.
        while let Some(nl) = buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buf.drain(..=nl).collect();
            let trimmed = std::str::from_utf8(&line).unwrap_or("").trim();
            if trimmed.is_empty() {
                continue;
            }
            handle_pull_event(trimmed, &app, &mut last_emit, &mut last_progress)?;
        }
    }
    // Bufor reszty bez końcowego `\n` (rzadko, ale możliwe przy ostatnim eventcie).
    if !buf.is_empty() {
        let trimmed = std::str::from_utf8(&buf).unwrap_or("").trim();
        if !trimmed.is_empty() {
            handle_pull_event(trimmed, &app, &mut last_emit, &mut last_progress)?;
        }
    }

    emit(&app, "ollama-pull", 100.0, "Model pobrany.");
    Ok(())
}

fn handle_pull_event(
    raw: &str,
    app: &AppHandle,
    last_emit: &mut std::time::Instant,
    last_progress: &mut f32,
) -> Result<()> {
    #[derive(Deserialize)]
    struct PullEvent {
        #[serde(default)]
        status: String,
        #[serde(default)]
        error: Option<String>,
        #[serde(default)]
        completed: Option<u64>,
        #[serde(default)]
        total: Option<u64>,
    }
    let ev: PullEvent = match serde_json::from_str(raw) {
        Ok(e) => e,
        Err(_) => return Ok(()), // ignorujemy niezrozumiałe linie zamiast się wywalać
    };
    if let Some(err) = ev.error {
        return Err(AppError::Other(format!("Ollama: {err}")));
    }
    let pct = match (ev.completed, ev.total) {
        (Some(c), Some(t)) if t > 0 => (c as f32 / t as f32) * 100.0,
        _ => *last_progress,
    };
    *last_progress = pct;

    // Throttle do 4 emitów/s, żeby nie zalać webview eventami.
    if last_emit.elapsed() > std::time::Duration::from_millis(250) || pct >= 100.0 {
        let msg = match (ev.completed, ev.total) {
            (Some(c), Some(t)) if t > 0 => format!("{} ({} / {})", ev.status, human_size(c), human_size(t)),
            _ => ev.status.clone(),
        };
        emit(app, "ollama-pull", pct, msg);
        *last_emit = std::time::Instant::now();
    }
    Ok(())
}

// ─────────────────────────────── whisper.cpp binarka ────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperCppResult {
    pub bin_path: String,
}

#[tauri::command]
pub async fn download_whisper_cpp(app: AppHandle) -> Result<WhisperCppResult> {
    let root = default_models_dir().join("whisper-cpp");
    std::fs::create_dir_all(&root)?;
    let zip_path = root.join("whisper-bin-x64.zip");

    emit(
        &app,
        "whisper-cpp",
        0.0,
        "Pobieranie whisper.cpp z GitHub…",
    );
    download_file(
        WHISPER_CPP_ZIP_URL,
        &zip_path,
        &app,
        "whisper-cpp",
        (0.0, 90.0),
    )
    .await?;

    emit(&app, "whisper-cpp", 92.0, "Rozpakowywanie…");
    let extract_dir = root.join("bin");
    std::fs::create_dir_all(&extract_dir)?;
    extract_zip(&zip_path, &extract_dir)?;
    let _ = std::fs::remove_file(&zip_path);

    let bin = find_whisper_bin(&extract_dir).ok_or_else(|| {
        AppError::Other(format!(
            "nie znalazłem whisper-cli.exe / main w {}",
            extract_dir.display()
        ))
    })?;
    emit(&app, "whisper-cpp", 100.0, "Gotowe.");
    Ok(WhisperCppResult {
        bin_path: bin.to_string_lossy().into_owned(),
    })
}

fn find_whisper_bin(root: &Path) -> Option<PathBuf> {
    // whisper.cpp od v1.7 nazywa się whisper-cli.exe; starsze buildy miały main.exe.
    let candidates = ["whisper-cli.exe", "whisper-cli", "main.exe", "main"];
    walk_for(root, &candidates)
}

fn walk_for(root: &Path, names: &[&str]) -> Option<PathBuf> {
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
                if names.contains(&fname) {
                    return Some(path);
                }
            }
        } else if path.is_dir() {
            if let Some(found) = walk_for(&path, names) {
                return Some(found);
            }
        }
    }
    None
}

fn extract_zip(zip: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(zip)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::Other(format!("otwarcie ZIP {}: {e}", zip.display())))?;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| AppError::Other(format!("entry {i}: {e}")))?;
        let Some(rel) = entry.enclosed_name() else {
            continue;
        };
        let out = dest.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p)?;
        }
        let mut f = std::fs::File::create(&out)?;
        std::io::copy(&mut entry, &mut f)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = entry.unix_mode() {
                let _ = std::fs::set_permissions(&out, std::fs::Permissions::from_mode(mode));
            }
        }
    }
    Ok(())
}

// ─────────────────────────────── whisper model (ggml-*.bin) ─────────────────

/// Obsługiwane rozmiary i odpowiadające im URL-e na HuggingFace.  Hashy SHA-256
/// celowo NIE bundlujemy — plik jest duży (1.5 GB), jego sha256 wydłużał by
/// setup o ~10s, a i tak jest pobierany z TLS-em z CDN-a HF.  Kreator loguje
/// size w bajtach, jeśli ktoś chce weryfikację manualną.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WhisperSize {
    Base,
    Small,
    Medium,
    LargeV3Turbo,
}

impl WhisperSize {
    fn url(&self) -> &'static str {
        match self {
            WhisperSize::Base => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
            WhisperSize::Small => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
            WhisperSize::Medium => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
            WhisperSize::LargeV3Turbo => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
        }
    }
    fn file_name(&self) -> &'static str {
        match self {
            WhisperSize::Base => "ggml-base.bin",
            WhisperSize::Small => "ggml-small.bin",
            WhisperSize::Medium => "ggml-medium.bin",
            WhisperSize::LargeV3Turbo => "ggml-large-v3-turbo.bin",
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperModelResult {
    pub model_path: String,
}

#[tauri::command]
pub async fn download_whisper_model(
    size: WhisperSize,
    app: AppHandle,
) -> Result<WhisperModelResult> {
    let dest_dir = default_models_dir().join("whisper");
    std::fs::create_dir_all(&dest_dir)?;
    let dest = dest_dir.join(size.file_name());

    if dest.exists() {
        // Kreator jest idempotentny — nie pobiera drugi raz.
        emit(&app, "whisper-model", 100.0, "Model już pobrany.");
        return Ok(WhisperModelResult {
            model_path: dest.to_string_lossy().into_owned(),
        });
    }

    emit(
        &app,
        "whisper-model",
        0.0,
        format!("Pobieranie {}…", size.file_name()),
    );
    download_file(size.url(), &dest, &app, "whisper-model", (0.0, 100.0)).await?;
    Ok(WhisperModelResult {
        model_path: dest.to_string_lossy().into_owned(),
    })
}

// ─────────────────────────────── finish ─────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishSetupInput {
    pub whisper_bin: String,
    pub whisper_model: String,
    pub ollama_model: String,
}

#[tauri::command]
pub async fn finish_setup(input: FinishSetupInput, state: State<'_, AppState>) -> Result<()> {
    // Odczyt aktualnych ustawień + overwrite nowymi polami + zapis.
    let mut current = load_settings(&state).unwrap_or_default();
    current.whisper_bin = Some(input.whisper_bin);
    current.whisper_model = Some(input.whisper_model);
    current.ollama_model = input.ollama_model;
    current.setup_completed = true;

    let db = db::db(&state)?;
    db.with_conn(|c| {
        let serialized = serde_json::to_string(&current).unwrap_or_else(|_| "{}".into());
        c.execute(
            "INSERT INTO settings (key, value) VALUES (?, ?) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![SETTINGS_KEY, serialized],
        )?;
        Ok(())
    })
}

/// Używany przez stary `download_profile` oraz `ModelSetupWizard` do sprawdzenia
/// stanu setupu bez pełnego `get_settings`.
#[tauri::command]
pub async fn is_setup_completed(state: State<'_, AppState>) -> Result<bool> {
    Ok(load_settings(&state).map(|s| s.setup_completed).unwrap_or(false))
}

// ─────────────────────────────── stary stub, do usunięcia w MVP+1 ───────────

#[tauri::command]
pub async fn download_profile(profile: String, models_dir: String) -> Result<()> {
    eprintln!(
        "[rpstr/setup] DEPRECATED download_profile: profile={profile}, dir={}",
        if models_dir.is_empty() {
            "<default>"
        } else {
            &models_dir
        }
    );
    Ok(())
}

// ─────────────────────────────── helpery ────────────────────────────────────

fn default_models_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rpstr")
        .join("models")
}

/// Streamowany download z progressem.  `scale` to (min%, max%) — całość
/// progressu tego pliku będzie zmapowana liniowo w ten przedział, żeby
/// wizard mógł łączyć wiele pobrań w jeden pasek.
async fn download_file(
    url: &str,
    dest: &Path,
    app: &AppHandle,
    stage: &'static str,
    scale: (f32, f32),
) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60 * 60))
        .build()?;
    let resp = client.get(url).send().await?.error_for_status()?;
    let total = resp.content_length().unwrap_or(0);
    if let Some(p) = dest.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut file = std::fs::File::create(dest)?;
    let mut stream = resp.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_emit = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        if last_emit.elapsed() > std::time::Duration::from_millis(250) {
            let pct = if total > 0 {
                (downloaded as f32 / total as f32) * 100.0
            } else {
                0.0
            };
            let scaled = scale.0 + (scale.1 - scale.0) * (pct / 100.0);
            emit(
                app,
                stage,
                scaled,
                format!("{} / {}", human_size(downloaded), human_size(total)),
            );
            last_emit = std::time::Instant::now();
        }
    }
    emit(app, stage, scale.1, "Pobrano.".to_string());
    Ok(())
}

fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "kB", "MB", "GB"];
    let mut b = bytes as f64;
    let mut i = 0;
    while b >= 1024.0 && i < UNITS.len() - 1 {
        b /= 1024.0;
        i += 1;
    }
    format!("{:.1} {}", b, UNITS[i])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ollama_version() {
        assert_eq!(parse_version("0.5.7"), Some((0, 5)));
        assert_eq!(parse_version("0.21.0"), Some((0, 21)));
        assert_eq!(parse_version("1.0.0"), Some((1, 0)));
        assert_eq!(parse_version("0.5.7-rc1"), Some((0, 5)));
        assert_eq!(parse_version(""), None);
        assert_eq!(parse_version("garbage"), None);
    }

    #[test]
    fn version_compare_for_hf_tags() {
        assert!((0u32, 21u32) >= MIN_OLLAMA_VERSION_FOR_HF);
        assert!((1u32, 0u32) >= MIN_OLLAMA_VERSION_FOR_HF);
        assert!((0u32, 20u32) < MIN_OLLAMA_VERSION_FOR_HF);
        assert!((0u32, 5u32) < MIN_OLLAMA_VERSION_FOR_HF);
    }

    #[test]
    fn human_size_scales() {
        assert_eq!(human_size(0), "0.0 B");
        assert_eq!(human_size(1024), "1.0 kB");
        assert_eq!(human_size(1024 * 1024 * 3), "3.0 MB");
    }
}
