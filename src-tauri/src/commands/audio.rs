//! Nagrywanie audio przez `cpal`. Bufor w RAM, NIGDY na dysku.
//! Plan (D3-4): cpal::Host → default_input_device → stream 16 kHz mono f32 →
//! VecDeque<f32> wewnątrz `AppState::recorder_buffer`.

use crate::error::Result;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingResult {
    pub duration_ms: u64,
}

#[tauri::command]
pub async fn start_recording() -> Result<()> {
    // TODO(D3): cpal input stream.
    eprintln!("[rpstr] start_recording (stub)");
    Ok(())
}

#[tauri::command]
pub async fn stop_recording() -> Result<RecordingResult> {
    // TODO(D3): flush buffer, zwróć czas trwania.
    Ok(RecordingResult { duration_ms: 0 })
}
