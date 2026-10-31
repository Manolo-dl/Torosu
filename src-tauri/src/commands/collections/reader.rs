use std::path::Path;
use osynic_osudb::entity::collection::collectiondb::CollectionDB;
use crate::{AppConfigState, commands::collections::model::OsuCollection};

#[tauri::command]
pub async fn read_osu_collections(
    config_state: tauri::State<'_, AppConfigState>,
) -> Result<Vec<OsuCollection>, String> {

    let osu_path = config_state.config.lock().unwrap()
        .osu_path.clone()
        .ok_or("osu! path not set")?;

    let collection_path = Path::new(&osu_path).join("collection.db");

    log::debug!("reading collection.db at {:?}", collection_path);
    let collections = tauri::async_runtime::spawn_blocking(move || read_from_collectiondb(&collection_path))
        .await
        .map_err(|e| e.to_string())??;

    log::info!("successfully parsed {} collections from collection.db", collections.len());

    Ok(collections)
}

fn read_from_collectiondb(path: &Path) -> Result<Vec<OsuCollection>, String> {
    let db = CollectionDB::from_file(path).map_err(|e| e.to_string())?;

    Ok(db.collections
        .into_iter()
        .map(|c| OsuCollection {
            name: c.name.unwrap_or_default(),
            md5s: c.beatmap_hashes.into_iter().flatten().collect(),
        })
        .collect())
}

#[tauri::command]
pub async fn write_text_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content.as_bytes()).map_err(|e| e.to_string())
}