use crate::db;
use crate::error::{AppError, Result};
use crate::state::AppState;
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;
use uuid::Uuid;

const REGULAMIN_VERSION: &str = "0.1";

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
pub async fn list_visits(state: State<'_, AppState>) -> Result<Vec<Visit>> {
    let db = db::db(&state)?;
    db.with_conn(|c| {
        let mut stmt = c.prepare(
            "SELECT v.id, p.pseudonym, v.started_at, v.status, v.consent_logged, \
                    v.transcript, v.model_used, v.tokens_input, v.tokens_output, v.cost_pln, \
                    s.accepted_codes \
             FROM visits v \
             JOIN patients p ON p.id = v.patient_id \
             LEFT JOIN summaries s ON s.visit_id = v.id \
             ORDER BY v.started_at DESC \
             LIMIT 500",
        )?;
        let rows = stmt.query_map([], |row| {
            let accepted_codes_json: Option<String> = row.get(10)?;
            let accepted_codes = accepted_codes_json
                .as_deref()
                .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok());
            Ok(Visit {
                id: row.get(0)?,
                patient_id: row.get(1)?,
                started_at: row.get(2)?,
                status: row.get(3)?,
                consent_logged: row.get::<_, i64>(4)? != 0,
                transcript: row.get(5)?,
                summary: None,
                accepted_codes,
                model_used: row.get(6)?,
                tokens_input: row.get::<_, Option<i64>>(7)?.map(|v| v as u32),
                tokens_output: row.get::<_, Option<i64>>(8)?.map(|v| v as u32),
                cost_pln: row.get(9)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(AppError::Db)?);
        }
        Ok(out)
    })
}

#[tauri::command]
pub async fn create_visit(
    patient_pseudonym: String,
    state: State<'_, AppState>,
) -> Result<Visit> {
    let db = db::db(&state)?;
    let pseudonym = patient_pseudonym.trim().to_string();
    if pseudonym.is_empty() {
        return Err(AppError::Other("pseudonim nie może być pusty".into()));
    }
    let visit_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    db.with_conn(|c| {
        // INSERT OR IGNORE pacjenta (pseudonym unique); potem SELECT id.
        c.execute(
            "INSERT OR IGNORE INTO patients (id, pseudonym, created_at) VALUES (?, ?, ?)",
            params![Uuid::new_v4().to_string(), pseudonym, now],
        )?;
        let patient_id: String = c.query_row(
            "SELECT id FROM patients WHERE pseudonym = ?",
            params![pseudonym],
            |r| r.get(0),
        )?;
        c.execute(
            "INSERT INTO visits (id, patient_id, started_at, status, consent_logged) \
             VALUES (?, ?, ?, 'draft', 0)",
            params![visit_id, patient_id, now],
        )?;
        log_audit(c, "visit.create", Some(&visit_id))?;
        Ok(Visit {
            id: visit_id.clone(),
            patient_id: pseudonym.clone(),
            started_at: now.clone(),
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
    })
}

#[tauri::command]
pub async fn log_consent(visit_id: String, state: State<'_, AppState>) -> Result<()> {
    let db = db::db(&state)?;
    let now = Utc::now().to_rfc3339();
    db.with_conn(|c| {
        c.execute(
            "INSERT INTO consent_log (visit_id, logged_at, regulamin_version) VALUES (?, ?, ?)",
            params![visit_id, now, REGULAMIN_VERSION],
        )?;
        c.execute(
            "UPDATE visits SET consent_logged = 1 WHERE id = ?",
            params![visit_id],
        )?;
        log_audit(c, "visit.consent", Some(&visit_id))?;
        Ok(())
    })
}

#[tauri::command]
pub async fn save_visit(
    visit_id: String,
    patch: Value,
    state: State<'_, AppState>,
) -> Result<Visit> {
    let db = db::db(&state)?;
    let now = Utc::now().to_rfc3339();
    let transcript = patch.get("transcript").and_then(|v| v.as_str()).map(String::from);
    let summary = patch.get("summary").cloned();
    let accepted_codes = patch
        .get("acceptedCodes")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>());

    db.with_conn_mut(|c| {
        let tx = c.transaction()?;
        if let Some(t) = &transcript {
            tx.execute(
                "UPDATE visits SET transcript = ?, status = 'saved' WHERE id = ?",
                params![t, visit_id],
            )?;
        } else {
            tx.execute(
                "UPDATE visits SET status = 'saved' WHERE id = ?",
                params![visit_id],
            )?;
        }
        if let Some(s) = &summary {
            let chief = s.get("chiefComplaint").and_then(|v| v.as_str()).unwrap_or("");
            let mse = s.get("mentalStateExam").and_then(|v| v.as_str()).unwrap_or("");
            let diag = s.get("diagnosis").and_then(|v| v.as_str()).unwrap_or("");
            let rec = s.get("recommendations").and_then(|v| v.as_str()).unwrap_or("");
            let meds = s.get("medications").and_then(|v| v.as_str()).unwrap_or("");
            let codes_json = accepted_codes
                .as_ref()
                .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".into()))
                .unwrap_or_else(|| "[]".into());
            tx.execute(
                "INSERT INTO summaries (visit_id, chief_complaint, mental_state_exam, diagnosis, \
                                        recommendations, medications, accepted_codes) \
                 VALUES (?, ?, ?, ?, ?, ?, ?) \
                 ON CONFLICT(visit_id) DO UPDATE SET \
                    chief_complaint = excluded.chief_complaint, \
                    mental_state_exam = excluded.mental_state_exam, \
                    diagnosis = excluded.diagnosis, \
                    recommendations = excluded.recommendations, \
                    medications = excluded.medications, \
                    accepted_codes = excluded.accepted_codes",
                params![visit_id, chief, mse, diag, rec, meds, codes_json],
            )?;
        }
        tx.execute(
            "INSERT INTO audit_log (logged_at, action, details) VALUES (?, 'visit.save', ?)",
            params![now, visit_id],
        )?;
        tx.commit()?;
        Ok(())
    })?;

    let patient_id: String = db.with_conn(|c| {
        let row: String = c.query_row(
            "SELECT p.pseudonym FROM visits v JOIN patients p ON p.id = v.patient_id WHERE v.id = ?",
            params![visit_id],
            |r| r.get(0),
        )?;
        Ok(row)
    })?;

    Ok(Visit {
        id: visit_id,
        patient_id,
        started_at: now.clone(),
        status: "saved".into(),
        consent_logged: true,
        transcript,
        summary,
        accepted_codes,
        model_used: None,
        tokens_input: None,
        tokens_output: None,
        cost_pln: None,
    })
}

fn log_audit(conn: &rusqlite::Connection, action: &str, details: Option<&str>) -> Result<()> {
    conn.execute(
        "INSERT INTO audit_log (logged_at, action, details) VALUES (?, ?, ?)",
        params![Utc::now().to_rfc3339(), action, details],
    )?;
    Ok(())
}
