//! The beam search over the rows: the state of a placement, the ways to
//! place each row, and the extra notes on top of a placement.

use super::*;

pub(super) const NONE: u8 = u8::MAX;

#[derive(Clone, Copy)]
pub(super) struct State {
    /// Per target key: when it is free again, and what was there.
    pub(super) release: [f64; MAX_KEYS],
    pub(super) last_src: [u8; MAX_KEYS],
    pub(super) last_is_ln: [bool; MAX_KEYS],
    /// Per source column: its last note, to keep its jacks on one key.
    pub(super) src_release: [f64; MAX_KEYS],
    pub(super) src_target: [u8; MAX_KEYS],
    /// Time and average position (0..1) of the previous row, in source and
    /// target.
    pub(super) prev: Option<(f64, f64, f64)>,
    /// The previous row when it was a single note: its source and target
    /// columns, the step that led to it when it was part of a connected run
    /// (±1, else 0), and how many notes that run has.
    pub(super) single: Option<(u8, u8, i8, u8)>,
    /// The way the last connected run of three or more went (±1).
    pub(super) last_roll: i8,
    /// The last rows, newest first.
    pub(super) history: [Past; HISTORY],
    /// Wide hands: what each hand (left, right) played last.
    pub(super) hands: [HandPast; 2],
    /// Per scratch: when it was last hit, and how many notes it had lately
    /// (each decaying over [`SCRATCH_DECAY_BEATS`]) as of then.
    pub(super) scratch_time: [f64; MAX_KEYS],
    pub(super) scratch_level: [f64; MAX_KEYS],
    /// Per scratch and the key next to it: when it was last hit, and how
    /// many notes it had lately (decaying over [`PINKY_BALANCE_BEATS`]).
    pub(super) pinky_time: [f64; MAX_KEYS],
    pub(super) pinky_level: [f64; MAX_KEYS],
}

impl State {
    pub(super) fn new() -> Self {
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
            hands: [HandPast { time: f64::NEG_INFINITY, ..Default::default() }; 2],
            scratch_time: [f64::NEG_INFINITY; MAX_KEYS],
            scratch_level: [0.0; MAX_KEYS],
            pinky_time: [f64::NEG_INFINITY; MAX_KEYS],
            pinky_level: [0.0; MAX_KEYS],
        }
    }
}

/// What was done with a row: the keys of the notes kept (as many of them,
/// picked by [`kept`], as there are keys) and the keys of the extra notes.
#[derive(Clone, Copy)]
pub(super) struct Decision {
    pub(super) keys: u32,
    pub(super) extras: [u8; MAX_EXTRAS],
    pub(super) n_extras: u8,
}

impl Decision {
    pub(super) const DROPPED: Self = Self { keys: 0, extras: [0; MAX_EXTRAS], n_extras: 0 };

    /// How many notes of the row are kept.
    pub(super) fn size(&self) -> usize {
        self.keys.count_ones() as usize
    }
}

/// The last row a hand played (its keys, as a mask of the target), the one
/// before it if linked, and which way the hand went to get there (±1, else 0).
#[derive(Clone, Copy, Default)]
pub(super) struct HandPast {
    pub(super) time: f64,
    pub(super) mask: u32,
    pub(super) prev: u32,
    pub(super) dir: i8,
}

/// A row already placed: its source and target columns as bit masks.
#[derive(Clone, Copy)]
pub(super) struct Past {
    pub(super) time: f64,
    pub(super) src: u32,
    pub(super) dst: u32,
    pub(super) decision: Decision,
}

#[derive(Clone, Copy)]
pub(super) struct Ctx<'a> {
    pub(super) notes: &'a [Note],
    pub(super) src_keys: usize,
    pub(super) dst_keys: usize,
    pub(super) keep_jacks: bool,
    pub(super) fill: Option<Fill>,
    pub(super) src_hands: [Hand; MAX_KEYS],
    pub(super) dst_hands: [Hand; MAX_KEYS],
    /// The keys of the left and right hand of the target, the middle keys
    /// aside, and whether both have 4 keys or more (see [`W_HAND_ROLL`]).
    pub(super) dst_groups: [u32; 2],
    pub(super) wide: bool,
    /// The scratches the pinky moves out to (10K2S, 14K2S DP), see [`W_SCRATCH`],
    /// and with them the keys next to them (see [`W_PINKY_BALANCE`]).
    pub(super) scratches: u32,
    pub(super) pinkies: u32,
    /// The keys each target key shares a finger with (see [`shared_fingers`]).
    pub(super) fingers: [u32; MAX_KEYS],
    /// How much a column coming back to its key is spread over the others,
    /// 0 to 1 (see [`SPREAD_FULL`]).
    pub(super) spread: f64,
    /// The middle keys either thumb plays (see [`W_THUMB_CHORD`]).
    pub(super) thumbs: u32,
    /// Whether keys going on with an anchor cost [`W_ANCHOR`]: 4K -> 7K with
    /// extra notes.
    pub(super) anchors: bool,
    /// Whether each key costs by its share where the row falls
    /// ([`KEY_SHARES`]): 4K -> 7K.
    pub(super) key_shares: bool,
}

impl Ctx<'_> {
    /// Every way to place the row, with its cost. Fewer notes are only tried
    /// when they do not all fit.
    pub(super) fn candidates(&self, state: &State, row: &Row, row_idx: usize, out: &mut Vec<(f64, Decision)>) {
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
        let choices = choices(&costs[..k], self.dst_keys);
        let mut combo = [0u8; MAX_KEYS];

        for size in (1..=share).rev() {
            let keep = kept(k, size);
            let src_pos = self.source_pos(row, &keep);
            let dropped = (share - size) as f64 * drop_cost;
            let before = out.len();
            let options: Vec<&[u8]> = keep.iter().map(|&note| choices[note].as_slice()).collect();

            each_combo(&options, &self.fingers, &mut combo[..size], &mut |combo| {
                let mut cost = dropped;
                for (&note, &col) in keep.iter().zip(combo) {
                    cost += costs[note][col as usize];
                }

                let dst_pos = combo.iter().map(|&c| pos(c as usize, self.dst_keys)).sum::<f64>() / size as f64;
                cost += self.flow_cost(state, row, src_pos, dst_pos, self.single(row, &keep, combo));

                let (extras, n_extras, extra_cost) = self.pick_extras(state, row, combo, &extra_costs);
                let keys = combo.iter().fold(0u32, |m, &c| m | 1 << c);
                let decision = Decision { keys, extras, n_extras };
                let mask = self.mask_of(&decision);
                let shaped = self.mirror_cost(state, row, mask) + self.hand_cost(state, row, mask);

                out.push((cost + extra_cost + shaped + tie_break(row_idx, rank(keys)), decision));
            });

            if out.len() > before && !reducing {
                return;
            }
        }

        // Every key is held: the whole row is dropped.
        out.push((share as f64 * drop_cost, Decision::DROPPED));
    }

    /// The target keys of a decision.
    pub(super) fn mask_of(&self, decision: &Decision) -> u32 {
        decision.extras[..decision.n_extras as usize].iter().fold(decision.keys, |m, &col| m | 1 << col)
    }

    /// Whether `decision`, taken from a row with the same source columns, can
    /// place this row too: every key it uses is free, and in a trill none
    /// jacks the row before. A trill's first two rows may be further apart
    /// than the rest (a 1/2 then 1/4s), so they can share a key (the middle
    /// one) that copied on at 1/4 would be a jack on every row.
    pub(super) fn fits(&self, state: &State, row: &Row, decision: &Decision) -> bool {
        let size = decision.size();
        if size == 0 || size > row.notes.len() {
            return false;
        }
        let prev = &state.history[0];
        if row.copy == 2 && row.time - prev.time <= row.beat_len * JACK_BEATS {
            let prev_fingers = keys_of(prev.dst).fold(0u32, |m, k| m | self.fingers[k as usize]);
            if self.mask_of(decision) & prev_fingers != 0 {
                return false;
            }
        }
        let notes = &self.notes[row.notes.clone()];
        kept(notes.len(), size)
            .iter()
            .zip(keys_of(decision.keys))
            .all(|(&i, col)| self.free(state, col as usize, row, notes[i].src))
            && decision.extras[..decision.n_extras as usize].iter().all(|&col| row.time > state.release[col as usize])
    }

    pub(super) fn commit(&self, state: &mut State, row: &Row, decision: &Decision) {
        let notes = &self.notes[row.notes.clone()];
        let size = decision.size();
        let keep = kept(notes.len(), size);

        if size > 0 {
            let combo: Vec<u8> = keys_of(decision.keys).collect();
            for (&i, &col) in keep.iter().zip(&combo) {
                let (note, col) = (&notes[i], col as usize);
                state.release[col] = note.end;
                state.last_src[col] = note.src;
                state.last_is_ln[col] = note.end > row.time;
                state.src_release[note.src as usize] = note.end;
                state.src_target[note.src as usize] = col as u8;
            }

            let dst_pos = combo.iter().map(|&c| pos(c as usize, self.dst_keys)).sum::<f64>() / size as f64;
            state.prev = Some((row.time, self.source_pos(row, &keep), dst_pos));

            state.single = self.single(row, &keep, &combo).map(|(src, dst)| {
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

        let src_mask = notes.iter().fold(0u32, |m, n| m | 1 << n.src);
        let dst_mask = self.mask_of(decision);
        state.history.rotate_right(1);
        state.history[0] = Past { time: row.time, src: src_mask, dst: dst_mask, decision: *decision };

        if self.wide {
            let link = patterns::link_ms(row.beat_len);
            for (past, &group) in state.hands.iter_mut().zip(&self.dst_groups) {
                let m = dst_mask & group;
                if m == 0 {
                    continue;
                }
                let linked = past.mask != 0 && row.time - past.time <= link;
                let step = if linked { center(m) - center(past.mask) } else { 0.0 };
                let dir = if step.abs() > 0.25 { step.signum() as i8 } else { 0 };
                *past = HandPast { time: row.time, mask: m, prev: if linked { past.mask } else { 0 }, dir };
            }
        }

        for &col in &decision.extras[..decision.n_extras as usize] {
            let end = self.template(row, decision, col).map_or(row.time, |n| self.notes[n].end);
            state.release[col as usize] = end;
            state.last_src[col as usize] = NONE;
            state.last_is_ln[col as usize] = end > row.time;
        }

        for col in keys_of(dst_mask & self.pinkies) {
            let col = col as usize;
            let decay = (-(row.time - state.pinky_time[col]) / (PINKY_BALANCE_BEATS * row.beat_len)).exp();
            state.pinky_level[col] = state.pinky_level[col] * decay + 1.0;
            state.pinky_time[col] = row.time;
        }
        for col in keys_of(dst_mask & self.scratches) {
            let col = col as usize;
            let decay = (-(row.time - state.scratch_time[col]) / (SCRATCH_DECAY_BEATS * row.beat_len)).exp();
            state.scratch_level[col] = state.scratch_level[col] * decay + 1.0;
            state.scratch_time[col] = row.time;
        }

        // The other keys of a finger wait for it as its own key does, so a
        // finger going to another of its keys costs what a new jack does.
        // After a long note the finger is up already: its column may go on to
        // the other key as it would on its own (a 7K FLN column leaving the
        // scratch for the key next to it, 1/4 beat after its release).
        for col in keys_of(dst_mask) {
            let col = col as usize;
            let src = if state.last_is_ln[col] { state.last_src[col] } else { NONE };
            for key in keys_of(self.fingers[col] & !dst_mask) {
                let key = key as usize;
                state.release[key] = state.release[key].max(state.release[col]);
                state.last_src[key] = src;
                state.last_is_ln[key] = false;
            }
        }
    }

    /// The note an extra note in `col` copies: the kept note closest to it.
    pub(super) fn template(&self, row: &Row, decision: &Decision, col: u8) -> Option<usize> {
        let size = decision.size();
        if size == 0 {
            return None;
        }
        kept(row.notes.len(), size)
            .iter()
            .zip(keys_of(decision.keys))
            .min_by_key(|(_, c)| c.abs_diff(col))
            .map(|(&i, _)| row.notes.start + i)
    }
}

/// Deterministic noise in `[0, TIE_BREAK)`, so the same settings always give
/// the same map.
pub(super) fn tie_break(row: usize, choice: usize) -> f64 {
    let mut x = (row as u64) << 32 | choice as u64;
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^= x >> 31;
    (x >> 11) as f64 / (1u64 << 53) as f64 * TIE_BREAK
}

/// Picks what to do with each row.
pub(super) fn search(ctx: &Ctx, rows: &[Row]) -> Vec<Decision> {
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

/// Adds the extra notes to a placement made without them (`path`), row by row,
/// without moving its notes: each row gets what [`Ctx::pick_extras`] gives
/// it, on keys with no note, on them or on another key of their finger, from
/// a jack of the map's ([`SOURCE_JACK_BEATS`]) before the extra note to one
/// after it: it does not turn a jack of two into three, nor shield a long
/// note, as 15 to 24% of a 10K map's extra notes did with a jack's gap of
/// the converter's own (3/8 beat). A repeated pattern copies the extra notes
/// of the row it repeats.
pub(super) fn add_extras(ctx: &Ctx, rows: &[Row], path: &[Decision]) -> Vec<Decision> {
    // Per row, when each key next has a note of the placement, after it.
    let mut next_use = vec![[f64::INFINITY; MAX_KEYS]; rows.len()];
    let mut upcoming = [f64::INFINITY; MAX_KEYS];
    for (ri, (row, decision)) in rows.iter().zip(path).enumerate().rev() {
        next_use[ri] = upcoming;
        for col in keys_of(decision.keys) {
            upcoming[col as usize] = row.time;
        }
    }

    let mut state = State::new();
    let mut out = Vec::with_capacity(rows.len());
    for (ri, (row, placed)) in rows.iter().zip(path).enumerate() {
        let jack_gap = row.beat_len * SOURCE_JACK_BEATS;
        // Whether an extra note in `col` of `decision` leaves the placement's
        // next notes alone.
        let clear = |decision: &Decision, col: u8| {
            let end = ctx.template(row, decision, col).map_or(row.time, |n| ctx.notes[n].end);
            keys_of(ctx.fingers[col as usize]).all(|k| next_use[ri][k as usize] - end > jack_gap)
        };
        let copied = (row.copy > 0)
            .then(|| state.history[row.copy as usize - 1].decision)
            .filter(|past| {
                past.keys == placed.keys
                    && ctx.fits(&state, row, past)
                    && past.extras[..past.n_extras as usize].iter().all(|&col| clear(past, col))
            });
        let decision = match copied {
            Some(past) => past,
            None if placed.keys == 0 => *placed,
            None => {
                let mut costs = ctx.extra_costs(&state, row);
                for col in 0..ctx.dst_keys as u8 {
                    // `release` holds the last note of the key's finger too.
                    if !clear(placed, col) || row.time - state.release[col as usize] <= jack_gap {
                        costs[col as usize] = f64::INFINITY;
                    }
                }
                let combo: Vec<u8> = keys_of(placed.keys).collect();
                let (extras, n_extras, _) = ctx.pick_extras(&state, row, &combo, &costs);
                Decision { keys: placed.keys, extras, n_extras }
            }
        };
        ctx.commit(&mut state, row, &decision);
        out.push(decision);
    }
    out
}
