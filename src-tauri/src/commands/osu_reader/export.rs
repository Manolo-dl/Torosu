use std::{fs, path::{Path, PathBuf}};

use crate::OsuReaderState;

/// Packs the whole set folder of the beatmap selected in osu! into an .osz at
/// `path`, as osu! itself exports it: every file, subfolders included.
#[tauri::command]
pub async fn export_selected_osz(state: tauri::State<'_, OsuReaderState>, path: String) -> Result<(), String> {
    let folder = {
        let snapshot = state.snapshot.lock().unwrap();
        snapshot
            .beatmap
            .as_ref()
            .and_then(|beatmap| beatmap.paths.as_ref())
            .map(|paths| paths.folder.clone())
            .ok_or_else(|| "no beatmap selected in osu!, or its folder could not be found".to_string())?
    };

    let output = PathBuf::from(path);
    log::info!("exporting {:?} to {:?}", folder, output);

    tauri::async_runtime::spawn_blocking(move || write_osz(&folder, &output))
        .await
        .map_err(|e| format!("osz export task panicked: {}", e))?
        .map_err(|e| {
            log::error!("failed to export the selected beatmap: {}", e);
            e
        })
}

/// Written to a `.part` file first and renamed when complete, so a failed
/// export never leaves a broken .osz behind.
fn write_osz(folder: &Path, output: &Path) -> Result<(), String> {
    let mut part_name = output.as_os_str().to_os_string();
    part_name.push(".part");
    let part_path = PathBuf::from(part_name);

    let result = write_osz_contents(folder, &part_path)
        .and_then(|()| fs::rename(&part_path, output).map_err(|e| e.to_string()));

    if result.is_err() {
        let _ = fs::remove_file(&part_path);
    }

    result
}

fn write_osz_contents(folder: &Path, output: &Path) -> Result<(), String> {
    use zip::write::SimpleFileOptions;

    let file = fs::File::create(output).map_err(|e| format!("{}: {}", output.display(), e))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let mut pending = vec![folder.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).map_err(|e| format!("{}: {}", dir.display(), e))? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }

            let Ok(relative) = path.strip_prefix(folder) else { continue };
            let name = relative.to_string_lossy().replace('\\', "/");
            // Leftovers of earlier exports or trainer runs, not part of the set.
            let lower = name.to_lowercase();
            if lower.ends_with(".osz") || lower.ends_with(".part") {
                continue;
            }

            let mut src = fs::File::open(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
            zip.start_file(name, options).map_err(|e| e.to_string())?;
            std::io::copy(&mut src, &mut zip).map_err(|e| e.to_string())?;
        }
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}
