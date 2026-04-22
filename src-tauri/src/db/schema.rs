//! Migracje SQLCipher.

use crate::error::Result;
use rusqlite::Connection;

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

COMMIT;
"#;

pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA_V1)?;
    Ok(())
}
