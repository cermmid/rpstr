//! Migracje SQLCipher + seed bazy ICD-10.

use crate::error::Result;
use rusqlite::{params, Connection};

/// Bundlowana baza ICD-10 (podzbiór polskiej MKCh-10, psychiatria).
const ICD10_CSV: &str = include_str!("../../resources/icd10.csv");

const SCHEMA_V1: &str = r#"
BEGIN;

CREATE TABLE IF NOT EXISTS _rpstr_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR REPLACE INTO _rpstr_meta (key, value) VALUES ('schema_version', '1');
INSERT OR IGNORE INTO _rpstr_meta (key, value) VALUES ('created_at', datetime('now'));

CREATE TABLE IF NOT EXISTS patients (
    id TEXT PRIMARY KEY,
    pseudonym TEXT UNIQUE NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS visits (
    id TEXT PRIMARY KEY,
    patient_id TEXT NOT NULL REFERENCES patients(id),
    started_at TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    consent_logged INTEGER NOT NULL DEFAULT 0,
    transcript TEXT,
    model_used TEXT,
    tokens_input INTEGER,
    tokens_output INTEGER,
    cost_pln REAL
);

CREATE INDEX IF NOT EXISTS idx_visits_started_at ON visits(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_visits_patient ON visits(patient_id);

CREATE TABLE IF NOT EXISTS summaries (
    visit_id TEXT PRIMARY KEY REFERENCES visits(id) ON DELETE CASCADE,
    chief_complaint TEXT,
    mental_state_exam TEXT,
    diagnosis TEXT,
    recommendations TEXT,
    medications TEXT,
    accepted_codes TEXT
);

CREATE TABLE IF NOT EXISTS consent_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    visit_id TEXT NOT NULL REFERENCES visits(id),
    logged_at TEXT NOT NULL,
    regulamin_version TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    logged_at TEXT NOT NULL,
    action TEXT NOT NULL,
    details TEXT
);

CREATE TABLE IF NOT EXISTS api_usage (
    month TEXT PRIMARY KEY,
    visits INTEGER NOT NULL DEFAULT 0,
    tokens_input INTEGER NOT NULL DEFAULT 0,
    tokens_output INTEGER NOT NULL DEFAULT 0,
    cost_pln REAL NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS icd10_codes (
    code TEXT PRIMARY KEY,
    label_pl TEXT NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS icd10_fts USING fts5(
    code,
    label_pl,
    tokenize = 'unicode61 remove_diacritics 2'
);

COMMIT;
"#;

pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA_V1)?;
    seed_icd10_if_empty(conn)?;
    Ok(())
}

fn seed_icd10_if_empty(conn: &Connection) -> Result<()> {
    let count: i64 = conn.query_row("SELECT count(*) FROM icd10_codes", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }
    let mut lines = ICD10_CSV.lines();
    let _header = lines.next(); // pomijamy "code,label_pl"

    // Rusqlite wymaga `&mut Connection` do transakcji, ale tu mamy `&` —
    // używamy manualnego BEGIN/COMMIT, co w tym kontekście daje ten sam
    // efekt (atomic load ~130 wierszy).
    conn.execute_batch("BEGIN")?;
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let Some((code, label)) = line.split_once(',') else {
            continue;
        };
        let code = code.trim();
        let label = label.trim();
        if code.is_empty() || label.is_empty() {
            continue;
        }
        conn.execute(
            "INSERT OR IGNORE INTO icd10_codes (code, label_pl) VALUES (?, ?)",
            params![code, label],
        )?;
        conn.execute(
            "INSERT INTO icd10_fts (code, label_pl) VALUES (?, ?)",
            params![code, label],
        )?;
    }
    conn.execute_batch("COMMIT")?;
    Ok(())
}
