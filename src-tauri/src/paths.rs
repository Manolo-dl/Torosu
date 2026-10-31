//! Where osu!'s folders are. The osu! folder itself comes from the
//! configuration: see `AppConfigState::osu_path`.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::{AppConfigState, OsuReaderState, error::Result};

/// osu!'s Songs folder. osu! can keep the songs somewhere else
/// (`BeatmapDirectory` in its settings): the folder the running osu! uses,
/// once a beatmap was read from its memory, or `Songs` in the osu! folder.
pub fn songs_dir(app: &AppHandle) -> Result<PathBuf> {
    match read_songs_dir(app) {
        Some(songs) => Ok(songs),
        None => Ok(app.state::<AppConfigState>().osu_path()?.join("Songs")),
    }
}

/// Songs folder of the last beatmap read from osu!'s memory.
fn read_songs_dir(app: &AppHandle) -> Option<PathBuf> {
    let state = app.state::<OsuReaderState>();
    let snapshot = state.snapshot.lock().unwrap();
    snapshot.beatmap.as_ref()?.paths.as_ref().map(|paths| paths.songs.clone())
}

/// Folder of a set inside `songs`. `folder_name` comes from osu!.db, so there
/// is nothing to search for; unsubmitted sets (id 0) work the same way.
pub fn beatmapset_folder(songs: &Path, folder_name: &str) -> Result<PathBuf> {
    let folder = songs.join(folder_name);
    if !folder.is_dir() {
        return Err(format!("Folder does not exist: {}", folder.display()).into());
    }
    Ok(folder)
}
