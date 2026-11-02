use reqwest::Client;
use futures_util::StreamExt;
use tauri::{Emitter, Manager};
use tokio::io::AsyncWriteExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

use crate::{error::Result, paths, state::{AppConfigState, SessionState}};

const MAX_SIZE: u64 = 200 * 1024 * 1024;
const DELAY_BETWEEN_DOWNLOADS_MS: u64 = 1000; // ~1 req/seg, osu! documentation recommendation
/// A chunk is a few KB, and the frontend redraws the queue on every report:
/// the progress of a download is sent at most this often.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub beatmap_set_id: u64,
    pub progress: f64,
    pub status: String,
}

#[tauri::command]
pub async fn start_downloads(
    app: tauri::AppHandle,
    beatmap_set_ids: Vec<u64>,
) -> Result<()> {

    let config_state = app.state::<AppConfigState>();
    let (downloader_config, open_file) = {
        let config = config_state.config.lock().unwrap();
        (config.downloader.clone(), config.open_file)
    };

    let osu_session = app
        .state::<SessionState>()
        .osu_session
        .lock()
        .unwrap()
        .clone()
        .ok_or("osu_session cookie not set")?;

    let songs_folder = match &downloader_config.output_path {
        Some(path) => PathBuf::from(path),
        None => paths::songs_dir(&app)?,
    };

    if !songs_folder.exists() {
        log::error!("Output folder not found: {}", songs_folder.display());
        return Err(format!("Output folder not found: {}", songs_folder.display()).into());
    }

    let client = Client::new();
    let max_concurrent = downloader_config.max_concurrent_downloads.max(1) as usize;
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    let tasks: Vec<_> = beatmap_set_ids.into_iter().map(|id| {
        let app = app.clone();
        let client = client.clone();
        let osu_session = osu_session.clone();
        let songs_folder = songs_folder.clone();
        let skip_video = downloader_config.skip_video;
        let semaphore = semaphore.clone();

        tokio::spawn(async move {
            
            let permit = semaphore.acquire().await.unwrap();

            app.emit("download:progress", DownloadProgress {
                beatmap_set_id: id, progress: 0.0, status: "downloading".to_string(),
            }).ok();

            let result = download_beatmap(&app, &client, id, &osu_session, &songs_folder, skip_video, open_file).await;

            match &result {
                Ok(_) => {
                    app.emit("download:progress", DownloadProgress {
                        beatmap_set_id: id, progress: 100.0, status: "done".to_string(),
                    }).ok();
                }
                Err(e) => {
                    log::warn!("Beatmap {} marked as failed: {}", id, e);
                    app.emit("download:progress", DownloadProgress {
                        beatmap_set_id: id, progress: 0.0, status: "failed".to_string(),
                    }).ok();
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(DELAY_BETWEEN_DOWNLOADS_MS)).await;
            drop(permit);

            (id, result)
        })
    }).collect();

    let mut results: Vec<(u64, Result<()>)> = Vec::new();
    for task in tasks {
        if let Ok(r) = task.await {
            results.push(r);
        }
    }

    let failed_ids: Vec<u64> = results.iter()
        .filter_map(|(id, r)| r.as_ref().err().map(|_| *id))
        .collect();

    if downloader_config.error_file && !failed_ids.is_empty() {
        let error_file_path = songs_folder.join("failed_downloads.txt");
        let content = failed_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join("\n");
        if let Err(e) = tokio::fs::write(&error_file_path, content).await {
            log::error!("Failed to write error file {:?}: {}", error_file_path, e);
        }
    }

    if downloader_config.notify {
        use tauri_plugin_notification::NotificationExt;

        if let Err(e) = app.notification()
            .builder()
            .title("Downloader")
            .body(format!("{} succeeded, {} failed", results.len() - failed_ids.len(), failed_ids.len()))
            .show()
        {
            log::error!("Failed to show notification: {}", e);
        }

    }

    Ok(())
}

async fn download_beatmap(
    app: &tauri::AppHandle,
    client: &Client,
    beatmap_set_id: u64,
    osu_session: &str,
    songs_folder: &Path,
    skip_video: bool,
    open_file: bool,
) -> Result<()> {

    let url = if skip_video {
        format!("https://osu.ppy.sh/beatmapsets/{}/download?noVideo=1", beatmap_set_id)
    } else {
        format!("https://osu.ppy.sh/beatmapsets/{}/download", beatmap_set_id)
    };

    let response = client
        .get(&url)
        .header("Cookie", format!("osu_session={}", osu_session))
        .header("Referer", format!("https://osu.ppy.sh/beatmapsets/{}", beatmap_set_id))
        .send()
        .await
        .map_err(|e| {
            log::error!("HTTP request failed for {}: {}", beatmap_set_id, e);
            e
        })?;

    if !response.status().is_success() {
        log::error!("Failed to download {}: HTTP {}", beatmap_set_id, response.status());
        return Err(format!("HTTP {}", response.status()).into());
    }

    let total = response.content_length().unwrap_or(0);

    if total > MAX_SIZE {
        log::error!("Beatmap set {} is too large: {} bytes", beatmap_set_id, total);
        return Err(format!("Beatmap set {} is too large ({} bytes)", beatmap_set_id, total).into());
    }

    let disposition = response.headers()
        .get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let filename = extract_filename(&disposition, beatmap_set_id);

    let temp_dir = songs_folder
        .parent()
        .ok_or("invalid songs folder path")?
        .join(".onisu-downloads-tmp");

    tokio::fs::create_dir_all(&temp_dir).await?;

    let tmp_path = temp_dir.join(format!("{}.osz.part", beatmap_set_id));
    let final_path = songs_folder.join(&filename);

    let mut file = tokio::fs::File::create(&tmp_path).await.map_err(|e| {
        log::error!("Failed to create file {:?}: {}", tmp_path, e);
        e
    })?;

    let mut downloaded: u64 = 0;
    let mut last_progress = Instant::now();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                log::error!("Stream interrupted for {}: {}", beatmap_set_id, e);
                let _ = tokio::fs::remove_file(&tmp_path).await;
                return Err(e.into());
            }
        };

        downloaded += chunk.len() as u64;

        if let Err(e) = file.write_all(&chunk).await {
            log::error!("Failed to write chunk for {}: {}", beatmap_set_id, e);
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(e.into());
        }

        // The 100% is reported by `start_downloads` with the "done" status.
        if total > 0 && last_progress.elapsed() >= PROGRESS_INTERVAL {
            last_progress = Instant::now();
            let progress = (downloaded as f64 / total as f64) * 100.0;
            app.emit("download:progress", DownloadProgress {
                beatmap_set_id,
                progress,
                status: "downloading".to_string(),
            }).ok();
        }
    }

    file.flush().await?;
    drop(file);

    tokio::fs::rename(&tmp_path, &final_path).await.map_err(|e| {
        log::error!("Failed to move file to {:?}: {}", final_path, e);
        e
    })?;

    if open_file {
        if let Err(e) = tauri_plugin_opener::open_path(&final_path, None::<&str>) {
            log::error!("Failed to open file {:?}: {}", final_path, e);
        } else {
            log::info!("Opened file {:?}", final_path);
        }
    }

    Ok(())
}

/// Extracts `filename` from a `Content-Disposition` header, e.g.
/// `attachment; filename="123 Artist - Title.osz"`
fn extract_filename(disposition: &str, beatmap_set_id: u64) -> String {
    let name = disposition
        .split_once("filename=")
        .map(|(_, rest)| rest.trim_start_matches('"'))
        .and_then(|rest| rest.split(['"', ';', '\n']).next())
        .filter(|name| !name.is_empty());

    match name {
        Some(name) => sanitize_filename(name),
        None => format!("{}.osz", beatmap_set_id),
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '\0' => '_',
            c if (c as u32) < 0x20 => '_',
            c => c,
        })
        .collect::<String>()
        .trim_end_matches(['.', ' '])
        .to_string()
}

#[tauri::command]
pub fn get_osu_session(session: tauri::State<'_, SessionState>) -> Option<String> {
    session.osu_session.lock().unwrap().clone()
}

/// Sets the cookie in use. It's saved in the configuration only with
/// `save_session`; otherwise it lasts until Torosu closes.
#[tauri::command]
pub fn set_osu_session(
    session: Option<String>,
    state: tauri::State<'_, AppConfigState>,
    current: tauri::State<'_, SessionState>,
) -> Result<()> {
    *current.osu_session.lock().unwrap() = session.clone();
    state.update(|config| {
        config.downloader.osu_session = if config.downloader.save_session { session } else { None };
    })
}