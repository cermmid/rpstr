use crate::error::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub backend: String,
    pub profile: String,
    pub models_dir: String,
    pub audio_retention_days: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monthly_budget_pln: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_mode_enabled: Option<bool>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            backend: "local-ollama".into(),
            profile: "standard".into(),
            models_dir: "".into(),
            audio_retention_days: 0,
            claude_model: Some("claude-haiku-4-5-20251001".into()),
            monthly_budget_pln: Some(50.0),
            batch_mode_enabled: Some(false),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostStats {
    pub visits: u32,
    pub tokens_input: u64,
    pub tokens_output: u64,
    pub total_pln: f64,
}

#[tauri::command]
pub async fn get_settings() -> Result<AppSettings> {
    // TODO(D6-7): odczyt z SQLCipher. Stub: defaults.
    Ok(AppSettings::default())
}

#[tauri::command]
pub async fn update_settings(patch: Value) -> Result<AppSettings> {
    // TODO(D6-7): merge + UPSERT. Na razie zwracamy defaults + patch (dev).
    eprintln!("[rpstr] update_settings: {patch}");
    let mut s = AppSettings::default();
    if let Some(b) = patch.get("backend").and_then(|v| v.as_str()) {
        s.backend = b.into();
    }
    if let Some(p) = patch.get("profile").and_then(|v| v.as_str()) {
        s.profile = p.into();
    }
    if let Some(d) = patch.get("modelsDir").and_then(|v| v.as_str()) {
        s.models_dir = d.into();
    }
    if let Some(r) = patch.get("audioRetentionDays").and_then(|v| v.as_u64()) {
        s.audio_retention_days = r as u32;
    }
    if let Some(m) = patch.get("claudeModel").and_then(|v| v.as_str()) {
        s.claude_model = Some(m.into());
    }
    if let Some(b) = patch.get("monthlyBudgetPln").and_then(|v| v.as_f64()) {
        s.monthly_budget_pln = Some(b);
    }
    if let Some(b) = patch.get("batchModeEnabled").and_then(|v| v.as_bool()) {
        s.batch_mode_enabled = Some(b);
    }
    Ok(s)
}

#[tauri::command]
pub async fn get_cost_stats(month: String) -> Result<CostStats> {
    // TODO(D6-7): SELECT z api_usage WHERE month = ?.
    eprintln!("[rpstr] get_cost_stats {month}");
    Ok(CostStats {
        visits: 0,
        tokens_input: 0,
        tokens_output: 0,
        total_pln: 0.0,
    })
}
