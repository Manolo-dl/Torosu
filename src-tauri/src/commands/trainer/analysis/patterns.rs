//! Finds the patterns of a map, row by row (the notes that start at the same
//! time), for the whole map and for each group of keys of a playstyle.
//!
//! Each row gets the pattern of the rows around it (a window inside its
//! phrase: the rows close enough to be played as one pattern), plus modifiers
//! that describe the row itself (a minijack, an anchor, a grace note...).
//! The same classifier runs on the rows of each hand, keeping only that
//! hand's keys: a 7K handstream is a jumptrill on one hand and a roll on the
//! other, and that is what tells it apart from a 4K one.
//!
//! Time thresholds are in beats, clamped in ms as krrcream's converter does:
//! what makes a jack is how fast the finger has to move, not the grid.

use std::collections::BTreeMap;

use rosu_map::{Beatmap, section::hit_objects::HitObjectKind};
use serde::Serialize;

use super::{
    layout::{Digit, Group, Hand, Layout},
    zones::{self, Zone, ZoneSection},
};
use crate::commands::trainer::naming;

/// osu! plays up to 18K (9K+9K).
pub const MAX_KEYS: usize = 18;

/// Notes closer than this (in ms) are the same row.
const SAME_ROW_MS: f64 = 1.0;
/// Rows closer than this (in ms) are a grace note: a chord played as a flam.
const GRACE_MS: f64 = 25.0;
/// Rows up to this far apart (in beats) are the same pattern, a 1/2 with some
/// leniency, but never further than `LINK_MAX_MS` nor closer than
/// `LINK_MIN_MS` at gimmick BPMs.
const LINK_BEATS: f64 = 0.55;
const LINK_MIN_MS: f64 = 60.0;
const LINK_MAX_MS: f64 = 300.0;
/// A note starting this soon (in beats, at most `INVERSE_MAX_MS`) after a long
/// note of its key ended: inverse LNs.
const INVERSE_BEATS: f64 = 0.3;
const INVERSE_MAX_MS: f64 = 150.0;
/// The window of a row: this many rows before and after it.
const WINDOW_BEFORE: usize = 3;
const WINDOW_AFTER: usize = 4;
/// Sections shorter than this many rows are not listed.
const SECTION_MIN_ROWS: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
pub enum Pattern {
    /// Single notes too far apart to make a pattern.
    Sparse,
    /// Chords too far apart, or alone.
    Chords,
    /// Single notes, without jacks.
    Stream,
    /// Single notes going over next keys: stairs and rolls.
    Roll,
    /// Two single notes alternating.
    Trill,
    /// A stream with jumps (2-note chords).
    Jumpstream,
    /// A stream with hands (3-note chords).
    Handstream,
    /// A stream with quads or bigger chords.
    Chordstream,
    /// Chords alternating between the hands: the same two rows, or on 7K
    /// and up, chords that change while each hand keeps its turn.
    Jumptrill,
    /// Two rows alternating that interleave (`O.O` / `.O.`).
    Bracket,
    /// The same keys again and again.
    Jack,
    /// Chords sharing keys with the next ones.
    Chordjack,
    /// Chordjacks of hands, quads or more.
    DenseChordjack,
    /// Mostly long notes, single.
    LnStream,
    /// Mostly long notes, in chords.
    LnChords,
    /// Long notes with tiny gaps between them, in the same key.
    InverseLn,
    /// Taps while other keys are held.
    HoldTap,
}

impl Pattern {
    pub const ALL: [Pattern; 17] = [
        Pattern::Sparse,
        Pattern::Chords,
        Pattern::Stream,
        Pattern::Roll,
        Pattern::Trill,
        Pattern::Jumpstream,
        Pattern::Handstream,
        Pattern::Chordstream,
        Pattern::Jumptrill,
        Pattern::Bracket,
        Pattern::Jack,
        Pattern::Chordjack,
        Pattern::DenseChordjack,
        Pattern::LnStream,
        Pattern::LnChords,
        Pattern::InverseLn,
        Pattern::HoldTap,
    ];

    fn jacky(self) -> bool {
        matches!(self, Pattern::Jack | Pattern::Chordjack | Pattern::DenseChordjack)
    }
}

/// What a row adds to its pattern, as bit flags.
pub mod modifier {
    /// The row repeats a key of the row next to it, inside a non-jack pattern.
    pub const MINIJACK: u16 = 1 << 0;
    /// A key of the row comes back in at least half the rows around it.
    pub const ANCHOR: u16 = 1 << 1;
    /// A row less than 25 ms from the next or previous one.
    pub const GRACE: u16 = 1 << 2;
    /// The row uses more than one hand.
    pub const SPLIT: u16 = 1 << 3;
    /// Every row around it is on the same hand.
    pub const ONE_HAND: u16 = 1 << 4;
    /// A chord that looks the same mirrored.
    pub const SYMMETRIC: u16 = 1 << 5;
    pub const THUMB: u16 = 1 << 6;
    pub const SCRATCH: u16 = 1 << 7;
    /// A phrase of 3 to 8 rows.
    pub const BURST: u16 = 1 << 8;
    /// Another key is held by a long note.
    pub const HELD: u16 = 1 << 9;

    pub const NAMES: [(u16, &str); 10] = [
        (MINIJACK, "minijack"),
        (ANCHOR, "anchor"),
        (GRACE, "grace"),
        (SPLIT, "split"),
        (ONE_HAND, "one hand"),
        (SYMMETRIC, "symmetric"),
        (THUMB, "thumb"),
        (SCRATCH, "scratch"),
        (BURST, "burst"),
        (HELD, "held"),
    ];
}

#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub time: f64,
    pub beat_len: f64,
    pub mask: u32,
    /// The notes of the row that are long notes.
    pub ln: u32,
    /// Keys held by an earlier long note.
    pub held: u32,
    /// Notes starting right after a long note of their key.
    pub inverse: u32,
    /// Where the row falls in the beat: 0 on it, 1 on a 1/2, 2 on a 1/4,
    /// 3 anywhere else.
    pub beat: u8,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Label {
    pub pattern: Pattern,
    pub modifiers: u16,
}

pub fn key_count(beatmap: &Beatmap) -> usize {
    beatmap.circle_size.round().clamp(1.0, MAX_KEYS as f32) as usize
}

/// Rows at most this far apart (in ms) are the same pattern.
pub fn link_ms(beat_len: f64) -> f64 {
    (beat_len * LINK_BEATS).clamp(LINK_MIN_MS, LINK_MAX_MS)
}

/// The rows of a mania map. Notes stacked on a note or inside a long note of
/// their key are skipped.
pub fn rows(beatmap: &Beatmap) -> Vec<Row> {
    let keys = key_count(beatmap);

    // The beat grid, without the fake BPMs of gimmick maps.
    let main_beat_len = naming::main_bpm(beatmap).map_or(500.0, |bpm| 60_000.0 / bpm);
    let mut grid: Vec<(f64, f64)> = beatmap
        .control_points
        .timing_points
        .iter()
        .filter(|tp| tp.beat_len.is_finite() && (main_beat_len / 10.0..=main_beat_len * 10.0).contains(&tp.beat_len))
        .map(|tp| (tp.time, tp.beat_len))
        .collect();
    if grid.is_empty() {
        grid.push((0.0, main_beat_len));
    }

    let mut notes: Vec<(f64, f64, usize)> = beatmap
        .hit_objects
        .iter()
        .filter_map(|obj| {
            let (x, end) = match &obj.kind {
                HitObjectKind::Circle(circle) => (circle.pos.x, obj.start_time),
                HitObjectKind::Hold(hold) => (hold.pos_x, obj.start_time + hold.duration),
                _ => return None,
            };
            let col = ((x * keys as f32 / 512.0).floor().max(0.0) as usize).min(keys - 1);
            Some((obj.start_time, end, col))
        })
        .collect();
    notes.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut release = [f64::NEG_INFINITY; MAX_KEYS];
    let mut was_ln = [false; MAX_KEYS];
    let mut rows: Vec<Row> = Vec::new();

    for (start, end, col) in notes {
        if rows.last().is_none_or(|row| start - row.time >= SAME_ROW_MS) {
            let tp = grid.partition_point(|&(t, _)| t <= start + 2.0).saturating_sub(1);
            let held = (0..keys).filter(|&k| release[k] > start).fold(0, |m, k| m | 1 << k);
            let (tp_time, beat_len) = grid[tp];
            rows.push(Row { time: start, beat_len, mask: 0, ln: 0, held, inverse: 0, beat: beat_position(start, tp_time, beat_len) });
        }
        let row = rows.last_mut().expect("a row was just pushed");
        let bit = 1u32 << col;
        if row.mask & bit != 0 || release[col] > start {
            continue;
        }
        row.mask |= bit;
        row.held &= !bit;
        let inverse_gap = (row.beat_len * INVERSE_BEATS).min(INVERSE_MAX_MS);
        if was_ln[col] && start - release[col] <= inverse_gap {
            row.inverse |= bit;
        }
        if end > start {
            row.ln |= bit;
        }
        release[col] = end;
        was_ln[col] = end > start;
    }

    rows.retain(|row| row.mask != 0);
    rows
}

/// Where `time` falls in the beat (see [`Row::beat`]), a few ms off the grid
/// included.
pub fn beat_position(time: f64, tp_time: f64, beat_len: f64) -> u8 {
    let frac = ((time - tp_time) / beat_len).rem_euclid(1.0);
    let tolerance = 3.0 / beat_len;
    let near = |x: f64| (frac - x).abs() < tolerance || (frac - x + 1.0).abs() < tolerance;
    if near(0.0) {
        0
    } else if near(0.5) {
        1
    } else if near(0.25) || near(0.75) {
        2
    } else {
        3
    }
}

/// What the classifier knows about the keys it looks at.
pub struct Scope<'a> {
    /// The hands, to tell split rows from one-hand ones. A single group when
    /// classifying one hand.
    pub groups: &'a [u32],
    pub thumb: u32,
    /// Keys either hand plays (a middle key).
    pub either: u32,
    pub scratch: u32,
    /// The key count, to spot symmetric chords; `None` when the layout is not
    /// symmetric.
    pub mirror: Option<usize>,
}

fn mirror(mask: u32, keys: usize) -> u32 {
    (0..keys).filter(|&k| mask & 1 << k != 0).fold(0, |m, k| m | 1 << (keys - 1 - k))
}

fn lowest(mask: u32) -> i32 {
    mask.trailing_zeros() as i32
}

/// The span of a mask, lowest to highest key.
fn span(mask: u32) -> (u32, u32) {
    (mask.trailing_zeros(), 31 - mask.leading_zeros())
}

/// The hand a row is on, the middle key aside; `None` when on both or none.
fn hand_of(mask: u32, scope: &Scope) -> Option<usize> {
    let mut hands = scope.groups.iter().enumerate().filter(|&(_, &g)| mask & g & !scope.either != 0);
    match (hands.next(), hands.next()) {
        (Some((i, _)), None) => Some(i),
        _ => None,
    }
}

/// The pattern of a window of rows of the same phrase.
fn pattern_of(w: &[Row], scope: &Scope) -> Pattern {
    let n = w.len() as f64;
    let notes: u32 = w.iter().map(|r| r.mask.count_ones()).sum();
    let mean = notes as f64 / n;
    let ln = w.iter().map(|r| r.ln.count_ones()).sum::<u32>() as f64 / notes as f64;
    let inverse = w.iter().map(|r| r.inverse.count_ones()).sum::<u32>() as f64 / notes as f64;
    let held = w.iter().filter(|r| r.held != 0).count() as f64 / n;

    if ln >= 0.5 {
        return if inverse >= 0.4 {
            Pattern::InverseLn
        } else if mean >= 1.75 {
            Pattern::LnChords
        } else {
            Pattern::LnStream
        };
    }
    if held >= 0.5 && w.len() > 1 {
        return Pattern::HoldTap;
    }
    if w.len() == 1 {
        return if mean >= 2.0 { Pattern::Chords } else { Pattern::Sparse };
    }

    let pairs = n - 1.0;
    let jack = w.windows(2).filter(|p| p[0].mask & p[1].mask != 0).count() as f64 / pairs;
    let repeat = w.windows(2).filter(|p| p[0].mask == p[1].mask).count() as f64 / pairs;

    if w.len() >= 4 {
        let alternating = w.windows(3).filter(|t| t[0].mask == t[2].mask && t[0].mask & t[1].mask == 0).count();
        if alternating as f64 >= 0.6 * (n - 2.0) {
            if mean < 1.5 {
                return Pattern::Trill;
            }
            let mid = w.len() / 2;
            let ((a_lo, a_hi), (b_lo, b_hi)) = (span(w[mid - 1].mask), span(w[mid].mask));
            return if a_lo < b_hi && b_lo < a_hi { Pattern::Bracket } else { Pattern::Jumptrill };
        }
    }

    // Chords taking turns between the hands, whatever the chords.
    if scope.groups.len() > 1 && mean >= 1.75 {
        let turns = w
            .windows(2)
            .filter(|p| p[0].mask & p[1].mask & scope.either == 0)
            .filter(|p| matches!((hand_of(p[0].mask, scope), hand_of(p[1].mask, scope)), (Some(a), Some(b)) if a != b))
            .count();
        if turns as f64 >= 0.75 * pairs {
            return Pattern::Jumptrill;
        }
    }

    if repeat >= 0.5 || jack >= 0.6 {
        return if mean >= 2.5 {
            Pattern::DenseChordjack
        } else if mean >= 1.6 {
            Pattern::Chordjack
        } else {
            Pattern::Jack
        };
    }

    let size = |pred: fn(u32) -> bool| w.iter().filter(|r| pred(r.mask.count_ones())).count() as f64 / n;
    if size(|s| s == 1) >= 0.75 {
        let adjacent = w
            .windows(2)
            .filter(|p| p[0].mask.count_ones() == 1 && p[1].mask.count_ones() == 1)
            .filter(|p| (lowest(p[1].mask) - lowest(p[0].mask)).abs() == 1)
            .count();
        return if adjacent as f64 >= 0.6 * pairs { Pattern::Roll } else { Pattern::Stream };
    }

    let big = size(|s| s >= 4);
    if big >= 0.25 {
        Pattern::Chordstream
    } else if big + size(|s| s == 3) >= 0.25 {
        Pattern::Handstream
    } else {
        Pattern::Jumpstream
    }
}

/// The pattern and modifiers of every row.
pub fn classify(rows: &[Row], scope: &Scope) -> Vec<Label> {
    use modifier::*;

    let n = rows.len();
    // The first and last row of each row's phrase.
    let mut first = vec![0usize; n];
    for i in 1..n {
        let linked = rows[i].time - rows[i - 1].time <= link_ms(rows[i].beat_len);
        first[i] = if linked { first[i - 1] } else { i };
    }
    let mut last = vec![0usize; n];
    for i in (0..n).rev() {
        last[i] = if i + 1 < n && first[i + 1] == first[i] { last[i + 1] } else { i };
    }

    (0..n)
        .map(|i| {
            let lo = first[i].max(i.saturating_sub(WINDOW_BEFORE));
            let hi = last[i].min(i + WINDOW_AFTER);
            let window = &rows[lo..=hi];
            let pattern = pattern_of(window, scope);
            let row = rows[i];
            let mut m = 0u16;

            if row.held != 0 {
                m |= HELD;
            }
            if (i > 0 && row.time - rows[i - 1].time < GRACE_MS) || (i + 1 < n && rows[i + 1].time - row.time < GRACE_MS) {
                m |= GRACE;
            }
            if row.mask & scope.thumb != 0 {
                m |= THUMB;
            }
            if row.mask & scope.scratch != 0 {
                m |= SCRATCH;
            }
            if scope.groups.len() > 1 {
                if scope.groups.iter().filter(|&&g| row.mask & g != 0).count() > 1 {
                    m |= SPLIT;
                }
                let used = window.iter().fold(0, |acc, r| acc | r.mask);
                if window.len() >= 4 && scope.groups.iter().any(|&g| used & !g == 0) {
                    m |= ONE_HAND;
                }
            }
            if let Some(keys) = scope.mirror {
                if row.mask.count_ones() > 1 && mirror(row.mask, keys) == row.mask {
                    m |= SYMMETRIC;
                }
            }
            if (3..=8).contains(&(last[i] + 1 - first[i])) {
                m |= BURST;
            }
            if !pattern.jacky() {
                let prev = i > lo && rows[i - 1].mask & row.mask != 0;
                let next = i < hi && rows[i + 1].mask & row.mask != 0;
                if prev || next {
                    m |= MINIJACK;
                }
                if window.len() >= 6 {
                    let anchored = (0..32).filter(|k| row.mask & 1 << k != 0).any(|k| {
                        window.iter().filter(|r| r.mask & 1 << k != 0).count() * 2 >= window.len()
                    });
                    if anchored {
                        m |= ANCHOR;
                    }
                }
            }

            Label { pattern, modifiers: m }
        })
        .collect()
}

/// The rows of one group, keeping only its keys, and where they are in the
/// whole map.
pub fn group_rows(rows: &[Row], mask: u32) -> (Vec<Row>, Vec<usize>) {
    rows.iter()
        .enumerate()
        .filter(|(_, r)| r.mask & mask != 0)
        .map(|(i, r)| (Row { mask: r.mask & mask, ln: r.ln & mask, held: r.held & mask, inverse: r.inverse & mask, ..*r }, i))
        .unzip()
}

/// Every row of a map, labelled for the whole map and for each hand.
pub struct Labelled {
    pub keys: usize,
    pub rows: Vec<Row>,
    pub labels: Vec<Label>,
    pub zones: Vec<Zone>,
    pub groups: Vec<GroupLabels>,
}

pub struct GroupLabels {
    pub group: Group,
    /// Indices into the rows of the map.
    pub rows: Vec<usize>,
    pub labels: Vec<Label>,
}

pub fn label(beatmap: &Beatmap, layout: &Layout) -> Labelled {
    let rows = rows(beatmap);
    label_rows(rows, layout)
}

pub fn label_rows(rows: Vec<Row>, layout: &Layout) -> Labelled {
    let groups = layout.groups();
    let masks: Vec<u32> = groups.iter().map(|g| g.mask).collect();
    let thumb = layout.digit_mask(Digit::Thumb);
    let either = layout.fingers.iter().enumerate().filter(|(_, f)| f.hand == Hand::Either).fold(0, |m, (c, _)| m | 1 << c);
    let scratch = layout.digit_mask(Digit::Scratch);
    let mirror = layout.symmetric().then(|| layout.keys());

    let labels = classify(&rows, &Scope { groups: &masks, thumb, either, scratch, mirror });
    let groups = groups
        .into_iter()
        .map(|group| {
            let (sub, idx) = group_rows(&rows, group.mask);
            let labels = classify(&sub, &Scope { groups: &[group.mask], thumb, either, scratch, mirror: None });
            GroupLabels { group, rows: idx, labels }
        })
        .collect();

    let zones = zones::zones(&rows);
    Labelled { keys: layout.keys(), rows, labels, zones, groups }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternShare {
    pub pattern: Pattern,
    pub rows: usize,
    pub notes: usize,
    /// Share of the notes.
    pub share: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub pattern: Pattern,
    pub start: f64,
    pub end: f64,
    pub rows: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupAnalysis {
    pub name: String,
    pub keys: Vec<usize>,
    /// Share of the map's notes this hand plays.
    pub share: f64,
    pub patterns: Vec<PatternShare>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnStats {
    pub key: usize,
    pub notes: usize,
    pub long_notes: usize,
    /// Notes repeating the previous row's note in this key, inside a phrase.
    pub jacks: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapAnalysis {
    pub keys: usize,
    pub layout: String,
    pub rows: usize,
    pub notes: usize,
    pub patterns: Vec<PatternShare>,
    /// How many rows have each modifier.
    pub modifiers: BTreeMap<String, usize>,
    pub groups: Vec<GroupAnalysis>,
    pub columns: Vec<ColumnStats>,
    pub sections: Vec<Section>,
    /// Where the map rests and where it peaks.
    pub zones: Vec<ZoneSection>,
}

fn shares(rows: &[Row], idx: impl Iterator<Item = (usize, Pattern)>, mask: u32) -> Vec<PatternShare> {
    let mut counts: BTreeMap<Pattern, (usize, usize)> = BTreeMap::new();
    let mut total = 0;
    for (i, pattern) in idx {
        let notes = (rows[i].mask & mask).count_ones() as usize;
        let entry = counts.entry(pattern).or_default();
        entry.0 += 1;
        entry.1 += notes;
        total += notes;
    }
    let mut out: Vec<PatternShare> = counts
        .into_iter()
        .map(|(pattern, (rows, notes))| PatternShare { pattern, rows, notes, share: notes as f64 / total.max(1) as f64 })
        .collect();
    out.sort_by(|a, b| b.notes.cmp(&a.notes));
    out
}

pub fn analyze(beatmap: &Beatmap, layout: &Layout) -> MapAnalysis {
    summarize(&label(beatmap, layout), layout)
}

pub fn summarize(l: &Labelled, layout: &Layout) -> MapAnalysis {
    let all = u32::MAX;
    let notes: usize = l.rows.iter().map(|r| r.mask.count_ones() as usize).sum();

    let mut modifiers = BTreeMap::new();
    for label in &l.labels {
        for (bit, name) in modifier::NAMES {
            if label.modifiers & bit != 0 {
                *modifiers.entry(name.to_string()).or_insert(0) += 1;
            }
        }
    }

    let groups = l
        .groups
        .iter()
        .map(|g| GroupAnalysis {
            name: g.group.name.clone(),
            keys: (0..l.keys).filter(|k| g.group.mask & 1 << k != 0).collect(),
            share: g.rows.iter().map(|&i| (l.rows[i].mask & g.group.mask).count_ones() as usize).sum::<usize>() as f64
                / notes.max(1) as f64,
            patterns: shares(&l.rows, g.rows.iter().copied().zip(g.labels.iter().map(|x| x.pattern)), g.group.mask),
        })
        .collect();

    let mut columns: Vec<ColumnStats> =
        (0..l.keys).map(|key| ColumnStats { key, notes: 0, long_notes: 0, jacks: 0 }).collect();
    for (i, row) in l.rows.iter().enumerate() {
        let linked = i > 0 && row.time - l.rows[i - 1].time <= link_ms(row.beat_len);
        for (k, col) in columns.iter_mut().enumerate() {
            if row.mask & 1 << k == 0 {
                continue;
            }
            col.notes += 1;
            col.long_notes += (row.ln >> k & 1) as usize;
            col.jacks += (linked && l.rows[i - 1].mask & 1 << k != 0) as usize;
        }
    }

    let mut sections = Vec::new();
    let mut start = 0;
    for i in 1..=l.rows.len() {
        if i < l.rows.len() && l.labels[i].pattern == l.labels[start].pattern {
            continue;
        }
        let pattern = l.labels[start].pattern;
        if i - start >= SECTION_MIN_ROWS && pattern != Pattern::Sparse {
            sections.push(Section { pattern, start: l.rows[start].time, end: l.rows[i - 1].time, rows: i - start });
        }
        start = i;
    }

    MapAnalysis {
        keys: l.keys,
        layout: layout.name.to_string(),
        rows: l.rows.len(),
        notes,
        patterns: shares(&l.rows, l.labels.iter().map(|x| x.pattern).enumerate(), all),
        modifiers,
        groups,
        columns,
        sections,
        zones: zones::sections(&l.rows, &l.zones),
    }
}

#[cfg(test)]
mod tests {
    use super::super::layout::layout;
    use super::*;

    /// A map from rows of columns, in 1/4s at 120 BPM.
    fn map(keys: usize, rows: &[&[usize]]) -> Beatmap {
        let mut osu = format!(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:{}\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
            keys
        );
        for (step, row) in rows.iter().enumerate() {
            for &col in row.iter() {
                let x = (col * 512 + 256) / keys;
                osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", x, step * 125));
            }
        }
        rosu_map::from_str::<Beatmap>(&osu).unwrap()
    }

    /// The pattern of the middle row.
    fn pattern(keys: usize, rows: &[&[usize]]) -> Pattern {
        let l = label(&map(keys, rows), &layout(keys, None));
        l.labels[rows.len() / 2].pattern
    }

    fn repeat(rows: &[&'static [usize]], times: usize) -> Vec<&'static [usize]> {
        rows.iter().copied().cycle().take(rows.len() * times).collect()
    }

    #[test]
    fn spots_the_basic_patterns() {
        assert_eq!(pattern(4, &repeat(&[&[0], &[1], &[2], &[3]], 4)), Pattern::Roll);
        assert_eq!(pattern(4, &repeat(&[&[0], &[2], &[1], &[3]], 4)), Pattern::Stream);
        assert_eq!(pattern(4, &repeat(&[&[0], &[1]], 8)), Pattern::Trill);
        assert_eq!(pattern(4, &repeat(&[&[0, 1], &[2, 3]], 8)), Pattern::Jumptrill);
        assert_eq!(pattern(4, &repeat(&[&[0, 2], &[1, 3]], 8)), Pattern::Bracket);
        assert_eq!(pattern(4, &repeat(&[&[0, 1], &[2], &[0, 3], &[1]], 4)), Pattern::Jumpstream);
        assert_eq!(pattern(4, &repeat(&[&[0, 1, 2], &[3], &[0, 2, 3], &[1]], 4)), Pattern::Handstream);
        assert_eq!(pattern(4, &repeat(&[&[0, 1, 3], &[0, 2, 3]], 8)), Pattern::DenseChordjack);
        assert_eq!(pattern(4, &repeat(&[&[0, 1], &[1, 2]], 8)), Pattern::Chordjack);
        assert_eq!(pattern(4, &repeat(&[&[2]], 16)), Pattern::Jack);
        assert_eq!(pattern(7, &repeat(&[&[0, 1, 4, 5], &[2], &[1, 3, 5, 6], &[4]], 4)), Pattern::Chordstream);
        // A 7K jumptrill changing its chords, as Ice Angel's Euphoria does.
        assert_eq!(pattern(7, &repeat(&[&[5, 6], &[0, 1], &[4, 5], &[1, 2, 3], &[4, 6], &[0, 2]], 3)), Pattern::Jumptrill);
    }

    #[test]
    fn labels_each_hand_on_its_own() {
        // A 4K stream 0 2 1 3: each hand trills.
        let l = label(&map(4, &repeat(&[&[0], &[2], &[1], &[3]], 6)), &layout(4, None));
        for g in &l.groups {
            assert_eq!(g.labels[g.labels.len() / 2].pattern, Pattern::Trill, "{}", g.group.name);
        }
        // Every row is on one hand or the other.
        assert!(l.labels.iter().all(|x| x.modifiers & modifier::SPLIT == 0));
    }

    #[test]
    fn spots_modifiers() {
        let l = label(&map(7, &repeat(&[&[0, 6], &[3], &[1, 5], &[3]], 4)), &layout(7, None));
        let m = l.labels[8].modifiers;
        assert!(m & modifier::SYMMETRIC != 0 && m & modifier::SPLIT != 0, "{:b}", m);
        assert!(l.labels[9].modifiers & modifier::THUMB != 0);
        assert!(l.labels[9].modifiers & modifier::ANCHOR != 0);
    }

    #[test]
    fn summarizes_a_map() {
        let a = analyze(&map(4, &repeat(&[&[0, 1], &[2, 3]], 8)), &layout(4, None));
        assert_eq!(a.patterns[0].pattern, Pattern::Jumptrill);
        assert_eq!(a.sections.len(), 1);
        assert_eq!(a.groups.len(), 2);
        assert_eq!(a.columns.iter().map(|c| c.notes).sum::<usize>(), 32);
    }
}
