//! Podsumowanie SOAP + sugerowane ICD-10.
//! Router: backend lokalny (Ollama HTTP) albo Claude BYOK. Wybór w settings.
//! Input: transkrypt po preprocess.rs (pseudonimizacja + filler removal).

use crate::error::Result;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedCode {
    pub code: String,
    pub label_pl: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub chief_complaint: String,
    pub mental_state_exam: String,
    pub diagnosis: String,
    pub recommendations: String,
    pub medications: String,
    pub suggested_codes: Vec<SuggestedCode>,
}

#[tauri::command]
pub async fn summarize(visit_id: String, transcript: String) -> Result<Summary> {
    eprintln!(
        "[rpstr] summarize visit {visit_id}, transcript_len={}",
        transcript.len()
    );
    // TODO(D5/D9):
    //   - sprawdź settings.backend
    //   - lokalny: POST http://localhost:11434/api/generate z promptem z prompts/soap_pl.txt
    //   - claude: commands::preprocess + commands::claude z prompt caching
    //   - parsuj JSON output, zwróć strukturyzowane pola
    Ok(Summary {
        chief_complaint: "(szkic — backend nie podłączony)".into(),
        mental_state_exam: "".into(),
        diagnosis: "".into(),
        recommendations: "".into(),
        medications: "".into(),
        suggested_codes: vec![],
    })
}
