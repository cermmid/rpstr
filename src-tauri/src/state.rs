use std::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    pub unlocked: Mutex<bool>,
}
