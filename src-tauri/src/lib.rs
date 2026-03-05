mod ai;
mod commands;
mod config;
mod db;
mod input_monitor;
mod tools;

use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

use commands::{DbState, ForegroundTitleState};
use config::AppConfig;
use db::Database;

#[cfg(target_os = "windows")]
fn get_foreground_window_title() -> String {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return String::new();
        }
        let mut buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut buf);
        if len > 0 {
            String::from_utf16_lossy(&buf[..len as usize])
        } else {
            String::new()
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn get_foreground_window_title() -> String {
    "Unknown".to_string()
}

pub(crate) fn toggle_overlay(app: &tauri::AppHandle) {
    // Capture foreground window title before showing overlay
    if let Some(fg_state) = app.try_state::<ForegroundTitleState>() {
        let title = get_foreground_window_title();
        if let Ok(mut t) = fg_state.title.lock() {
            *t = title;
        }
    }

    let window = app.get_webview_window("overlay").unwrap();
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // Resolve directories
            let config_dir = app.path().app_config_dir().expect("Failed to get config dir");
            let data_dir = app.path().app_data_dir().expect("Failed to get data dir");

            // Load or create config (backward-compatible, reads legacy fields)
            let (app_config, config_path) = AppConfig::load_or_create(&config_dir, &data_dir)?;

            // Initialize database (migration v5 creates settings table)
            let database = Database::initialize(&app_config.db_path)?;

            // Seed defaults from legacy config values (INSERT OR IGNORE — won't overwrite)
            database.seed_defaults_from_config(
                app_config.hotkey.as_deref(),
                app_config.ai_provider.as_deref(),
                app_config.ai_api_key.as_deref(),
                app_config.ai_base_url.as_deref(),
                app_config.task_archive_delay_secs,
            )?;

            // Rewrite config.toml to just db_path (strip legacy fields)
            AppConfig::rewrite_minimal(&config_path, &app_config.db_path)?;

            // Start input monitoring (uses its own DB connection via WAL)
            input_monitor::start_monitoring(app_config.db_path.clone());

            // Read hotkey from DB
            let hotkey = database
                .get_setting("hotkey", "default")?
                .unwrap_or_else(|| "Ctrl+Shift+Space".to_string());

            // Register managed state (no ConfigState — settings live in DB now)
            app.manage(DbState {
                db: Mutex::new(database),
            });
            app.manage(ForegroundTitleState {
                title: Mutex::new(String::new()),
            });

            // Register global shortcut from DB value
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            app.global_shortcut().on_shortcut(hotkey.as_str(), |app, _shortcut, event| {
                if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                    toggle_overlay(app);
                }
            })?;

            // Build system tray
            let show_item = MenuItem::with_id(app, "show", "Show UI", true, None::<&str>)?;
            let exit_item = MenuItem::with_id(app, "exit", "Exit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &exit_item])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => toggle_overlay(app),
                    "exit" => app.exit(0),
                    _ => {}
                })
                .tooltip("aiHelper")
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::get_setting,
            commands::update_setting,
            commands::update_hotkey,
            commands::insert_event,
            commands::get_recent_events,
            commands::send_chat_message,
            commands::create_conversation,
            commands::list_conversations,
            commands::delete_conversation,
            commands::load_conversation,
            commands::save_user_message,
            commands::set_active_child,
            commands::set_conversation_active_root,
            commands::create_task,
            commands::list_tasks,
            commands::update_task_completed,
            commands::archive_task,
            commands::delete_task,
            commands::create_note,
            commands::list_notes,
            commands::get_note,
            commands::update_note,
            commands::delete_note,
            commands::get_foreground_title,
            commands::get_input_stats,
            commands::get_recent_input,
            commands::get_top_windows,
            commands::create_project,
            commands::update_project,
            commands::delete_project,
            commands::list_projects,
            commands::add_project_rule,
            commands::delete_project_rule,
            commands::get_project_rules,
            commands::get_all_rules,
            commands::get_untagged_activity,
            commands::get_untagged_summary,
            commands::get_project_activities,
            commands::tag_activities,
            commands::clear_project_tags,
            commands::suggest_projects,
            commands::smart_suggest_projects,
            commands::get_all_project_summaries_today,
            commands::generate_tip,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
