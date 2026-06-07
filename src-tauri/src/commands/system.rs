//! System integration commands: global shortcut (re)registration and native
//! reminder notifications.
//!
//! These rely on the `global-shortcut` and `notification` plugins and are
//! desktop-only; the parsing/registration calls depend on the plugin versions
//! and may need minor adjustment when built on macOS.

use crate::platform;
use crate::state::AppState;
use knowledge_core::scheduler;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_notification::NotificationExt;

/// (Re)register the global shortcut that summons the popup. Unregisters any
/// previous shortcuts first. Invalid accelerators are ignored.
pub fn register_shortcut(app: &AppHandle, accelerator: &str) -> tauri::Result<()> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    if let Ok(shortcut) = accelerator.parse::<tauri_plugin_global_shortcut::Shortcut>() {
        let _ = gs.register(shortcut);
    }
    Ok(())
}

/// Handler invoked when the global shortcut fires: show the panel on the daily view.
pub fn on_global_shortcut(app: &AppHandle) {
    let _ = platform::show_daily_panel(app);
    let _ = app.emit("navigate", "daily");
}

/// Persist and re-register a new global shortcut accelerator.
#[tauri::command]
pub fn set_global_shortcut(
    app: AppHandle,
    state: State<AppState>,
    accelerator: String,
) -> Result<(), String> {
    state
        .db
        .lock()
        .unwrap()
        .set_setting("global_shortcut", &accelerator)
        .map_err(|e| e.to_string())?;
    register_shortcut(&app, &accelerator).map_err(|e| e.to_string())
}

/// Send a reminder notification if today's challenge isn't done yet (and
/// reminders are enabled). Called by the frontend on launch and can be wired to
/// the daily launchd trigger. Returns true if a notification was sent.
#[tauri::command]
pub fn send_reminder_if_due(app: AppHandle, state: State<AppState>) -> Result<bool, String> {
    let (enabled, completed_today, streak, due_reviews) = {
        let db = state.db.lock().unwrap();
        let settings = db.get_settings().map_err(|e| e.to_string())?;
        let today = scheduler::today_local();
        let completed = db
            .get_session_by_date(&today)
            .map_err(|e| e.to_string())?
            .map(|s| s.completed)
            .unwrap_or(false);
        let streak = db.get_streak().map_err(|e| e.to_string())?;
        let due = db.count_due_reviews(&today).map_err(|e| e.to_string())?;
        (settings.reminders_enabled, completed, streak, due)
    };

    if !enabled || completed_today {
        return Ok(false);
    }

    let body = if streak.current_streak > 0 {
        format!(
            "Your {}-day streak is at risk — finish today's challenge to keep it going.{}",
            streak.current_streak,
            if due_reviews > 0 {
                format!(" {due_reviews} reviews due.")
            } else {
                String::new()
            }
        )
    } else {
        "A fresh set of interview-style questions is ready for you.".to_string()
    };

    app.notification()
        .builder()
        .title("Today's challenge is waiting")
        .body(body)
        .show()
        .map_err(|e| e.to_string())?;
    Ok(true)
}
