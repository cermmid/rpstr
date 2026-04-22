use crate::error::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Visit {
    pub id: String,
    pub patient_id: String,
    pub started_at: String,
    pub status: String,
    pub consent_logged: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_codes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_used: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_input: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_output: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_pln: Option<f64>,
}

#[tauri::command]
pub async fn list_visits() -> Result<Vec<Visit>> {
    // TODO(D6-7): SELECT z SQLCipher.
    Ok(vec![])
}

#[tauri::command]
pub async fn create_visit(patient_pseudonym: String) -> Result<Visit> {
    // TODO(D6-7): INSERT do SQLCipher. Póki co zwracamy obiekt w pamięci.
    Ok(Visit {
        id: Uuid::new_v4().to_string(),
        patient_id: patient_pseudonym,
        started_at: Utc::now().to_rfc3339(),
        status: "draft".into(),
        consent_logged: false,
        transcript: None,
        summary: None,
        accepted_codes: None,
        model_used: None,
        tokens_input: None,
        tokens_output: None,
        cost_pln: None,
    })
}

#[tauri::command]
pub async fn log_consent(visit_id: String) -> Result<()> {
    // TODO: INSERT INTO consent_log (visit_id, now, regulamin_version).
    eprintln!("[rpstr] consent logged for visit {visit_id}");
    Ok(())
}

#[tauri::command]
pub async fn save_visit(visit_id: String, patch: Value) -> Result<Visit> {
    eprintln!("[rpstr] save_visit {visit_id}: {patch}");
    Ok(Visit {
        id: visit_id,
        patient_id: "?".into(),
        started_at: Utc::now().to_rfc3339(),
        status: "saved".into(),
        consent_logged: true,
        transcript: None,
        summary: None,
        accepted_codes: None,
        model_used: None,
        tokens_input: None,
        tokens_output: None,
        cost_pln: None,
    })
}
