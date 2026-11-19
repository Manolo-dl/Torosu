//! Timing statistics of a judged replay: overall and per key.
//!
//! Every time here is real time (ms): the map's divided by the rate, so plays
//! with DT or HT compare with the rest.

use serde::Serialize;

use crate::commands::replays::{
    judge::{self, Judged, Judgement, Windows},
    scores::ManiaCounts,
};

/// How a set of hits or releases was timed (ms; negative is early).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timing {
    pub count: usize,
    pub mean: f64,
    pub median: f64,
    /// Unstable rate: ten times the standard deviation.
    pub unstable_rate: f64,
    pub early: usize,
    pub late: usize,
    /// Mean of the early ones and of the rest (zero counts as late), as
    /// osu!'s results screen shows them (`Error: -10.26ms - 8.69ms avg`).
    pub early_mean: f64,
    pub late_mean: f64,
}

impl Timing {
    pub fn of(offsets: &[f64]) -> Self {
        if offsets.is_empty() {
            return Self::default();
        }
        let n = offsets.len() as f64;
        let mean = offsets.iter().sum::<f64>() / n;
        let variance = offsets.iter().map(|o| (o - mean).powi(2)).sum::<f64>() / n;
        let mut sorted = offsets.to_vec();
        sorted.sort_by(f64::total_cmp);
        let middle = sorted.len() / 2;
        let median = if sorted.len().is_multiple_of(2) { (sorted[middle - 1] + sorted[middle]) / 2.0 } else { sorted[middle] };
        let mean_of = |side: &mut dyn Iterator<Item = &f64>| {
            let (sum, n) = side.fold((0.0, 0), |(sum, n), o| (sum + o, n + 1));
            if n == 0 { 0.0 } else { sum / n as f64 }
        };
        Self {
            count: offsets.len(),
            mean,
            median,
            unstable_rate: variance.sqrt() * 10.0,
            early: offsets.iter().filter(|&&o| o < 0.0).count(),
            late: offsets.iter().filter(|&&o| o > 0.0).count(),
            early_mean: mean_of(&mut offsets.iter().filter(|&&o| o < 0.0)),
            late_mean: mean_of(&mut offsets.iter().filter(|&&o| o >= 0.0)),
        }
    }
}

/// The timing of a set of notes three ways: everything osu! counts in its
/// unstable rate, only the normal notes (rice), and only the long notes (when
/// each was pressed and when it was let go).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Views {
    /// Every press and every release of a long note, as osu!'s results screen counts them.
    pub all: Timing,
    /// The presses of the normal notes.
    pub rice: Timing,
    /// The presses and releases of the long notes.
    pub ln: Timing,
}

impl Views {
    /// Of `notes`: the presses of the notes that were hit, and the releases
    /// of those long notes within `limit` (ms) of their end.
    fn of<'a>(notes: impl Iterator<Item = &'a NoteEvent> + Clone, limit: f64) -> Self {
        let hit = |n: &NoteEvent| n.hit.filter(|_| n.judgement != Judgement::Miss);
        let release = |n: &NoteEvent| hit(n).and(n.release).filter(|r| r.abs() <= limit);
        let rice: Vec<f64> = notes.clone().filter(|n| n.end.is_none()).filter_map(hit).collect();
        let ln_presses = notes.clone().filter(|n| n.end.is_some()).filter_map(hit);
        let releases = notes.filter_map(release);
        let ln: Vec<f64> = ln_presses.chain(releases).collect();
        Self { all: Timing::of(&[rice.as_slice(), &ln].concat()), rice: Timing::of(&rice), ln: Timing::of(&ln) }
    }
}

/// How one key was played.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyStats {
    pub key: usize,
    pub notes: usize,
    pub counts: ManiaCounts,
    pub timing: Views,
    /// Long notes let go before the 50 window of their end.
    pub early_releases: usize,
    /// Long notes held past the 50 window of their end.
    pub late_releases: usize,
}

/// One note as it was played, for the charts.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteEvent {
    /// When the note comes, in the song (ms).
    pub time: f64,
    /// When a long note ends, in the song (ms).
    pub end: Option<f64>,
    pub key: usize,
    pub judgement: Judgement,
    /// With ScoreV2, of the end of a long note.
    pub tail: Option<Judgement>,
    /// ms from the note; `None` if it wasn't pressed.
    pub hit: Option<f64>,
    /// ms from the end of a long note when it was let go.
    pub release: Option<f64>,
}

/// The statistics of a replay.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub counts: ManiaCounts,
    pub timing: Views,
    pub keys: Vec<KeyStats>,
    pub notes: Vec<NoteEvent>,
}

/// The statistics of the `judged` notes of a map of `keys` keys, judged with
/// `windows` at `rate`.
pub fn stats(judged: &[Judged], keys: usize, windows: &Windows, rate: f64) -> Stats {
    let notes: Vec<NoteEvent> = judged
        .iter()
        .map(|j| NoteEvent {
            time: j.note.start / rate,
            end: j.note.end.map(|end| end / rate),
            key: j.note.key,
            judgement: j.judgement,
            tail: j.tail,
            hit: j.hit.map(|hit| hit / rate),
            release: j.release.map(|release| release / rate),
        })
        .collect();

    // The 50 window in real time: a release beyond it is not a timing.
    let limit = windows.n50 / rate;

    let keys = (0..keys)
        .map(|key| {
            let on_key = notes.iter().filter(move |n| n.key == key);
            let judged_on_key: Vec<Judged> = judged.iter().filter(|j| j.note.key == key).copied().collect();
            let released = || on_key.clone().filter(|n| n.hit.is_some()).filter_map(|n| n.release);
            KeyStats {
                key,
                notes: judged_on_key.len(),
                counts: judge::counts(&judged_on_key),
                timing: Views::of(on_key.clone(), limit),
                early_releases: released().filter(|&r| r < -limit).count(),
                late_releases: released().filter(|&r| r > limit).count(),
            }
        })
        .collect();

    Stats { counts: judge::counts(judged), timing: Views::of(notes.iter(), limit), keys, notes }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::replays::judge::Note;

    fn judged(key: usize, start: f64, judgement: Judgement, hit: Option<f64>, release: Option<f64>) -> Judged {
        let end = release.map(|_| start + 500.0);
        Judged { note: Note { key, start, end }, judgement, tail: None, hit, release }
    }

    #[test]
    fn timing_of_offsets() {
        let timing = Timing::of(&[-10.0, 0.0, 10.0, 20.0]);
        assert_eq!((timing.count, timing.mean, timing.median, timing.early, timing.late), (4, 5.0, 5.0, 1, 2));
        assert_eq!((timing.early_mean, timing.late_mean), (-10.0, 10.0));
        assert!((timing.unstable_rate - 111.803).abs() < 0.001);
        assert_eq!(Timing::of(&[]), Timing::default());
    }

    #[test]
    fn per_key_and_in_real_time() {
        let windows = Windows::stable(8.0, judge::mods::DOUBLE_TIME);
        let notes = [
            judged(0, 1500.0, Judgement::Max, Some(-15.0), None),
            judged(0, 3000.0, Judgement::N300, Some(30.0), None),
            // An early press out of the 50 window: a miss, not a timing.
            judged(1, 1500.0, Judgement::Miss, Some(-240.0), None),
            judged(1, 4500.0, Judgement::Max, Some(0.0), Some(-30.0)),
            judged(1, 6000.0, Judgement::Miss, Some(3.0), Some(-600.0)),
        ];

        let stats = stats(&notes, 2, &windows, 1.5);

        assert_eq!((stats.notes[0].time, stats.notes[0].hit), (1000.0, Some(-10.0)));
        assert_eq!(stats.notes[3].end, Some(3333.3333333333335));
        // Two normal notes; of the long notes, only the one hit: its press and release.
        let t = stats.timing;
        assert_eq!((t.rice.count, t.all.count, t.ln.count), (2, 4, 2));
        assert_eq!((stats.keys[0].timing.rice.mean, stats.keys[0].notes), (5.0, 2));
        assert_eq!(stats.keys[0].timing.ln.count, 0);
        assert_eq!((stats.keys[1].timing.ln.count, stats.keys[1].timing.ln.mean), (2, -10.0));
        assert_eq!((stats.keys[1].early_releases, stats.keys[1].late_releases), (1, 0));
        assert_eq!(stats.keys[1].counts, ManiaCounts { max: 1, miss: 2, ..Default::default() });
    }
}
