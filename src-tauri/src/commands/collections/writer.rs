use std::collections::HashSet;

use crate::{AppConfigState, commands::collections::model::ImportCollection};

#[tauri::command]
pub async fn import_collections(
    config_state: tauri::State<'_, AppConfigState>,
    collections: Vec<ImportCollection>
) -> Result<(), String> {

    let osu_path = config_state.config.lock().unwrap()
        .osu_path.clone()
        .ok_or("osu! path not set")?;

    let collection_path = std::path::Path::new(&osu_path).join("collection.db");

    log::debug!("reading collection.db for import at {:?}", collection_path);

    let mut collection_list = osu_db::CollectionList::from_file(&collection_path)
        .map_err(|e| {
            log::error!("Failed to parse collection.db: {:?}", e);
            format!("Failed to parse collection.db: {:?}", e)
        })?;

    let mut added_count = 0;
    let mut new_collections_count = 0;

    for import in collections  {

        let existing = collection_list.collections.iter_mut()
            .find(|c| c.name.as_deref() == Some(import.name.as_str()));

        match existing {
            Some(col) => {
                let existing_hashes: HashSet<String> = col.beatmap_hashes.iter()
                    .filter_map(|h| h.clone())
                    .collect();

                for md5 in import.md5s {
                    if !existing_hashes.contains(&md5) {
                        col.beatmap_hashes.push(Some(md5));
                        added_count += 1;
                    }
                }
            }
            None => {
                let count = import.md5s.len();
                collection_list.collections.push(osu_db::collection::Collection {
                    name: Some(import.name.clone()),
                    beatmap_hashes: import.md5s.into_iter().map(Some).collect(),
                });
                new_collections_count += 1;
                added_count += count;
                log::debug!("Created new collection '{}' with {} maps", import.name, count);
            }
        }
    }

    collection_list.to_file(&collection_path).map_err(|e| {
        log::error!("Failed to save collection.db: {:?}", e);
        format!("Failed to save collection.db: {:?}", e)
    })?;

    log::info!("Imported collections: {} new collections, {} maps added", new_collections_count, added_count);

    Ok(())
}