use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use futures_util::future::{try_join, try_join_all};
use rosu_map::{Beatmap, section::general::GameMode};
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::{
    config::model::TrainerConfig,
    trainer::{
        audio_processing,
        model::{Density, DifficultySettings, RateChangeParams, RunTrainerRequest, TrainerProgress, TrainerTask, TrainerTaskParams},
        naming::{self, NameInfo},
        osu_fixups, pipeline, storyboard, timing, video,
    },
};
use crate::{
    error::{Error, Result},
    osz::{self, OszWriter},
    state::AppConfigState,
};

/// Generates a diff for each convert of `request`, all with the same
/// difficulty settings, and writes them as .osz into Songs, like cosu-trainer
/// does. Returns the names of the new diffs.
///
/// The ones that stay in the original set go into one .osz named after its
/// folder, which osu! merges into it on import: the background, the video and
/// the other diffs stay where they are. A convert that scales a shared .osb
/// goes into a new set instead (see [`build_convert`]).
#[tauri::command]
pub async fn run_trainer(app: AppHandle, request: RunTrainerRequest) -> Result<Vec<String>> {
    log::debug!("run_trainer called for {} with {} convert(s)", request.osu_file_path, request.converts.len());
    if request.converts.is_empty() {
        return Err("there is nothing to convert".into());
    }

    let config = Arc::new(app.state::<AppConfigState>().config.lock().unwrap().trainer.clone());
    let progress = Progress::new(app.clone());
    progress.set(5);

    let source = Arc::new(Source::read(Path::new(&request.osu_file_path), request.difficulty)?);

    // The tasks, the key converter above all, are CPU work: each convert on
    // a blocking thread of its own.
    let converts = try_join_all(request.converts.into_iter().enumerate().map(|(i, convert)| {
        let (source, config) = (source.clone(), config.clone());
        async move {
            tauri::async_runtime::spawn_blocking(move || build_convert(&source, i + 1, convert.tasks, &config)).await?
        }
    }))
    .await?;
    progress.set(15);

    check_names(&converts)?;
    let names: Vec<String> = converts.iter().map(|c| c.version.clone()).collect();

    let media = make_media(&source, &converts, &config, &progress).await?;
    progress.set(90);

    let paths = write_osz_files(source, converts, media).await?;
    progress.set(100);
    log::info!("generated {} diff(s) into {} .osz: {:?}", names.len(), paths.len(), names);

    if config.open_osz {
        // The .osz is in Songs already, where osu! also finds it: not opening
        // it only delays the import.
        for path in &paths {
            if let Err(e) = osz::open(path) {
                log::warn!("could not open {:?}: {}", path, e);
            }
        }
    }

    Ok(names)
}

/// The map the converts start from.
struct Source {
    folder: PathBuf,
    /// The map with the shared difficulty settings applied.
    beatmap: Beatmap,
    /// Its `[Events]`, read raw: rosu-map drops the storyboard and the video.
    events: Vec<String>,
    /// What it had before, to name the converts after what changed.
    version: String,
    audio: String,
    original: DifficultySettings,
    difficulty: DifficultySettings,
}

impl Source {
    fn read(path: &Path, mut difficulty: DifficultySettings) -> Result<Self> {
        let folder = path
            .parent()
            .ok_or_else(|| {
                log::error!("run_trainer failed: invalid .osu path {:?}", path);
                "Invalid .osu path".to_string()
            })?
            .to_path_buf();

        let mut beatmap = rosu_map::from_path::<Beatmap>(path).map_err(|e| {
            log::error!("failed to parse .osu file {:?}: {}", path, e);
            e
        })?;

        let events = storyboard::read_events(path).unwrap_or_else(|e| {
            log::warn!("failed to read storyboard from {:?}: {}", path, e);
            Vec::new()
        });

        let original = DifficultySettings {
            hp: beatmap.hp_drain_rate as f64,
            cs: beatmap.circle_size as f64,
            od: beatmap.overall_difficulty as f64,
            ar: beatmap.approach_rate as f64,
        };

        // In mania CS is the key count: changing it would break the map, and
        // the key converter sets it itself.
        if beatmap.mode == GameMode::Mania {
            difficulty.cs = original.cs;
        }

        beatmap.hp_drain_rate = difficulty.hp as f32;
        beatmap.circle_size = difficulty.cs as f32;
        beatmap.overall_difficulty = difficulty.od as f32;
        beatmap.approach_rate = difficulty.ar as f32;

        Ok(Self {
            folder,
            version: beatmap.version.clone(),
            audio: beatmap.audio_file.clone(),
            beatmap,
            events,
            original,
            difficulty,
        })
    }
}

/// A convert with its tasks applied, ready to be written but for its audio
/// and video.
struct Convert {
    /// From 1, as the trainer shows them.
    number: usize,
    /// Its name (`Version`).
    version: String,
    /// The .osz it goes into.
    osz: PathBuf,
    new_set: bool,
    osu_name: String,
    osu: String,
    /// The audio its rate change generates, by its name in the set.
    audio: Option<(String, RateChangeParams)>,
    video: Option<VideoJob>,
}

/// A video re-encoded at a rate, into `name` next to the `original`.
#[derive(Clone)]
struct VideoJob {
    original: String,
    name: String,
    rate: f64,
}

/// Applies the tasks of convert `number` to a copy of the map and makes its
/// .osu.
fn build_convert(source: &Source, number: usize, mut tasks: Vec<TrainerTask>, config: &TrainerConfig) -> Result<Convert> {
    for task in tasks.iter_mut() {
        match &mut task.task {
            // Extra notes only apply when converting to more keys; dropped
            // here so the name does not claim them either.
            TrainerTaskParams::KeyConverter(params) if params.target_keys as f64 <= source.original.cs.round() => {
                params.density = Density::None;
            }
            // Kept below 1.0x, raised with the speed above.
            TrainerTaskParams::RateChange(params) if config.auto_pitch => params.adjust_pitch = params.rate < 1.0,
            _ => {}
        }
    }

    let mut beatmap = source.beatmap.clone();
    pipeline::run(&mut beatmap, &tasks).map_err(|e| {
        log::error!("convert {}: pipeline execution failed: {}", number, e);
        format!("Convert {}: {}", number, e)
    })?;

    let rate = tasks.iter().find_map(|t| match &t.task {
        TrainerTaskParams::RateChange(params) => Some(params.clone()),
        _ => None,
    });

    // The .osb is shared by every diff of the set, so it cannot be rewritten
    // in place. It is merged into the new diff instead, and the .osz becomes a
    // new set: otherwise osu! would also load the original .osb, out of sync.
    // Without an .osb (e.g. the storyboard lives in the .osu) there is nothing
    // to overlap with, so the diff stays in the original set.
    let osb_events = match rate.as_ref().filter(|p| p.scale_storyboard) {
        Some(_) => match storyboard::find_osb(&source.folder, &beatmap.artist, &beatmap.title, &beatmap.creator) {
            Some(osb) => storyboard::read_events(&osb)
                .inspect(|ev| log::debug!("merging {} storyboard line(s) from {:?}", ev.len(), osb))
                .unwrap_or_else(|e| {
                    log::warn!("failed to read storyboard {:?}: {}", osb, e);
                    Vec::new()
                }),
            None => {
                log::debug!("no .osb for this diff in {:?}, keeping it in the original set", source.folder);
                Vec::new()
            }
        },
        None => Vec::new(),
    };
    let new_set = !osb_events.is_empty();

    let mut events = source.events.clone();
    events.extend(osb_events);
    let mut events = storyboard::scale_events(&events, rate.as_ref().map_or(1.0, |p| p.rate));

    if let Some(params) = rate.as_ref().filter(|_| new_set) {
        beatmap.title = naming::storyboard_set_title(&beatmap.title, params);
        if !beatmap.title_unicode.is_empty() {
            beatmap.title_unicode = naming::storyboard_set_title(&beatmap.title_unicode, params);
        }
    }

    beatmap.version = naming::version(&config.name_format, &NameInfo {
        original_version: &source.version,
        original: &source.original,
        new: &source.difficulty,
        rate: rate.as_ref(),
        bpm: timing::main_bpm(&beatmap),
        sv_normalized: tasks.iter().any(|t| matches!(t.task, TrainerTaskParams::SVNormalizer(_))),
        fln: tasks.iter().find_map(|t| match &t.task {
            TrainerTaskParams::FlnConverter(params) => Some(params),
            _ => None,
        }),
        keys: tasks.iter().find_map(|t| match &t.task {
            TrainerTaskParams::KeyConverter(params) => Some(params),
            _ => None,
        }),
    });

    let audio = rate.as_ref().map(|params| (naming::audio_filename(params), params.clone()));
    if let Some((name, _)) = &audio {
        beatmap.audio_file = name.clone();
    }

    // osu! plays the video at 1x whatever the rate, so it is re-encoded at the
    // new speed into a new file; the other diffs keep the original one.
    let video = match rate.as_ref().filter(|p| p.scale_video) {
        Some(params) => match storyboard::video_file(&events) {
            Some(file) if source.folder.join(&file).is_file() => {
                let name = naming::video_filename(&file, params);
                storyboard::set_video_file(&mut events, &name);
                Some(VideoJob { original: file, name, rate: params.rate })
            }
            Some(file) => {
                log::warn!("video {:?} not found in {:?}, left as is", file, source.folder);
                None
            }
            None => {
                log::debug!("scale video requested but the beatmap has no video");
                None
            }
        },
        None => None,
    };

    let osu = beatmap.encode_to_string().map_err(|e| {
        log::error!("failed to encode beatmap: {}", e);
        e
    })?;
    let osu = storyboard::inject_events(osu, &events);
    let osu = osu_fixups::sanitize(osu, (!new_set).then_some(beatmap.beatmap_set_id));

    let songs = source.folder.parent().ok_or("beatmap folder has no parent (Songs) directory")?;
    let osz = if new_set {
        songs.join(naming::new_set_osz_name(&beatmap))
    } else {
        let mut name = source.folder.file_name().ok_or("beatmap folder has no name")?.to_os_string();
        name.push(".osz");
        songs.join(name)
    };

    Ok(Convert {
        number,
        version: beatmap.version.clone(),
        osz,
        new_set,
        osu_name: naming::osu_filename(&beatmap),
        osu,
        audio,
        video,
    })
}

/// Two converts with the same name in the same .osz would be the same .osu
/// file, one overwriting the other: nothing is written then.
fn check_names(converts: &[Convert]) -> Result<()> {
    let mut seen: HashMap<(&Path, String), usize> = HashMap::new();
    for convert in converts {
        if let Some(first) = seen.insert((&convert.osz, convert.osu_name.to_lowercase()), convert.number) {
            return Err(format!(
                "Converts {} and {} make the same difficulty, \"{}\": change one of them.",
                first, convert.number, convert.version
            )
            .into());
        }
    }
    Ok(())
}

/// The generated audio and videos, by their name in the set.
struct Media {
    audio: HashMap<String, Vec<u8>>,
    videos: HashMap<String, TempFile>,
}

/// Makes every distinct audio and video the converts use, all at the same
/// time, each on a blocking thread. The audio is decoded once for all of them.
async fn make_media(source: &Source, converts: &[Convert], config: &TrainerConfig, progress: &Progress) -> Result<Media> {
    let audio_jobs: HashMap<String, RateChangeParams> = converts.iter().filter_map(|c| c.audio.clone()).collect();
    let video_jobs: HashMap<String, VideoJob> =
        converts.iter().filter_map(|c| c.video.clone()).map(|v| (v.name.clone(), v)).collect();

    // Progress jobs: decoding the audio first, then each audio, then each video.
    let decoding = usize::from(!audio_jobs.is_empty());
    let first_video = decoding + audio_jobs.len();
    progress.start_jobs(first_video + video_jobs.len());

    let quality = audio_processing::vorbis_quality(config.high_quality_audio);
    let audio_src = source.folder.join(&source.audio);

    let audio = async {
        if audio_jobs.is_empty() {
            return Ok(HashMap::new());
        }
        let decoded = tauri::async_runtime::spawn_blocking(move || audio_processing::decode_audio(&audio_src)).await??;
        let decoded = Arc::new(decoded);
        progress.job(0, 1.0);

        let audio = try_join_all(audio_jobs.into_iter().enumerate().map(|(i, (name, params))| {
            let (decoded, progress, job) = (decoded.clone(), progress.clone(), decoding + i);
            log::debug!("making {} (rate={}, adjust_pitch={})", name, params.rate, params.adjust_pitch);
            async move {
                let bytes = tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
                    let channels = decoded.channels as u16;
                    let bytes = if params.adjust_pitch {
                        // Encoding is the last fifth.
                        let samples = audio_processing::apply_timestretch(&decoded, params.rate, |p| progress.job(job, p * 0.8))?;
                        audio_processing::encode_audio(&samples, decoded.sample_rate, channels, quality)?
                    } else if params.rate > 1.0 {
                        let samples = audio_processing::apply_rate_change(&decoded, params.rate, |p| progress.job(job, p * 0.8));
                        audio_processing::encode_audio(&samples, decoded.sample_rate, channels, quality)?
                    } else {
                        let sample_rate = audio_processing::resampled_rate(decoded.sample_rate, params.rate);
                        audio_processing::encode_audio(&decoded.samples, sample_rate, channels, quality)?
                    };
                    progress.job(job, 1.0);
                    Ok(bytes)
                })
                .await?
                .map_err(|e| {
                    log::error!("audio processing failed: {}", e);
                    e
                })?;
                log::debug!("audio processing complete: {} ({} bytes)", name, bytes.len());
                Ok::<_, Error>((name, bytes))
            }
        }))
        .await?;
        Ok::<_, Error>(audio.into_iter().collect())
    };

    let videos = try_join_all(video_jobs.into_values().enumerate().map(|(i, job)| {
        let (progress, src, rate) = (progress.clone(), source.folder.join(&job.original), job.rate);
        let high_quality = config.high_quality_video;
        async move {
            let ext = job.name.rsplit_once('.').map_or("mp4", |(_, ext)| ext);
            let tmp = TempFile::new(ext);
            let dst = tmp.0.clone();
            log::debug!("re-encoding video {:?} at {}x into {:?}", src, rate, dst);

            tauri::async_runtime::spawn_blocking(move || {
                video::apply_rate(&src, &dst, rate, high_quality, |p| progress.job(first_video + i, p as f32))
            })
            .await?
            .map_err(|e| {
                log::error!("video processing failed: {:#}", e);
                format!("video processing failed: {:#}", e)
            })?;
            Ok::<_, Error>((job.name, tmp))
        }
    }));

    let (audio, videos) = try_join(audio, videos).await?;
    Ok(Media { audio, videos: videos.into_iter().collect() })
}

/// Writes the .osz files, at the same time, each with its converts. Returns
/// their paths.
async fn write_osz_files(source: Arc<Source>, converts: Vec<Convert>, media: Media) -> Result<Vec<PathBuf>> {
    // By .osz, in the converts' order.
    let mut groups: Vec<Vec<Convert>> = Vec::new();
    for convert in converts {
        match groups.iter_mut().find(|group| group[0].osz == convert.osz) {
            Some(group) => group.push(convert),
            None => groups.push(vec![convert]),
        }
    }

    // Copying the assets can mean hundreds of MB of video, so it is kept off
    // the async runtime. The re-encoded videos are deleted once all are written.
    let media = Arc::new(media);
    try_join_all(groups.into_iter().map(|group| {
        let (source, media) = (source.clone(), media.clone());
        async move { tauri::async_runtime::spawn_blocking(move || write_osz(&source, &group, &media)).await? }
    }))
    .await
}

/// Writes the .osz of `group`, converts that go into the same one: their
/// .osu, the audio and videos they use, and for a new set the rest of the set.
fn write_osz(source: &Source, group: &[Convert], media: &Media) -> Result<PathBuf> {
    let path = &group[0].osz;
    log::debug!("creating osz archive at {:?} ({} diff(s))", path, group.len());

    let mut osz = OszWriter::create(path).map_err(|e| {
        log::error!("failed to create osz archive {:?}: {}", path, e);
        e
    })?;
    // Names in the .osz, lowercase: converts can share their audio and video.
    let mut written: Vec<String> = Vec::new();

    for convert in group {
        if let Some((name, _)) = &convert.audio {
            if !written.contains(&name.to_lowercase()) {
                osz.add_bytes(name, &media.audio[name])?;
                written.push(name.to_lowercase());
            }
        }
        if let Some(video) = &convert.video {
            if !written.contains(&video.name.to_lowercase()) {
                osz.add_file(&video.name, &media.videos[&video.name].0)?;
                written.push(video.name.to_lowercase());
            }
        }
        osz.add_bytes(&convert.osu_name, convert.osu.as_bytes())?;
        written.push(convert.osu_name.to_lowercase());
    }

    // A new set does not share the original folder, so it needs its own copy
    // of the background, the video, the storyboard sprites and the hitsounds.
    // Its converts all change the rate, so the original audio is never played;
    // the original video is, unless every convert replaced it.
    if group[0].new_set {
        let mut skip = vec![source.audio.as_str()];
        if group.iter().all(|c| c.video.is_some()) {
            skip.extend(group[0].video.as_ref().map(|v| v.original.as_str()));
        }

        let assets = collect_assets(&source.folder, &skip).map_err(|e| {
            log::error!("failed to list beatmap files in {:?}: {}", source.folder, e);
            e
        })?;
        for (name, file) in assets {
            // An earlier run may have left a file with the same name in the folder.
            if written.contains(&name.to_lowercase()) {
                log::debug!("skipping {} from the set folder, it is generated", name);
                continue;
            }
            osz.add_file(&name, &file)?;
        }
    }

    osz.finish()?;
    Ok(path.clone())
}

/// Reports how far the run is, as `trainer-progress`: the tasks up to 15%,
/// then the audio and the videos up to 90% (each job an equal share), and
/// writing the .osz the rest.
#[derive(Clone)]
struct Progress {
    app: AppHandle,
    /// How far each job is (0 to 1), and the last percent sent.
    state: Arc<Mutex<(Vec<f32>, u8)>>,
}

impl Progress {
    fn new(app: AppHandle) -> Self {
        Self { app, state: Arc::new(Mutex::new((Vec::new(), 0))) }
    }

    fn set(&self, percent: u8) {
        self.state.lock().unwrap().1 = percent;
        let _ = self.app.emit("trainer-progress", TrainerProgress { percent });
    }

    fn start_jobs(&self, count: usize) {
        self.state.lock().unwrap().0 = vec![0.0; count];
    }

    /// Job `index` is `done` (0 to 1) through.
    fn job(&self, index: usize, done: f32) {
        let mut state = self.state.lock().unwrap();
        let (jobs, last) = &mut *state;
        let Some(job) = jobs.get_mut(index) else { return };
        *job = done.clamp(0.0, 1.0);

        let average = jobs.iter().sum::<f32>() / jobs.len() as f32;
        let percent = 15 + (average * 75.0) as u8;
        if percent > *last {
            *last = percent;
            let _ = self.app.emit("trainer-progress", TrainerProgress { percent });
        }
    }
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
fn collect_assets(folder: &Path, skip: &[&str]) -> Result<Vec<(String, PathBuf)>> {
    let skip: Vec<String> = skip.iter().map(|s| s.replace('\\', "/").to_lowercase()).collect();

    let mut assets: Vec<(String, PathBuf)> = osz::files_in(folder)?
        .into_iter()
        .filter(|(name, _)| {
            let lower = name.to_lowercase();
            !(lower.ends_with(".osu")
                || lower.ends_with(".osb")
                || lower.ends_with(".osz")
                || skip.contains(&lower)
                || naming::is_generated_audio(name)
                || naming::is_generated_video(name))
        })
        .collect();

    assets.sort();
    Ok(assets)
}
