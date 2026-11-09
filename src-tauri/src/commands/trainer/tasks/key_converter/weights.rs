//! The weights of the costs the placement is searched on, and its limits:
//! what a good pattern is, in numbers (see the module docs).

pub(super) const BEAM_WIDTH: usize = 8;

/// On two stages (12K to 18K), the keys each note of a row tries, its
/// cheapest. With 6, converts to 14K and 18K came out the same as trying
/// every key, but 4 to 9 times faster.
pub(super) const KEY_CHOICES: usize = 6;

/// On two stages, the most ways a row tries to place its notes: rows of up to
/// 4 notes try all their keys, bigger ones (12 notes on 12K) fewer per note.
pub(super) const ROW_CHOICES: usize = KEY_CHOICES.pow(4);

/// Notes in the same column closer than this (in beats) are a jack, as in the
/// original: a 1/4 with some leniency. New ones are avoided.
pub(super) const JACK_BEATS: f64 = 0.375;

/// The same column in consecutive source rows up to this far apart (in beats)
/// is a jack of the map, kept on one key: fast maps jack at 1/2.
pub(super) const SOURCE_JACK_BEATS: f64 = 0.55;

/// Rows further apart than this (in beats) are not compared for flow.
pub(super) const FLOW_BEATS: f64 = 2.0;

pub(super) const COST_DROP: f64 = 150.0;

/// Going down in keys, dropping one more note than the row's share: cheaper
/// than a new jack, so the map thins out instead of jacking.
pub(super) const COST_DROP_REDUCING: f64 = 60.0;

pub(super) const COST_NEW_JACK: f64 = 100.0;

pub(super) const COST_BROKEN_JACK: f64 = 40.0;

/// Reward for an extra note on a downbeat or a chord; less elsewhere.
pub(super) const EXTRA_REWARD: f64 = 10.0;

/// How far (in keys) a column can stray from its natural place for free.
pub(super) const FREE_DEVIATION: f64 = 0.75;

/// A source trill: two rows alternating for at least this many rows (single
/// notes) or chord rows (jumptrills), at most this far apart (in beats).
pub(super) const TRILL_MIN_NOTES: usize = 6;

pub(super) const TRILL_MIN_CHORDS: usize = 4;

pub(super) const TRILL_MAX_GAP_BEATS: f64 = 0.5;

/// Leeway for gaps measured in beats: osu! rounds the notes to the ms, so a
/// 1/2 at 174 BPM (172.4 ms) comes out as 172 or 173.
pub(super) const ROUNDING_MS: f64 = 2.0;

/// Breaking a real trill.
pub(super) const COST_BROKEN_TRILL: f64 = 10.0;

/// Rows of history kept to spot back-and-forths and to copy patterns.
pub(super) const HISTORY: usize = 3;

/// Reward for copying a repeated pattern (see [`mark_patterns`]): it wins
/// whenever it fits.
pub(super) const W_COPY: f64 = 40.0;

/// Upper bound of the noise that breaks ties between equal choices, so the
/// patterns do not always go the same way.
pub(super) const TIE_BREAK: f64 = 0.3;

// Weights of the pattern costs, tuned against hand conversions (Delta Dan,
// krrcream's packs) and feedback on Black dress.

/// Moving the other way than the source.
pub(super) const W_DIRECTION: f64 = 3.0;

/// Per key of difference between how far the source and the target move
/// (relative to the key count).
pub(super) const W_SHAPE: f64 = 0.5;

/// Reward for turning two connected notes (next keys, same way) into three,
/// when the source goes that way too.
pub(super) const W_ROLL: f64 = 1.0;

/// Making a connected run longer than three notes.
pub(super) const W_LONG_ROLL: f64 = 4.0;

/// Starting a connected run the same way as the last one.
pub(super) const W_SAME_ROLL: f64 = 2.0;

/// Using a key that was just released, decaying over a beat.
pub(super) const W_FRESH: f64 = 10.0;

/// The same in chord rows, where a key coming back from 2 rows ago anchors a
/// jumpstream instead of breaking a stream.
pub(super) const W_FRESH_CHORDS: f64 = 4.0;

/// A column going to another key than its previous note.
pub(super) const W_SWITCH: f64 = 0.3;

/// Per squared key a column strays from its natural place.
pub(super) const W_DEVIATION: f64 = 1.0;

/// A key coming back 2 / 3 notes later within a beat, outside a trill.
pub(super) const W_REPEAT2: f64 = 12.0;

pub(super) const W_REPEAT3: f64 = 6.0;

/// A note going to the other hand.
pub(super) const W_CROSS: f64 = 6.0;

/// New keys per source key from which a column coming back to its key is
/// spread over the others ([`W_FRESH`], [`W_REPEAT2`]) in full, and up to
/// which it is not at all: 4K -> 7K (0.75) and 7K -> 10K (0.43) roll the
/// back-and-forths of the source over the new keys, 10K -> 12K (0.2) keeps
/// them, as its mapper meant them with 10 keys to choose from. With half the
/// cost, a 10K map's columns coming back two notes later kept their key 40%
/// of the time; with none, 80%.
pub(super) const SPREAD_FROM: f64 = 0.2;

pub(super) const SPREAD_FULL: f64 = 0.4;

/// Where the target has few new keys, a column moving to another key while
/// it played within the last beat costs this, times how little it spreads
/// ([`SPREAD_FULL`]): the scratch's reward or the pinky's balance do not pull
/// a column away mid-pattern. On a 10K map going to 12K, with 4 the columns
/// coming back two notes later kept their key 96 to 98% of the time (85 to
/// 89% without); with 8, 98 to 100%, but the scratch kept columns for runs of
/// 10.
pub(super) const W_STICK: f64 = 4.0;

/// Reward for a note on a scratch the pinky moves out to (10K2S, 14K2S DP),
/// on the beat; [`SCRATCH_BEAT`] of it elsewhere. Hand-made 10K2S maps use
/// the scratch as an accent, 74% of its notes on the beat; without the
/// reward, the scratch gets few notes and those in runs, from the source
/// columns whose natural place it is.
pub(super) const W_SCRATCH: f64 = 8.0;

/// The share of [`W_SCRATCH`] by position in the beat (1/1, 1/2, 1/4, other).
pub(super) const SCRATCH_BEAT: [f64; 4] = [1.0, 0.5, 0.0, 0.0];

/// A scratch costs this per note it had lately and would have next (see
/// [`Note::ahead`]), each decaying over [`SCRATCH_DECAY_BEATS`]: hand-made
/// maps hit it in short groups with long rests in between (a third of the
/// gaps a beat or less, a quarter over 8 beats), never for bars on end.
/// With 8 and 3, 4K, 7K and 10K converts give each scratch 0.5 to 1.6x the
/// notes of a key, in runs of 7 at most (see `corpus_scratch`); a reward of
/// 10, or a level of 2, makes runs of 10 to 20 again.
pub(super) const W_SCRATCH_LEVEL: f64 = 3.0;

pub(super) const SCRATCH_DECAY_BEATS: f64 = 4.0;

/// Per note a scratch has had more than the key next to it (the other key
/// of its pinky) lately, decaying over [`PINKY_BALANCE_BEATS`], a note on it
/// costs this more, and one on that key this less, by
/// [`PINKY_BALANCE_CAP`] notes at most; the same the other way. Hand-made
/// 10K2S maps share the pinky's notes about evenly (6.6% on the scratch, 6.0%
/// on the key); without it the scratch took four times as many, as the
/// source column whose natural place it is stays on it. With 2, converts get
/// 0.9 to 1.3 notes on the scratch per note on the key. A bigger reward
/// brings runs back: a scratch left alone for long would take every note.
pub(super) const W_PINKY_BALANCE: f64 = 2.0;

pub(super) const PINKY_BALANCE_BEATS: f64 = 16.0;

pub(super) const PINKY_BALANCE_CAP: f64 = 2.0;

/// Most extra notes a row can get.
pub(super) const MAX_EXTRAS: usize = 4;

/// Full: chords grow by the key ratio times this. Measured on Enie's 7K
/// H.A.R.D.C.O.R.E. against the 4K: 1 note -> 1.5, 2 -> 3.4, 3 -> 4.6. 4K ->
/// 7K grows by [`FULL_GROWTH`] instead.
pub(super) const FULL_SCALE: f64 = 0.9;

/// Full, 4K -> 7K: how many notes a row grows by, by where it falls in
/// the beat (on it, on a 1/2, on a 1/4, elsewhere: 1/3s, 1/8s) and how many
/// notes it has (1, 2, 3 or more), in a stream (rows 1/4 beat apart or closer
/// on both sides). Only for 4K -> 7K, the one it is measured on: the other
/// key counts keep [`FULL_SCALE`] until they are. Measured row by row on 5 hand-made 7K
/// maps of DJ Sharpnel's songs against 4K maps of the same songs (12,000
/// rows): their chords do not grow evenly. A single note on the beat or its
/// 1/2 becomes a jump or a hand (2.8 and 2.5 notes), a jump about a hand,
/// a hand a quad; the 1/4s in between stay light, 55% of them single notes,
/// and the 1/3s and 1/8s do not grow. With 0.9 times the key ratio all along
/// (Enie's 7K H.A.R.D.C.O.R.E.: 1 note -> 1.5, 2 -> 3.4, 3 -> 4.6), single
/// notes on the beat had 1.55 notes, and the 1/4s a jump 56% of the time and
/// three notes 9% where hand-made maps have 2 notes.
pub(super) const FULL_GROWTH: [[f64; 3]; 4] = [[1.8, 1.3, 1.1], [1.55, 0.95, 0.6], [0.5, 0.3, 0.3], [0.0, 0.0, 0.0]];

/// Full: what a single note outside a stream grows by, against one in it:
/// 1.0 and 0.85 more notes on the beat and its 1/2 instead of 1.8 and 1.55.
pub(super) const FULL_LONE: f64 = 0.55;

/// Full: how much a full row of the source grows (tyrcs: a quad -> 4.1).
pub(super) const FULL_ROW_GROWTH: f64 = 0.5;

/// Full: reward per extra note, high enough to fill every row's quota.
pub(super) const FULL_REWARD: f64 = 25.0;

/// An extra note in a key used a moment ago (a minijack): more than any
/// reward, so extra notes never make one. Repeated patterns (chordjacks) copy
/// their extra notes on their own.
pub(super) const W_EXTRA_JACK: f64 = 40.0;

/// Where the source alternates hands (jumptrills, trills, brackets), a note on
/// the hand the previous row used: that hand would run at twice the speed of
/// the other. More than Full's reward, so phases stay on their own hand; the
/// middle key goes with whichever phase gets it first.
pub(super) const W_HAND_TWICE: f64 = 30.0;

/// Full: reward, in those rows, for mirroring the previous row on
/// the other hand (`OO.` then `.OO`), as 45% of the rows of hand-made 7K
/// jumptrills do.
pub(super) const W_MIRROR: f64 = 2.0;

/// 4K -> 7K: an extra note on the middle key (either thumb's) in a row of
/// more than two notes, per note past two. Only there, as [`FULL_GROWTH`].
/// Hand-made 7K maps (DJ
/// Sharpnel's songs, see [`FULL_GROWTH`]) build their big chords with the
/// hands and leave the thumb out: it is in 33% of their rows of 3 notes on
/// the beat, 42% of 4 and 53% of 5, less than the other keys, while in jumps
/// it is there as often as they are (30%). Without it, extra notes went to it
/// as the one key no hand was busy with: 43%, 56% and 54%, and the thumb was
/// in the big chords on the beat and its 1/2 for bars. With it, 34%, 44% and
/// 55%, the same from 1 to 3: it only settles near ties. On the notes of the
/// map too, it took the thumb out of big chords and put it in single notes.
pub(super) const W_THUMB_CHORD: f64 = 1.0;

// Wide hands (4 keys or more each: 8K 4+4, 9K, 10K and up), measured against
// hand-made 10K maps (see `corpus_feel`). There each hand plays patterns of
// its own: stairs, runs on three of its keys (pinky-ring-middle, ...), open
// chords rather than pills. With 7K's three keys a hand, rows grow into the
// keys next to them instead, as hand conversions do.

/// Wide hands: reward for a hand going on with its stair, to the next key the
/// way it went last.
pub(super) const W_HAND_ROLL: f64 = 0.7;

/// Wide hands: reward for a hand staying on three next keys for three rows.
pub(super) const W_HAND_GROUP: f64 = 1.0;

/// Wide hands: the most a note can take off what its hand plays costs (a
/// little more, for rounding).
pub(super) const HAND_REWARD_MAX: f64 = W_HAND_ROLL + W_HAND_GROUP + 1e-9;

/// Wide hands: a pill (two next keys of one hand) where the source has none.
/// Open chords stay open, as when mappers adapt a 7K pattern to 10K, and
/// pills of the source may stay pills.
pub(super) const W_PILL: f64 = 1.0;

/// Wide hands, Dense and More dense: the share of the reward for extra notes
/// in each density zone of the source (Break, Calm, Normal, Dense, Peak). The
/// calm parts stay calm and the dense ones get denser.
pub(super) const ZONE_FILL: [f64; 5] = [0.0, 0.2, 0.6, 1.0, 1.2];

/// Wide hands, Dense and More dense: the share of the reward a second extra
/// note in a row keeps. Two more notes in every row of a stream leave no room
/// for patterns on 10K: they come out as pills or as a checkerboard.
pub(super) const SECOND_EXTRA_WIDE: f64 = 0.25;
