use std::path::Path;

use tauri::{ipc::Response, Manager};

use crate::{commands::osu_reader::songs_folder, AppConfigState};

/// Reads a file inside the Songs folder (e.g. a beatmap background) and sends
/// it as raw bytes, which the frontend turns into a `blob:` URL. `path` can be
/// absolute or relative to the Songs folder.
///
/// The `asset:` protocol is not used for this: WebKitGTK treats it as another
/// origin than the dev server (`http://localhost:1420`) and blocks the load
/// ("Unsafe attempt to load URL asset://localhost/..."). A blob URL is always
/// same-origin.
#[tauri::command]
pub async fn read_songs_file(app: tauri::AppHandle, path: String) -> Result<Response, String> {
    // osu! can keep the songs somewhere else (`BeatmapDirectory`); the reader knows where.
    let songs = match songs_folder(&app) {
        Some(songs) => songs,
        None => {
            let osu_path = app
                .state::<AppConfigState>()
                .config
                .lock()
                .unwrap()
                .osu_path
                .clone()
                .ok_or_else(|| "osu! path is not set".to_string())?;
            Path::new(&osu_path).join("Songs")
        }
    };

    let songs = tokio::fs::canonicalize(&songs)
        .await
        .map_err(|e| format!("Songs folder not found: {}", e))?;

    let file = tokio::fs::canonicalize(songs.join(&path)).await.map_err(|e| {
        log::warn!("songs file {:?} not found: {}", path, e);
        e.to_string()
    })?;

    // `path` comes from the frontend, but `..` or a symlink must not read outside Songs.
    if !file.starts_with(&songs) {
        log::warn!("refused to read {:?}: outside the Songs folder", file);
        return Err("path is outside the Songs folder".to_string());
    }

    let bytes = tokio::fs::read(&file).await.map_err(|e| {
        log::warn!("failed to read {:?}: {}", file, e);
        e.to_string()
    })?;

    Ok(Response::new(bytes))
}
