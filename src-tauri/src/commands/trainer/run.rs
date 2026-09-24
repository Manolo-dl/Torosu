use std::{fs, path::Path};

use rosu_map::Beatmap;

use tauri::Emitter;

use crate::commands::trainer::{
    audio_processing, model::{RunTrainerRequest, TrainerTaskParams, TrainerProgress}, osu_fixups, pipeline,
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

    // The .osz is written next to the beatmap folder, i.e. inside Songs, like
    // cosu-trainer does. It only carries the new diff and the new audio: osu!
    // merges it into the existing set on import, so the background, the video
    // and the other diffs stay where they are.
    let songs_dir = beatmap_folder
        .parent()
        .ok_or_else(|| "beatmap folder has no parent (Songs) directory".to_string())?;

    let mut osz_name = beatmap_folder
        .file_name()
        .ok_or_else(|| "beatmap folder has no name".to_string())?
        .to_os_string();
    osz_name.push(".osz");

    let osz_path = songs_dir.join(osz_name);

    // If there is a RateChange task, process the audio on a dedicated blocking
    // thread so the tokio runtime (and with it tosu's websocket listener) is not
    // stalled.
    let rate_task = request.tasks.iter().find_map(|t| {
        if let TrainerTaskParams::RateChange(params) = &t.task {
            Some(params.clone())
        } else {
            None
        }
    });

    let mut audio_entry: Option<(String, Vec<u8>)> = None;

    if let Some(params) = rate_task {
        log::debug!("processing audio for rate change: rate={}, adjust_pitch={}", params.rate, params.adjust_pitch);

        let audio_src = beatmap_folder.join(&beatmap.audio_file);
        let rate = params.rate;
        let adjust_pitch = params.adjust_pitch;
        let app_audio = app.clone();

        let (processed_samples, output_sample_rate, channels) = tokio::task::spawn_blocking(move || {
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
            Ok::<_, anyhow::Error>((samples, sample_rate, decoded.channels))
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

        let wav_filename = format!("audio_trainer_{}x.wav", rate);

        let wav_bytes =
            audio_processing::encode_wav(&processed_samples, output_sample_rate, channels as u16)
                .map_err(|e| {
                    log::error!("failed to encode wav: {}", e);
                    e.to_string()
                })?;

        log::debug!("audio processing complete: {} ({} bytes)", wav_filename, wav_bytes.len());

        beatmap.audio_file = wav_filename.clone();
        audio_entry = Some((wav_filename, wav_bytes));
    }

    // Write the generated diff, with audio_file already updated if applicable.
    let output_osu = beatmap.encode_to_string().map_err(|e| {
        log::error!("failed to encode beatmap: {}", e);
        e.to_string()
    })?;

    let output_osu = osu_fixups::sanitize(output_osu, beatmap.beatmap_set_id);

    let new_osu_name = format!(
        "{} - {} ({}) [{} (trainer)].osu",
        beatmap.artist, beatmap.title, beatmap.creator, beatmap.version
    );

    emit_progress!(94);

    log::debug!("creating osz archive at {:?}", osz_path);
    write_osz(&osz_path, audio_entry.as_ref(), &new_osu_name, &output_osu).map_err(|e| {
        log::error!("failed to create osz archive {:?}: {}", osz_path, e);
        e
    })?;

    emit_progress!(100);
    log::info!("successfully generated trainer map: {}", beatmap.version);

    Ok(())
}

/// Writes the .osz straight from memory: no staging, no copying the set.
fn write_osz(
    output_path: &Path,
    audio: Option<&(String, Vec<u8>)>,
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

    zip.start_file(osu_name, options).map_err(|e| e.to_string())?;
    zip.write_all(osu_content.as_bytes()).map_err(|e| e.to_string())?;

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}