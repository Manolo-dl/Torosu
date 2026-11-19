//! The `.osg` osu! stable writes next to the replay of a local score: how the
//! score went, one record per judgement, as the game counted it.
//!
//! The format is not documented; this is what the files hold. After an `i32`
//! version (a date, `20260924`) and an `i32` record count come records of a
//! fixed size, 29 bytes, or 45 with ScoreV2, which appends 16 more. The first
//! 29 are:
//!
//! | Bytes | Value |
//! |---|---|
//! | 0..4 | `i32` time (ms, map time) |
//! | 4 | always 0 |
//! | 5..17 | `u16` 300, 100, 50, geki (MAX), katu (200), miss so far |
//! | 17..21 | `i32` score |
//! | 21..23 | `u16` highest combo |
//! | 23..25 | `u16` combo |
//! | 25 | almost always 0 |
//! | 26 | health, 0 to 200 |
//! | 27..29 | 0 (1 at 28 with ScoreV2) |

use std::path::{Path, PathBuf};

use crate::{
    commands::replays::{judge::Judgement, scores::ManiaCounts},
    error::Result,
};

const HEADER: usize = 8;
const RECORD: usize = 29;

/// The state of the score after a judgement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphPoint {
    /// ms, map time.
    pub time: i32,
    pub counts: ManiaCounts,
    pub score: i32,
    pub combo: u16,
    pub max_combo: u16,
    /// 0 to 200.
    pub health: u8,
}

/// The `.osg` of the replay at `replay`.
pub fn path_of(replay: &Path) -> PathBuf {
    replay.with_extension("osg")
}

/// Reads an `.osg`.
pub fn read(path: &Path) -> Result<Vec<GraphPoint>> {
    parse(&std::fs::read(path)?).ok_or_else(|| format!("{} is not a score graph osu! can write", path.display()).into())
}

fn parse(bytes: &[u8]) -> Option<Vec<GraphPoint>> {
    let count = usize::try_from(i32_at(bytes, 4)?).ok()?;
    let body = bytes.len().checked_sub(HEADER)?;
    if count == 0 {
        return (body == 0).then(Vec::new);
    }
    let size = body / count;
    if size < RECORD || body % count != 0 {
        return None;
    }

    bytes[HEADER..]
        .chunks_exact(size)
        .map(|record| {
            let u16_at = |at: usize| u16::from_le_bytes([record[at], record[at + 1]]);
            Some(GraphPoint {
                time: i32_at(record, 0)?,
                counts: ManiaCounts {
                    n300: u16_at(5).into(),
                    n100: u16_at(7).into(),
                    n50: u16_at(9).into(),
                    max: u16_at(11).into(),
                    n200: u16_at(13).into(),
                    miss: u16_at(15).into(),
                },
                score: i32_at(record, 17)?,
                max_combo: u16_at(21),
                combo: u16_at(23),
                health: record[26],
            })
        })
        .collect()
}

fn i32_at(bytes: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// When the game gave each judgement: what changed from one record to the
/// next. A record can hold more than one.
pub fn judgements(graph: &[GraphPoint]) -> Vec<(i32, Judgement)> {
    let mut out = Vec::new();
    let mut previous = ManiaCounts::default();
    for point in graph {
        let now = point.counts;
        for (judgement, before, after) in [
            (Judgement::Max, previous.max, now.max),
            (Judgement::N300, previous.n300, now.n300),
            (Judgement::N200, previous.n200, now.n200),
            (Judgement::N100, previous.n100, now.n100),
            (Judgement::N50, previous.n50, now.n50),
            (Judgement::Miss, previous.miss, now.miss),
        ] {
            for _ in before..after {
                out.push((point.time, judgement));
            }
        }
        previous = now;
    }
    out
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use osynic_osudb::entity::osu::osudb::OsuDB;

    use super::*;
    use crate::commands::replays::{frames, judge, scores};

    /// A record at `time` with `counts` (300, 100, 50, MAX, 200, miss), padded
    /// to `size` bytes.
    fn record(time: i32, counts: [u16; 6], score: i32, combo: u16, size: usize) -> Vec<u8> {
        let mut out = time.to_le_bytes().to_vec();
        out.push(0);
        for count in counts {
            out.extend(count.to_le_bytes());
        }
        out.extend(score.to_le_bytes());
        out.extend(combo.to_le_bytes());
        out.extend(combo.to_le_bytes());
        out.extend([0, 200, 0, 0]);
        out.resize(size, 0);
        out
    }

    fn file(records: &[Vec<u8>]) -> Vec<u8> {
        let mut out = 20260924i32.to_le_bytes().to_vec();
        out.extend((records.len() as i32).to_le_bytes());
        records.iter().for_each(|r| out.extend(r));
        out
    }

    #[test]
    fn reads_the_records_of_both_sizes() {
        for size in [29, 45] {
            let bytes = file(&[record(1000, [0, 0, 0, 1, 0, 0], 300, 1, size), record(1100, [1, 0, 0, 1, 0, 1], 500, 0, size)]);

            let graph = parse(&bytes).unwrap();

            assert_eq!(graph.len(), 2);
            assert_eq!(graph[1].time, 1100);
            assert_eq!(graph[1].counts, ManiaCounts { max: 1, n300: 1, miss: 1, ..Default::default() });
            assert_eq!((graph[1].score, graph[1].health), (500, 200));
        }
    }

    #[test]
    fn rejects_what_is_not_a_graph() {
        assert!(parse(&[1, 2, 3]).is_none());
        let mut bytes = file(&[record(1000, [0; 6], 0, 0, 29)]);
        bytes.pop();
        assert!(parse(&bytes).is_none());
    }

    #[test]
    fn tells_each_judgement_and_when() {
        let graph = parse(&file(&[
            record(1000, [0, 0, 0, 1, 0, 0], 0, 1, 29),
            record(1200, [1, 0, 0, 1, 0, 2], 0, 0, 29),
        ]))
        .unwrap();

        assert_eq!(
            judgements(&graph),
            [(1000, Judgement::Max), (1200, Judgement::N300), (1200, Judgement::Miss), (1200, Judgement::Miss)]
        );
    }

    /// With osu! installed: the judge against what the game counted, over the
    /// latest local mania replays. Prints how far each one is.
    #[test]
    #[ignore = "needs an osu! folder in OSU_PATH"]
    fn the_judge_agrees_with_the_game() {
        let osu = PathBuf::from(std::env::var("OSU_PATH").unwrap());
        let db = OsuDB::from_file(osu.join("osu!.db")).unwrap();
        let files: HashMap<String, PathBuf> = db
            .beatmaps
            .into_iter()
            .filter_map(|b| Some((b.hash?, osu.join("Songs").join(b.folder_name?).join(b.file_name?))))
            .collect();

        let (mut plays, mut notes, mut off) = (0, 0, 0);
        for score in scores::read(&osu).unwrap().iter().filter(|s| s.replay.is_some()).take(60) {
            let replay_path = scores::replays_dir(&osu).join(score.replay.as_ref().unwrap());
            let Some(map) = files.get(&score.beatmap_md5).and_then(|p| rosu_map::from_path(p).ok()) else { continue };
            let Ok(chart) = judge::Chart::new(&map, score.mods) else { continue };
            let replay = frames::read(&replay_path).unwrap();
            let judged = judge::judge(
                &chart,
                &frames::presses(&replay.frames, chart.keys),
                &judge::Rules::stable(chart.od, score.mods),
            );
            let ours = judge::counts(&judged);
            let game = read(&path_of(&replay_path)).ok().and_then(|g| g.last().map(|p| p.counts)).unwrap_or(score.counts);

            let diff = [
                ours.max.abs_diff(game.max),
                ours.n300.abs_diff(game.n300),
                ours.n200.abs_diff(game.n200),
                ours.n100.abs_diff(game.n100),
                ours.n50.abs_diff(game.n50),
                ours.miss.abs_diff(game.miss),
            ];
            let total: u32 = diff.iter().sum();
            println!("{:5} of {:5} off {:?} {}", total, judged.len(), diff, score.beatmap_md5);
            plays += 1;
            notes += judged.len();
            off += total as usize;
        }
        println!("{plays} plays: {off} judgements off of {notes} ({:.2}%)", off as f64 * 100.0 / notes as f64);
        // Each wrong judgement counts twice: where it is and where it should be.
        assert!((off as f64) < notes as f64 * 0.1, "{off} of {notes}");
    }
}
