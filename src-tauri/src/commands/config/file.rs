use crate::{app_root_dir, commands::config::model::AppConfig};

pub fn read_config() -> AppConfig {

    let path = app_root_dir().join("configuration.toml");

    match std::fs::read_to_string(path) {
        Ok(content) => toml::from_str(&content).unwrap_or_else(|e| {
            log::warn!("Failed to parse configuration.toml, using defaults: {}", e);
            AppConfig::default()
        }),
        Err(_) => {
            log::info!("configuration.toml not found, using defaults");
            AppConfig::default()
        }
    }
}

pub fn write_config(config: &AppConfig) -> Result<(), String> {
     let path = app_root_dir().join("configuration.toml");

     let toml = toml::to_string_pretty(config).map_err(|e| {
        log::error!("Failed to serialize configuration.toml: {}", e);
        e.to_string()
     })?;

     std::fs::write(path, toml).map_err(|e| {
        log::error!("Failed to write configuration.toml: {}", e);
        e.to_string()
     })?;

     Ok(())
}

#[tauri::command]
pub async fn update_config(config: AppConfig, state: tauri::State<'_, crate::AppConfigState>) -> Result<(), String> {
    let mut current = state.config.lock().unwrap();
    *current = config;
    Ok(())
}