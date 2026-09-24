use reqwest::Client;
use futures_util::StreamExt;
use tauri::{Emitter, Manager};
use tokio::io::AsyncWriteExt;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::AppConfigState;

const MAX_SIZE: u64 = 200 * 1024 * 1024;
const DELAY_BETWEEN_DOWNLOADS_MS: u64 = 1000; // ~1 req/seg, osu! documentation recommendation

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
) -> Result<(), String> {

    let config_state = app.state::<AppConfigState>();
    let (osu_path, downloader_config, open_file) = {
        let config = config_state.config.lock().unwrap();
        (
            config.osu_path.clone().ok_or("osu! path not set")?,
            config.downloader.clone(),
            config.open_file.clone(),
        )
    };

    let osu_session = downloader_config
        .osu_session
        .clone()
        .ok_or("osu_session not set in configuration")?;

    let songs_folder = downloader_config
        .output_path
        .clone()
        .unwrap_or_else(|| format!("{}/Songs", osu_path));

    if !std::path::Path::new(&songs_folder).exists() {
        log::error!("Output folder not found: {}", songs_folder);
        return Err(format!("Output folder not found: {}", songs_folder));
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

    let mut results: Vec<(u64, Result<(), String>)> = Vec::new();
    for task in tasks {
        if let Ok(r) = task.await {
            results.push(r);
        }
    }

    let failed_ids: Vec<u64> = results.iter()
        .filter_map(|(id, r)| r.as_ref().err().map(|_| *id))
        .collect();

    if downloader_config.error_file && !failed_ids.is_empty() {
        let error_file_path = format!("{}/failed_downloads.txt", songs_folder);
        let content = failed_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join("\n");
        if let Err(e) = std::fs::write(&error_file_path, content) {
            log::error!("Failed to write error file {}: {}", error_file_path, e);
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
    songs_folder: &str,
    skip_video: bool,
    open_file: bool,
) -> Result<(), String> {

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
            e.to_string()
        })?;

    if !response.status().is_success() {
        log::error!("Failed to download {}: HTTP {}", beatmap_set_id, response.status());
        return Err(format!("HTTP {}", response.status()));
    }

    let total = response.content_length().unwrap_or(0);

    if total > MAX_SIZE {
        log::error!("Beatmap set {} is too large: {} bytes", beatmap_set_id, total);
        return Err(format!("Beatmap set {} is too large ({} bytes)", beatmap_set_id, total));
    }

    let disposition = response.headers()
        .get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let filename = extract_filename(&disposition, beatmap_set_id);

    let temp_dir = std::path::Path::new(songs_folder)
        .parent()
        .ok_or_else(|| "invalid songs folder path".to_string())?
        .join(".onisu-downloads-tmp");

    tokio::fs::create_dir_all(&temp_dir).await.map_err(|e| e.to_string())?;

    let tmp_path = temp_dir.join(format!("{}.osz.part", beatmap_set_id));
    let final_path = format!("{}/{}", songs_folder, filename);

    let mut file = tokio::fs::File::create(&tmp_path).await.map_err(|e| {
        log::error!("Failed to create file {:?}: {}", tmp_path, e);
        e.to_string()
    })?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                log::error!("Stream interrupted for {}: {}", beatmap_set_id, e);
                let _ = tokio::fs::remove_file(&tmp_path).await;
                return Err(e.to_string());
            }
        };

        downloaded += chunk.len() as u64;

        if let Err(e) = file.write_all(&chunk).await {
            log::error!("Failed to write chunk for {}: {}", beatmap_set_id, e);
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(e.to_string());
        }

        if total > 0 {
            let progress = (downloaded as f64 / total as f64) * 100.0;
            app.emit("download:progress", DownloadProgress {
                beatmap_set_id,
                progress,
                status: "downloading".to_string(),
            }).ok();
        }
    }

    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);
    
    tokio::fs::rename(&tmp_path, &final_path).await.map_err(|e| {
        log::error!("Failed to move file to {}: {}", final_path, e);
        e.to_string()
    })?;

    if open_file {
        if let Err(e) = open::that(&final_path) {
            log::error!("Failed to open file {}: {}", final_path, e);
        } else {
            log::info!("Opened file {}", final_path);
        }
    }

    Ok(())
}

fn extract_filename(disposition: &str, beatmap_set_id: u64) -> String {
    let re = regex::Regex::new(r#"filename="?([^";\n]+)"?"#).unwrap();
    let name = if let Some(cap) = re.captures(disposition) {
        cap[1].to_string()
    } else {
        return format!("{}.osz", beatmap_set_id);
    };

    sanitize_filename(&name)
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
pub async fn append_text_file(path: String, content: String) -> Result<(), String> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;

    file.write_all(content.as_bytes()).map_err(|e| {
        log::error!("Failed to write file {}: {}", path, e);
        e.to_string()
    })
}

#[tauri::command]
pub fn get_osu_session(state: tauri::State<'_, AppConfigState>) -> Option<String> {
    state.config.lock().unwrap().downloader.osu_session.clone()
}

#[tauri::command]
pub fn set_osu_session(
    session: Option<String>,
    state: tauri::State<'_, AppConfigState>,
) -> Result<(), String> {
    state.config.lock().unwrap().downloader.osu_session = session;
    Ok(())
}