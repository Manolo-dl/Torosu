//! Converts an osu!mania map to another key count, in either direction.
//!
//! Based on osu-mania-converter (4K/7K -> 8K/10K), but instead of picking a
//! column mapping per beat it decides every row (the notes that start at the
//! same time) on its own, with a beam search over a cost that describes what a
//! good pattern is:
//!
//! - **Flow**: when the source moves right, the target moves right too, about
//!   as far. Now and then two connected notes (next keys) grow into three, but
//!   longer runs, or runs going the same way again and again, cost more: the
//!   rolls stay varied.
//! - **Hands**: a note stays on its hand (left, right, or the middle key on odd
//!   key counts), but may cross over now and then to keep a stream flowing.
//! - **Repeated patterns** are spotted in the source and copied as they are:
//!   a chordjack (the same row again, up to 1/2 beat later) repeats its whole
//!   target row, extra notes included, and a jumptrill or trill (two rows
//!   alternating) alternates between two fixed target rows, one per hand,
//!   like tyrcs' 7K Lolit speed does. Single-note trills need a few more notes
//!   to count: the short back-and-forths a 4K stream is full of, just because
//!   it has 4 keys, are not copied. A key coming back 2 or 3 notes later
//!   within a beat costs more there, so streams roll over the extra keys.
//! - **Consistency**: each column keeps loosely to the keys around its natural
//!   place.
//! - **Spread**: a key that was just used costs more, decaying over a beat. The
//!   notes spread over every key, without anchors or minijacks.
//! - **Jacks**: no new jacks; the map's own jacks (the same column in
//!   consecutive rows, up to 1/2 beat apart) stay on a single key.
//! - **Extra notes** (optional, only with more keys) widen the chords into the
//!   keys next to them, on the same hand, as hand conversions do (7K Delta Dan),
//!   more on the stronger beats. *Full* scales every chord with the key count
//!   instead, like Enie's 7K H.A.R.D.C.O.R.E. does with the 4K one: jumps
//!   become hands and hands become quints, so handstreams stay handstreams.
//!   A full row of the source (a quad on 4K) only grows by a note or so, as
//!   in tyrcs' Lolit speed, not into a wall.
//!
//! - **Fewer keys**: chords shrink with the key count, as hand-made 4K maps
//!   of the same songs do against their 7K versions (Delta Dan, Lolit speed,
//!   H.A.R.D.C.O.R.E.): 2 notes -> 1, 3 or 4 -> 2, 5 or 6 -> 3. Notes are
//!   dropped rather than turned into jacks.
//!
//! A note that cannot be placed (every key it could go to is held) is dropped,
//! so the output is always a valid map.

use anyhow::{Result, bail};
use rosu_map::{
    Beatmap,
    section::{
        general::GameMode,
        hit_objects::{HitObject, HitObjectKind},
    },
};

use crate::commands::trainer::{
    model::{Density, KeyConverterParams},
    naming,
};

/// osu! stable plays 1K to 10K.
pub const MAX_KEYS: usize = 10;

const BEAM_WIDTH: usize = 8;

/// Notes in the same column closer than this (in beats) are a jack, as in the
/// original: a 1/4 with some leniency. New ones are avoided.
const JACK_BEATS: f64 = 0.375;
/// The same column in consecutive source rows up to this far apart (in beats)
/// is a jack of the map, kept on one key: fast maps jack at 1/2.
const SOURCE_JACK_BEATS: f64 = 0.55;

/// Leniency for notes a couple of ms off the grid.
const GRID_LENIENCY_MS: f64 = 2.0;

/// Rows further apart than this (in beats) are not compared for flow.
const FLOW_BEATS: f64 = 2.0;

const COST_DROP: f64 = 150.0;
/// Going down in keys, dropping one more note than the row's share: cheaper
/// than a new jack, so the map thins out instead of jacking.
const COST_DROP_REDUCING: f64 = 60.0;
const COST_NEW_JACK: f64 = 100.0;
const COST_BROKEN_JACK: f64 = 40.0;
/// Reward for an extra note on a downbeat or a chord; less elsewhere.
const EXTRA_REWARD: f64 = 10.0;
/// How far (in keys) a column can stray from its natural place for free.
const FREE_DEVIATION: f64 = 0.75;
/// A source trill: two rows alternating for at least this many rows (single
/// notes) or chord rows (jumptrills), at most this far apart (in beats).
const TRILL_MIN_NOTES: usize = 6;
const TRILL_MIN_CHORDS: usize = 4;
const TRILL_MAX_GAP_BEATS: f64 = 0.5;
/// Breaking a real trill.
const COST_BROKEN_TRILL: f64 = 10.0;
/// Rows of history kept to spot back-and-forths and to copy patterns.
const HISTORY: usize = 3;
/// Reward for copying a repeated pattern (see [`mark_patterns`]): it wins
/// whenever it fits.
const W_COPY: f64 = 40.0;
/// Upper bound of the noise that breaks ties between equal choices, so the
/// patterns do not always go the same way.
const TIE_BREAK: f64 = 0.3;

// Weights of the pattern costs, tuned against hand conversions (Delta Dan,
// krrcream's packs) and feedback on Black dress.

/// Moving the other way than the source.
const W_DIRECTION: f64 = 3.0;
/// Per key of difference between how far the source and the target move
/// (relative to the key count).
const W_SHAPE: f64 = 0.5;
/// Reward for turning two connected notes (next keys, same way) into three,
/// when the source goes that way too.
const W_ROLL: f64 = 1.0;
/// Making a connected run longer than three notes.
const W_LONG_ROLL: f64 = 4.0;
/// Starting a connected run the same way as the last one.
const W_SAME_ROLL: f64 = 2.0;
/// Using a key that was just released, decaying over a beat.
const W_FRESH: f64 = 10.0;
/// The same in chord rows, where a key coming back from 2 rows ago anchors a
/// jumpstream instead of breaking a stream.
const W_FRESH_CHORDS: f64 = 4.0;
/// A column going to another key than its previous note.
const W_SWITCH: f64 = 0.3;
/// Per squared key a column strays from its natural place.
const W_DEVIATION: f64 = 1.0;
/// A key coming back 2 / 3 notes later within a beat, outside a trill.
const W_REPEAT2: f64 = 12.0;
const W_REPEAT3: f64 = 6.0;
/// A note going to the other hand.
const W_CROSS: f64 = 6.0;

/// Most extra notes a row can get.
const MAX_EXTRAS: usize = 4;
/// Full: chords grow by the key ratio times this. Measured on Enie's 7K
/// H.A.R.D.C.O.R.E. against the 4K: 1 note -> 1.5, 2 -> 3.4, 3 -> 4.6.
const FULL_SCALE: f64 = 0.9;
/// Full: how much a full row of the source grows (tyrcs: a quad -> 4.1).
const FULL_ROW_GROWTH: f64 = 0.5;
/// Full: reward per extra note, high enough to fill every row's quota.
const FULL_REWARD: f64 = 25.0;
/// Full: an extra note in a key used a moment ago (a minijack). Handstreams
/// on more keys cannot avoid them all.
const W_EXTRA_JACK: f64 = 8.0;

#[derive(Clone, Copy)]
struct Fill {
    /// Most extra notes per row; with `scaled`, each row's quota instead.
    max: usize,
    /// Rows weaker than this (see [`beat_strength`]) get none.
    min_strength: f64,
    /// Min gap in beats from the previous note in that key.
    gap_beats: f64,
    /// Chords grow with the key count (Full).
    scaled: bool,
}

fn fill_settings(density: Density) -> Option<Fill> {
    match density {
        Density::None => None,
        Density::Dense => Some(Fill { max: 1, min_strength: 0.7, gap_beats: 0.5, scaled: false }),
        Density::MoreDense => Some(Fill { max: 2, min_strength: 0.0, gap_beats: JACK_BEATS, scaled: false }),
        Density::Full => Some(Fill { max: MAX_EXTRAS, min_strength: 0.0, gap_beats: 0.0, scaled: true }),
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Side {
    Left,
    Middle,
    Right,
}

fn side(col: usize, keys: usize) -> Side {
    match (2 * col + 1).cmp(&keys) {
        std::cmp::Ordering::Less => Side::Left,
        std::cmp::Ordering::Equal => Side::Middle,
        std::cmp::Ordering::Greater => Side::Right,
    }
}

/// A note stays on its hand; the middle key goes either way.
fn same_hand(src: Side, dst: Side) -> bool {
    src == Side::Middle || dst == Side::Middle || src == dst
}

/// Every set of `size` keys out of `keys`, sorted, indexed by size.
fn combinations(keys: usize) -> Vec<Vec<Vec<u8>>> {
    let mut by_size = vec![Vec::new(); keys + 1];
    for mask in 1u32..(1 << keys) {
        let combo: Vec<u8> = (0..keys as u8).filter(|&c| mask & (1 << c) != 0).collect();
        by_size[combo.len()].push(combo);
    }
    by_size
}

/// Which of `k` notes (sorted by column) are kept when only `m` fit: spread
/// out, so the outer notes of a chord survive.
fn kept(k: usize, m: usize) -> Vec<usize> {
    match m {
        0 => Vec::new(),
        1 => vec![k / 2],
        _ => (0..m).map(|i| (i * (k - 1) + (m - 1) / 2) / (m - 1)).collect(),
    }
}

struct Note {
    /// Index in `beatmap.hit_objects`.
    idx: usize,
    end: f64,
    src: u8,
}

struct Row {
    time: f64,
    /// Indices into the notes, sorted by source column.
    notes: std::ops::Range<usize>,
    beat_len: f64,
    /// How much an extra note fits here: 1 on the beat, less off it.
    strength: f64,
    /// Part of a trill or jumptrill of the source: keys coming back are fine.
    trill: bool,
    /// Repeats the row this many rows back (1: chordjack, 2: trill), so its
    /// target row is copied too. 0 when it does not.
    copy: u8,
    /// Part of a repeated pattern: its extra notes stay on the same hand as
    /// the chord, so each row of a jumptrill sits on one hand.
    grouped: bool,
    /// Full: how many extra notes this row should get.
    quota: u8,
    /// How many of its notes fit on the target keys: all of them, unless
    /// going down in keys.
    share: u8,
}

const NONE: u8 = u8::MAX;

#[derive(Clone, Copy)]
struct State {
    /// Per target key: when it is free again, and what was there.
    release: [f64; MAX_KEYS],
    last_src: [u8; MAX_KEYS],
    last_is_ln: [bool; MAX_KEYS],
    /// Per source column: its last note, to keep its jacks on one key.
    src_release: [f64; MAX_KEYS],
    src_target: [u8; MAX_KEYS],
    /// Time and average position (0..1) of the previous row, in source and
    /// target.
    prev: Option<(f64, f64, f64)>,
    /// The previous row when it was a single note: its source and target
    /// columns, the step that led to it when it was part of a connected run
    /// (±1, else 0), and how many notes that run has.
    single: Option<(u8, u8, i8, u8)>,
    /// The way the last connected run of three or more went (±1).
    last_roll: i8,
    /// The last rows, newest first.
    history: [Past; HISTORY],
}

impl State {
    fn new() -> Self {
        Self {
            release: [f64::NEG_INFINITY; MAX_KEYS],
            last_src: [NONE; MAX_KEYS],
            last_is_ln: [false; MAX_KEYS],
            src_release: [f64::NEG_INFINITY; MAX_KEYS],
            src_target: [NONE; MAX_KEYS],
            prev: None,
            single: None,
            last_roll: 0,
            history: [Past { time: f64::NEG_INFINITY, src: 0, dst: 0, decision: Decision::DROPPED }; HISTORY],
        }
    }
}

/// What was done with a row: the keys of the notes kept (the `size` of them
/// picked by [`kept`]) and the keys of the extra notes.
#[derive(Clone, Copy)]
struct Decision {
    size: u8,
    combo: u16,
    extras: [u8; MAX_EXTRAS],
    n_extras: u8,
}

impl Decision {
    const DROPPED: Self = Self { size: 0, combo: 0, extras: [0; MAX_EXTRAS], n_extras: 0 };
}

/// A row already placed: its source and target columns as bit masks.
#[derive(Clone, Copy)]
struct Past {
    time: f64,
    src: u16,
    dst: u16,
    decision: Decision,
}

struct Ctx<'a> {
    notes: &'a [Note],
    combos: Vec<Vec<Vec<u8>>>,
    src_keys: usize,
    dst_keys: usize,
    keep_jacks: bool,
    fill: Option<Fill>,
}

fn pos(col: usize, keys: usize) -> f64 {
    (col as f64 + 0.5) / keys as f64
}

impl Ctx<'_> {
    fn fresh(&self, state: &State, col: usize, row: &Row) -> f64 {
        let weight = if row.notes.len() > 1 || row.quota > 0 { W_FRESH_CHORDS } else { W_FRESH };
        weight * (-(row.time - state.release[col]) / row.beat_len).exp()
    }

    fn free(&self, state: &State, col: usize, row: &Row, src: u8) -> bool {
        let chained = row.time == state.release[col] && state.last_src[col] == src && state.last_is_ln[col];
        row.time > state.release[col] || chained
    }

    /// Cost of each source note of the row in each target key, `INFINITY`
    /// where it cannot go.
    fn note_costs(&self, state: &State, row: &Row) -> [[f64; MAX_KEYS]; MAX_KEYS] {
        let mut costs = [[f64::INFINITY; MAX_KEYS]; MAX_KEYS];
        let jack_gap = row.beat_len * JACK_BEATS;

        for (i, note) in self.notes[row.notes.clone()].iter().enumerate() {
            let src = note.src as usize;
            // A jack is the same column in consecutive rows. Coming back to
            // it after other notes, as fast 4K streams do, is not one.
            let source_jack = state.src_target[src] != NONE
                && state.history[0].src & (1 << src) != 0
                && row.time - state.src_release[src] <= row.beat_len * SOURCE_JACK_BEATS;

            for col in 0..self.dst_keys {
                if !self.free(state, col, row, note.src) {
                    continue;
                }

                let mut cost = 0.0;
                if !same_hand(side(src, self.src_keys), side(col, self.dst_keys)) {
                    cost += W_CROSS;
                }
                let keeps_jack = source_jack && state.src_target[src] as usize == col;

                if source_jack && self.keep_jacks && !keeps_jack {
                    cost += COST_BROKEN_JACK;
                }
                if !keeps_jack {
                    cost += self.repeat_cost(state, row, src, col);
                    if state.last_src[col] != note.src && row.time - state.release[col] <= jack_gap {
                        cost += COST_NEW_JACK;
                    }
                    cost += self.fresh(state, col, row);
                }

                if state.src_target[src] != NONE && state.src_target[src] as usize != col {
                    cost += W_SWITCH;
                }
                let natural = pos(src, self.src_keys) * self.dst_keys as f64 - 0.5;
                let stray = ((col as f64 - natural).abs() - FREE_DEVIATION).max(0.0);
                cost += W_DEVIATION * stray * stray;

                costs[i][col] = cost;
            }
        }

        costs
    }

    /// Cost of a key coming back 2 or 3 notes later (a back-and-forth), and
    /// of breaking a real trill. Only between single notes: in jumpstreams a
    /// chord sharing a key with the one 2 rows back is what holds them
    /// together (tyrcs' Lolit speed: 82% of the rows).
    fn repeat_cost(&self, state: &State, row: &Row, src: usize, col: usize) -> f64 {
        let mut cost = 0.0;
        for (lag, weight) in [(2, W_REPEAT2), (3, W_REPEAT3)] {
            let past = &state.history[lag - 1];
            if row.time - past.time > row.beat_len {
                break;
            }
            if row.notes.len() > 1 || past.dst.count_ones() > 1 {
                continue;
            }
            let src_repeats = past.src & (1 << src) != 0;
            let dst_repeats = past.dst & (1 << col) != 0;

            if row.trill && lag == 2 {
                if src_repeats && !dst_repeats {
                    cost += COST_BROKEN_TRILL;
                }
            } else if dst_repeats {
                cost += weight;
            }
        }
        cost
    }

    /// Cost of an extra note in each key (negative: a reward), `INFINITY`
    /// where it does not fit.
    fn extra_costs(&self, state: &State, row: &Row) -> [f64; MAX_KEYS] {
        let mut costs = [f64::INFINITY; MAX_KEYS];
        let Some(fill) = self.fill else { return costs };
        if row.strength < fill.min_strength {
            return costs;
        }

        let gap = row.beat_len * fill.gap_beats;
        let jack_gap = row.beat_len * JACK_BEATS;
        let reward = if fill.scaled { FULL_REWARD } else { EXTRA_REWARD * row.strength };

        for (col, cost) in costs.iter_mut().enumerate().take(self.dst_keys) {
            let since = row.time - state.release[col];
            if since > 0.0 && since >= gap {
                let jack = if since <= jack_gap { W_EXTRA_JACK } else { 0.0 };
                *cost = self.fresh(state, col, row) + jack - reward;
            }
        }
        costs
    }

    /// The best extra notes for a row placed in `combo`: the chord grows
    /// into the keys next to it, on the same hand, one note at a time. Full
    /// takes any free key instead, so big chords stay open (`O.O.O.O`, not
    /// `OOOO...`), as in hand conversions of handstreams.
    fn pick_extras(&self, row: &Row, combo: &[u8], extra_costs: &[f64; MAX_KEYS]) -> ([u8; MAX_EXTRAS], u8, f64) {
        let mut extras = [0u8; MAX_EXTRAS];
        let Some(fill) = self.fill else { return (extras, 0, 0.0) };
        let max = if fill.scaled { row.quota as usize } else { fill.max };

        // Full spreads the extras over any key, except in repeated patterns.
        let anywhere = fill.scaled && !row.grouped;

        let mut n = 0;
        let mut total = 0.0;
        while n < max {
            let mut best: Option<(f64, u8)> = None;
            if anywhere {
                for next in 0..self.dst_keys as u8 {
                    let cost = extra_costs[next as usize];
                    if cost < 0.0
                        && !combo.contains(&next)
                        && !extras[..n].contains(&next)
                        && best.is_none_or(|(b, _)| cost < b)
                    {
                        best = Some((cost, next));
                    }
                }
            }
            for &col in combo.iter().chain(&extras[..n]).filter(|_| !anywhere) {
                let neighbours = [col.checked_sub(1), Some(col + 1).filter(|&c| (c as usize) < self.dst_keys)];
                for next in neighbours.into_iter().flatten() {
                    // Without Full, a second extra in the same row is worth
                    // half: rows mostly grow by one note, sometimes two.
                    let cost = extra_costs[next as usize] + if n > 0 && !fill.scaled { EXTRA_REWARD / 2.0 } else { 0.0 };
                    if cost < 0.0
                        && !combo.contains(&next)
                        && !extras[..n].contains(&next)
                        && same_hand(side(col as usize, self.dst_keys), side(next as usize, self.dst_keys))
                        && best.is_none_or(|(b, _)| cost < b)
                    {
                        best = Some((cost, next));
                    }
                }
            }
            let Some((cost, col)) = best else { break };
            extras[n] = col;
            total += cost;
            n += 1;
        }
        (extras, n as u8, total)
    }

    /// Cost of the flow from the previous row to this one. `single` is the
    /// source and target column when both rows are single notes.
    fn flow_cost(&self, state: &State, row: &Row, src_pos: f64, dst_pos: f64, single: Option<(u8, u8)>) -> f64 {
        let Some((prev_time, prev_src, prev_dst)) = state.prev else { return 0.0 };
        if row.time - prev_time > row.beat_len * FLOW_BEATS {
            return 0.0;
        }

        let ds = src_pos - prev_src;
        let dt = dst_pos - prev_dst;
        let mut cost = 0.0;
        if ds.abs() > 1e-9 && (dt.abs() < 1e-9 || ds.signum() != dt.signum()) {
            cost += W_DIRECTION;
        }

        cost += W_SHAPE * (ds - dt).abs() * self.dst_keys as f64;

        if let (Some((src, dst)), Some((prev_src, prev_dst, prev_step, run))) = (single, state.single) {
            let ds = src as i32 - prev_src as i32;
            let dt = dst as i32 - prev_dst as i32;
            if dt.abs() == 1 && dt == prev_step as i32 {
                match run {
                    2 if ds.signum() == dt.signum() && dt != state.last_roll as i32 => cost -= W_ROLL,
                    2 => cost += W_SAME_ROLL,
                    _ => cost += W_LONG_ROLL,
                }
            }
        }
        cost
    }

    /// The source and target column of a single note row placed in `combo`.
    fn single(&self, row: &Row, keep: &[usize], combo: &[u8]) -> Option<(u8, u8)> {
        (row.notes.len() == 1 && combo.len() == 1).then(|| (self.notes[row.notes.start + keep[0]].src, combo[0]))
    }

    fn source_pos(&self, row: &Row, keep: &[usize]) -> f64 {
        let notes = &self.notes[row.notes.clone()];
        keep.iter().map(|&i| pos(notes[i].src as usize, self.src_keys)).sum::<f64>() / keep.len() as f64
    }

    /// Every way to place the row, with its cost. Fewer notes are only tried
    /// when they do not all fit.
    fn candidates(&self, state: &State, row: &Row, row_idx: usize, out: &mut Vec<(f64, Decision)>) {
        let k = row.notes.len();

        // A repeated pattern copies the row it repeats, whenever it fits.
        if row.copy > 0 {
            let past = state.history[row.copy as usize - 1].decision;
            if self.fits(state, row, &past) {
                out.push((-W_COPY, past));
                return;
            }
        }

        let costs = self.note_costs(state, row);
        let extra_costs = self.extra_costs(state, row);

        // Going up, fewer notes are only tried when they do not all fit. Going
        // down, every size up to the row's share competes, so a note can be
        // dropped to avoid a jack.
        let reducing = self.dst_keys < self.src_keys;
        let share = (row.share as usize).min(self.dst_keys);
        let drop_cost = if reducing { COST_DROP_REDUCING } else { COST_DROP };

        for size in (1..=share).rev() {
            let keep = kept(k, size);
            let src_pos = self.source_pos(row, &keep);
            let dropped = (share - size) as f64 * drop_cost;
            let before = out.len();

            for (ci, combo) in self.combos[size].iter().enumerate() {
                let mut cost = dropped;
                for (&note, &col) in keep.iter().zip(combo) {
                    cost += costs[note][col as usize];
                }
                if !cost.is_finite() {
                    continue;
                }

                let dst_pos = combo.iter().map(|&c| pos(c as usize, self.dst_keys)).sum::<f64>() / size as f64;
                cost += self.flow_cost(state, row, src_pos, dst_pos, self.single(row, &keep, combo));

                let (extras, n_extras, extra_cost) = self.pick_extras(row, combo, &extra_costs);
                let decision = Decision { size: size as u8, combo: ci as u16, extras, n_extras };

                out.push((cost + extra_cost + tie_break(row_idx, ci), decision));
            }

            if out.len() > before && !reducing {
                return;
            }
        }

        // Every key is held: the whole row is dropped.
        out.push((share as f64 * drop_cost, Decision::DROPPED));
    }

    /// Whether `decision`, taken from a row with the same source columns, can
    /// place this row too: every key it uses is free.
    fn fits(&self, state: &State, row: &Row, decision: &Decision) -> bool {
        let size = decision.size as usize;
        if size == 0 || size > row.notes.len() {
            return false;
        }
        let notes = &self.notes[row.notes.clone()];
        let combo = &self.combos[size][decision.combo as usize];
        kept(notes.len(), size)
            .iter()
            .zip(combo)
            .all(|(&i, &col)| self.free(state, col as usize, row, notes[i].src))
            && decision.extras[..decision.n_extras as usize].iter().all(|&col| row.time > state.release[col as usize])
    }

    fn commit(&self, state: &mut State, row: &Row, decision: &Decision) {
        let notes = &self.notes[row.notes.clone()];
        let size = decision.size as usize;
        let keep = kept(notes.len(), size);

        if size > 0 {
            let combo = &self.combos[size][decision.combo as usize];
            for (&i, &col) in keep.iter().zip(combo) {
                let (note, col) = (&notes[i], col as usize);
                state.release[col] = note.end;
                state.last_src[col] = note.src;
                state.last_is_ln[col] = note.end > row.time;
                state.src_release[note.src as usize] = note.end;
                state.src_target[note.src as usize] = col as u8;
            }

            let dst_pos = combo.iter().map(|&c| pos(c as usize, self.dst_keys)).sum::<f64>() / size as f64;
            state.prev = Some((row.time, self.source_pos(row, &keep), dst_pos));

            state.single = self.single(row, &keep, combo).map(|(src, dst)| {
                let Some((_, prev_dst, prev_step, run)) = state.single else { return (src, dst, 0, 1) };
                let step = dst as i32 - prev_dst as i32;
                if step.abs() != 1 {
                    return (src, dst, 0, 1);
                }
                let run = if step == prev_step as i32 { run + 1 } else { 2 };
                if run == 3 {
                    state.last_roll = step as i8;
                }
                (src, dst, step as i8, run)
            });
        } else {
            state.single = None;
        }

        let src_mask = notes.iter().fold(0u16, |m, n| m | 1 << n.src);
        let mut dst_mask = if size > 0 {
            self.combos[size][decision.combo as usize].iter().fold(0u16, |m, &c| m | 1 << c)
        } else {
            0
        };
        for &col in &decision.extras[..decision.n_extras as usize] {
            dst_mask |= 1 << col;
        }
        state.history.rotate_right(1);
        state.history[0] = Past { time: row.time, src: src_mask, dst: dst_mask, decision: *decision };

        for &col in &decision.extras[..decision.n_extras as usize] {
            let end = self.template(row, decision, col).map_or(row.time, |n| self.notes[n].end);
            state.release[col as usize] = end;
            state.last_src[col as usize] = NONE;
            state.last_is_ln[col as usize] = end > row.time;
        }
    }

    /// The note an extra note in `col` copies: the kept note closest to it.
    fn template(&self, row: &Row, decision: &Decision, col: u8) -> Option<usize> {
        let size = decision.size as usize;
        if size == 0 {
            return None;
        }
        let combo = &self.combos[size][decision.combo as usize];
        kept(row.notes.len(), size)
            .iter()
            .zip(combo)
            .min_by_key(|(_, c)| c.abs_diff(col))
            .map(|(&i, _)| row.notes.start + i)
    }
}

/// Deterministic noise in `[0, TIE_BREAK)`, so the same settings always give
/// the same map.
fn tie_break(row: usize, choice: usize) -> f64 {
    let mut x = (row as u64) << 32 | choice as u64;
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^= x >> 31;
    (x >> 11) as f64 / (1u64 << 53) as f64 * TIE_BREAK
}

/// Picks what to do with each row.
fn search(ctx: &Ctx, rows: &[Row]) -> Vec<Decision> {
    // Back pointers of every beam entry, to rebuild the best path at the end.
    let mut nodes: Vec<(u32, Decision)> = Vec::new();
    let mut beam: Vec<(f64, State, u32)> = vec![(0.0, State::new(), u32::MAX)];
    let mut candidates: Vec<(f64, usize, Decision)> = Vec::new();
    let mut scratch: Vec<(f64, Decision)> = Vec::new();

    for (ri, row) in rows.iter().enumerate() {
        candidates.clear();

        for (bi, (cost, state, _)) in beam.iter().enumerate() {
            scratch.clear();
            ctx.candidates(state, row, ri, &mut scratch);
            candidates.extend(scratch.iter().map(|&(c, d)| (cost + c, bi, d)));
        }

        let keep = BEAM_WIDTH.min(candidates.len());
        if candidates.len() > keep {
            candidates.select_nth_unstable_by(keep - 1, |a, b| a.0.total_cmp(&b.0));
            candidates.truncate(keep);
        }

        beam = candidates
            .iter()
            .map(|&(cost, bi, decision)| {
                let (_, parent, parent_node) = &beam[bi];
                let mut state = *parent;
                ctx.commit(&mut state, row, &decision);
                nodes.push((*parent_node, decision));
                (cost, state, (nodes.len() - 1) as u32)
            })
            .collect();
    }

    let mut node = beam.iter().min_by(|a, b| a.0.total_cmp(&b.0)).map_or(u32::MAX, |b| b.2);
    let mut path = Vec::with_capacity(rows.len());
    while node != u32::MAX {
        let (parent, decision) = nodes[node as usize];
        path.push(decision);
        node = parent;
    }
    path.reverse();
    path
}

fn column_x(col: usize, keys: usize) -> f32 {
    ((col as f32 + 0.5) * 512.0 / keys as f32).floor()
}

fn set_x(obj: &mut HitObject, x: f32) {
    match &mut obj.kind {
        HitObjectKind::Circle(circle) => circle.pos.x = x,
        HitObjectKind::Hold(hold) => hold.pos_x = x,
        _ => {}
    }
}

/// Spots the patterns of the source that repeat rows, so the conversion can
/// repeat them too (see [`Row::copy`]):
///
/// - Chordjacks and jacks: the same row again, up to [`SOURCE_JACK_BEATS`]
///   later. Not with split jacks.
/// - Jumptrills and trills: two rows with nothing in common alternating, fast,
///   for [`TRILL_MIN_CHORDS`] rows when they are chords or
///   [`TRILL_MIN_NOTES`] when they are single notes. Shorter back-and-forths
///   are just what streams look like on few keys.
fn mark_patterns(rows: &mut [Row], notes: &[Note], keep_jacks: bool) {
    let cols: Vec<u16> = rows.iter().map(|row| notes[row.notes.clone()].iter().fold(0u16, |m, n| m | 1 << n.src)).collect();

    if keep_jacks {
        for i in 1..rows.len() {
            if cols[i] == cols[i - 1] && rows[i].time - rows[i - 1].time <= rows[i].beat_len * SOURCE_JACK_BEATS {
                rows[i].copy = 1;
                rows[i].grouped = true;
                rows[i - 1].grouped = true;
            }
        }
    }

    // Length of the alternation ending at each row.
    let mut run = vec![0usize; rows.len()];
    for i in 2..rows.len() {
        let fast = rows[i].time - rows[i - 1].time <= rows[i].beat_len * TRILL_MAX_GAP_BEATS
            && rows[i - 1].time - rows[i - 2].time <= rows[i].beat_len * TRILL_MAX_GAP_BEATS;
        if fast && cols[i] == cols[i - 2] && cols[i] & cols[i - 1] == 0 {
            run[i] = if run[i - 1] > 0 { run[i - 1] + 1 } else { 3 };
        }
    }

    // Marks every run long enough, walking back from where each one ends.
    let mut i = rows.len();
    while i > 0 {
        i -= 1;
        if run[i] == 0 {
            continue;
        }
        let start = i + 1 - run[i];
        let chords = rows[start..=i].iter().all(|row| row.notes.len() > 1);
        if run[i] >= if chords { TRILL_MIN_CHORDS } else { TRILL_MIN_NOTES } {
            for (j, row) in rows[start..=i].iter_mut().enumerate() {
                row.trill = true;
                row.grouped = true;
                if j >= 2 {
                    row.copy = 2;
                }
            }
        }
        i = start;
    }
}

/// How strong a position in the beat is: on the beat, on a half, on a
/// quarter, or anything else.
fn beat_strength(time: f64, tp_time: f64, beat_len: f64) -> f64 {
    let frac = ((time - tp_time) / beat_len).rem_euclid(1.0);
    let tolerance = 3.0 / beat_len;
    let near = |x: f64| (frac - x).abs() < tolerance || (frac - x + 1.0).abs() < tolerance;

    if near(0.0) {
        1.0
    } else if near(0.5) {
        0.7
    } else if near(0.25) || near(0.75) {
        0.45
    } else {
        0.25
    }
}

pub fn apply(beatmap: &mut Beatmap, params: &KeyConverterParams) -> Result<()> {
    if beatmap.mode != GameMode::Mania {
        bail!("the key converter only works on osu!mania maps");
    }

    let src_keys = beatmap.circle_size.round().clamp(1.0, MAX_KEYS as f32) as usize;
    let dst_keys = params.target_keys as usize;
    if !(1..=MAX_KEYS).contains(&dst_keys) {
        bail!("the key converter supports 1K to {}K, not {}K", MAX_KEYS, dst_keys);
    }
    if src_keys == dst_keys {
        log::warn!("key converter: the map is already {}K, left as is", dst_keys);
        return Ok(());
    }

    // The beat grid. Gimmick maps use fake BPMs (0.001, 100000...), which are
    // skipped.
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

    // Rows: the notes that start at the same time, sorted by column.
    let mut notes: Vec<Note> = Vec::new();
    let mut rows: Vec<Row> = Vec::new();

    for (idx, obj) in beatmap.hit_objects.iter().enumerate() {
        let (x, end) = match &obj.kind {
            HitObjectKind::Circle(circle) => (circle.pos.x, obj.start_time),
            HitObjectKind::Hold(hold) => (hold.pos_x, obj.start_time + hold.duration),
            _ => continue,
        };
        let src = ((x * src_keys as f32 / 512.0).floor().max(0.0) as usize).min(src_keys - 1) as u8;
        let time = obj.start_time;

        match rows.last_mut() {
            Some(row) if (time - row.time).abs() < 1.0 => row.notes.end += 1,
            _ => {
                let tp = grid.partition_point(|&(t, _)| t <= time + GRID_LENIENCY_MS).saturating_sub(1);
                let (tp_time, beat_len) = grid[tp];
                rows.push(Row {
                    time,
                    notes: notes.len()..notes.len() + 1,
                    beat_len,
                    strength: beat_strength(time, tp_time, beat_len),
                    trill: false,
                    copy: 0,
                    grouped: false,
                    quota: 0,
                    share: 0,
                });
            }
        }
        notes.push(Note { idx, end, src });
    }

    for row in &mut rows {
        notes[row.notes.clone()].sort_by_key(|n| n.src);
        if row.notes.len() > 1 {
            row.strength = (row.strength + 0.3).min(1.0);
        }
    }

    mark_patterns(&mut rows, &notes, params.keep_jacks);

    // Extra notes only make sense with more keys.
    let fill = fill_settings(params.density).filter(|_| dst_keys > src_keys);

    // Going down, each row keeps its size times the key ratio, rounded, at
    // least one note: hand-made maps do it that consistently.
    for row in rows.iter_mut() {
        let k = row.notes.len();
        row.share = if dst_keys < src_keys {
            let size = (k as f64 * dst_keys as f64 / src_keys as f64).round() as usize;
            size.clamp(1, k.min(dst_keys)) as u8
        } else {
            k.min(dst_keys) as u8
        };
    }

    // Full: each row's chord grows to its size times the key ratio. The
    // fraction is dithered per row, so the average comes out right.
    if fill.is_some_and(|f| f.scaled) {
        let scale = dst_keys as f64 / src_keys as f64 * FULL_SCALE;
        for (i, row) in rows.iter_mut().enumerate() {
            let k = row.notes.len();
            let target = if k == src_keys { k as f64 + FULL_ROW_GROWTH } else { k as f64 * scale };
            // Any choice index no row uses, so it is independent noise.
            let dither = tie_break(i, u32::MAX as usize) / TIE_BREAK;
            let size = (target.floor() + if dither < target.fract() { 1.0 } else { 0.0 }) as usize;
            row.quota = size.saturating_sub(k).min(dst_keys - k.min(dst_keys)).min(MAX_EXTRAS) as u8;
        }
    }
    let ctx = Ctx {
        notes: &notes,
        combos: combinations(dst_keys),
        src_keys,
        dst_keys,
        keep_jacks: params.keep_jacks,
        fill,
    };

    let path = search(&ctx, &rows);

    let mut targets: Vec<Option<u8>> = vec![None; beatmap.hit_objects.len()];
    let mut extra: Vec<HitObject> = Vec::new();

    for (row, decision) in rows.iter().zip(&path) {
        let size = decision.size as usize;
        if size > 0 {
            let combo = &ctx.combos[size][decision.combo as usize];
            for (&i, &col) in kept(row.notes.len(), size).iter().zip(combo) {
                targets[notes[row.notes.start + i].idx] = Some(col);
            }
        }
        for &col in &decision.extras[..decision.n_extras as usize] {
            if let Some(n) = ctx.template(row, decision, col) {
                let mut copy = beatmap.hit_objects[notes[n].idx].clone();
                set_x(&mut copy, column_x(col as usize, dst_keys));
                extra.push(copy);
            }
        }
    }

    let total = beatmap.hit_objects.len();
    let mut objects: Vec<HitObject> = Vec::with_capacity(total + extra.len());
    for (mut obj, target) in std::mem::take(&mut beatmap.hit_objects).into_iter().zip(&targets) {
        match (&obj.kind, target) {
            (HitObjectKind::Circle(_) | HitObjectKind::Hold(_), Some(col)) => {
                set_x(&mut obj, column_x(*col as usize, dst_keys));
                objects.push(obj);
            }
            (HitObjectKind::Circle(_) | HitObjectKind::Hold(_), None) => {}
            _ => objects.push(obj),
        }
    }

    let dropped = total - objects.len();
    let added = extra.len();
    objects.extend(extra);
    objects.sort_by(|a, b| a.start_time.total_cmp(&b.start_time));

    beatmap.hit_objects = objects;
    beatmap.circle_size = dst_keys as f32;

    log::debug!(
        "key converter: {}K -> {}K, {} row(s), {} dropped, {} added",
        src_keys,
        dst_keys,
        rows.len(),
        dropped,
        added
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_outer_notes_of_a_chord() {
        assert_eq!(kept(4, 4), vec![0, 1, 2, 3]);
        assert_eq!(kept(4, 2), vec![0, 3]);
        assert_eq!(kept(5, 3), vec![0, 2, 4]);
        assert_eq!(kept(3, 1), vec![1]);
    }

    /// A map at 120 BPM with streams, jacks, chords and LNs, generated
    /// deterministically.
    fn map(keys: usize) -> String {
        let mut osu = format!(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:{}\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
            keys
        );
        let mut seed = 7u64;
        let mut busy = vec![-1i64; keys];
        for step in 0..400i64 {
            let time = step * 125;
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let chord = 1 + (seed >> 60) as usize % 3;
            for k in 0..chord {
                let col = ((seed >> (8 * k + 8)) as usize) % keys;
                if busy[col] >= time {
                    continue;
                }
                let x = ((col as f32 + 0.5) * 512.0 / keys as f32) as i32;
                if (seed >> 40) % 5 == 0 {
                    let end = time + 375;
                    busy[col] = end;
                    osu.push_str(&format!("{},192,{},128,0,{}:0:0:0:0:\n", x, time, end));
                } else {
                    busy[col] = time;
                    osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", x, time));
                }
            }
        }
        osu
    }

    fn stream(cols: &[usize]) -> String {
        let mut osu = String::from(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:4\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
        );
        for (step, col) in cols.iter().enumerate() {
            osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", col * 128 + 64, step * 125));
        }
        osu
    }

    /// Columns of a converted map, one per note.
    fn columns(beatmap: &Beatmap, keys: usize) -> Vec<usize> {
        beatmap.hit_objects.iter().map(|o| column(o, keys)).collect()
    }

    #[test]
    fn keeps_real_trills() {
        let (_, beatmap) = convert(&stream(&[0, 1, 0, 1, 0, 1, 0, 1, 0, 1]), 7, Density::None);
        let cols = columns(&beatmap, 7);
        assert!(cols.windows(3).all(|w| w[0] == w[2] && w[0] != w[1]), "{:?}", cols);
    }

    /// A 4K map from rows of columns, in 1/4s at 120 BPM.
    fn rows_map(rows: &[&[usize]]) -> String {
        let mut osu = String::from(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:4\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
        );
        for (step, row) in rows.iter().enumerate() {
            for col in row.iter() {
                osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", col * 128 + 64, step * 125));
            }
        }
        osu
    }

    /// The keys of each row of a converted map.
    fn key_rows(beatmap: &Beatmap, keys: usize) -> Vec<Vec<usize>> {
        let mut rows: Vec<(f64, Vec<usize>)> = Vec::new();
        for obj in &beatmap.hit_objects {
            match rows.last_mut() {
                Some((time, cols)) if *time == obj.start_time => cols.push(column(obj, keys)),
                _ => rows.push((obj.start_time, vec![column(obj, keys)])),
            }
        }
        rows.into_iter().map(|(_, mut cols)| {
            cols.sort();
            cols
        }).collect()
    }

    #[test]
    fn keeps_jumptrills() {
        let trill: Vec<&[usize]> = (0..12).map(|i| if i % 2 == 0 { &[0, 1][..] } else { &[2, 3][..] }).collect();
        for density in [Density::None, Density::Full] {
            let (_, beatmap) = convert(&rows_map(&trill), 7, density);
            let rows = key_rows(&beatmap, 7);
            assert!(rows.windows(3).all(|w| w[0] == w[2]), "{:?}", rows);
            assert!(rows.windows(2).all(|w| w[0].iter().all(|c| !w[1].contains(c))), "{:?}", rows);
        }
    }

    #[test]
    fn keeps_chordjacks() {
        // Quads every 1/2 beat.
        let osu = rows_map(&[&[0, 1, 2, 3], &[], &[0, 1, 2, 3], &[], &[0, 1, 2, 3], &[], &[0, 1, 2, 3]]);
        let (_, beatmap) = convert(&osu, 7, Density::Full);
        let rows = key_rows(&beatmap, 7);
        assert!(rows.windows(2).all(|w| w[0] == w[1]), "{:?}", rows);
        // A full row grows by a note or so, not into a wall.
        assert!(rows[0].len() <= 5, "{:?}", rows);
    }

    #[test]
    fn rolls_short_back_and_forths() {
        // A 4K stream that comes back to the same columns, like the one in
        // Black dress: on 7K it should roll over the keys instead.
        let (_, beatmap) = convert(&stream(&[1, 2, 3, 0, 2, 1, 0, 3, 2, 1, 2, 1, 3, 2, 0, 1, 2, 0]), 7, Density::None);
        let cols = columns(&beatmap, 7);
        let back_and_forths = cols.windows(3).filter(|w| w[0] == w[2]).count();
        assert!(back_and_forths <= 1, "{:?}", cols);
    }

    /// A 4K stair, 0123 0123..., in 1/4s at 120 BPM.
    fn stair() -> String {
        let mut osu = String::from(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:4\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
        );
        for step in 0..64 {
            osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", (step % 4) * 128 + 64, step * 125));
        }
        osu
    }

    fn column(obj: &HitObject, keys: usize) -> usize {
        let x = match &obj.kind {
            HitObjectKind::Circle(c) => c.pos.x,
            HitObjectKind::Hold(h) => h.pos_x,
            _ => unreachable!(),
        };
        ((x * keys as f32 / 512.0) as usize).min(keys - 1)
    }

    /// Checks no two notes overlap or stack in a key, and returns how many
    /// jacks (1/4 apart or closer) there are.
    fn check(beatmap: &Beatmap, keys: usize) -> usize {
        let mut release = vec![f64::NEG_INFINITY; keys];
        let mut jacks = 0;
        for obj in &beatmap.hit_objects {
            let col = column(obj, keys);
            assert!(obj.start_time > release[col], "overlap in key {} at {}", col, obj.start_time);
            if obj.start_time - release[col] <= 500.0 * JACK_BEATS {
                jacks += 1;
            }
            release[col] = match &obj.kind {
                HitObjectKind::Hold(h) => obj.start_time + h.duration,
                _ => obj.start_time,
            };
        }
        jacks
    }

    fn convert(osu: &str, dst: usize, density: Density) -> (Beatmap, Beatmap) {
        let original = rosu_map::from_str::<Beatmap>(osu).unwrap();
        let mut beatmap = original.clone();
        let params = KeyConverterParams { target_keys: dst as u8, density, keep_jacks: true };
        apply(&mut beatmap, &params).unwrap();
        (original, beatmap)
    }

    #[test]
    fn expanding_keeps_every_note_and_adds_no_jacks() {
        for (src, dst) in [(4, 7), (4, 10), (7, 8), (7, 10), (4, 5), (6, 9)] {
            let (original, beatmap) = convert(&map(src), dst, Density::None);
            assert_eq!(beatmap.circle_size, dst as f32);
            assert_eq!(beatmap.hit_objects.len(), original.hit_objects.len(), "{}K -> {}K", src, dst);
            assert!(check(&beatmap, dst) <= check(&original, src), "{}K -> {}K", src, dst);
        }
    }

    #[test]
    fn expanding_uses_every_key_evenly() {
        let (_, beatmap) = convert(&map(4), 7, Density::None);
        let mut used = [0usize; 7];
        for obj in &beatmap.hit_objects {
            used[column(obj, 7)] += 1;
        }
        let even = beatmap.hit_objects.len() / 7;
        assert!(used.iter().all(|&n| n > even / 2), "{:?}", used);
    }

    #[test]
    fn stairs_keep_their_direction() {
        let (_, beatmap) = convert(&stair(), 7, Density::None);
        let cols: Vec<usize> = beatmap.hit_objects.iter().map(|o| column(o, 7)).collect();
        // Within each 0123 the target goes right too.
        let rising = cols.chunks(4).filter(|c| c.windows(2).all(|w| w[0] < w[1])).count();
        assert!(rising >= 15, "{:?}", cols);
    }

    #[test]
    fn density_adds_notes_without_many_jacks() {
        let (original, beatmap) = convert(&map(4), 10, Density::MoreDense);
        assert!(beatmap.hit_objects.len() > original.hit_objects.len());
        // Chordjacks copy their extra notes too, so their jacks grow with
        // them; the rest adds none.
        assert!(check(&beatmap, 10) as f64 <= check(&original, 4) as f64 * 1.2);
    }

    #[test]
    fn full_scales_chords_with_the_keys() {
        let (original, beatmap) = convert(&map(4), 7, Density::Full);
        let ratio = beatmap.hit_objects.len() as f64 / original.hit_objects.len() as f64;
        // 7/4 * 0.9, give or take what does not fit.
        assert!((1.4..=1.65).contains(&ratio), "{}", ratio);
        check(&beatmap, 7);
    }

    #[test]
    fn reducing_shrinks_chords_without_jacks() {
        // A 7K hand trill, like the one in Ice Angel: on 4K it should become a
        // jumptrill, not three keys jacking every row.
        let mut osu = String::from(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:7\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
        );
        for step in 0..16 {
            let cols: &[usize] = if step % 2 == 0 { &[0, 1, 2] } else { &[4, 5, 6] };
            for col in cols {
                osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", (col * 512 + 256) / 7, step * 125));
            }
        }
        let (_, beatmap) = convert(&osu, 4, Density::None);
        let rows = key_rows(&beatmap, 4);
        assert!(rows.iter().all(|r| r.len() == 2), "{:?}", rows);
        assert_eq!(check(&beatmap, 4), 0, "{:?}", rows);
    }

    #[test]
    fn reducing_stays_playable() {
        for (src, dst) in [(7, 4), (10, 7), (4, 1)] {
            let (_, beatmap) = convert(&map(src), dst, Density::MoreDense);
            assert_eq!(beatmap.circle_size, dst as f32);
            check(&beatmap, dst);
        }
    }
}
