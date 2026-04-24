use crate::db;
use crate::error::Result;
use crate::state::AppState;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

pub const SETTINGS_KEY: &str = "app_settings";

/// Domyślny tag Ollamy dla Bielik 7B.  Format `hf.co/<repo>:<quant>` wymaga
/// Ollamy 0.21+; jeśli pull się nie uda (stare Ollamy / brak internetu do HF),
/// wizard ma fallback na `qwen2.5:3b` (zawsze dostępny w oficjalnej bibliotece).
pub const DEFAULT_OLLAMA_MODEL: &str = "hf.co/speakleash/Bielik-7B-Instruct-v0.1-GGUF:Q4_K_M";
pub const FALLBACK_OLLAMA_MODEL: &str = "qwen2.5:3b";

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

    // Iteracja 2 — ścieżki pobieranych komponentów i stan kreatora.
    // `#[serde(default)]` żeby stara baza lekarza (bez tych pól) dalej parsowała.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whisper_bin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whisper_model: Option<String>,
    #[serde(default = "default_ollama_model")]
    pub ollama_model: String,
    #[serde(default)]
    pub setup_completed: bool,
}

fn default_ollama_model() -> String {
    DEFAULT_OLLAMA_MODEL.into()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            backend: "local-ollama".into(),
            profile: "standard".into(),
            models_dir: String::new(),
            audio_retention_days: 0,
            claude_model: Some("claude-haiku-4-5-20251001".into()),
            monthly_budget_pln: Some(50.0),
            batch_mode_enabled: Some(false),
            whisper_bin: None,
            whisper_model: None,
            ollama_model: DEFAULT_OLLAMA_MODEL.into(),
            setup_completed: false,
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
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings> {
    load_settings(&state)
}

/// Synchroniczny helper — używany przez `transcribe.rs`/`summarize.rs` żeby
/// odczytać ścieżki modeli bez async-command'u.  Jeśli baza jeszcze nie jest
/// odblokowana (brak `unlock_vault`), zwraca default, nie błąd.
pub fn load_settings(state: &AppState) -> Result<AppSettings> {
    let Ok(db) = db::db(state) else {
        return Ok(AppSettings::default());
    };
    db.with_conn(|c| {
        let raw: Option<String> = c
            .query_row(
                "SELECT value FROM settings WHERE key = ?",
                params![SETTINGS_KEY],
                |r| r.get(0),
            )
            .optional()?;
        match raw {
            Some(s) => Ok(serde_json::from_str(&s).unwrap_or_default()),
            None => Ok(AppSettings::default()),
        }
    })
}

#[tauri::command]
pub async fn update_settings(
    patch: Value,
    state: State<'_, AppState>,
) -> Result<AppSettings> {
    let db = db::db(&state)?;
    db.with_conn(|c| {
        let raw: Option<String> = c
            .query_row(
                "SELECT value FROM settings WHERE key = ?",
                params![SETTINGS_KEY],
                |r| r.get(0),
            )
            .optional()?;
        let mut current: AppSettings = raw
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        if let Some(b) = patch.get("backend").and_then(|v| v.as_str()) {
            current.backend = b.into();
        }
        if let Some(p) = patch.get("profile").and_then(|v| v.as_str()) {
            current.profile = p.into();
        }
        if let Some(d) = patch.get("modelsDir").and_then(|v| v.as_str()) {
            current.models_dir = d.into();
        }
        if let Some(r) = patch.get("audioRetentionDays").and_then(|v| v.as_u64()) {
            current.audio_retention_days = r as u32;
        }
        if let Some(m) = patch.get("claudeModel").and_then(|v| v.as_str()) {
            current.claude_model = Some(m.into());
        }
        if let Some(b) = patch.get("monthlyBudgetPln").and_then(|v| v.as_f64()) {
            current.monthly_budget_pln = Some(b);
        }
        if let Some(b) = patch.get("batchModeEnabled").and_then(|v| v.as_bool()) {
            current.batch_mode_enabled = Some(b);
        }
        if let Some(v) = patch.get("whisperBin").and_then(|v| v.as_str()) {
            current.whisper_bin = Some(v.into());
        }
        if let Some(v) = patch.get("whisperModel").and_then(|v| v.as_str()) {
            current.whisper_model = Some(v.into());
        }
        if let Some(v) = patch.get("ollamaModel").and_then(|v| v.as_str()) {
            current.ollama_model = v.into();
        }
        if let Some(v) = patch.get("setupCompleted").and_then(|v| v.as_bool()) {
            current.setup_completed = v;
        }

        let serialized = serde_json::to_string(&current).unwrap_or_else(|_| "{}".into());
        c.execute(
            "INSERT INTO settings (key, value) VALUES (?, ?) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![SETTINGS_KEY, serialized],
        )?;
        Ok(current)
    })
}

#[tauri::command]
pub async fn get_cost_stats(month: String, state: State<'_, AppState>) -> Result<CostStats> {
    let db = db::db(&state)?;
    db.with_conn(|c| {
        let row = c
            .query_row(
                "SELECT visits, tokens_input, tokens_output, cost_pln \
                 FROM api_usage WHERE month = ?",
                params![month],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)? as u32,
                        r.get::<_, i64>(1)? as u64,
                        r.get::<_, i64>(2)? as u64,
                        r.get::<_, f64>(3)?,
                    ))
                },
            )
            .optional()?;
        Ok(match row {
            Some((v, ti, to, cp)) => CostStats {
                visits: v,
                tokens_input: ti,
                tokens_output: to,
                total_pln: cp,
            },
            None => CostStats {
                visits: 0,
                tokens_input: 0,
                tokens_output: 0,
                total_pln: 0.0,
            },
        })
    })
}
