use std::path::Path;

use crate::commands::osu_db::model::BeatmapsetFolder;

/// Folder of a set inside `Songs`. `folder_name` comes from osu!.db, so there is
/// nothing to search for; unsubmitted sets (id 0) work the same way.
pub fn get_beatmapset_folder(osu_path: &str, folder_name: &str) -> Result<BeatmapsetFolder, String> {
    let folder_path = Path::new(osu_path).join("Songs").join(folder_name);

    if !folder_path.is_dir() {
        return Err(format!("Folder does not exist: {}", folder_path.display()));
    }

    Ok(BeatmapsetFolder { folder_path: folder_path.to_string_lossy().into_owned() })
}
