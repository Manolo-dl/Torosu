//! What the commands share: put in Tauri's state (`app.manage`) at startup.

use std::{path::PathBuf, sync::{Mutex, atomic::AtomicBool}};

use crate::{commands::{config::{file::write_config, model::AppConfig}, osu_reader::ReaderSnapshot}, error};

pub struct OsuReaderState {
    pub started: AtomicBool,
    pub snapshot: Mutex<ReaderSnapshot>,
}

pub struct AppConfigState {
    pub config: Mutex<AppConfig>,
    /// `configuration.toml`, written on every change (see `update`).
    pub path: PathBuf,
}

impl AppConfigState {
    /// The osu! folder, which every tool needs.
    pub fn osu_path(&self) -> error::Result<PathBuf> {
        self.config.lock().unwrap().osu_path.clone().map(PathBuf::from).ok_or_else(|| "osu! path not set".into())
    }

    /// Changes the configuration and writes it to `configuration.toml` right
    /// away: an update restarts Torosu without closing its window, and a crash
    /// or a shutdown don't close it either.
    pub fn update(&self, change: impl FnOnce(&mut AppConfig)) -> error::Result<()> {
        let mut config = self.config.lock().unwrap();
        change(&mut config);
        write_config(&self.path, &config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{commands::config::file::read_config, test_utils::TestDir};

    #[test]
    fn writes_every_change_right_away() {
        let dir = TestDir::new("config-update");
        let state = AppConfigState { config: Mutex::new(AppConfig::default()), path: dir.0.join("configuration.toml") };

        state.update(|config| config.osu_path = Some("/osu".into())).unwrap();

        assert_eq!(read_config(&state.path).osu_path.as_deref(), Some("/osu"));
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
