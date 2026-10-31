use osynic_osudb::entity::collection::collectiondb::CollectionDB;
use crate::{AppConfigState, commands::collections::model::OsuCollection};

#[tauri::command]
pub async fn read_osu_collections(
    config_state: tauri::State<'_, AppConfigState>,
) -> Result<Vec<OsuCollection>, String> {

    let osu_path = config_state.config.lock().unwrap()
        .osu_path.clone()
        .ok_or("osu! path not set")?;

    let collection_path = std::path::Path::new(&osu_path).join("collection.db");

    log::debug!("reading collection.db at {:?}", collection_path);
    let db = CollectionDB::from_file(&collection_path).map_err(|e| e.to_string())?;

    let collections: Vec<OsuCollection> = db.collections
        .into_iter()
        .map(|c| OsuCollection {
            name: c.name.unwrap_or_default(),
            md5s: c.beatmap_hashes.into_iter().flatten().collect(),
        })
        .collect();

    log::info!("successfully parsed {} collections from collection.db", collections.len());

    Ok(collections)
}

#[tauri::command]
pub async fn write_text_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content.as_bytes()).map_err(|e| e.to_string())
}