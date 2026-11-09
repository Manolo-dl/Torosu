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
//!   notes spread over every key, without anchors or minijacks. Where the
//!   target has few new keys (10K -> 12K, 7K -> 8K), a column coming back to
//!   its key is not spread, and one that just played keeps to its key: the
//!   source's jacks, trills and back-and-forths stay as they are.
//! - **Jacks**: no new jacks; the map's own jacks (the same column in
//!   consecutive rows, up to 1/2 beat apart) stay on a single key. Where a
//!   finger has two keys (10K8K, DP, 9K9K, a scratch and its pinky), going
//!   from one to the other is a jack too: hand-made maps hardly ever do it.
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
//! - **Wide hands** (4 keys or more each: 8K to 18K): each hand plays its
//!   own patterns, as in hand-made 10K maps, instead of the hands taking
//!   turns as on 7K. A hand goes on with its stair or stays on three of its
//!   keys (pinky-ring-middle, ...), and no pills are made where the source has
//!   none: open chords stay open. Extra notes go on any key that suits what
//!   its hand plays rather than next to the chord, mostly in the denser parts
//!   of the source.
//! - **Scratches** (10K2S, 14K2S DP): the pinky plays the scratch and the key
//!   next to it, so they never have notes at once. The scratch is an accent,
//!   on the beat, in short groups with rests in between: a note only goes
//!   there if its column does not have to follow it (jacks, chordjacks,
//!   short notes right after it), so it never takes a column for bars. The
//!   pinky shares its notes about evenly between the scratch and that key.
//!   That is 12K, every key a lane. 10K2S keeps the scratches out of the
//!   patterns, as 7K+1 does: the map is converted to 10K on the inner keys,
//!   and the scratches get notes of their own on top, on the accents of the
//!   music: the map's big chords and the sounds of its audio that stand out,
//!   as hand-made 10K2S maps hit them. The scratch's hand is free then, as
//!   there: nothing pressed or held with it (a muri), and its next notes as
//!   far as the map is easy. On a chord, the hand's note in it may go to the
//!   scratch instead, moving out (see [`add_scratch_lanes`]). The easier the
//!   map, the fewer scratches come close together; the quiet parts are left
//!   to the keys, but for the accent sounds of the map (a crash), whose notes
//!   move out to the scratches.
//!   7K+1 (8K) does the same with a 7K map and one scratch on the
//!   left, but its scratch goes with the hand's notes, as in hand-made 7K+1
//!   maps: the 7K map stays as it is, every note of it, and the scratch only
//!   adds notes, far fewer than on 10K2S (see [`ScratchRules`]).
//!
//! The hands come from the playstyle of the target (`analysis::layout`): on
//! 8K `3+2+3` the two middle keys are the thumbs', on `7K+1` the first key is
//! a scratch, on 14K `EZ2AC` the four middle keys go with either hand. 12K to
//! 18K are played solo, two stages of one player; co-op is not converted to.
//!
//! On two stages each note only tries its [`KEY_CHOICES`] cheapest keys, and
//! rows of many notes fewer ([`ROW_CHOICES`]): trying every set of keys a
//! chord could go to would take too long.
//!
//! A note that cannot be placed (every key it could go to is held) is dropped,
//! so the output is always a valid map.

mod costs;
mod keys;
mod rows;
mod scratch;
mod search;
#[cfg(test)]
mod tests;
mod weights;

use keys::*;
use rows::*;
#[cfg(test)]
pub(crate) use scratch::scratch_free_ms;
use scratch::*;
use search::*;
use weights::*;

use anyhow::Result;
use crate::error::Error;
use rosu_map::{
    Beatmap,
    section::{
        general::GameMode,
        hit_objects::{HitObject, HitObjectCircle, HitObjectKind, hit_samples::HitSampleInfo},
    },
    util::Pos,
};

use crate::commands::trainer::{
    accents::MapAccents,
    analysis::{
        layout::{self, Digit, Finger, Hand, Layout},
        patterns::{self, Pattern},
        zones::Zone,
    },
    mania::{self, MAX_KEYS},
    model::{Density, KeyConverterParams},
    timing::BeatGrid,
};

/// [`apply_with`] without the audio, as the tests convert.
#[cfg(test)]
pub fn apply(beatmap: &mut Beatmap, params: &KeyConverterParams) -> Result<()> {
    apply_with(beatmap, params, None)
}

/// [`apply`], with the accents of the map's audio where the convert follows
/// them (see [`uses_audio`]); without them it goes by the map alone.
pub fn apply_with(beatmap: &mut Beatmap, params: &KeyConverterParams, accents: Option<MapAccents>) -> Result<()> {
    convert_marked(beatmap, params, accents).map(|_| ())
}

/// Whether a convert with `params` follows the audio: 10K2S puts its
/// scratches on the accents of the music.
pub fn uses_audio(params: &KeyConverterParams) -> bool {
    layout::layout(params.target_keys as usize, params.playstyle.as_deref()).inner_keys().is_some()
}

/// [`apply_with`], telling for each hit object of the result whether it is
/// an extra note of the density, not one of the map's.
fn convert_marked(beatmap: &mut Beatmap, params: &KeyConverterParams, accents: Option<MapAccents>) -> Result<Vec<bool>> {
    if beatmap.mode != GameMode::Mania {
        return Err(Error::ManiaOnly { task: "KeyConverter" }.into());
    }

    let src_keys = mania::key_count(beatmap);
    let dst_keys = params.target_keys as usize;
    for keys in [src_keys, dst_keys] {
        if !mania::is_playable(keys) {
            return Err(Error::UnsupportedKeys { keys }.into());
        }
    }
    if src_keys == dst_keys {
        log::warn!("key converter: the map is already {}K, left as is", dst_keys);
        return Ok(vec![false; beatmap.hit_objects.len()]);
    }

    let grid = BeatGrid::new(beatmap);

    // How the source and the target are played.
    let src_layout = layout::layout(src_keys, None);
    let dst_layout = layout::layout(dst_keys, params.playstyle.as_deref());
    if dst_layout.fingers.iter().any(|f| f.player > 0) {
        return Err(Error::OnePlayer { layout: dst_layout.name.to_string() }.into());
    }

    // 10K2S and 7K+1: the map converted to 10K or 7K on the inner keys, and
    // the scratches' own notes on top.
    if let Some(inner_keys) = dst_layout.inner_keys() {
        let extras = if src_keys != inner_keys {
            let inner = KeyConverterParams { target_keys: inner_keys as u8, playstyle: None, ..params.clone() };
            convert_marked(beatmap, &inner, None)?
        } else {
            vec![false; beatmap.hit_objects.len()]
        };
        add_scratch_lanes(beatmap, &dst_layout, inner_keys, &extras, params.density, accents, ScratchRules::of(&dst_layout));
        return Ok(vec![false; beatmap.hit_objects.len()]);
    }

    // Rows: the notes that start at the same time, with the patterns and
    // density zones of the source there.
    // 4K -> 7K is tuned on hand-made maps of its own (see `FULL_GROWTH`,
    // `W_THUMB_CHORD` and `W_ANCHOR`), and reads a map timed at half its
    // tempo at the tempo of the music; the other key counts are not yet.
    let measured = src_keys == 4 && dst_keys == 7;
    let half = measured && half_timed(beatmap, &grid);
    let (mut notes, mut rows) = read_rows(beatmap, src_keys, &src_layout, &grid, half);
    let spread = spread(src_keys, dst_keys);
    mark_patterns(&mut rows, &notes, params.keep_jacks);
    look_ahead(&rows, &mut notes, spread < 1.0);

    // Extra notes only make sense with more keys.
    let fill = fill_settings(params.density).filter(|_| dst_keys > src_keys);

    // A row has a note per finger at most: on 10K2S, 12 keys but 10 fingers.
    let fingers = shared_fingers(&dst_layout);
    let max_chord = fingers_count(&fingers, dst_keys);
    set_shares(&mut rows, src_keys, dst_keys, max_chord);
    if fill.is_some_and(|f| f.scaled) {
        set_full_quotas(&mut rows, src_keys, dst_keys, max_chord, spread, measured);
    }

    // The two hands of the target, the middle keys aside.
    let dst_hands = hands(&dst_layout);
    let hand_mask = |hand: Hand| (0..dst_keys).filter(|&c| dst_hands[c] == hand).fold(0u32, |m, c| m | 1 << c);
    let dst_groups = [hand_mask(Hand::Left), hand_mask(Hand::Right)];
    let scratches = (0..dst_keys)
        .filter(|&k| dst_layout.fingers[k].digit == Digit::Scratch && fingers[k] != 1 << k)
        .fold(0u32, |m, k| m | 1 << k);

    let ctx = Ctx {
        notes: &notes,
        src_keys,
        dst_keys,
        keep_jacks: params.keep_jacks,
        fill,
        src_hands: hands(&src_layout),
        dst_hands,
        dst_groups,
        wide: dst_groups.iter().all(|g| g.count_ones() >= 4),
        scratches,
        pinkies: keys_of(scratches).fold(scratches, |m, s| m | fingers[s as usize]),
        spread,
        fingers,
        thumbs: (0..dst_keys)
            .filter(|&k| measured && dst_layout.fingers[k].hand == Hand::Either && dst_layout.fingers[k].digit == Digit::Thumb)
            .fold(0u32, |m, k| m | 1 << k),
        anchors: measured && fill.is_some(),
        key_shares: measured,
    };

    // Where the target has few new keys, the extra notes go on top of the
    // source's own placement, the one without them: placing both at once, a
    // good key for an extra note moved the notes of the map, and the streams
    // lost their direction.
    let path = if spread < 1.0 && fill.is_some() {
        let placed = search(&Ctx { fill: None, ..ctx }, &rows);
        add_extras(&ctx, &rows, &placed)
    } else {
        search(&ctx, &rows)
    };

    Ok(write(beatmap, &ctx, &rows, &path))
}

/// How much a column coming back to its key is spread over the others (see
/// `SPREAD_FULL`); going down in keys, in full: the target has no room to
/// keep them.
fn spread(src_keys: usize, dst_keys: usize) -> f64 {
    if dst_keys < src_keys {
        return 1.0;
    }
    let room = (dst_keys - src_keys) as f64 / src_keys as f64;
    ((room - SPREAD_FROM) / (SPREAD_FULL - SPREAD_FROM)).clamp(0.0, 1.0)
}

/// Moves the notes of the map to the keys `path` gives them, drops those it
/// has no key for, and adds the extra notes, each a copy of the note of its
/// row closest to it. Returns, for each hit object of the result, whether it
/// is an extra note.
fn write(beatmap: &mut Beatmap, ctx: &Ctx, rows: &[Row], path: &[Decision]) -> Vec<bool> {
    let (notes, dst_keys) = (ctx.notes, ctx.dst_keys);
    let mut targets: Vec<Option<u8>> = vec![None; beatmap.hit_objects.len()];
    let mut extra: Vec<HitObject> = Vec::new();

    for (row, decision) in rows.iter().zip(path) {
        for (&i, col) in kept(row.notes.len(), decision.size()).iter().zip(keys_of(decision.keys)) {
            targets[notes[row.notes.start + i].idx] = Some(col);
        }
        for &col in &decision.extras[..decision.n_extras as usize] {
            if let Some(n) = ctx.template(row, decision, col) {
                let mut copy = beatmap.hit_objects[notes[n].idx].clone();
                set_x(&mut copy, mania::column_x(col as usize, dst_keys));
                extra.push(copy);
            }
        }
    }

    let total = beatmap.hit_objects.len();
    // The hit objects, and whether each is an extra note.
    let mut objects: Vec<(HitObject, bool)> = Vec::with_capacity(total + extra.len());
    for (mut obj, target) in std::mem::take(&mut beatmap.hit_objects).into_iter().zip(&targets) {
        match (&obj.kind, target) {
            (HitObjectKind::Circle(_) | HitObjectKind::Hold(_), Some(col)) => {
                set_x(&mut obj, mania::column_x(*col as usize, dst_keys));
                objects.push((obj, false));
            }
            (HitObjectKind::Circle(_) | HitObjectKind::Hold(_), None) => {}
            _ => objects.push((obj, false)),
        }
    }

    let dropped = total - objects.len();
    let added = extra.len();
    objects.extend(extra.into_iter().map(|obj| (obj, true)));
    objects.sort_by(|a, b| a.0.start_time.total_cmp(&b.0.start_time));
    let (objects, extras): (Vec<HitObject>, Vec<bool>) = objects.into_iter().unzip();

    beatmap.hit_objects = objects;
    beatmap.circle_size = dst_keys as f32;

    log::debug!(
        "key converter: {}K -> {}K, {} row(s), {} dropped, {} added",
        ctx.src_keys,
        dst_keys,
        rows.len(),
        dropped,
        added
    );
    extras
}

/// Moves a note to the column at `x`.
fn set_x(obj: &mut HitObject, x: f32) {
    match &mut obj.kind {
        HitObjectKind::Circle(circle) => circle.pos.x = x,
        HitObjectKind::Hold(hold) => hold.pos_x = x,
        _ => {}
    }
}
