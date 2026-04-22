use parking_lot::Mutex;
use std::sync::Arc;

use crate::audio;
use crate::db::Database;

/// Globalny stan aplikacji, trzymany przez Tauri i wstrzykiwany do command'ów.
///
/// `db` jest `Option`: przed `unlock_vault` jest `None`, po odblokowaniu
/// zawiera handle do zaszyfrowanej bazy.  Wątek recordera (cpal) jest `!Send`,
/// więc siedzi za `AudioController` z własnym kanałem.
pub struct AppState {
    pub db: Mutex<Option<Database>>,
    pub audio: Arc<audio::AudioController>,
    pub last_pcm: Mutex<Option<audio::Pcm>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            db: Mutex::new(None),
            audio: Arc::new(audio::AudioController::spawn()),
            last_pcm: Mutex::new(None),
        }
    }
}
