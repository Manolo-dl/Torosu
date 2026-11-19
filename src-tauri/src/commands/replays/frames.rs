//! The key presses of a mania replay (.osr).
//!
//! An .osr is the score's header (as in `scores.db`) and its frames, as text
//! (`w|x|y|z,` each) compressed with LZMA in its "alone" format, not as .xz:
//! osynic_osudb only reads the latter, so the file is read here.
//!
//! Each frame of a mania replay holds which keys are down in `x`, one bit per
//! key from the left, and how long after the previous frame it comes (`w`).
//! Its times are the map's, so with DT or HT they run 1.5 or 0.75 times as
//! fast as the real ones.

use std::{io::Read, path::Path};

use liblzma::{read::XzDecoder, stream::Stream};

use crate::{commands::replays::scores::ManiaCounts, error::Result};

const MANIA: u8 = 3;
/// The frame where stable keeps the seed of Random instead of keys.
const SEED_FRAME: i64 = -12345;
/// Where the frames stable writes before the first one with input point to.
const STARTUP_FRAME: (f64, f64) = (256.0, -500.0);
/// No replay comes close: hours of frames are a few MB.
const MAX_FRAMES_TEXT: u64 = 256 << 20;

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
    let bytes = std::fs::read(path)?;
    parse(&bytes).ok_or_else(|| format!("{} is not an osu!mania replay osu! can read", path.display()).into())
}

fn parse(bytes: &[u8]) -> Option<ManiaReplay> {
    let mut r = Reader { bytes, at: 0 };
    if r.u8()? != MANIA {
        return None;
    }
    r.i32()?; // game version
    let beatmap_md5 = r.string()?;
    let player_name = r.string()?;
    r.string()?; // replay hash
    let [n300, n100, n50, max, n200, miss] = [r.u16()?, r.u16()?, r.u16()?, r.u16()?, r.u16()?, r.u16()?];
    r.i32()?; // score
    r.u16()?; // highest combo
    r.u8()?; // perfect
    let mods = r.i32()? as u32;
    r.string()?; // health graph
    r.take(8)?; // timestamp
    let length = usize::try_from(r.i32()?).ok()?;
    let compressed = r.take(length)?;

    let mut text = String::new();
    let stream = Stream::new_lzma_decoder(u64::MAX).ok()?;
    XzDecoder::new_stream(compressed, stream).take(MAX_FRAMES_TEXT).read_to_string(&mut text).ok()?;
    let (frames, scroll_speed_scale) = frames(&text);

    Some(ManiaReplay {
        beatmap_md5,
        player_name,
        mods,
        counts: ManiaCounts {
            max: max.into(),
            n300: n300.into(),
            n200: n200.into(),
            n100: n100.into(),
            n50: n50.into(),
            miss: miss.into(),
        },
        frames,
        scroll_speed_scale,
    })
}

/// The frames of the replay's text, and the scroll speed scale of its first
/// frames.
fn frames(text: &str) -> (Vec<Frame>, Option<f32>) {
    let mut frames: Vec<Frame> = Vec::new();
    let mut scroll_speed_scale = None;
    let mut time = 0;
    for (index, entry) in text.split(',').enumerate() {
        let mut fields = entry.split('|').map(str::parse::<f64>);
        let (Some(Ok(delta)), Some(Ok(x)), Some(Ok(y))) = (fields.next(), fields.next(), fields.next()) else {
            continue;
        };
        let delta = delta as i64;
        if delta == SEED_FRAME {
            continue;
        }
        // Frames that are left out still move the clock.
        time += delta;
        if scroll_speed_scale.is_none() && y > 0.0 && y.is_finite() {
            scroll_speed_scale = Some(y as f32);
        }
        if (index < 2 && (x, y) == STARTUP_FRAME) || delta < 0 {
            continue;
        }
        // A frame before the previous one would undo presses already made.
        if frames.last().is_some_and(|last| time < last.time) {
            continue;
        }
        frames.push(Frame { time, keys: x.max(0.0) as u32 });
    }
    (frames, scroll_speed_scale)
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let out = self.bytes.get(self.at..self.at.checked_add(n)?)?;
        self.at += n;
        Some(out)
    }

    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }

    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    /// `0x00` for none, or `0x0b`, a ULEB128 length and UTF-8.
    fn string(&mut self) -> Option<String> {
        match self.u8()? {
            0x00 => Some(String::new()),
            0x0b => {
                let mut length = 0usize;
                for shift in (0..35).step_by(7) {
                    let byte = self.u8()?;
                    length |= usize::from(byte & 0x7f) << shift;
                    if byte & 0x80 == 0 {
                        return Some(String::from_utf8_lossy(self.take(length)?).into_owned());
                    }
                }
                None
            }
            _ => None,
        }
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
    use std::io::Write;

    use liblzma::{stream::LzmaOptions, write::XzEncoder};

    use super::*;

    fn string(out: &mut Vec<u8>, s: &str) {
        out.push(0x0b);
        out.push(s.len() as u8);
        out.extend(s.as_bytes());
    }

    /// An .osr of `mode` with `mods` and the frames of `text`, as osu! writes it.
    pub(crate) fn osr(mode: u8, mods: u32, text: &str) -> Vec<u8> {
        let mut compressed = Vec::new();
        let stream = Stream::new_lzma_encoder(&LzmaOptions::new_preset(1).unwrap()).unwrap();
        let mut encoder = XzEncoder::new_stream(&mut compressed, stream);
        encoder.write_all(text.as_bytes()).unwrap();
        encoder.finish().unwrap();

        let mut out = vec![mode];
        out.extend(20261118i32.to_le_bytes());
        string(&mut out, "5556dadd32fc89b9b8055a27ab1bac9f");
        string(&mut out, "Manolo_");
        string(&mut out, "replay");
        for count in [10u16, 2, 1, 30, 4, 3] {
            out.extend(count.to_le_bytes());
        }
        out.extend(900_000i32.to_le_bytes());
        out.extend(40u16.to_le_bytes());
        out.push(0);
        out.extend(mods.to_le_bytes());
        out.push(0);
        out.extend(639_174_849_939_639_372i64.to_le_bytes());
        out.extend((compressed.len() as i32).to_le_bytes());
        out.extend(&compressed);
        out.extend(0i64.to_le_bytes());
        out
    }

    /// Stable's startup frames, then key frames at (delta, keys) and the seed.
    pub(crate) fn frames_text(keys: &[(i64, u32)]) -> String {
        let mut text = String::from("0|256|-500|0,-1|256|-500|0,");
        for (delta, keys) in keys {
            text += &format!("{delta}|{keys}|1.5|0,");
        }
        text + "-12345|0|0|7171,"
    }

    #[test]
    fn reads_an_osr_file() {
        let dir = crate::test_utils::TestDir::new("replay-frames");
        let path = dir.0.join("play.osr");
        std::fs::write(&path, osr(MANIA, 64, &frames_text(&[(1000, 0b01), (50, 0b11), (40, 0)]))).unwrap();

        let replay = read(&path).unwrap();

        assert_eq!(replay.beatmap_md5, "5556dadd32fc89b9b8055a27ab1bac9f");
        assert_eq!(replay.mods, 64);
        assert_eq!(replay.counts, ManiaCounts { max: 30, n300: 10, n200: 4, n100: 2, n50: 1, miss: 3 });
        assert_eq!(
            replay.frames,
            [Frame { time: 999, keys: 0b01 }, Frame { time: 1049, keys: 0b11 }, Frame { time: 1089, keys: 0 }]
        );
        assert_eq!(replay.scroll_speed_scale, Some(1.5));
    }

    #[test]
    fn only_reads_mania_replays() {
        assert!(parse(&osr(0, 0, &frames_text(&[(1000, 1)]))).is_none());
        assert!(parse(&[MANIA, 1, 2]).is_none());
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
