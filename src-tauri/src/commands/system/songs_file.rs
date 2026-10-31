use tauri::ipc::Response;

use crate::{error::Result, paths};

/// Reads a file inside the Songs folder (e.g. a beatmap background) and sends
/// it as raw bytes, which the frontend turns into a `blob:` URL. `path` can be
/// absolute or relative to the Songs folder.
///
/// The `asset:` protocol is not used for this: WebKitGTK treats it as another
/// origin than the dev server (`http://localhost:1420`) and blocks the load
/// ("Unsafe attempt to load URL asset://localhost/..."). A blob URL is always
/// same-origin.
#[tauri::command]
pub async fn read_songs_file(app: tauri::AppHandle, path: String) -> Result<Response> {
    let songs = tokio::fs::canonicalize(paths::songs_dir(&app)?)
        .await
        .map_err(|e| format!("Songs folder not found: {}", e))?;

    let file = tokio::fs::canonicalize(songs.join(&path)).await.map_err(|e| {
        log::warn!("songs file {:?} not found: {}", path, e);
        e
    })?;

    // `path` comes from the frontend, but `..` or a symlink must not read outside Songs.
    if !file.starts_with(&songs) {
        log::warn!("refused to read {:?}: outside the Songs folder", file);
        return Err("path is outside the Songs folder".into());
    }

    let bytes = tokio::fs::read(&file).await.map_err(|e| {
        log::warn!("failed to read {:?}: {}", file, e);
        e
    })?;

    Ok(Response::new(bytes))
}
