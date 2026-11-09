//! What each way to place a row costs: the flow, the hands, the jacks and
//! repeats, the spread over the keys, the extra notes and the scratches.

use super::*;

impl Ctx<'_> {
    pub(super) fn fresh(&self, state: &State, col: usize, row: &Row) -> f64 {
        let weight = if row.notes.len() > 1 || (row.quota > 0 && self.fill.is_some()) { W_FRESH_CHORDS } else { W_FRESH };
        weight * (-(row.time - state.release[col]) / row.beat_len).exp()
    }

    pub(super) fn free(&self, state: &State, col: usize, row: &Row, src: u8) -> bool {
        let chained = row.time == state.release[col] && state.last_src[col] == src && state.last_is_ln[col];
        row.time > state.release[col] || chained
    }

    /// Whether the source alternates hands from the previous row to this one
    /// in a jumptrill, trill or bracket.
    pub(super) fn alternating(&self, state: &State, row: &Row) -> bool {
        let prev = &state.history[0];
        let alternating = row.trill || matches!(row.pattern, Pattern::Jumptrill | Pattern::Trill | Pattern::Bracket);
        if !alternating || prev.dst == 0 || row.time - prev.time > patterns::link_ms(row.beat_len) {
            return false;
        }
        let src_hands = |mask: u32| keys_of(mask).fold(0, |m, c| m | hand_bit(self.src_hands[c as usize]));
        let now = self.notes[row.notes.clone()].iter().fold(0, |m, n| m | hand_bit(self.src_hands[n.src as usize]));
        let before = src_hands(prev.src);
        now != 0 && before != 0 && now & before == 0
    }

    /// Where the source alternates hands: the hands the previous target row
    /// used, which this row should leave alone. 0 elsewhere.
    pub(super) fn busy_hands(&self, state: &State, row: &Row) -> u8 {
        if !self.alternating(state, row) {
            return 0;
        }
        let prev = state.history[0].dst;
        (0..self.dst_keys).filter(|&c| prev & 1 << c != 0).fold(0, |m, c| m | hand_bit(self.dst_hands[c]))
    }

    /// With extra notes, where the source alternates hands: a reward for a row
    /// mirroring the previous one, the middle key aside.
    pub(super) fn mirror_cost(&self, state: &State, row: &Row, mask: u32) -> f64 {
        if self.fill.is_none() || !self.alternating(state, row) {
            return 0.0;
        }
        let either = (0..self.dst_keys).filter(|&c| self.dst_hands[c] == Hand::Either).fold(0u32, |m, c| m | 1 << c);
        let mirrored = keys_of(mask & !either).fold(0u32, |m, c| m | 1 << (self.dst_keys - 1 - c as usize));
        if mirrored != 0 && mirrored == state.history[0].dst & !either {
            -W_MIRROR
        } else {
            0.0
        }
    }

    /// Wide hands: the cost of what each hand plays in a row of `mask`. A pill
    /// costs where the source has none; going on with the hand's stair, or
    /// staying on three of its keys, is rewarded.
    pub(super) fn hand_cost(&self, state: &State, row: &Row, mask: u32) -> f64 {
        if !self.wide {
            return 0.0;
        }
        let link = patterns::link_ms(row.beat_len);
        let pills = self.dst_groups.iter().map(|&g| (mask & g & ((mask & g) >> 1)).count_ones()).sum::<u32>();
        // Where the source has a pill it may stay one; hand-made 10K maps have
        // fewer pills than 7K ones, so it is not pushed to.
        let mut cost = if row.pill { 0.0 } else { W_PILL * pills as f64 };
        for (past, &group) in state.hands.iter().zip(&self.dst_groups) {
            let m = mask & group;
            if m == 0 {
                continue;
            }
            if past.mask == 0 || row.time - past.time > link {
                continue;
            }
            let step = center(m) - center(past.mask);
            if (0.5..=1.5).contains(&step.abs()) && past.dir != 0 && step.signum() as i8 == past.dir {
                cost -= W_HAND_ROLL;
            }
            let used = m | past.mask | past.prev;
            if past.prev != 0 && 32 - used.leading_zeros() - used.trailing_zeros() <= 3 {
                cost -= W_HAND_GROUP;
            }
        }
        cost
    }

    /// Cost of each source note of the row in each target key, `INFINITY`
    /// where it cannot go.
    pub(super) fn note_costs(&self, state: &State, row: &Row) -> [[f64; MAX_KEYS]; MAX_KEYS] {
        let mut costs = [[f64::INFINITY; MAX_KEYS]; MAX_KEYS];
        let jack_gap = row.beat_len * JACK_BEATS;
        let busy = self.busy_hands(state, row);
        // With extra notes, in jumptrills the middle key is an accent on the
        // beat, not every other row.
        let accent_only = self.fill.is_some() && row.beat != 0 && self.alternating(state, row);

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
                if !same_hand(self.src_hands[src], self.dst_hands[col]) {
                    cost += W_CROSS;
                }
                if hand_bit(self.dst_hands[col]) & busy != 0 || (accent_only && self.dst_hands[col] == Hand::Either) {
                    cost += W_HAND_TWICE;
                }
                let keeps_jack = source_jack && state.src_target[src] as usize == col;

                if source_jack && self.keep_jacks && !keeps_jack {
                    cost += COST_BROKEN_JACK;
                }
                let own = state.last_src[col] == note.src;
                if !keeps_jack {
                    cost += self.repeat_cost(state, row, src, col);
                    if state.last_src[col] != note.src && row.time - state.release[col] <= jack_gap {
                        cost += COST_NEW_JACK;
                    }
                    // A column coming back to its key is spread only where
                    // the target has room for it (see `spread`).
                    cost += self.fresh(state, col, row) * if own { self.spread } else { 1.0 };
                }

                if state.src_target[src] != NONE && state.src_target[src] as usize != col {
                    cost += W_SWITCH;
                    if row.time - state.src_release[src] <= row.beat_len + ROUNDING_MS {
                        cost += W_STICK * (1.0 - self.spread);
                    }
                }
                let natural = pos(src, self.src_keys) * self.dst_keys as f64 - 0.5;
                let stray = ((col as f64 - natural).abs() - FREE_DEVIATION).max(0.0);
                cost += W_DEVIATION * stray * stray;
                if self.scratches & 1 << col != 0 {
                    cost += self.scratch_cost(state, row, col, note.ahead);
                }
                // The pinky's balance moves a column when it starts, not one
                // coming back to its key, where the target keeps those.
                cost += self.pinky_cost(state, row, col) * if own { self.spread } else { 1.0 };

                costs[i][col] = cost;
            }
        }

        costs
    }

    /// The cost of a note on a scratch here: a reward for an accent on the
    /// beat, less the more the scratch was used lately and would be next
    /// (`ahead`, see [`Note::ahead`]).
    pub(super) fn scratch_cost(&self, state: &State, row: &Row, col: usize, ahead: f64) -> f64 {
        let decay = (-(row.time - state.scratch_time[col]) / (SCRATCH_DECAY_BEATS * row.beat_len)).exp();
        W_SCRATCH_LEVEL * (state.scratch_level[col] * decay + ahead) - W_SCRATCH * SCRATCH_BEAT[row.beat.min(3) as usize]
    }

    /// On a scratch or the key next to it: how many notes more than the other
    /// it had lately, times [`W_PINKY_BALANCE`] (negative when it had fewer).
    pub(super) fn pinky_cost(&self, state: &State, row: &Row, col: usize) -> f64 {
        if self.pinkies & 1 << col == 0 {
            return 0.0;
        }
        let level = |k: usize| {
            state.pinky_level[k] * (-(row.time - state.pinky_time[k]) / (PINKY_BALANCE_BEATS * row.beat_len)).exp()
        };
        let others = keys_of(self.fingers[col] & !(1 << col)).map(|k| level(k as usize)).sum::<f64>();
        W_PINKY_BALANCE * (level(col) - others).max(-PINKY_BALANCE_CAP)
    }

    /// Cost of a key coming back 2 or 3 notes later (a back-and-forth), and
    /// of breaking a real trill. Only between single notes: in jumpstreams a
    /// chord sharing a key with the one 2 rows back is what holds them
    /// together (tyrcs' Lolit speed: 82% of the rows).
    pub(super) fn repeat_cost(&self, state: &State, row: &Row, src: usize, col: usize) -> f64 {
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
                // The source's own back-and-forths are spread only where the
                // target has room for them (see `spread`).
                cost += weight * if src_repeats { self.spread } else { 1.0 };
            }
        }
        cost
    }

    /// Cost of a note on the middle key in a row of `size` notes.
    pub(super) fn thumb_cost(&self, size: usize) -> f64 {
        W_THUMB_CHORD * size.saturating_sub(2) as f64
    }

    /// Cost of an extra note in each key (negative: a reward), `INFINITY`
    /// where it does not fit.
    pub(super) fn extra_costs(&self, state: &State, row: &Row) -> [f64; MAX_KEYS] {
        let mut costs = [f64::INFINITY; MAX_KEYS];
        let Some(fill) = self.fill else { return costs };
        if row.strength < fill.min_strength {
            return costs;
        }

        let gap = row.beat_len * fill.gap_beats;
        let jack_gap = row.beat_len * JACK_BEATS;
        let reward = if fill.scaled {
            FULL_REWARD
        } else if self.wide {
            EXTRA_REWARD * row.strength * ZONE_FILL[Zone::ALL.iter().position(|&z| z == row.zone).unwrap_or(2)]
        } else {
            EXTRA_REWARD * row.strength
        };

        let alternating = self.alternating(state, row);
        let busy = self.busy_hands(state, row);
        // A repeated pattern copies its extra notes with the row, so one on a
        // scratch comes back as often as the row's columns do.
        let extra_ahead =
            if row.grouped { self.notes[row.notes.clone()].iter().map(|n| n.ahead).fold(0.0, f64::max) } else { 0.0 };
        for (col, cost) in costs.iter_mut().enumerate().take(self.dst_keys) {
            let since = row.time - state.release[col];
            if since > 0.0 && since >= gap {
                let jack = if since <= jack_gap { W_EXTRA_JACK } else { 0.0 };
                // In jumptrills the middle key is an accent on the beat.
                let middle = alternating && self.dst_hands[col] == Hand::Either && row.beat != 0;
                let hand = if hand_bit(self.dst_hands[col]) & busy != 0 || middle { W_HAND_TWICE } else { 0.0 };
                let scratch = if self.scratches & 1 << col != 0 { self.scratch_cost(state, row, col, extra_ahead) } else { 0.0 };
                let scratch = scratch + self.pinky_cost(state, row, col);
                *cost = self.fresh(state, col, row) + jack + hand - reward + scratch;
            }
        }
        costs
    }

    /// The best extra notes for a row placed in `combo`: the chord grows
    /// into the keys next to it, on the same hand, one note at a time. Full,
    /// and any density on wide hands, take any free key instead, so big chords
    /// stay open (`O.O.O.O`, not `OOOO...`), as in hand conversions of
    /// handstreams; on wide hands a pill costs [`W_PILL`] more.
    pub(super) fn pick_extras(&self, state: &State, row: &Row, combo: &[u8], extra_costs: &[f64; MAX_KEYS]) -> ([u8; MAX_EXTRAS], u8, f64) {
        let mut extras = [0u8; MAX_EXTRAS];
        let Some(fill) = self.fill else { return (extras, 0, 0.0) };
        let max = if fill.scaled { row.quota as usize } else { fill.max };

        // Full spreads the extras over any key, except in repeated patterns,
        // where they stay next to the chord so each phase of a jumptrill sits
        // on one hand. Where the target has few new keys and wide hands
        // (10K -> 12K), they go anywhere on the chord's hand instead, as
        // elsewhere: a 10K trill of single notes does not turn into a
        // jumptrill of pills.
        let near = self.spread < 1.0;
        let anywhere = (fill.scaled || self.wide) && (!row.grouped || (self.wide && near));
        let chord_hands = combo.iter().fold(0u8, |m, &c| m | hand_bit(self.dst_hands[c as usize]));
        // Where the target has few new keys, an extra note does not go next
        // to a note of its row unless the source has notes next to each other
        // there: a single note of a stream does not turn into two next keys,
        // in a hand or across the middle (a thumb on each middle key), which
        // 6 to 12% of the rows a 10K map's extra notes went to did.
        let apart = near && !row.adjacent;

        let mut n = 0;
        let mut total = 0.0;
        while n < max {
            // The cost the choice is made on, the key, and the cost it adds
            // (what each hand plays is costed with the whole row later).
            let mut best: Option<(f64, u8, f64)> = None;
            let placed = combo.iter().chain(&extras[..n]);
            let taken = placed.clone().fold(0u32, |m, &c| m | 1 << c);
            // Every key of the fingers already in the row (see `shared_fingers`):
            // on 10K2S a scratch and the key next to it never go together.
            let held = placed.fold(0u32, |m, &c| m | self.fingers[c as usize]);
            if anywhere {
                // Wide hands: an extra note goes on with what its hand plays.
                let hand_now = self.hand_cost(state, row, taken);
                for next in 0..self.dst_keys as u8 {
                    let mut added = extra_costs[next as usize];
                    if !added.is_finite() || held & 1 << next != 0 {
                        continue;
                    }
                    if self.thumbs & 1 << next != 0 {
                        added += self.thumb_cost(combo.len() + n + 1);
                    }
                    let hand = hand_bit(self.dst_hands[next as usize]);
                    if row.grouped && hand != 0 && chord_hands != 0 && hand & chord_hands == 0 {
                        continue;
                    }
                    if apart && (taken << 1 | taken >> 1) & 1 << next != 0 {
                        continue;
                    }
                    if n > 0 && !fill.scaled {
                        added += EXTRA_REWARD * if self.wide { 1.0 - SECOND_EXTRA_WIDE } else { 0.5 };
                    }
                    // A key its hand cannot make the best is not costed.
                    if self.wide && added - HAND_REWARD_MAX >= best.map_or(0.0, |(b, _, _)| b) {
                        continue;
                    }
                    let hand = if self.wide { self.hand_cost(state, row, taken | 1 << next) - hand_now } else { 0.0 };
                    let cost = added + hand;
                    if cost < 0.0 && best.is_none_or(|(b, _, _)| cost < b) {
                        best = Some((cost, next, added));
                    }
                }
            }
            for &col in combo.iter().chain(&extras[..n]).filter(|_| !anywhere) {
                let neighbours = [col.checked_sub(1), Some(col + 1).filter(|&c| (c as usize) < self.dst_keys)];
                for next in neighbours.into_iter().flatten() {
                    // Without Full, a second extra in the same row is worth
                    // half: rows mostly grow by one note, sometimes two.
                    let mut cost = extra_costs[next as usize] + if n > 0 && !fill.scaled { EXTRA_REWARD / 2.0 } else { 0.0 };
                    if self.thumbs & 1 << next != 0 {
                        cost += self.thumb_cost(combo.len() + n + 1);
                    }
                    if cost < 0.0
                        && held & 1 << next == 0
                        && same_hand(self.dst_hands[col as usize], self.dst_hands[next as usize])
                        && best.is_none_or(|(b, _, _)| cost < b)
                    {
                        best = Some((cost, next, cost));
                    }
                }
            }
            let Some((_, col, added)) = best else { break };
            extras[n] = col;
            total += added;
            n += 1;
        }
        (extras, n as u8, total)
    }

    /// Cost of the flow from the previous row to this one. `single` is the
    /// source and target column when both rows are single notes.
    pub(super) fn flow_cost(&self, state: &State, row: &Row, src_pos: f64, dst_pos: f64, single: Option<(u8, u8)>) -> f64 {
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
    pub(super) fn single(&self, row: &Row, keep: &[usize], combo: &[u8]) -> Option<(u8, u8)> {
        (row.notes.len() == 1 && combo.len() == 1).then(|| (self.notes[row.notes.start + keep[0]].src, combo[0]))
    }

    pub(super) fn source_pos(&self, row: &Row, keep: &[usize]) -> f64 {
        let notes = &self.notes[row.notes.clone()];
        keep.iter().map(|&i| pos(notes[i].src as usize, self.src_keys)).sum::<f64>() / keep.len() as f64
    }
}
