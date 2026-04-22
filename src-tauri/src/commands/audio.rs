use crate::error::{AppError, Result};
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingResult {
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub num_samples: usize,
}

#[tauri::command]
pub async fn start_recording(state: State<'_, AppState>) -> Result<()> {
    state.audio.start().map_err(AppError::Other)
}

#[tauri::command]
pub async fn stop_recording(state: State<'_, AppState>) -> Result<RecordingResult> {
    let pcm = state.audio.stop().map_err(AppError::Other)?;
    let result = RecordingResult {
        duration_ms: pcm.duration_ms,
        sample_rate: pcm.sample_rate,
        num_samples: pcm.samples.len(),
    };
    *state.last_pcm.lock() = Some(pcm);
    Ok(result)
}
