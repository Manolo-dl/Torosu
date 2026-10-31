use std::{fs, path::{Path, PathBuf}};
use rosu_map::{Beatmap, section::general::GameMode};
use tauri::Emitter;
use crate::commands::trainer::{
    audio_processing,
    model::{Density, DifficultySettings, RunTrainerRequest, TrainerTaskParams, TrainerProgress},
    naming, osu_fixups, pipeline, storyboard, video,
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

    // In mania CS is the key count: changing it would break the map, and the
    // key converter sets it itself.
    let mut request = request;
    if beatmap.mode == GameMode::Mania {
        request.difficulty.cs = original_difficulty.cs;
    }

    // Extra notes only apply when converting to more keys; dropped here so
    // the diff name does not claim them either.
    for task in request.tasks.iter_mut() {
        if let TrainerTaskParams::KeyConverter(params) = &mut task.task {
            if params.target_keys as f64 <= original_difficulty.cs.round() {
                params.density = Density::None;
            }
        }
    }

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
    let mut events = storyboard::scale_events(&events, rate);

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
        request.tasks.iter().any(|t| matches!(t.task, TrainerTaskParams::SVNormalizer(_))),
        request.tasks.iter().find_map(|t| match &t.task {
            TrainerTaskParams::FlnConverter(params) => Some(params),
            _ => None,
        }),
        request.tasks.iter().find_map(|t| match &t.task {
            TrainerTaskParams::KeyConverter(params) => Some(params),
            _ => None,
        }),
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
    // thread so the tokio runtime is not stalled.

    let mut audio_entry: Option<(String, Vec<u8>)> = None;

    // The video is re-encoded after the audio, so the audio gets less of the bar.
    let scale_video = rate_task.as_ref().is_some_and(|p| p.scale_video);
    let (audio_span, audio_done) = if scale_video { (25.0, 58) } else { (55.0, 88) };

    if let Some(params) = &rate_task {
        log::debug!("processing audio for rate change: rate={}, adjust_pitch={}", params.rate, params.adjust_pitch);

        let audio_src = beatmap_folder.join(&beatmap.audio_file);
        let rate = params.rate;
        let adjust_pitch = params.adjust_pitch;
        let app_audio = app.clone();

        let audio_bytes = tokio::task::spawn_blocking(move || {
            let decoded = audio_processing::decode_audio(&audio_src)?;
            let channels = decoded.channels as u16;
            let _ = app_audio.emit("trainer-progress", TrainerProgress { percent: 30 });

            let (samples, sample_rate) = if adjust_pitch {
                let sample_rate = decoded.sample_rate;
                let app_stretch = app_audio.clone();
                let stretched = audio_processing::apply_timestretch(decoded, rate, |p| {
                    let pct = (30.0 + p * audio_span) as u8;
                    let _ = app_stretch.emit("trainer-progress", TrainerProgress { percent: pct });
                })?;
                (stretched, sample_rate)
            } else {
                audio_processing::apply_resample(decoded, rate)
            };

            let _ = app_audio.emit("trainer-progress", TrainerProgress { percent: audio_done });

            // Vorbis encoding is CPU-bound too, so it stays on the blocking pool.
            audio_processing::encode_audio(&samples, sample_rate, channels)
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

    // osu! plays the video at 1x whatever the rate, so it is re-encoded at the
    // new speed into a new file; the other diffs keep the original one.
    let mut video_entry: Option<(String, TempFile)> = None;
    let mut original_video: Option<String> = None;

    if let Some(params) = rate_task.as_ref().filter(|p| p.scale_video) {
        match storyboard::video_file(&events) {
            Some(file) if beatmap_folder.join(&file).is_file() => {
                let src = beatmap_folder.join(&file);
                let name = naming::video_filename(&file, params);
                let ext = name.rsplit_once('.').map_or("mp4", |(_, ext)| ext);
                let tmp = TempFile::new(ext);

                log::debug!("re-encoding video {:?} at {}x into {:?}", src, params.rate, tmp.0);

                let rate = params.rate;
                let dst = tmp.0.clone();
                let app_video = app.clone();

                tokio::task::spawn_blocking(move || {
                    video::apply_rate(&src, &dst, rate, |p| {
                        let pct = (60.0 + p * 30.0) as u8;
                        let _ = app_video.emit("trainer-progress", TrainerProgress { percent: pct });
                    })
                })
                .await
                .map_err(|e| format!("video processing task panicked: {}", e))?
                .map_err(|e| {
                    log::error!("video processing failed: {:#}", e);
                    format!("video processing failed: {:#}", e)
                })?;

                storyboard::set_video_file(&mut events, &name);
                original_video = Some(file);
                video_entry = Some((name, tmp));
            }
            Some(file) => log::warn!("video {:?} not found in {:?}, left as is", file, beatmap_folder),
            None => log::debug!("scale video requested but the beatmap has no video"),
        }
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
    let mut assets = if new_set {
        let skip: Vec<&str> = std::iter::once(original_audio.as_str())
            .chain(original_video.as_deref())
            .collect();

        collect_assets(&beatmap_folder, &skip).map_err(|e| {
            log::error!("failed to list beatmap files in {:?}: {}", beatmap_folder, e);
            e
        })?
    } else {
        Vec::new()
    };

    if let Some((name, tmp)) = &video_entry {
        assets.push((name.clone(), tmp.0.clone()));
    }

    log::debug!("creating osz archive at {:?} ({} extra file(s))", osz_path, assets.len());

    // Copying the assets can mean hundreds of MB of video, so it is kept off
    // the async runtime too.
    let osz_target = osz_path.clone();
    tokio::task::spawn_blocking(move || {
        let result = write_osz(&osz_target, audio_entry.as_ref(), &assets, &new_osu_name, &output_osu);
        // The re-encoded video is already in the .osz (or failed to get there).
        drop(video_entry);
        result
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

/// A file in the system temp dir, removed when dropped.
struct TempFile(PathBuf);

impl TempFile {
    fn new(extension: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let name = format!("torosu-{}-{}.{}", std::process::id(), nanos, extension);
        Self(std::env::temp_dir().join(name))
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Every file of the set except the diffs, the .osb (merged into the new diff),
/// the files in `skip` (replaced by processed ones: audio, video) and the files
/// earlier runs generated, as `(path inside the .osz, path on disk)`.
fn collect_assets(folder: &Path, skip: &[&str]) -> Result<Vec<(String, PathBuf)>, String> {
    let skip: Vec<String> = skip.iter().map(|s| s.replace('\\', "/").to_lowercase()).collect();
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
                || skip.contains(&lower)
                || naming::is_generated_audio(&name)
                || naming::is_generated_video(&name)
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