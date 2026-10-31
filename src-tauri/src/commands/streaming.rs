use crate::commands::network::AppState;
use crate::streaming::detector::StreamingConfig;
use tauri::State;

#[tauri::command]
pub fn get_streaming_config(state: State<'_, AppState>) -> StreamingConfig {
    state.streaming_detector.get_config()
}

#[tauri::command]
pub fn add_user_streaming_app(state: State<'_, AppState>, app_name: String) -> Result<StreamingConfig, String> {
    let clean = app_name.trim();
    if clean.is_empty() {
        return Err("Application name cannot be empty".to_string());
    }

    state.streaming_detector.add_user_app(clean);
    let _ = state.db.add_streaming_app(clean);

    Ok(state.streaming_detector.get_config())
}

#[tauri::command]
pub fn remove_user_streaming_app(state: State<'_, AppState>, app_name: String) -> Result<StreamingConfig, String> {
    let clean = app_name.trim();
    state.streaming_detector.remove_user_app(clean);
    let _ = state.db.remove_streaming_app(clean);

    Ok(state.streaming_detector.get_config())
}

