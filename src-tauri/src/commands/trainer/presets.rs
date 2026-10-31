//! The trainer presets the user saves: lists of converts with their tasks.
//! They are kept as the frontend sends them, in `trainer-presets.json` next to
//! `configuration.toml`; the frontend owns their shape.

use std::path::PathBuf;

use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::{error::Result, state::AppConfigState};

fn presets_path(app: &AppHandle) -> PathBuf {
    app.state::<AppConfigState>().path.with_file_name("trainer-presets.json")
}

/// The saved presets, an empty list when there are none yet.
#[tauri::command]
pub async fn read_trainer_presets(app: AppHandle) -> Result<Value> {
    let path = presets_path(&app);
    let text = match tokio::fs::read_to_string(&path).await {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Value::Array(Vec::new())),
        Err(e) => return Err(e.into()),
    };

    // A broken file would block the trainer; it is overwritten on the next save.
    Ok(serde_json::from_str(&text).unwrap_or_else(|e| {
        log::warn!("ignoring unreadable {:?}: {}", path, e);
        Value::Array(Vec::new())
    }))
}

/// Replaces the saved presets.
#[tauri::command]
pub async fn write_trainer_presets(app: AppHandle, presets: Value) -> Result<()> {
    let path = presets_path(&app);
    let text = serde_json::to_string_pretty(&presets).map_err(|e| e.to_string())?;

    // Written aside and renamed, so a crash never leaves half a file.
    let part = path.with_extension("json.part");
    tokio::fs::write(&part, text).await?;
    tokio::fs::rename(&part, &path).await?;
    Ok(())
}
