//! Otwarcie SQLCipher z kluczem wyprowadzonym z hasła lekarza.
//!
//! Plan implementacji (D6-7 MVP):
//!   1. Pobierz salt z systemowego keyring (`keyring::Entry::new("rpstr", "db-salt")`).
//!      Jeśli brak — wygeneruj i zapisz.
//!   2. Wyprowadź 32-bajtowy klucz PBKDF2-SHA256 z hasła + salt, 600k iteracji.
//!   3. Otwórz SQLite z `PRAGMA key='x''<hex_key>''';`.
//!   4. Zweryfikuj otwarcie próbnym SELECT — zła wartość klucza da błąd
//!      "file is not a database".
//!   5. Uruchom migracje ze `schema.rs`.

use crate::error::Result;
use rusqlite::Connection;
use std::path::Path;

pub fn open_encrypted(_path: &Path, _password: &str) -> Result<Connection> {
    // TODO(D6-7): pełna implementacja PBKDF2 + PRAGMA key.
    Err(crate::error::AppError::NotImplemented("db::crypto::open_encrypted"))
}
