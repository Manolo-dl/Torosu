use sqlx::SqlitePool;
use crate::commands::collections::model::{CollectionMeta, CollectionRow, OsuCollection};

pub async fn get_meta(pool: &SqlitePool) -> Option<CollectionMeta> {
    sqlx::query_as::<_, CollectionMeta>(
        "SELECT last_modified, file_size FROM collection_meta WHERE id = 1"
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

pub async fn set_meta(pool: &SqlitePool, last_modified: i64, file_size: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO collection_meta (id, last_modified, file_size)
         VALUES (1, $1, $2)
         ON CONFLICT(id) DO UPDATE SET last_modified = $1, file_size = $2"
    )
    .bind(last_modified)
    .bind(file_size)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_collections(pool: &SqlitePool) -> Result<Vec<OsuCollection>, sqlx::Error> {
    let collections = sqlx::query_as::<_, CollectionRow>(
        "SELECT name FROM collections"
    )
    .fetch_all(pool)
    .await?;

    let all_md5s = sqlx::query_as::<_, (String, String)>(
        "SELECT collection_name, md5 FROM collection_md5s"
    )
    .fetch_all(pool)
    .await?;

    let mut md5s_by_collection: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    for (name, md5) in all_md5s {
        md5s_by_collection.entry(name).or_default().push(md5);
    }

    Ok(collections.into_iter().map(|c| OsuCollection {
        name: c.name.clone(),
        md5s: md5s_by_collection.remove(&c.name).unwrap_or_default(),
    }).collect())
}

pub async fn save_collections(pool: &SqlitePool, collections: &[OsuCollection]) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM collection_md5s").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM collections").execute(&mut *tx).await?;

    for col in collections {
        sqlx::query("INSERT INTO collections (name) VALUES ($1)")
            .bind(&col.name)
            .execute(&mut *tx)
            .await?;

        for md5 in &col.md5s {
            sqlx::query("INSERT INTO collection_md5s (md5, collection_name) VALUES ($1, $2)")
                .bind(md5)
                .bind(&col.name)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

pub async fn init_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS collection_meta (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            last_modified INTEGER NOT NULL,
            file_size INTEGER NOT NULL
        )"
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS collections (
            name TEXT PRIMARY KEY
        )"
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS collection_md5s (
            md5 TEXT NOT NULL,
            collection_name TEXT NOT NULL,
            FOREIGN KEY (collection_name) REFERENCES collections(name)
        )"
    )
    .execute(pool)
    .await?;

    Ok(())
}