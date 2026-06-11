//! macOS platform integration: dock-hiding (Accessory policy), NSPanel overlay,
//! and launchd-based daily scheduling.
//!
//! NOTE: this module compiles only on macOS and is therefore NOT built in the
//! Linux development/CI environment. The NSPanel tuning calls depend on the
//! pinned `tauri-nspanel` revision; if its API differs, adjust [`convert_to_panel`]
//! accordingly. Everything here is best-effort: failures degrade to a normal
//! window rather than crashing.

use knowledge_core::scheduler::launchd_plist;
use tauri::{App, AppHandle, Manager};

const LAUNCH_AGENT_LABEL: &str = "com.knowledgeapp.app";

/// One-time setup: show the main window on launch.
///
/// NOTE: earlier builds hid the Dock icon (Accessory policy) and converted the
/// window to a hidden NSPanel, which made the installed app appear "not to
/// open." We now keep a normal, reliably-visible window plus a menu-bar icon
/// (see `build_tray` in lib.rs) for quick access.
pub fn setup(app: &mut App) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
    Ok(())
}

/// Bring the main window to the foreground, centered.
pub fn show_daily_panel(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
    Ok(())
}

/// Hide the window without quitting the app.
pub fn hide_panel(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    Ok(())
}

/// Install or remove a launchd LaunchAgent that opens the app daily at `hour`.
pub fn set_daily_schedule(_app: &AppHandle, hour: u8, enabled: bool) -> tauri::Result<()> {
    let Ok(home) = std::env::var("HOME") else {
        return Ok(());
    };
    let agents_dir = std::path::Path::new(&home).join("Library/LaunchAgents");
    let plist_path = agents_dir.join(format!("{LAUNCH_AGENT_LABEL}.plist"));

    if !enabled {
        let _ = std::process::Command::new("launchctl")
            .arg("unload")
            .arg(&plist_path)
            .status();
        let _ = std::fs::remove_file(&plist_path);
        return Ok(());
    }

    std::fs::create_dir_all(&agents_dir)?;
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    std::fs::write(&plist_path, launchd_plist(LAUNCH_AGENT_LABEL, &exe, hour))?;
    let _ = std::process::Command::new("launchctl")
        .arg("load")
        .arg(&plist_path)
        .status();
    Ok(())
}
