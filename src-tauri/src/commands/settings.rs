//! Settings + Ollama status commands.

use crate::state::AppState;
use knowledge_core::models::Settings;
use tauri::State;

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    state
        .db
        .lock()
        .unwrap()
        .get_settings()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_settings(state: State<AppState>, settings: Settings) -> Result<(), String> {
    state
        .db
        .lock()
        .unwrap()
        .save_settings(&settings)
        .map_err(|e| e.to_string())
}

/// List models available to the local Ollama server.
#[tauri::command]
pub async fn list_ollama_models(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state.ollama().list_models().await.map_err(|e| e.to_string())
}

/// True if the local Ollama server is reachable.
#[tauri::command]
pub async fn check_ollama_health(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.ollama().health().await.is_ok())
}
