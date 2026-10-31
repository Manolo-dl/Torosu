use std::path::Path;

use crate::{commands::config::model::AppConfig, error::Result};

pub fn read_config(path: &Path) -> AppConfig {

    let mut config: AppConfig = match std::fs::read_to_string(path) {
        Ok(content) => toml::from_str(&content).unwrap_or_else(|e| {
            log::warn!("Failed to parse configuration.toml, using defaults: {}", e);
            AppConfig::default()
        }),
        Err(_) => {
            log::info!("configuration.toml not found, using defaults");
            AppConfig::default()
        }
    };

    // Only a saved cookie gets to the file, so one there was saved on purpose,
    // also by versions without `save_session`.
    config.downloader.save_session |= config.downloader.osu_session.is_some();
    config
}

pub fn write_config(path: &Path, config: &AppConfig) -> Result<()> {
    std::fs::write(path, toml::to_string_pretty(config)?)?;
    Ok(())
}

#[tauri::command]
pub fn get_config(state: tauri::State<'_, crate::state::AppConfigState>) -> AppConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub async fn update_config(
    config: AppConfig,
    state: tauri::State<'_, crate::state::AppConfigState>,
    session: tauri::State<'_, crate::state::SessionState>,
) -> Result<()> {
    let mut config = config;
    // The cookie is set from the downloader, not here: the one in use is
    // saved or forgotten as `save_session` says.
    config.downloader.osu_session = if config.downloader.save_session {
        session.osu_session.lock().unwrap().clone()
    } else {
        None
    };
    *state.config.lock().unwrap() = config;
    Ok(())
}