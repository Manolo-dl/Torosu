//! Names for everything the trainer writes: the diff name (`Version`), the
//! `.osu` file and the generated audio.
//!
//! Every variant that would produce different audio or a different diff gets a
//! different name. Otherwise, when osu! merges the .osz into the set folder, the
//! new files overwrite the ones from an earlier run.

use rosu_map::Beatmap;

use crate::commands::trainer::model::{DifficultySettings, RateChangeParams};

/// Formats a number with at most `decimals` decimals, without trailing zeros:
/// 1.3 -> "1.3", 1.25 -> "1.25", 1.0 -> "1", 1.1500000000000001 -> "1.15".
fn fmt_num(value: f64, decimals: usize) -> String {
    let s = format!("{:.*}", decimals, value);
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

fn fmt_rate(rate: f64) -> String {
    format!("{}x", fmt_num(rate, 2))
}

/// Without adjust pitch the audio is resampled, so its pitch follows the speed.
fn pitch_tag(params: &RateChangeParams) -> Option<&'static str> {
    match (params.adjust_pitch, params.rate) {
        (true, _) => None,
        (false, r) if r > 1.0 => Some("pitch raised"),
        (false, r) if r < 1.0 => Some("pitch lowered"),
        _ => None,
    }
}

/// BPM of the timing point that lasts the longest, like the one osu! shows on
/// song select. Expects the timing points to be already rate-scaled.
pub fn main_bpm(beatmap: &Beatmap) -> Option<f64> {
    let points = &beatmap.control_points.timing_points;
    let end = beatmap.hit_objects.last().map_or(0.0, |h| h.start_time);

    points
        .iter()
        .enumerate()
        .map(|(i, tp)| {
            let next = points.get(i + 1).map_or(end, |n| n.time);
            ((next - tp.time).max(0.0), tp.beat_len)
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .filter(|&(_, beat_len)| beat_len > 0.0)
        .map(|(_, beat_len)| 60_000.0 / beat_len)
}

/// Builds the new `Version`, e.g.:
///
/// - `Insane 1.3x (234bpm)`
/// - `Insane 1.3x (234bpm) (pitch raised) AR9.5 OD8`
/// - `Insane HP7` when only the difficulty settings change
/// - `Insane (SV normalized)` when the SV changes are normalized
/// - `Insane (trainer)` when nothing changes
///
/// `beatmap` must already have the tasks applied; `original` holds the
/// difficulty settings before they were overwritten.
pub fn version(
    beatmap: &Beatmap,
    original_version: &str,
    original: &DifficultySettings,
    new: &DifficultySettings,
    rate: Option<&RateChangeParams>,
    sv_normalized: bool,
) -> String {
    let mut parts = vec![original_version.to_string()];

    if let Some(params) = rate {
        let mut rate_part = fmt_rate(params.rate);
        if let Some(bpm) = main_bpm(beatmap) {
            rate_part.push_str(&format!(" ({}bpm)", bpm.round()));
        }
        if let Some(tag) = pitch_tag(params) {
            rate_part.push_str(&format!(" ({})", tag));
        }
        parts.push(rate_part);
    }

    if sv_normalized {
        parts.push("(SV normalized)".to_string());
    }

    let stats = [
        ("HP", original.hp, new.hp),
        ("CS", original.cs, new.cs),
        ("AR", original.ar, new.ar),
        ("OD", original.od, new.od),
    ];
    for (label, before, after) in stats {
        if (before - after).abs() > 1e-3 {
            parts.push(format!("{}{}", label, fmt_num(after, 1)));
        }
    }

    if parts.len() == 1 {
        parts.push("(trainer)".to_string());
    }

    parts.join(" ")
}

/// `Artist - Title (Creator) [Version].osu`, the naming osu! itself uses.
pub fn osu_filename(beatmap: &Beatmap) -> String {
    sanitize_filename(&format!(
        "{} - {} ({}) [{}].osu",
        beatmap.artist, beatmap.title, beatmap.creator, beatmap.version
    ))
}

/// `audio 1.3x.ogg`, or `audio 1.3x (pitch raised).ogg` when resampled: each
/// (rate, pitch) combination gets its own file.
pub fn audio_filename(params: &RateChangeParams) -> String {
    let mut name = format!("audio {}", fmt_rate(params.rate));
    if let Some(tag) = pitch_tag(params) {
        name.push_str(&format!(" ({})", tag));
    }
    name.push_str(".ogg");
    name
}

/// Whether `name` is an audio file written by an earlier run (see
/// [`audio_filename`]), so it is not copied again into a new set.
pub fn is_generated_audio(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("audio ").and_then(|r| r.strip_suffix(".ogg")) else {
        return false;
    };
    let rate = rest
        .strip_suffix(" (pitch raised)")
        .or_else(|| rest.strip_suffix(" (pitch lowered)"))
        .unwrap_or(rest);

    rate.strip_suffix('x').is_some_and(|r| r.parse::<f64>().is_ok())
}

/// Containers the video is re-encoded into (as H.264). Others, like .mpg,
/// cannot hold H.264 and become .mp4.
const VIDEO_EXTENSIONS: [&str; 4] = ["avi", "mp4", "flv", "m4v"];

/// `video 1.5x.avi` next to the original `video.avi` (same folder inside the
/// set), so the other diffs keep their video.
pub fn video_filename(original: &str, params: &RateChangeParams) -> String {
    let original = original.replace('\\', "/");
    let (dir, file) = original.rsplit_once('/').map_or(("", original.as_str()), |(d, f)| (d, f));
    let (stem, ext) = file.rsplit_once('.').unwrap_or((file, ""));

    let ext = ext.to_lowercase();
    let ext = if VIDEO_EXTENSIONS.contains(&ext.as_str()) { ext.as_str() } else { "mp4" };

    let name = format!("{} {}.{}", stem, fmt_rate(params.rate), ext);
    if dir.is_empty() { name } else { format!("{}/{}", dir, name) }
}

/// Whether `name` is a video written by an earlier run (see [`video_filename`]).
pub fn is_generated_video(name: &str) -> bool {
    name.rsplit_once('.')
        .filter(|(_, ext)| VIDEO_EXTENSIONS.contains(ext))
        .and_then(|(rest, _)| rest.rsplit_once(' '))
        .and_then(|(_, rate)| rate.strip_suffix('x'))
        .is_some_and(|r| r.parse::<f64>().is_ok())
}

/// Title of the new set the diff goes to when the storyboard is scaled, e.g.
/// `Lagtrain (1.5x SB)`: one set per rate, apart from the original.
pub fn storyboard_set_title(title: &str, params: &RateChangeParams) -> String {
    format!("{} ({} SB)", title, fmt_rate(params.rate))
}

/// `Artist - Title.osz`. Without the leading set ID of the original folder,
/// so osu! does not tie it to the original set.
pub fn new_set_osz_name(beatmap: &Beatmap) -> String {
    sanitize_filename(&format!("{} - {}.osz", beatmap.artist, beatmap.title))
}

/// Drops the characters Windows does not allow in file names (osu! runs there
/// or under Wine), so the file inside the .osz can always be extracted.
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') && !c.is_control())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_numbers_without_noise() {
        assert_eq!(fmt_num(1.3, 2), "1.3");
        assert_eq!(fmt_num(1.25, 2), "1.25");
        assert_eq!(fmt_num(1.0, 2), "1");
        assert_eq!(fmt_num(1.1500000000000001, 2), "1.15");
        assert_eq!(fmt_num(9.5, 1), "9.5");
        assert_eq!(fmt_num(10.0, 1), "10");
    }

    #[test]
    fn audio_name_depends_on_pitch() {
        let ap = RateChangeParams { rate: 1.3, adjust_pitch: true, scale_storyboard: false, scale_video: false };
        let up = RateChangeParams { rate: 1.3, adjust_pitch: false, scale_storyboard: false, scale_video: false };
        let down = RateChangeParams { rate: 0.8, adjust_pitch: false, scale_storyboard: false, scale_video: false };
        assert_eq!(audio_filename(&ap), "audio 1.3x.ogg");
        assert_eq!(audio_filename(&up), "audio 1.3x (pitch raised).ogg");
        assert_eq!(audio_filename(&down), "audio 0.8x (pitch lowered).ogg");
    }

    #[test]
    fn recognizes_generated_audio() {
        assert!(is_generated_audio("audio 1.5x (pitch raised).ogg"));
        assert!(is_generated_audio("audio 0.8x (pitch lowered).ogg"));
        assert!(is_generated_audio("audio 1.25x.ogg"));
        assert!(!is_generated_audio("audio.mp3"));
        assert!(!is_generated_audio("audio extra.ogg"));
        assert!(!is_generated_audio("sb/audio 1.5x.ogg"));
    }

    #[test]
    fn video_name_keeps_folder_and_container() {
        let p = RateChangeParams { rate: 1.5, adjust_pitch: false, scale_storyboard: false, scale_video: true };
        assert_eq!(video_filename("bga.avi", &p), "bga 1.5x.avi");
        assert_eq!(video_filename("sb\\my video.FLV", &p), "sb/my video 1.5x.flv");
        assert_eq!(video_filename("bga.mpg", &p), "bga 1.5x.mp4");
        assert!(is_generated_video("bga 1.5x.avi"));
        assert!(is_generated_video("bga 1.5x.mp4"));
        assert!(!is_generated_video("bga.avi"));
    }

    #[test]
    fn strips_illegal_filename_chars() {
        assert_eq!(sanitize_filename("a/b: c? [d*].osu"), "ab c [d].osu");
    }
}

