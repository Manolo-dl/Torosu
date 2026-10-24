use std::path::Path;

use crate::{AppConfigState, commands::packs::creator::sanitize};

/// Whether a pack titled `folder_name` can be created without clashing with a
/// folder already in `Songs`.
#[tauri::command]
pub async fn validate_pack_folder(folder_name: String, osu_state: tauri::State<'_, AppConfigState>) -> Result<bool, String> {

    let osu_path = osu_state.config.lock().unwrap()
        .osu_path.clone()
        .ok_or("osu! path not set")?;

    let songs = Path::new(&osu_path).join("Songs");
    let existing = std::fs::read_dir(&songs)
        .map_err(|e| {
            log::error!("failed to read {:?}: {}", songs, e);
            e.to_string()
        })?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok());

    Ok(!is_pack_folder_taken(&folder_name, existing))
}

/// osu! names the folder after the .osz, which is the sanitized title, and a
/// pack split in parts adds " - Part N" (see `creator::part_title`). Windows and
/// Wine ignore case and trailing dots or spaces in folder names.
fn is_pack_folder_taken(title: &str, existing: impl IntoIterator<Item = String>) -> bool {
    let title = sanitize(title).to_lowercase();
    let name = normalize(&title);
    let part_prefix = format!("{} - part ", title);

    existing.into_iter().any(|folder| {
        let folder = normalize(&folder.to_lowercase()).to_string();
        folder == name
            || folder
                .strip_prefix(&part_prefix)
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    })
}

fn normalize(name: &str) -> &str {
    name.trim_end_matches(['.', ' '])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn taken(title: &str, folders: &[&str]) -> bool {
        is_pack_folder_taken(title, folders.iter().map(|f| f.to_string()))
    }

    #[test]
    fn finds_the_folder_osu_creates() {
        assert!(taken("rate-pack-testing", &["123 Some Set", "rate-pack-testing"]));
        assert!(!taken("rate-pack-testing", &["123 Some Set", "rate-pack-testing 2"]));
    }

    #[test]
    fn ignores_case_and_trailing_dots() {
        assert!(taken("My Pack", &["my pack"]));
        assert!(taken("My Pack.", &["My Pack"]));
    }

    #[test]
    fn uses_the_sanitized_title() {
        assert!(taken("a/b: c", &["a_b_ c"]));
    }

    #[test]
    fn finds_split_packs() {
        assert!(taken("Big", &["Big - Part 2"]));
        assert!(!taken("Big", &["Big - Party", "Big - Part "]));
    }
}
