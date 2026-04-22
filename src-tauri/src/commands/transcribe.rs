//! Transkrypcja whisper-rs. Model wybrany w profilu (Small/Medium/Large-v3-turbo).
//! Plan (D3-4): whisper_rs::WhisperContext::new_with_params(model_path) →
//! FullParams { language = "pl" } → run na buforze PCM z audio.rs.

use crate::error::Result;
use serde::Serialize;

#[derive(Serialize)]
pub struct TranscribeResult {
    pub transcript: String,
}

#[tauri::command]
pub async fn transcribe(visit_id: String) -> Result<TranscribeResult> {
    eprintln!("[rpstr] transcribe visit {visit_id} (stub)");
    // Póki co zwracamy placeholder, żeby UI dało się przeklikać end-to-end.
    Ok(TranscribeResult {
        transcript: "[transkrypt niedostępny — moduł whisper nie podłączony w D3]".into(),
    })
}
