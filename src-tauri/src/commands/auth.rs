use crate::error::Result;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn unlock_vault(password: String, state: State<'_, AppState>) -> Result<bool> {
    // TODO(D6-7): PBKDF2 + PRAGMA key + próbny SELECT.
    // Na razie dev stub: akceptuj dowolne niepuste hasło.
    if password.trim().is_empty() {
        return Ok(false);
    }
    *state.unlocked.lock().unwrap() = true;
    Ok(true)
}
