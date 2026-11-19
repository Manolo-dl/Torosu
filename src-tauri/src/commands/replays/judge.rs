//! Judges a mania replay against its map, close to how osu! stable does
//! (ScoreV1), to know when each note was hit and released.
//!
//! It is simpler than stable: it doesn't model body breaks of long notes or
//! the quirks of how stable plays back frames, so a few judgements can differ
//! from the game's. The timing of each hit, which is what the analysis needs,
//! doesn't depend on them.

use rosu_map::{Beatmap, section::{general::GameMode, hit_objects::HitObjectKind}};
use serde::Serialize;

use crate::{
    commands::{
        replays::{frames::Press, scores::ManiaCounts},
        trainer::mania,
    },
    error::Result,
};

pub mod mods {
    pub const EASY: u32 = 1 << 1;
    pub const HARD_ROCK: u32 = 1 << 4;
    pub const DOUBLE_TIME: u32 = 1 << 6;
    pub const HALF_TIME: u32 = 1 << 8;
    pub const NIGHTCORE: u32 = 1 << 9;
    pub const RANDOM: u32 = 1 << 21;
    pub const SCORE_V2: u32 = 1 << 29;
    pub const MIRROR: u32 = 1 << 30;
}

/// Speed of the song with `mods`.
pub fn rate(mods: u32) -> f64 {
    if mods & (mods::DOUBLE_TIME | mods::NIGHTCORE) != 0 {
        1.5
    } else if mods & mods::HALF_TIME != 0 {
        0.75
    } else {
        1.0
    }
}

/// A note of the map, on the key that plays it (Mirror swaps them).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Note {
    pub key: usize,
    pub start: f64,
    /// When a long note ends; `None` for a normal note.
    pub end: Option<f64>,
}

/// What the replay is judged against.
#[derive(Debug, Clone)]
pub struct Chart {
    pub keys: usize,
    pub od: f32,
    /// Sorted by `start`.
    pub notes: Vec<Note>,
}

impl Chart {
    /// The notes of a mania map as played with `mods`. Converts and Random
    /// are not supported: their notes are not the ones in the file.
    pub fn new(beatmap: &Beatmap, mods: u32) -> Result<Self> {
        if beatmap.mode != GameMode::Mania {
            return Err("only osu!mania maps can be analysed, not converts".into());
        }
        if mods & mods::RANDOM != 0 {
            return Err("replays with Random can't be analysed: their notes are shuffled".into());
        }

        let keys = mania::key_count(beatmap);
        let mirror = mods & mods::MIRROR != 0;
        let mut notes: Vec<Note> = beatmap
            .hit_objects
            .iter()
            .filter_map(|obj| {
                let (x, end) = mania::note(obj)?;
                let column = mania::column(x, keys);
                let key = if mirror { keys - 1 - column } else { column };
                let end = matches!(obj.kind, HitObjectKind::Hold(_)).then_some(end);
                Some(Note { key, start: obj.start_time, end })
            })
            .collect();
        notes.sort_by(|a, b| a.start.total_cmp(&b.start));

        Ok(Self { keys, od: beatmap.overall_difficulty, notes })
    }
}

/// How far from a note (ms, map time) each judgement reaches.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Windows {
    pub max: f64,
    pub n300: f64,
    pub n200: f64,
    pub n100: f64,
    pub n50: f64,
    pub miss: f64,
}

impl Windows {
    /// Stable's windows for `od` with `mods`: the classic ones, or with
    /// ScoreV2 the ones lazer has. They are fixed in real time, so in map
    /// time they grow with DT and shrink with HT; HR narrows them and EZ
    /// widens them by 1.4.
    pub fn stable(od: f32, mods: u32) -> Self {
        let od = f64::from(od.clamp(0.0, 10.0));
        let difficulty = if mods & mods::HARD_ROCK != 0 {
            1.4
        } else if mods & mods::EASY != 0 {
            1.0 / 1.4
        } else {
            1.0
        };
        let scale = rate(mods) / difficulty;
        let window = |base: f64| (base * scale).floor() + 0.5;

        if mods & mods::SCORE_V2 != 0 {
            // From OD 0 to 5 to 10.
            let range = |at0: f64, at5: f64, at10: f64| {
                let ms = if od > 5.0 { at5 + (at10 - at5) * (od - 5.0) / 5.0 } else { at5 + (at5 - at0) * (od - 5.0) / 5.0 };
                window(ms)
            };
            return Self {
                max: range(22.4, 19.4, 13.9),
                n300: range(64.0, 49.0, 34.0),
                n200: range(97.0, 82.0, 67.0),
                n100: range(127.0, 112.0, 97.0),
                n50: range(151.0, 136.0, 121.0),
                miss: range(188.0, 173.0, 158.0),
            };
        }

        Self {
            max: window(16.0),
            n300: window(64.0 - 3.0 * od),
            n200: window(97.0 - 3.0 * od),
            n100: window(127.0 - 3.0 * od),
            n50: window(151.0 - 3.0 * od),
            miss: window(188.0 - 3.0 * od),
        }
    }

    /// Judgement of a normal note hit `offset` ms away.
    fn of_hit(&self, offset: f64) -> Judgement {
        let offset = offset.abs();
        if offset <= self.max {
            Judgement::Max
        } else if offset <= self.n300 {
            Judgement::N300
        } else if offset <= self.n200 {
            Judgement::N200
        } else if offset <= self.n100 {
            Judgement::N100
        } else if offset <= self.n50 {
            Judgement::N50
        } else {
            Judgement::Miss
        }
    }

    /// Judgement of a long note: stable judges it once, from how far both
    /// ends were (`head` and `tail`, ms).
    fn of_long_note(&self, head: f64, tail: f64) -> Judgement {
        let head = head.round().abs();
        let both = head + tail.round().abs();
        let (max, n300, n200, n100) = (self.max.floor(), self.n300.floor(), self.n200.floor(), self.n100.floor());
        if head <= max * 1.2 && both <= max * 2.4 {
            Judgement::Max
        } else if head <= n300 * 1.1 && both <= n300 * 2.2 {
            Judgement::N300
        } else if head <= n200 && both <= n200 * 2.0 {
            Judgement::N200
        } else if head <= n100 && both <= n100 * 2.0 {
            Judgement::N100
        } else {
            Judgement::N50
        }
    }

    /// With ScoreV2, the judgement of the end of a long note released
    /// `release` ms from it: windows 1.5 times as wide, and held past the
    /// 100 one is a miss.
    fn of_release(&self, release: f64) -> Judgement {
        let lenient = |window: f64| window.floor() * 1.5;
        let off = release.round().abs();
        if off <= lenient(self.max) {
            Judgement::Max
        } else if off <= lenient(self.n300) {
            Judgement::N300
        } else if off <= lenient(self.n200) {
            Judgement::N200
        } else if release >= lenient(self.n100) {
            Judgement::Miss
        } else if off <= lenient(self.n100) {
            Judgement::N100
        } else if off <= lenient(self.n50) {
            Judgement::N50
        } else {
            Judgement::Miss
        }
    }
}

/// From best to worst.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Judgement {
    Max,
    N300,
    N200,
    N100,
    N50,
    Miss,
}

/// How a note was played.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Judged {
    pub note: Note,
    /// Of the whole note; with ScoreV2, of the start of a long note.
    pub judgement: Judgement,
    /// With ScoreV2, the end of a long note is judged on its own.
    pub tail: Option<Judgement>,
    /// When it was pressed (ms, map time; negative is early); `None` if it
    /// wasn't.
    pub hit: Option<f64>,
    /// When a long note was released, against its end (ms, map time);
    /// `None` for normal notes and long notes not pressed.
    pub release: Option<f64>,
}

/// How the notes are judged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rules {
    pub windows: Windows,
    /// Long notes judged at both ends, as stable does with ScoreV2.
    pub score_v2: bool,
}

impl Rules {
    /// Stable's, for a map of `od` played with `mods`.
    pub fn stable(od: f32, mods: u32) -> Self {
        Self { windows: Windows::stable(od, mods), score_v2: mods & mods::SCORE_V2 != 0 }
    }
}

/// Judges every note of `chart` with the `presses` of each key.
pub fn judge(chart: &Chart, presses: &[Vec<Press>], rules: &Rules) -> Vec<Judged> {
    let mut judged = Vec::with_capacity(chart.notes.len());
    for key in 0..chart.keys {
        let notes = chart.notes.iter().filter(|note| note.key == key);
        judge_key(notes, presses.get(key).map_or(&[], Vec::as_slice), rules, &mut judged);
    }
    judged.sort_by(|a, b| a.note.start.total_cmp(&b.note.start).then(a.note.key.cmp(&b.note.key)));
    judged
}

/// A press judges the first note of its key that is still waiting, if it is
/// within the miss window; a note no press reached in time is a miss.
fn judge_key<'a>(notes: impl Iterator<Item = &'a Note>, presses: &[Press], rules: &Rules, out: &mut Vec<Judged>) {
    let windows = &rules.windows;
    let mut presses = presses.iter().peekable();
    for &note in notes {
        // Presses too early for this note hit nothing.
        while presses.next_if(|press| press.start < note.start - windows.miss).is_some() {}

        let Some(press) = presses.next_if(|press| press.start <= note.start + windows.miss) else {
            let tail = (rules.score_v2 && note.end.is_some()).then_some(Judgement::Miss);
            out.push(Judged { note, judgement: Judgement::Miss, tail, hit: None, release: None });
            continue;
        };

        let hit = press.start - note.start;
        let Some(end) = note.end else {
            out.push(Judged { note, judgement: windows.of_hit(hit), tail: None, hit: Some(hit), release: None });
            continue;
        };

        // Pressed again before its end, the key grabs the same long note.
        let mut last = *press;
        let mut regrabbed = false;
        while let Some(again) = presses.next_if(|again| again.start < end) {
            last = *again;
            regrabbed = true;
        }
        let release = last.end - end;
        let (judgement, tail) = if rules.score_v2 {
            (windows.of_hit(hit), Some(windows.of_release(release)))
        } else {
            let judgement = long_note(hit, release, windows);
            // Stable gives a long note let go and grabbed again 200 at best.
            (if regrabbed { judgement.max(Judgement::N200) } else { judgement }, None)
        };
        out.push(Judged { note, judgement, tail, hit: Some(hit), release: Some(release) });
    }
}

/// Stable's judgement of a long note pressed `hit` ms from its start and
/// released `release` ms from its end.
fn long_note(hit: f64, release: f64, windows: &Windows) -> Judgement {
    let n50 = windows.n50.floor();
    if release < -n50 {
        return Judgement::Miss;
    }
    // Held past the window, stable judges it when the window closes.
    let tail = release.min(n50);
    // Pressed before the 50 window, its head counts as hit at the end.
    let head = if hit < -n50 { f64::INFINITY } else { hit };
    windows.of_long_note(head, tail)
}

/// How many of each judgement.
pub fn counts(judged: &[Judged]) -> ManiaCounts {
    let mut counts = ManiaCounts::default();
    for judgement in judged.iter().flat_map(|note| std::iter::once(note.judgement).chain(note.tail)) {
        match judgement {
            Judgement::Max => counts.max += 1,
            Judgement::N300 => counts.n300 += 1,
            Judgement::N200 => counts.n200 += 1,
            Judgement::N100 => counts.n100 += 1,
            Judgement::N50 => counts.n50 += 1,
            Judgement::Miss => counts.miss += 1,
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 4K map at OD 8 with `objects` (`x,y,time,type,...` lines).
    fn beatmap(objects: &str) -> Beatmap {
        let osu = format!(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:4\nOverallDifficulty:8\n\n\
             [TimingPoints]\n0,500,4,2,0,100,1,0\n\n[HitObjects]\n{objects}"
        );
        rosu_map::from_str(&osu).unwrap()
    }

    fn press(start: f64, end: f64) -> Press {
        Press { start, end }
    }

    #[test]
    fn windows_follow_od_and_mods() {
        let od8 = Windows::stable(8.0, 0);
        assert_eq!((od8.max, od8.n300, od8.n50, od8.miss), (16.5, 40.5, 127.5, 164.5));

        let dt = Windows::stable(8.0, mods::DOUBLE_TIME);
        assert_eq!((dt.max, dt.n300), (24.5, 60.5));

        let hr = Windows::stable(8.0, mods::HARD_ROCK);
        assert_eq!((hr.max, hr.n300), (11.5, 28.5));

        let v2 = Windows::stable(8.0, mods::SCORE_V2);
        assert_eq!((v2.max, v2.n300, v2.miss), (16.5, 40.5, 164.5));
        assert_eq!(Windows::stable(5.0, mods::SCORE_V2).max, 19.5);
    }

    #[test]
    fn mirror_plays_the_notes_on_the_other_side() {
        let map = beatmap("64,192,1000,1,0,0:0:0:0:\n448,192,2000,1,0,0:0:0:0:\n");

        let keys: Vec<usize> = Chart::new(&map, mods::MIRROR).unwrap().notes.iter().map(|n| n.key).collect();

        assert_eq!(keys, [3, 0]);
    }

    #[test]
    fn random_is_not_supported() {
        assert!(Chart::new(&beatmap("64,192,1000,1,0,0:0:0:0:\n"), mods::RANDOM).is_err());
    }

    #[test]
    fn judges_normal_notes_by_their_offset() {
        let map = beatmap(
            "64,192,1000,1,0,0:0:0:0:\n64,192,2000,1,0,0:0:0:0:\n64,192,3000,1,0,0:0:0:0:\n64,192,4000,1,0,0:0:0:0:\n",
        );
        let chart = Chart::new(&map, 0).unwrap();
        // On time, 30 ms late, an early press out of the 50 window, and none.
        let presses = vec![vec![press(1000.0, 1050.0), press(2030.0, 2080.0), press(2850.0, 2900.0)]];

        let judged = judge(&chart, &presses, &Rules::stable(8.0, 0));

        let got: Vec<_> = judged.iter().map(|j| (j.judgement, j.hit)).collect();
        assert_eq!(
            got,
            [
                (Judgement::Max, Some(0.0)),
                (Judgement::N300, Some(30.0)),
                (Judgement::Miss, Some(-150.0)),
                (Judgement::Miss, None),
            ]
        );
    }

    #[test]
    fn a_press_too_early_hits_nothing() {
        let chart = Chart::new(&beatmap("64,192,1000,1,0,0:0:0:0:\n"), 0).unwrap();
        let presses = vec![vec![press(500.0, 550.0), press(1005.0, 1050.0)]];

        let judged = judge(&chart, &presses, &Rules::stable(8.0, 0));

        assert_eq!((judged[0].judgement, judged[0].hit), (Judgement::Max, Some(5.0)));
    }

    #[test]
    fn judges_long_notes_by_both_ends() {
        let map = beatmap(
            "64,192,1000,128,0,1500:0:0:0:0:\n64,192,2000,128,0,2500:0:0:0:0:\n64,192,3000,128,0,3500:0:0:0:0:\n",
        );
        let chart = Chart::new(&map, 0).unwrap();
        // Clean, released 200 ms early, and held long past the end.
        let presses = vec![vec![press(1005.0, 1495.0), press(2000.0, 2300.0), press(3000.0, 4000.0)]];

        let judged = judge(&chart, &presses, &Rules::stable(8.0, 0));

        let got: Vec<_> = judged.iter().map(|j| (j.judgement, j.release)).collect();
        assert_eq!(
            got,
            [(Judgement::Max, Some(-5.0)), (Judgement::Miss, Some(-200.0)), (Judgement::N200, Some(500.0))]
        );
        assert_eq!(counts(&judged), ManiaCounts { max: 1, n200: 1, miss: 1, ..Default::default() });
    }

    #[test]
    fn a_long_note_grabbed_again_gets_200_at_best() {
        let chart = Chart::new(&beatmap("64,192,1000,128,0,2000:0:0:0:0:\n64,192,3000,1,0,0:0:0:0:\n"), 0).unwrap();
        // Let go at 1500, grabbed again until the end; then the next note.
        let presses = vec![vec![press(1000.0, 1500.0), press(1520.0, 2000.0), press(3000.0, 3050.0)]];

        let judged = judge(&chart, &presses, &Rules::stable(8.0, 0));

        let got: Vec<_> = judged.iter().map(|j| (j.judgement, j.release)).collect();
        assert_eq!(got, [(Judgement::N200, Some(0.0)), (Judgement::Max, None)]);
    }

    #[test]
    fn score_v2_judges_both_ends_of_a_long_note() {
        let chart = Chart::new(&beatmap("64,192,1000,128,0,1500:0:0:0:0:\n64,192,2000,128,0,2500:0:0:0:0:\n"), 0).unwrap();
        // The second one held long past its end.
        let presses = vec![vec![press(1005.0, 1550.0), press(2000.0, 3000.0)]];

        let judged = judge(&chart, &presses, &Rules::stable(8.0, mods::SCORE_V2));

        let got: Vec<_> = judged.iter().map(|j| (j.judgement, j.tail)).collect();
        assert_eq!(got, [(Judgement::Max, Some(Judgement::N300)), (Judgement::Max, Some(Judgement::Miss))]);
        assert_eq!(counts(&judged), ManiaCounts { max: 2, n300: 1, miss: 1, ..Default::default() });
    }
}
