//! Storyboard support for the rate change.
//!
//! rosu-map only keeps the background and the breaks from `[Events]`, so the
//! storyboard (and the video) are read straight from the raw `.osu`/`.osb`,
//! their times are scaled and the lines are injected back into the encoded diff.
//!
//! Every time in a storyboard is linear in the song time: commands inside loops
//! and triggers are relative to their parent, but dividing both by the rate
//! keeps them consistent, so all of them are simply divided by the rate.

use std::{collections::HashMap, fs, path::Path};

use anyhow::Result;

/// Reads the `[Events]` of a `.osu` or `.osb`, with the `[Variables]` already
/// expanded. Background and break lines are left out: rosu-map encodes those.
pub fn read_events(path: &Path) -> Result<Vec<String>> {
    let bytes = fs::read(path)?;
    let content = String::from_utf8_lossy(&bytes);

    let mut section = String::new();
    let mut variables: Vec<(String, String)> = Vec::new();
    let mut events = Vec::new();

    for line in content.lines() {
        let line = line.trim_start_matches('\u{feff}').trim_end();
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed.to_string();
            continue;
        }

        match section.as_str() {
            "[Variables]" => {
                if let Some((name, value)) = trimmed.split_once('=') {
                    if name.starts_with('$') {
                        variables.push((name.to_string(), value.to_string()));
                    }
                }
            }
            "[Events]" if !trimmed.is_empty() && !trimmed.starts_with("//") => {
                if !is_encoded_by_rosu(line) {
                    events.push(line.to_string());
                }
            }
            _ => {}
        }
    }

    // Longest first, so `$ab` is not replaced by the value of `$a`.
    variables.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    if !variables.is_empty() {
        for line in events.iter_mut() {
            for (name, value) in &variables {
                if line.contains(name.as_str()) {
                    *line = line.replace(name.as_str(), value);
                }
            }
        }
    }

    Ok(events)
}

fn is_encoded_by_rosu(line: &str) -> bool {
    if line.starts_with([' ', '_']) {
        return false;
    }

    matches!(
        line.split(',').next().map(str::trim),
        Some("0" | "Background" | "2" | "Break")
    )
}

/// Divides every time in the event lines by `rate`.
pub fn scale_events(events: &[String], rate: f64) -> Vec<String> {
    events.iter().map(|line| scale_line(line, rate)).collect()
}

fn scale_line(line: &str, rate: f64) -> String {
    let body = line.trim_start_matches([' ', '_']);
    let indent = &line[..line.len() - body.len()];

    let mut fields = split_fields(body);
    let Some(kind) = fields.first().map(|f| f.trim().to_string()) else {
        return line.to_string();
    };

    // Indices of the fields holding times (or durations) for each line type.
    let time_fields: &[usize] = if indent.is_empty() {
        match kind.as_str() {
            // Video,offset,"file"
            "1" | "Video" => &[1],
            // Colour,time,r,g,b (old background colour)
            "3" | "Colour" => &[1],
            // Sample,time,layer,"path",volume
            "5" | "Sample" => &[1],
            // Animation,layer,origin,"path",x,y,frameCount,frameDelay[,looptype]
            "6" | "Animation" => &[7],
            _ => &[],
        }
    } else {
        match kind.as_str() {
            // L,startTime,loopCount
            "L" => &[1],
            // T,triggerName,startTime,endTime[,groupNumber]
            "T" => &[2, 3],
            // event,easing,startTime,endTime,params...
            _ => &[2, 3],
        }
    };

    let mut changed = false;
    for &i in time_fields {
        let Some(field) = fields.get_mut(i) else { continue };
        // endTime can be empty: it then equals startTime.
        let Ok(value) = field.trim().parse::<f64>() else { continue };

        let scaled = value / rate;
        // The frame delay is a float in osu!; every other time is an integer.
        *field = if kind == "6" || kind == "Animation" {
            fmt_float(scaled)
        } else {
            format!("{}", scaled.round() as i64)
        };
        changed = true;
    }

    if !changed {
        return line.to_string();
    }

    format!("{}{}", indent, fields.join(","))
}

/// Splits on commas that are not inside quotes (file paths may contain them).
fn split_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for c in line.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                current.push(c);
            }
            ',' if !in_quotes => fields.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    fields.push(current);

    fields
}

fn fmt_float(value: f64) -> String {
    let s = format!("{:.4}", value);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Appends `events` at the end of the `[Events]` section of an encoded `.osu`.
pub fn inject_events(osu: String, events: &[String]) -> String {
    const HEADER: &str = "[Events]\n";

    if events.is_empty() {
        return osu;
    }

    let Some(start) = osu.find(HEADER) else {
        log::warn!("no [Events] section in encoded beatmap, storyboard dropped");
        return osu;
    };

    let body_start = start + HEADER.len();
    // Sections are separated by a blank line, so this lands on that blank line
    // and the last event line keeps its own newline.
    let at = osu[body_start..]
        .find("\n[")
        .map_or(osu.len(), |p| body_start + p);

    let mut out = String::with_capacity(osu.len() + events.iter().map(|e| e.len() + 1).sum::<usize>());
    out.push_str(&osu[..at]);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    for line in events {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(&osu[at..]);
    out
}

/// The `.osb` of the set: osu! names it `Artist - Title (Creator).osb`, but any
/// other one in the folder is taken if that one does not exist.
pub fn find_osb(folder: &Path, artist: &str, title: &str, creator: &str) -> Option<std::path::PathBuf> {
    let expected = format!("{} - {} ({}).osb", artist, title, creator).to_lowercase();

    let mut candidates: Vec<_> = fs::read_dir(folder)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("osb"))
        })
        .collect();

    let names: HashMap<_, _> = candidates
        .iter()
        .enumerate()
        .filter_map(|(i, p)| Some((p.file_name()?.to_string_lossy().to_lowercase(), i)))
        .collect();

    match names.get(&expected) {
        Some(&i) => Some(candidates.swap_remove(i)),
        None => {
            candidates.sort();
            candidates.into_iter().next()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_object_and_command_times() {
        let events = vec![
            "Sprite,Foreground,Centre,\"sb/a,b.png\",320,240".to_string(),
            " F,0,1300,2600,0,1".to_string(),
            " M,1,1300,,320,240,0,0".to_string(),
            " L,1300,4".to_string(),
            "  F,0,0,130,1,0".to_string(),
            " T,HitSoundClap,1300,2600".to_string(),
            "_S,0,0,1300,1".to_string(),
        ];

        assert_eq!(
            scale_events(&events, 1.3),
            vec![
                "Sprite,Foreground,Centre,\"sb/a,b.png\",320,240",
                " F,0,1000,2000,0,1",
                " M,1,1000,,320,240,0,0",
                " L,1000,4",
                "  F,0,0,100,1,0",
                " T,HitSoundClap,1000,2000",
                "_S,0,0,1000,1",
            ]
        );
    }

    #[test]
    fn scales_video_sample_and_frame_delay() {
        let events = vec![
            "Video,-260,\"video.mp4\"".to_string(),
            "Sample,1300,0,\"hit.wav\",70".to_string(),
            "Animation,Background,Centre,\"anim.png\",0,0,4,100,LoopForever".to_string(),
        ];

        assert_eq!(
            scale_events(&events, 1.3),
            vec![
                "Video,-200,\"video.mp4\"",
                "Sample,1000,0,\"hit.wav\",70",
                "Animation,Background,Centre,\"anim.png\",0,0,4,76.9231,LoopForever",
            ]
        );
    }

    #[test]
    fn injects_at_end_of_events() {
        let osu = "[Events]\n0,0,\"bg.jpg\",0,0\n\n[TimingPoints]\n".to_string();
        let out = inject_events(osu, &["Video,0,\"v.mp4\"".to_string()]);
        assert_eq!(out, "[Events]\n0,0,\"bg.jpg\",0,0\nVideo,0,\"v.mp4\"\n\n[TimingPoints]\n");
    }
}
