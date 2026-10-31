use std::path::{Path, PathBuf};

use crate::{error::Result, osz::{self, OszWriter}, state::OsuReaderState};

/// Packs the whole set folder of the beatmap selected in osu! into an .osz at
/// `path`, as osu! itself exports it: every file, subfolders included.
#[tauri::command]
pub async fn export_selected_osz(state: tauri::State<'_, OsuReaderState>, path: String) -> Result<()> {
    let folder = {
        let snapshot = state.snapshot.lock().unwrap();
        snapshot
            .beatmap
            .as_ref()
            .and_then(|beatmap| beatmap.paths.as_ref())
            .map(|paths| paths.folder.clone())
            .ok_or("no beatmap selected in osu!, or its folder could not be found")?
    };

    let output = PathBuf::from(path);
    log::info!("exporting {:?} to {:?}", folder, output);

    tauri::async_runtime::spawn_blocking(move || write_osz(&folder, &output))
        .await?
        .map_err(|e| {
            log::error!("failed to export the selected beatmap: {}", e);
            e
        })
}

fn write_osz(folder: &Path, output: &Path) -> Result<()> {
    let mut osz = OszWriter::create(output)?;

    for (name, path) in osz::files_in(folder)? {
        // Leftovers of earlier exports or trainer runs, not part of the set.
        let lower = name.to_lowercase();
        if lower.ends_with(".osz") || lower.ends_with(".part") {
            continue;
        }

        osz.add_file(&name, &path)?;
    }

    osz.finish()
}
