//! BYOK Claude API — wywołanie z prompt caching + tool use + limitem budżetu.
//!
//! Dźwignie minimalizacji kosztów (zgodnie z planem):
//!   1. Prompt caching: system prompt + baza ICD-10 (~13 500 tok) w bloku
//!      `{"type": "text", "text": "...", "cache_control": {"type": "ephemeral"}}`.
//!      Pierwszy request = cache write (+25%). Kolejne w ciągu 5 min = cache read (-90%).
//!   2. Domyślny model Haiku 4.5 (`claude-haiku-4-5-20251001`), opcja eskalacji
//!      do Sonnet 4.6 per-wizyta.
//!   3. `max_tokens: 1500` — twardy limit, żeby model nie rozlał się na narracje.
//!   4. Tool use ze schema'ą Summary wymusza JSON bez rozwlekłych wstępów (-30-50% output).
//!   5. Input zawsze po preprocess.rs (pseudonimizacja + filler removal, -15-25%).
//!   6. Licznik tokenów → api_usage w DB → respektuj monthly_budget_pln z settings.
//!
//! Referencja pricing (do potwierdzenia przed release):
//!   Haiku 4.5: ~$1/MTok input, ~$5/MTok output, cache read ~$0.1/MTok.

use crate::error::Result;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct ClaudeRequest {
    pub transcript_pseudonymized: String,
    pub model: String,
}

pub async fn call_claude(_req: ClaudeRequest) -> Result<serde_json::Value> {
    // TODO(MVP+1, tydzień 3): pełna implementacja z reqwest.
    //
    // Szkic requesta:
    //
    // let body = json!({
    //     "model": req.model,
    //     "max_tokens": 1500,
    //     "system": [
    //         { "type": "text", "text": SYSTEM_PROMPT },
    //         { "type": "text", "text": ICD10_CATALOG,
    //           "cache_control": { "type": "ephemeral" } },
    //     ],
    //     "tools": [SOAP_TOOL_SCHEMA],
    //     "tool_choice": { "type": "tool", "name": "emit_summary" },
    //     "messages": [
    //         { "role": "user", "content": req.transcript_pseudonymized }
    //     ],
    // });
    //
    // reqwest::Client::new()
    //   .post("https://api.anthropic.com/v1/messages")
    //   .header("x-api-key", key_from_keyring()?)
    //   .header("anthropic-version", "2023-06-01")
    //   .json(&body)
    //   .send().await?
    //   .json::<Value>().await?;
    //
    // Następnie: usage.input_tokens + usage.output_tokens + usage.cache_read_input_tokens
    // → zapis do api_usage + sprawdzenie budżetu.
    let _ = json!({});
    Err(crate::error::AppError::NotImplemented("claude::call_claude"))
}
