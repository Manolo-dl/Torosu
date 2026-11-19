//! The plays osu! keeps on this computer: `scores.db` and the replays of
//! `Data/r`.
//!
//! `scores.db` holds the header of every local score (what an .osr starts
//! with) but not its key presses: osu! writes those to
//! `Data/r/<beatmap md5>-<time>.osr`, `<time>` being when the score was set as
//! a Windows FILETIME (100 ns ticks since 1601), and next to it, since 2026, a
//! `.osg` with how the score went (see `graph`).

use std::{collections::HashSet, path::{Path, PathBuf}};

use chrono::{DateTime, Utc};
use osynic_osudb::entity::{osu::field::mode::Mode, scores::{field::replay::Replay, scoresdb::ScoresDB}};
use serde::Serialize;

use crate::error::Result;

/// Ticks of a .NET `DateTime` (100 ns since the year 1) at the Unix epoch.
const UNIX_EPOCH_TICKS: i64 = 621_355_968_000_000_000;
/// Ticks of a .NET `DateTime` at 1601, where Windows' FILETIME starts.
const FILETIME_EPOCH_TICKS: i64 = 504_911_232_000_000_000;

/// Judgement counts of a mania play, with osu! stable's names in mania.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManiaCounts {
    /// MAX (geki).
    pub max: u32,
    pub n300: u32,
    /// 200 (katu).
    pub n200: u32,
    pub n100: u32,
    pub n50: u32,
    pub miss: u32,
}

/// A mania play of `scores.db`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalScore {
    pub beatmap_md5: String,
    pub player_name: String,
    /// osu!'s mod bits.
    pub mods: u32,
    pub score: u32,
    pub max_combo: u32,
    pub perfect: bool,
    pub counts: ManiaCounts,
    /// When it was set, in milliseconds since the Unix epoch.
    pub timestamp: i64,
    /// `0` if it was not submitted.
    pub online_id: u64,
    /// File name of its replay in `Data/r`, `None` if osu! has none (it can
    /// delete them, or the score came from another computer's `scores.db`).
    pub replay: Option<String>,
}

/// The mania plays of `<osu>/scores.db`, newest first.
pub fn read(osu: &Path) -> Result<Vec<LocalScore>> {
    let db = ScoresDB::from_file(osu.join("scores.db"))?;
    let replays = replay_files(&replays_dir(osu));

    let mut scores: Vec<LocalScore> = db
        .beatmaps
        .into_iter()
        .flat_map(|beatmap| beatmap.scores)
        .filter(|play| play.mode == Mode::Mania)
        .filter_map(|play| {
            let beatmap_md5 = play.beatmap_hash.clone()?;
            let replay = Some(replay_name(&beatmap_md5, &play.timestamp)).filter(|name| replays.contains(name));
            Some(local_score(play, beatmap_md5, replay))
        })
        .collect();

    scores.sort_by_key(|score| std::cmp::Reverse(score.timestamp));
    Ok(scores)
}

/// Where osu! keeps the replays of the local scores.
pub fn replays_dir(osu: &Path) -> PathBuf {
    osu.join("Data").join("r")
}

/// Name osu! gives the replay of a score of `beatmap_md5` set at `time`.
pub fn replay_name(beatmap_md5: &str, time: &DateTime<Utc>) -> String {
    format!("{beatmap_md5}-{}.osr", dotnet_ticks(time) - FILETIME_EPOCH_TICKS)
}

/// `time` in .NET ticks, which keep the 100 ns of `scores.db`.
fn dotnet_ticks(time: &DateTime<Utc>) -> i64 {
    UNIX_EPOCH_TICKS + time.timestamp() * 10_000_000 + i64::from(time.timestamp_subsec_nanos() / 100)
}

/// The `.osr` files of `dir`; empty if it doesn't exist.
fn replay_files(dir: &Path) -> HashSet<String> {
    let Ok(entries) = std::fs::read_dir(dir) else { return HashSet::new() };
    entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".osr"))
        .collect()
}

fn local_score(play: Replay, beatmap_md5: String, replay: Option<String>) -> LocalScore {
    LocalScore {
        beatmap_md5,
        player_name: play.player_name.unwrap_or_default(),
        mods: play.mods.bits(),
        score: play.score,
        max_combo: play.max_combo.into(),
        perfect: play.perfect_combo,
        counts: ManiaCounts {
            max: play.count_geki.into(),
            n300: play.count_300.into(),
            n200: play.count_katsu.into(),
            n100: play.count_100.into(),
            n50: play.count_50.into(),
            miss: play.count_miss.into(),
        },
        timestamp: play.timestamp.timestamp_millis(),
        online_id: play.online_score_id,
        replay,
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use osynic_osudb::entity::{osu::field::modification::ModSet, scores::scores::Scores};

    use super::*;
    use crate::test_utils::TestDir;

    /// The time of `ticks` (.NET ticks), as `scores.db` stores it.
    fn at(ticks: i64) -> DateTime<Utc> {
        let since_unix = ticks - UNIX_EPOCH_TICKS;
        Utc.timestamp_opt(since_unix.div_euclid(10_000_000), (since_unix.rem_euclid(10_000_000) * 100) as u32).unwrap()
    }

    fn play(mode: Mode, md5: &str, ticks: i64) -> Replay {
        Replay {
            mode,
            version: 20261118,
            beatmap_hash: Some(md5.into()),
            player_name: Some("Manolo_".into()),
            replay_hash: Some("r".into()),
            count_300: 10,
            count_100: 2,
            count_50: 1,
            count_geki: 30,
            count_katsu: 4,
            count_miss: 3,
            score: 900_000,
            max_combo: 40,
            perfect_combo: false,
            mods: ModSet::from_bits(64),
            life_graph: None,
            timestamp: at(ticks),
            replay_data: None,
            raw_replay_data: None,
            online_score_id: 0,
        }
    }

    /// A real score and the replay osu! wrote for it.
    #[test]
    fn names_the_replay_as_osu_does() {
        let time = at(639_174_849_939_639_372);
        assert_eq!(
            replay_name("5556dadd32fc89b9b8055a27ab1bac9f", &time),
            "5556dadd32fc89b9b8055a27ab1bac9f-134263617939639372.osr"
        );
    }

    #[test]
    fn reads_the_mania_plays_with_their_replays() {
        let dir = TestDir::new("local-scores");
        let older = play(Mode::Mania, "aaaa", 639_000_000_000_000_000);
        let newer = play(Mode::Mania, "bbbb", 639_100_000_000_000_000);
        let standard = play(Mode::Standard, "cccc", 639_200_000_000_000_000);
        let db = ScoresDB {
            version: 20261118,
            beatmaps: vec![
                Scores { hash: Some("aaaa".into()), scores: vec![older.clone()] },
                Scores { hash: Some("bbbb".into()), scores: vec![newer] },
                Scores { hash: Some("cccc".into()), scores: vec![standard] },
            ],
        };
        db.save(dir.0.join("scores.db")).unwrap();
        std::fs::create_dir_all(replays_dir(&dir.0)).unwrap();
        let replay = replay_name("aaaa", &older.timestamp);
        std::fs::write(replays_dir(&dir.0).join(&replay), b"").unwrap();

        let scores = read(&dir.0).unwrap();

        assert_eq!(scores.iter().map(|s| s.beatmap_md5.as_str()).collect::<Vec<_>>(), ["bbbb", "aaaa"]);
        assert_eq!(scores[0].replay, None);
        assert_eq!(scores[1].replay.as_deref(), Some(replay.as_str()));
        assert_eq!(scores[1].counts, ManiaCounts { max: 30, n300: 10, n200: 4, n100: 2, n50: 1, miss: 3 });
        assert_eq!(scores[1].mods, 64);
    }

    /// With osu! installed: most of its mania scores find their replay.
    #[test]
    #[ignore = "needs an osu! folder in OSU_PATH"]
    fn finds_the_replays_of_a_real_install() {
        let osu = PathBuf::from(std::env::var("OSU_PATH").unwrap());
        let scores = read(&osu).unwrap();
        let with_replay = scores.iter().filter(|s| s.replay.is_some()).count();
        assert!(with_replay * 10 >= scores.len() * 9, "{with_replay} of {}", scores.len());
    }
}
