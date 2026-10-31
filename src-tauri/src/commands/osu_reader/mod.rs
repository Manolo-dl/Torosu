pub mod export;

use std::{sync::atomic::Ordering, thread, time::Duration};

use osu_memory_reader::{Beatmap, BeatmapAttributes, BeatmapPaths, Error, OsuReader};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::state::{AppConfigState, OsuReaderState};

/// How often the selected beatmap is read while osu! is open.
const POLL_INTERVAL: Duration = Duration::from_millis(250);
/// How often to look for osu! while it is closed or still loading.
const ATTACH_INTERVAL: Duration = Duration::from_secs(2);

/// The beatmap selected in osu!, sent to the frontend as `osu-beatmap`.
#[derive(Clone, Serialize)]
pub struct SelectedBeatmap {
    pub beatmap: Beatmap,
    /// `None` if the osu! folder could not be found.
    pub paths: Option<BeatmapPaths>,
    /// `None` if the `.osu` file could not be read.
    pub attributes: Option<BeatmapAttributes>,
}

/// State of the reader, sent to the frontend as `osu-reader-status`.
#[derive(Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum ReaderStatus {
    /// osu! is not running, or is still loading.
    Waiting,
    Connected,
    /// osu! is running but its memory can't be read, usually for lack of permissions.
    Error { message: String },
}

#[derive(Clone, Serialize)]
pub struct ReaderSnapshot {
    pub status: ReaderStatus,
    pub beatmap: Option<SelectedBeatmap>,
}

impl ReaderSnapshot {
    pub fn new() -> Self {
        Self { status: ReaderStatus::Waiting, beatmap: None }
    }
}

/// Starts reading osu! in the background (once) and returns what is known so
/// far. Later changes arrive as `osu-reader-status` and `osu-beatmap` events,
/// so the frontend should listen to them before calling this.
#[tauri::command]
pub fn connect_osu_reader(app: AppHandle, state: tauri::State<'_, OsuReaderState>) -> crate::error::Result<ReaderSnapshot> {
    let snapshot = state.snapshot.lock().unwrap().clone();

    if !state.started.swap(true, Ordering::SeqCst) {
        thread::Builder::new()
            .name("osu-reader".into())
            .spawn(move || run(app))
            .map_err(|e| format!("failed to start the osu! reader: {e}"))?;
    }

    Ok(snapshot)
}

fn run(app: AppHandle) {
    loop {
        let mut reader = match OsuReader::attach() {
            Ok(reader) => reader,
            Err(e) => {
                set_status(&app, status_for(&e));
                thread::sleep(ATTACH_INTERVAL);
                continue;
            }
        };

        if reader.game_folder().is_none() {
            let osu_path = app.state::<AppConfigState>().config.lock().unwrap().osu_path.clone();
            if let Some(path) = osu_path {
                reader.set_game_folder(path);
            }
        }

        log::info!("attached to osu! (pid {})", reader.pid());
        set_status(&app, ReaderStatus::Connected);

        let mut last_checksum = String::new();
        while reader.is_running() {
            // Isolated read errors are normal while osu! swaps beatmaps; the next poll retries.
            if let Ok(Some(beatmap)) = reader.beatmap() {
                if beatmap.checksum != last_checksum {
                    last_checksum = beatmap.checksum.clone();
                    set_beatmap(&app, read_selected(&reader, beatmap));
                }
            }
            thread::sleep(POLL_INTERVAL);
        }

        log::info!("osu! closed, waiting for it to start again");
        set_status(&app, ReaderStatus::Waiting);
    }
}

fn status_for(error: &Error) -> ReaderStatus {
    match error {
        Error::ProcessNotFound | Error::PatternNotFound { .. } => ReaderStatus::Waiting,
        _ => ReaderStatus::Error { message: error.to_string() },
    }
}

fn read_selected(reader: &OsuReader, beatmap: Beatmap) -> SelectedBeatmap {
    let paths = reader
        .beatmap_paths(&beatmap)
        .map_err(|e| log::warn!("could not resolve the beatmap paths: {e}"))
        .ok();

    // Always in the map's own mode: osu!'s automatic converts are not used.
    let attributes = paths.as_ref().and_then(|paths| {
        osu_memory_reader::attributes(paths, None)
            .map_err(|e| log::warn!("could not compute the beatmap attributes: {e}"))
            .ok()
    });

    SelectedBeatmap { beatmap, paths, attributes }
}

fn set_status(app: &AppHandle, status: ReaderStatus) {
    let state = app.state::<OsuReaderState>();
    let mut snapshot = state.snapshot.lock().unwrap();
    if snapshot.status == status {
        return;
    }

    if let ReaderStatus::Error { message } = &status {
        log::error!("could not read osu!: {message}");
    }
    snapshot.status = status.clone();
    let _ = app.emit("osu-reader-status", status);
}

fn set_beatmap(app: &AppHandle, beatmap: SelectedBeatmap) {
    let state = app.state::<OsuReaderState>();
    state.snapshot.lock().unwrap().beatmap = Some(beatmap.clone());
    let _ = app.emit("osu-beatmap", beatmap);
}
