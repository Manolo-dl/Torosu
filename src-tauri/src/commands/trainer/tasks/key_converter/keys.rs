//! Hands, fingers and sets of keys (as bit masks) of the source and the
//! target, and the ways a row's notes can go to keys.

use super::*;

/// A note stays on its hand; a key played by either thumb goes either way.
pub(super) fn same_hand(src: Hand, dst: Hand) -> bool {
    src == Hand::Either || dst == Hand::Either || src == dst
}

/// A hand as a bit: 1 left, 2 right, 0 for a key of either thumb.
pub(super) fn hand_bit(hand: Hand) -> u8 {
    match hand {
        Hand::Left => 1,
        Hand::Right => 2,
        Hand::Either => 0,
    }
}

/// The hand of each key of a playstyle.
pub(super) fn hands(layout: &Layout) -> [Hand; MAX_KEYS] {
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
pub(super) fn shared_fingers(layout: &Layout) -> [u32; MAX_KEYS] {
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
pub(super) fn keys_of(mut mask: u32) -> impl Iterator<Item = u8> {
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
pub(super) fn rank(mask: u32) -> usize {
    let binomial = |n: usize, k: usize| if k > n { 0 } else { (0..k).fold(1, |c, i| c * (n - i) / (i + 1)) };
    keys_of(mask).enumerate().map(|(i, key)| binomial(key as usize, i + 1)).sum()
}

/// The keys each note of a row can go to, left to right: every key it fits
/// in, or on two stages its cheapest ones: [`KEY_CHOICES`] at most, and fewer
/// in rows of many notes, so they have at most [`ROW_CHOICES`] ways to go.
pub(super) fn choices(costs: &[[f64; MAX_KEYS]], keys: usize) -> Vec<Vec<u8>> {
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
pub(super) fn each_combo(choices: &[&[u8]], fingers: &[u32; MAX_KEYS], combo: &mut [u8], f: &mut impl FnMut(&[u8])) {
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
pub(super) fn fingers_count(fingers: &[u32; MAX_KEYS], keys: usize) -> usize {
    (0..keys).filter(|&k| fingers[k].trailing_zeros() as usize == k).count()
}

/// Which of `k` notes (sorted by column) are kept when only `m` fit: spread
/// out, so the outer notes of a chord survive.
pub(super) fn kept(k: usize, m: usize) -> Vec<usize> {
    match m {
        0 => Vec::new(),
        1 => vec![k / 2],
        _ => (0..m).map(|i| (i * (k - 1) + (m - 1) / 2) / (m - 1)).collect(),
    }
}

/// The middle of the keys of a mask.
pub(super) fn center(mask: u32) -> f64 {
    let (sum, n) = keys_of(mask).fold((0, 0), |(s, n), k| (s + k as usize, n + 1));
    sum as f64 / n.max(1) as f64
}

pub(super) fn pos(col: usize, keys: usize) -> f64 {
    (col as f64 + 0.5) / keys as f64
}
