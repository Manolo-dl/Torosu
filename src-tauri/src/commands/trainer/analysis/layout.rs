//! Playstyles: which finger of which hand plays each key.
//!
//! Every key count has its usual ways to be played, and the patterns of a map
//! only make sense against one of them: on 8K a `3+2+3` player hits the two
//! middle keys with the thumbs, a `7K+1` player has a scratch on the left.
//! osu! 7K is always `3+1+3`.
//!
//! A mirrored playstyle (`2+3` for `3+2`, 7K+1 with the scratch on the right) is not listed:
//! players get it with osu!'s Mirror mod. The analysis groups
//! the keys by hand (and by player in co-op), so each group gets its own
//! patterns.
//!
//! osu! plays up to 10K on one stage; 12K to 18K are two stages side by side
//! (6K+6K ... 9K+9K), made for co-op but mostly played solo, with the
//! playstyles of the osu! wiki (Beatmapping/osu!mania 10K+ playstyles):
//! `10K2S` on 12K, `DP` and `EZ2AC` on 14K and 16K, `10K8K` and `9K9K` on 18K.
//! 12K is also played as twelve lanes (`12K`), the patterns running into the
//! outer keys, where 10K2S keeps them for a scratch's own notes.

use serde::Serialize;

use crate::commands::trainer::mania;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum Hand {
    Left,
    Right,
    /// A middle key played by whichever thumb is free.
    Either,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum Digit {
    Thumb,
    Index,
    Middle,
    Ring,
    Pinky,
    /// The turntable key of 7K+1 (IIDX style), played with the whole hand.
    Scratch,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub struct Finger {
    /// 0, or 1 for the second player of a co-op layout.
    pub player: u8,
    pub hand: Hand,
    pub digit: Digit,
}

#[derive(Clone, Debug, Serialize)]
pub struct Layout {
    pub name: &'static str,
    pub description: &'static str,
    pub fingers: Vec<Finger>,
}

/// A group of keys played by one hand (or by the thumbs in the middle).
#[derive(Clone, Debug, Serialize)]
pub struct Group {
    pub name: String,
    pub mask: u32,
}

impl Layout {
    pub fn keys(&self) -> usize {
        self.fingers.len()
    }

    /// The keys of each hand, left to right.
    pub fn groups(&self) -> Vec<Group> {
        let mut groups: Vec<(u8, Hand, u32)> = Vec::new();
        for (col, finger) in self.fingers.iter().enumerate() {
            match groups.iter_mut().find(|(p, h, _)| *p == finger.player && *h == finger.hand) {
                Some((_, _, mask)) => *mask |= 1 << col,
                None => groups.push((finger.player, finger.hand, 1 << col)),
            }
        }
        let players = self.fingers.iter().any(|f| f.player > 0);
        groups
            .into_iter()
            .map(|(player, hand, mask)| {
                let hand = match hand {
                    Hand::Left => "Left",
                    Hand::Right => "Right",
                    Hand::Either => "Thumb",
                };
                let name = if players { format!("P{} {}", player + 1, hand) } else { hand.to_string() };
                Group { name, mask }
            })
            .collect()
    }

    /// The key count of the map under the scratches when they are lanes of
    /// their own, notes for some sounds over a map of the other keys that
    /// stands without them (10K2S: a 10K map and a scratch on each side, as
    /// 7K+1 is a 7K map and a scratch), rather than keys the patterns run into
    /// (12K).
    pub fn inner_keys(&self) -> Option<usize> {
        matches!(self.name, "10K2S" | "7K+1").then(|| self.keys() - self.digit_mask(Digit::Scratch).count_ones() as usize)
    }

    pub fn digit_mask(&self, digit: Digit) -> u32 {
        self.fingers.iter().enumerate().filter(|(_, f)| f.digit == digit).fold(0, |m, (c, _)| m | 1 << c)
    }

    /// Whether the layout looks the same mirrored, so mirrored patterns are
    /// the same pattern.
    pub fn symmetric(&self) -> bool {
        let n = self.keys();
        (0..n).all(|c| {
            let (a, b) = (self.fingers[c], self.fingers[n - 1 - c]);
            a.digit == b.digit
                && matches!(
                    (a.hand, b.hand),
                    (Hand::Left, Hand::Right) | (Hand::Right, Hand::Left) | (Hand::Either, Hand::Either)
                )
        })
    }
}

/// Parses a layout written as one token per key: `L`/`R` and the finger in
/// piano numbering (1 thumb, 2 index, 3 middle, 4 ring, 5 pinky), `T` for a
/// thumb of either hand, `M` for a middle key either hand moves to, `LS`/`RS`
/// for a scratch, and `|` between the stages of two players.
fn parse(spec: &str) -> Vec<Finger> {
    let mut fingers = Vec::new();
    let mut player = 0;
    for token in spec.split_whitespace() {
        if token == "|" {
            player += 1;
            continue;
        }
        let (hand, digit) = match token {
            "T" => (Hand::Either, Digit::Thumb),
            "M" => (Hand::Either, Digit::Index),
            "LS" => (Hand::Left, Digit::Scratch),
            "RS" => (Hand::Right, Digit::Scratch),
            _ => {
                let hand = if token.starts_with('L') { Hand::Left } else { Hand::Right };
                let digit = match &token[1..] {
                    "1" => Digit::Thumb,
                    "2" => Digit::Index,
                    "3" => Digit::Middle,
                    "4" => Digit::Ring,
                    _ => Digit::Pinky,
                };
                (hand, digit)
            }
        };
        fingers.push(Finger { player, hand, digit });
    }
    fingers
}

/// One player on each stage, each with the usual layout of its key count.
fn coop(stage: usize) -> String {
    let one = layouts(stage).into_iter().next().map(|l| l.fingers).unwrap_or_default();
    let spec = |fingers: &[Finger]| {
        fingers
            .iter()
            .map(|f| match (f.hand, f.digit) {
                (Hand::Either, _) => "T".to_string(),
                (Hand::Left, Digit::Scratch) => "LS".to_string(),
                (Hand::Right, Digit::Scratch) => "RS".to_string(),
                (hand, digit) => format!(
                    "{}{}",
                    if hand == Hand::Left { 'L' } else { 'R' },
                    match digit {
                        Digit::Thumb => 1,
                        Digit::Index => 2,
                        Digit::Middle => 3,
                        Digit::Ring => 4,
                        _ => 5,
                    }
                ),
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    format!("{} | {}", spec(&one), spec(&one))
}

/// One player, a stage per hand: the fingers spread over the keys, from the
/// pinky outside to the thumb inside.
fn solo(stage: usize) -> String {
    let side = |hand: char| -> Vec<String> {
        (0..stage).map(|i| format!("{}{}", hand, 5 - i * 5 / stage)).collect()
    };
    let left = side('L');
    let mut right = side('R');
    right.reverse();
    format!("{} {}", left.join(" "), right.join(" "))
}

/// The playstyles of a key count, the most common first.
pub fn layouts(keys: usize) -> Vec<Layout> {
    let fixed: &[(&str, &str, &str)] = match keys {
        1 => &[("1", "One key, either hand", "T")],
        2 => &[("1+1", "An index per hand", "L2 R2")],
        3 => &[("1+1+1", "Indexes and a thumb", "L2 T R2"), ("2+1", "Two left, one right", "L3 L2 R2")],
        4 => &[("2+2", "Index and middle of each hand", "L3 L2 R2 R3")],
        5 => &[
            ("2+1+2", "Thumb on the middle key", "L3 L2 T R2 R3"),
            ("3+2", "Middle key with the left index", "L4 L3 L2 R2 R3"),
        ],
        6 => &[("3+3", "Three fingers per hand", "L4 L3 L2 R2 R3 R4")],
        // osu! 7K is always played 3+1+3.
        7 => &[("3+1+3", "Thumb on the middle key", "L4 L3 L2 T R2 R3 R4")],
        8 => &[
            ("4+4", "Four fingers per hand", "L5 L4 L3 L2 R2 R3 R4 R5"),
            ("3+2+3", "Both thumbs on the middle keys", "L4 L3 L2 L1 R1 R2 R3 R4"),
            ("7K+1", "7K with a scratch on the left (IIDX)", "LS L4 L3 L2 T R2 R3 R4"),
        ],
        9 => &[("4+1+4", "Thumb on the middle key (pop'n)", "L5 L4 L3 L2 T R2 R3 R4 R5")],
        10 => &[("5+5", "Every finger, thumbs in the middle", "L5 L4 L3 L2 L1 R1 R2 R3 R4 R5")],
        // 12K: every key a lane, the patterns running into the outer ones;
        // 10K2S: a 10K map, and a scratch on each side for some sounds. The
        // same fingers, the outer key with the pinky.
        12 => &[
            ("12K", "Every key a lane, the patterns run into the outer ones", "LS L5 L4 L3 L2 L1 R1 R2 R3 R4 R5 RS"),
            ("10K2S", "10K with a scratch on each side for some sounds", "LS L5 L4 L3 L2 L1 R1 R2 R3 R4 R5 RS"),
        ],
        // DP: 7 keys per hand in two rows, the hand resting on keys 1, 2, 4,
        // 6 and 7 from the thumb; the ring finger also takes key 5 and the
        // index key 3.
        14 => &[
            ("DP", "7 keys per hand in two rows (IIDX)", "L5 L4 L4 L3 L2 L2 L1 R1 R2 R2 R3 R4 R4 R5"),
            ("EZ2AC", "10K and 4 middle keys either hand moves to", "L5 L4 L3 L2 L1 M M M M R1 R2 R3 R4 R5"),
        ],
        16 => &[
            ("DP", "DP with a scratch on each side", "LS L5 L4 L4 L3 L2 L2 L1 R1 R2 R2 R3 R4 R4 R5 RS"),
            ("EZ2AC", "10K2S and 4 middle keys either hand moves to", "LS L5 L4 L3 L2 L1 M M M M R1 R2 R3 R4 R5 RS"),
        ],
        // 10K8K: 4 more keys for each hand's fingers but the thumb, outside
        // (4K10K4K) or inside its 10K keys, which is the same for the fingers.
        18 => &[
            ("10K8K", "10K and 4 more keys per hand, the thumbs aside", "L5 L4 L3 L2 L5 L4 L3 L2 L1 R1 R2 R3 R4 R5 R2 R3 R4 R5"),
            ("9K9K", "9 keys per hand in two rows", "L5 L5 L4 L4 L3 L3 L2 L2 L1 R1 R2 R2 R3 R3 R4 R4 R5 R5"),
        ],
        _ => &[],
    };
    let mut all: Vec<Layout> =
        fixed.iter().map(|&(name, description, spec)| Layout { name, description, fingers: parse(spec) }).collect();

    // Two stages can also be played in co-op, a player per stage.
    if keys > mania::STAGE_MAX_KEYS && mania::is_playable(keys) {
        all.push(Layout { name: "Co-op", description: "A player per stage", fingers: parse(&coop(keys / 2)) });
    }
    if !all.is_empty() {
        return all;
    }

    // Anything else: half per hand, a thumb in the middle when odd.
    let half = keys / 2;
    let mut spec = solo(half).split_whitespace().take(half).collect::<Vec<_>>().join(" ");
    if keys % 2 == 1 {
        spec.push_str(" T");
    }
    let right: Vec<String> = solo(half).split_whitespace().skip(half).map(String::from).collect();
    spec = format!("{} {}", spec, right.join(" "));
    vec![Layout { name: "Split", description: "Half the keys per hand", fingers: parse(&spec) }]
}

/// Playstyles renamed, as (old name, new one): presets saved before keep theirs.
const RENAMED: [(&str, &str); 1] = [("1+7", "7K+1")];

/// The layout called `name` (or what it was called, see [`RENAMED`]), or the
/// usual one of the key count.
pub fn layout(keys: usize, name: Option<&str>) -> Layout {
    let all = layouts(keys);
    let name = name.map(|n| RENAMED.iter().find(|r| r.0 == n).map_or(n, |r| r.1));
    name.and_then(|n| all.iter().find(|l| l.name == n).cloned()).unwrap_or_else(|| all[0].clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_count_has_a_layout() {
        for keys in 1..=18 {
            for layout in layouts(keys) {
                assert_eq!(layout.keys(), keys, "{}K {}", keys, layout.name);
                let groups = layout.groups();
                assert_eq!(groups.iter().fold(0, |m, g| m | g.mask), (1u32 << keys) - 1, "{}K {}", keys, layout.name);
            }
        }
    }

    #[test]
    fn groups_follow_the_hands() {
        let names = |keys, name| layout(keys, Some(name)).groups().into_iter().map(|g| g.name).collect::<Vec<_>>();
        assert_eq!(names(7, "3+1+3"), ["Left", "Thumb", "Right"]);
        assert_eq!(names(14, "Co-op"), ["P1 Left", "P1 Thumb", "P1 Right", "P2 Left", "P2 Thumb", "P2 Right"]);
        assert_eq!(names(14, "DP"), ["Left", "Right"]);
        assert_eq!(names(16, "EZ2AC"), ["Left", "Thumb", "Right"]);
        assert_eq!(names(18, "9K9K"), ["Left", "Right"]);
        assert!(layout(7, None).symmetric());
        assert!(!layout(5, Some("3+2")).symmetric());
        assert!(!layout(8, Some("7K+1")).symmetric());
        // Saved under its old name.
        assert_eq!(layout(8, Some("1+7")).name, "7K+1");
        for keys in [12, 14, 16, 18] {
            assert!(layouts(keys).iter().all(|l| l.symmetric()), "{}K", keys);
        }
    }

    #[test]
    fn wide_playstyles_follow_the_wiki() {
        let ten = layout(10, None);
        // 10K2S and 10K8K keep the hands on the 10K keys.
        assert_eq!(layout(12, Some("10K2S")).digit_mask(Digit::Scratch), 1 | 1 << 11);
        assert_eq!(layout(18, Some("10K8K")).digit_mask(Digit::Thumb), 1 << 8 | 1 << 9);
        assert_eq!(layout(18, Some("10K8K")).fingers[4..14], ten.fingers[..]);
        // EZ2AC: the 10K keys around 4 middle keys for either hand.
        let ez2ac = layout(14, Some("EZ2AC"));
        assert_eq!([&ez2ac.fingers[..5], &ez2ac.fingers[9..]].concat(), ten.fingers);
        assert!(ez2ac.fingers[5..9].iter().all(|f| f.hand == Hand::Either && f.digit != Digit::Thumb));
        // DP: every finger of a hand on its 7 keys.
        let dp = layout(14, Some("DP"));
        assert_eq!(dp.groups().iter().map(|g| g.mask.count_ones()).collect::<Vec<_>>(), [7, 7]);
        for digit in [Digit::Thumb, Digit::Index, Digit::Middle, Digit::Ring, Digit::Pinky] {
            assert_ne!(dp.digit_mask(digit) & 0x7f, 0, "{:?}", digit);
        }
    }
}
