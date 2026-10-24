use std::{collections::HashMap, fs, path::{Path, PathBuf}};
use crate::AppConfigState;
use crate::commands::osu_db::folders::get_beatmapset_folder;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedDiff {
    pub folder_name: String,
    pub file_name: String,
    pub audio: String,
    pub new_diff_name: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackRequest {
    pub title: String,
    pub final_creator: String,
    pub diffs: Vec<SelectedDiff>,
}

#[tauri::command]
pub async fn create_pack(
    request: PackRequest,
    config_state: tauri::State<'_, AppConfigState>,
) -> Result<(), String> {

    log::debug!("create_pack called with title={}, creator={}, {} diffs", request.title, request.final_creator, request.diffs.len());

    let (osu_path, packs) = {
        let config = config_state.config.lock().unwrap();
        (config.osu_path.clone().ok_or("osu! path not set")?, config.packs.clone())
    };

    let parts = split_by_set(&request.diffs, packs.max_sets_per_pack as usize);
    if parts.len() > 1 {
        log::info!("splitting {} diffs into {} packs (max {} sets per pack)", request.diffs.len(), parts.len(), packs.max_sets_per_pack);
    }

    let mut osz_paths = Vec::with_capacity(parts.len());
    for (index, diffs) in parts.iter().enumerate() {
        let title = part_title(&request.title, index, parts.len());
        let osz_path = build_osz(&osu_path, &title, &request.final_creator, diffs, packs.keep_background).await?;
        osz_paths.push(osz_path);
    }

    for osz_path in &osz_paths {
        open_osz(osz_path)?;
    }

    log::info!("successfully created pack '{}' with {} diffs in {} set(s)", request.title, request.diffs.len(), osz_paths.len());

    Ok(())
}

/// Splits the diffs into packs of at most `max` original sets each. What makes
/// a pack heavy is each set's own audio and background, not its diffs, so the
/// diffs of a set always stay together. `max` 0 never splits.
fn split_by_set(diffs: &[SelectedDiff], max: usize) -> Vec<Vec<&SelectedDiff>> {
    let mut sets: Vec<Vec<&SelectedDiff>> = Vec::new();
    for diff in diffs {
        match sets.iter_mut().find(|set| set[0].folder_name == diff.folder_name) {
            Some(set) => set.push(diff),
            None => sets.push(vec![diff]),
        }
    }

    split_evenly(&sets, max)
        .into_iter()
        .map(|part| part.iter().flatten().copied().collect())
        .collect()
}

/// Splits `items` into the fewest parts of at most `max` items, as even as
/// possible (101 with a max of 100 gives 51 + 50). `max` 0 never splits.
fn split_evenly<T>(items: &[T], max: usize) -> Vec<&[T]> {
    if max == 0 || items.len() <= max {
        return vec![items];
    }

    let parts = items.len().div_ceil(max);
    items.chunks(items.len().div_ceil(parts)).collect()
}

fn part_title(title: &str, index: usize, parts: usize) -> String {
    if parts > 1 {
        format!("{} - Part {}", title, index + 1)
    } else {
        title.to_string()
    }
}

/// Writes one set with `diffs` into an .osz in the temp folder and returns its path.
async fn build_osz(
    osu_path: &str,
    title: &str,
    creator: &str,
    diffs: &[&SelectedDiff],
    keep_background: bool,
) -> Result<PathBuf, String> {

    let safe_title = sanitize(title);
    let temp_dir = std::env::temp_dir().join(format!("osu-pack-{}", safe_title));

    // Leftovers of a failed run would end up in the .osz.
    if temp_dir.exists() {
        let _ = fs::remove_dir_all(&temp_dir);
    }

    log::debug!("creating temp directory at {:?}", temp_dir);
    fs::create_dir_all(&temp_dir).map_err(|e| {
        log::error!("failed to create temp directory {:?}: {}", temp_dir, e);
        e.to_string()
    })?;

    // Diffs of the same set share their audio and background: copied once.
    let mut copied: HashMap<PathBuf, String> = HashMap::new();

    for (index, diff) in diffs.iter().enumerate() {
        log::debug!("processing diff {}/{}: folder_name={}, file_name={}", index + 1, diffs.len(), diff.folder_name, diff.file_name);

        let folder = get_beatmapset_folder(osu_path, &diff.folder_name)?;
        let folder = Path::new(&folder.folder_path);
        let osu_file_path = folder.join(&diff.file_name);

        let content = fs::read_to_string(&osu_file_path).map_err(|e| {
            log::error!("failed to read {:?}: {}", osu_file_path, e);
            format!("Failed to read {}: {}", diff.file_name, e)
        })?;

        let audio = match find_file(folder, &diff.audio) {
            Some(source) => Some(copy_once(&mut copied, &source, "audio", &temp_dir).await?),
            None => {
                log::warn!("audio file not found for diff {}: {:?}", diff.file_name, folder.join(&diff.audio));
                None
            }
        };

        let background = match background_filename(&content).filter(|_| keep_background) {
            Some(name) => match find_file(folder, &name) {
                Some(source) => Some(copy_once(&mut copied, &source, "bg", &temp_dir).await?),
                None => {
                    log::warn!("background not found for diff {}: {:?}", diff.file_name, folder.join(&name));
                    None
                }
            },
            None => None,
        };

        let modified_content = modify_osu_metadata(
            &content,
            title,
            creator,
            &diff.new_diff_name,
            audio.as_deref(),
            background.as_deref(),
        );

        let new_osu_name = format!("{}.osu", sanitize(&diff.new_diff_name));
        fs::write(temp_dir.join(&new_osu_name), modified_content).map_err(|e| {
            log::error!("failed to write {:?}: {}", temp_dir.join(&new_osu_name), e);
            e.to_string()
        })?;

        log::debug!("wrote modified .osu file: {}", new_osu_name);
    }

    let osz_path = std::env::temp_dir().join(format!("{}.osz", safe_title));
    log::debug!("creating osz archive at {:?}", osz_path);

    create_osz(&temp_dir, &osz_path).map_err(|e| {
        log::error!("failed to create osz archive {:?}: {}", osz_path, e);
        e
    })?;

    if let Err(e) = fs::remove_dir_all(&temp_dir) {
        log::warn!("failed to clean up temp directory {:?}: {}", temp_dir, e);
    } else {
        log::debug!("cleaned up temp directory {:?}", temp_dir);
    }

    Ok(osz_path)
}

/// Copies `source` into the pack the first time a diff uses it and returns
/// its name inside the pack (`audio-1.mp3`, `bg-2.jpg`, ...).
async fn copy_once(copied: &mut HashMap<PathBuf, String>, source: &Path, prefix: &str, temp_dir: &Path) -> Result<String, String> {
    if let Some(name) = copied.get(source) {
        return Ok(name.clone());
    }

    let number = copied.values().filter(|name| name.starts_with(&format!("{prefix}-"))).count() + 1;
    let name = match source.extension().and_then(|e| e.to_str()) {
        Some(extension) => format!("{prefix}-{number}.{extension}"),
        None => format!("{prefix}-{number}"),
    };

    let destination = temp_dir.join(&name);
    tokio::fs::copy(source, &destination).await.map_err(|e| {
        log::error!("failed to copy {:?} -> {:?}: {}", source, destination, e);
        e.to_string()
    })?;
    log::debug!("copied {:?} to {:?}", source, destination);

    copied.insert(source.to_path_buf(), name.clone());
    Ok(name)
}

/// `name` inside `folder` as the .osu file writes it. osu! runs on Windows (or
/// Wine), so a different case or `\` separators still find the file.
fn find_file(folder: &Path, name: &str) -> Option<PathBuf> {
    let path = folder.join(name.replace('\\', "/"));
    if path.is_file() {
        return Some(path);
    }

    let parent = path.parent()?;
    let file_name = path.file_name()?.to_str()?;
    fs::read_dir(parent).ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|candidate| {
            candidate.is_file()
                && candidate.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.eq_ignore_ascii_case(file_name))
        })
}

/// An `[Events]` line that points to a file: `type,start,"file"...`.
struct FileEvent<'a> {
    kind: &'a str,
    start: &'a str,
    file: &'a str,
    /// Whatever follows the file name (the offsets), starting with its comma.
    rest: &'a str,
}

impl FileEvent<'_> {
    fn parse(line: &str) -> Option<FileEvent<'_>> {
        let (kind, rest) = line.split_once(',')?;
        let (start, rest) = rest.split_once(',')?;

        let (file, rest) = match rest.strip_prefix('"') {
            Some(quoted) => {
                let end = quoted.find('"')?;
                (&quoted[..end], &quoted[end + 1..])
            }
            None => rest.split_at(rest.find(',').unwrap_or(rest.len())),
        };

        Some(FileEvent { kind: kind.trim(), start, file, rest })
    }

    fn is_background(&self) -> bool {
        self.kind == "0" || self.kind.eq_ignore_ascii_case("Background")
    }

    fn is_video(&self) -> bool {
        self.kind == "1" || self.kind.eq_ignore_ascii_case("Video")
    }
}

/// File name of the background image, as written in the `[Events]` section.
fn background_filename(content: &str) -> Option<String> {
    let mut in_events_section = false;

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            in_events_section = trimmed.starts_with("[Events]");
            continue;
        }

        if in_events_section && !trimmed.starts_with("//") {
            if let Some(event) = FileEvent::parse(trimmed).filter(|e| e.is_background()) {
                return Some(event.file.to_string()).filter(|f| !f.is_empty());
            }
        }
    }

    None
}

/// Rewrites the metadata for the pack. `audio_filename` and `background` are
/// the names of the copied files; the audio line is kept as is without one, and
/// the background is dropped. Videos are dropped too, since they aren't copied.
fn modify_osu_metadata(
    content: &str,
    title: &str,
    creator: &str,
    version: &str,
    audio_filename: Option<&str>,
    background: Option<&str>,
) -> String {
    let mut result = String::with_capacity(content.len());
    let mut in_events_section = false;

    for line in content.lines() {
        let trimmed = line.trim_start();

        if trimmed.starts_with('[') {
            in_events_section = trimmed.starts_with("[Events]");
            result.push_str(line);
            result.push('\n');
            continue;
        }

        if in_events_section && !trimmed.starts_with("//") {
            if let Some(event) = FileEvent::parse(trimmed) {
                if event.is_video() {
                    continue;
                }
                if event.is_background() {
                    if let Some(background) = background {
                        result.push_str(&format!("{},{},\"{}\"{}\n", event.kind, event.start, background, event.rest));
                    }
                    continue;
                }
            }
        }

        if let Some(audio) = audio_filename.filter(|_| trimmed.starts_with("AudioFilename:")) {
            result.push_str(&format!("AudioFilename: {}", audio));
        } else if trimmed.starts_with("Title:") && !trimmed.starts_with("TitleUnicode:") {
            result.push_str(&format!("Title:{}", title));
        } else if trimmed.starts_with("TitleUnicode:") {
            result.push_str(&format!("TitleUnicode:{}", title));
        } else if trimmed.starts_with("Creator:") {
            result.push_str(&format!("Creator:{}", creator));
        } else if trimmed.starts_with("Version:") {
            result.push_str(&format!("Version:{}", version));
        } else if trimmed.starts_with("BeatmapID:") {
            continue;
        } else if trimmed.starts_with("BeatmapSetID:") {
            continue;
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    result
}

fn open_osz(osz_path: &Path) -> Result<(), String> {
    log::debug!("opening osz with default handler: {:?}", osz_path);

    if let Err(e) = tauri_plugin_opener::open_path(osz_path, None::<&str>) {
        log::warn!("default handler failed to open .osz ({}), trying osu-wine --osuhandler", e);

        let osu_wine_result = std::process::Command::new("osu-wine")
            .arg("--osuhandler")
            .arg(osz_path)
            .spawn();

        match osu_wine_result {
            Ok(_) => {
                log::debug!("opened .osz via osu-wine --osuhandler");
            }
            Err(wine_err) => {
                log::error!("failed to open .osz with both default handler and osu-wine: {} / {}", e, wine_err);
                return Err(format!("Failed to open .osz: {}", e));
            }
        }
    } else {
        log::debug!("opened .osz with default handler");
    }

    Ok(())
}

fn create_osz(source_dir: &Path, output_path: &Path) -> Result<(), String> {
    use std::io::{Read, Write};
    use zip::write::SimpleFileOptions;

    let file = std::fs::File::create(output_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for entry in fs::read_dir(source_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.is_file() {
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .ok_or("Invalid file name")?;

            zip.start_file(file_name, options).map_err(|e| e.to_string())?;

            let mut f = std::fs::File::open(&path).map_err(|e| e.to_string())?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
            zip.write_all(&buffer).map_err(|e| e.to_string())?;
        }
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if "/\\:*?\"<>|".contains(c) { '_' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const OSU: &str = "osu file format v14

[General]
AudioFilename: song.mp3

[Metadata]
Title:Old
TitleUnicode:Old
Creator:someone
Version:Hard
BeatmapID:123
BeatmapSetID:45

[Events]
//Background and Video events
0,0,\"My BG.jpg\",0,0
Video,0,\"intro.mp4\"
//Storyboard Layer 0 (Background)

[TimingPoints]
0,500,4,2,0,100,1,0
";

    #[test]
    fn splits_evenly() {
        let items: Vec<u32> = (0..101).collect();
        let sizes: Vec<usize> = split_evenly(&items, 100).iter().map(|p| p.len()).collect();
        assert_eq!(sizes, [51, 50]);

        let sizes: Vec<usize> = split_evenly(&items[..30], 10).iter().map(|p| p.len()).collect();
        assert_eq!(sizes, [10, 10, 10]);

        assert_eq!(split_evenly(&items, 0).len(), 1);
        assert_eq!(split_evenly(&items[..5], 10).len(), 1);
    }

    #[test]
    fn splits_by_set_keeping_its_diffs_together() {
        let diff = |folder: &str, name: &str| SelectedDiff {
            folder_name: folder.into(),
            file_name: format!("{name}.osu"),
            audio: "audio.mp3".into(),
            new_diff_name: name.into(),
        };
        let diffs = [diff("A", "a1"), diff("B", "b1"), diff("A", "a2"), diff("C", "c1"), diff("A", "a3")];
        let names = |max| -> Vec<Vec<&str>> {
            split_by_set(&diffs, max).iter().map(|p| p.iter().map(|d| d.new_diff_name.as_str()).collect()).collect()
        };

        // 3 sets, 5 diffs: no split with a max of 3 sets.
        assert_eq!(names(3), [vec!["a1", "a2", "a3", "b1", "c1"]]);
        assert_eq!(names(2), [vec!["a1", "a2", "a3", "b1"], vec!["c1"]]);
        assert_eq!(names(1), [vec!["a1", "a2", "a3"], vec!["b1"], vec!["c1"]]);
        assert_eq!(names(0).len(), 1);
    }

    #[test]
    fn names_parts_only_when_split() {
        assert_eq!(part_title("Pack", 0, 1), "Pack");
        assert_eq!(part_title("Pack", 1, 3), "Pack - Part 2");
    }

    #[test]
    fn parses_quoted_and_unquoted_events() {
        let event = FileEvent::parse("0,0,\"bg, with comma.jpg\",10,20").unwrap();
        assert!(event.is_background());
        assert_eq!(event.file, "bg, with comma.jpg");
        assert_eq!(event.rest, ",10,20");

        let event = FileEvent::parse("0,0,bg.png").unwrap();
        assert_eq!(event.file, "bg.png");
        assert_eq!(event.rest, "");

        assert!(FileEvent::parse("Video,0,\"v.mp4\"").unwrap().is_video());
        assert!(FileEvent::parse("2,100,200").is_some_and(|e| !e.is_background() && !e.is_video()));
    }

    #[test]
    fn finds_the_background() {
        assert_eq!(background_filename(OSU).as_deref(), Some("My BG.jpg"));
        assert_eq!(background_filename("[Events]\n//nothing\n"), None);
    }

    #[test]
    fn keeps_the_background_with_its_new_name() {
        let out = modify_osu_metadata(OSU, "Pack", "me", "[x] Hard", Some("audio-1.mp3"), Some("bg-1.jpg"));
        assert!(out.contains("0,0,\"bg-1.jpg\",0,0\n"));
        assert!(!out.contains("intro.mp4"));
        assert!(out.contains("AudioFilename: audio-1.mp3"));
        assert!(out.contains("Title:Pack\n") && out.contains("TitleUnicode:Pack\n"));
        assert!(out.contains("Creator:me\n") && out.contains("Version:[x] Hard\n"));
        assert!(!out.contains("BeatmapID") && !out.contains("BeatmapSetID"));
        assert!(out.contains("//Storyboard Layer 0 (Background)"));
    }

    #[test]
    fn drops_the_background_when_not_kept() {
        let out = modify_osu_metadata(OSU, "Pack", "me", "Hard", None, None);
        assert!(!out.contains("My BG.jpg") && !out.contains("intro.mp4"));
        assert!(out.contains("AudioFilename: song.mp3"));
    }
}
