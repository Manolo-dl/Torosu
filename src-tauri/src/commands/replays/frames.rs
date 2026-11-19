//! The key presses of a mania replay (.osr).
//!
//! Each frame of a mania replay holds which keys are down in `x`, one bit per
//! key from the left, and how long after the previous frame it comes. Its
//! times are the map's, so with DT or HT they run 1.5 or 0.75 times as fast
//! as the real ones.

use std::path::Path;

use osynic_osudb::entity::{osu::field::mode::Mode, scores::field::replay::Replay};

use crate::{commands::replays::scores::ManiaCounts, error::Result};

/// The frame where stable keeps the seed of Random instead of keys.
const SEED_FRAME: i64 = -12345;
/// Where the frames stable writes before the first one with input point to.
const STARTUP_FRAME: (f32, f32) = (256.0, -500.0);

/// The keys down from `time` (ms, map time) on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    pub time: i64,
    /// One bit per key, from the left.
    pub keys: u32,
}

/// A key held from `start` to `end` (ms, map time).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Press {
    pub start: f64,
    pub end: f64,
}

/// What a mania replay says about the play.
#[derive(Debug, Clone)]
pub struct ManiaReplay {
    pub beatmap_md5: String,
    pub player_name: String,
    /// osu!'s mod bits.
    pub mods: u32,
    pub counts: ManiaCounts,
    pub frames: Vec<Frame>,
    /// Stable writes the scroll speed in the `y` of the first frames, divided
    /// by the map's BPM (times the rate) over 100.
    pub scroll_speed_scale: Option<f32>,
}

/// Reads the mania replay at `path`.
pub fn read(path: &Path) -> Result<ManiaReplay> {
    let replay = Replay::from_file(path)?;
    if replay.mode != Mode::Mania {
        return Err(format!("{} is not an osu!mania replay", path.display()).into());
    }
    Ok(mania_replay(replay))
}

fn mania_replay(replay: Replay) -> ManiaReplay {
    let actions = replay.replay_data.unwrap_or_default();

    let mut frames = Vec::with_capacity(actions.len());
    let mut scroll_speed_scale = None;
    let mut time = 0;
    for (index, action) in actions.iter().enumerate() {
        if action.delta == SEED_FRAME {
            continue;
        }
        // Frames that are left out still move the clock.
        time += action.delta;
        if scroll_speed_scale.is_none() && action.y > 0.0 && action.y.is_finite() {
            scroll_speed_scale = Some(action.y);
        }
        if (index < 2 && (action.x, action.y) == STARTUP_FRAME) || action.delta < 0 {
            continue;
        }
        // A frame before the previous one would undo presses already made.
        if frames.last().is_some_and(|last: &Frame| time < last.time) {
            continue;
        }
        frames.push(Frame { time, keys: action.x.max(0.0) as u32 });
    }

    ManiaReplay {
        beatmap_md5: replay.beatmap_hash.unwrap_or_default(),
        player_name: replay.player_name.unwrap_or_default(),
        mods: replay.mods.bits(),
        counts: ManiaCounts {
            max: replay.count_geki.into(),
            n300: replay.count_300.into(),
            n200: replay.count_katsu.into(),
            n100: replay.count_100.into(),
            n50: replay.count_50.into(),
            miss: replay.count_miss.into(),
        },
        frames,
        scroll_speed_scale,
    }
}

/// The presses of each of `keys` keys, in order. A key still down at the
/// last frame is released there.
pub fn presses(frames: &[Frame], keys: usize) -> Vec<Vec<Press>> {
    let mut presses = vec![Vec::new(); keys];
    let mut down: Vec<Option<f64>> = vec![None; keys];

    for frame in frames {
        let time = frame.time as f64;
        for (key, start) in down.iter_mut().enumerate() {
            let pressed = frame.keys & (1 << key) != 0;
            match (*start, pressed) {
                (None, true) => *start = Some(time),
                (Some(from), false) => {
                    presses[key].push(Press { start: from, end: time });
                    *start = None;
                }
                _ => {}
            }
        }
    }

    let last = frames.last().map_or(0.0, |frame| frame.time as f64);
    for (key, start) in down.into_iter().enumerate() {
        if let Some(from) = start {
            presses[key].push(Press { start: from, end: last });
        }
    }
    presses
}

#[cfg(test)]
pub(crate) mod tests {
    use chrono::Utc;
    use osynic_osudb::entity::{osu::field::modification::ModSet, scores::field::action::Action};

    use super::*;

    fn action(delta: i64, x: f32, y: f32) -> Action {
        Action { delta, x, y, z: 0.0 }
    }

    /// A mania replay of `actions`, as osu! would write it.
    pub(crate) fn replay(mods: u32, actions: Vec<Action>) -> Replay {
        Replay {
            mode: Mode::Mania,
            version: 20261118,
            beatmap_hash: Some("md5".into()),
            player_name: Some("Manolo_".into()),
            replay_hash: Some("r".into()),
            count_300: 0,
            count_100: 0,
            count_50: 0,
            count_geki: 0,
            count_katsu: 0,
            count_miss: 0,
            score: 0,
            max_combo: 0,
            perfect_combo: false,
            mods: ModSet::from_bits(mods),
            life_graph: None,
            timestamp: Utc::now(),
            replay_data: Some(actions),
            raw_replay_data: None,
            online_score_id: 0,
        }
    }

    /// Key frames at (delta, keys), after stable's startup frames.
    pub(crate) fn key_actions(keys: &[(i64, u32)]) -> Vec<Action> {
        let mut actions = vec![action(0, 256.0, -500.0), action(-1, 256.0, -500.0)];
        actions.extend(keys.iter().map(|&(delta, keys)| action(delta, keys as f32, 1.5)));
        actions.push(action(SEED_FRAME, 0.0, 0.0));
        actions
    }

    #[test]
    fn leaves_out_the_startup_and_seed_frames() {
        let replay = mania_replay(replay(0, key_actions(&[(1000, 0b01), (50, 0b11), (40, 0)])));

        assert_eq!(
            replay.frames,
            [Frame { time: 999, keys: 0b01 }, Frame { time: 1049, keys: 0b11 }, Frame { time: 1089, keys: 0 }]
        );
        assert_eq!(replay.scroll_speed_scale, Some(1.5));
    }

    #[test]
    fn goes_through_an_osr_file() {
        let dir = crate::test_utils::TestDir::new("replay-frames");
        let path = dir.0.join("play.osr");
        replay(64, key_actions(&[(1000, 1), (30, 0)])).save(&path, None).unwrap();

        let read = read(&path).unwrap();

        assert_eq!(read.mods, 64);
        assert_eq!(read.frames, [Frame { time: 999, keys: 1 }, Frame { time: 1029, keys: 0 }]);
    }

    #[test]
    fn splits_the_frames_into_presses_per_key() {
        let frames = [
            Frame { time: 100, keys: 0b001 },
            Frame { time: 150, keys: 0b101 },
            Frame { time: 180, keys: 0b100 },
            Frame { time: 300, keys: 0b001 },
        ];

        let presses = presses(&frames, 3);

        assert_eq!(presses[0], [Press { start: 100.0, end: 180.0 }, Press { start: 300.0, end: 300.0 }]);
        assert!(presses[1].is_empty());
        assert_eq!(presses[2], [Press { start: 150.0, end: 300.0 }]);
    }
}
