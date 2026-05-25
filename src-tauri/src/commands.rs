use crate::app::AppState;
use crate::domain::{PullRequestKey, PullRequestModel};
use crate::error::{AppError, AppResult};
use crate::github::{get_pr_details, parse_github_pr_url};
use log::{info, warn};
use tauri::{Emitter, State, Wry};

fn into_command_error(err: impl std::fmt::Display) -> String {
    err.to_string()
}

#[tauri::command]
pub fn emit_event(app_handle: tauri::AppHandle<Wry>) -> Result<(), String> {
    app_handle
        .emit("error-event", "Data from backend")
        .map_err(into_command_error)
}

#[tauri::command]
pub async fn start_task(
    app_handle: tauri::AppHandle<Wry>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    start_monitor(app_handle, &state)
        .await
        .map_err(into_command_error)
}

#[tauri::command]
pub async fn stop_task(state: State<'_, AppState>) -> Result<(), String> {
    state.monitor.stop().await;
    Ok(())
}

#[tauri::command]
pub async fn get_pr_list(state: State<'_, AppState>) -> Result<Vec<PullRequestModel>, String> {
    info!("Getting all PRs");
    state
        .storage
        .get_all_pull_requests()
        .await
        .map_err(into_command_error)
}

#[tauri::command]
pub async fn get_all_prs(state: State<'_, AppState>) -> Result<Vec<PullRequestModel>, String> {
    info!("Getting all PRs");
    state
        .storage
        .get_all_pull_requests()
        .await
        .map_err(into_command_error)
}

#[tauri::command]
pub async fn has_token(state: State<'_, AppState>) -> Result<bool, String> {
    state
        .credentials
        .get_github_token()
        .await
        .map(|token| token.is_some())
        .map_err(into_command_error)
}

#[tauri::command]
pub async fn add_token(state: State<'_, AppState>, token: String) -> Result<(), String> {
    state
        .credentials
        .set_github_token(token)
        .await
        .map_err(into_command_error)
}

#[tauri::command]
pub async fn add_item(
    state: State<'_, AppState>,
    url: String,
) -> Result<Vec<PullRequestModel>, String> {
    add_item_inner(&state, &url)
        .await
        .map_err(into_command_error)
}

async fn add_item_inner(state: &AppState, url: &str) -> AppResult<Vec<PullRequestModel>> {
    info!("Adding item: {url}");

    let Some(key) = parse_github_pr_url(url) else {
        warn!("Failed to parse PR: {url}");
        return Err(AppError::InvalidPullRequestUrl);
    };

    info!(
        "Parsed PR: owner={}, repo={}, pr_number={}",
        key.owner, key.repo, key.pr_number
    );

    let token = state
        .credentials
        .get_github_token()
        .await?
        .ok_or(AppError::MissingToken)?;
    let pull_request = get_pr_details(&token, &key).await?;

    state.storage.add_pull_request(pull_request).await?;
    Ok(state.storage.get_all_pull_requests().await?)
}

#[tauri::command]
pub async fn get_refresh_time(state: State<'_, AppState>) -> Result<u64, String> {
    state
        .storage
        .get_refresh_time()
        .await
        .map_err(into_command_error)
}

#[tauri::command]
pub async fn set_refresh_time(
    app_handle: tauri::AppHandle<Wry>,
    state: State<'_, AppState>,
    time_in_minutes: u64,
) -> Result<(), String> {
    if time_in_minutes == 0 {
        return Err(into_command_error(AppError::InvalidRefreshTime));
    }

    let time_in_seconds = time_in_minutes * 60;
    state
        .storage
        .set_refresh_time(time_in_seconds)
        .await
        .map_err(into_command_error)?;
    state.monitor.stop().await;
    start_monitor(app_handle, &state)
        .await
        .map_err(into_command_error)
}

#[tauri::command]
pub async fn delete_pr(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    pr_number: u64,
) -> Result<(), String> {
    state
        .storage
        .delete_pull_request(PullRequestKey::new(owner, repo, pr_number))
        .await
        .map_err(into_command_error)
}

#[tauri::command]
pub async fn get_show_notification(state: State<'_, AppState>) -> Result<bool, String> {
    state
        .storage
        .get_show_notification()
        .await
        .map_err(into_command_error)
}

#[tauri::command]
pub async fn set_show_notification(state: State<'_, AppState>, show: bool) -> Result<(), String> {
    state
        .storage
        .set_show_notification(show)
        .await
        .map_err(into_command_error)
}

#[tauri::command]
pub async fn get_theme(state: State<'_, AppState>) -> Result<String, String> {
    state.storage.get_theme().await.map_err(into_command_error)
}

#[tauri::command]
pub async fn set_theme(state: State<'_, AppState>, theme: String) -> Result<(), String> {
    state
        .storage
        .set_theme(theme)
        .await
        .map_err(into_command_error)
}

async fn start_monitor(app_handle: tauri::AppHandle<Wry>, state: &AppState) -> AppResult<()> {
    let Some(token) = state.credentials.get_github_token().await? else {
        info!("Token not found");
        return Ok(());
    };
    let refresh_time_secs = state.storage.get_refresh_time().await?;

    state
        .monitor
        .start(state.storage.clone(), token, refresh_time_secs, app_handle)
        .await;

    Ok(())
}
