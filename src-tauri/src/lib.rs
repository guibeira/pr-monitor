use octocrab::models::pulls::MergeableState;
use regex::Regex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{window::Color, Emitter, Manager, State, Wry};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_positioner::WindowExt;
use tokio::sync::Mutex as TokioMutex;

use log::{error, info, warn};
use octocrab::Octocrab;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug, Serialize, Deserialize)]
struct PullRequestModel {
    owner: String,
    repo: String,
    pr_number: u64,
    title: String,
    state: String,
    closed_at: String,
    url: String,
}

async fn get_pr_details(
    token: String,
    owner: &String,
    repo: &String,
    pr_number: &String,
) -> Result<PullRequestModel, String> {
    let pr_number: u64 = pr_number.parse().unwrap();
    if let Ok(octocrab) = Octocrab::builder().personal_token(token).build() {
        let pull_request = octocrab.pulls(owner, repo).get(pr_number).await;
        if let Ok(pr) = pull_request {
            log::info!("{:#?}", pr);
            if pr.mergeable_state.is_some() {
                let closed_at = pr.closed_at;
                let mut closed_str = "".to_string();
                if closed_at.is_some() {
                    closed_str = closed_at.unwrap().format("%d/%m/%Y %H:%M").to_string();
                }
                let state = match pr.state {
                    Some(state) => match state {
                        octocrab::models::IssueState::Open => "open",
                        octocrab::models::IssueState::Closed => "closed",
                        _ => "",
                    },
                    None => "",
                };

                return Ok(PullRequestModel {
                    owner: owner.clone(),
                    repo: repo.clone(),
                    pr_number,
                    title: pr.title.unwrap_or("".to_string()),
                    state: state.to_string(),
                    closed_at: closed_str,
                    url: pr.url.to_string(),
                });
            }
        } else {
            error!("Error: {:?}", pull_request);
            return Err("Can't load pr details".to_string());
        }
    }
    Err("Can't load pr details".to_string())
}

enum PrStatus {
    Merged,
    Behind,
    UpToDate,
    Conflicts,
    Blocked,
    Unknown,
}

async fn needs_update_pr(owner: String, repo: String, pr_number: u64, token: String) -> PrStatus {
    if let Ok(octocrab) = Octocrab::builder().personal_token(token).build() {
        let pull_request = octocrab.pulls(owner, repo).get(pr_number).await;
        if let Ok(pr) = pull_request {
            if pr.merged_at.is_some() {
                log::info!("PR was merged, we not need to update the branch");
                return PrStatus::Merged;
            }

            if let Some(mergeable_state) = pr.mergeable_state {
                log::info!("Mergeable state: {:?}", mergeable_state);
                match mergeable_state {
                    MergeableState::Behind => {
                        log::info!("PR is behind, we need to update the branch");
                        return PrStatus::Behind;
                    }
                    _ => match mergeable_state {
                        MergeableState::Clean => return PrStatus::UpToDate,
                        MergeableState::Dirty => return PrStatus::Conflicts,
                        MergeableState::Unknown => return PrStatus::Unknown,
                        MergeableState::Blocked => return PrStatus::Blocked,
                        MergeableState::Unstable => return PrStatus::Unknown,
                        _ => return PrStatus::Unknown,
                    },
                }
            }
        } else {
            log::error!("Error: {:?}", pull_request);
            return PrStatus::Unknown;
        }
    } else {
        log::error!("Failed to create Octocrab instance");
        return PrStatus::Unknown;
    };
    log::error!("Failed to get PR details");
    PrStatus::Unknown
}

async fn update_pr_branch(
    owner: &str,
    repo: &str,
    pr_number: u64,
    token: &str,
) -> Result<(), String> {
    let owner = owner.to_string();
    let repo = repo.to_string();
    if let Ok(octocrab) = Octocrab::builder().personal_token(token).build() {
        let pull_request = octocrab.pulls(owner, repo).update_branch(pr_number).await;
        if let Ok(issue) = pull_request {
            log::info!("{:#?}", issue);
            Ok(())
        } else {
            log::error!("Error: {:?}", pull_request);
            Err("Can't update pr branch".to_string())
        }
    } else {
        log::error!("Failed to create Octocrab instance");
        Err("Can't update pr branch".to_string())
    }
}

#[tauri::command]
fn emit_event(app_handle: tauri::AppHandle<Wry>) {
    app_handle.emit("error-event", "Data from backend").unwrap();
}

#[tauri::command]
async fn start_task(
    app_handle: tauri::AppHandle<Wry>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.start_monitor(app_handle).await;
    Ok(())
}

#[tauri::command]
async fn get_pr_list(state: State<'_, AppState>) -> Result<Vec<PullRequestModel>, String> {
    log::info!("Getting all PRs");
    Ok(state.get_all_prs())
}

#[tauri::command]
async fn stop_task(state: State<'_, AppState>) -> Result<(), String> {
    state.stop_monitor().await;
    Ok(())
}

#[tauri::command]
async fn has_token(state: State<'_, AppState>) -> Result<bool, String> {
    let db = state.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT count(id) FROM token").unwrap();
    let count_iter = stmt
        .query_map(params![], |row| row.get(0))
        .unwrap()
        .map(|r| r.unwrap());

    let count: i32 = count_iter.collect::<Vec<i32>>()[0];
    Ok(count > 0)
}

#[tauri::command]
async fn add_token(state: State<'_, AppState>, token: String) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db.execute("DELETE FROM token", params![]).unwrap();
    db.execute("INSERT INTO token (key) VALUES (?)", params![token])
        .unwrap();
    Ok(())
}

#[tauri::command]
async fn add_item(
    state: State<'_, AppState>,
    url: String,
) -> Result<Vec<PullRequestModel>, String> {
    info!("Adding item: {}", url);

    if let Some((owner, repo, pr_number)) = parse_github_pr_url(&url) {
        info!(
            "Parsed PR: owner={}, repo={}, pr_number={}",
            owner, repo, pr_number
        );
        let token = state.get_token();
        if token.is_none() {
            return Err("There is no token, can't check pr details".to_string());
        }
        let token = token.unwrap();
        let pr_status = get_pr_details(token, &owner, &repo, &pr_number).await;
        if pr_status.is_err() {
            let error_msg = pr_status.unwrap_err();
            return Err(error_msg);
        }
        let pull_request = pr_status.unwrap();
        state.add_item(pull_request)?;
        Ok(state.get_all_prs())
    } else {
        warn!("Failed to parse PR: {}", url);
        Err("Failed to parse PR".to_string())
    }
}

#[tauri::command]
async fn get_all_prs(state: State<'_, AppState>) -> Result<Vec<PullRequestModel>, String> {
    log::info!("Getting all PRs");
    Ok(state.get_all_prs())
}

#[tauri::command]
async fn get_refresh_time(state: State<'_, AppState>) -> Result<u64, String> {
    Ok(state.get_refresh_time())
}

#[tauri::command]
async fn set_refresh_time(
    app_handle: tauri::AppHandle<Wry>,
    state: State<'_, AppState>,
    time_in_minutes: u64,
) -> Result<(), String> {
    let time_in_seconds = time_in_minutes * 60;
    state.set_refresh_time(time_in_seconds);
    state.stop_monitor().await;
    state.start_monitor(app_handle).await;
    Ok(())
}

#[tauri::command]
async fn delete_pr(state: State<'_, AppState>, pr_number: u64) -> Result<(), String> {
    state.delete_pr(pr_number)
}

#[tauri::command]
async fn get_show_notification(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.get_show_notification())
}

#[tauri::command]
async fn set_show_notification(state: State<'_, AppState>, show: bool) -> Result<(), String> {
    state.set_show_notification(show);
    Ok(())
}

#[tauri::command]
async fn get_theme(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.get_theme())
}

#[tauri::command]
async fn set_theme(state: State<'_, AppState>, theme: String) -> Result<(), String> {
    state.set_theme(theme);
    Ok(())
}

fn parse_github_pr_url(url: &str) -> Option<(String, String, String)> {
    let re = Regex::new(r"github\.com/([^/]+)/([^/]+)/pull/(\d+)").unwrap();
    if let Some(caps) = re.captures(url) {
        let owner = caps.get(1)?.as_str().to_string();
        let repo = caps.get(2)?.as_str().to_string();
        let pr_number = caps.get(3)?.as_str().to_string();
        Some((owner, repo, pr_number))
    } else {
        None
    }
}

struct AppState {
    db: Arc<Mutex<Connection>>,
    running: Arc<TokioMutex<bool>>,
}

impl AppState {
    fn new(db_path: std::path::PathBuf) -> Self {
        let conn = Connection::open(db_path).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pull_request (
                id INTEGER PRIMARY KEY,
                owner TEXT NOT NULL,
                repo TEXT NOT NULL,
                pr_number INTEGER NOT NULL,
                title TEXT NOT NULL,
                state TEXT NOT NULL,
                url TEXT NOT NULL,
                closed_at TEXT
            );",
            params![],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
            params![],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS token (
                id INTEGER PRIMARY KEY,
                key TEXT NOT NULL
            );",
            params![],
        )
        .unwrap();

        Self {
            db: Arc::new(Mutex::new(conn)),
            running: Arc::new(TokioMutex::new(false)),
        }
    }

    fn get_theme(&self) -> String {
        let db = self.db.lock().unwrap();
        let mut stmt = db
            .prepare("SELECT value FROM settings WHERE key = 'theme'")
            .unwrap();
        let theme_iter = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap());
        let theme: Option<String> = theme_iter.collect::<Vec<String>>().pop();
        theme.unwrap_or("system".to_string())
    }

    fn set_theme(&self, theme: String) {
        let db = self.db.lock().unwrap();
        db.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('theme', ?)",
            params![theme],
        )
        .unwrap();
    }

    fn get_refresh_time(&self) -> u64 {
        let db = self.db.lock().unwrap();
        let mut stmt = db
            .prepare("SELECT value FROM settings WHERE key = 'refresh_time'")
            .unwrap();
        let time_iter = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap());
        let time: Option<String> = time_iter.collect::<Vec<String>>().pop();
        time.and_then(|t| t.parse::<u64>().ok()).unwrap_or(300)
    }

    fn set_refresh_time(&self, time_in_seconds: u64) {
        let db = self.db.lock().unwrap();
        db.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('refresh_time', ?)",
            params![time_in_seconds.to_string()],
        )
        .unwrap();
    }

    fn get_show_notification(&self) -> bool {
        let db = self.db.lock().unwrap();
        let mut stmt = db
            .prepare("SELECT value FROM settings WHERE key = 'show_notification'")
            .unwrap();
        let show_iter = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap());
        let show: Option<String> = show_iter.collect::<Vec<String>>().pop();
        show.and_then(|s| s.parse::<bool>().ok()).unwrap_or(true)
    }

    fn set_show_notification(&self, show: bool) {
        let db = self.db.lock().unwrap();
        db.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('show_notification', ?)",
            params![show.to_string()],
        )
        .unwrap();
    }

    fn get_token(&self) -> Option<String> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare("SELECT key FROM token").unwrap();

        let item_iter = stmt
            .query_map(params![], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap());
        item_iter.collect::<Vec<String>>().pop()
    }

    async fn start_monitor(&self, app_handle: tauri::AppHandle<Wry>) {
        let token = self.get_token();
        if token.is_none() {
            info!("Token not found");
            return;
        }

        let is_running = self.running.clone();
        let mut running = is_running.lock().await;
        if *running {
            info!("Task is already running!");
            return;
        }
        *running = true;
        drop(running);

        info!("Starting monitor PRs");
        let db = self.db.clone();
        let running = self.running.clone();
        let token = token.unwrap();
        let refresh_time_secs = self.get_refresh_time();

        tokio::spawn(async move {
            let refresh_duration = std::time::Duration::from_secs(refresh_time_secs);
            loop {
                info!(
                    "Running task with refresh time: {} seconds",
                    refresh_time_secs
                );
                let running_guard = running.lock().await;
                if !*running_guard {
                    info!("Task stopped!");
                    break;
                }

                let show_notification = {
                    let db = db.lock().unwrap();
                    let mut stmt = db
                        .prepare("SELECT value FROM settings WHERE key = 'show_notification'")
                        .unwrap();
                    let show_iter = stmt
                        .query_map([], |row| row.get(0))
                        .unwrap()
                        .map(|r| r.unwrap());
                    let show: Option<String> = show_iter.collect::<Vec<String>>().pop();
                    show.and_then(|s| s.parse::<bool>().ok()).unwrap_or(true)
                };

                let get_all_pull_request = {
                    let db = db.lock().expect("Failed to lock db");
                    let mut stmt = db
                        .prepare(
                            "SELECT owner, repo, pr_number, title, state, closed_at, url FROM pull_request where state = 'open'",
                        )
                        .unwrap();
                    let pull_request_iter = stmt
                        .query_map([], |row| {
                            let owner: String = row.get(0).unwrap_or_else(|_| "".to_string());
                            let repo: String = row.get(1).unwrap_or_else(|_| "".to_string());
                            let pr_number: u64 = row.get(2).unwrap_or(1);
                            let title: String = row.get(3).unwrap_or_else(|_| "".to_string());
                            let state: String = row.get(4).unwrap_or_else(|_| "".to_string());
                            let closed_at: String = row.get(5).unwrap_or_else(|_| "".to_string());
                            let url: String = row.get(6).unwrap_or_else(|_| "".to_string());

                            Ok(PullRequestModel {
                                owner,
                                repo,
                                pr_number,
                                title,
                                state,
                                closed_at,
                                url,
                            })
                        })
                        .unwrap();

                    pull_request_iter.filter_map(Result::ok).collect::<Vec<_>>()
                };

                for pr in get_all_pull_request {
                    if !*running_guard {
                        info!("Task stopped during processing!");
                        break;
                    }

                    let pr_status = needs_update_pr(
                        pr.owner.clone(),
                        pr.repo.clone(),
                        pr.pr_number,
                        token.clone(),
                    )
                    .await;

                    match pr_status {
                        PrStatus::Merged => {
                            info!("PR was merged, updating status");
                            let db = db.lock().unwrap();
                            db.execute(
                                "UPDATE pull_request SET state = 'closed' WHERE pr_number = ?",
                                params![pr.pr_number],
                            )
                            .unwrap();
                            app_handle.emit("pr-closed", pr.pr_number).unwrap();
                        }
                        PrStatus::Behind => {
                            info!("PR is behind, updating branch");
                            if let Err(e) =
                                update_pr_branch(&pr.owner, &pr.repo, pr.pr_number, &token).await
                            {
                                error!("Failed to update PR branch: {}", e);
                                if show_notification {
                                    app_handle
                                        .notification()
                                        .builder()
                                        .title("Failed to update PR")
                                        .body(&e)
                                        .show()
                                        .expect("Failed to show notification");
                                }
                            }
                        }
                        PrStatus::Conflicts | PrStatus::Blocked | PrStatus::Unknown => {
                            if show_notification {
                                let status_str = match pr_status {
                                    PrStatus::Conflicts => "has conflicts",
                                    PrStatus::Blocked => "is blocked",
                                    _ => "has an unknown status",
                                };
                                let title = format!("PR Not Updated: #{}", pr.pr_number);
                                let body = format!("PR {} - please check.", status_str);
                                app_handle
                                    .notification()
                                    .builder()
                                    .title(&title)
                                    .body(&body)
                                    .show()
                                    .expect("Failed to show notification");
                            }
                        }
                        PrStatus::UpToDate => {
                            info!("PR is up to date");
                        }
                    }
                }
                drop(running_guard);
                tokio::time::sleep(refresh_duration).await;
            }
        });
    }

    async fn stop_monitor(&self) {
        let mut running = self.running.lock().await;
        *running = false;
    }

    fn add_item(&self, pull_request: PullRequestModel) -> Result<(), String> {
        let db = self.db.lock().unwrap();
        let mut stmt = db
            .prepare("SELECT count(id) FROM pull_request WHERE pr_number = ?")
            .unwrap();

        let count: i32 = stmt
            .query_row(params![&pull_request.pr_number], |row| row.get(0))
            .unwrap_or(0);

        if count > 0 {
            return Err("Pull request already exists".to_string());
        }
        db.execute(
            "
            INSERT INTO pull_request (owner, repo, pr_number, title, state, url, closed_at)
            VALUES (?,?,?,?,?,?,?)",
            params![
                pull_request.owner,
                pull_request.repo,
                pull_request.pr_number,
                pull_request.title,
                pull_request.state,
                pull_request.url,
                pull_request.closed_at
            ],
        )
        .unwrap();
        Ok(())
    }

    fn delete_pr(&self, pr_number: u64) -> Result<(), String> {
        let db = self.db.lock().unwrap();
        db.execute(
            "DELETE FROM pull_request WHERE pr_number = ?",
            params![pr_number],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn get_all_prs(&self) -> Vec<PullRequestModel> {
        let db = self.db.lock().unwrap();
        let mut stmt = db
            .prepare(
                "SELECT owner, repo, pr_number, title, state, closed_at, url FROM pull_request",
            )
            .unwrap();
        let pull_request_iter = stmt
            .query_map([], |row| {
                Ok(PullRequestModel {
                    owner: row.get(0)?,
                    repo: row.get(1)?,
                    pr_number: row.get(2)?,
                    title: row.get(3)?,
                    state: row.get(4)?,
                    closed_at: row.get(5)?,
                    url: row.get(6)?,
                })
            })
            .unwrap();

        pull_request_iter.filter_map(Result::ok).collect()
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
            let window = app.get_webview_window("main").unwrap();
            #[cfg(target_os = "macos")]
            window.set_background_color(Some(Color(0, 0, 0, 0)))?;

            let app_handle = app.handle().clone();
            let app_data_dir = app_handle.path().app_data_dir().unwrap();
            if !app_data_dir.exists() {
                std::fs::create_dir_all(&app_data_dir).unwrap();
            }
            let db_path = app_data_dir.join("monitor.db");
            app.manage(AppState::new(db_path));

            let quit = MenuItemBuilder::new("Quit").id("quit").build(app)?;
            let menu = MenuBuilder::new(app).items(&[&quit]).build()?;
            let _ = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| if event.id().as_ref() == "quit" { app.exit(0) })
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
                })
                .build(app)?;

            Ok(())
        })
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            add_item,
            start_task,
            stop_task,
            emit_event,
            get_pr_list,
            has_token,
            add_token,
            get_all_prs,
            get_refresh_time,
            set_refresh_time,
            delete_pr,
            get_show_notification,
            set_show_notification,
            get_theme,
            set_theme
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
