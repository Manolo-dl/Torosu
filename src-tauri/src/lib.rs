use std::{path::{Path, PathBuf}, sync::{Mutex, atomic::AtomicBool}};
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

use crate::commands::config::model::AppConfig;
use crate::commands::osu_reader::ReaderSnapshot;

mod commands;

pub struct OsuReaderState {
    pub started: AtomicBool,
    pub snapshot: Mutex<ReaderSnapshot>,
}

pub struct AppConfigState {
    pub config: Mutex<AppConfig>,
    /// `configuration.toml`, written when the app closes.
    pub path: PathBuf,
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

pub fn run() {
    // reqwest is built without a bundled crypto provider; share ring with the updater
    let _ = rustls::crypto::ring::default_provider().install_default();

    tauri::Builder::default()
        .setup(|app| {
            app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;

            // The settings live in the user's folders: a system install (the
            // .deb or the .rpm, in /usr/bin) can't write next to the executable.
            let config_dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&config_dir)?;

            let config_path = config_dir.join("configuration.toml");

            if let Some(legacy_dir) = legacy_dir() {
                migrate_legacy_files(&legacy_dir, &config_path);
            }

            // Load Configuration
            let config = commands::config::file::read_config(&config_path);
            app.manage(SessionState { osu_session: Mutex::new(config.downloader.osu_session.clone()) });
            app.manage(AppConfigState { config: Mutex::new(config), path: config_path });

            // Updates are offered by the frontend (see `check_update`).
            app.manage(UpdateState { pending: Mutex::new(None) });

            // osu! memory reader, started by the first page that needs it
            app.manage(OsuReaderState {
                started: AtomicBool::new(false),
                snapshot: Mutex::new(ReaderSnapshot::new()),
            });

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_log::Builder::new()
            .max_file_size(50_000)
            .format(|out, message, record| {
                out.finish(format_args!(
                    "[{} {} {}] {}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                    record.level(),
                    record.target(),
                    message
                ))
            })
            .level(log::LevelFilter::Debug)
            .level_for("h2", log::LevelFilter::Off)
            .level_for("hyper", log::LevelFilter::Off)
            .level_for("hyper_util", log::LevelFilter::Off)
            .level_for("reqwest", log::LevelFilter::Off)
            .level_for("tracing", log::LevelFilter::Off)
            .level_for("tao", log::LevelFilter::Off)
            .level_for("wry", log::LevelFilter::Off)
            .level_for("rustls", log::LevelFilter::Off)
            .level_for("zbus", log::LevelFilter::Off)
            .level_for("rustls_platform_verifier", log::LevelFilter::Off)
            .level_for("tauri_plugin_updater", log::LevelFilter::Off)
            .targets([
                Target::new(TargetKind::LogDir { file_name: None }),
                Target::new(TargetKind::Stdout),
            ])
            .build()
        )
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .invoke_handler(tauri::generate_handler![
            commands::config::file::get_config,
            commands::config::file::update_config,
            commands::system::osu_path::get_osu_path,
            commands::system::osu_path::save_osu_path,
            commands::system::songs_file::read_songs_file,
            commands::system::update::check_update,
            commands::system::update::install_update,
            commands::collections::reader::read_osu_collections,
            commands::collections::reader::write_text_file,
            commands::collections::writer::import_collections,
            commands::osu_db::reader::read_osu_db_full,
            commands::system::download::start_downloads,
            commands::system::download::get_osu_session,
            commands::system::download::set_osu_session,
            commands::packs::folders::validate_pack_folder,
            commands::packs::creator::create_pack,
            commands::dev::logs::read_logs,
            commands::osu_reader::connect_osu_reader,
            commands::osu_reader::export::export_selected_osz,
            commands::trainer::run::run_trainer,
            commands::trainer::video::is_ffmpeg_available
        ])
        .on_window_event(|window, event| {
            
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                
                let config_state = window.app_handle().state::<AppConfigState>();
                let config = config_state.config.lock().unwrap().clone();

                if let Err(e) = commands::config::file::write_config(&config_state.path, &config) {
                    log::error!("failed to write configuration.toml on close: {e}");
                } else  {
                    log::info!("configuration.toml saved on close");
                }

            }
        }) 
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Where earlier versions kept `configuration.toml` and `osu_cache.db`: next to
/// the executable, or to the AppImage.
fn legacy_dir() -> Option<PathBuf> {
    if let Ok(appimage) = std::env::var("APPIMAGE") {
        return Path::new(&appimage).parent().map(Path::to_path_buf);
    }

    std::env::current_exe().ok()?.parent().map(Path::to_path_buf)
}

/// Moves the settings of earlier versions out of `legacy_dir`, unless the new
/// place already has its own, and deletes the SQLite cache of osu!.db and
/// collection.db they kept there: reading it was slower than reading those
/// files, so it is no longer used.
fn migrate_legacy_files(legacy_dir: &Path, config_path: &Path) {
    move_legacy_file(&legacy_dir.join("configuration.toml"), config_path);

    for name in ["osu_cache.db", "osu_cache.db-journal", "osu_cache.db-wal", "osu_cache.db-shm"] {
        let path = legacy_dir.join(name);
        if !path.is_file() {
            continue;
        }

        match std::fs::remove_file(&path) {
            Ok(()) => log::info!("removed the old cache {:?}", path),
            Err(e) => log::warn!("could not remove the old cache {:?}: {}", path, e),
        }
    }
}

fn move_legacy_file(from: &Path, to: &Path) {
    if !from.is_file() || to.exists() {
        return;
    }

    // A rename can't cross file systems (e.g. an AppImage on another drive).
    let moved = std::fs::rename(from, to).or_else(|_| {
        std::fs::copy(from, to)?;
        std::fs::remove_file(from)
    });

    match moved {
        Ok(()) => log::info!("moved {:?} to {:?}", from, to),
        Err(e) => log::warn!("could not move {:?} to {:?}: {}", from, to, e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An empty folder in the temp dir, removed when dropped.
    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("torosu-test-{}-{}", std::process::id(), name));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn write(&self, name: &str, content: &str) {
            std::fs::write(self.0.join(name), content).unwrap();
        }

        fn read(&self, name: &str) -> Option<String> {
            std::fs::read_to_string(self.0.join(name)).ok()
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn migrate(old: &TestDir, new: &TestDir) {
        migrate_legacy_files(&old.0, &new.0.join("configuration.toml"));
    }

    #[test]
    fn moves_the_settings_of_earlier_versions() {
        let (old, new) = (TestDir::new("move-old"), TestDir::new("move-new"));
        old.write("configuration.toml", "dev_mode = true");

        migrate(&old, &new);

        assert_eq!(new.read("configuration.toml").as_deref(), Some("dev_mode = true"));
        assert_eq!(old.read("configuration.toml"), None);
    }

    #[test]
    fn keeps_what_the_new_place_already_has() {
        let (old, new) = (TestDir::new("keep-old"), TestDir::new("keep-new"));
        old.write("configuration.toml", "old");
        new.write("configuration.toml", "new");

        migrate(&old, &new);

        assert_eq!(new.read("configuration.toml").as_deref(), Some("new"));
        assert_eq!(old.read("configuration.toml").as_deref(), Some("old"));
    }

    #[test]
    fn deletes_the_old_cache_and_nothing_else() {
        let (old, new) = (TestDir::new("cache-old"), TestDir::new("cache-new"));
        old.write("osu_cache.db", "cache");
        old.write("osu_cache.db-journal", "journal");
        old.write("torosu.AppImage", "app");

        migrate(&old, &new);

        assert_eq!(old.read("osu_cache.db"), None);
        assert_eq!(old.read("osu_cache.db-journal"), None);
        assert_eq!(old.read("torosu.AppImage").as_deref(), Some("app"));
    }
}