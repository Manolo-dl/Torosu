use std::collections::HashMap;
use std::panic;
use std::path::Path;
use osynic_osudb::entity::osu::osudb::OsuDB;
use crate::{AppConfigState, commands::osu_db::model::{OsuBeatmapSet, OsuDiff}, error::Result};

#[tauri::command]
pub async fn read_osu_db_full(
    config_state: tauri::State<'_, AppConfigState>,
) -> Result<Vec<OsuBeatmapSet>> {

    let osu_path = config_state.osu_path()?;

    let sets = tauri::async_runtime::spawn_blocking(move || read_from_osudb(&osu_path)).await??;

    log::info!("successfully parsed {} beatmap sets from osu!.db", sets.len());

    Ok(sets)
}

fn read_from_osudb(osu_path: &Path) -> Result<Vec<OsuBeatmapSet>> {

    use osynic_osudb::entity::osu::field::mode::Mode;
    use osynic_osudb::entity::osu::field::rank::RankedStatus;

    let db_path = osu_path.join("osu!.db");

    log::debug!("Attempting to read osu!.db from {:?}", db_path);

    let db_path_clone = db_path.clone();
    let db = panic::catch_unwind(move || OsuDB::from_file(&db_path_clone))
        .map_err(|e| {
            log::error!("osu!.db parsing panicked unexpectedly: {:?}", e);
            format!("osu!.db appears to be corrupted at {:?} and could not be parsed", db_path)
        })?
        .map_err(|e| {
            log::error!("Failed to parse osu!.db at {:?}: {}", db_path, e);
            e
        })?;

    let mut sets: HashMap<String, OsuBeatmapSet> = HashMap::new();

    for b in db.beatmaps {

        let md5 = match b.hash {
            Some(h) => h,
            None => continue,
        };

        let folder_name = match b.folder_name {
            Some(f) if !f.is_empty() => f,
            _ => continue,
        };

        if matches!(b.status, RankedStatus::Unused) { continue; }

        let stars = match b.mode {
            Mode::Standard => &b.std_ratings,
            Mode::Taiko => &b.taiko_ratings,
            Mode::CatchTheBeat => &b.ctb_ratings,
            Mode::Mania => &b.mania_ratings,
        }.iter().next()
        .map(|(_, s)| *s)
        .unwrap_or(0.0);

        let status = match b.status {
            RankedStatus::Ranked => "ranked",
            RankedStatus::Loved => "loved",
            RankedStatus::Approved => "approved",
            RankedStatus::Qualified => "qualified",
            RankedStatus::PendingWipGraveyard => "unranked",
            RankedStatus::Unsubmitted => "unsubmitted",
            _ => "unknown",
        }.to_string();

        let diff = OsuDiff {
            md5,
            diff_name: b.difficulty_name.unwrap_or_default(),
            mode: b.mode.raw(),
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
            status,
        };

        let set = sets.entry(folder_name.clone())
            .or_insert_with(|| OsuBeatmapSet {
                folder_name,
                beatmapset_id: 0,
                title: b.title_ascii.unwrap_or_default(),
                artist: b.artist_ascii.unwrap_or_default(),
                diffs: Vec::new(),
            });

        // Unsubmitted diffs (e.g. a rate of a ranked map) have no id or -1;
        // the set keeps the id of the diffs that do.
        set.beatmapset_id = set.beatmapset_id.max(b.beatmapset_id.max(0) as u32);
        set.diffs.push(diff);
    }

    Ok(sets.into_values().collect())
}