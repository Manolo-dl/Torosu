//! The rows of the source (the notes that start at the same time) and what
//! the conversion reads in them: their place in the beat, the patterns that
//! repeat them (jacks, trills) and how many extra notes each gets.

use super::*;

#[derive(Clone, Copy)]
pub(super) struct Fill {
    /// Most extra notes per row; with `scaled`, each row's quota instead.
    pub(super) max: usize,
    /// Rows weaker than this (see [`beat_strength`]) get none.
    pub(super) min_strength: f64,
    /// Min gap in beats from the previous note in that key.
    pub(super) gap_beats: f64,
    /// Chords grow with the key count (Full).
    pub(super) scaled: bool,
}

pub(super) fn fill_settings(density: Density) -> Option<Fill> {
    match density {
        Density::None => None,
        Density::Dense => Some(Fill { max: 1, min_strength: 0.7, gap_beats: 0.5, scaled: false }),
        Density::MoreDense => Some(Fill { max: 2, min_strength: 0.0, gap_beats: JACK_BEATS, scaled: false }),
        Density::Full => Some(Fill { max: MAX_EXTRAS, min_strength: 0.0, gap_beats: 0.0, scaled: true }),
    }
}

pub(super) struct Note {
    /// Index in `beatmap.hit_objects`.
    pub(super) idx: usize,
    pub(super) end: f64,
    pub(super) src: u8,
    /// The notes of its source column that would follow it onto its key
    /// (see [`look_ahead`]), each decaying over [`SCRATCH_DECAY_BEATS`]: what
    /// a scratch taking it would get too.
    pub(super) ahead: f64,
}

pub(super) struct Row {
    pub(super) time: f64,
    /// Indices into the notes, sorted by source column.
    pub(super) notes: std::ops::Range<usize>,
    pub(super) beat_len: f64,
    /// How much an extra note fits here: 1 on the beat, less off it.
    pub(super) strength: f64,
    /// Part of a trill or jumptrill of the source: keys coming back are fine.
    pub(super) trill: bool,
    /// Repeats the row this many rows back (1: chordjack, 2: trill), so its
    /// target row is copied too. 0 when it does not.
    pub(super) copy: u8,
    /// Part of a repeated pattern: its extra notes stay on the same hand as
    /// the chord, so each row of a jumptrill sits on one hand.
    pub(super) grouped: bool,
    /// Full: how many extra notes this row should get.
    pub(super) quota: u8,
    /// How many of its notes fit on the target keys: all of them, unless
    /// going down in keys.
    pub(super) share: u8,
    /// Two of its notes on next columns of the same hand (a pill).
    pub(super) pill: bool,
    /// Two of its notes on next columns, in any hand.
    pub(super) adjacent: bool,
    /// The pattern, density zone and beat position of the source here.
    pub(super) pattern: Pattern,
    pub(super) zone: Zone,
    pub(super) beat: u8,
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
pub(super) fn mark_patterns(rows: &mut [Row], notes: &[Note], keep_jacks: bool) {
    let cols: Vec<u32> = rows.iter().map(|row| notes[row.notes.clone()].iter().fold(0u32, |m, n| m | 1 << n.src)).collect();

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
        let max_gap = rows[i].beat_len * TRILL_MAX_GAP_BEATS + ROUNDING_MS;
        let fast = rows[i].time - rows[i - 1].time <= max_gap && rows[i - 1].time - rows[i - 2].time <= max_gap;
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

/// Sets [`Note::ahead`]: for each note, the notes of its source column that
/// would have to follow it onto its key, each weighted by how soon it comes:
/// one starting within a jack of a short note (going to the other key of the
/// finger would be one; after a long note it is not, see [`Ctx::commit`]), a
/// jack of the map in the next row, or a copy of its row. A note other notes
/// come after but may go elsewhere has none. Where the target has few new
/// keys (`near`), columns keep to their keys (see [`W_STICK`]): any note of
/// its column within a beat follows it.
pub(super) fn look_ahead(rows: &[Row], notes: &mut [Note], near: bool) {
    // Per source column, the row and start of its next note and its `ahead`.
    let mut next: [Option<(usize, f64, f64)>; MAX_KEYS] = [None; MAX_KEYS];
    for (i, row) in rows.iter().enumerate().rev() {
        for note in &mut notes[row.notes.clone()] {
            let col = note.src as usize;
            note.ahead = match next[col] {
                Some((j, time, ahead))
                    if (note.end <= row.time && time - row.time <= row.beat_len * JACK_BEATS)
                        || (j == i + 1 && time - row.time <= row.beat_len * SOURCE_JACK_BEATS)
                        || (rows[j].copy > 0 && j == i + rows[j].copy as usize)
                        || (near && time - row.time <= row.beat_len + ROUNDING_MS) =>
                {
                    (-(time - row.time) / (SCRATCH_DECAY_BEATS * row.beat_len)).exp() * (1.0 + ahead)
                }
                _ => 0.0,
            };
            next[col] = Some((i, row.time, note.ahead));
        }
    }
}

/// How strong a position in the beat is: on the beat, on a half, on a
/// quarter, or anything else.
pub(super) fn beat_strength(time: f64, tp_time: f64, beat_len: f64) -> f64 {
    match patterns::beat_position(time, tp_time, beat_len) {
        0 => 1.0,
        1 => 0.7,
        2 => 0.45,
        _ => 0.25,
    }
}

/// The rows of the map: the notes that start at the same time, sorted by
/// column, each with where it falls in the beat and the pattern and density
/// zone of the source there.
pub(super) fn read_rows(beatmap: &Beatmap, src_keys: usize, src_layout: &Layout, grid: &BeatGrid) -> (Vec<Note>, Vec<Row>) {
    let source = patterns::label(beatmap, src_layout);
    let source_at = |time: f64| -> (Pattern, Zone, u8) {
        let i = source.rows.partition_point(|r| r.time < time - 0.5);
        match source.rows.get(i) {
            Some(r) if (r.time - time).abs() < 1.0 => (source.labels[i].pattern, source.zones[i], r.beat),
            _ => (Pattern::Sparse, Zone::Normal, 0),
        }
    };

    let mut notes: Vec<Note> = Vec::new();
    let mut rows: Vec<Row> = Vec::new();
    for (idx, obj) in beatmap.hit_objects.iter().enumerate() {
        let Some((x, end)) = mania::note(obj) else { continue };
        let src = mania::column(x, src_keys) as u8;
        let time = obj.start_time;

        match rows.last_mut() {
            Some(row) if (time - row.time).abs() < 1.0 => row.notes.end += 1,
            _ => {
                let (tp_time, beat_len) = grid.at(time);
                let (pattern, zone, beat) = source_at(time);
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
                    pill: false,
                    adjacent: false,
                    pattern,
                    zone,
                    beat,
                });
            }
        }
        notes.push(Note { idx, end, src, ahead: 0.0 });
    }

    let src_hands = hands(src_layout);
    for row in &mut rows {
        notes[row.notes.clone()].sort_by_key(|n| n.src);
        if row.notes.len() > 1 {
            row.strength = (row.strength + 0.3).min(1.0);
        }
        row.pill = notes[row.notes.clone()].windows(2).any(|w| {
            let (a, b) = (w[0].src as usize, w[1].src as usize);
            b == a + 1 && src_hands[a] != Hand::Either && src_hands[a] == src_hands[b]
        });
        row.adjacent = notes[row.notes.clone()].windows(2).any(|w| w[1].src == w[0].src + 1);
    }
    (notes, rows)
}

/// How many notes of each row fit on the target keys ([`Row::share`]): all
/// of them going up, a note per finger at most (`max_chord`). Going down, its
/// size times the key ratio, rounded, at least one note: hand-made maps do it
/// that consistently.
pub(super) fn set_shares(rows: &mut [Row], src_keys: usize, dst_keys: usize, max_chord: usize) {
    for row in rows.iter_mut() {
        let k = row.notes.len();
        row.share = if dst_keys < src_keys {
            let size = (k as f64 * dst_keys as f64 / src_keys as f64).round() as usize;
            size.clamp(1, k.min(max_chord)) as u8
        } else {
            k.min(max_chord) as u8
        };
    }
}

/// Full: how many extra notes each row gets ([`Row::quota`]). Its chord grows
/// with the key count: on 4K -> 7K (`measured`) by where it falls in the
/// beat, its notes and whether it is in a stream (see [`FULL_GROWTH`]),
/// elsewhere by the key ratio ([`FULL_SCALE`]). The fraction is dithered per
/// row, so the average comes out right.
pub(super) fn set_full_quotas(rows: &mut [Row], src_keys: usize, dst_keys: usize, max_chord: usize, spread: f64, measured: bool) {
    let scale = dst_keys as f64 / src_keys as f64 * FULL_SCALE;
    let near = |a: &Row, b: &Row| b.time - a.time <= b.beat_len * 0.25 + ROUNDING_MS;
    let stream: Vec<bool> =
        (0..rows.len()).map(|i| i > 0 && i + 1 < rows.len() && near(&rows[i - 1], &rows[i]) && near(&rows[i], &rows[i + 1])).collect();
    for (i, row) in rows.iter_mut().enumerate() {
        let k = row.notes.len();
        let lone = if k == 1 && !stream[i] { FULL_LONE } else { 1.0 };
        let target = if k == src_keys {
            k as f64 + FULL_ROW_GROWTH
        } else if measured {
            k as f64 + FULL_GROWTH[row.beat.min(3) as usize][k.clamp(1, 3) - 1] * lone
        } else {
            k as f64 * scale
        };
        // Any choice index no row uses, so it is independent noise. A
        // repeated pattern (a trill, a chordjack) is copied from its first
        // rows, so it takes their size all along. Where the target has few
        // new keys it gets the size its rows should have, not a draw: a 10K
        // trill on 12K (1 note x 1.08) turned into a jumptrill 8% of the
        // time. With more room the draw stays, or 4K -> 7K (a 1/4 grows by
        // half a note) would double every jack.
        let dither = tie_break(i, u32::MAX as usize) / TIE_BREAK;
        let size = if row.grouped && spread < 1.0 {
            target.round() as usize
        } else {
            (target.floor() + if dither < target.fract() { 1.0 } else { 0.0 }) as usize
        };
        row.quota = size.saturating_sub(k).min(max_chord - k.min(max_chord)).min(MAX_EXTRAS) as u8;
    }
    // The two rows a trill or jumptrill alternates get the same size, so
    // both hands play the same chords.
    for i in 1..rows.len() {
        if rows[i].trill && rows[i - 1].trill && rows[i].copy == 0 {
            rows[i].quota = rows[i - 1].quota;
        }
    }
}
