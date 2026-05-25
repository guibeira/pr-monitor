use crate::error::{AppError, AppResult};
use crate::monitor::Monitor;
use crate::storage::Storage;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{window::Color, Manager};
use tauri_plugin_positioner::WindowExt;

pub struct AppState {
    pub storage: Storage,
    pub monitor: Monitor,
}

impl AppState {
    pub fn new(db_path: std::path::PathBuf) -> AppResult<Self> {
        Ok(Self {
            storage: Storage::new(db_path)?,
            monitor: Monitor::default(),
        })
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Focused(false) = event {
                let _ = window.hide();
            }
        })
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_positioner::init())
        .setup(|app| {
            let window = app
                .get_webview_window("main")
                .ok_or(AppError::WindowNotFound("main"))?;

            #[cfg(target_os = "macos")]
            window.set_background_color(Some(Color(0, 0, 0, 0)))?;

            let app_handle = app.handle().clone();
            let app_data_dir = app_handle.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let db_path = app_data_dir.join("monitor.db");
            app.manage(AppState::new(db_path)?);

            let quit = MenuItemBuilder::new("Quit").id("quit").build(app)?;
            let menu = MenuBuilder::new(app).items(&[&quit]).build()?;

            let mut tray_builder = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id().as_ref() == "quit" {
                        app.exit(0)
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    let app = tray.app_handle();
                    tauri_plugin_positioner::on_tray_event(app.app_handle(), &event);
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(window) = app.get_webview_window("main") {
                            if !window.is_visible().unwrap_or(false) {
                                let _ = window.move_window(
                                    tauri_plugin_positioner::Position::TrayBottomCenter,
                                );
                                let _ = window.show();
                                let _ = window.set_focus();
                            } else {
                                let _ = window.hide();
                            }
                        }
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            let _ = tray_builder.build(app)?;

            Ok(())
        })
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            crate::commands::add_item,
            crate::commands::start_task,
            crate::commands::stop_task,
            crate::commands::emit_event,
            crate::commands::get_pr_list,
            crate::commands::has_token,
            crate::commands::add_token,
            crate::commands::get_all_prs,
            crate::commands::get_refresh_time,
            crate::commands::set_refresh_time,
            crate::commands::delete_pr,
            crate::commands::get_show_notification,
            crate::commands::set_show_notification,
            crate::commands::get_theme,
            crate::commands::set_theme
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
