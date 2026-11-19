//! Replay analysis: the plays osu! keeps on this computer and how they went,
//! column by column.

pub mod frames;
pub mod graph;
pub mod judge;
pub mod live;
pub mod scores;
pub mod stats;

use std::path::Path;

use serde::Serialize;
use tauri::AppHandle;

use crate::{error::Result, paths, state::AppConfigState};

use judge::{Chart, Rules, Windows};
use scores::{LocalScore, ManiaCounts};
use stats::Stats;

/// The mania plays osu! keeps on this computer, newest first, with the
/// replay of each one when osu! still has it.
#[tauri::command]
pub async fn list_local_scores(config_state: tauri::State<'_, AppConfigState>) -> Result<Vec<LocalScore>> {
    let osu = config_state.osu_path()?;
    let scores = tauri::async_runtime::spawn_blocking(move || scores::read(&osu)).await??;
    log::info!("read {} local mania scores", scores.len());
    Ok(scores)
}

/// What a replay says about how its map was played.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    /// Of the map the replay was played on.
    pub beatmap_md5: String,
    pub player_name: String,
    pub mods: u32,
    pub keys: usize,
    pub od: f32,
    pub rate: f64,
    /// In real time (ms).
    pub windows: Windows,
    /// See [`frames::ManiaReplay::scroll_speed_scale`].
    pub scroll_speed_scale: Option<f32>,
    /// What the game counted: from the `.osg` when there is one, otherwise
    /// the replay's header. `stats.counts` are the analysis' own.
    pub game_counts: ManiaCounts,
    /// When the game counted each miss (ms, song time), from the `.osg`.
    pub game_misses: Vec<f64>,
    pub stats: Stats,
}

/// Analyses the replay `replay` (a file name in `Data/r`) of the map
/// `beatmap` (`<set folder>/<file>.osu` in the songs folder).
#[tauri::command]
pub async fn analyze_replay(
    app: AppHandle,
    config_state: tauri::State<'_, AppConfigState>,
    replay: String,
    beatmap: String,
) -> Result<Analysis> {
    let replay_path = scores::replays_dir(&config_state.osu_path()?).join(file_name(&replay)?);
    let beatmap_path = paths::songs_dir(&app)?.join(&beatmap);
    tauri::async_runtime::spawn_blocking(move || analyze(&replay_path, &beatmap_path)).await?
}

/// `name` if it is only a file name, so the command reads nothing outside `Data/r`.
fn file_name(name: &str) -> Result<&str> {
    match Path::new(name).file_name() {
        Some(file) if file == name => Ok(name),
        _ => Err(format!("{name} is not the name of a replay").into()),
    }
}

pub fn analyze(replay_path: &Path, beatmap_path: &Path) -> Result<Analysis> {
    let replay = frames::read(replay_path)?;
    let beatmap = rosu_map::from_path(beatmap_path)
        .map_err(|e| format!("could not read {}: {e}", beatmap_path.display()))?;
    let chart = Chart::new(&beatmap, replay.mods)?;
    let rules = Rules::stable(chart.od, replay.mods);
    let rate = judge::rate(replay.mods);

    let judged = judge::judge(&chart, &frames::presses(&replay.frames, chart.keys), &rules);
    let stats = stats::stats(&judged, chart.keys, &rules.windows, rate);

    // An old replay has no `.osg`, and one osu! can't read is no reason to fail.
    let graph = graph::read(&graph::path_of(replay_path)).ok().filter(|graph| !graph.is_empty());
    let game_counts = graph.as_ref().and_then(|g| g.last()).map_or(replay.counts, |point| point.counts);
    let game_misses = graph
        .iter()
        .flat_map(|g| graph::judgements(g))
        .filter(|(_, judgement)| *judgement == judge::Judgement::Miss)
        .map(|(time, _)| f64::from(time) / rate)
        .collect();

    let w = rules.windows;
    Ok(Analysis {
        beatmap_md5: replay.beatmap_md5,
        player_name: replay.player_name,
        mods: replay.mods,
        keys: chart.keys,
        od: chart.od,
        rate,
        windows: Windows {
            max: w.max / rate,
            n300: w.n300 / rate,
            n200: w.n200 / rate,
            n100: w.n100 / rate,
            n50: w.n50 / rate,
            miss: w.miss / rate,
        },
        scroll_speed_scale: replay.scroll_speed_scale,
        game_counts,
        game_misses,
        stats,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{commands::replays::frames::tests::{frames_text, osr}, test_utils::TestDir};

    #[test]
    fn only_takes_file_names() {
        assert!(file_name("abc-123.osr").is_ok());
        assert!(file_name("../scores.db").is_err());
        assert!(file_name("/etc/passwd").is_err());
    }

    #[test]
    fn analyses_a_replay_of_its_map() {
        let dir = TestDir::new("analyze-replay");
        dir.write(
            "map.osu",
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:4\nOverallDifficulty:8\n\n\
             [TimingPoints]\n0,500,4,2,0,100,1,0\n\n[HitObjects]\n64,192,1000,1,0,0:0:0:0:\n192,192,1500,1,0,0:0:0:0:\n",
        );
        // Key 1 at 1009 (after the -1 of the startup frames) and key 2 never.
        std::fs::write(dir.0.join("play.osr"), osr(3, 0, &frames_text(&[(1010, 0b01), (40, 0)]))).unwrap();

        let analysis = analyze(&dir.0.join("play.osr"), &dir.0.join("map.osu")).unwrap();

        assert_eq!((analysis.keys, analysis.rate), (4, 1.0));
        assert_eq!(analysis.stats.counts, ManiaCounts { max: 1, miss: 1, ..Default::default() });
        assert_eq!(analysis.stats.hits.mean, 9.0);
        assert_eq!(analysis.game_counts, ManiaCounts { max: 30, n300: 10, n200: 4, n100: 2, n50: 1, miss: 3 });
        assert!(analysis.game_misses.is_empty());
    }

    /// With osu! installed: prints the analysis of the latest local replays.
    #[test]
    #[ignore = "needs an osu! folder in OSU_PATH"]
    fn analyses_the_latest_replays() {
        use osynic_osudb::entity::osu::osudb::OsuDB;

        let osu = std::path::PathBuf::from(std::env::var("OSU_PATH").unwrap());
        let db = OsuDB::from_file(osu.join("osu!.db")).unwrap();
        let files: std::collections::HashMap<String, std::path::PathBuf> = db
            .beatmaps
            .into_iter()
            .filter_map(|b| Some((b.hash?, osu.join("Songs").join(b.folder_name?).join(b.file_name?))))
            .collect();

        for score in scores::read(&osu).unwrap().iter().filter(|s| s.replay.is_some()).take(5) {
            let Some(map) = files.get(&score.beatmap_md5) else { continue };
            let replay = scores::replays_dir(&osu).join(score.replay.as_ref().unwrap());
            let Ok(a) = analyze(&replay, map) else { continue };
            let s = &a.stats;
            println!(
                "{}K x{} | hits {} mean {:+.1} median {:+.1} UR {:.0} | releases {} mean {:+.1} | game misses {} ours {}",
                a.keys, a.rate, s.hits.count, s.hits.mean, s.hits.median, s.hits.unstable_rate,
                s.releases.count, s.releases.mean, a.game_counts.miss, s.counts.miss,
            );
            println!(
                "  as osu! shows it: Error {:.2}ms - {:.2}ms avg, UR {:.2}",
                s.combined.early_mean, s.combined.late_mean, s.combined.unstable_rate
            );
            for k in &s.keys {
                println!(
                    "  key {} hits {:4} mean {:+5.1} UR {:4.0} | rel {:3} mean {:+5.1} early {} late {} | miss {}",
                    k.key + 1, k.hits.count, k.hits.mean, k.hits.unstable_rate,
                    k.releases.count, k.releases.mean, k.early_releases, k.late_releases, k.counts.miss,
                );
            }
        }
    }
}
