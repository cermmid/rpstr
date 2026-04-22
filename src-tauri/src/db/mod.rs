pub mod crypto;
pub mod schema;

use crate::error::{AppError, Result};
use parking_lot::Mutex;
use rusqlite::Connection;

/// Thread-safe wrapper wokół połączenia SQLCipher.  Tauri command'y pobierają
/// go ze state przez `with_conn` — żadna bezpośrednia ekspozycja `Connection`.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(password: &str) -> Result<Self> {
        let conn = crypto::open(password)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        f(&self.conn.lock())
    }

    pub fn with_conn_mut<T>(&self, f: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
        f(&mut self.conn.lock())
    }
}

/// Pobiera bazę ze stanu albo podnosi `VaultLocked`, jeśli nie jest odblokowana.
pub fn db<'a>(state: &'a crate::state::AppState) -> Result<parking_lot::MappedMutexGuard<'a, Database>> {
    let guard = state.db.lock();
    parking_lot::MutexGuard::try_map(guard, |opt| opt.as_mut())
        .map_err(|_| AppError::VaultLocked)
}
