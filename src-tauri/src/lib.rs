#![allow(dead_code)]

mod audio;
mod commands;
mod db;
mod error;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::auth::unlock_vault,
            commands::setup::probe_system,
            commands::setup::download_profile,
            commands::visits::list_visits,
            commands::visits::get_visit,
            commands::visits::create_visit,
            commands::visits::log_consent,
            commands::visits::save_visit,
            commands::audio::start_recording,
            commands::audio::stop_recording,
            commands::transcribe::transcribe,
            commands::summarize::summarize,
            commands::icd10::search_icd10,
            commands::export::export_pdf,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::get_cost_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running rpstr");
}
