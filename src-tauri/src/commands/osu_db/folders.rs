use sqlx::SqlitePool;
use walkdir::WalkDir;

use crate::{commands::osu_db::model::BeatmapsetFolder};

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

async fn resolve_beatmapset_folder(osu_path: &str, file_name: &str, beatmapset_id: u32) -> Result<BeatmapsetFolder, String> {

    let songs_folder = format!("{}/Songs", osu_path);

    for entry in WalkDir::new(songs_folder).max_depth(2) {

        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_name().to_string_lossy() == file_name {

            let folder_path = entry.path()
                .parent()
                .ok_or("Could not get parent folder")?
                .to_string_lossy()
                .to_string();

            return Ok(BeatmapsetFolder {
                folder_path,
            })
        }
    }

    Err(format!("Could not find folder for beatmapset_id {} with file_name {}", beatmapset_id, file_name))
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
            if check_beatmapset_folder_exists(&folder.folder_path)
            .await?
            {
                return Ok(folder);
            }
        }
    }

    let folder = resolve_beatmapset_folder(osu_path, file_name, beatmapset_id)
        .await?;

    if use_sqlite {
        save_beatmapset_folder(pool, beatmapset_id, &folder.folder_path)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(folder)
}

pub async fn check_beatmapset_folder_exists(folder_path: &str) -> Result<bool, String> {
    Ok(std::path::Path::new(folder_path).exists())
}