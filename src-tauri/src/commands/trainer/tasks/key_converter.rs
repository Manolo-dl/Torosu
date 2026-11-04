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
//!
//! The hands come from the playstyle of the target (`analysis::layout`): on
//! 8K `3+2+3` the two middle keys are the thumbs', on `1+7` the first key is
//! a scratch, on 14K `EZ2AC` the four middle keys go with either hand. 12K to
//! 18K are played solo, two stages of one player; co-op is not converted to.
//!
//! On two stages each note only tries its [`KEY_CHOICES`] cheapest keys, and
//! rows of many notes fewer ([`ROW_CHOICES`]): trying every set of keys a
//! chord could go to would take too long.
//!
//! A note that cannot be placed (every key it could go to is held) is dropped,
//! so the output is always a valid map.

use anyhow::{Result, bail};
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

const BEAM_WIDTH: usize = 8;
/// On two stages (12K to 18K), the keys each note of a row tries, its
/// cheapest. With 6, converts to 14K and 18K came out the same as trying
/// every key, but 4 to 9 times faster.
const KEY_CHOICES: usize = 6;
/// On two stages, the most ways a row tries to place its notes: rows of up to
/// 4 notes try all their keys, bigger ones (12 notes on 12K) fewer per note.
const ROW_CHOICES: usize = KEY_CHOICES.pow(4);

/// Notes in the same column closer than this (in beats) are a jack, as in the
/// original: a 1/4 with some leniency. New ones are avoided.
const JACK_BEATS: f64 = 0.375;
/// The same column in consecutive source rows up to this far apart (in beats)
/// is a jack of the map, kept on one key: fast maps jack at 1/2.
const SOURCE_JACK_BEATS: f64 = 0.55;

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
/// Leeway for gaps measured in beats: osu! rounds the notes to the ms, so a
/// 1/2 at 174 BPM (172.4 ms) comes out as 172 or 173.
const ROUNDING_MS: f64 = 2.0;
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
/// New keys per source key from which a column coming back to its key is
/// spread over the others ([`W_FRESH`], [`W_REPEAT2`]) in full, and up to
/// which it is not at all: 4K -> 7K (0.75) and 7K -> 10K (0.43) roll the
/// back-and-forths of the source over the new keys, 10K -> 12K (0.2) keeps
/// them, as its mapper meant them with 10 keys to choose from. With half the
/// cost, a 10K map's columns coming back two notes later kept their key 40%
/// of the time; with none, 80%.
const SPREAD_FROM: f64 = 0.2;
const SPREAD_FULL: f64 = 0.4;
/// Where the target has few new keys, a column moving to another key while
/// it played within the last beat costs this, times how little it spreads
/// ([`SPREAD_FULL`]): the scratch's reward or the pinky's balance do not pull
/// a column away mid-pattern. On a 10K map going to 12K, with 4 the columns
/// coming back two notes later kept their key 96 to 98% of the time (85 to
/// 89% without); with 8, 98 to 100%, but the scratch kept columns for runs of
/// 10.
const W_STICK: f64 = 4.0;

// Scratches as lanes of their own (10K2S, see [`add_scratch_lanes`]): where
// they go, fitted on 63 hand-made 10K2S maps (20 songs, 14 mappers) without
// their scratches, against where their mappers hit them (see
// `corpus_scratch_truth`), and on the other maps of the songs of some of
// them (see `corpus_scratch_sources`). A place for one is a row of the map;
// each gets a score from where it falls in the bar, how big a chord the map
// has there and how the audio sounds there against the whole song, its rows
// and its 1/2 beats with no note (see `accents`), and the best of them get a
// scratch.

/// Score of a place on the first beat of a bar, and on its other beats. The
/// 1/2s get nothing more: they only get one for a sound or a chord.
const ACCENT_BAR: f64 = 1.35;
const ACCENT_BEAT: f64 = 0.82;
/// Score of a row off the 1/2s (1/4s, 1/3s...): hand-made maps put 3% of
/// their scratches there, and only the hard ones, as the pinky has to move
/// out and back in a hurry.
const ACCENT_OFFBEAT: f64 = -1.0;
/// Score per how big the chord of a row is against the biggest one 4 beats
/// around (its notes over theirs, up to 1): the map's own accents, where its
/// mapper put the big chords. A hand-made 10K2S map without its scratches
/// hides them, as its keys leave the hand free where a scratch goes, but the
/// other maps of its song show them.
const ACCENT_CHORD: f64 = 2.0;
/// A row of the map, the sound aside.
const ACCENT_ROW: f64 = -0.3;
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
const ACCENT_LOW: f64 = 0.98;
const ACCENT_MID: f64 = 1.42;
const ACCENT_HIGH: f64 = 0.91;
const ACCENT_LOUD: f64 = 1.48;
const ACCENT_SECTION: f64 = 0.24;
/// What a sound scores with those as strong as the song's middle one: taken
/// off, so the scores keep the scale of the rest (a log of odds).
const ACCENT_AVERAGE: f64 = 2.4;
/// Without the audio, a row sounds like the ones around it, and a place with
/// no note has no sound to mark.
const ACCENT_UNHEARD: f64 = 0.45;
/// A row with an accent sound of the map scores this much more, as much as
/// the biggest chord around, and strong enough whatever the audio says: its
/// mapper picked it for an accent (a crash, a water drop). That is additions or a sample of its own that fewer
/// than [`SCRATCH_RARE_SOUND`] of the map's notes have, [`SCRATCH_RARE_APART`]
/// beats apart or more as a rule: one on every beat for bars is a part to
/// play (a hi-hat roll). A scratch of its hand takes that note, moved out to
/// it, so the sound comes from the scratch. Hand-made 10K2S maps keep their
/// sounds out of their notes, so they tell nothing of it, but the maps they
/// are converted from often have them.
const ACCENT_HITSOUND: f64 = 2.0;
const SCRATCH_RARE_SOUND: f64 = 0.02;
const SCRATCH_RARE_APART: f64 = 2.0;
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
const SCRATCH_PHRASE_BARS: f64 = 4.0;
const SCRATCH_QUIET_DB: f64 = 3.0;
const SCRATCH_QUIET_MOST: usize = 1;
const SCRATCH_SPARSE_PHRASE: f64 = 0.5;
const SCRATCH_SPARSE_MOST: usize = 2;
/// How many scratches a map gets: this many a bar, times its notes a bar to
/// this power. Hand-made maps with 4 notes a bar have 0.6 a bar, with 16
/// 1.5, with 32 2.3, give or take 70%: the more a map plays, the more accents
/// it marks. The notes tell it a little better than the rows: bigger chords,
/// more scratches.
const SCRATCH_RATE: f64 = 0.26;
const SCRATCH_RATE_POWER: f64 = 0.63;
/// That times this per density (none, dense, more dense, full): what most
/// mappers put, and up to half as many more. Twice as many made a 3 star map
/// a scratch every beat for bars; and where the map is busy, the most a span
/// can have ([`SCRATCH_SPANS`]) holds them back anyway.
const SCRATCH_DENSITY: [f64; 4] = [1.0, 1.15, 1.3, 1.5];
/// A place gets a scratch only with a score above this, its hand's chances
/// counted ([`SCRATCH_FREE_AFTER`]): where its hand is about to play, only
/// for a big accent.
const SCRATCH_MIN_SCORE: f64 = 0.0;
/// A place gets a scratch only if it scores more than this share of the
/// song's places, and with the audio, if its sound alone is stronger than
/// this share of theirs: the scratches mark the strong sounds, and where a
/// map has fewer than [`SCRATCH_RATE`] asks for, it gets fewer scratches
/// rather than some on weaker sounds. Half the scratches of hand-made maps
/// are on sounds stronger than three quarters of their song's, in a row as
/// much as alone, and a fifth on sounds weaker than its middle one (see
/// `corpus_scratch_sounds`), which the converts leave alone: without it, the
/// count went on to the quiet parts, and so did the scratches in a row.
const SCRATCH_ACCENT_FROM: f64 = 0.5;
const SCRATCH_SOUND_FROM: f64 = 0.5;

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
const SCRATCH_STRONG: f64 = 0.8;
const SCRATCH_STRONG_TAKES: u32 = 3;
const SCRATCH_TAKE: f64 = 0.75;
/// The best of the places that get a scratch (this share of them), both
/// hands free, get both, as hand-made maps do for their biggest accents (9%
/// of their rows with a scratch).
const SCRATCH_BOTH: f64 = 0.15;
/// Scratches are at least this far apart (in beats): hand-made maps hardly
/// ever have two 1/4 beat apart (the hardest, 1 in 10 at most).
const SCRATCH_GAP: f64 = 0.5;
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
const SCRATCH_SPANS: [(f64, f64, f64); 2] = [(4.0, 0.6, 1.5), (8.0, 0.9, 2.2)];
/// At most this many notes in a row on a scratch, each a beat or less after
/// the last: an accent, not a lane. The longest run of a hand-made map is 3
/// notes (2 to 7); dense maps, where a hand is mostly busy, gave the other
/// scratch runs of 20 without it.
const SCRATCH_RUN: usize = 4;
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
const SCRATCH_FREE_MS: f64 = 855.0;
const SCRATCH_FREE_POWER: f64 = 0.75;
/// How likely a scratch is with the hand's next note starting this far after
/// it, and with its last one released this far before it, as (up to how far,
/// in [`SCRATCH_FREE_MS`] of the map, times what), against a hand free for
/// longer: the rate of scratches on the rows and 1/2 beats of the hand-made
/// maps without them (see `corpus_scratch_places`). Hardly ever under half
/// of it, and the hand comes back a little later than it leaves. A long note
/// of the hand released on the scratch, which 4 hard maps have, is not made
/// either.
const SCRATCH_FREE_AFTER: [(f64, f64); 5] = [(0.5, 0.0), (0.7, 0.1), (0.85, 0.2), (1.0, 0.55), (1.25, 0.75)];
const SCRATCH_FREE_BEFORE: [(f64, f64); 4] = [(0.5, 0.0), (0.7, 0.25), (0.85, 0.4), (1.0, 0.6)];
/// The key of the scratch's pinky, from its notes' ends too, as (up to how
/// far, in beats, times what): never 3/8 beat away or closer (a jack of the
/// pinky), less 1/2 beat away.
const SCRATCH_PINKY_NEAR: [(f64, f64); 2] = [(0.38, 0.0), (0.56, 0.4)];
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
const SCRATCH_MOVE: f64 = 0.75;
/// The density's extra notes of a scratch's hand make way for it this far
/// around it (in [`SCRATCH_FREE_MS`] of the map), the hand's room in
/// hand-made maps; on its pinky's key, [`SCRATCH_PINKY_NEAR`]'s.
const SCRATCH_FREE_EXTRAS: f64 = 1.0;
/// Reward for a note on a scratch the pinky moves out to (10K2S, 14K2S DP),
/// on the beat; [`SCRATCH_BEAT`] of it elsewhere. Hand-made 10K2S maps use
/// the scratch as an accent, 74% of its notes on the beat; without the
/// reward, the scratch gets few notes and those in runs, from the source
/// columns whose natural place it is.
const W_SCRATCH: f64 = 8.0;
/// The share of [`W_SCRATCH`] by position in the beat (1/1, 1/2, 1/4, other).
const SCRATCH_BEAT: [f64; 4] = [1.0, 0.5, 0.0, 0.0];
/// A scratch costs this per note it had lately and would have next (see
/// [`Note::ahead`]), each decaying over [`SCRATCH_DECAY_BEATS`]: hand-made
/// maps hit it in short groups with long rests in between (a third of the
/// gaps a beat or less, a quarter over 8 beats), never for bars on end.
/// With 8 and 3, 4K, 7K and 10K converts give each scratch 0.5 to 1.6x the
/// notes of a key, in runs of 7 at most (see `corpus_scratch`); a reward of
/// 10, or a level of 2, makes runs of 10 to 20 again.
const W_SCRATCH_LEVEL: f64 = 3.0;
const SCRATCH_DECAY_BEATS: f64 = 4.0;
/// Per note a scratch has had more than the key next to it (the other key
/// of its pinky) lately, decaying over [`PINKY_BALANCE_BEATS`], a note on it
/// costs this more, and one on that key this less, by
/// [`PINKY_BALANCE_CAP`] notes at most; the same the other way. Hand-made
/// 10K2S maps share the pinky's notes about evenly (6.6% on the scratch, 6.0%
/// on the key); without it the scratch took four times as many, as the
/// source column whose natural place it is stays on it. With 2, converts get
/// 0.9 to 1.3 notes on the scratch per note on the key. A bigger reward
/// brings runs back: a scratch left alone for long would take every note.
const W_PINKY_BALANCE: f64 = 2.0;
const PINKY_BALANCE_BEATS: f64 = 16.0;
const PINKY_BALANCE_CAP: f64 = 2.0;

/// Most extra notes a row can get.
const MAX_EXTRAS: usize = 4;
/// Full: chords grow by the key ratio times this. Measured on Enie's 7K
/// H.A.R.D.C.O.R.E. against the 4K: 1 note -> 1.5, 2 -> 3.4, 3 -> 4.6.
const FULL_SCALE: f64 = 0.9;
/// Full: how much a full row of the source grows (tyrcs: a quad -> 4.1).
const FULL_ROW_GROWTH: f64 = 0.5;
/// Full: reward per extra note, high enough to fill every row's quota.
const FULL_REWARD: f64 = 25.0;
/// An extra note in a key used a moment ago (a minijack): more than any
/// reward, so extra notes never make one. Repeated patterns (chordjacks) copy
/// their extra notes on their own.
const W_EXTRA_JACK: f64 = 40.0;
/// Where the source alternates hands (jumptrills, trills, brackets), a note on
/// the hand the previous row used: that hand would run at twice the speed of
/// the other. More than Full's reward, so phases stay on their own hand; the
/// middle key goes with whichever phase gets it first.
const W_HAND_TWICE: f64 = 30.0;
/// Full: reward, in those rows, for mirroring the previous row on
/// the other hand (`OO.` then `.OO`), as 45% of the rows of hand-made 7K
/// jumptrills do.
const W_MIRROR: f64 = 2.0;

// Wide hands (4 keys or more each: 8K 4+4, 9K, 10K and up), measured against
// hand-made 10K maps (see `corpus_feel`). There each hand plays patterns of
// its own: stairs, runs on three of its keys (pinky-ring-middle, ...), open
// chords rather than pills. With 7K's three keys a hand, rows grow into the
// keys next to them instead, as hand conversions do.

/// Wide hands: reward for a hand going on with its stair, to the next key the
/// way it went last.
const W_HAND_ROLL: f64 = 0.7;
/// Wide hands: reward for a hand staying on three next keys for three rows.
const W_HAND_GROUP: f64 = 1.0;
/// Wide hands: the most a note can take off what its hand plays costs (a
/// little more, for rounding).
const HAND_REWARD_MAX: f64 = W_HAND_ROLL + W_HAND_GROUP + 1e-9;
/// Wide hands: a pill (two next keys of one hand) where the source has none.
/// Open chords stay open, as when mappers adapt a 7K pattern to 10K, and
/// pills of the source may stay pills.
const W_PILL: f64 = 1.0;
/// Wide hands, Dense and More dense: the share of the reward for extra notes
/// in each density zone of the source (Break, Calm, Normal, Dense, Peak). The
/// calm parts stay calm and the dense ones get denser.
const ZONE_FILL: [f64; 5] = [0.0, 0.2, 0.6, 1.0, 1.2];
/// Wide hands, Dense and More dense: the share of the reward a second extra
/// note in a row keeps. Two more notes in every row of a stream leave no room
/// for patterns on 10K: they come out as pills or as a checkerboard.
const SECOND_EXTRA_WIDE: f64 = 0.25;

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

/// A note stays on its hand; a key played by either thumb goes either way.
fn same_hand(src: Hand, dst: Hand) -> bool {
    src == Hand::Either || dst == Hand::Either || src == dst
}

/// A hand as a bit: 1 left, 2 right, 0 for a key of either thumb.
fn hand_bit(hand: Hand) -> u8 {
    match hand {
        Hand::Left => 1,
        Hand::Right => 2,
        Hand::Either => 0,
    }
}

/// The hand of each key of a playstyle.
fn hands(layout: &Layout) -> [Hand; MAX_KEYS] {
    let mut hands = [Hand::Either; MAX_KEYS];
    for (hand, finger) in hands.iter_mut().zip(&layout.fingers) {
        *hand = finger.hand;
    }
    hands
}

/// The keys each key shares a finger with, itself included: on 10K8K each
/// finger but the thumb has two keys, on DP the ring and index fingers, and
/// a scratch goes with the pinky of its hand. Up to 10K every key has a
/// finger of its own.
fn shared_fingers(layout: &Layout) -> [u32; MAX_KEYS] {
    let fingers = &layout.fingers;
    let has_pinky = |hand| fingers.iter().any(|f| f.hand == hand && f.digit == Digit::Pinky);
    // Keys of either hand are reached by moving the hand, so none is shared.
    let finger = |f: &Finger| match f.digit {
        _ if f.hand == Hand::Either => None,
        Digit::Scratch if has_pinky(f.hand) => Some((f.player, f.hand, Digit::Pinky)),
        Digit::Scratch => None,
        digit => Some((f.player, f.hand, digit)),
    };
    let mut shared = [0u32; MAX_KEYS];
    for (k, a) in fingers.iter().enumerate() {
        let same = |b: &Finger| finger(a).is_some() && finger(a) == finger(b);
        shared[k] = fingers.iter().enumerate().filter(|&(j, b)| j == k || same(b)).fold(0, |m, (j, _)| m | 1 << j);
    }
    shared
}

/// The keys of a mask, left to right.
fn keys_of(mut mask: u32) -> impl Iterator<Item = u8> {
    std::iter::from_fn(move || {
        (mask != 0).then(|| {
            let key = mask.trailing_zeros() as u8;
            mask &= mask - 1;
            key
        })
    })
}

/// Where a set of keys comes among the sets of its size, ordered as masks:
/// the choice [`tie_break`] gets for it.
fn rank(mask: u32) -> usize {
    let binomial = |n: usize, k: usize| if k > n { 0 } else { (0..k).fold(1, |c, i| c * (n - i) / (i + 1)) };
    keys_of(mask).enumerate().map(|(i, key)| binomial(key as usize, i + 1)).sum()
}

/// The keys each note of a row can go to, left to right: every key it fits
/// in, or on two stages its cheapest ones: [`KEY_CHOICES`] at most, and fewer
/// in rows of many notes, so they have at most [`ROW_CHOICES`] ways to go.
fn choices(costs: &[[f64; MAX_KEYS]], keys: usize) -> Vec<Vec<u8>> {
    let mut choices: Vec<Vec<u8>> =
        costs.iter().map(|c| (0..keys as u8).filter(|&k| c[k as usize].is_finite()).collect()).collect();
    if keys > mania::STAGE_MAX_KEYS {
        for (choice, c) in choices.iter_mut().zip(costs) {
            choice.sort_by(|&a, &b| c[a as usize].total_cmp(&c[b as usize]));
            choice.truncate(KEY_CHOICES);
        }
        // The note with the most keys left loses its dearest one.
        while choices.iter().fold(1usize, |ways, c| ways.saturating_mul(c.len().max(1))) > ROW_CHOICES {
            choices.iter_mut().max_by_key(|c| c.len()).and_then(|c| c.pop());
        }
        for choice in &mut choices {
            choice.sort_unstable();
        }
    }
    choices
}

/// Calls `f` with every way to place notes on keys left to right, each note
/// on one of its `choices` and each finger on one of its keys at most (see
/// [`shared_fingers`]: on 10K2S a scratch and the key next to it), in the
/// order of their masks.
fn each_combo(choices: &[&[u8]], fingers: &[u32; MAX_KEYS], combo: &mut [u8], f: &mut impl FnMut(&[u8])) {
    fn place(
        choices: &[&[u8]],
        fingers: &[u32; MAX_KEYS],
        combo: &mut [u8],
        n: usize,
        below: u8,
        taken: u32,
        f: &mut impl FnMut(&[u8]),
    ) {
        if n == 0 {
            return f(combo);
        }
        for &key in choices[n - 1].iter().take_while(|&&key| key < below) {
            if taken & 1 << key != 0 {
                continue;
            }
            combo[n - 1] = key;
            place(choices, fingers, combo, n - 1, key, taken | fingers[key as usize], f);
        }
    }
    place(choices, fingers, combo, choices.len(), u8::MAX, 0, f);
}

/// How many notes a row of the target can have at once: one per finger.
fn fingers_count(fingers: &[u32; MAX_KEYS], keys: usize) -> usize {
    (0..keys).filter(|&k| fingers[k].trailing_zeros() as usize == k).count()
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
    /// The notes of its source column that would follow it onto its key
    /// (see [`look_ahead`]), each decaying over [`SCRATCH_DECAY_BEATS`]: what
    /// a scratch taking it would get too.
    ahead: f64,
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
    /// Two of its notes on next columns of the same hand (a pill).
    pill: bool,
    /// Two of its notes on next columns, in any hand.
    adjacent: bool,
    /// The pattern, density zone and beat position of the source here.
    pattern: Pattern,
    zone: Zone,
    beat: u8,
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
    /// Wide hands: what each hand (left, right) played last.
    hands: [HandPast; 2],
    /// Per scratch: when it was last hit, and how many notes it had lately
    /// (each decaying over [`SCRATCH_DECAY_BEATS`]) as of then.
    scratch_time: [f64; MAX_KEYS],
    scratch_level: [f64; MAX_KEYS],
    /// Per scratch and the key next to it: when it was last hit, and how
    /// many notes it had lately (decaying over [`PINKY_BALANCE_BEATS`]).
    pinky_time: [f64; MAX_KEYS],
    pinky_level: [f64; MAX_KEYS],
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
struct Decision {
    keys: u32,
    extras: [u8; MAX_EXTRAS],
    n_extras: u8,
}

impl Decision {
    const DROPPED: Self = Self { keys: 0, extras: [0; MAX_EXTRAS], n_extras: 0 };

    /// How many notes of the row are kept.
    fn size(&self) -> usize {
        self.keys.count_ones() as usize
    }
}

/// The last row a hand played (its keys, as a mask of the target), the one
/// before it if linked, and which way the hand went to get there (±1, else 0).
#[derive(Clone, Copy, Default)]
struct HandPast {
    time: f64,
    mask: u32,
    prev: u32,
    dir: i8,
}

/// The middle of the keys of a mask.
fn center(mask: u32) -> f64 {
    let (sum, n) = keys_of(mask).fold((0, 0), |(s, n), k| (s + k as usize, n + 1));
    sum as f64 / n.max(1) as f64
}

/// A row already placed: its source and target columns as bit masks.
#[derive(Clone, Copy)]
struct Past {
    time: f64,
    src: u32,
    dst: u32,
    decision: Decision,
}

#[derive(Clone, Copy)]
struct Ctx<'a> {
    notes: &'a [Note],
    src_keys: usize,
    dst_keys: usize,
    keep_jacks: bool,
    fill: Option<Fill>,
    src_hands: [Hand; MAX_KEYS],
    dst_hands: [Hand; MAX_KEYS],
    /// The keys of the left and right hand of the target, the middle keys
    /// aside, and whether both have 4 keys or more (see [`W_HAND_ROLL`]).
    dst_groups: [u32; 2],
    wide: bool,
    /// The scratches the pinky moves out to (10K2S, 14K2S DP), see [`W_SCRATCH`],
    /// and with them the keys next to them (see [`W_PINKY_BALANCE`]).
    scratches: u32,
    pinkies: u32,
    /// The keys each target key shares a finger with (see [`shared_fingers`]).
    fingers: [u32; MAX_KEYS],
    /// How much a column coming back to its key is spread over the others,
    /// 0 to 1 (see [`SPREAD_FULL`]).
    spread: f64,
}

fn pos(col: usize, keys: usize) -> f64 {
    (col as f64 + 0.5) / keys as f64
}

impl Ctx<'_> {
    fn fresh(&self, state: &State, col: usize, row: &Row) -> f64 {
        let weight = if row.notes.len() > 1 || (row.quota > 0 && self.fill.is_some()) { W_FRESH_CHORDS } else { W_FRESH };
        weight * (-(row.time - state.release[col]) / row.beat_len).exp()
    }

    fn free(&self, state: &State, col: usize, row: &Row, src: u8) -> bool {
        let chained = row.time == state.release[col] && state.last_src[col] == src && state.last_is_ln[col];
        row.time > state.release[col] || chained
    }

    /// Whether the source alternates hands from the previous row to this one
    /// in a jumptrill, trill or bracket.
    fn alternating(&self, state: &State, row: &Row) -> bool {
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
    fn busy_hands(&self, state: &State, row: &Row) -> u8 {
        if !self.alternating(state, row) {
            return 0;
        }
        let prev = state.history[0].dst;
        (0..self.dst_keys).filter(|&c| prev & 1 << c != 0).fold(0, |m, c| m | hand_bit(self.dst_hands[c]))
    }

    /// With extra notes, where the source alternates hands: a reward for a row
    /// mirroring the previous one, the middle key aside.
    fn mirror_cost(&self, state: &State, row: &Row, mask: u32) -> f64 {
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
    fn hand_cost(&self, state: &State, row: &Row, mask: u32) -> f64 {
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
    fn note_costs(&self, state: &State, row: &Row) -> [[f64; MAX_KEYS]; MAX_KEYS] {
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
    fn scratch_cost(&self, state: &State, row: &Row, col: usize, ahead: f64) -> f64 {
        let decay = (-(row.time - state.scratch_time[col]) / (SCRATCH_DECAY_BEATS * row.beat_len)).exp();
        W_SCRATCH_LEVEL * (state.scratch_level[col] * decay + ahead) - W_SCRATCH * SCRATCH_BEAT[row.beat.min(3) as usize]
    }

    /// On a scratch or the key next to it: how many notes more than the other
    /// it had lately, times [`W_PINKY_BALANCE`] (negative when it had fewer).
    fn pinky_cost(&self, state: &State, row: &Row, col: usize) -> f64 {
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
                // The source's own back-and-forths are spread only where the
                // target has room for them (see `spread`).
                cost += weight * if src_repeats { self.spread } else { 1.0 };
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
    fn pick_extras(&self, state: &State, row: &Row, combo: &[u8], extra_costs: &[f64; MAX_KEYS]) -> ([u8; MAX_EXTRAS], u8, f64) {
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
                    let cost = extra_costs[next as usize] + if n > 0 && !fill.scaled { EXTRA_REWARD / 2.0 } else { 0.0 };
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
    fn mask_of(&self, decision: &Decision) -> u32 {
        decision.extras[..decision.n_extras as usize].iter().fold(decision.keys, |m, &col| m | 1 << col)
    }

    /// Whether `decision`, taken from a row with the same source columns, can
    /// place this row too: every key it uses is free.
    fn fits(&self, state: &State, row: &Row, decision: &Decision) -> bool {
        let size = decision.size();
        if size == 0 || size > row.notes.len() {
            return false;
        }
        let notes = &self.notes[row.notes.clone()];
        kept(notes.len(), size)
            .iter()
            .zip(keys_of(decision.keys))
            .all(|(&i, col)| self.free(state, col as usize, row, notes[i].src))
            && decision.extras[..decision.n_extras as usize].iter().all(|&col| row.time > state.release[col as usize])
    }

    fn commit(&self, state: &mut State, row: &Row, decision: &Decision) {
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
    fn template(&self, row: &Row, decision: &Decision, col: u8) -> Option<usize> {
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

/// Adds the extra notes to a placement made without them (`path`), row by row,
/// without moving its notes: each row gets what [`Ctx::pick_extras`] gives
/// it, on keys with no note, on them or on another key of their finger, from
/// a jack of the map's ([`SOURCE_JACK_BEATS`]) before the extra note to one
/// after it: it does not turn a jack of two into three, nor shield a long
/// note, as 15 to 24% of a 10K map's extra notes did with a jack's gap of
/// the converter's own (3/8 beat). A repeated pattern copies the extra notes
/// of the row it repeats.
fn add_extras(ctx: &Ctx, rows: &[Row], path: &[Decision]) -> Vec<Decision> {
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

/// A place a scratch can go (see [`add_scratch_lanes`]).
struct Place {
    time: f64,
    beat_len: f64,
    /// The notes of the map there: none on a 1/2 beat with no note.
    notes: usize,
    /// The first beat of a bar, another beat, or off the 1/2s.
    bar: bool,
    beat: bool,
    offbeat: bool,
    /// How long it is to the next place, in bars.
    bars: f64,
    loudness: Option<f32>,
    /// How strongly a sound starts there in the low, mid and high bands (see
    /// [`ACCENT_LOW`]).
    bands: [f32; 3],
    /// Whether a note of the row has a rare sound (see [`ACCENT_HITSOUND`]).
    hitsound: bool,
    /// Per scratch, how likely a note on it is with what its hand plays
    /// around: 0 where it cannot have one.
    sides: Vec<f64>,
    /// Per scratch, the key of its hand whose note in the row its note would
    /// take (see [`SCRATCH_MOVE`]), as a mask: none for a note of its own.
    takes: Vec<u32>,
    /// The same on a strong accent, where the scratch may take every note
    /// of its hand in the row (see [`SCRATCH_STRONG`]).
    strong_sides: Vec<f64>,
    strong_takes: Vec<u32>,
}

/// What some keys play around a time (see [`add_scratch_lanes`]).
struct Around {
    /// The keys with a short note in the row and with a long one, when the
    /// last of those ends, and whether one holds a note across it.
    row: u32,
    long: u32,
    long_end: f64,
    held: bool,
    /// How long (in ms) since the last of their notes before the row ended,
    /// and until the first one after it starts.
    before: f64,
    after: f64,
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
fn add_scratch_lanes(
    beatmap: &mut Beatmap,
    layout: &Layout,
    inner_keys: usize,
    extras: &[bool],
    density: Density,
    accents: Option<MapAccents>,
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
        let base = SCRATCH_RATE * notes_per_bar.powf(SCRATCH_RATE_POWER) * bars;
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

/// How much each place (in time) sounds like an accent (see [`ACCENT_BAR`]
/// and the next): where it falls in the bar, how big a chord the map plays
/// there and, with the audio, how strongly a sound starts there and how loud
/// it is against the whole song. With the audio, also how strong its sound
/// alone is, `None` without it.
fn accent_scores(places: &[Place], heard: bool) -> Vec<(f64, Option<f64>)> {
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
fn look_ahead(rows: &[Row], notes: &mut [Note], near: bool) {
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
fn beat_strength(time: f64, tp_time: f64, beat_len: f64) -> f64 {
    match patterns::beat_position(time, tp_time, beat_len) {
        0 => 1.0,
        1 => 0.7,
        2 => 0.45,
        _ => 0.25,
    }
}

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
        bail!("the key converter only works on osu!mania maps");
    }

    let src_keys = mania::key_count(beatmap);
    let dst_keys = params.target_keys as usize;
    for keys in [src_keys, dst_keys] {
        if !mania::is_playable(keys) {
            bail!("the key converter supports 1K to 10K and 12K, 14K, 16K and 18K, not {}K", keys);
        }
    }
    if src_keys == dst_keys {
        log::warn!("key converter: the map is already {}K, left as is", dst_keys);
        return Ok(vec![false; beatmap.hit_objects.len()]);
    }

    let grid = BeatGrid::new(beatmap);

    // The patterns and density zones of the source.
    let src_layout = layout::layout(src_keys, None);
    let dst_layout = layout::layout(dst_keys, params.playstyle.as_deref());
    if dst_layout.fingers.iter().any(|f| f.player > 0) {
        bail!("the key converter maps for one player, not {}", dst_layout.name);
    }

    // 10K2S: the map converted to 10K on the inner keys, and the scratches'
    // own notes on top.
    if let Some(inner_keys) = dst_layout.inner_keys() {
        let extras = if src_keys != inner_keys {
            let inner = KeyConverterParams { target_keys: inner_keys as u8, playstyle: None, ..params.clone() };
            convert_marked(beatmap, &inner, None)?
        } else {
            vec![false; beatmap.hit_objects.len()]
        };
        add_scratch_lanes(beatmap, &dst_layout, inner_keys, &extras, params.density, accents);
        return Ok(vec![false; beatmap.hit_objects.len()]);
    }
    let source = Some(patterns::label(beatmap, &src_layout));
    let source_at = |time: f64| -> (Pattern, Zone, u8) {
        let Some(source) = &source else { return (Pattern::Sparse, Zone::Normal, 0) };
        let i = source.rows.partition_point(|r| r.time < time - 0.5);
        match source.rows.get(i) {
            Some(r) if (r.time - time).abs() < 1.0 => (source.labels[i].pattern, source.zones[i], r.beat),
            _ => (Pattern::Sparse, Zone::Normal, 0),
        }
    };

    // Rows: the notes that start at the same time, sorted by column.
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

    let src_hands = hands(&src_layout);
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

    // How much a column coming back to its key is spread over the others
    // (see `SPREAD_FULL`); going down in keys, as before: the target has no
    // room to keep them.
    let spread = if dst_keys < src_keys {
        1.0
    } else {
        let room = (dst_keys - src_keys) as f64 / src_keys as f64;
        ((room - SPREAD_FROM) / (SPREAD_FULL - SPREAD_FROM)).clamp(0.0, 1.0)
    };

    mark_patterns(&mut rows, &notes, params.keep_jacks);
    look_ahead(&rows, &mut notes, spread < 1.0);

    // Extra notes only make sense with more keys.
    let fill = fill_settings(params.density).filter(|_| dst_keys > src_keys);

    // A row has a note per finger at most: on 10K2S, 12 keys but 10 fingers.
    let fingers = shared_fingers(&dst_layout);
    let max_chord = fingers_count(&fingers, dst_keys);
    let scratches = (0..dst_keys)
        .filter(|&k| dst_layout.fingers[k].digit == Digit::Scratch && fingers[k] != 1 << k)
        .fold(0u32, |m, k| m | 1 << k);

    // Going down, each row keeps its size times the key ratio, rounded, at
    // least one note: hand-made maps do it that consistently.
    for row in rows.iter_mut() {
        let k = row.notes.len();
        row.share = if dst_keys < src_keys {
            let size = (k as f64 * dst_keys as f64 / src_keys as f64).round() as usize;
            size.clamp(1, k.min(max_chord)) as u8
        } else {
            k.min(max_chord) as u8
        };
    }

    // Full: each row's chord grows to its size times the key ratio. The
    // fraction is dithered per row, so the average comes out right.
    if fill.is_some_and(|f| f.scaled) {
        let scale = dst_keys as f64 / src_keys as f64 * FULL_SCALE;
        for (i, row) in rows.iter_mut().enumerate() {
            let k = row.notes.len();
            let target = if k == src_keys { k as f64 + FULL_ROW_GROWTH } else { k as f64 * scale };
            // Any choice index no row uses, so it is independent noise. A
            // repeated pattern (a trill, a chordjack) is copied from its first
            // rows, so it takes their size all along. Where the target has few
            // new keys it gets the size its rows should have, not a draw: a
            // 10K trill on 12K (1 note x 1.08) turned into a jumptrill 8% of
            // the time. With more room the draw stays, or 4K -> 7K (x1.575)
            // would double every jack.
            let dither = tie_break(i, u32::MAX as usize) / TIE_BREAK;
            let size = if row.grouped && spread < 1.0 {
                target.round() as usize
            } else {
                (target.floor() + if dither < target.fract() { 1.0 } else { 0.0 }) as usize
            };
            row.quota = size.saturating_sub(k).min(max_chord - k.min(max_chord)).min(MAX_EXTRAS) as u8;
        }
    }
    // The two rows a trill or jumptrill alternates get the same size, so
    // both hands play the same chords.
    for i in 1..rows.len() {
        if rows[i].trill && rows[i - 1].trill && rows[i].copy == 0 {
            rows[i].quota = rows[i - 1].quota;
        }
    }

    // The two hands of the target, the middle keys aside.
    let dst_hands = hands(&dst_layout);
    let hand_mask = |hand: Hand| (0..dst_keys).filter(|&c| dst_hands[c] == hand).fold(0u32, |m, c| m | 1 << c);
    let dst_groups = [hand_mask(Hand::Left), hand_mask(Hand::Right)];
    let wide = dst_groups.iter().all(|g| g.count_ones() >= 4);

    let ctx = Ctx {
        notes: &notes,
        src_keys,
        dst_keys,
        keep_jacks: params.keep_jacks,
        fill,
        src_hands,
        dst_hands,
        dst_groups,
        wide,
        scratches,
        pinkies: keys_of(scratches).fold(scratches, |m, s| m | fingers[s as usize]),
        spread,
        fingers,
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

    let mut targets: Vec<Option<u8>> = vec![None; beatmap.hit_objects.len()];
    let mut extra: Vec<HitObject> = Vec::new();

    for (row, decision) in rows.iter().zip(&path) {
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
        src_keys,
        dst_keys,
        rows.len(),
        dropped,
        added
    );

    Ok(extras)
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
                let x = mania::column_x(col, keys) as i32;
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
        let (x, _) = mania::note(obj).expect("only notes");
        mania::column(x, keys)
    }

    /// Checks no two notes overlap or stack in a key, and returns how many
    /// jacks (1/4 apart or closer) there are.
    fn check(beatmap: &Beatmap, keys: usize) -> usize {
        let mut release = vec![f64::NEG_INFINITY; keys];
        let mut jacks = 0;
        for obj in &beatmap.hit_objects {
            let (x, end) = mania::note(obj).expect("only notes");
            let col = mania::column(x, keys);
            assert!(obj.start_time > release[col], "overlap in key {} at {}", col, obj.start_time);
            if obj.start_time - release[col] <= 500.0 * JACK_BEATS {
                jacks += 1;
            }
            release[col] = end;
        }
        jacks
    }

    fn convert(osu: &str, dst: usize, density: Density) -> (Beatmap, Beatmap) {
        let original = rosu_map::from_str::<Beatmap>(osu).unwrap();
        let mut beatmap = original.clone();
        let params = KeyConverterParams { target_keys: dst as u8, density, keep_jacks: true, playstyle: None };
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
    fn jumptrills_keep_a_hand_per_phase() {
        // A long 4K jumptrill: on 7K each phase stays on its own hand, the
        // middle key with one of them at most, repeating the same two rows.
        let trill: Vec<&[usize]> = (0..32).map(|i| if i % 2 == 0 { &[0, 1][..] } else { &[2, 3][..] }).collect();
        for density in [Density::None, Density::Full] {
            let (_, beatmap) = convert(&rows_map(&trill), 7, density);
            check(&beatmap, 7);
            let rows = key_rows(&beatmap, 7);
            assert!(rows.windows(3).all(|w| w[0] == w[2]), "{:?}", rows);
            let side = |r: &[usize]| r.iter().filter(|&&c| c != 3).fold(0, |m, &c| m | if c < 3 { 1 } else { 2 });
            for w in rows.windows(2) {
                assert!(side(&w[0]) & side(&w[1]) == 0, "{:?}", rows);
                assert!(!(w[0].contains(&3) && w[1].contains(&3)), "{:?}", rows);
            }
        }
    }

    #[test]
    fn every_8k_playstyle_stays_playable() {
        // Every 8K playstyle moves the hands (3+2+3: the middle keys are the
        // thumbs'; 1+7: the first key is a scratch), and all stay playable.
        let mut params = KeyConverterParams { target_keys: 8, density: Density::None, keep_jacks: true, playstyle: None };
        let original = rosu_map::from_str::<Beatmap>(&map(4)).unwrap();
        for playstyle in [None, Some("3+2+3".to_string()), Some("1+7".to_string())] {
            params.playstyle = playstyle;
            let mut beatmap = original.clone();
            apply(&mut beatmap, &params).unwrap();
            check(&beatmap, 8);
        }
    }

    /// Share of the chords with two next keys on one hand of a 10K map.
    fn pill_rate(beatmap: &Beatmap) -> f64 {
        let rows = key_rows(beatmap, 10);
        let chords: Vec<&Vec<usize>> = rows.iter().filter(|r| r.len() > 1).collect();
        let pills = chords.iter().filter(|r| r.windows(2).any(|w| w[1] == w[0] + 1 && (w[0] < 5) == (w[1] < 5))).count();
        pills as f64 / chords.len().max(1) as f64
    }

    #[test]
    fn wide_hands_keep_chords_open() {
        // 7K -> 10K: extra notes no longer grow every chord into a pill (More
        // dense made 99% of them pills), and the plain conversion makes few.
        for (name, density) in [("None", Density::None), ("MoreDense", Density::MoreDense), ("Full", Density::Full)] {
            let (_, beatmap) = convert(&map(7), 10, density);
            check(&beatmap, 10);
            let rate = pill_rate(&beatmap);
            assert!(rate < 0.4, "{}: {:.2}", name, rate);
        }
    }

    #[test]
    fn reducing_stays_playable() {
        for (src, dst) in [(7, 4), (10, 7), (4, 1), (18, 7), (14, 4), (12, 10), (16, 14)] {
            let (_, beatmap) = convert(&map(src), dst, Density::MoreDense);
            assert_eq!(beatmap.circle_size, dst as f32);
            check(&beatmap, dst);
        }
    }

    fn convert_to(osu: &str, playstyle: &str, dst: usize, density: Density) -> Beatmap {
        let mut beatmap = rosu_map::from_str::<Beatmap>(osu).unwrap();
        let params = KeyConverterParams { target_keys: dst as u8, density, keep_jacks: true, playstyle: Some(playstyle.to_string()) };
        apply(&mut beatmap, &params).unwrap();
        beatmap
    }

    #[test]
    fn two_stages_stay_playable() {
        // 12K to 18K in every playstyle of the osu! wiki: every note kept and
        // no new jacks, and with Full no overlaps either. 10K2S adds the
        // scratches' own notes to them, or moves a pinky's out to its scratch.
        for dst in [12, 14, 16, 18] {
            for layout in layout::layouts(dst).iter().filter(|l| l.fingers.iter().all(|f| f.player == 0)) {
                let scratches = layout.inner_keys().map_or(0, |_| layout.digit_mask(Digit::Scratch));
                for src in [4, 7, 10] {
                    let original = rosu_map::from_str::<Beatmap>(&map(src)).unwrap();
                    let beatmap = convert_to(&map(src), layout.name, dst, Density::None);
                    let scratched = beatmap.hit_objects.iter().filter(|o| scratches & 1 << column(o, dst) != 0).count();
                    let kept = beatmap.hit_objects.len() - scratched;
                    let notes = original.hit_objects.len();
                    assert!(kept <= notes && kept + scratched >= notes, "{}K -> {}K {}", src, dst, layout.name);
                    assert!(check(&beatmap, dst) <= check(&original, src), "{}K -> {}K {}", src, dst, layout.name);
                    check(&convert_to(&map(src), layout.name, dst, Density::Full), dst);
                }
            }
        }
    }

    /// Notes a finger plays on another of its keys within a jack's time.
    fn finger_jacks(beatmap: &Beatmap, layout: &Layout) -> usize {
        let keys = layout.keys();
        let shared = shared_fingers(layout);
        let mut release = vec![f64::NEG_INFINITY; keys];
        let mut jacks = 0;
        for obj in &beatmap.hit_objects {
            let (x, end) = mania::note(obj).expect("only notes");
            let col = mania::column(x, keys);
            let gap = |k: u8| obj.start_time - release[k as usize];
            if keys_of(shared[col] & !(1 << col)).any(|k| gap(k) > 0.0 && gap(k) <= 500.0 * JACK_BEATS) {
                jacks += 1;
            }
            release[col] = end;
        }
        jacks
    }

    #[test]
    fn fingers_do_not_jack_on_other_keys() {
        // A finger with two keys (DP, 9K9K, 10K8K) going from one to the
        // other in a stream is a jack too. Hand-made maps do it for 0 to 2.5%
        // of the notes; on this dense map converts did it for 3.5 to 22%
        // before fingers were shared, and for 0.2 to 3.3% after.
        for (dst, name) in [(16, "DP"), (18, "9K9K"), (18, "10K8K")] {
            let layout = layout::layout(dst, Some(name));
            for density in [Density::None, Density::Full] {
                let beatmap = convert_to(&map(7), name, dst, density);
                let rate = finger_jacks(&beatmap, &layout) as f64 / beatmap.hit_objects.len() as f64;
                assert!(rate < 0.04, "{}K {}: {:.3}", dst, name, rate);
            }
        }
    }

    /// Checks no finger plays two of its keys at once (see [`shared_fingers`]):
    /// in one row, or while it holds a long note on the other.
    fn check_fingers(beatmap: &Beatmap, layout: &Layout, what: &str) {
        let keys = layout.keys();
        let shared = shared_fingers(layout);
        let mut release = vec![f64::NEG_INFINITY; keys];
        for obj in &beatmap.hit_objects {
            let (x, end) = mania::note(obj).expect("only notes");
            let col = mania::column(x, keys);
            for k in keys_of(shared[col] & !(1 << col)) {
                assert!(obj.start_time > release[k as usize], "{}: keys {} and {} at {}", what, k, col, obj.start_time);
            }
            release[col] = end;
        }
    }

    /// A 7K full LN map at 120 BPM: rows of one or two notes in 1/4s, each
    /// note held until 1/4 beat before the next of its column, as FLN maps do.
    fn fln_map() -> String {
        let keys = 7;
        let mut seed = 11u64;
        let mut notes: Vec<(i64, usize)> = Vec::new();
        for step in 0..400i64 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            for k in 0..1 + (seed >> 62) as usize % 2 {
                let col = ((seed >> (8 * k + 8)) as usize) % keys;
                if !notes.contains(&(step * 125, col)) {
                    notes.push((step * 125, col));
                }
            }
        }
        let mut osu = String::from(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:7\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
        );
        for &(time, col) in &notes {
            let x = mania::column_x(col, keys) as i32;
            match notes.iter().filter(|&&(t, c)| c == col && t > time).map(|&(t, _)| t).min() {
                Some(next) if next - 125 > time => osu.push_str(&format!("{},192,{},128,0,{}:0:0:0:0:\n", x, time, next - 125)),
                _ => osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", x, time)),
            }
        }
        osu
    }

    /// How many notes each scratch of a 12K map and the key next to it get
    /// against an even key, and the scratch's longest run of notes a beat
    /// apart or closer (at 120 BPM).
    fn scratch_use(beatmap: &Beatmap) -> (f64, f64, usize) {
        let cols = columns(beatmap, 12);
        let share = |a, b| cols.iter().filter(|&&c| c == a || c == b).count() as f64 / 2.0 / cols.len() as f64 * 12.0;
        let (scratch, pinky) = (share(0, 11), share(1, 10));
        let mut longest = 0;
        for s in [0, 11] {
            let times: Vec<f64> = beatmap.hit_objects.iter().filter(|o| column(o, 12) == s).map(|o| o.start_time).collect();
            let mut run = 0;
            for (i, t) in times.iter().enumerate() {
                run = if i > 0 && t - times[i - 1] <= 505.0 { run + 1 } else { 1 };
                longest = longest.max(run);
            }
        }
        (scratch, pinky, longest)
    }

    #[test]
    fn scratches_are_accents() {
        // 10K2S from 4K, 7K, 10K and 7K FLN: the scratches get about as many
        // notes as a key, and the pinky shares its notes with the key next to
        // them, but never for bars on end. With FLN a column can leave the
        // scratch when its long note ends.
        let sources = [("4K", map(4)), ("7K", map(7)), ("10K", map(10)), ("7K FLN", fln_map())];
        for (src, osu) in &sources {
            for (name, density) in [("None", Density::None), ("MoreDense", Density::MoreDense), ("Full", Density::Full)] {
                let beatmap = convert_to(osu, "12K", 12, density);
                let (share, pinky, longest) = scratch_use(&beatmap);
                assert!(share >= 0.35, "{} -> 12K {}: {:.2}x a key", src, name, share);
                assert!((0.6..=1.6).contains(&(share / pinky)), "{} -> 12K {}: {:.2}x a key, the key next to it {:.2}x", src, name, share, pinky);
                assert!(longest <= 8, "{} -> 12K {}: a run of {}", src, name, longest);
            }
        }
    }

    #[test]
    fn near_key_counts_keep_the_source_patterns() {
        // 10K -> 12K has 2 new keys: a 10K column coming back two notes later
        // stays on its key, as its mapper meant it, instead of rolling over
        // the keys next to it. Two columns take turns in 1/2s at 174 BPM
        // (osu! rounds them to 172 or 173 ms), with a chord on every beat
        // pair, so they are no trill to copy.
        let beat = 60000.0 / 174.0;
        let mut osu = format!(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,{},4,1,0,60,1,0\n\n[HitObjects]\n",
            beat
        );
        let mut odd = Vec::new();
        for step in 0..48 {
            let time = (step as f64 * beat / 2.0).round();
            let cols: &[usize] = match step % 4 {
                0 => &[1, 8],
                1 | 3 => &[4],
                _ => &[1],
            };
            if step % 2 == 1 {
                odd.push(time);
            }
            for &col in cols {
                osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", mania::column_x(col, 10) as i32, time));
            }
        }
        for density in [Density::None, Density::Full] {
            let (_, beatmap) = convert(&osu, 12, density);
            // The keys column 4 went to: mostly one.
            let mut keys = [0usize; 12];
            for obj in beatmap.hit_objects.iter().filter(|o| odd.contains(&o.start_time)) {
                keys[column(obj, 12)] += 1;
            }
            let most = keys.iter().max().copied().unwrap_or(0);
            assert!(most * 10 >= odd.len() * 8, "{:?}", keys);
        }
    }

    #[test]
    fn near_key_counts_keep_columns_by_the_scratch() {
        // 10K -> 12K: a column by the edge comes back every beat, taking turns
        // with another, with a chord now and then. Its notes stay on one key
        // rather than going to the scratch for a few and then to the keys
        // around it as the scratch's costs and the pinky's balance change.
        let beat = 60000.0 / 174.0;
        let mut osu = format!(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,{},4,1,0,60,1,0\n\n[HitObjects]\n",
            beat
        );
        let mut edge = Vec::new();
        for step in 0..48 {
            let time = (step as f64 * beat / 2.0).round();
            let cols: &[usize] = match step % 8 {
                1 | 3 => &[0, 8],
                s if s % 2 == 1 => &[8],
                _ => &[6],
            };
            if step % 2 == 1 {
                edge.push(time);
            }
            for &col in cols {
                osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", mania::column_x(col, 10) as i32, time));
            }
        }
        for density in [Density::None, Density::Full] {
            let (_, beatmap) = convert(&osu, 12, density);
            let mut keys = [0usize; 12];
            for obj in beatmap.hit_objects.iter().filter(|o| edge.contains(&o.start_time)) {
                let key = column(obj, 12);
                // The chords' other note goes to the left; this is the right.
                if key >= 6 {
                    keys[key] += 1;
                }
            }
            let most = keys.iter().max().copied().unwrap_or(0);
            assert!(most * 10 >= edge.len() * 8, "{:?}", keys);
        }
    }

    /// The notes of a map as (start, end, key), in time.
    fn notes_of(beatmap: &Beatmap, keys: usize) -> Vec<(f64, f64, usize)> {
        beatmap.hit_objects.iter().map(|o| (o.start_time, mania::note(o).unwrap().1, column(o, keys))).collect()
    }

    fn notes_per_second(beatmap: &Beatmap) -> f64 {
        let first = beatmap.hit_objects.first().map_or(0.0, |o| o.start_time);
        let last = beatmap.hit_objects.last().map_or(0.0, |o| o.start_time);
        beatmap.hit_objects.len() as f64 / ((last - first) / 1000.0).max(1.0)
    }

    /// Checks that no scratch of a 10K2S map has a note of its hand (the five
    /// keys next to it) in its row or held across it, a muri, nor closer than
    /// [`SCRATCH_FREE_MS`] lets it for a map as hard as `inner`, the 10K map
    /// under the scratches. Returns how soon after a scratch its hand plays
    /// again at the soonest, in ms.
    fn check_scratch_hands(beatmap: &Beatmap, inner: &Beatmap, what: &str) -> f64 {
        let free_ms = scratch_free_ms(notes_per_second(inner));
        let (after, before) = (free_ms * SCRATCH_FREE_AFTER[0].0, free_ms * SCRATCH_FREE_BEFORE[0].0);
        let notes = notes_of(beatmap, 12);
        let mut soonest = f64::INFINITY;
        for &(time, release, scratch) in notes.iter().filter(|n| matches!(n.2, 0 | 11)) {
            let hand = if scratch == 0 { 1..=5 } else { 6..=10 };
            for &(start, end, key) in notes.iter().filter(|n| hand.contains(&n.2)) {
                assert!((start - time).abs() > 1.0, "{}: a muri, the scratch at {} with key {}", what, time, key);
                assert!(!(start < time && end > time), "{}: a muri, the scratch at {} while key {} holds", what, time, key);
                assert!(!(start > time && start < release), "{}: key {} while the scratch at {} holds", what, key, time);
                if start > time {
                    assert!(start - time >= after, "{}: key {} {} ms after the scratch at {}", what, key, start - time, time);
                    soonest = soonest.min(start - time);
                } else {
                    assert!(time - end >= before, "{}: key {} {} ms before the scratch at {}", what, key, time - end, time);
                }
            }
        }
        soonest
    }

    #[test]
    fn ten_k_two_s_is_a_10k_map_and_scratches() {
        // 10K2S: the 10K map as it is on the inner keys but for the notes the
        // scratches took (their hand's one in a chord, or their pinky's, and
        // on the strong accents all of them, one moved out to the scratch),
        // and notes of their own on the scratches, plain, never with the key
        // of their pinky nor 3/8 beat around it, mostly on the beats. Their
        // hand is free: nothing in their row, nothing held across it.
        let layout = layout::layout(12, Some("10K2S"));
        let original = rosu_map::from_str::<Beatmap>(&map(10)).unwrap();
        let sorted = |mut notes: Vec<(f64, f64, usize)>| {
            notes.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.cmp(&b.2)));
            notes
        };
        let original_notes = sorted(notes_of(&original, 10));
        for density in [Density::None, Density::Full] {
            let beatmap = convert_to(&map(10), "10K2S", 12, density);
            check(&beatmap, 12);
            check_fingers(&beatmap, &layout, "10K -> 10K2S");
            check_scratch_hands(&beatmap, &original, "10K -> 10K2S");
            let notes = notes_of(&beatmap, 12);
            let inner: Vec<(f64, f64, usize)> =
                notes.iter().filter(|n| n.2 != 0 && n.2 != 11).map(|&(s, e, k)| (s, e, k - 1)).collect();
            let scratch: Vec<&(f64, f64, usize)> = notes.iter().filter(|n| n.2 == 0 || n.2 == 11).collect();
            // The notes of the map not on their key: ones a scratch of their
            // hand took, at their time.
            assert!(inner.iter().all(|n| original_notes.contains(n)));
            let mut taken = Vec::new();
            for &(start, _, key) in original_notes.iter().filter(|n| !inner.contains(n)) {
                let side = if key < 5 { 0 } else { 11 };
                let full = density == Density::Full;
                assert!(
                    scratch.iter().any(|n| n.2 == side && n.0 == start),
                    "full {}: the note at {} on key {} went without a scratch",
                    full,
                    start,
                    key
                );
                taken.push((start, side));
            }
            // 25 bars at 120 BPM of rows on every 1/4, mostly on both hands,
            // and no audio.
            assert!(scratch.len() >= 10, "{} scratch notes", scratch.len());
            for &&(start, end, key) in &scratch {
                assert!(original_notes.iter().any(|n| n.0 == start), "a scratch at {}, where the map has no note", start);
                // A long one is one of the map's, moved out.
                let hand = if key == 0 { 0..5 } else { 5..10 };
                assert!(
                    start == end || original_notes.iter().any(|n| n.0 == start && n.1 == end && hand.contains(&n.2)),
                    "a long scratch at {} of its own",
                    start
                );
                let pinky = if key == 0 { 1 } else { 10 };
                let gap = 500.0 * JACK_BEATS;
                assert!(
                    notes.iter().filter(|n| n.2 == pinky).all(|&(s, e, _)| s - start > gap || start - e > gap),
                    "a scratch at {} next to its pinky's key",
                    start
                );
            }
            // Hand-made maps: 3% on the 1/4s and 2% elsewhere.
            let on_half = scratch.iter().filter(|n| (n.0 / 250.0 - (n.0 / 250.0).round()).abs() < 0.01).count();
            assert!(on_half * 10 >= scratch.len() * 9, "{} of {} on the 1/2s", on_half, scratch.len());
            // Those of their own, plain: the hit sound alone.
            for obj in beatmap.hit_objects.iter().filter(|o| matches!(column(o, 12), 0 | 11)) {
                if !taken.contains(&(obj.start_time, column(obj, 12))) {
                    assert!(obj.samples.iter().all(|s| s.name == HitSampleInfo::HIT_NORMAL), "{:?}", obj.samples);
                }
            }
        }
        // From 7K, the inner keys hold the 7K map converted to 10K: every note
        // of it but the ones the scratches took, and the density's extra
        // notes but those in a scratch's way.
        let original = rosu_map::from_str::<Beatmap>(&map(7)).unwrap();
        let (_, ten) = convert(&map(7), 10, Density::Full);
        let beatmap = convert_to(&map(7), "10K2S", 12, Density::Full);
        check_scratch_hands(&beatmap, &ten, "7K -> 10K2S");
        let ten = notes_of(&ten, 10);
        let notes = notes_of(&beatmap, 12);
        let inner: Vec<(f64, f64, usize)> =
            notes.iter().filter(|n| n.2 != 0 && n.2 != 11).map(|&(s, e, k)| (s, e, k - 1)).collect();
        let scratched = notes.len() - inner.len();
        assert!(inner.iter().all(|n| ten.contains(n)));
        assert!(inner.len() + scratched >= original.hit_objects.len());
        assert!(inner.len() > ten.len() * 8 / 10, "{} of {}", inner.len(), ten.len());
    }

    #[test]
    fn ten_k_two_s_marks_the_big_chords() {
        // A 10K stream on the index fingers with a chord on the second beat
        // of every bar, of the left pinky and the right middle finger and
        // pinky: the chords are the accents. Their left scratch takes the
        // pinky's note, sounds and all, the hand having nothing else there;
        // the right hand keeps its two notes.
        let (beat, bars) = (500.0, 16usize);
        let mut osu = String::from(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
        );
        for q in 0..bars * 16 {
            let time = q as f64 * beat / 4.0;
            let (cols, sound): (&[usize], u8) = if q % 16 == 4 { (&[0, 7, 9], 2) } else { (&[3 + 3 * (q % 2)], 0) };
            for &col in cols {
                osu.push_str(&format!("{},192,{},1,{},0:0:0:0:\n", mania::column_x(col, 10) as i32, time, sound));
            }
        }
        let original = rosu_map::from_str::<Beatmap>(&osu).unwrap();
        let beatmap = convert_to(&osu, "10K2S", 12, Density::None);
        check(&beatmap, 12);
        check_fingers(&beatmap, &layout::layout(12, Some("10K2S")), "10K -> 10K2S");
        check_scratch_hands(&beatmap, &original, "10K -> 10K2S");
        let chords: Vec<f64> = (0..bars).map(|bar| (bar as f64 * 4.0 + 1.0) * beat).collect();
        let at = |time: f64, col: usize| beatmap.hit_objects.iter().find(|o| o.start_time == time && column(o, 12) == col);
        let mut scratched = 0;
        for &time in &chords {
            let Some(note) = at(time, 0) else { continue };
            scratched += 1;
            assert!((1..=5).all(|k| at(time, k).is_none()), "the scratch at {} has its hand's notes too", time);
            assert!([8, 10].iter().all(|&k| at(time, k).is_some()), "the chord at {} lost its right hand", time);
            assert!(note.samples.iter().any(|s| s.name == HitSampleInfo::HIT_WHISTLE), "{:?}", note.samples);
        }
        assert!(scratched * 10 >= chords.len() * 9, "{} of {} chords", scratched, chords.len());
        // The rest of the map is as it was: every note on its key, or at a
        // scratch of its hand.
        let scratch_at = |time: f64, side: usize| beatmap.hit_objects.iter().any(|o| o.start_time == time && column(o, 12) == side);
        for o in &original.hit_objects {
            let key = column(o, 10);
            let on_key = beatmap.hit_objects.iter().any(|n| n.start_time == o.start_time && column(n, 12) == key + 1);
            assert!(on_key || scratch_at(o.start_time, if key < 5 { 0 } else { 11 }), "the note at {} on key {} went", o.start_time, key);
        }
    }

    #[test]
    fn ten_k_two_s_frees_the_hand_by_difficulty() {
        // 10K maps at 120 BPM, an easy one (a note a beat, the hands taking
        // turns) and a hard one (jumps on every 1/4), each with a chord on
        // every bar. No scratch has its hand's notes with it, and the easy
        // map's hand stays free for longer after it: past the other hand's
        // next note at least. The hard map's comes back on the next notes.
        let header = "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n";
        let note = |col: usize, time: f64| format!("{},192,{},1,0,0:0:0:0:\n", mania::column_x(col, 10) as i32, time);
        let mut easy = String::from(header);
        for b in 0..256usize {
            let cols: Vec<usize> = if b % 4 == 0 { vec![1, 3, 6, 8] } else if b % 2 == 0 { vec![1 + b % 3] } else { vec![6 + b % 3] };
            for col in cols {
                easy.push_str(&note(col, b as f64 * 500.0));
            }
        }
        let mut hard = String::from(header);
        for q in 0..512usize {
            let cols: Vec<usize> = if q % 16 == 0 { vec![0, 2, 7, 9] } else { vec![1 + q % 3, 8 - q % 3] };
            for col in cols {
                hard.push_str(&note(col, q as f64 * 125.0));
            }
        }
        let layout = layout::layout(12, Some("10K2S"));
        let mut soonest = Vec::new();
        for (name, osu) in [("easy", &easy), ("hard", &hard)] {
            let original = rosu_map::from_str::<Beatmap>(osu).unwrap();
            let beatmap = convert_to(osu, "10K2S", 12, Density::None);
            check(&beatmap, 12);
            check_fingers(&beatmap, &layout, name);
            let scratches = beatmap.hit_objects.iter().filter(|o| matches!(column(o, 12), 0 | 11)).count();
            assert!(scratches >= 10, "{}: {} scratch notes", name, scratches);
            soonest.push(check_scratch_hands(&beatmap, &original, name));
        }
        assert!(soonest[0] >= 500.0 - 1.0, "easy: the hand back {} ms after a scratch", soonest[0]);
        assert!(soonest[1] <= 250.0 + 1.0, "hard: the hand back {} ms after a scratch at the soonest", soonest[1]);
    }

    /// Checks that no span of a 10K2S map has more scratches than
    /// [`SCRATCH_SPANS`] lets it for a map as hard as `inner`, the 10K map
    /// under the scratches. Returns the most there are in any 8 beats.
    fn check_scratch_spans(beatmap: &Beatmap, inner: &Beatmap, beat_len: f64, what: &str) -> usize {
        let nps = notes_per_second(inner);
        let mut times: Vec<f64> =
            beatmap.hit_objects.iter().filter(|o| matches!(column(o, 12), 0 | 11)).map(|o| o.start_time).collect();
        times.dedup();
        let busiest = |beats: f64| {
            (0..times.len()).map(|i| times[i..].iter().take_while(|&&t| t - times[i] <= beats * beat_len + 1.0).count()).max()
        };
        for (beats, a, b) in SCRATCH_SPANS {
            let most = scratch_most(nps, a, b);
            let found = busiest(beats).unwrap_or(0);
            assert!(found <= most, "{}: {} scratches in {} beats, {} at most at {:.1} notes a second", what, found, beats, most, nps);
        }
        busiest(8.0).unwrap_or(0)
    }

    #[test]
    fn ten_k_two_s_rests_by_difficulty() {
        // An easy 10K map and a hard one at 120 BPM (a note a beat, and rows
        // of one to three notes on every 1/4), with no density and with
        // Full: the easy map gets fewer scratches close together, and with
        // Full no fewer notes than without, its scratches added to the map's
        // notes rather than taking them.
        let header = "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n";
        let mut easy = String::from(header);
        for b in 0..256usize {
            let cols: Vec<usize> = if b % 4 == 0 { vec![1, 3, 6, 8] } else { vec![[2, 7, 1, 8, 3, 6][b % 6]] };
            for col in cols {
                easy.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", mania::column_x(col, 10) as i32, b as f64 * 500.0));
            }
        }
        let mut busiest = Vec::new();
        for (name, osu) in [("easy", easy), ("hard", map(10))] {
            let original = rosu_map::from_str::<Beatmap>(&osu).unwrap();
            let none = convert_to(&osu, "10K2S", 12, Density::None);
            let full = convert_to(&osu, "10K2S", 12, Density::Full);
            for beatmap in [&none, &full] {
                check(beatmap, 12);
            }
            assert!(full.hit_objects.len() >= none.hit_objects.len(), "{}: Full has fewer notes", name);
            let scratches = none.hit_objects.iter().filter(|o| matches!(column(o, 12), 0 | 11)).count();
            assert!(scratches >= 10, "{}: {} scratch notes", name, scratches);
            busiest.push(check_scratch_spans(&none, &original, 500.0, name).max(check_scratch_spans(&full, &original, 500.0, name)));
        }
        assert!(busiest[0] < busiest[1], "{:?}", busiest);
    }

    #[test]
    fn ten_k_two_s_follows_the_sounds() {
        // A 10K map of a note a beat on the thumbs, over a quiet song with a
        // click on the second beat of every other bar: the scratches go on
        // the clicks.
        let (beat, bars) = (500.0, 32usize);
        let mut osu = String::from(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
        );
        for b in 0..bars * 4 {
            osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", mania::column_x(4 + b % 2, 10) as i32, b as f64 * beat));
        }
        let clicks: Vec<f64> = (0..bars).step_by(2).map(|bar| (bar as f64 * 4.0 + 1.0) * beat).collect();
        let sample_rate = 44_100.0;
        let mut samples = vec![0.0f32; ((bars as f64 * 4.0 * beat / 1000.0 + 1.0) * sample_rate) as usize];
        let mut seed = 3u32;
        let mut noise = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1 << 24) as f32 - 0.5
        };
        for sample in samples.iter_mut() {
            *sample = noise() * 0.002;
        }
        for &click in &clicks {
            let first = (click / 1000.0 * sample_rate) as usize;
            for sample in &mut samples[first..first + (0.03 * sample_rate) as usize] {
                *sample = noise() * 0.5;
            }
        }
        let accents = crate::commands::trainer::accents::Accents::from_mono(&samples, sample_rate);
        let mut beatmap = rosu_map::from_str::<Beatmap>(&osu).unwrap();
        let params =
            KeyConverterParams { target_keys: 12, density: Density::None, keep_jacks: true, playstyle: Some("10K2S".to_string()) };
        apply_with(&mut beatmap, &params, Some(accents.at_rate(1.0))).unwrap();
        check_fingers(&beatmap, &layout::layout(12, Some("10K2S")), "10K -> 10K2S");
        let scratches: Vec<f64> =
            beatmap.hit_objects.iter().filter(|o| matches!(column(o, 12), 0 | 11)).map(|o| o.start_time).collect();
        let heard = clicks.iter().filter(|&&c| scratches.iter().any(|&t| (t - c).abs() < 1.0)).count();
        assert!(heard * 10 >= clicks.len() * 9, "{} of {} clicks: {:?}", heard, clicks.len(), scratches);
        // Without the audio they go on the first beats of the bars instead.
        let mut beatmap = rosu_map::from_str::<Beatmap>(&osu).unwrap();
        apply(&mut beatmap, &params).unwrap();
        let unheard: Vec<f64> =
            beatmap.hit_objects.iter().filter(|o| matches!(column(o, 12), 0 | 11)).map(|o| o.start_time).collect();
        let on_bars = unheard.iter().filter(|&&t| t % (4.0 * beat) == 0.0).count();
        assert!(!unheard.is_empty() && on_bars * 10 >= unheard.len() * 8, "{} of {} on the bars", on_bars, unheard.len());
    }

    #[test]
    fn ten_k_two_s_marks_the_strong_sounds() {
        // A 10K map of a note a beat on the thumbs over a song of a click
        // every 1/2 beat, quiet for 16 bars and loud for 16: the scratches
        // mark the loud ones alone, as many as the map asks for or not.
        let (beat, bars) = (500.0, 32usize);
        let mut osu = String::from(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
        );
        for b in 0..bars * 4 {
            osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", mania::column_x(4 + b % 2, 10) as i32, b as f64 * beat));
        }
        let sample_rate = 44_100.0;
        let mut samples = vec![0.0f32; ((bars as f64 * 4.0 * beat / 1000.0 + 1.0) * sample_rate) as usize];
        let mut seed = 9u32;
        let mut noise = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1 << 24) as f32 - 0.5
        };
        for sample in samples.iter_mut() {
            *sample = noise() * 0.002;
        }
        for half in 0..bars * 8 {
            let first = (half as f64 * beat / 2000.0 * sample_rate) as usize;
            let volume = if half < bars * 4 { 0.03 } else { 0.5 };
            for sample in &mut samples[first..first + (0.03 * sample_rate) as usize] {
                *sample = noise() * volume;
            }
        }
        let accents = crate::commands::trainer::accents::Accents::from_mono(&samples, sample_rate);
        let mut beatmap = rosu_map::from_str::<Beatmap>(&osu).unwrap();
        let params =
            KeyConverterParams { target_keys: 12, density: Density::Full, keep_jacks: true, playstyle: Some("10K2S".to_string()) };
        apply_with(&mut beatmap, &params, Some(accents.at_rate(1.0))).unwrap();
        check_fingers(&beatmap, &layout::layout(12, Some("10K2S")), "10K -> 10K2S");
        let scratches: Vec<f64> =
            beatmap.hit_objects.iter().filter(|o| matches!(column(o, 12), 0 | 11)).map(|o| o.start_time).collect();
        let loud = scratches.iter().filter(|&&t| t >= bars as f64 * 2.0 * beat - 1.0).count();
        assert!(scratches.len() >= 10 && loud == scratches.len(), "{} of {} in the loud half: {:?}", loud, scratches.len(), scratches);
        // On the map's notes, the beats, not on the clicks between them.
        assert!(scratches.iter().all(|&t| (t / beat).fract() == 0.0), "{:?}", scratches);
    }

    #[test]
    fn ten_k_two_s_moves_the_accent_sounds_out() {
        // A 10K map of a note every 1/2 beat on the right hand, with a water
        // drop of its own on the left middle finger's key 1/2 beat into every
        // 8th bar, and a hi-hat on every beat of a bar: the water drops go to
        // the left scratch, sound and all, and the hi-hats, a part to play,
        // stay where they are.
        let (beat, bars) = (500.0, 32usize);
        let mut osu = String::from(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
        );
        let mut drops = Vec::new();
        for half in 0..bars * 8 {
            let time = half as f64 * beat / 2.0;
            let (col, sample) = if half % 64 == 1 {
                drops.push(time);
                (2, "water-drop.wav")
            } else if half / 8 == 21 && half % 2 == 0 {
                (7, "hihat.wav")
            } else {
                ([6, 8][half % 2], "")
            };
            osu.push_str(&format!("{},192,{},1,0,0:0:0:0:{}\n", mania::column_x(col, 10) as i32, time, sample));
        }
        let beatmap = convert_to(&osu, "10K2S", 12, Density::None);
        check(&beatmap, 12);
        let at = |time: f64, col: usize| beatmap.hit_objects.iter().find(|o| o.start_time == time && column(o, 12) == col);
        let sample = |o: &HitObject, name: &str| o.samples.iter().any(|s| format!("{:?}", s.name).contains(name));
        let moved = drops.iter().filter(|&&t| at(t, 0).is_some_and(|o| sample(o, "water-drop"))).count();
        assert!(moved * 10 >= drops.len() * 8, "{} of {} water drops on the scratch", moved, drops.len());
        let hats = beatmap.hit_objects.iter().filter(|o| sample(o, "hihat")).collect::<Vec<_>>();
        assert!(hats.len() == 4 && hats.iter().all(|o| column(o, 12) == 8), "{:?}", hats.iter().map(|o| column(o, 12)).collect::<Vec<_>>());
    }

    #[test]
    fn near_key_counts_add_notes_around_the_map() {
        // 10K -> 12K: the denser fills place the map's notes where the plain
        // conversion does, so its streams keep their direction, and add their
        // notes around them, still playable, not next to a note of their row
        // where the source has none next to each other, and not within a
        // jack of the map's (1/2 beat) of a note on their key: no jack of two
        // turns into three, no long note gets a shield.
        let (original, plain) = convert(&map(10), 12, Density::None);
        let adjacent = |r: &[usize]| r.windows(2).any(|w| w[1] == w[0] + 1);
        let source = key_rows(&original, 10);
        let rows = key_rows(&plain, 12);
        let times: Vec<f64> = {
            let mut t: Vec<f64> = plain.hit_objects.iter().map(|o| o.start_time).collect();
            t.dedup();
            t
        };
        for density in [Density::Dense, Density::MoreDense, Density::Full] {
            let (_, beatmap) = convert(&map(10), 12, density);
            assert!(beatmap.hit_objects.len() > plain.hit_objects.len());
            check(&beatmap, 12);
            check_fingers(&beatmap, &layout::layout(12, None), "10K -> 12K");
            for ((time, row), src) in times.iter().zip(&rows).zip(&source) {
                let mut keys: Vec<usize> =
                    beatmap.hit_objects.iter().filter(|o| o.start_time == *time).map(|o| column(o, 12)).collect();
                keys.sort();
                assert!(row.iter().all(|k| keys.contains(k)), "at {}: {:?} then {:?}", time, row, keys);
                if keys.len() > row.len() && !adjacent(src) && !adjacent(row) {
                    assert!(!adjacent(&keys), "at {}: {:?} from {:?}", time, keys, src);
                }
            }
            let placed: Vec<(f64, f64, usize)> = plain
                .hit_objects
                .iter()
                .map(|o| (o.start_time, mania::note(o).unwrap().1, column(o, 12)))
                .collect();
            let gap = 500.0 * SOURCE_JACK_BEATS;
            for obj in &beatmap.hit_objects {
                let (start, end, key) = (obj.start_time, mania::note(obj).unwrap().1, column(obj, 12));
                if placed.contains(&(start, end, key)) {
                    continue;
                }
                let near = placed.iter().any(|&(s, e, k)| k == key && ((0.0..=gap).contains(&(start - e)) || (0.0..=gap).contains(&(s - end))));
                assert!(!near, "an extra note at {} in key {} next to a note of the map", start, key);
            }
        }
    }

    #[test]
    fn near_key_counts_keep_trills_of_single_notes() {
        // A 10K trill of single notes on 12K: Full (x1.08) keeps it single
        // notes instead of drawing a jumptrill for the whole of it, and the
        // denser fills add their notes open, not as pills next to the trill.
        let beat = 60000.0 / 174.0;
        let mut osu = format!(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,{},4,1,0,60,1,0\n\n[HitObjects]\n",
            beat
        );
        for (trill, cols) in [(0, [5, 8]), (1, [6, 9]), (2, [1, 3]), (3, [2, 7])] {
            for step in 0..12 {
                let time = ((trill * 16 + step) as f64 * beat / 2.0).round();
                osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", mania::column_x(cols[step % 2], 10) as i32, time));
            }
        }
        let (_, beatmap) = convert(&osu, 12, Density::Full);
        let rows = key_rows(&beatmap, 12);
        assert!(rows.iter().all(|r| r.len() == 1), "{:?}", rows);
        for density in [Density::Dense, Density::MoreDense] {
            let (_, beatmap) = convert(&osu, 12, density);
            let rows = key_rows(&beatmap, 12);
            let pills = rows.iter().filter(|r| r.windows(2).any(|w| w[1] == w[0] + 1 && (w[0] < 6) == (w[1] < 6))).count();
            assert!(pills * 10 <= rows.len(), "{} pills: {:?}", pills, rows);
        }
    }

    #[test]
    fn a_finger_plays_one_key_at_a_time() {
        // 10K2S: the pinky plays the scratch and the key next to it, so the
        // first two keys (and the last two) never have notes at once. The same
        // goes for the fingers with two keys on DP, 9K9K and 10K8K.
        let densities = [("None", Density::None), ("MoreDense", Density::MoreDense), ("Full", Density::Full)];
        for dst in [12, 14, 16, 18] {
            for layout in layout::layouts(dst).iter().filter(|l| l.fingers.iter().all(|f| f.player == 0)) {
                for src in [4, 7, 10] {
                    for (name, density) in densities {
                        let beatmap = convert_to(&map(src), layout.name, dst, density);
                        check_fingers(&beatmap, layout, &format!("{}K -> {}K {} {}", src, dst, layout.name, name));
                    }
                }
            }
        }
    }

    #[test]
    fn shared_fingers_follow_the_playstyle() {
        for keys in 1..=10 {
            for layout in layout::layouts(keys) {
                let shared = shared_fingers(&layout);
                assert!((0..keys).all(|k| shared[k] == 1 << k), "{}K {}", keys, layout.name);
            }
        }
        let shared = |keys, name| shared_fingers(&layout::layout(keys, Some(name)));
        assert_eq!(shared(18, "10K8K")[0], 1 | 1 << 4);
        assert_eq!(shared(14, "DP")[1], 0b110);
        // The scratch goes with the pinky; the middle keys of EZ2AC with no one.
        assert_eq!(shared(12, "10K2S")[11], 0b11 << 10);
        assert_eq!(shared(14, "EZ2AC")[6], 1 << 6);
    }

    #[test]
    fn converts_only_to_what_osu_plays() {
        let mut beatmap = rosu_map::from_str::<Beatmap>(&map(7)).unwrap();
        for (keys, playstyle) in [(11, None), (20, None), (14, Some("Co-op"))] {
            let params = KeyConverterParams { target_keys: keys, density: Density::None, keep_jacks: true, playstyle: playstyle.map(String::from) };
            assert!(apply(&mut beatmap, &params).is_err(), "{}K {:?}", keys, playstyle);
        }
    }

    #[test]
    fn combos_go_in_the_order_of_their_masks() {
        let own: [u32; MAX_KEYS] = std::array::from_fn(|k| 1 << k);
        let mut seen = Vec::new();
        each_combo(&[&[0, 1, 2], &[1, 2, 3]], &own, &mut [0; 2], &mut |c| seen.push(c.to_vec()));
        assert_eq!(seen, [[0, 1], [0, 2], [1, 2], [0, 3], [1, 3], [2, 3]]);
        // Keys 0 and 1 of one finger (a scratch and its pinky) never go together.
        let mut shared = own;
        shared[0] = 0b11;
        shared[1] = 0b11;
        seen.clear();
        each_combo(&[&[0, 1, 2], &[1, 2, 3]], &shared, &mut [0; 2], &mut |c| seen.push(c.to_vec()));
        assert_eq!(seen, [[0, 2], [1, 2], [0, 3], [1, 3], [2, 3]]);
        assert_eq!(fingers_count(&shared, 4), 3);
        assert_eq!(fingers_count(&shared_fingers(&layout::layout(12, Some("10K2S"))), 12), 10);
        // And their tie breaks are the ones of the list of every mask.
        let mut next = [0usize; 11];
        for mask in 1u32..1 << 10 {
            let size = mask.count_ones() as usize;
            assert_eq!(rank(mask), next[size], "{:b}", mask);
            next[size] += 1;
        }
    }
}
