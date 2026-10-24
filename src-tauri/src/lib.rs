use std::{path::PathBuf, sync::{Mutex, atomic::AtomicBool}};
use sqlx::SqlitePool;
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_updater::UpdaterExt;

use crate::commands::config::model::AppConfig;
use crate::commands::osu_reader::ReaderSnapshot;

mod commands;

pub struct DbState {
    pub pool: SqlitePool,
}

pub struct OsuReaderState {
    pub started: AtomicBool,
    pub snapshot: Mutex<ReaderSnapshot>,
}

pub struct AppConfigState {
    pub config: Mutex<AppConfig>,
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

            // Application folder
            let app_dir = app_root_dir();
            std::fs::create_dir_all(&app_dir).unwrap();

            // Load Configuration
            let config = commands::config::file::read_config();
            app.manage(SessionState { osu_session: Mutex::new(config.downloader.osu_session.clone()) });
            app.manage(AppConfigState { config: Mutex::new(config) });

            // Database connection
            let db_path = app_dir.join("osu_cache.db");

            let pool = tauri::async_runtime::block_on(async {
                SqlitePool::connect(&format!("sqlite://{}?mode=rwc", db_path.display()))
                    .await
                    .expect("failed to connect to sqlite")
            });

            tauri::async_runtime::block_on(async {
                commands::osu_db::cache::init_schema(&pool).await.expect("failed to init osudb schema");
                commands::collections::cache::init_schema(&pool).await.expect("failed to init collection schema")
            });

            app.manage(DbState { pool });

            // Update check
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = update(handle).await {
                    log::error!("failed to check for updates: {e}");
                }
            });

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
            .level_for("sqlx", log::LevelFilter::Off)
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
            commands::dev::database::get_database_tables,
            commands::dev::database::get_table_columns,
            commands::dev::database::execute_query,
            commands::osu_reader::connect_osu_reader,
            commands::trainer::run::run_trainer,
            commands::trainer::video::is_ffmpeg_available
        ])
        .on_window_event(|window, event| {
            
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                
                let config_state = window.app_handle().state::<AppConfigState>();
                let config = config_state.config.lock().unwrap().clone();

                if let Err(e) = commands::config::file::write_config(&config) {
                    log::error!("failed to write configuration.toml on close: {e}");
                } else  {
                    log::info!("configuration.toml saved on close");
                }

            }
        }) 
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn update(app: tauri::AppHandle) -> tauri_plugin_updater::Result<()> {
    if let Some(update) = app
        .updater_builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?
        .check()
        .await?
    {
        let mut downloaded = 0;

        update
            .download_and_install(
                |chunk_length, content_length| {
                    downloaded += chunk_length;
                    log::info!("downloaded {downloaded} of {content_length:?}");
                },
                || {
                    log::info!("download complete, installing update...");
                },
            )
            .await?;

        log::info!("update installed");
        app.restart();
    }

    Ok(())
}

pub fn app_root_dir() -> PathBuf {

    // If the application is runnig in AppImage, use the AppImage directory as the root directory
    if let Ok(appimage_path) = std::env::var("APPIMAGE") {
        if let Some(parent) = std::path::Path::new(&appimage_path).parent() {
            log::debug!("APPIMAGE environment variable found, using AppImage directory as root: {}", parent.display());
            return parent.to_path_buf();
        }
    }

    std::env::current_exe()
        .expect("failed to get current exe path")
        .parent()
        .expect("failed to get parent directory")
        .to_path_buf()
}