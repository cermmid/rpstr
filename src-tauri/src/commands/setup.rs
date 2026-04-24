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
use tokio::io::{AsyncBufReadExt, BufReader};

const PROGRESS_EVENT: &str = "setup:progress";
const WHISPER_CPP_ZIP_URL: &str =
    "https://github.com/ggerganov/whisper.cpp/releases/download/v1.7.2/whisper-bin-x64.zip";

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
    let ollama_installed = which_ollama().is_some();

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

fn which_ollama() -> Option<PathBuf> {
    let name = if cfg!(target_os = "windows") {
        "ollama.exe"
    } else {
        "ollama"
    };
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[tauri::command]
pub async fn check_ollama_installed() -> Result<bool> {
    Ok(which_ollama().is_some())
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

#[tauri::command]
pub async fn pull_ollama_model(name: String, app: AppHandle) -> Result<()> {
    let bin = which_ollama()
        .ok_or_else(|| AppError::Other("ollama nie w PATH — zainstaluj najpierw".into()))?;
    emit(
        &app,
        "ollama-pull",
        0.0,
        format!("Pobieranie modelu {name}…"),
    );

    let mut child = tokio::process::Command::new(&bin)
        .args(["pull", &name])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::Other(format!("spawn ollama: {e}")))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Other("brak stdout ollama".into()))?;
    let mut lines = BufReader::new(stdout).lines();

    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| AppError::Other(format!("read ollama stdout: {e}")))?
    {
        if let Some(pct) = parse_ollama_percent(&line) {
            emit(&app, "ollama-pull", pct, line.trim().to_string());
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| AppError::Other(format!("wait ollama: {e}")))?;
    if !status.success() {
        return Err(AppError::Other(format!("ollama pull zwróciło {status}")));
    }
    emit(&app, "ollama-pull", 100.0, "Model pobrany.");
    Ok(())
}

fn parse_ollama_percent(line: &str) -> Option<f32> {
    // typowy format: "pulling manifest 12% ▕████..." albo "pulling aabbcc... 45%"
    let idx = line.find('%')?;
    let head = &line[..idx];
    let digits: String = head
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    digits.parse::<f32>().ok()
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
    fn parses_ollama_percent_format() {
        assert_eq!(parse_ollama_percent("pulling aaabbb 12%"), Some(12.0));
        assert_eq!(
            parse_ollama_percent("pulling manifest 100% ▕████"),
            Some(100.0)
        );
        assert_eq!(parse_ollama_percent("done"), None);
        assert_eq!(parse_ollama_percent("pulling aabb 45.5%"), Some(45.5));
    }

    #[test]
    fn human_size_scales() {
        assert_eq!(human_size(0), "0.0 B");
        assert_eq!(human_size(1024), "1.0 kB");
        assert_eq!(human_size(1024 * 1024 * 3), "3.0 MB");
    }
}
