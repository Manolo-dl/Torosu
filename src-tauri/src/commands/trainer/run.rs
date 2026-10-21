use std::{fs, path::{Path, PathBuf}};
use rosu_map::Beatmap;
use tauri::Emitter;
use crate::commands::trainer::{
    audio_processing,
    model::{DifficultySettings, RunTrainerRequest, TrainerTaskParams, TrainerProgress},
    naming, osu_fixups, pipeline, storyboard,
};

#[tauri::command]
pub async fn run_trainer(app: tauri::AppHandle, request: RunTrainerRequest) -> Result<(), String> {
    log::debug!("run_trainer called for {}", request.osu_file_path);

    macro_rules! emit_progress {
        ($pct:expr) => {
            let _ = app.emit("trainer-progress", TrainerProgress { percent: $pct });
        };
    }

    emit_progress!(5);

    let original_path = Path::new(&request.osu_file_path);
    let beatmap_folder = original_path.parent()
        .ok_or_else(|| {
            log::error!("run_trainer failed: invalid .osu path {:?}", original_path);
            "Invalid .osu path".to_string()
        })?
        .to_path_buf();

    let mut beatmap = rosu_map::from_path::<Beatmap>(original_path).map_err(|e| {
        log::error!("failed to parse .osu file {:?}: {}", original_path, e);
        e.to_string()
    })?;

    // rosu-map drops the storyboard and the video, so they are read raw.
    let osu_events = storyboard::read_events(original_path).unwrap_or_else(|e| {
        log::warn!("failed to read storyboard from {:?}: {}", original_path, e);
        Vec::new()
    });

    // Kept to name the diff after what actually changed.
    let original_version = beatmap.version.clone();
    let original_audio = beatmap.audio_file.clone();
    let original_difficulty = DifficultySettings {
        hp: beatmap.hp_drain_rate as f64,
        cs: beatmap.circle_size as f64,
        od: beatmap.overall_difficulty as f64,
        ar: beatmap.approach_rate as f64,
    };

    beatmap.hp_drain_rate = request.difficulty.hp as f32;
    beatmap.circle_size = request.difficulty.cs as f32;
    beatmap.overall_difficulty = request.difficulty.od as f32;
    beatmap.approach_rate = request.difficulty.ar as f32;

    emit_progress!(10);

    pipeline::run(&mut beatmap, &request.tasks).map_err(|e| {
        log::error!("pipeline execution failed: {}", e);
        e.to_string()
    })?;

    emit_progress!(20);

    let rate_task = request.tasks.iter().find_map(|t| {
        if let TrainerTaskParams::RateChange(params) = &t.task {
            Some(params.clone())
        } else {
            None
        }
    });

    // The .osb is shared by every diff of the set, so it cannot be rewritten in
    // place. It is merged into the new diff instead, and the .osz becomes a new
    // set: otherwise osu! would also load the original .osb, out of sync.
    // Without an .osb (e.g. the storyboard lives in the .osu) there is nothing
    // to overlap with, so the diff stays in the original set.
    let rate = rate_task.as_ref().map_or(1.0, |p| p.rate);

    let osb_events = if rate_task.as_ref().is_some_and(|p| p.scale_storyboard) {
        match storyboard::find_osb(&beatmap_folder, &beatmap.artist, &beatmap.title, &beatmap.creator) {
            Some(osb) => storyboard::read_events(&osb)
                .inspect(|ev| log::debug!("merging {} storyboard line(s) from {:?}", ev.len(), osb))
                .unwrap_or_else(|e| {
                    log::warn!("failed to read storyboard {:?}: {}", osb, e);
                    Vec::new()
                }),
            None => {
                log::debug!("no .osb for this diff in {:?}, keeping it in the original set", beatmap_folder);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let new_set = !osb_events.is_empty();

    let mut events = osu_events;
    events.extend(osb_events);
    let events = storyboard::scale_events(&events, rate);

    if let Some(params) = rate_task.as_ref().filter(|_| new_set) {
        beatmap.title = naming::storyboard_set_title(&beatmap.title, params);
        if !beatmap.title_unicode.is_empty() {
            beatmap.title_unicode = naming::storyboard_set_title(&beatmap.title_unicode, params);
        }
    }

    beatmap.version = naming::version(
        &beatmap,
        &original_version,
        &original_difficulty,
        &request.difficulty,
        rate_task.as_ref(),
    );

    // The .osz is written next to the beatmap folder, i.e. inside Songs, like
    // cosu-trainer does. It only carries the new diff and the new audio: osu!
    // merges it into the existing set on import, so the background, the video
    // and the other diffs stay where they are.
    let songs_dir = beatmap_folder
        .parent()
        .ok_or_else(|| "beatmap folder has no parent (Songs) directory".to_string())?;

    let osz_name = if new_set {
        naming::new_set_osz_name(&beatmap).into()
    } else {
        let mut name = beatmap_folder
            .file_name()
            .ok_or_else(|| "beatmap folder has no name".to_string())?
            .to_os_string();
        name.push(".osz");
        name
    };

    let osz_path = songs_dir.join(osz_name);

    // If there is a RateChange task, process the audio on a dedicated blocking
    // thread so the tokio runtime (and with it tosu's websocket listener) is not
    // stalled.

    let mut audio_entry: Option<(String, Vec<u8>)> = None;

    if let Some(params) = &rate_task {
        log::debug!("processing audio for rate change: rate={}, adjust_pitch={}", params.rate, params.adjust_pitch);

        let audio_src = beatmap_folder.join(&beatmap.audio_file);
        let rate = params.rate;
        let adjust_pitch = params.adjust_pitch;
        let app_audio = app.clone();

        let audio_bytes = tokio::task::spawn_blocking(move || {
            let decoded = audio_processing::decode_audio(&audio_src)?;
            let _ = app_audio.emit("trainer-progress", TrainerProgress { percent: 30 });

            let (samples, sample_rate) = if adjust_pitch {
                let app_stretch = app_audio.clone();
                let stretched = audio_processing::apply_timestretch(&decoded, rate, |p| {
                    let pct = (30.0 + p * 55.0) as u8;
                    let _ = app_stretch.emit("trainer-progress", TrainerProgress { percent: pct });
                })?;
                (stretched, decoded.sample_rate)
            } else {
                audio_processing::apply_resample(&decoded, rate)
            };

            let _ = app_audio.emit("trainer-progress", TrainerProgress { percent: 88 });

            // Vorbis encoding is CPU-bound too, so it stays on the blocking pool.
            audio_processing::encode_audio(&samples, sample_rate, decoded.channels as u16)
        })
        .await
        .map_err(|e| {
            log::error!("audio processing task panicked: {}", e);
            format!("audio processing task panicked: {}", e)
        })?
        .map_err(|e| {
            log::error!("audio processing failed: {}", e);
            e.to_string()
        })?;

        let audio_filename = naming::audio_filename(params);

        log::debug!("audio processing complete: {} ({} bytes)", audio_filename, audio_bytes.len());

        beatmap.audio_file = audio_filename.clone();
        audio_entry = Some((audio_filename, audio_bytes));
    }

    // Write the generated diff, with audio_file already updated if applicable.
    let output_osu = beatmap.encode_to_string().map_err(|e| {
        log::error!("failed to encode beatmap: {}", e);
        e.to_string()
    })?;

    let output_osu = storyboard::inject_events(output_osu, &events);
    let output_osu = osu_fixups::sanitize(output_osu, (!new_set).then_some(beatmap.beatmap_set_id));

    let new_osu_name = naming::osu_filename(&beatmap);

    emit_progress!(94);

    // A new set does not share the original folder, so it needs its own copy of
    // the background, the video, the storyboard sprites and the hitsounds.
    let assets = if new_set {
        collect_assets(&beatmap_folder, &original_audio).map_err(|e| {
            log::error!("failed to list beatmap files in {:?}: {}", beatmap_folder, e);
            e
        })?
    } else {
        Vec::new()
    };

    log::debug!("creating osz archive at {:?} ({} extra file(s))", osz_path, assets.len());

    // Copying the assets can mean hundreds of MB of video, so it is kept off
    // the async runtime too.
    let osz_target = osz_path.clone();
    tokio::task::spawn_blocking(move || {
        write_osz(&osz_target, audio_entry.as_ref(), &assets, &new_osu_name, &output_osu)
    })
    .await
    .map_err(|e| format!("osz writing task panicked: {}", e))?
    .map_err(|e| {
        log::error!("failed to create osz archive {:?}: {}", osz_path, e);
        e
    })?;

    emit_progress!(100);
    log::info!("successfully generated trainer map: {}", beatmap.version);

    Ok(())
}

/// Every file of the set except the diffs, the .osb (merged into the new diff)
/// and the original audio (replaced by the processed one), as
/// `(path inside the .osz, path on disk)`.
fn collect_assets(folder: &Path, original_audio: &str) -> Result<Vec<(String, PathBuf)>, String> {
    let original_audio = original_audio.replace('\\', "/").to_lowercase();
    let mut assets = Vec::new();
    let mut pending = vec![folder.to_path_buf()];

    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();

            if path.is_dir() {
                pending.push(path);
                continue;
            }

            let Ok(relative) = path.strip_prefix(folder) else { continue };
            let name = relative.to_string_lossy().replace('\\', "/");
            let lower = name.to_lowercase();

            if lower.ends_with(".osu")
                || lower.ends_with(".osb")
                || lower.ends_with(".osz")
                || lower == original_audio
                || naming::is_generated_audio(&name)
            {
                continue;
            }

            assets.push((name, path));
        }
    }

    assets.sort();
    Ok(assets)
}

/// Writes the .osz straight from memory: no staging. The set is only copied
/// (`assets`) when exporting it as a new set.
///
/// It is written to a `.part` file first and renamed when complete: osu! picks
/// up any .osz in Songs, and a half-written one imports as a broken set.
fn write_osz(
    output_path: &Path,
    audio: Option<&(String, Vec<u8>)>,
    assets: &[(String, PathBuf)],
    osu_name: &str,
    osu_content: &str,
) -> Result<(), String> {
    let mut part_name = output_path.as_os_str().to_os_string();
    part_name.push(".part");
    let part_path = PathBuf::from(part_name);

    let result = write_osz_contents(&part_path, audio, assets, osu_name, osu_content)
        .and_then(|()| fs::rename(&part_path, output_path).map_err(|e| e.to_string()));

    if result.is_err() {
        let _ = fs::remove_file(&part_path);
    }

    result
}

fn write_osz_contents(
    output_path: &Path,
    audio: Option<&(String, Vec<u8>)>,
    assets: &[(String, PathBuf)],
    osu_name: &str,
    osu_content: &str,
) -> Result<(), String> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    let file = fs::File::create(output_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    if let Some((audio_name, audio_bytes)) = audio {
        zip.start_file(audio_name, options).map_err(|e| e.to_string())?;
        zip.write_all(audio_bytes).map_err(|e| e.to_string())?;
    }

    let audio_name = audio.map(|(name, _)| name.to_lowercase());

    for (name, path) in assets {
        // An earlier run may have left a file with the same name in the folder.
        if audio_name.as_deref() == Some(name.to_lowercase().as_str()) || name.eq_ignore_ascii_case(osu_name) {
            log::debug!("skipping {} from the set folder, it is generated", name);
            continue;
        }

        let mut src = fs::File::open(path).map_err(|e| format!("{}: {}", path.display(), e))?;
        zip.start_file(name, options).map_err(|e| e.to_string())?;
        std::io::copy(&mut src, &mut zip).map_err(|e| e.to_string())?;
    }

    zip.start_file(osu_name, options).map_err(|e| e.to_string())?;
    zip.write_all(osu_content.as_bytes()).map_err(|e| e.to_string())?;

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}