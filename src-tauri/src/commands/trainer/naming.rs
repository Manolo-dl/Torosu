//! Names for everything the trainer writes: the diff name (`Version`), the
//! `.osu` file and the generated audio.
//!
//! Every variant that would produce different audio or a different diff gets a
//! different name. Otherwise, when osu! merges the .osz into the set folder, the
//! new files overwrite the ones from an earlier run.

use rosu_map::Beatmap;

use crate::commands::trainer::model::{
    Density, DifficultySettings, FlnConverterParams, KeyConverterParams, LengthUnit, NoteLength, RateChangeParams,
};

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

/// The name format the trainer starts with, which gives the names it always
/// gave, e.g. `Insane 1.3x (234bpm) (pitch raised) OD8`.
pub const DEFAULT_NAME_FORMAT: &str = "{diff} {rate} ({bpm}bpm) ({pitch}) {tasks} {stats}";

/// What a convert changed, for its name.
pub struct NameInfo<'a> {
    pub original_version: &'a str,
    /// The difficulty settings before and after.
    pub original: &'a DifficultySettings,
    pub new: &'a DifficultySettings,
    pub rate: Option<&'a RateChangeParams>,
    /// The BPM after the tasks (see `timing::main_bpm`).
    pub bpm: Option<f64>,
    pub sv_normalized: bool,
    pub fln: Option<&'a FlnConverterParams>,
    pub keys: Option<&'a KeyConverterParams>,
}

/// Builds the new `Version` from `format`, free text with these placeholders:
///
/// - `{diff}`: the original name, `Insane`
/// - `{rate}`: `1.3x`; `{bpm}`: `234`; `{pitch}`: `pitch raised` or `pitch lowered`
/// - `{sv}`: `SV normalized`; `{keys}`: `10K Dense`; `{fln}`: `FLN BG4 ML4`
/// - `{tasks}`: those three in parentheses, `(SV normalized) (10K Dense)`
/// - `{stats}`: the difficulty settings that changed, `AR9.5 OD8`
///
/// Empty placeholders vanish, and so do the `(...)` and `[...]` around them
/// when every placeholder inside is empty: `({bpm}bpm)` without a rate change.
/// A name equal to the original, which would overwrite its .osu, gets
/// ` (trainer)`.
pub fn version(format: &str, info: &NameInfo) -> String {
    let sv = info.sv_normalized.then(|| "SV normalized".to_string());
    let keys = info.keys.map(key_tag);
    let fln = info.fln.map(fln_tag);

    let value = |field: &str| -> Option<String> {
        match field {
            "diff" => Some(info.original_version.to_string()),
            "rate" => info.rate.map(|p| fmt_rate(p.rate)),
            "bpm" => info.rate.and(info.bpm).map(|bpm| fmt_num(bpm.round(), 0)),
            "pitch" => info.rate.and_then(pitch_tag).map(str::to_string),
            "sv" => sv.clone(),
            "keys" => keys.clone(),
            "fln" => fln.clone(),
            "tasks" => {
                let tags: Vec<String> = [&sv, &keys, &fln].into_iter().flatten().map(|tag| format!("({})", tag)).collect();
                (!tags.is_empty()).then(|| tags.join(" "))
            }
            "stats" => {
                let stats: Vec<String> = [
                    ("HP", info.original.hp, info.new.hp),
                    ("CS", info.original.cs, info.new.cs),
                    ("AR", info.original.ar, info.new.ar),
                    ("OD", info.original.od, info.new.od),
                ]
                .into_iter()
                .filter(|(_, before, after)| (before - after).abs() > 1e-3)
                .map(|(label, _, after)| format!("{}{}", label, fmt_num(after, 1)))
                .collect();
                (!stats.is_empty()).then(|| stats.join(" "))
            }
            _ => None,
        }
    };

    let name = render_template(format, &value);
    if name.is_empty() || name == info.original_version {
        format!("{} (trainer)", info.original_version)
    } else {
        name
    }
}

const NAME_FIELDS: [&str; 9] = ["diff", "rate", "bpm", "pitch", "sv", "keys", "fln", "tasks", "stats"];

/// A part of a name format.
enum Part {
    Text(String),
    Field(String),
    /// `(...)` or `[...]`.
    Group(char, Vec<Part>, char),
}

fn parse_template(format: &str) -> Vec<Part> {
    // The groups being read, innermost last; the bottom one is the whole format.
    let mut stack: Vec<(Option<char>, Vec<Part>)> = vec![(None, Vec::new())];
    let push_text = |parts: &mut Vec<Part>, text: &str| match parts.last_mut() {
        Some(Part::Text(last)) => last.push_str(text),
        _ => parts.push(Part::Text(text.to_string())),
    };

    let mut chars = format.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let closes = match c {
            ')' => Some('('),
            ']' => Some('['),
            _ => None,
        };
        match c {
            '(' | '[' => stack.push((Some(c), Vec::new())),
            _ if closes.is_some() && stack.len() > 1 && stack.last().unwrap().0 == closes => {
                let (open, parts) = stack.pop().unwrap();
                stack.last_mut().unwrap().1.push(Part::Group(open.unwrap(), parts, c));
            }
            '{' => {
                let rest = &format[i + 1..];
                match rest.find('}').map(|end| &rest[..end]).filter(|name| NAME_FIELDS.contains(name)) {
                    Some(name) => {
                        stack.last_mut().unwrap().1.push(Part::Field(name.to_string()));
                        for _ in 0..=name.len() {
                            chars.next();
                        }
                    }
                    // Not a placeholder: the user's own text.
                    None => push_text(&mut stack.last_mut().unwrap().1, "{"),
                }
            }
            _ => push_text(&mut stack.last_mut().unwrap().1, c.encode_utf8(&mut [0; 4])),
        }
    }

    // A group that is never closed was text.
    while stack.len() > 1 {
        let (open, parts) = stack.pop().unwrap();
        let parent = &mut stack.last_mut().unwrap().1;
        push_text(parent, open.unwrap().encode_utf8(&mut [0; 4]));
        for part in parts {
            match part {
                Part::Text(text) => push_text(parent, &text),
                other => parent.push(other),
            }
        }
    }
    stack.pop().unwrap().1
}

/// The text of `parts`, how many placeholders they have and how many of
/// those are filled.
fn render_parts(parts: &[Part], value: &dyn Fn(&str) -> Option<String>) -> (String, usize, usize) {
    let mut out = String::new();
    let (mut fields, mut filled) = (0, 0);
    for part in parts {
        match part {
            Part::Text(text) => out.push_str(text),
            Part::Field(name) => {
                fields += 1;
                if let Some(v) = value(name) {
                    filled += 1;
                    out.push_str(&v);
                }
            }
            Part::Group(open, inner, close) => {
                let (text, f, ok) = render_parts(inner, value);
                fields += f;
                filled += ok;
                if f == 0 || ok > 0 {
                    out.push(*open);
                    out.push_str(&text);
                    out.push(*close);
                }
            }
        }
    }
    (out, fields, filled)
}

fn render_template(format: &str, value: &dyn Fn(&str) -> Option<String>) -> String {
    let (text, _, _) = render_parts(&parse_template(format), value);

    // One space between words, and none right inside the brackets that are left.
    let mut out = String::new();
    for word in text.split_whitespace() {
        if !out.is_empty() && !out.ends_with(['(', '[']) && !word.starts_with([')', ']']) {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

/// What `format` gives for a sample convert: Insane at 1.3x without adjusting
/// the pitch, with the SV normalized and OD 8, to preview it in the settings.
#[tauri::command]
pub fn trainer_name_example(format: String) -> String {
    let original = DifficultySettings { hp: 5.0, cs: 4.0, od: 7.0, ar: 9.0 };
    let new = DifficultySettings { od: 8.0, ..original };
    let rate = RateChangeParams { rate: 1.3, adjust_pitch: false, scale_storyboard: false, scale_video: false };
    version(&format, &NameInfo {
        original_version: "Insane",
        original: &original,
        new: &new,
        rate: Some(&rate),
        bpm: Some(234.0),
        sv_normalized: true,
        fln: None,
        keys: None,
    })
}

/// `10K`, `10K More Dense`, `8K Full 1+7`, `7K, split jacks`... Every setting
/// that changes the notes is in the name.
fn key_tag(params: &KeyConverterParams) -> String {
    let mut tag = format!("{}K", params.target_keys);
    match params.density {
        Density::None => {}
        Density::Dense => tag.push_str(" Dense"),
        Density::MoreDense => tag.push_str(" More Dense"),
        Density::Full => tag.push_str(" Full"),
    }
    if let Some(playstyle) = &params.playstyle {
        tag.push_str(&format!(" {}", playstyle));
    }
    if !params.keep_jacks {
        tag.push_str(", split jacks");
    }
    tag
}

/// `FLN` reworks the existing LNs too, `INV` keeps them. `BG` is the gap and
/// `ML` the minimum length, as a beat divisor (`BG4` is 1/4 beat) or in ms.
fn fln_tag(params: &FlnConverterParams) -> String {
    let length = |l: NoteLength| match l.unit {
        LengthUnit::Beat => fmt_num(l.value, 2),
        LengthUnit::Ms => format!("{}ms", fmt_num(l.value, 0)),
    };
    let kind = if params.rework_existing_ln { "FLN" } else { "INV" };
    format!("{} BG{} ML{}", kind, length(params.gap), length(params.min_length))
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

    /// The name of Insane (OD 7, 234 BPM after the rate) with these changes.
    fn name(format: &str, rate: Option<&RateChangeParams>, od: f64, sv_normalized: bool, keys: Option<&KeyConverterParams>) -> String {
        let original = DifficultySettings { hp: 5.0, cs: 4.0, od: 7.0, ar: 9.0 };
        let new = DifficultySettings { od, ..original };
        version(format, &NameInfo {
            original_version: "Insane",
            original: &original,
            new: &new,
            rate,
            bpm: Some(234.0),
            sv_normalized,
            fln: None,
            keys,
        })
    }

    #[test]
    fn default_format_gives_the_usual_names() {
        let raised = RateChangeParams { rate: 1.3, adjust_pitch: false, scale_storyboard: false, scale_video: false };
        let kept = RateChangeParams { adjust_pitch: true, ..raised.clone() };
        let keys = KeyConverterParams { target_keys: 10, density: Density::Dense, keep_jacks: true, playstyle: None };
        let f = DEFAULT_NAME_FORMAT;

        assert_eq!(name(f, Some(&raised), 8.0, false, None), "Insane 1.3x (234bpm) (pitch raised) OD8");
        assert_eq!(name(f, Some(&kept), 7.0, false, None), "Insane 1.3x (234bpm)");
        assert_eq!(name(f, None, 7.0, true, Some(&keys)), "Insane (SV normalized) (10K Dense)");
        assert_eq!(name(f, None, 8.0, false, None), "Insane OD8");
        assert_eq!(name(f, None, 7.0, false, None), "Insane (trainer)");
    }

    #[test]
    fn brackets_vanish_with_their_empty_placeholders() {
        let kept = RateChangeParams { rate: 1.3, adjust_pitch: true, scale_storyboard: false, scale_video: false };

        assert_eq!(name("[{rate}] {diff} ({stats})", None, 8.0, false, None), "Insane (OD8)");
        assert_eq!(name("[{rate}] {diff}", Some(&kept), 7.0, false, None), "[1.3x] Insane");
        assert_eq!(name("{diff} ({rate} {pitch})", Some(&kept), 7.0, false, None), "Insane (1.3x)");
        assert_eq!(name("{diff} ({bpm} BPM, {sv})", Some(&kept), 7.0, false, None), "Insane (234 BPM,)");
    }

    #[test]
    fn keeps_the_users_own_text() {
        let kept = RateChangeParams { rate: 1.3, adjust_pitch: true, scale_storyboard: false, scale_video: false };

        assert_eq!(name("{diff} - practice {rate}", Some(&kept), 7.0, false, None), "Insane - practice 1.3x");
        assert_eq!(name("{diff} (mine) {rate}", None, 8.0, false, None), "Insane (mine)");
        // Unknown placeholders and unclosed brackets are text too.
        assert_eq!(name("{diff} {foo} ({rate}", Some(&kept), 7.0, false, None), "Insane {foo} (1.3x");
        // Nothing but empty placeholders: the original name is not reused.
        assert_eq!(name("{rate}", None, 7.0, false, None), "Insane (trainer)");
    }

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

