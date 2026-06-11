//! Tauri application wiring: state, plugins, tray, and command registration.

mod commands;
mod platform;
mod state;

use state::AppState;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::MacosLauncher;

/// Show or hide the main window (used by the menu-bar icon's left click).
fn toggle_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}

/// Build the menu-bar (status-bar) icon: left-click toggles the window, and a
/// right-click menu provides quick navigation + quit. Mirrors thuki's quick
/// access from the menu bar.
fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Knowledge", true, None::<&str>)?;
    let open_today = MenuItem::with_id(app, "open_today", "Today's Challenge", true, None::<&str>)?;
    let open_ask = MenuItem::with_id(app, "open_ask", "Ask the web", true, None::<&str>)?;
    let open_settings = MenuItem::with_id(app, "open_settings", "Settings", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&open, &open_today, &open_ask, &open_settings, &sep, &quit],
    )?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("Knowledge")
        .menu(&menu)
        // Left click toggles the window; the menu opens on right click.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                let _ = platform::show_daily_panel(app);
            }
            "open_today" => navigate(app, "daily"),
            "open_ask" => navigate(app, "ask"),
            "open_settings" => navigate(app, "settings"),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Show the window and ask the frontend to route to `route`.
fn navigate(app: &tauri::AppHandle, route: &str) {
    let _ = platform::show_daily_panel(app);
    let _ = app.emit("navigate", route);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .try_init();

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default();

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    builder
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        commands::system::on_global_shortcut(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            // Resolve the per-user data directory and open the database there.
            let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
            let db = knowledge_core::db::Db::open(&data_dir.join("knowledge.db"))
                .map_err(|e| format!("failed to open database: {e}"))?;

            // Register the user's global shortcut from saved settings.
            let shortcut = db
                .get_settings()
                .map(|s| s.global_shortcut)
                .unwrap_or_else(|_| "CmdOrCtrl+Shift+K".to_string());
            app.manage(AppState::new(db, data_dir));

            build_tray(app)?;
            let _ = commands::system::register_shortcut(app.handle(), &shortcut);

            // macOS: hide dock icon + convert main window to a floating panel.
            platform::setup(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::list_ollama_models,
            commands::settings::check_ollama_health,
            commands::daily::get_today_session,
            commands::daily::submit_answer,
            commands::daily::get_streak,
            commands::daily::get_progress,
            commands::daily::should_show_today,
            commands::daily::mark_shown_today,
            commands::topics::list_topics,
            commands::topics::set_topic_pref,
            commands::topics::add_custom_topic,
            commands::topics::delete_custom_topic,
            commands::topics::list_paths,
            commands::review::get_due_reviews,
            commands::review::count_due_reviews,
            commands::review::submit_review,
            commands::practice::generate_free_response,
            commands::practice::grade_free_response,
            commands::insight::generate_weakness_report,
            commands::coding::list_ds_categories,
            commands::coding::generate_coding_problem,
            commands::coding::review_solution,
            commands::focus::generate_focus_question,
            commands::focus::submit_focus_answer,
            commands::web::fetch_url,
            commands::web::web_search,
            commands::web::ask_web,
            commands::models::list_installed_models,
            commands::models::recommended_models,
            commands::models::pull_model,
            commands::models::delete_model,
            commands::system::set_global_shortcut,
            commands::system::send_reminder_if_due,
            commands::chat::ask_followup,
            commands::repo::link_repo,
            commands::repo::list_repos,
            commands::repo::get_repo,
            commands::repo::delete_repo,
            commands::chat::ask_repo,
            commands::window::show_daily_panel,
            commands::window::hide_panel,
            commands::window::set_daily_schedule,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
