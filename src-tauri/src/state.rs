//! What the commands share: put in Tauri's state (`app.manage`) at startup.

use std::{path::PathBuf, sync::{Mutex, atomic::AtomicBool}};

use crate::{commands::{config::model::AppConfig, osu_reader::ReaderSnapshot}, error};

pub struct OsuReaderState {
    pub started: AtomicBool,
    pub snapshot: Mutex<ReaderSnapshot>,
}

pub struct AppConfigState {
    pub config: Mutex<AppConfig>,
    /// `configuration.toml`, written when the app closes.
    pub path: PathBuf,
}

impl AppConfigState {
    /// The osu! folder, which every tool needs.
    pub fn osu_path(&self) -> error::Result<PathBuf> {
        self.config.lock().unwrap().osu_path.clone().map(PathBuf::from).ok_or_else(|| "osu! path not set".into())
    }
}

/// The update found by `check_update`, waiting for the user to install it.
pub struct UpdateState {
    pub pending: Mutex<Option<tauri_plugin_updater::Update>>,
}

/// The osu_session cookie in use; it only reaches the configuration file
/// with `save_session`.
pub struct SessionState {
    pub osu_session: Mutex<Option<String>>,
}
