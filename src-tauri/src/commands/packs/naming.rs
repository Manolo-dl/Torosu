//! Names of the diffs added to a pack, from the name format in the settings
//! (see `name_format`).

use serde::Deserialize;

use crate::name_format;

/// The name the pack creator always gave: `[Mapper] Artist - Title (Insane)`.
pub const DEFAULT_PACK_NAME_FORMAT: &str = "[{creator}] {artist} - {title} ({diff})";

const FIELDS: [&str; 7] = ["creator", "artist", "title", "diff", "stars", "mode", "keys"];

const MODE_NAMES: [&str; 4] = ["osu!", "osu!taiko", "osu!catch", "osu!mania"];

/// What a diff is called from, as the frontend has it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackDiff {
    pub creator: String,
    pub artist: String,
    pub title: String,
    pub diff_name: String,
    pub stars: f64,
    pub mode: u8,
    /// The key count in mania.
    pub circle_size: f64,
}

/// The name of `diff` in the pack, from `format`:
///
/// - `{creator}`: who mapped the diff; `{artist}`, `{title}`: the song's
/// - `{diff}`: the diff's own name
/// - `{stars}`: `5.23`; `{mode}`: `osu!mania`; `{keys}`: `7K`, only in mania
///
/// A format that leaves nothing gives the default name instead.
pub fn diff_name(format: &str, diff: &PackDiff) -> String {
    let text = |s: &str| (!s.trim().is_empty()).then(|| s.to_string());
    let value = |field: &str| -> Option<String> {
        match field {
            "creator" => text(&diff.creator),
            "artist" => text(&diff.artist),
            "title" => text(&diff.title),
            "diff" => text(&diff.diff_name),
            "stars" => Some(format!("{:.2}", diff.stars)),
            "mode" => MODE_NAMES.get(diff.mode as usize).map(|name| name.to_string()),
            "keys" => (diff.mode == 3).then(|| format!("{}K", diff.circle_size.round())),
            _ => None,
        }
    };

    let name = name_format::render(format, &FIELDS, &value);
    if name.is_empty() {
        name_format::render(DEFAULT_PACK_NAME_FORMAT, &FIELDS, &value)
    } else {
        name
    }
}

/// The names of `diffs` in the pack, in their order.
#[tauri::command]
pub fn pack_diff_names(format: String, diffs: Vec<PackDiff>) -> Vec<String> {
    diffs.iter().map(|diff| diff_name(&format, diff)).collect()
}

/// What `format` gives for a sample diff, to preview it in the settings.
#[tauri::command]
pub fn pack_name_example(format: String) -> String {
    diff_name(&format, &sample())
}

fn sample() -> PackDiff {
    PackDiff {
        creator: "Mapper".into(),
        artist: "Artist".into(),
        title: "Title".into(),
        diff_name: "Insane".into(),
        stars: 5.23,
        mode: 3,
        circle_size: 7.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_format_gives_the_usual_name() {
        assert_eq!(diff_name(DEFAULT_PACK_NAME_FORMAT, &sample()), "[Mapper] Artist - Title (Insane)");
    }

    #[test]
    fn fills_every_placeholder() {
        assert_eq!(diff_name("{artist} - {title} [{diff}] {stars}* {mode} {keys}", &sample()), "Artist - Title [Insane] 5.23* osu!mania 7K");
        // No key count outside mania, and its brackets go with it.
        let taiko = PackDiff { mode: 1, ..sample() };
        assert_eq!(diff_name("{diff} ({keys})", &taiko), "Insane");
    }

    #[test]
    fn an_empty_name_falls_back_to_the_default() {
        assert_eq!(diff_name("", &sample()), "[Mapper] Artist - Title (Insane)");
        let no_creator = PackDiff { creator: String::new(), ..sample() };
        assert_eq!(diff_name("{creator}", &no_creator), "Artist - Title (Insane)");
    }
}
