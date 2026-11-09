//! The scores of the places a scratch can go: where they fall in the bar,
//! the chords of the map there and how the audio sounds (see [`Place`]).

use super::*;

/// Score of a place on the first beat of a bar, and on its other beats. The
/// 1/2s get nothing more: they only get one for a sound or a chord.
pub(super) const ACCENT_BAR: f64 = 1.35;

pub(super) const ACCENT_BEAT: f64 = 0.82;

/// Score of a row off the 1/2s (1/4s, 1/3s...): hand-made maps put 3% of
/// their scratches there, and only the hard ones, as the pinky has to move
/// out and back in a hurry.
pub(super) const ACCENT_OFFBEAT: f64 = -1.0;

/// Score per how big the chord of a row is against the biggest one 4 beats
/// around (its notes over theirs, up to 1): the map's own accents, where its
/// mapper put the big chords. A hand-made 10K2S map without its scratches
/// hides them, as its keys leave the hand free where a scratch goes, but the
/// other maps of its song show them.
pub(super) const ACCENT_CHORD: f64 = 2.0;

/// A row of the map, the sound aside.
pub(super) const ACCENT_ROW: f64 = -0.3;

/// Score per how strongly a sound starts at a place, by band (see
/// `accents`): the kicks and bass (30 to 200 Hz) and the middle (200 Hz to
/// 2 kHz) against the whole song, the share of its places where they start
/// less strongly (0 to 1), and the cymbals and hats (2 to 11 kHz), there all
/// along, against the 4 beats around; per how loud the place is against the
/// whole song (the same share), and per standard deviation the 8 beats
/// around are louder than the song. Fitted with a logistic regression on
/// every place of the hand-made 10K2S maps, against where their mappers put
/// a scratch (see `corpus_scratch_sounds`): the scratches mark the sounds
/// that stand out in the song, not the loudest of every bar. Scored against
/// the 4 beats around instead, half the scratches of a converted 3 star map
/// went to its quiet parts, where its hands are free, and half of those in a
/// row to sounds weaker than the song's middle one.
pub(super) const ACCENT_LOW: f64 = 0.98;

pub(super) const ACCENT_MID: f64 = 1.42;

pub(super) const ACCENT_HIGH: f64 = 0.91;

pub(super) const ACCENT_LOUD: f64 = 1.48;

pub(super) const ACCENT_SECTION: f64 = 0.24;

/// What a sound scores with those as strong as the song's middle one: taken
/// off, so the scores keep the scale of the rest (a log of odds).
pub(super) const ACCENT_AVERAGE: f64 = 2.4;

/// Without the audio, a row sounds like the ones around it, and a place with
/// no note has no sound to mark.
pub(super) const ACCENT_UNHEARD: f64 = 0.45;

/// A row with an accent sound of the map scores this much more, as much as
/// the biggest chord around, and strong enough whatever the audio says: its
/// mapper picked it for an accent (a crash, a water drop). That is additions or a sample of its own that fewer
/// than [`SCRATCH_RARE_SOUND`] of the map's notes have, [`SCRATCH_RARE_APART`]
/// beats apart or more as a rule: one on every beat for bars is a part to
/// play (a hi-hat roll). A scratch of its hand takes that note, moved out to
/// it, so the sound comes from the scratch. Hand-made 10K2S maps keep their
/// sounds out of their notes, so they tell nothing of it, but the maps they
/// are converted from often have them.
pub(super) const ACCENT_HITSOUND: f64 = 2.0;

/// How much each place (in time) sounds like an accent (see [`ACCENT_BAR`]
/// and the next): where it falls in the bar, how big a chord the map plays
/// there and, with the audio, how strongly a sound starts there and how loud
/// it is against the whole song. With the audio, also how strong its sound
/// alone is, `None` without it.
pub(super) fn accent_scores(places: &[Place], heard: bool) -> Vec<(f64, Option<f64>)> {
    let median = |values: &mut Vec<f64>| {
        values.sort_by(f64::total_cmp);
        values.get(values.len() / 2).copied().unwrap_or(0.0)
    };
    let spread = |values: &[f64]| {
        let mean = values.iter().sum::<f64>() / values.len().max(1) as f64;
        (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len().max(1) as f64).sqrt()
    };
    // Where the audio says nothing (past its end), as loud as the song.
    let mut known: Vec<f64> = places.iter().filter_map(|p| p.loudness.map(f64::from)).collect();
    let song_spread = spread(&known);
    let song = median(&mut known);
    let loudness: Vec<f64> = places.iter().map(|p| p.loudness.map_or(song, f64::from)).collect();
    // The places `beats` around place `i`.
    let around = |i: usize, beats: f64| {
        let (time, width) = (places[i].time, places[i].beat_len * beats);
        places.partition_point(|p| p.time < time - width)..places.partition_point(|p| p.time <= time + width)
    };
    // The values of the song's places in order, and the share of them below
    // a value.
    let sorted = |mut values: Vec<f64>| {
        values.sort_by(f64::total_cmp);
        values
    };
    let in_song = |sorted: &[f64], value: f64| sorted.partition_point(|&x| x < value) as f64 / sorted.len().max(1) as f64;
    let low = sorted(places.iter().map(|p| p.bands[0] as f64).collect());
    let mid = sorted(places.iter().map(|p| p.bands[1] as f64).collect());
    let loud = sorted(loudness.clone());

    places
        .iter()
        .enumerate()
        .map(|(i, place)| {
            let row = place.notes > 0;
            let metre = if place.bar {
                ACCENT_BAR
            } else if place.beat {
                ACCENT_BEAT
            } else if place.offbeat {
                ACCENT_OFFBEAT
            } else {
                0.0
            };
            let chord = if row {
                let biggest = places[around(i, 4.0)].iter().map(|p| p.notes).max().unwrap_or(1);
                ACCENT_ROW + ACCENT_CHORD * place.notes as f64 / biggest as f64
            } else {
                0.0
            };
            let picked = if place.hitsound { ACCENT_HITSOUND } else { 0.0 };
            if !heard {
                return (metre + chord + picked + if row { ACCENT_UNHEARD } else { 0.0 }, None);
            }
            let near = around(i, 4.0);
            let high = places[near.clone()].iter().filter(|p| p.bands[2] < place.bands[2]).count() as f64 / near.len() as f64;
            let part = &loudness[around(i, 8.0)];
            let section = (part.iter().sum::<f64>() / part.len() as f64 - song) / (song_spread + 1e-6);
            let sound = ACCENT_LOW * in_song(&low, place.bands[0] as f64)
                + ACCENT_MID * in_song(&mid, place.bands[1] as f64)
                + ACCENT_HIGH * high
                + ACCENT_LOUD * in_song(&loud, loudness[i])
                + ACCENT_SECTION * section;
            let scored = if row { chord + sound } else { sound };
            (metre + scored + picked - ACCENT_AVERAGE, Some(sound))
        })
        .collect()
}
