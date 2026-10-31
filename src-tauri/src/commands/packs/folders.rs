use std::path::Path;

use crate::{commands::packs::creator::sanitize, error::Result, paths};

/// Whether a pack titled `folder_name` can be created without clashing with a
/// folder already in `Songs`.
#[tauri::command]
pub async fn validate_pack_folder(app: tauri::AppHandle, folder_name: String) -> Result<bool> {

    // Songs can hold thousands of folders, and this runs at every change of the title.
    let songs = paths::songs_dir(&app)?;
    let existing = tauri::async_runtime::spawn_blocking(move || folders_in(&songs)).await??;

    Ok(!is_pack_folder_taken(&folder_name, existing))
}

/// Names of the folders inside `dir`.
fn folders_in(dir: &Path) -> Result<Vec<String>> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        log::error!("failed to read {:?}: {}", dir, e);
        e
    })?;

    Ok(entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect())
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
