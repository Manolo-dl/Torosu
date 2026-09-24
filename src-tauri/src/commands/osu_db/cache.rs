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
        "SELECT beatmapset_id, title, artist, status FROM beatmapsets"
    )
    .fetch_all(pool)
    .await?;

    let all_diffs = sqlx::query_as::<_, DiffRow>(
        "SELECT beatmapset_id, md5, diff_name, mode, length, stars, last_played,
         circle_size, approach_rate, hp_drain, overall_difficulty, file_name, audio, creator FROM diffs"
    )
    .fetch_all(pool)
    .await?;

    let mut diffs_by_set: HashMap<i64, Vec<OsuDiff>> = HashMap::new();
    for d in all_diffs {
        diffs_by_set.entry(d.beatmapset_id).or_default().push(OsuDiff {
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
        });
    }

    let result = sets.into_iter().map(|set| OsuBeatmapSet {
        beatmapset_id: set.beatmapset_id as u32,
        title: set.title,
        artist: set.artist,
        status: set.status,
        diffs: diffs_by_set.remove(&set.beatmapset_id).unwrap_or_default(),
    }).collect();

    Ok(result)
}

pub async fn save_beatmapsets(pool: &SqlitePool, sets: &[OsuBeatmapSet]) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    let existing_ids: Vec<i64> = sqlx::query_as::<_, (i64,)>(
        "SELECT beatmapset_id FROM beatmapsets"
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|(id,)| id)
    .collect();

    let existing_set: std::collections::HashSet<i64> = existing_ids.into_iter().collect();

    let new_ids: std::collections::HashSet<i64> = sets.iter()
        .map(|s| s.beatmapset_id as i64)
        .collect();

    for id in existing_set.difference(&new_ids) {
        sqlx::query("DELETE FROM diffs WHERE beatmapset_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM beatmapsets WHERE beatmapset_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }

    for set in sets {

        if existing_set.contains(&(set.beatmapset_id as i64)) {
            continue;
        }
        
        sqlx::query(
            "INSERT INTO beatmapsets (beatmapset_id, title, artist, status)
             VALUES ($1, $2, $3, $4)"
        )
        .bind(set.beatmapset_id as i64)
        .bind(&set.title)
        .bind(&set.artist)
        .bind(&set.status)
        .execute(&mut *tx)
        .await?;

        for diff in &set.diffs {
            sqlx::query(
                "INSERT INTO diffs (md5, beatmapset_id, diff_name, mode, length, stars,
                 last_played, circle_size, approach_rate, hp_drain, overall_difficulty, file_name, audio, creator)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)"
            )
            .bind(&diff.md5)
            .bind(set.beatmapset_id as i64)
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
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

pub async fn init_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {

    const CURRENT_VERSION: i64 = 3;

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
            beatmapset_id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            artist TEXT NOT NULL,
            status TEXT NOT NULL
        )"
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS diffs (
            md5 TEXT PRIMARY KEY,
            beatmapset_id INTEGER NOT NULL,
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
            FOREIGN KEY (beatmapset_id) REFERENCES beatmapsets(beatmapset_id)
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

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS beatmapset_folders (
            beatmapset_id INTEGER PRIMARY KEY,
            folder_path TEXT NOT NULL,
            FOREIGN KEY (beatmapset_id) REFERENCES beatmapsets(beatmapset_id)
        )"
    )
    .execute(pool)
    .await?;

    Ok(())
}