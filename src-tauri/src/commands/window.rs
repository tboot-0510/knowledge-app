//! Window + scheduling commands that delegate to the platform layer.

use crate::platform;
use crate::state::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn show_daily_panel(app: AppHandle) -> Result<(), String> {
    platform::show_daily_panel(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hide_panel(app: AppHandle) -> Result<(), String> {
    platform::hide_panel(&app).map_err(|e| e.to_string())
}

/// Persist the daily schedule hour and (on macOS) install/remove the LaunchAgent.
#[tauri::command]
pub fn set_daily_schedule(
    app: AppHandle,
    state: State<AppState>,
    hour: u8,
    enabled: bool,
) -> Result<(), String> {
    state
        .db
        .lock()
        .unwrap()
        .set_setting("schedule_hour", &hour.to_string())
        .map_err(|e| e.to_string())?;
    platform::set_daily_schedule(&app, hour, enabled).map_err(|e| e.to_string())
}
