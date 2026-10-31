use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

use crate::{error::Result, state::UpdateState};

/// A newer version on the update server.
#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    /// Release notes.
    pub notes: Option<String>,
    pub date: Option<String>,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

/// Looks for a newer version, without installing it: the frontend offers it
/// and calls [`install_update`] if the user wants it.
#[tauri::command]
pub async fn check_update(app: AppHandle, state: tauri::State<'_, UpdateState>) -> Result<Option<UpdateInfo>> {
    let update = app
        .updater_builder()
        .timeout(Duration::from_secs(30))
        .build()?
        .check()
        .await
        .map_err(|e| {
            log::error!("failed to check for updates: {e}");
            e
        })?;

    let info = update.as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        current_version: u.current_version.clone(),
        notes: u.body.clone(),
        date: u.date.map(|d| d.to_string()),
    });
    if let Some(info) = &info {
        log::info!("update available: {} -> {}", info.current_version, info.version);
    }
    *state.pending.lock().unwrap() = update;
    Ok(info)
}

/// Downloads and installs the update found by [`check_update`], reporting
/// `update:progress`, then restarts the app.
#[tauri::command]
pub async fn install_update(app: AppHandle, state: tauri::State<'_, UpdateState>) -> Result<()> {
    let update = state.pending.lock().unwrap().clone().ok_or("no update to install")?;

    let mut downloaded = 0u64;
    let mut last_emit = Instant::now();
    update
        .download_and_install(
            |chunk, total| {
                downloaded += chunk as u64;
                // A chunk is a few KB: report a few times per second at most.
                if last_emit.elapsed() >= Duration::from_millis(100) || total == Some(downloaded) {
                    last_emit = Instant::now();
                    let _ = app.emit("update:progress", UpdateProgress { downloaded, total });
                }
            },
            || log::info!("update downloaded, installing..."),
        )
        .await
        .map_err(|e| {
            log::error!("failed to install the update: {e}");
            e
        })?;

    log::info!("update installed, restarting");
    app.restart();
}
