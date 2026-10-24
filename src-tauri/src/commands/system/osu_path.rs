use std::path::{Path, PathBuf};

use tauri::Manager;

use crate::AppConfigState;

/// Allows the Songs folder in the `asset:` protocol scope at runtime.
fn allow_songs_in_asset_scope(app: &tauri::AppHandle, osu_path: &str) {
    let songs = Path::new(osu_path).join("Songs");

    match app.asset_protocol_scope().allow_directory(&songs, true) {
        Ok(()) => log::info!("asset scope: allowed {:?}", songs),
        Err(e) => log::error!("asset scope: failed to allow {:?}: {}", songs, e),
    }
}

#[tauri::command]
pub async fn get_osu_path(
    app: tauri::AppHandle
) -> Result<String, String> {
    let path = resolve_osu_path(&app).await?;
    allow_songs_in_asset_scope(&app, &path);
    Ok(path)
}

async fn get_osu_path_from_memory() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let reader = osu_memory_reader::OsuReader::attach().map_err(|e| {
            log::debug!("osu! path not read from memory: {}", e);
            e.to_string()
        })?;

        reader
            .game_folder()
            .map(|path| path.to_string_lossy().into_owned())
            .ok_or_else(|| "osu! is running but its folder was not found".to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

async fn resolve_osu_path(app: &tauri::AppHandle) -> Result<String, String> {
    log::debug!("Getting osu! path");

    // Check if the path is already stored in the state
    let config_state = app.state::<AppConfigState>();
    let config = config_state.config.lock().unwrap().clone();

    // 1. Check path from config
    if let Some(path) = config.osu_path.clone() {
        return Ok(path);
    }

    // 2. Check path of the running osu!
    if let Ok(path) = get_osu_path_from_memory().await {
        save_detected_path(&config_state, &path);
        return Ok(path);
    }

    // 3. Check default paths
    log::debug!("checking default paths");

    if let Ok(local) = app.path().local_data_dir() {
        
        let candidates: Vec<PathBuf> = if cfg!(target_os = "windows") {
            vec![
                local.join("osu!"),
            ]
        } else {
            vec![
                local.join("osu-wine/osu!"),
                local.join("osu"),
            ]
        };

        for path in &candidates {
            if path.join("Songs").exists() {
                save_detected_path(&config_state, &path.to_string_lossy().to_string());
                return Ok(path.to_string_lossy().to_string());
            }
        }
    }

    log::error!("osu! folder not found");
    Err("osu! folder not found".to_string())
}

fn save_detected_path(state: &tauri::State<AppConfigState>, path: &str) {
    let mut config = state.config.lock().unwrap();
    config.osu_path = Some(path.to_string());
}

// save osu! path when selected manually by the user
#[tauri::command]
pub fn save_osu_path(app: tauri::AppHandle, path: String) -> Result<(), String> {
    // Every tool reads osu!.db, so a folder without it is not osu!'s.
    if !Path::new(&path).join("osu!.db").is_file() {
        log::warn!("refused osu! path {:?}: no osu!.db in it", path);
        return Err(format!("{} is not an osu! folder: it has no osu!.db", path));
    }

    allow_songs_in_asset_scope(&app, &path);

    let config_state = app.state::<AppConfigState>();
    save_detected_path(&config_state, &path);

    Ok(())
}
