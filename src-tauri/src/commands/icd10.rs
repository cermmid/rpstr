//! Wyszukiwanie w bundlowanej bazie ICD-10 (FTS5 z `unicode61 remove_diacritics`).
//!
//! Query trafia do wirtualnej tabeli `icd10_fts` i jest rankowana przez BM25.
//! Dla bardzo krótkich zapytań (1-2 znaki) wpadamy w prefix match na samym kodzie.

use crate::db;
use crate::error::{AppError, Result};
use crate::state::AppState;
use rusqlite::params;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Icd10Match {
    pub code: String,
    pub label_pl: String,
}

#[tauri::command]
pub async fn search_icd10(query: String, state: State<'_, AppState>) -> Result<Vec<Icd10Match>> {
    let db = db::db(&state)?;
    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }

    // Strategy:
    //  - jeśli zaczyna się od litery A-Z + cyfra (np. "F32"), potraktuj jak prefix kodu
    //  - inaczej FTS5 match z wildcardem na końcu każdego tokenu
    let is_code_like = q.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false)
        && q.chars().nth(1).map(|c| c.is_ascii_digit()).unwrap_or(false);

    db.with_conn(|c| {
        let rows: Vec<Icd10Match> = if is_code_like {
            let mut stmt = c.prepare(
                "SELECT code, label_pl FROM icd10_codes \
                 WHERE code LIKE ? ORDER BY code LIMIT 20",
            )?;
            let pattern = format!("{}%", q.to_uppercase());
            let iter = stmt.query_map(params![pattern], |row| {
                Ok(Icd10Match {
                    code: row.get(0)?,
                    label_pl: row.get(1)?,
                })
            })?;
            collect(iter)?
        } else {
            // FTS5 wymaga bezpiecznego query stringu — tokenizujemy i dodajemy * do każdego.
            let match_query = build_fts_query(q);
            let mut stmt = c.prepare(
                "SELECT code, label_pl FROM icd10_fts \
                 WHERE icd10_fts MATCH ? \
                 ORDER BY bm25(icd10_fts) LIMIT 20",
            )?;
            let iter = stmt.query_map(params![match_query], |row| {
                Ok(Icd10Match {
                    code: row.get(0)?,
                    label_pl: row.get(1)?,
                })
            })?;
            match collect(iter) {
                Ok(v) => v,
                // Niepoprawny FTS match (np. same znaki specjalne) — zwracamy pustą listę.
                Err(_) => Vec::new(),
            }
        };
        Ok(rows)
    })
}

fn build_fts_query(input: &str) -> String {
    input
        .split_whitespace()
        .filter(|tok| tok.chars().any(|c| c.is_alphanumeric()))
        .map(|tok| {
            // Wywal znaki specjalne FTS5 (", *, :, ^, ()).
            let cleaned: String = tok
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-')
                .collect();
            format!("{cleaned}*")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn collect<I>(iter: I) -> Result<Vec<Icd10Match>>
where
    I: Iterator<Item = std::result::Result<Icd10Match, rusqlite::Error>>,
{
    let mut out = Vec::new();
    for item in iter {
        out.push(item.map_err(AppError::Db)?);
    }
    Ok(out)
}
