//! Migracje SQLCipher.
//!
//! Tabele:
//!   - patients (pseudonym UNIQUE, created_at)
//!   - visits (fk patient, started_at, status, consent_logged)
//!   - summaries (fk visit, chief_complaint, mse, diagnosis, recommendations, medications, accepted_codes JSON)
//!   - icd10_codes (code PK, label_pl, description) + FTS5 virtual shadow
//!   - consent_log (visit_id, timestamp, regulamin_version)
//!   - audit_log (timestamp, action, details JSON) — kto/kiedy/co
//!   - api_usage (month YYYY-MM, tokens_input, tokens_output, cost_pln)

pub const MIGRATIONS: &[&str] = &[
    r#"
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
    "#,
];
