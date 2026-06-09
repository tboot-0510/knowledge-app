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

/// One-time setup: hide the dock icon and convert the main window to a floating
/// panel that can appear above other apps (mirrors thuki's overlay model).
pub fn setup(app: &mut App) -> tauri::Result<()> {
    // Accessory = no Dock icon, behaves like a menu-bar/overlay utility.
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    if let Some(window) = app.get_webview_window("main") {
        convert_to_panel(&window);
    }
    Ok(())
}

/// Convert the main window's NSWindow into a non-activating floating NSPanel.
fn convert_to_panel(window: &tauri::WebviewWindow) {
    use tauri_nspanel::WebviewWindowExt;
    match window.to_panel() {
        Ok(_panel) => {
            // Optional tuning to make the panel float above other apps and be
            // available on all spaces / over fullscreen apps. The exact setters
            // depend on the tauri-nspanel revision; enable as needed:
            //
            //   _panel.set_level(NSFloatingWindowLevel);
            //   _panel.set_collection_behaviour(
            //       CanJoinAllSpaces | FullScreenAuxiliary);
            //   _panel.set_style_mask(NSWindowStyleMaskNonactivatingPanel);
        }
        Err(e) => {
            tracing::warn!("failed to convert window to NSPanel, using normal window: {e:?}");
        }
    }
}

/// Bring the daily-challenge panel to the foreground, centered.
pub fn show_daily_panel(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.center();
        let _ = w.show();
        let _ = w.set_focus();
    }
    Ok(())
}

/// Order the panel out (hide) without quitting the app.
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
