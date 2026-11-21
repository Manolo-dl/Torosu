use std::{path::{Path, PathBuf}, sync::{Mutex, atomic::AtomicBool}};
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

use crate::commands::osu_reader::ReaderSnapshot;
use crate::state::{AppConfigState, OsuReaderState, SessionState, UpdateState};

mod commands;
mod error;
mod name_format;
mod osz;
mod paths;
mod skill;
mod state;
#[cfg(test)]
mod test_utils;
#[cfg(test)]
mod translations;

/// Whether the session is Wayland's (Linux only): the webview needs a
/// workaround there (see `main`), and global shortcuts don't work.
pub fn wayland_session() -> bool {
    cfg!(target_os = "linux") && std::env::var_os("WAYLAND_DISPLAY").is_some()
}

/// Whether NVIDIA's own driver is loaded in a version before 555, the first
/// with explicit sync, which WebKitGTK's DMABUF renderer needs (see `main`).
pub fn nvidia_without_explicit_sync() -> bool {
    std::fs::read_to_string("/proc/driver/nvidia/version")
        .ok()
        .and_then(|text| nvidia_major_version(&text))
        .is_some_and(|major| major < 555)
}

/// The major version in /proc/driver/nvidia/version: "NVRM version: NVIDIA
/// UNIX Open Kernel Module for x86_64  595.99.02  Release Build …".
fn nvidia_major_version(text: &str) -> Option<u32> {
    let line = text.lines().find(|line| line.starts_with("NVRM version"))?;
    line.split_whitespace()
        .find(|word| word.contains('.') && word.chars().next().is_some_and(|c| c.is_ascii_digit()))?
        .split('.')
        .next()?
        .parse()
        .ok()
}

#[cfg(test)]
mod nvidia_tests {
    use super::nvidia_major_version;

    #[test]
    fn reads_the_major_version() {
        let open = "NVRM version: NVIDIA UNIX Open Kernel Module for x86_64  595.99.02  Release Build  (dvs-builder@U22)  Wed Aug 26 2026\nGCC version:  gcc version 13.3.0\n";
        assert_eq!(nvidia_major_version(open), Some(595));
        let old = "NVRM version: NVIDIA UNIX x86_64 Kernel Module  535.183.01  Sun May 12 19:39:15 UTC 2024\n";
        assert_eq!(nvidia_major_version(old), Some(535));
        assert_eq!(nvidia_major_version("nothing here"), None);
    }
}

pub fn run() {
    // reqwest is built without a bundled crypto provider; share ring with the updater
    let _ = rustls::crypto::ring::default_provider().install_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
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
            // The song's position for the title bar's visualizer, when it asks.
            app.manage(commands::osu_reader::audio::OsuAudioState::default());
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
            commands::system::osu_path::clear_osu_path,
            commands::system::songs_file::read_songs_file,
            commands::system::update::check_update,
            commands::system::update::install_update,
            commands::system::session::is_wayland,
            commands::system::session::show_notification,
            commands::system::session::log_frontend,
            commands::collections::reader::read_osu_collections,
            commands::system::text_file::write_text_file,
            commands::collections::writer::import_collections,
            commands::osu_db::reader::read_osu_db_full,
            commands::system::download::start_downloads,
            commands::system::download::get_osu_session,
            commands::system::download::set_osu_session,
            commands::packs::folders::validate_pack_folder,
            commands::packs::creator::create_pack,
            commands::dev::logs::read_logs,
            commands::osu_reader::connect_osu_reader,
            commands::osu_reader::audio::watch_osu_audio,
            commands::osu_reader::export::export_selected_osz,
            commands::trainer::run::run_trainer,
            commands::trainer::preview::preview_converts,
            commands::trainer::naming::trainer_name_example,
            commands::packs::naming::pack_diff_names,
            commands::packs::naming::pack_name_example,
            commands::trainer::presets::read_trainer_presets,
            commands::trainer::presets::write_trainer_presets,
            commands::trainer::video::is_ffmpeg_available,
            commands::replays::list_local_scores,
            commands::replays::analyze_replay
        ])
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
    use crate::test_utils::TestDir;

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