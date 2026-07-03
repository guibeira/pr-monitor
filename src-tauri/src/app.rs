use crate::credentials::CredentialStore;
use crate::diagnostics;
use crate::error::{AppError, AppResult};
use crate::monitor::Monitor;
use crate::storage::Storage;
use log::LevelFilter;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::plugin::TauriPlugin;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{window::Color, Manager, PhysicalPosition, PhysicalSize, Rect, Runtime, WebviewWindow};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

pub struct AppState {
    pub credentials: CredentialStore,
    pub storage: Storage,
    pub monitor: Monitor,
}

impl AppState {
    pub fn new(db_path: std::path::PathBuf) -> AppResult<Self> {
        Ok(Self {
            credentials: CredentialStore::default(),
            storage: Storage::new(db_path)?,
            monitor: Monitor::default(),
        })
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    diagnostics::install_panic_hook();

    let run_result = tauri::Builder::default()
        .plugin(log_plugin())
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
            log::info!("App initialized");
            log::info!("Log directory: {}", diagnostics::app_log_dir().display());

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
                        rect,
                        ..
                    } = event
                    {
                        if let Some(window) = app.get_webview_window("main") {
                            if !window.is_visible().unwrap_or(false) {
                                if let Err(err) = move_window_to_tray_bottom_center(&window, &rect)
                                {
                                    log::warn!("Failed to position tray window: {err}");
                                }
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
        .run(tauri::generate_context!());

    if let Err(err) = run_result {
        log::error!("error while running tauri application: {err}");
        diagnostics::record_fatal_error("error while running tauri application", &err);
        eprintln!("error while running tauri application: {err}");
        std::process::exit(1);
    }
}

fn log_plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri_plugin_log::Builder::new()
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir { file_name: None }),
        ])
        .rotation_strategy(RotationStrategy::KeepAll)
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .max_file_size(1_000_000)
        .level(LevelFilter::Info)
        .build()
}

fn move_window_to_tray_bottom_center<R: Runtime>(
    window: &WebviewWindow<R>,
    rect: &Rect,
) -> tauri::Result<()> {
    let tray_position: PhysicalPosition<f64> = rect.position.to_physical(1.0);
    let tray_size: PhysicalSize<f64> = rect.size.to_physical(1.0);
    let window_size = window.outer_size()?;

    let x = tray_position.x + (tray_size.width / 2.0) - (window_size.width as f64 / 2.0);
    let y = tray_position.y;

    window.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32))
}
