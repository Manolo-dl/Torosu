//! Converts an osu!mania map to another key count, in either direction.
//!
//! Based on osu-mania-converter (4K/7K -> 8K/10K), generalised:
//!
//! - Every note keeps its source column's relative order: a *layout* maps the
//!   source columns onto the target ones, order-preserving, so stairs stay
//!   stairs and trills stay trills. Each source column also stays on its hand
//!   (left, right, or the middle key on odd key counts).
//! - The map is cut into groups (one per beat by default) and each group picks
//!   a layout. Changing layouts between groups is what spreads the notes over
//!   the extra keys; going down, a layout folds several columns into one.
//! - The layouts are chosen with a beam search over a cost (new jacks, broken
//!   jacks, moving a finger that holds an LN, dropped notes...), instead of the
//!   original greedy choice with one beat of lookahead and its 15 hand-written
//!   rules for the 7K middle key.
//! - A note that cannot be placed (it would land inside an LN, or on another
//!   note) is dropped, so the output is always a valid map.

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

const BEAM_WIDTH: usize = 12;

/// Notes in the same column closer than this (in beats) are a jack, as in the
/// original: a 1/4 with some leniency. 1/2 apart is just a repeated key.
const JACK_BEATS: f64 = 0.375;

/// Shorter LNs are played like taps and do not lock the finger.
const LOCK_MIN_MS: f64 = 100.0;

/// Leniency for notes a couple of ms off the beat when grouping.
const GRID_LENIENCY_MS: f64 = 2.0;

const COST_DROP: f64 = 150.0;
const COST_NEW_JACK: f64 = 100.0;
const COST_LN_FINGER: f64 = 30.0;
const COST_BROKEN_JACK: f64 = 20.0;
/// Keeping the same layout costs more than moving one column, so it changes
/// whenever it can, like the original alternating shifts.
const COST_STAY: f64 = 3.0;
const COST_MOVE: f64 = 1.0;
const COST_IMBALANCE: f64 = 2.0;
/// Upper bound of the noise that breaks ties between equal layouts, so the
/// patterns do not always move the same way.
const TIE_BREAK: f64 = 0.5;

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

/// A source column stays on its hand; the middle key goes either way.
fn same_hand(src: Side, dst: Side) -> bool {
    src == Side::Middle || dst == Side::Middle || src == dst
}

/// Every order-preserving map from `src` columns to `dst` columns that keeps
/// each column on its hand: injective when expanding, folding when reducing.
fn layouts(src: usize, dst: usize) -> Vec<Vec<u8>> {
    fn rec(i: usize, min: usize, src: usize, dst: usize, cur: &mut Vec<u8>, out: &mut Vec<Vec<u8>>) {
        if i == src {
            out.push(cur.clone());
            return;
        }
        let injective = dst >= src;
        for d in min..dst {
            // Leave room for the remaining columns.
            if injective && dst - d < src - i {
                break;
            }
            if !same_hand(side(i, src), side(d, dst)) {
                continue;
            }
            cur.push(d as u8);
            rec(i + 1, if injective { d + 1 } else { d }, src, dst, cur, out);
            cur.pop();
        }
    }

    let mut out = Vec::new();
    rec(0, 0, src, dst, &mut Vec::with_capacity(src), &mut out);
    out
}

struct Note {
    /// Index in `beatmap.hit_objects`.
    idx: usize,
    time: f64,
    end: f64,
    src: usize,
    beat_len: f64,
}

impl Note {
    fn jack_gap(&self) -> f64 {
        self.beat_len * JACK_BEATS
    }

    fn holds(&self) -> bool {
        self.end - self.time >= LOCK_MIN_MS
    }
}

const NONE: u8 = u8::MAX;

#[derive(Clone)]
struct State {
    /// Per target column: when it is free again (end of its last note).
    release: Vec<f64>,
    last_src: Vec<u8>,
    last_is_ln: Vec<bool>,
    holding: Vec<bool>,
    /// Per source column: its last note, to keep its jacks together.
    src_release: Vec<f64>,
    src_target: Vec<u8>,
}

impl State {
    fn new(src: usize, dst: usize) -> Self {
        Self {
            release: vec![f64::NEG_INFINITY; dst],
            last_src: vec![NONE; dst],
            last_is_ln: vec![false; dst],
            holding: vec![false; dst],
            src_release: vec![f64::NEG_INFINITY; src],
            src_target: vec![NONE; src],
        }
    }
}

#[derive(Default)]
struct Placement {
    /// Target column of each note of the group, `None` when dropped.
    targets: Vec<Option<u8>>,
    new_jacks: usize,
}

/// Places a group with `layout`, updating `state`, and returns its cost.
fn place(
    state: &mut State,
    group: &[Note],
    layout: &[u8],
    prev: Option<&[u8]>,
    keep_jacks: bool,
    dst: usize,
    mut out: Option<&mut Placement>,
) -> f64 {
    let start = group[0].time;
    let mut cost = 0.0;

    // A finger holding an LN cannot move to another key.
    for d in 0..dst {
        if state.holding[d] && state.release[d] > start {
            let src = state.last_src[d] as usize;
            if layout[src] as usize != d {
                cost += COST_LN_FINGER;
            }
        }
    }

    match prev {
        Some(prev) if prev == layout => cost += COST_STAY,
        Some(prev) => {
            let moved: u32 = prev.iter().zip(layout).map(|(&a, &b)| a.abs_diff(b) as u32).sum();
            cost += COST_MOVE * moved as f64;
        }
        None => {}
    }

    let (mut left, mut right) = (0i32, 0i32);

    for note in group {
        let d = layout[note.src] as usize;
        let src = note.src as u8;

        // A chained LN tail of the same source column is fine, anything else
        // starting where (or before) the column is released is not.
        let chained = note.time == state.release[d] && state.last_src[d] == src && state.last_is_ln[d];
        if note.time <= state.release[d] && !chained {
            cost += COST_DROP;
            if let Some(out) = out.as_deref_mut() {
                out.targets.push(None);
            }
            continue;
        }

        if state.last_src[d] != src && note.time - state.release[d] <= note.jack_gap() {
            cost += COST_NEW_JACK;
            if let Some(out) = out.as_deref_mut() {
                out.new_jacks += 1;
            }
        }

        let prev_target = state.src_target[note.src];
        if keep_jacks
            && prev_target != NONE
            && prev_target as usize != d
            && note.time - state.src_release[note.src] <= note.jack_gap()
        {
            cost += COST_BROKEN_JACK;
        }

        match side(d, dst) {
            Side::Left => left += 1,
            Side::Right => right += 1,
            Side::Middle => {}
        }

        state.release[d] = note.end;
        state.last_src[d] = src;
        state.last_is_ln[d] = note.end > note.time;
        state.holding[d] = note.holds();
        state.src_release[note.src] = note.end;
        state.src_target[note.src] = d as u8;

        if let Some(out) = out.as_deref_mut() {
            out.targets.push(Some(d as u8));
        }
    }

    cost + COST_IMBALANCE * (left - right).abs() as f64
}

/// Deterministic noise in `[0, TIE_BREAK)`, so the same settings always give
/// the same map (and the same file name stays valid).
fn tie_break(group: usize, layout: usize) -> f64 {
    let mut x = (group as u64) << 32 | layout as u64;
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^= x >> 31;
    (x >> 11) as f64 / (1u64 << 53) as f64 * TIE_BREAK
}

struct Beam {
    cost: f64,
    state: State,
    layout: Option<usize>,
    node: u32,
}

/// Picks a layout per group, returns the index of each one.
fn search(groups: &[&[Note]], layouts: &[Vec<u8>], src: usize, dst: usize, keep_jacks: bool) -> Vec<usize> {
    // Back pointers of every beam entry, to rebuild the best path at the end.
    let mut nodes: Vec<(u32, u16)> = Vec::new();
    let mut beam = vec![Beam { cost: 0.0, state: State::new(src, dst), layout: None, node: u32::MAX }];
    let mut scratch = State::new(src, dst);
    let mut candidates: Vec<(f64, usize, usize)> = Vec::new();

    for (gi, group) in groups.iter().enumerate() {
        candidates.clear();

        for (bi, entry) in beam.iter().enumerate() {
            let prev = entry.layout.map(|l| layouts[l].as_slice());
            for (li, layout) in layouts.iter().enumerate() {
                scratch.clone_from(&entry.state);
                let cost = place(&mut scratch, group, layout, prev, keep_jacks, dst, None);
                candidates.push((entry.cost + cost + tie_break(gi, li), bi, li));
            }
        }

        let keep = BEAM_WIDTH.min(candidates.len());
        if candidates.len() > keep {
            candidates.select_nth_unstable_by(keep - 1, |a, b| a.0.total_cmp(&b.0));
            candidates.truncate(keep);
        }

        beam = candidates
            .iter()
            .map(|&(cost, bi, li)| {
                let parent = &beam[bi];
                let mut state = parent.state.clone();
                let prev = parent.layout.map(|l| layouts[l].as_slice());
                place(&mut state, group, &layouts[li], prev, keep_jacks, dst, None);
                nodes.push((parent.node, li as u16));
                Beam { cost, state, layout: Some(li), node: (nodes.len() - 1) as u32 }
            })
            .collect();
    }

    let best = beam.iter().min_by(|a, b| a.cost.total_cmp(&b.cost)).map_or(u32::MAX, |b| b.node);

    let mut path = Vec::with_capacity(groups.len());
    let mut node = best;
    while node != u32::MAX {
        let (parent, layout) = nodes[node as usize];
        path.push(layout as usize);
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

/// Extra notes copied next to the existing ones, into the keys the layout
/// left empty, as `(max per group, min gap in beats)`.
fn fill_settings(density: Density) -> Option<(usize, f64)> {
    match density {
        Density::None => None,
        Density::Dense => Some((1, 1.0)),
        Density::MoreDense => Some((2, JACK_BEATS)),
    }
}

pub fn apply(beatmap: &mut Beatmap, params: &KeyConverterParams) -> Result<()> {
    if beatmap.mode != GameMode::Mania {
        bail!("the key converter only works on osu!mania maps");
    }

    let src = beatmap.circle_size.round().clamp(1.0, MAX_KEYS as f32) as usize;
    let dst = params.target_keys as usize;
    if !(1..=MAX_KEYS).contains(&dst) {
        bail!("the key converter supports 1K to {}K, not {}K", MAX_KEYS, dst);
    }
    if src == dst {
        log::warn!("key converter: the map is already {}K, left as is", dst);
        return Ok(());
    }

    let layouts = layouts(src, dst);
    if layouts.is_empty() {
        bail!("no way to map {}K onto {}K", src, dst);
    }

    // The beat grid. Gimmick maps use fake BPMs (0.001, 100000...), which are
    // skipped: they would cut the map into groups of a few ms.
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

    let beats = params.beats_per_pattern.max(1) as f64;

    let mut notes = Vec::new();
    let mut keys = Vec::new();
    for (idx, obj) in beatmap.hit_objects.iter().enumerate() {
        let (x, end) = match &obj.kind {
            HitObjectKind::Circle(circle) => (circle.pos.x, obj.start_time),
            HitObjectKind::Hold(hold) => (hold.pos_x, obj.start_time + hold.duration),
            _ => continue,
        };
        let time = obj.start_time;
        let tp = grid.partition_point(|&(t, _)| t <= time + GRID_LENIENCY_MS).saturating_sub(1);
        let (tp_time, beat_len) = grid[tp];

        notes.push(Note {
            idx,
            time,
            end,
            src: ((x * src as f32 / 512.0).floor().max(0.0) as usize).min(src - 1),
            beat_len,
        });
        keys.push((tp, ((time - tp_time + GRID_LENIENCY_MS) / (beat_len * beats)).floor() as i64));
    }

    // Groups are runs of notes in the same stretch of the grid.
    let mut groups: Vec<&[Note]> = Vec::new();
    let mut start = 0;
    for i in 1..=notes.len() {
        if i == notes.len() || keys[i] != keys[start] {
            if i > start {
                groups.push(&notes[start..i]);
            }
            start = i;
        }
    }

    let path = search(&groups, &layouts, src, dst, params.keep_jacks);

    // Replays the chosen layouts to know where each note goes.
    let mut state = State::new(src, dst);
    let mut placed: Vec<Option<u8>> = vec![None; beatmap.hit_objects.len()];
    let mut new_jacks = 0;
    let mut prev: Option<usize> = None;

    for (group, &li) in groups.iter().zip(&path) {
        let mut out = Placement::default();
        place(&mut state, group, &layouts[li], prev.map(|l| layouts[l].as_slice()), params.keep_jacks, dst, Some(&mut out));
        for (note, target) in group.iter().zip(out.targets) {
            placed[note.idx] = target;
        }
        new_jacks += out.new_jacks;
        prev = Some(li);
    }

    let mut extra: Vec<HitObject> = Vec::new();

    if let Some((max_per_group, gap_beats)) = fill_settings(params.density).filter(|_| dst > src) {
        // Per target column, the (start, end) of its notes, sorted.
        let mut occupied: Vec<Vec<(f64, f64)>> = vec![Vec::new(); dst];
        for note in &notes {
            if let Some(d) = placed[note.idx] {
                occupied[d as usize].push((note.time, note.end));
            }
        }

        let free = |column: &[(f64, f64)], start: f64, end: f64, gap: f64| {
            let i = column.partition_point(|&(s, _)| s < start);
            let after_prev = i == 0 || column[i - 1].1 + gap < start;
            let before_next = i == column.len() || end + gap < column[i].0;
            after_prev && before_next
        };

        for group in &groups {
            let used: Vec<usize> = group.iter().filter_map(|n| placed[n.idx]).map(usize::from).collect();
            let mut taken = used.clone();
            let mut made = 0;

            for note in group.iter() {
                if made >= max_per_group {
                    break;
                }
                let Some(d) = placed[note.idx].map(usize::from) else { continue };

                let candidates = [d.checked_sub(1), Some(d + 1).filter(|&c| c < dst)];
                for c in candidates.into_iter().flatten() {
                    if taken.contains(&c) || side(c, dst) != side(d, dst) {
                        continue;
                    }
                    if !free(&occupied[c], note.time, note.end, note.beat_len * gap_beats) {
                        continue;
                    }

                    let mut copy = beatmap.hit_objects[note.idx].clone();
                    set_x(&mut copy, column_x(c, dst));
                    extra.push(copy);

                    let at = occupied[c].partition_point(|&(s, _)| s < note.time);
                    occupied[c].insert(at, (note.time, note.end));
                    taken.push(c);
                    made += 1;
                    break;
                }
            }
        }
    }

    let total = beatmap.hit_objects.len();
    let mut objects: Vec<HitObject> = Vec::with_capacity(total + extra.len());
    for (mut obj, target) in std::mem::take(&mut beatmap.hit_objects).into_iter().zip(&placed) {
        match (&obj.kind, target) {
            (HitObjectKind::Circle(_) | HitObjectKind::Hold(_), Some(d)) => {
                set_x(&mut obj, column_x(*d as usize, dst));
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
    beatmap.circle_size = dst as f32;

    log::debug!(
        "key converter: {}K -> {}K, {} group(s), {} layout(s), {} dropped, {} added, {} new jack(s)",
        src,
        dst,
        groups.len(),
        layouts.len(),
        dropped,
        added,
        new_jacks
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_keep_order_and_hands() {
        // Two keys per hand, each into 5: like the original 4K -> 10K pairs.
        assert_eq!(layouts(4, 10).len(), 100);
        // 7K -> 8K: the skipped key decides where the middle one goes.
        assert_eq!(layouts(7, 8).len(), 8);
        assert!(layouts(7, 8).iter().all(|l| l.windows(2).all(|w| w[0] < w[1])));
        // Going down folds columns, still in order and on their hand.
        for layout in layouts(7, 4) {
            assert!(layout.windows(2).all(|w| w[0] <= w[1]));
            assert!(layout[..3].iter().all(|&d| d < 2));
            assert!(layout[4..].iter().all(|&d| d >= 2));
        }
    }

    /// A 4K map at 120 BPM with streams, jacks, chords and LNs, generated
    /// deterministically.
    fn map(keys: usize) -> String {
        let mut osu = format!(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:{}\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
            keys
        );
        let mut seed = 7u64;
        let mut busy = vec![0i64; keys];
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

    fn column(obj: &HitObject, keys: usize) -> usize {
        let x = match &obj.kind {
            HitObjectKind::Circle(c) => c.pos.x,
            HitObjectKind::Hold(h) => h.pos_x,
            _ => unreachable!(),
        };
        ((x * keys as f32 / 512.0) as usize).min(keys - 1)
    }

    /// No two notes overlap or stack in a column.
    fn assert_playable(beatmap: &Beatmap, keys: usize) {
        let mut release = vec![f64::NEG_INFINITY; keys];
        for obj in &beatmap.hit_objects {
            let col = column(obj, keys);
            assert!(obj.start_time > release[col], "overlap in column {} at {}", col, obj.start_time);
            release[col] = match &obj.kind {
                HitObjectKind::Hold(h) => obj.start_time + h.duration,
                _ => obj.start_time,
            };
        }
    }

    fn convert(src: usize, dst: usize, density: Density) -> (usize, Beatmap) {
        let mut beatmap = rosu_map::from_str::<Beatmap>(&map(src)).unwrap();
        let before = beatmap.hit_objects.len();
        let params = KeyConverterParams { target_keys: dst as u8, density, beats_per_pattern: 1, keep_jacks: true };
        apply(&mut beatmap, &params).unwrap();
        (before, beatmap)
    }

    #[test]
    fn expanding_keeps_every_note() {
        for (src, dst) in [(4, 7), (4, 10), (7, 8), (7, 10), (4, 5), (6, 9)] {
            let (before, beatmap) = convert(src, dst, Density::None);
            assert_eq!(beatmap.circle_size, dst as f32);
            assert_eq!(beatmap.hit_objects.len(), before, "{}K -> {}K", src, dst);
            assert_playable(&beatmap, dst);
        }
    }

    #[test]
    fn expanding_uses_the_new_keys() {
        let (_, beatmap) = convert(4, 10, Density::None);
        let mut used = [false; 10];
        for obj in &beatmap.hit_objects {
            used[column(obj, 10)] = true;
        }
        assert!(used.iter().all(|&u| u));
    }

    #[test]
    fn density_adds_notes() {
        let (before, beatmap) = convert(4, 10, Density::MoreDense);
        assert!(beatmap.hit_objects.len() > before);
        assert_playable(&beatmap, 10);
    }

    #[test]
    fn reducing_stays_playable() {
        for (src, dst) in [(7, 4), (10, 7), (4, 1)] {
            let (_, beatmap) = convert(src, dst, Density::None);
            assert_eq!(beatmap.circle_size, dst as f32);
            assert_playable(&beatmap, dst);
        }
    }
}
