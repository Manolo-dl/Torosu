use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::commands::osu_db::model::{BeatmapSetRow, DiffRow, OsuBeatmapSet, OsuDbMeta, OsuDiff};

pub async fn get_meta(pool: &SqlitePool) -> Option<OsuDbMeta> {
    sqlx::query_as::<_, OsuDbMeta>(
        "SELECT last_modified, file_size FROM osudb_meta WHERE id = 1"
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

pub async fn set_meta(pool: &SqlitePool, last_modified: i64, file_size: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO osudb_meta (id, last_modified, file_size)
         VALUES (1, $1, $2)
         ON CONFLICT(id) DO UPDATE SET last_modified = $1, file_size = $2"
    )
    .bind(last_modified)
    .bind(file_size)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_beatmapsets(pool: &SqlitePool) -> Result<Vec<OsuBeatmapSet>, sqlx::Error> {
    let sets = sqlx::query_as::<_, BeatmapSetRow>(
        "SELECT folder_name, beatmapset_id, title, artist FROM beatmapsets"
    )
    .fetch_all(pool)
    .await?;

    let all_diffs = sqlx::query_as::<_, DiffRow>(
        "SELECT folder_name, md5, diff_name, mode, length, stars, last_played,
         circle_size, approach_rate, hp_drain, overall_difficulty, file_name, audio, creator, status FROM diffs"
    )
    .fetch_all(pool)
    .await?;

    let mut diffs_by_folder: HashMap<String, Vec<OsuDiff>> = HashMap::new();
    for d in all_diffs {
        diffs_by_folder.entry(d.folder_name).or_default().push(OsuDiff {
            md5: d.md5,
            diff_name: d.diff_name,
            mode: d.mode as u8,
            bpm: 0.0,
            length: d.length as u32,
            stars: d.stars,
            last_played: d.last_played != 0,
            circle_size: d.circle_size as f32,
            approach_rate: d.approach_rate as f32,
            hp_drain: d.hp_drain as f32,
            overall_difficulty: d.overall_difficulty as f32,
            file_name: d.file_name,
            audio: d.audio,
            creator: d.creator,
            status: d.status,
        });
    }

    let result = sets.into_iter().map(|set| OsuBeatmapSet {
        diffs: diffs_by_folder.remove(&set.folder_name).unwrap_or_default(),
        folder_name: set.folder_name,
        beatmapset_id: set.beatmapset_id as u32,
        title: set.title,
        artist: set.artist,
    }).collect();

    Ok(result)
}

pub async fn save_beatmapsets(pool: &SqlitePool, sets: &[OsuBeatmapSet]) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // Sets are replaced as a whole: a folder that changed (a new rate, a
    // diff that got ranked) must not keep its old diffs.
    sqlx::query("DELETE FROM diffs").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM beatmapsets").execute(&mut *tx).await?;

    for set in sets {
        sqlx::query(
            "INSERT INTO beatmapsets (folder_name, beatmapset_id, title, artist)
             VALUES ($1, $2, $3, $4)"
        )
        .bind(&set.folder_name)
        .bind(set.beatmapset_id as i64)
        .bind(&set.title)
        .bind(&set.artist)
        .execute(&mut *tx)
        .await?;

        for diff in &set.diffs {
            sqlx::query(
                // osu!.db can list the same .osu file twice; keep the first one.
                "INSERT OR IGNORE INTO diffs (md5, folder_name, diff_name, mode, length, stars,
                 last_played, circle_size, approach_rate, hp_drain, overall_difficulty, file_name, audio, creator, status)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)"
            )
            .bind(&diff.md5)
            .bind(&set.folder_name)
            .bind(&diff.diff_name)
            .bind(diff.mode as i64)
            .bind(diff.length as i64)
            .bind(diff.stars)
            .bind(diff.last_played as i64)
            .bind(diff.circle_size as f64)
            .bind(diff.approach_rate as f64)
            .bind(diff.hp_drain as f64)
            .bind(diff.overall_difficulty as f64)
            .bind(&diff.file_name)
            .bind(&diff.audio)
            .bind(&diff.creator)
            .bind(&diff.status)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

pub async fn init_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {

    // 6 matches main (0.6.1), which already moved to this schema.
    const CURRENT_VERSION: i64 = 6;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS schema_version (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            version INTEGER NOT NULL
        )"
    )
    .execute(pool)
    .await?;

    let current: Option<(i64,)> = sqlx::query_as(
        "SELECT version FROM schema_version WHERE id = 1"
    )
    .fetch_optional(pool)
    .await?;

    let needs_migration = match current {
        Some((v,)) => v < CURRENT_VERSION,
        None => true,
    };

    if needs_migration {
        log::info!("migrating osu_db schema, dropping old tables");
        sqlx::query("DROP TABLE IF EXISTS beatmapset_folders").execute(pool).await?;
        sqlx::query("DROP TABLE IF EXISTS diffs").execute(pool).await?;
        sqlx::query("DROP TABLE IF EXISTS beatmapsets").execute(pool).await?;
        sqlx::query("DROP TABLE IF EXISTS osudb_meta").execute(pool).await?;
    }

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS osudb_meta (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            last_modified INTEGER NOT NULL,
            file_size INTEGER NOT NULL
        )"
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS beatmapsets (
            folder_name TEXT PRIMARY KEY,
            beatmapset_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            artist TEXT NOT NULL
        )"
    )
    .execute(pool)
    .await?;

    // Not unique: unsubmitted sets all have id 0.
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_beatmapsets_beatmapset_id ON beatmapsets(beatmapset_id)"
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS diffs (
            md5 TEXT NOT NULL,
            folder_name TEXT NOT NULL,
            diff_name TEXT NOT NULL,
            mode INTEGER NOT NULL,
            length INTEGER NOT NULL,
            stars REAL NOT NULL,
            last_played INTEGER NOT NULL,
            circle_size REAL NOT NULL,
            approach_rate REAL NOT NULL,
            hp_drain REAL NOT NULL,
            overall_difficulty REAL NOT NULL,
            file_name TEXT NOT NULL,
            audio TEXT NOT NULL,
            creator TEXT NOT NULL,
            status TEXT NOT NULL,
            PRIMARY KEY (folder_name, md5),
            FOREIGN KEY (folder_name) REFERENCES beatmapsets(folder_name)
        )"
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO schema_version (id, version) VALUES (1, $1)
            ON CONFLICT(id) DO UPDATE SET version = $1"
    ).bind(CURRENT_VERSION)
    .execute(pool)
    .await?;

    Ok(())
}