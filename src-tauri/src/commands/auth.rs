use crate::db::Database;
use crate::error::{AppError, Result};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn unlock_vault(password: String, state: State<'_, AppState>) -> Result<bool> {
    if password.trim().is_empty() {
        return Ok(false);
    }
    match Database::open(&password) {
        Ok(db) => {
            *state.db.lock() = Some(db);
            Ok(true)
        }
        Err(AppError::VaultLocked) => Ok(false),
        Err(e) => Err(e),
    }
}
