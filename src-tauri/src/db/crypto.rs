//! Otwarcie SQLCipher z kluczem wyprowadzonym z hasła lekarza.
//!
//! Salt 16 B trzymamy obok bazy w pliku `salt.bin` — nie jest to sekret
//! kryptograficzny, służy wyłącznie do odseparowania równoległych instalacji.
//! Faktyczny sekret to hasło użytkownika, z którego PBKDF2-HMAC-SHA256 (600k
//! iteracji) wyprowadza 32-bajtowy klucz AES-256 dla SQLCipher.
//!
//! Weryfikacja hasła: przy otwarciu istniejącej bazy próbujemy `SELECT`
//! z tabeli `_rpstr_meta`.  Zły klucz → SQLCipher zgłasza błąd
//! "file is not a database" i podnosimy `AppError::VaultLocked`.

use crate::error::{AppError, Result};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use rusqlite::Connection;
use sha2::Sha256;
use std::fs;
use std::path::{Path, PathBuf};

const PBKDF2_ITERS: u32 = 600_000;
const KEY_LEN: usize = 32;
const SALT_LEN: usize = 16;

pub fn data_dir() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("RPSTR_DATA_DIR") {
        return Ok(PathBuf::from(p));
    }
    let base = dirs::data_dir().ok_or_else(|| AppError::Other("brak katalogu danych".into()))?;
    Ok(base.join("rpstr"))
}

fn db_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("db.sqlite"))
}

fn salt_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("salt.bin"))
}

fn load_or_create_salt() -> Result<[u8; SALT_LEN]> {
    let path = salt_path()?;
    if let Ok(bytes) = fs::read(&path) {
        if bytes.len() == SALT_LEN {
            let mut salt = [0u8; SALT_LEN];
            salt.copy_from_slice(&bytes);
            return Ok(salt);
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    fs::write(&path, salt)?;
    Ok(salt)
}

fn derive_key(password: &str, salt: &[u8]) -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERS, &mut key);
    key
}

/// Otwiera połączenie do zaszyfrowanej bazy.  Tworzy ją i uruchamia migracje
/// przy pierwszym uruchomieniu; inaczej weryfikuje hasło przez probny SELECT.
pub fn open(password: &str) -> Result<Connection> {
    let db_file = db_path()?;
    if let Some(parent) = db_file.parent() {
        fs::create_dir_all(parent)?;
    }
    let db_exists = db_file.exists();
    open_at(&db_file, password, db_exists)
}

/// Wariant używany w testach i kreatorze backupu — pozwala wskazać inną ścieżkę.
pub fn open_at(path: &Path, password: &str, existing: bool) -> Result<Connection> {
    let salt = load_or_create_salt()?;
    let key = derive_key(password, &salt);

    let conn = Connection::open(path)?;
    // PRAGMA key musi być pierwszym wykonanym statementem.
    conn.pragma_update(None, "key", format!("x'{}'", hex::encode(key)))?;

    if existing {
        // Test klucza: SELECT z tabeli meta.  Zła wartość = błąd SQLCipher.
        conn.query_row("SELECT count(*) FROM _rpstr_meta", [], |_| Ok(()))
            .map_err(|_| AppError::VaultLocked)?;
    } else {
        super::schema::migrate(&conn)?;
    }
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn temp_db() -> NamedTempFile {
        NamedTempFile::new().expect("temp file")
    }

    #[test]
    fn derive_key_is_deterministic() {
        let salt = [0u8; SALT_LEN];
        let a = derive_key("hunter2", &salt);
        let b = derive_key("hunter2", &salt);
        assert_eq!(a, b);
    }

    #[test]
    fn derive_key_differs_for_different_password() {
        let salt = [0u8; SALT_LEN];
        assert_ne!(derive_key("a", &salt), derive_key("b", &salt));
    }

    #[test]
    fn create_then_reopen_with_same_password() {
        let tmp = temp_db();
        let dir = tmp.path().parent().unwrap().to_path_buf();
        std::env::set_var("RPSTR_DATA_DIR", &dir);
        let db = tmp.path();

        let _ = std::fs::remove_file(db);
        let c = open_at(db, "hasło-testowe", false).unwrap();
        drop(c);
        let c2 = open_at(db, "hasło-testowe", true).unwrap();
        drop(c2);

        let _ = std::fs::remove_file(db);
    }

    #[test]
    fn icd10_seed_loads_and_fts_works() {
        let tmp = temp_db();
        let dir = tmp.path().parent().unwrap().to_path_buf();
        std::env::set_var("RPSTR_DATA_DIR", &dir);
        let db = tmp.path();

        let _ = std::fs::remove_file(db);
        let conn = open_at(db, "test-pass", false).unwrap();

        // Seed wczytany
        let count: i64 = conn
            .query_row("SELECT count(*) FROM icd10_codes", [], |r| r.get(0))
            .unwrap();
        assert!(count > 100, "expected >100 ICD-10 codes, got {count}");

        // FTS znajduje "depresja" → kody z grupy F32/F33
        let mut stmt = conn
            .prepare("SELECT code FROM icd10_fts WHERE icd10_fts MATCH 'depresj*' ORDER BY bm25(icd10_fts)")
            .unwrap();
        let codes: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(codes.iter().any(|c| c.starts_with("F32") || c.starts_with("F33")));

        // Prefix na kodzie
        let label: String = conn
            .query_row(
                "SELECT label_pl FROM icd10_codes WHERE code = ?",
                ["F41.1"],
                |r| r.get(0),
            )
            .unwrap();
        assert!(label.to_lowercase().contains("uogólnione") || label.to_lowercase().contains("lekowe"));

        let _ = std::fs::remove_file(db);
    }

    #[test]
    fn reopen_with_wrong_password_fails() {
        let tmp = temp_db();
        let dir = tmp.path().parent().unwrap().to_path_buf();
        std::env::set_var("RPSTR_DATA_DIR", &dir);
        let db = tmp.path();

        let _ = std::fs::remove_file(db);
        let _ = open_at(db, "correct-horse", false).unwrap();
        let r = open_at(db, "battery-staple", true);
        assert!(matches!(r, Err(AppError::VaultLocked)));
        let _ = std::fs::remove_file(db);
    }
}
