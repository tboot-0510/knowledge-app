//! Non-macOS fallback for the platform layer.
//!
//! Used when building/running on Linux or Windows (e.g. for development with a
//! webkit/webview2 backend). Window show/hide still work via the standard Tauri
//! API; the macOS-only overlay and launchd scheduling are no-ops.

use tauri::{App, AppHandle, Manager};

/// One-time platform setup at app launch. Nothing special off macOS.
pub fn setup(_app: &App) -> tauri::Result<()> {
    Ok(())
}

/// Bring the daily-challenge window to the foreground.
pub fn show_daily_panel(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
    Ok(())
}

/// Hide the window (off macOS we just hide the normal window).
pub fn hide_panel(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    Ok(())
}

/// Install/remove the daily launch schedule. No-op off macOS.
pub fn set_daily_schedule(_app: &AppHandle, _hour: u8, _enabled: bool) -> tauri::Result<()> {
    Ok(())
}
