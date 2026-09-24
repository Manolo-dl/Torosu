use std::{fs, path::Path};
use crate::{AppConfigState, DbState};
use crate::commands::osu_db::folders::get_beatmapset_folder;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedDiff {
    pub beatmapset_id: u32,
    pub file_name: String,
    pub audio: String,
    pub new_diff_name: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackRequest {
    pub title: String,
    pub final_creator: String,
    pub diffs: Vec<SelectedDiff>,
}

#[tauri::command]
pub async fn create_pack(
    request: PackRequest,
    db_state: tauri::State<'_, DbState>,
    config_state: tauri::State<'_, AppConfigState>,
) -> Result<(), String> {

    log::debug!("create_pack called with title={}, creator={}, {} diffs", request.title, request.final_creator, request.diffs.len());

    let pool = &db_state.pool;

    let osu_path = config_state.config.lock().unwrap()
        .osu_path.clone()
        .ok_or("osu! path not set")?;

    let use_sqlite = config_state.config.lock().unwrap().sqlite_db.clone();
    
    let safe_title = sanitize(&request.title);
    let temp_dir = std::env::temp_dir().join(format!("osu-pack-{}", safe_title));

    log::debug!("creating temp directory at {:?}", temp_dir);
    fs::create_dir_all(&temp_dir).map_err(|e| {
        log::error!("failed to create temp directory {:?}: {}", temp_dir, e);
        e.to_string()
    })?;

    for (index, diff) in request.diffs.iter().enumerate() {
        log::debug!("processing diff {}/{}: beatmapset_id={}, file_name={}", index + 1, request.diffs.len(), diff.beatmapset_id, diff.file_name);

        let folder = get_beatmapset_folder(pool, use_sqlite, &osu_path, diff.beatmapset_id, &diff.file_name).await?;
        let osu_file_path = Path::new(&folder.folder_path).join(&diff.file_name);

        let content = fs::read_to_string(&osu_file_path).map_err(|e| {
            log::error!("failed to read {:?}: {}", osu_file_path, e);
            format!("Failed to read {}: {}", diff.file_name, e)
        })?;

        let audio_extension = diff.audio.rsplit('.').next().unwrap_or("mp3");
        let new_audio_file_name = format!("audio-{}.{}", index + 1, audio_extension);
        let audio_source = Path::new(&folder.folder_path).join(&diff.audio);
        let audio_destination = temp_dir.join(&new_audio_file_name);

        if audio_source.exists() {
            tokio::fs::copy(&audio_source, &audio_destination).await.map_err(|e| {
                log::error!("failed to copy audio {:?} -> {:?}: {}", audio_source, audio_destination, e);
                e.to_string()
            })?;
            log::debug!("copied audio {:?} to {:?}", audio_source, audio_destination);
        } else {
            log::warn!("audio file not found for diff {}: {:?}", diff.file_name, audio_source);
        }

        let modified_content = modify_osu_metadata(
            &content,
            &request.title,
            &request.final_creator,
            &diff.new_diff_name,
            &new_audio_file_name,
        );

        let new_osu_name = format!("{}.osu", sanitize(&diff.new_diff_name));
        fs::write(temp_dir.join(&new_osu_name), modified_content).map_err(|e| {
            log::error!("failed to write {:?}: {}", temp_dir.join(&new_osu_name), e);
            e.to_string()
        })?;

        log::debug!("wrote modified .osu file: {}", new_osu_name);
    }

    let osz_path = std::env::temp_dir().join(format!("{}.osz", safe_title));
    log::debug!("creating osz archive at {:?}", osz_path);

    create_osz(&temp_dir, &osz_path).map_err(|e| {
        log::error!("failed to create osz archive {:?}: {}", osz_path, e);
        e
    })?;

    log::debug!("opening osz with default handler: {:?}", osz_path);

    if let Err(e) = open::that(&osz_path) {
        log::warn!("default handler failed to open .osz ({}), trying osu-wine --osuhandler", e);

        let osu_wine_result = std::process::Command::new("osu-wine")
            .arg("--osuhandler")
            .arg(&osz_path)
            .spawn();

        match osu_wine_result {
            Ok(_) => {
                log::debug!("opened .osz via osu-wine --osuhandler");
            }
            Err(wine_err) => {
                log::error!("failed to open .osz with both default handler and osu-wine: {} / {}", e, wine_err);
                return Err(format!("Failed to open .osz: {}", e));
            }
        }
    } else {
        log::debug!("opened .osz with default handler");
    }

    if let Err(e) = fs::remove_dir_all(&temp_dir) {
        log::warn!("failed to clean up temp directory {:?}: {}", temp_dir, e);
    } else {
        log::debug!("cleaned up temp directory {:?}", temp_dir);
    }

    log::info!("successfully created pack '{}' with {} diffs", request.title, request.diffs.len());

    Ok(())
}

fn modify_osu_metadata(
    content: &str,
    title: &str,
    creator: &str,
    version: &str,
    audio_filename: &str,
) -> String {
    let mut result = String::with_capacity(content.len());
    let mut in_events_section = false;
    let mut background_line_handled = false;

    for line in content.lines() {
        let trimmed = line.trim_start();

        if trimmed.starts_with('[') {
            in_events_section = trimmed.starts_with("[Events]");
            result.push_str(line);
            result.push('\n');
            continue;
        }

        if in_events_section
            && !background_line_handled
            && !trimmed.starts_with("//")
            && !trimmed.is_empty()
            && trimmed.contains(',')
        {
            background_line_handled = true;
            continue;
        }

        if trimmed.starts_with("AudioFilename:") {
            result.push_str(&format!("AudioFilename: {}", audio_filename));
        } else if trimmed.starts_with("Title:") && !trimmed.starts_with("TitleUnicode:") {
            result.push_str(&format!("Title:{}", title));
        } else if trimmed.starts_with("TitleUnicode:") {
            result.push_str(&format!("TitleUnicode:{}", title));
        } else if trimmed.starts_with("Creator:") {
            result.push_str(&format!("Creator:{}", creator));
        } else if trimmed.starts_with("Version:") {
            result.push_str(&format!("Version:{}", version));
        } else if trimmed.starts_with("BeatmapID:") {
            continue;
        } else if trimmed.starts_with("BeatmapSetID:") {
            continue;
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    result
}

fn create_osz(source_dir: &Path, output_path: &Path) -> Result<(), String> {
    use std::io::{Read, Write};
    use zip::write::SimpleFileOptions;

    let file = std::fs::File::create(output_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for entry in fs::read_dir(source_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.is_file() {
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .ok_or("Invalid file name")?;

            zip.start_file(file_name, options).map_err(|e| e.to_string())?;

            let mut f = std::fs::File::open(&path).map_err(|e| e.to_string())?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
            zip.write_all(&buffer).map_err(|e| e.to_string())?;
        }
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if "/\\:*?\"<>|".contains(c) { '_' } else { c })
        .collect()
}