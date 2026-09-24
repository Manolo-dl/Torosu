use std::collections::HashMap;
use std::panic;
use osynic_osudb::entity::osu::osudb::OsuDB;
use crate::{AppConfigState, DbState, commands::osu_db::model::{OsuBeatmapSet, OsuDiff}};
use super::cache;

#[tauri::command]
pub async fn read_osu_db_full(
    config_state: tauri::State<'_, AppConfigState>,
    db_state: tauri::State<'_, DbState>,
) -> Result<Vec<OsuBeatmapSet>, String> {

    let osu_path = config_state.config.lock().unwrap()
        .osu_path.clone()
        .ok_or("osu! path not set")?;

    let use_sqlite = config_state.config.lock().unwrap().sqlite_db.clone();

    let db_path = std::path::Path::new(&osu_path).join("osu!.db");

    
    let metadata = std::fs::metadata(&db_path).map_err(|e| {
        log::error!("Failed to read osu!.db metadata: {:?}: {}", db_path, e);
        e.to_string()
    })?;
    
    let file_size = metadata.len() as i64;
    let last_modified = metadata
    .modified()
    .map_err(|e| e.to_string())?
    .duration_since(std::time::UNIX_EPOCH)
    .map_err(|e| e.to_string())?
    .as_secs() as i64;

    let pool = &db_state.pool;

    if use_sqlite {
        log::debug!("Attempting to read osu!.db from {:?}", db_path);
        if let Some(meta) = cache::get_meta(pool).await {
            if meta.last_modified == last_modified && meta.file_size == file_size {
                log::debug!("loading osu!.db from cache");
                return cache::get_beatmapsets(pool).await.map_err(|e| {
                    log::error!("Failed to read osu!.db from cache: {}", e);
                    e.to_string()
                });
            }
        }
    }

    log::debug!("cache miss, reading osu!.db at {:?}", db_path);
    let sets = read_from_osudb(&osu_path)?;
    log::info!("successfully parsed {} beatmap sets from osu!.db", sets.len());

    if use_sqlite {
        log::debug!("saving osu!.db to cache");
        cache::save_beatmapsets(pool, &sets).await.map_err(|e| {
            log::error!("Failed to save osu!.db to cache: {}", e);
            e.to_string()
        })?;
    
        cache::set_meta(pool, last_modified, file_size).await.map_err(|e| {
            log::error!("Failed to save osu!.db metadata to cache: {}", e);
            e.to_string()
        })?;
    }

    Ok(sets)
}

fn read_from_osudb(osu_path: &str) -> Result<Vec<OsuBeatmapSet>, String> {

    use osynic_osudb::entity::osu::field::mode::Mode;
    use osynic_osudb::entity::osu::field::rank::RankedStatus;

    let db_path = std::path::Path::new(osu_path).join("osu!.db");

    log::debug!("Attempting to read osu!.db from {:?}", db_path);

    let db_path_clone = db_path.clone();
    let db = panic::catch_unwind(move || OsuDB::from_file(&db_path_clone))
        .map_err(|e| {
            log::error!("osu!.db parsing panicked unexpectedly: {:?}", e);
            format!("osu!.db appears to be corrupted at {:?} and could not be parsed", db_path)
        })?
        .map_err(|e| {
            log::error!("Failed to parse osu!.db at {:?}: {}", db_path, e);
            e.to_string()
        })?;

    let mut sets: HashMap<u32, OsuBeatmapSet> = HashMap::new();

    for b in db.beatmaps {

        let md5 = match b.hash {
            Some(h) => h,
            None => continue,
        };

        if b.beatmapset_id < 0 { continue; }

        if matches!(b.status, RankedStatus::Unknown | RankedStatus::Unsubmitted | RankedStatus::Unused) { continue; }

        let stars = match b.mode {
            Mode::Standard => &b.std_ratings,
            Mode::Taiko => &b.taiko_ratings,
            Mode::CatchTheBeat => &b.ctb_ratings,
            Mode::Mania => &b.mania_ratings,
        }.iter().next()
        .map(|(_, s)| *s)
        .unwrap_or(0.0);

        let bpm = b.timing_points.iter()
            .find(|tp| !tp.inherits)
            .map(|tp| (60000.0 / tp.bpm).round())
            .unwrap_or(0.0);

        let status = match b.status {
            RankedStatus::Ranked => "ranked",
            RankedStatus::Loved => "loved",
            RankedStatus::Approved => "approved",
            RankedStatus::Qualified => "qualified",
            RankedStatus::PendingWipGraveyard => "unranked",
            _ => "unknown",
        }.to_string();

        let diff = OsuDiff {
            md5,
            diff_name: b.difficulty_name.unwrap_or_default(),
            mode: b.mode.raw(),
            bpm,
            length: b.total_time,
            stars,
            last_played: b.last_played.is_some(),
            circle_size: b.circle_size,
            approach_rate: b.approach_rate,
            hp_drain: b.hp_drain,
            overall_difficulty: b.overall_difficulty,
            file_name: b.file_name.unwrap_or_default(),
            audio: b.audio.unwrap_or_default(),
            creator: b.creator.unwrap_or_default(),
        };

        sets.entry(b.beatmapset_id as u32)
            .or_insert_with(|| OsuBeatmapSet {
                beatmapset_id: b.beatmapset_id as u32,
                title: b.title_ascii.unwrap_or_default(),
                artist: b.artist_ascii.unwrap_or_default(),
                status: status.to_string(),
                diffs: Vec::new(),
            })
            .diffs.push(diff);
    }

    Ok(sets.into_values().collect())
}