use std::path::Path;

use sqlx::SqlitePool;

use crate::commands::osu_db::model::BeatmapsetFolder;

async fn resolve_cached_beatmapset_folder(pool: &SqlitePool, beatmapset_id: u32) -> Result<Option<BeatmapsetFolder>, sqlx::Error> {
    Ok(
        sqlx::query_as::<_, BeatmapsetFolder>(
            "SELECT folder_path FROM beatmapset_folders WHERE beatmapset_id = $1"
        )
        .bind(beatmapset_id)
        .fetch_optional(pool)
        .await?
    )
}

/// Looks for `file_name` in `Songs` and in its direct subfolders.
fn resolve_beatmapset_folder(osu_path: &str, file_name: &str, beatmapset_id: u32) -> Result<BeatmapsetFolder, String> {

    let songs_folder = Path::new(osu_path).join("Songs");

    let found = if songs_folder.join(file_name).is_file() {
        Some(songs_folder)
    } else {
        std::fs::read_dir(&songs_folder)
            .map_err(|e| e.to_string())?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .find(|dir| dir.join(file_name).is_file())
    };

    found
        .map(|folder| BeatmapsetFolder { folder_path: folder.to_string_lossy().into_owned() })
        .ok_or_else(|| format!("Could not find folder for beatmapset_id {} with file_name {}", beatmapset_id, file_name))
}

async fn save_beatmapset_folder(pool: &SqlitePool, beatmapset_id: u32, folder_path: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO beatmapset_folders (beatmapset_id, folder_path)
            VALUES ($1, $2)
            ON CONFLICT(beatmapset_id) DO UPDATE SET folder_path = $2"
    )
    .bind(beatmapset_id)
    .bind(folder_path)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_beatmapset_folder(
    pool: &SqlitePool,
    use_sqlite: bool,
    osu_path: &str,
    beatmapset_id: u32,
    file_name: &str,
) -> Result<BeatmapsetFolder, String> {

    if use_sqlite {
        let cached = resolve_cached_beatmapset_folder(pool, beatmapset_id)
        .await
        .map_err(|e| e.to_string())?;
    
        if let Some(folder) = cached {
            if check_beatmapset_folder_exists(&folder.folder_path) {
                return Ok(folder);
            }
        }
    }

    let folder = resolve_beatmapset_folder(osu_path, file_name, beatmapset_id)?;

    if use_sqlite {
        save_beatmapset_folder(pool, beatmapset_id, &folder.folder_path)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(folder)
}

pub fn check_beatmapset_folder_exists(folder_path: &str) -> bool {
    Path::new(folder_path).exists()
}