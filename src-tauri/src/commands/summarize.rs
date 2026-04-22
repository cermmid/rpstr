//! Podsumowanie SOAP + sugerowane ICD-10.
//!
//! Router backend:
//!   - `local-ollama` (domyślnie): POST do http://localhost:11434/api/chat
//!     z `format: "json"`, model wg AppSettings.profile.
//!   - `claude-byok`: TODO(MVP+1) — pseudonimizacja + call_claude z prompt
//!     caching.
//!
//! Input to transkrypt ze `transcribe.rs` (potem zastąpi go dodatkowy pass
//! przez `preprocess.rs`).  Wynik parsujemy jako JSON pasujący do `Summary`.

use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};

const SYSTEM_PROMPT: &str = include_str!("../../../prompts/soap_pl.txt");
const OLLAMA_URL: &str = "http://localhost:11434/api/chat";

/// Struktura wysyłana do frontendu (camelCase dla zgodności z TypeScript/JS).
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedCode {
    pub code: String,
    pub label_pl: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub chief_complaint: String,
    pub mental_state_exam: String,
    pub diagnosis: String,
    pub recommendations: String,
    pub medications: String,
    pub suggested_codes: Vec<SuggestedCode>,
}

/// Wire format do deserializacji outputu LLM (snake_case zgodny z promptem).
/// LLM-y lepiej trzymają się snake_case dla medycznej struktury SOAP, dlatego
/// prompt opisuje właśnie taki schema — tu mapujemy na camelCase dla UI.
#[derive(Deserialize)]
struct SummaryWire {
    #[serde(default)]
    chief_complaint: String,
    #[serde(default)]
    mental_state_exam: String,
    #[serde(default)]
    diagnosis: String,
    #[serde(default)]
    recommendations: String,
    #[serde(default)]
    medications: String,
    #[serde(default)]
    suggested_codes: Vec<SuggestedCodeWire>,
}

#[derive(Deserialize)]
struct SuggestedCodeWire {
    #[serde(default)]
    code: String,
    #[serde(default)]
    label_pl: String,
}

impl From<SummaryWire> for Summary {
    fn from(w: SummaryWire) -> Self {
        Self {
            chief_complaint: w.chief_complaint,
            mental_state_exam: w.mental_state_exam,
            diagnosis: w.diagnosis,
            recommendations: w.recommendations,
            medications: w.medications,
            suggested_codes: w
                .suggested_codes
                .into_iter()
                .map(|c| SuggestedCode {
                    code: c.code,
                    label_pl: c.label_pl,
                })
                .collect(),
        }
    }
}

impl Default for Summary {
    fn default() -> Self {
        Self {
            chief_complaint: String::new(),
            mental_state_exam: String::new(),
            diagnosis: String::new(),
            recommendations: String::new(),
            medications: String::new(),
            suggested_codes: Vec::new(),
        }
    }
}

#[tauri::command]
pub async fn summarize(visit_id: String, transcript: String) -> Result<Summary> {
    eprintln!(
        "[rpstr/summarize] visit={visit_id} transcript_len={}",
        transcript.len()
    );

    // TODO(D6-7): odczyt AppSettings z SQLCipher. Póki co hardkod: local-ollama,
    // profil "standard" → model llama3.1:8b.
    let backend = std::env::var("RPSTR_BACKEND").unwrap_or_else(|_| "local-ollama".into());
    match backend.as_str() {
        "local-ollama" => {
            let model = std::env::var("RPSTR_OLLAMA_MODEL")
                .unwrap_or_else(|_| "llama3.1:8b".into());
            call_ollama(&model, &transcript).await
        }
        "claude-byok" => Err(AppError::NotImplemented("claude-byok w D5 — patrz commands/claude.rs")),
        other => Err(AppError::Other(format!("nieznany backend: {other}"))),
    }
}

// ─────────────────────────────── Ollama client ───────────────────────────────

#[derive(Serialize)]
struct OllamaRequest<'a> {
    model: &'a str,
    messages: Vec<OllamaMessage<'a>>,
    format: &'a str,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaResponseMessage,
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    #[allow(dead_code)]
    role: String,
    content: String,
}

async fn call_ollama(model: &str, transcript: &str) -> Result<Summary> {
    let req = OllamaRequest {
        model,
        messages: vec![
            OllamaMessage {
                role: "system",
                content: SYSTEM_PROMPT,
            },
            OllamaMessage {
                role: "user",
                content: transcript,
            },
        ],
        format: "json",
        stream: false,
        options: OllamaOptions {
            temperature: 0.2,
            num_predict: 1500,
        },
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()?;

    let response = client
        .post(OLLAMA_URL)
        .json(&req)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| AppError::Other(format!("Ollama HTTP: {e}")))?;

    let body: OllamaResponse = response
        .json()
        .await
        .map_err(|e| AppError::Other(format!("Ollama response parse: {e}")))?;

    parse_summary_json(&body.message.content)
}

/// Parsuje JSON output modelu.  Llama czasem wtrąca ```json``` albo tekst przed
/// JSON-em — obcinamy najpierw do pierwszego `{` i ostatniego `}`.
fn parse_summary_json(raw: &str) -> Result<Summary> {
    let trimmed = extract_json_object(raw).unwrap_or(raw);
    let wire: SummaryWire = serde_json::from_str(trimmed).map_err(|e| {
        AppError::Other(format!(
            "LLM zwrócił niepoprawny JSON ({e}): {}",
            preview(raw, 200)
        ))
    })?;
    Ok(wire.into())
}

fn extract_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end > start {
        Some(&s[start..=end])
    } else {
        None
    }
}

fn preview(s: &str, max: usize) -> String {
    let trimmed: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        format!("{trimmed}…")
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_json() {
        let raw = r#"{
            "chief_complaint": "Bezsenność od 3 miesięcy",
            "mental_state_exam": "Kontakt prawidłowy, nastrój obniżony",
            "diagnosis": "Epizod depresyjny umiarkowany",
            "recommendations": "Psychoterapia CBT, higiena snu",
            "medications": "Escitalopram 10 mg 1-0-0",
            "suggested_codes": [
                {"code": "F32.1", "label_pl": "Epizod depresji umiarkowany"}
            ]
        }"#;
        let s = parse_summary_json(raw).unwrap();
        assert_eq!(s.suggested_codes.len(), 1);
        assert_eq!(s.suggested_codes[0].code, "F32.1");
    }

    #[test]
    fn strips_markdown_fences() {
        let raw = "```json\n{\"chief_complaint\":\"x\",\"mental_state_exam\":\"\",\"diagnosis\":\"\",\"recommendations\":\"\",\"medications\":\"\",\"suggested_codes\":[]}\n```";
        let s = parse_summary_json(raw).unwrap();
        assert_eq!(s.chief_complaint, "x");
    }

    #[test]
    fn fails_on_garbage() {
        assert!(parse_summary_json("nope").is_err());
    }
}
