use parking_lot::Mutex;
use std::sync::Arc;

use crate::audio;

/// Globalny stan aplikacji, trzymany przez Tauri i wstrzykiwany do command'ów.
///
/// Wszystko w `Arc<Mutex<..>>`, żeby command'y (async, multi-threaded) mogły się
/// bezpiecznie współdzielić.  Wątek recordera (cpal) jest `!Send`, więc siedzi
/// za `AudioController`, który rozmawia z nim przez kanał — sam kontroler jest
/// thread-safe.
pub struct AppState {
    pub unlocked: Mutex<bool>,
    pub audio: Arc<audio::AudioController>,
    pub last_pcm: Mutex<Option<audio::Pcm>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            unlocked: Mutex::new(false),
            audio: Arc::new(audio::AudioController::spawn()),
            last_pcm: Mutex::new(None),
        }
    }
}
