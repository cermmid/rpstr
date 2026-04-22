use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct SystemProbe {
    #[serde(rename = "freeDiskGb")]
    pub free_disk_gb: f64,
    #[serde(rename = "totalRamGb")]
    pub total_ram_gb: u32,
    pub gpu: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelProfile {
    Lite,
    Standard,
    Pro,
    CloudOnly,
}

#[tauri::command]
pub async fn probe_system() -> Result<SystemProbe> {
    // TODO(D5): realny probe (sysinfo crate + wmic / sysctl).
    Ok(SystemProbe {
        free_disk_gb: 42.0,
        total_ram_gb: 16,
        gpu: "cpu-only".into(),
    })
}

#[tauri::command]
pub async fn download_profile(profile: String, models_dir: String) -> Result<()> {
    // TODO(D5): realne pobieranie z CDN, weryfikacja SHA-256, HTTP Range resume.
    tracing_stub(&format!(
        "download_profile: profile={profile}, dir={}",
        if models_dir.is_empty() {
            "<default>"
        } else {
            &models_dir
        }
    ));
    Ok(())
}

fn tracing_stub(msg: &str) {
    eprintln!("[rpstr] {msg}");
}
