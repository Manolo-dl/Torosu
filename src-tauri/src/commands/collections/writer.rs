use std::{collections::HashSet, fs, path::Path};

use osynic_osudb::entity::collection::{collection::Collection, collectiondb::CollectionDB};

use crate::{
    commands::{
        collections::model::{ImportCollection, ImportResult},
        system::osu_process::osu_running,
    },
    error::{Error, Result},
    state::AppConfigState,
};

/// Adds `collections` to osu!'s collection.db: maps to those with the same
/// name, new ones for the rest. Not while osu! runs: it keeps the collections
/// in memory and writes collection.db over when it closes, so whatever was
/// imported meanwhile would be lost.
#[tauri::command]
pub async fn import_collections(
    config_state: tauri::State<'_, AppConfigState>,
    collections: Vec<ImportCollection>,
) -> Result<ImportResult> {
    if osu_running() {
        return Err(Error::OsuRunning);
    }
    let collection_path = config_state.osu_path()?.join("collection.db");

    tauri::async_runtime::spawn_blocking(move || merge_into_collectiondb(&collection_path, collections)).await?
}

/// Reads collection.db, adds `collections` to it and writes it back.
fn merge_into_collectiondb(collection_path: &Path, collections: Vec<ImportCollection>) -> Result<ImportResult> {
    log::debug!("reading collection.db for import at {:?}", collection_path);

    let mut collection_list = CollectionDB::from_file(collection_path).map_err(|e| {
        log::error!("Failed to parse collection.db: {:?}", e);
        Error::CollectionDbRead { detail: format!("{:?}", e) }
    })?;

    let result = merge(&mut collection_list, collections);
    if result.maps == 0 {
        log::info!("Nothing to import: every map was in its collection already");
        return Ok(result);
    }

    save(&collection_list, collection_path)?;
    log::info!(
        "Imported collections: {} maps, {} new collections, {} merged",
        result.maps, result.created, result.merged
    );
    Ok(result)
}

/// Adds each of `collections` to the one of its name, or as a new one; a map
/// is never put twice in a collection, nor a collection made with none.
fn merge(collection_list: &mut CollectionDB, collections: Vec<ImportCollection>) -> ImportResult {
    let mut result = ImportResult::default();

    for import in collections {
        let existing = collection_list.collections.iter_mut().find(|c| c.name.as_deref() == Some(import.name.as_str()));
        match existing {
            Some(collection) => {
                let mut hashes: HashSet<String> = collection.beatmap_hashes.iter().flatten().cloned().collect();
                let before = collection.beatmap_hashes.len();
                for md5 in import.md5s {
                    if hashes.insert(md5.clone()) {
                        collection.beatmap_hashes.push(Some(md5));
                    }
                }
                let added = collection.beatmap_hashes.len() - before;
                if added > 0 {
                    result.maps += added;
                    result.merged += 1;
                }
            }
            None => {
                let mut hashes = HashSet::new();
                let md5s: Vec<Option<String>> =
                    import.md5s.into_iter().filter(|md5| hashes.insert(md5.clone())).map(Some).collect();
                if md5s.is_empty() {
                    continue;
                }
                log::debug!("Created new collection '{}' with {} maps", import.name, md5s.len());
                result.maps += md5s.len();
                result.created += 1;
                collection_list.collections.push(Collection { name: Some(import.name), beatmap_hashes: md5s });
            }
        }
    }

    result
}

/// Writes `collection_list` over collection.db, keeping the old one as
/// collection.db.torosu-backup. Written next to it first and renamed over it:
/// a write that fails half way leaves collection.db as it was.
fn save(collection_list: &CollectionDB, collection_path: &Path) -> Result<()> {
    let write_error = |detail: String| {
        log::error!("Failed to save collection.db: {}", detail);
        Error::CollectionDbWrite { detail }
    };

    let mut bytes = Vec::new();
    collection_list.to_writer(&mut bytes).map_err(|e| write_error(format!("{e:?}")))?;

    fs::copy(collection_path, collection_path.with_extension("db.torosu-backup"))
        .map_err(|e| write_error(format!("could not keep a copy of it: {e}")))?;

    let temporary = collection_path.with_extension("db.torosu-new");
    fs::write(&temporary, &bytes).map_err(|e| write_error(e.to_string()))?;
    fs::rename(&temporary, collection_path).map_err(|e| {
        let _ = fs::remove_file(&temporary);
        write_error(e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collection(name: &str, md5s: &[&str]) -> Collection {
        Collection { name: Some(name.into()), beatmap_hashes: md5s.iter().map(|m| Some(m.to_string())).collect() }
    }

    fn import(name: &str, md5s: &[&str]) -> ImportCollection {
        ImportCollection { name: name.into(), md5s: md5s.iter().map(|m| m.to_string()).collect() }
    }

    fn hashes(db: &CollectionDB, name: &str) -> Vec<String> {
        let collection = db.collections.iter().find(|c| c.name.as_deref() == Some(name)).unwrap();
        collection.beatmap_hashes.iter().flatten().cloned().collect()
    }

    #[test]
    fn adds_to_the_collection_of_the_same_name_and_makes_the_others() {
        let mut db = CollectionDB { version: 20250107, collections: vec![collection("dans", &["a", "b"])] };
        let result = merge(&mut db, vec![import("dans", &["b", "c"]), import("ln", &["d"])]);
        assert_eq!(result, ImportResult { maps: 2, created: 1, merged: 1 });
        assert_eq!(hashes(&db, "dans"), ["a", "b", "c"]);
        assert_eq!(hashes(&db, "ln"), ["d"]);
    }

    #[test]
    fn never_puts_a_map_twice_nor_makes_an_empty_collection() {
        let mut db = CollectionDB { version: 20250107, collections: vec![collection("dans", &["a"])] };
        let result = merge(&mut db, vec![import("dans", &["a"]), import("new", &["x", "x"]), import("empty", &[])]);
        assert_eq!(result, ImportResult { maps: 1, created: 1, merged: 0 });
        assert_eq!(hashes(&db, "new"), ["x"]);
        assert!(db.collections.iter().all(|c| c.name.as_deref() != Some("empty")));
    }

    #[test]
    fn writes_collection_db_and_keeps_the_old_one() {
        let dir = std::env::temp_dir().join(format!("torosu-collections-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("collection.db");
        CollectionDB { version: 20250107, collections: vec![collection("dans", &["a"])] }.to_file(&path).unwrap();

        let result = merge_into_collectiondb(&path, vec![import("dans", &["b"])]).unwrap();
        assert_eq!(result, ImportResult { maps: 1, created: 0, merged: 1 });
        assert_eq!(hashes(&CollectionDB::from_file(&path).unwrap(), "dans"), ["a", "b"]);
        let backup = CollectionDB::from_file(dir.join("collection.db.torosu-backup")).unwrap();
        assert_eq!(hashes(&backup, "dans"), ["a"]);
        assert!(!dir.join("collection.db.torosu-new").exists());

        fs::remove_dir_all(&dir).unwrap();
    }
}
