use crate::domain::PrStatus;
use crate::error::{AppError, AppResult};
use crate::github::{needs_update_pr, update_pr_branch};
use crate::storage::Storage;
use log::{error, info};
use std::sync::Arc;
use tauri::{Emitter, Wry};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::{watch, Mutex};
use tokio::task::JoinHandle;

#[derive(Clone, Default)]
pub struct Monitor {
    handle: Arc<Mutex<Option<MonitorHandle>>>,
}

struct MonitorHandle {
    stop_tx: watch::Sender<bool>,
    task: JoinHandle<()>,
}

impl Monitor {
    pub async fn start(
        &self,
        storage: Storage,
        token: String,
        refresh_time_secs: u64,
        app_handle: tauri::AppHandle<Wry>,
    ) {
        let mut monitor = self.handle.lock().await;
        if let Some(handle) = monitor.as_ref() {
            if !handle.task.is_finished() {
                info!("Task is already running!");
                return;
            }
        }

        if let Some(handle) = monitor.take() {
            handle.task.abort();
        }

        info!("Starting monitor PRs");
        let (stop_tx, mut stop_rx) = watch::channel(false);

        let task = tokio::spawn(async move {
            let refresh_duration = std::time::Duration::from_secs(refresh_time_secs);
            let mut interval = tokio::time::interval(refresh_duration);

            'monitor: loop {
                tokio::select! {
                    changed = stop_rx.changed() => {
                        if changed.is_err() || *stop_rx.borrow() {
                            info!("Task stopped!");
                            break;
                        }
                    }
                    _ = interval.tick() => {
                        info!(
                            "Running task with refresh time: {} seconds",
                            refresh_time_secs
                        );

                        let check_pull_requests =
                            check_pull_requests(&storage, &token, &app_handle);

                        tokio::select! {
                            changed = stop_rx.changed() => {
                                if changed.is_err() || *stop_rx.borrow() {
                                    info!("Task stopped during processing!");
                                    break 'monitor;
                                }
                            }
                            result = check_pull_requests => {
                                if let Err(err) = result {
                                    error!("Monitor check failed: {err}");
                                }
                            }
                        }
                    }
                }
            }
        });

        *monitor = Some(MonitorHandle { stop_tx, task });
    }

    pub async fn stop(&self) {
        let handle = {
            let mut monitor = self.handle.lock().await;
            monitor.take()
        };

        if let Some(handle) = handle {
            let _ = handle.stop_tx.send(true);
            if let Err(err) = handle.task.await {
                error!("Monitor task failed to stop cleanly: {err}");
            }
        }
    }
}

async fn check_pull_requests(
    storage: &Storage,
    token: &str,
    app_handle: &tauri::AppHandle<Wry>,
) -> AppResult<()> {
    let show_notification = storage.get_show_notification().await?;
    let pull_requests = storage.get_open_pull_requests().await?;

    for pr in pull_requests {
        let key = pr.key();
        let pr_status = needs_update_pr(&key, token).await?;

        match pr_status {
            PrStatus::Merged => {
                info!("PR was merged, updating status");
                storage.mark_pull_request_closed(key.clone()).await?;
                app_handle
                    .emit("pr-closed", &key)
                    .map_err(|err| AppError::Event(err.to_string()))?;
            }
            PrStatus::Behind => {
                info!("PR is behind, updating branch");
                if let Err(err) = update_pr_branch(&key, token).await {
                    error!("Failed to update PR branch: {err}");
                    if show_notification {
                        app_handle
                            .notification()
                            .builder()
                            .title("Failed to update PR")
                            .body(err.to_string())
                            .show()
                            .map_err(|err| AppError::Notification(err.to_string()))?;
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
                    let title = format!("PR Not Updated: #{}", key.pr_number);
                    let body = format!("PR {} - please check.", status_str);
                    app_handle
                        .notification()
                        .builder()
                        .title(&title)
                        .body(&body)
                        .show()
                        .map_err(|err| AppError::Notification(err.to_string()))?;
                }
            }
            PrStatus::UpToDate => {
                info!("PR is up to date");
            }
        }
    }

    Ok(())
}
