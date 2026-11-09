//! Scratches as lanes of their own (10K2S, see [`add_scratch_lanes`]): where
//! they go, fitted on 63 hand-made 10K2S maps (20 songs, 14 mappers) without
//! their scratches, against where their mappers hit them (see
//! `corpus_scratch_truth`), and on the other maps of the songs of some of
//! them (see `corpus_scratch_sources`). A place for one is a row of the map;
//! each gets a score from where it falls in the bar, how big a chord the map
//! has there and how the audio sounds there against the whole song, its rows
//! and its 1/2 beats with no note (see `accents`), and the best of them get a
//! scratch.

mod accents;

use super::*;
use accents::*;

pub(super) const SCRATCH_RARE_SOUND: f64 = 0.02;

pub(super) const SCRATCH_RARE_APART: f64 = 2.0;

/// The song goes in phrases of this many bars, and with the audio, not all
/// of them get scratches, as in hand-made maps, where a third to two thirds
/// of the bars have none and some parts are left to the keys, strong hits
/// and all: the phrases more than [`SCRATCH_QUIET_DB`] quieter than the
/// song's middle one (an intro, a break) get [`SCRATCH_QUIET_MOST`] at most,
/// on the map's accent sounds alone ([`ACCENT_HITSOUND`]), and the quieter
/// half of the others [`SCRATCH_SPARSE_MOST`] at most, even in a song as
/// loud all along.
/// Hand-made maps put more scratches in their louder phrases (see
/// `corpus_scratch_sounds`), though not by much. Without it, the converts
/// had a scratch in 7 to 9 bars in 10.
pub(super) const SCRATCH_PHRASE_BARS: f64 = 4.0;

pub(super) const SCRATCH_QUIET_DB: f64 = 3.0;

pub(super) const SCRATCH_QUIET_MOST: usize = 1;

pub(super) const SCRATCH_SPARSE_PHRASE: f64 = 0.5;

pub(super) const SCRATCH_SPARSE_MOST: usize = 2;

/// How many scratches a map gets: this many a bar, times its notes a bar to
/// this power. Hand-made maps with 4 notes a bar have 0.6 a bar, with 16
/// 1.5, with 32 2.3, give or take 70%: the more a map plays, the more accents
/// it marks. The notes tell it a little better than the rows: bigger chords,
/// more scratches.
pub(super) const SCRATCH_RATE: f64 = 0.26;

pub(super) const SCRATCH_RATE_POWER: f64 = 0.63;

/// The same for 7K+1, far fewer: hand-made 7K+1 maps (100 in a Songs folder)
/// have 0.04 a bar times their notes a bar to the 0.87, give or take 70%:
/// 0.45 at 16 notes a bar, 0.82 at 32 (see `corpus_scratch_7k1`).
pub(super) const SCRATCH_RATE_7K1: f64 = 0.04;

pub(super) const SCRATCH_RATE_POWER_7K1: f64 = 0.87;

/// That times this per density (none, dense, more dense, full): what most
/// mappers put, and up to half as many more. Twice as many made a 3 star map
/// a scratch every beat for bars; and where the map is busy, the most a span
/// can have ([`SCRATCH_SPANS`]) holds them back anyway.
pub(super) const SCRATCH_DENSITY: [f64; 4] = [1.0, 1.15, 1.3, 1.5];

/// A place gets a scratch only with a score above this, its hand's chances
/// counted ([`SCRATCH_FREE_AFTER`]): where its hand is about to play, only
/// for a big accent.
pub(super) const SCRATCH_MIN_SCORE: f64 = 0.0;

/// A place gets a scratch only if it scores more than this share of the
/// song's places, and with the audio, if its sound alone is stronger than
/// this share of theirs: the scratches mark the strong sounds, and where a
/// map has fewer than [`SCRATCH_RATE`] asks for, it gets fewer scratches
/// rather than some on weaker sounds. Half the scratches of hand-made maps
/// are on sounds stronger than three quarters of their song's, in a row as
/// much as alone, and a fifth on sounds weaker than its middle one (see
/// `corpus_scratch_sounds`), which the converts leave alone: without it, the
/// count went on to the quiet parts, and so did the scratches in a row.
pub(super) const SCRATCH_ACCENT_FROM: f64 = 0.5;

pub(super) const SCRATCH_SOUND_FROM: f64 = 0.5;

/// The places scoring more than this share of the song's are its strong
/// accents: there a scratch may take every note its hand has in the row,
/// [`SCRATCH_STRONG_TAKES`] at most, a long one too if the hand plays nothing
/// else while it holds (a long scratch: 7% of the hand-made ones), as mappers
/// leave the hand free on the big hits. Each note past the first that goes
/// costs [`SCRATCH_TAKE`]. Otherwise a scratch could mark 4 in 10 of the
/// strong sounds of a converted 3 star map, the others chords and long
/// notes of both hands. Only the scratches most mappers put take notes away:
/// those the density adds ([`SCRATCH_DENSITY`]) only add notes, so a denser
/// convert never has fewer.
pub(super) const SCRATCH_STRONG: f64 = 0.8;

pub(super) const SCRATCH_STRONG_TAKES: u32 = 3;

pub(super) const SCRATCH_TAKE: f64 = 0.75;

/// The best of the places that get a scratch (this share of them), both
/// hands free, get both, as hand-made maps do for their biggest accents (9%
/// of their rows with a scratch).
pub(super) const SCRATCH_BOTH: f64 = 0.15;

/// Scratches are at least this far apart (in beats): hand-made maps hardly
/// ever have two 1/4 beat apart (the hardest, 1 in 10 at most).
pub(super) const SCRATCH_GAP: f64 = 0.5;

/// The most scratches (places with one, both scratches being one) in any span
/// of so many beats, by how hard the map is: (beats, a, b) for `a + b` times
/// the log of the map's notes a second, rounded down, 2 at least (see
/// [`scratch_most`]). Mappers differ, from a scratch now and then to
/// scratches for bars on end: the calmer quarter of the hand-made maps have
/// up to 2 in any 4 beats and 3 in 8 below 4 notes a second, 5 or 6 and 7 or
/// 8 from there to 10, 4 and 6 above (see `corpus_scratch_hands`). The
/// converts are calmer where the map is easy, the scratches accents with
/// rests in between rather than a part to play: 2 and 3 at 3 notes a second,
/// 3 and 5 at 9 (a 3 star 10K map), 4 and 6 at 12. No hand-made map is
/// harder than 4 stars; the harder ones get more the harder they are, 5 and 7
/// at 19 notes a second (7.5 stars), in groups with rests in between. Without
/// it, a 3 star map had a scratch every beat for bars, and on the busy parts
/// of harder ones the scratches ran on, taking turns on both sides: 24 in a
/// row a beat apart, and over 100 with twice as many scratches.
pub(super) const SCRATCH_SPANS: [(f64, f64, f64); 2] = [(4.0, 0.6, 1.5), (8.0, 0.9, 2.2)];

/// At most this many notes in a row on a scratch, each a beat or less after
/// the last: an accent, not a lane. The longest run of a hand-made map is 3
/// notes (2 to 7); dense maps, where a hand is mostly busy, gave the other
/// scratch runs of 20 without it.
pub(super) const SCRATCH_RUN: usize = 4;

/// The hand of a scratch (the five keys next to it, its pinky's included) is
/// free around it, as in hand-made 10K2S maps. None of its keys has a note
/// in the row or holds one across it: that is a muri, the pinky moving out
/// while the other fingers press, which none of the 4550 scratches of 59
/// hand-made maps has. And its other notes keep away by how hard the map is:
/// this many ms, times the map's notes a second (the scratches aside) to the
/// minus this power, is where the closest 5% of a map's scratches have the
/// hand's next note, whatever the BPM (see `corpus_scratch_hands`). That is
/// 510 ms at 2 notes a second, a beat or two of an easy map, the hand free
/// for a note or more of the other one (two thirds of the scratches); 180 ms
/// at 8, 1/2 beat of a hard one; 110 ms at 15, where the hand is back on the
/// next row for three scratches in four.
pub(super) const SCRATCH_FREE_MS: f64 = 855.0;

pub(super) const SCRATCH_FREE_POWER: f64 = 0.75;

/// How likely a scratch is with the hand's next note starting this far after
/// it, and with its last one released this far before it, as (up to how far,
/// in [`SCRATCH_FREE_MS`] of the map, times what), against a hand free for
/// longer: the rate of scratches on the rows and 1/2 beats of the hand-made
/// maps without them (see `corpus_scratch_places`). Hardly ever under half
/// of it, and the hand comes back a little later than it leaves. A long note
/// of the hand released on the scratch, which 4 hard maps have, is not made
/// either.
pub(super) const SCRATCH_FREE_AFTER: [(f64, f64); 5] = [(0.5, 0.0), (0.7, 0.1), (0.85, 0.2), (1.0, 0.55), (1.25, 0.75)];

pub(super) const SCRATCH_FREE_BEFORE: [(f64, f64); 4] = [(0.5, 0.0), (0.7, 0.25), (0.85, 0.4), (1.0, 0.6)];

/// The key of the scratch's pinky, from its notes' ends too, as (up to how
/// far, in beats, times what): never 3/8 beat away or closer (a jack of the
/// pinky), less 1/2 beat away.
pub(super) const SCRATCH_PINKY_NEAR: [(f64, f64); 2] = [(0.38, 0.0), (0.56, 0.4)];

/// A scratch may take the note its hand has in the row instead, a short one,
/// in a chord (the other hand playing too) or when it is its pinky's: it
/// moves out to the scratch, sounds and all, with this times the chances of
/// a scratch with what the hand plays around. The big chords of the other
/// maps of hand-made maps' songs, where those put their scratches, have both
/// hands in them, and there the scratch's hand leaves the chord to the other
/// one: the accent stays, one key wider, and nothing is pressed with the
/// scratch. Below 1, so where a scratch fits as well without it the map stays
/// as it was. A hand with more notes there keeps them, the scratch going
/// elsewhere: taking them, a convert with more scratches (Full) had fewer
/// notes. Taking a single note of a stream, or with nothing on the other
/// hand, the patterns lose their shape, and the scratches go where the
/// mappers' do not (see `corpus_scratch_truth`).
pub(super) const SCRATCH_MOVE: f64 = 0.75;

/// The density's extra notes of a scratch's hand make way for it this far
/// around it (in [`SCRATCH_FREE_MS`] of the map), the hand's room in
/// hand-made maps; on its pinky's key, [`SCRATCH_PINKY_NEAR`]'s.
pub(super) const SCRATCH_FREE_EXTRAS: f64 = 1.0;

/// A place a scratch can go (see [`add_scratch_lanes`]).
pub(super) struct Place {
    pub(super) time: f64,
    pub(super) beat_len: f64,
    /// The notes of the map there: none on a 1/2 beat with no note.
    pub(super) notes: usize,
    /// The first beat of a bar, another beat, or off the 1/2s.
    pub(super) bar: bool,
    pub(super) beat: bool,
    pub(super) offbeat: bool,
    /// How long it is to the next place, in bars.
    pub(super) bars: f64,
    pub(super) loudness: Option<f32>,
    /// How strongly a sound starts there in the low, mid and high bands (see
    /// [`ACCENT_LOW`]).
    pub(super) bands: [f32; 3],
    /// Whether a note of the row has a rare sound (see [`ACCENT_HITSOUND`]).
    pub(super) hitsound: bool,
    /// Per scratch, how likely a note on it is with what its hand plays
    /// around: 0 where it cannot have one.
    pub(super) sides: Vec<f64>,
    /// Per scratch, the key of its hand whose note in the row its note would
    /// take (see [`SCRATCH_MOVE`]), as a mask: none for a note of its own.
    pub(super) takes: Vec<u32>,
    /// The same on a strong accent, where the scratch may take every note
    /// of its hand in the row (see [`SCRATCH_STRONG`]).
    pub(super) strong_sides: Vec<f64>,
    pub(super) strong_takes: Vec<u32>,
}

/// What some keys play around a time (see [`add_scratch_lanes`]).
pub(super) struct Around {
    /// The keys with a short note in the row and with a long one, when the
    /// last of those ends, and whether one holds a note across it.
    pub(super) row: u32,
    pub(super) long: u32,
    pub(super) long_end: f64,
    pub(super) held: bool,
    /// How long (in ms) since the last of their notes before the row ended,
    /// and until the first one after it starts.
    pub(super) before: f64,
    pub(super) after: f64,
}

/// How a layout's scratches are played, for [`add_scratch_lanes`].
#[derive(Clone, Copy)]
pub(super) struct ScratchRules {
    /// 10K2S: the pinky moves out to the scratch, so its hand is free around
    /// it, and the scratch may take the hand's note in the row. On 7K+1 the
    /// scratch is a key of its own, played with the hand's notes: 79% of the
    /// scratches of hand-made 7K+1 maps have a note of the left hand in their
    /// row, 81% are on a chord, but 1% only while it holds a long note.
    pub(super) free_hand: bool,
    /// How many: [`SCRATCH_RATE`] and [`SCRATCH_RATE_POWER`], or 7K+1's.
    pub(super) rate: f64,
    pub(super) rate_power: f64,
}

impl ScratchRules {
    pub(super) fn of(layout: &Layout) -> Self {
        if layout.name == "7K+1" {
            ScratchRules { free_hand: false, rate: SCRATCH_RATE_7K1, rate_power: SCRATCH_RATE_POWER_7K1 }
        } else {
            ScratchRules { free_hand: true, rate: SCRATCH_RATE, rate_power: SCRATCH_RATE_POWER }
        }
    }
}

/// Scratches as lanes of their own (see [`Layout::inner_keys`]): moves a map of
/// `inner_keys` keys onto the layout's other keys as it is, and gives the
/// scratches notes of their own on top, on the accents of the music, as
/// hand-made 10K2S maps do. Every row of the map is a place for one, scored
/// by where it falls in the bar, how big its chord is against the ones around
/// and, with the audio, by how strongly a sound starts there and how loud it
/// is against the whole song: the scratches go mostly on the beats, on the
/// map's big chords and on the sounds that stand out, more in the louder
/// parts. The best places get one, as many as hand-made maps have for how
/// much the map plays ([`SCRATCH_RATE`], more with a denser `density`) but
/// only those that score well enough ([`SCRATCH_MIN_SCORE`]) and whose sound
/// is among the song's stronger ones ([`SCRATCH_SOUND_FROM`]): where the
/// hands are busy, only the big accents, where they take the hand's notes
/// ([`SCRATCH_STRONG`]). The map's own accent sounds count too, a crash or a
/// water drop, their note moving out to the scratch ([`ACCENT_HITSOUND`]);
/// and not every phrase gets scratches, the quiet ones left to the keys but
/// for those sounds ([`SCRATCH_PHRASE_BARS`]). They are 1/2 beat apart at least, and only so many in a
/// few beats, the fewer the easier the map ([`SCRATCH_SPANS`]). Each goes on
/// the scratch whose hand is freer, taking turns when both are, both for the
/// biggest accents, and a few in a row at most ([`SCRATCH_RUN`]). A
/// scratch's hand is free: no note in the row nor held across it, a muri, and
/// its other notes away by how hard the map is ([`SCRATCH_FREE_MS`]), the key
/// of its pinky none 3/8 beat around; but the scratch may take the hand's note
/// in the row, moving it out ([`SCRATCH_MOVE`]). The density's extra notes in
/// its way make room for it. Without the scratches, the map stands as it was
/// but for the notes they took.
///
/// 7K+1 (see [`ScratchRules`]) has the same places but its own count, and its
/// scratch goes with its hand's notes: the 7K map stands as it was, every note
/// of it, the scratch only adding notes.
pub(super) fn add_scratch_lanes(
    beatmap: &mut Beatmap,
    layout: &Layout,
    inner_keys: usize,
    extras: &[bool],
    density: Density,
    accents: Option<MapAccents>,
    rules: ScratchRules,
) {
    let keys = layout.keys();
    let scratches = layout.digit_mask(Digit::Scratch);
    let inner: Vec<usize> = (0..keys).filter(|&k| scratches & 1 << k == 0).collect();
    debug_assert_eq!(inner.len(), inner_keys);

    // The sound of a note beyond its hit sound (its additions, or a sample of
    // its own), and how many of the map's notes have each: the rare ones mark
    // accents (see [`ACCENT_HITSOUND`]).
    let sound_of = |obj: &HitObject| -> Option<String> {
        let names: Vec<String> =
            obj.samples.iter().filter(|s| s.name != HitSampleInfo::HIT_NORMAL).map(|s| format!("{:?}", s.name)).collect();
        (!names.is_empty()).then(|| names.join(" "))
    };
    let mut sounds: std::collections::HashMap<String, Vec<f64>> = std::collections::HashMap::new();
    let mut counted = 0;
    for (i, obj) in beatmap.hit_objects.iter().enumerate() {
        if mania::note(obj).is_some() && !extras.get(i).copied().unwrap_or(false) {
            counted += 1;
            if let Some(sound) = sound_of(obj) {
                sounds.entry(sound).or_default().push(obj.start_time);
            }
        }
    }
    // Rare, and apart: a sound on every beat for bars is a part (a hi-hat
    // roll), not an accent.
    let beat_grid = BeatGrid::new(beatmap);
    let accent_sounds: std::collections::HashSet<String> = sounds
        .iter()
        .filter(|(_, times)| (times.len() as f64) < counted as f64 * SCRATCH_RARE_SOUND)
        .filter(|(_, times)| {
            let mut gaps: Vec<f64> = times.windows(2).map(|w| (w[1] - w[0]) / beat_grid.at(w[0]).1).collect();
            gaps.sort_by(f64::total_cmp);
            gaps.get(gaps.len() / 2).is_none_or(|&gap| gap >= SCRATCH_RARE_APART)
        })
        .map(|(sound, _)| sound.clone())
        .collect();
    let rare = |obj: &HitObject| sound_of(obj).is_some_and(|s| accent_sounds.contains(&s));
    // The notes with a rare sound, as (time, key), in time.
    let mut marked: Vec<(f64, usize)> = Vec::new();

    // Per key, the map's notes as (start, end), in time: the density's extra
    // notes make way for the scratches rather than keep them out.
    let mut lanes: Vec<Vec<(f64, f64)>> = vec![Vec::new(); keys];
    // The rows, as (time, notes).
    let mut rows: Vec<(f64, usize)> = Vec::new();
    // How many notes the map has, from when to when, the extra notes too.
    let (mut total, mut from, mut to) = (0usize, f64::INFINITY, f64::NEG_INFINITY);
    for (i, obj) in beatmap.hit_objects.iter_mut().enumerate() {
        let Some((x, end)) = mania::note(obj) else { continue };
        let key = inner[mania::column(x, inner_keys).min(inner_keys - 1)];
        set_x(obj, mania::column_x(key, keys));
        (total, from, to) = (total + 1, from.min(obj.start_time), to.max(obj.start_time));
        if extras.get(i).copied().unwrap_or(false) {
            continue;
        }
        lanes[key].push((obj.start_time, end));
        if rare(obj) {
            marked.push((obj.start_time, key));
        }
        match rows.last_mut() {
            Some(row) if (obj.start_time - row.0).abs() < 1.0 => row.1 += 1,
            _ => rows.push((obj.start_time, 1)),
        }
    }
    beatmap.circle_size = keys as f32;
    for lane in &mut lanes {
        lane.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    marked.sort_by(|a, b| a.0.total_cmp(&b.0));
    // The keys with a note of a rare sound at `time`.
    let marked_at = |time: f64| {
        marked[marked.partition_point(|m| m.0 < time - 1.0)..].iter().take_while(|m| m.0 <= time + 1.0).fold(0u32, |k, m| k | 1 << m.1)
    };
    let (Some(&(first, _)), Some(&(last, _))) = (rows.first(), rows.last()) else { return };
    let free_ms = scratch_free_ms(total as f64 / ((to - from) / 1000.0).max(1.0));

    // What the keys of `mask` play around `time`.
    let around = |mask: u32, time: f64| {
        let mut around = Around { row: 0, long: 0, long_end: time, held: false, before: f64::INFINITY, after: f64::INFINITY };
        for k in keys_of(mask) {
            let lane = &lanes[k as usize];
            let mut i = lane.partition_point(|&(start, _)| start < time - 1.0);
            if let Some(&(_, end)) = i.checked_sub(1).map(|j| &lane[j]) {
                if end > time + 1.0 {
                    around.held = true;
                } else {
                    around.before = around.before.min((time - end).max(0.0));
                }
            }
            if let Some(&(start, end)) = lane.get(i).filter(|&&(start, _)| start <= time + 1.0) {
                if end - start <= 1.0 {
                    around.row |= 1 << k;
                } else {
                    around.long |= 1 << k;
                    around.long_end = around.long_end.max(end);
                }
                i += 1;
            }
            if let Some(&(start, _)) = lane.get(i) {
                around.after = around.after.min(start - time);
            }
        }
        around
    };
    // `factor` of the first `(up to, factor)` the gap is within.
    let near = |table: &[(f64, f64)], gap: f64| table.iter().find(|&&(within, _)| gap <= within).map_or(1.0, |&(_, f)| f);
    let fingers = shared_fingers(layout);
    let hand = |s: usize| {
        (0..keys).filter(|&k| k != s && layout.fingers[k].hand == layout.fingers[s].hand).fold(0u32, |m, k| m | 1 << k)
    };
    let sides: Vec<usize> = keys_of(scratches).map(|s| s as usize).collect();
    // The keys of the other hand than scratch `s`'s, the scratches aside.
    let others = |s: usize| ((1u32 << keys) - 1) & !hand(s) & !scratches & !(1 << s);
    // How likely a note on scratch `s` is with what its hand plays around
    // `time`, and the key of the hand whose note in the row it would take
    // (see [`SCRATCH_MOVE`]).
    let side = |s: usize, time: f64, beat_len: f64| -> (f64, u32) {
        let mine = around(hand(s), time);
        if !rules.free_hand {
            return (if mine.held { 0.0 } else { 1.0 }, 0);
        }
        // A long note of the hand there: a muri, and none to take.
        if mine.long != 0 || mine.held {
            return (0.0, 0);
        }
        let pinky = around(fingers[s] & !(1 << s), time);
        let free = near(&SCRATCH_FREE_AFTER, mine.after / free_ms)
            * near(&SCRATCH_FREE_BEFORE, mine.before / free_ms)
            * near(&SCRATCH_PINKY_NEAR, pinky.after.min(pinky.before) / beat_len);
        // Its note in the row can go to it in a chord, the other hand playing
        // too, when it is its pinky's, or when it has a rare sound.
        let other = around(others(s), time);
        let takes = mine.row == pinky.row || other.row != 0 || other.long != 0 || mine.row & marked_at(time) != 0;
        match mine.row.count_ones() {
            0 => (free, 0),
            1 if takes => (free * SCRATCH_MOVE, mine.row),
            _ => (0.0, 0),
        }
    };
    // The same on a strong accent (see [`SCRATCH_STRONG`]): the scratch may
    // take every note its hand has in the row, a long one too, the hand free
    // while it holds.
    let strong_side = |s: usize, time: f64, beat_len: f64| -> (f64, u32) {
        if !rules.free_hand {
            return (0.0, 0);
        }
        let mine = around(hand(s), time);
        let taken = mine.row | mine.long;
        let n = taken.count_ones();
        if mine.held || n == 0 || n > SCRATCH_STRONG_TAKES || mine.long.count_ones() > 1 {
            return (0.0, 0);
        }
        if mine.long != 0 && mine.after < mine.long_end - time {
            return (0.0, 0);
        }
        let pinky = around(fingers[s] & !(1 << s), time);
        let free = near(&SCRATCH_FREE_AFTER, mine.after / free_ms)
            * near(&SCRATCH_FREE_BEFORE, mine.before / free_ms)
            * near(&SCRATCH_PINKY_NEAR, pinky.after.min(pinky.before) / beat_len);
        (free * SCRATCH_MOVE * SCRATCH_TAKE.powi(n as i32 - 1), taken)
    };
    let grid = BeatGrid::new(beatmap);
    let timing = &beatmap.control_points.timing_points;
    let meter = |time: f64| {
        let i = timing.partition_point(|tp| tp.time <= time + 1.0);
        i.checked_sub(1).map_or(4, |i| timing[i].time_signature.numerator.get() as i64).max(1)
    };

    // The places: the rows, and the 1/2 beats with no note, as osu! rounds
    // them to the ms. Only the rows get scratches (see `eligible`); the
    // others tell how the song sounds between them.
    let mut times = rows;
    times.extend(grid.ticks(first, last, 2.0).into_iter().map(|t| (t.round(), 0)));
    times.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f64, usize)> = Vec::with_capacity(times.len());
    for (time, notes) in times {
        match merged.last_mut() {
            Some(place) if time - place.0 <= ROUNDING_MS + 1.0 => {
                if notes > 0 {
                    *place = (time, place.1 + notes);
                }
            }
            _ => merged.push((time, notes)),
        }
    }
    let places: Vec<Place> = merged
        .iter()
        .enumerate()
        .map(|(i, &(time, notes))| {
            let (tp_time, beat_len) = grid.at(time);
            let meter = meter(time);
            let position = patterns::beat_position(time, tp_time, beat_len);
            let bar = position == 0 && (((time - tp_time) / beat_len).round() as i64).rem_euclid(meter) == 0;
            let next = merged.get(i + 1).map_or(time, |&(t, _)| t);
            let chances: Vec<(f64, u32)> = sides.iter().map(|&s| side(s, time, beat_len)).collect();
            let strong: Vec<(f64, u32)> = sides.iter().map(|&s| strong_side(s, time, beat_len)).collect();
            Place {
                time,
                beat_len,
                notes,
                bar,
                beat: position == 0 && !bar,
                offbeat: position >= 2,
                bars: (next - time) / (beat_len * meter as f64),
                bands: accents.map_or([0.0; 3], |a| a.band_onsets(time)),
                hitsound: notes > 0 && marked_at(time) != 0,
                loudness: accents.and_then(|a| a.loudness(time, time + 120.0)),
                sides: chances.iter().map(|c| c.0).collect(),
                takes: chances.iter().map(|c| c.1).collect(),
                strong_sides: strong.iter().map(|c| c.0).collect(),
                strong_takes: strong.iter().map(|c| c.1).collect(),
            }
        })
        .collect();
    let accented = accent_scores(&places, accents.is_some());
    let scores: Vec<f64> = accented.iter().map(|a| a.0).collect();
    // How strong each place is: the share of the song's places scoring less
    // and, with the audio, of those with a weaker sound.
    let share_below = |values: &[f64]| -> Vec<f64> {
        let mut sorted = values.to_vec();
        sorted.sort_by(f64::total_cmp);
        values.iter().map(|&v| sorted.partition_point(|&x| x < v) as f64 / sorted.len().max(1) as f64).collect()
    };
    let strength = share_below(&scores);
    let sound_strength = match accented.iter().map(|a| a.1).collect::<Option<Vec<f64>>>() {
        Some(sounds) => share_below(&sounds),
        None => vec![1.0; places.len()],
    };
    // Per scratch, its chances at place `i` and the notes it takes there: on
    // a strong accent, the most it may take, past what most mappers put only
    // those that leave the map's notes (`deleting`, see [`SCRATCH_STRONG`]).
    let options = |i: usize, deleting: bool| -> Vec<(f64, u32)> {
        let place = &places[i];
        (0..sides.len())
            .map(|k| {
                let basic = (place.sides[k], place.takes[k]);
                let strong = (place.strong_sides[k], place.strong_takes[k]);
                let allowed = strength[i] >= SCRATCH_STRONG && (deleting || strong.1.count_ones() <= 1);
                if allowed && strong.0 > basic.0 { strong } else { basic }
            })
            .collect()
    };
    let possible = |i: usize, deleting: bool| options(i, deleting).iter().map(|o| o.0).fold(0.0, f64::max);
    // The best places, each with a scratch free, as many as the map's density
    // asks for, a little apart, with a score good enough.
    // Scratches go on the rows of the map only, never where it has no note:
    // they mark the sounds it plays. Hand-made 10K2S maps leave the keys out
    // where a scratch goes (44% of their scratches have no key note in the
    // row), but the other maps of their songs have a note there for 90% of
    // their scratches, against 59% of their 1/2 beats (see
    // `corpus_scratch_sources`). On a 1/2 beat with no note, a scratch adds a
    // sound the map left alone.
    // A rare sound of the map is strong enough whatever the audio says.
    let eligible = |i: usize| {
        let place = &places[i];
        place.notes > 0 && strength[i] >= SCRATCH_ACCENT_FROM && (sound_strength[i] >= SCRATCH_SOUND_FROM || place.hitsound)
    };
    // How loud each phrase is against the song's others (see
    // [`SCRATCH_PHRASE_BARS`]), from its rows.
    let mut bar = 0.0;
    let phrase: Vec<usize> = places
        .iter()
        .map(|p| {
            let i = (bar / SCRATCH_PHRASE_BARS) as usize;
            bar += p.bars;
            i
        })
        .collect();
    let mut sums = vec![(0.0, 0usize); phrase.last().map_or(0, |&i| i + 1)];
    for (i, place) in places.iter().enumerate().filter(|(_, p)| p.notes > 0) {
        if let Some(loud) = place.loudness {
            sums[phrase[i]].0 += loud as f64;
            sums[phrase[i]].1 += 1;
        }
    }
    let loudness: Vec<f64> = sums.iter().map(|&(sum, n)| if n > 0 { sum / n as f64 } else { f64::NEG_INFINITY }).collect();
    let mut heard_phrases: Vec<f64> = loudness.iter().copied().filter(|l| l.is_finite()).collect();
    heard_phrases.sort_by(f64::total_cmp);
    let middle = heard_phrases.get(heard_phrases.len() / 2).copied().unwrap_or(0.0);
    // The quiet phrases; of the others, the share of them less loud.
    let quiet = |p: usize| accents.is_some() && loudness[p] < middle - SCRATCH_QUIET_DB;
    let others: Vec<f64> = heard_phrases.iter().copied().filter(|&l| l >= middle - SCRATCH_QUIET_DB).collect();
    let louder = |p: usize| others.partition_point(|&l| l < loudness[p]) as f64 / others.len().max(1) as f64;
    // The most scratch places each phrase may have.
    let phrase_most = |p: usize| {
        if accents.is_none() {
            usize::MAX
        } else if quiet(p) {
            SCRATCH_QUIET_MOST
        } else if louder(p) < SCRATCH_SPARSE_PHRASE {
            SCRATCH_SPARSE_MOST
        } else {
            usize::MAX
        }
    };
    let mut in_phrase = vec![0usize; sums.len()];
    let adjusted: Vec<f64> = (0..places.len())
        .map(|i| {
            if eligible(i) && possible(i, true) > 0.0 {
                scores[i] + possible(i, true).ln()
            } else {
                f64::NEG_INFINITY
            }
        })
        .collect();
    let bars: f64 = places.iter().map(|p| p.bars).sum();
    let (count, base) = if bars > 0.0 {
        let notes_per_bar = places.iter().map(|p| p.notes).sum::<usize>() as f64 / bars;
        let density = match density {
            Density::None => SCRATCH_DENSITY[0],
            Density::Dense => SCRATCH_DENSITY[1],
            Density::MoreDense => SCRATCH_DENSITY[2],
            Density::Full => SCRATCH_DENSITY[3],
        };
        let base = rules.rate * notes_per_bar.powf(rules.rate_power) * bars;
        ((base * density).round() as usize, base.round() as usize)
    } else {
        (0, 0)
    };
    let mut order: Vec<usize> = (0..places.len()).filter(|&i| adjusted[i] > SCRATCH_MIN_SCORE).collect();
    order.sort_by(|&a, &b| adjusted[b].total_cmp(&adjusted[a]).then(a.cmp(&b)));
    // The most scratches in any span of each length, by how hard the map is.
    let nps = total as f64 / ((to - from) / 1000.0).max(1.0);
    let spans: Vec<(f64, usize)> = SCRATCH_SPANS.iter().map(|&(beats, a, b)| (beats, scratch_most(nps, a, b))).collect();
    let mut picked: Vec<usize> = Vec::with_capacity(count);
    // The times of the places picked, in order.
    let mut picked_times: Vec<f64> = Vec::with_capacity(count);
    // Per place picked, each scratch's chances and the notes it takes.
    let mut chosen: Vec<Vec<(f64, u32)>> = vec![Vec::new(); places.len()];
    for &i in &order {
        if picked.len() >= count {
            break;
        }
        let place = &places[i];
        // Past what most mappers put, the scratches add notes.
        let deleting = picked.len() < base;
        let chances = possible(i, deleting);
        if chances <= 0.0 || scores[i] + chances.ln() <= SCRATCH_MIN_SCORE {
            continue;
        }
        let within = |span: f64| {
            let start = picked_times.partition_point(|&t| t < place.time - span);
            &picked_times[start..picked_times.partition_point(|&t| t <= place.time + span)]
        };
        let gap = place.beat_len * SCRATCH_GAP - ROUNDING_MS;
        if within(gap).iter().any(|&t| (t - place.time).abs() < gap) {
            continue;
        }
        if (quiet(phrase[i]) && !place.hitsound) || in_phrase[phrase[i]] >= phrase_most(phrase[i]) {
            continue;
        }
        // The busiest span of each length with this place in it.
        let crowded = spans.iter().any(|&(beats, most)| {
            let span = place.beat_len * beats + ROUNDING_MS;
            let mut around = within(span).to_vec();
            around.insert(around.partition_point(|&t| t < place.time), place.time);
            let mut first = 0;
            (0..around.len()).any(|last| {
                while around[last] - around[first] > span {
                    first += 1;
                }
                last - first + 1 > most
            })
        });
        if crowded {
            continue;
        }
        picked_times.insert(picked_times.partition_point(|&t| t < place.time), place.time);
        chosen[i] = options(i, deleting);
        in_phrase[phrase[i]] += 1;
        picked.push(i);
    }
    // Best first: the score from which both scratches may go.
    let both = picked.get((picked.len() as f64 * SCRATCH_BOTH) as usize).map_or(f64::NEG_INFINITY, |&i| adjusted[i]);
    picked.sort_unstable();

    // In time, the scratch whose hand is freer, or the one that waited
    // longest; both on the biggest accents.
    let mut last_hit = vec![f64::NEG_INFINITY; sides.len()];
    let mut runs = vec![0usize; sides.len()];
    let mut added: Vec<(f64, usize, f64, u32)> = Vec::new();
    for i in picked {
        let place = &places[i];
        // How many notes in a row each scratch would have with this one.
        let run: Vec<usize> = (0..sides.len())
            .map(|k| if place.time - last_hit[k] <= place.beat_len + ROUNDING_MS { runs[k] + 1 } else { 1 })
            .collect();
        let option = &chosen[i];
        let free: Vec<usize> = (0..sides.len()).filter(|&k| option[k].0 > 0.0 && run[k] <= SCRATCH_RUN).collect();
        let these = if free.len() > 1 && adjusted[i] > both && free.iter().all(|&k| option[k].0 >= 1.0) {
            free
        } else {
            let best = free.iter().copied().reduce(|a, b| match option[a].0.total_cmp(&option[b].0) {
                std::cmp::Ordering::Less => b,
                std::cmp::Ordering::Equal if last_hit[b] < last_hit[a] => b,
                _ => a,
            });
            best.into_iter().collect()
        };
        for k in these {
            runs[k] = run[k];
            last_hit[k] = place.time;
            added.push((place.time, sides[k], place.beat_len, option[k].1));
        }
    }

    // The extra notes in a scratch's way go: its hand's [`SCRATCH_FREE_EXTRAS`]
    // around it or across it, its pinky's key's 3/8 beat around.
    let room = free_ms * SCRATCH_FREE_EXTRAS;
    let in_way = |key: usize, start: f64, end: f64| {
        if !rules.free_hand {
            return false;
        }
        added.iter().any(|&(time, s, beat_len, takes)| {
            // A long note the scratch takes: its hand free while it holds.
            let until = if takes != 0 { around(takes, time).long_end } else { time };
            let hand = hand(s) & 1 << key != 0 && start <= until + room && end >= time - room;
            let pinky = fingers[s] & !(1 << s) & 1 << key != 0
                && start <= time + beat_len * SCRATCH_PINKY_NEAR[0].0
                && end >= time - beat_len * SCRATCH_PINKY_NEAR[0].0;
            hand || pinky
        })
    };
    let mut i = 0;
    beatmap.hit_objects.retain(|obj| {
        let extra = extras.get(i).copied().unwrap_or(false);
        i += 1;
        !extra || mania::note(obj).is_none_or(|(x, end)| !in_way(mania::column(x, keys), obj.start_time, end))
    });

    // The notes the scratches take: the long one or the one next to it moves
    // out to it, sounds and all, the others go, their additions on it.
    let mut gone = vec![false; beatmap.hit_objects.len()];
    for &(time, s, _, takes) in added.iter().filter(|a| a.3 != 0) {
        let objects = &beatmap.hit_objects;
        let key = |i: usize| mania::note(&objects[i]).map(|(x, end)| (mania::column(x, keys), end - objects[i].start_time));
        let row: Vec<usize> = (objects.partition_point(|o| o.start_time < time - 1.0)..objects.len())
            .take_while(|&i| objects[i].start_time <= time + 1.0)
            .filter(|&i| key(i).is_some_and(|(k, _)| takes & 1 << k != 0))
            .collect();
        // The note with a rare sound first, then the long one, then the one
        // next to the scratch.
        let order = |i: usize| key(i).map_or((true, true, usize::MAX), |(k, length)| (!rare(&objects[i]), length <= 1.0, k.abs_diff(s)));
        let Some(&moved) = row.iter().min_by_key(|&&i| order(i)) else {
            continue;
        };
        let sounds: Vec<HitSampleInfo> = row
            .iter()
            .filter(|&&i| i != moved)
            .flat_map(|&i| objects[i].samples.clone())
            .filter(|sample| sample.name != HitSampleInfo::HIT_NORMAL)
            .collect();
        let note = &mut beatmap.hit_objects[moved];
        set_x(note, mania::column_x(s, keys));
        for sample in sounds {
            if !note.samples.contains(&sample) {
                note.samples.push(sample);
            }
        }
        for i in row.into_iter().filter(|&i| i != moved) {
            gone[i] = true;
        }
    }
    let mut i = 0;
    beatmap.hit_objects.retain(|_| {
        i += 1;
        !gone[i - 1]
    });

    // The other scratches' notes: a plain note made from the nearest note
    // (one of the row if there is one) without its sample or additions.
    let mut notes = Vec::with_capacity(added.len());
    for &(time, s, _, _) in added.iter().filter(|a| a.3 == 0) {
        let objects = &beatmap.hit_objects;
        let i = objects.partition_point(|o| o.start_time < time);
        let distance = |o: &&HitObject| (o.start_time - time).abs();
        let Some(template) = [i.checked_sub(1), Some(i)]
            .into_iter()
            .flatten()
            .filter_map(|i| objects.get(i))
            .min_by(|a, b| distance(a).total_cmp(&distance(b)))
        else {
            continue;
        };
        let mut note = template.clone();
        note.start_time = time;
        note.kind = HitObjectKind::Circle(HitObjectCircle {
            pos: Pos::new(mania::column_x(s, keys), 192.0),
            new_combo: false,
            combo_offset: 0,
        });
        let volume = note.samples.first().map_or(100, |sample| sample.volume);
        note.samples.retain(|sample| sample.name == HitSampleInfo::HIT_NORMAL);
        if note.samples.is_empty() {
            note.samples.push(HitSampleInfo::new(HitSampleInfo::HIT_NORMAL, None, 0, volume));
        }
        notes.push(note);
    }
    beatmap.hit_objects.extend(notes);
    beatmap.hit_objects.sort_by(|a, b| a.start_time.total_cmp(&b.start_time));
}

/// How far (in ms) the hand of a scratch keeps away from it in a map of
/// `nps` notes a second (see [`SCRATCH_FREE_MS`]).
pub(crate) fn scratch_free_ms(nps: f64) -> f64 {
    SCRATCH_FREE_MS * nps.max(0.1).powf(-SCRATCH_FREE_POWER)
}

/// The most scratches in a span of [`SCRATCH_SPANS`] `(_, a, b)` in a map of
/// `nps` notes a second.
pub(crate) fn scratch_most(nps: f64, a: f64, b: f64) -> usize {
    ((a + b * nps.max(1.0).ln()).floor().max(0.0) as usize).max(2)
}
