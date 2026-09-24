use osynic_osudb::entity::collection::collectiondb::CollectionDB;
use crate::{AppConfigState, DbState, commands::collections::{cache, model::OsuCollection}};

#[tauri::command]
pub async fn read_osu_collections(
    config_state: tauri::State<'_, AppConfigState>,
    db_state: tauri::State<'_, DbState>,
) -> Result<Vec<OsuCollection>, String> {

    let osu_path = config_state.config.lock().unwrap()
        .osu_path.clone()
        .ok_or("osu! path not set")?;

    let collection_path = std::path::Path::new(&osu_path).join("collection.db");
    let use_cache = config_state.config.lock().unwrap().sqlite_db;

    let metadata = std::fs::metadata(&collection_path).map_err(|e| e.to_string())?;
    let file_size = metadata.len() as i64;
    let last_modified = metadata
        .modified()
        .map_err(|e| e.to_string())?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    let pool = &db_state.pool;
    
    if use_cache {
        log::debug!("Attempting to read collection.db from {:?}", collection_path);
    
        if let Some(meta) = cache::get_meta(pool).await {
            if meta.last_modified == last_modified && meta.file_size == file_size {
                log::debug!("loading collection.db from cache");
                return cache::get_collections(pool).await.map_err(|e| e.to_string());
            }
        }
    }

    log::debug!("cache miss, reading collection.db");
    let db = CollectionDB::from_file(&collection_path).map_err(|e| e.to_string())?;

    let collections: Vec<OsuCollection> = db.collections
        .into_iter()
        .map(|c| OsuCollection {
            name: c.name.unwrap_or_default(),
            md5s: c.beatmap_hashes.into_iter().flatten().collect(),
        })
        .collect();

    log::info!("successfully parsed {} collections from collection.db", collections.len());

    if use_cache {
        log::debug!("saving collections to cache");
        cache::save_collections(pool, &collections).await.map_err(|e| e.to_string())?;
        cache::set_meta(pool, last_modified, file_size).await.map_err(|e| e.to_string())?;
    }

    Ok(collections)
}

#[tauri::command]
pub async fn write_text_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content.as_bytes()).map_err(|e| e.to_string())
}