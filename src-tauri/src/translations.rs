//! The interface's texts (`src/shared/i18n/translations` in the frontend):
//! every language has every key of the English one, which the others fall
//! back on, and no key it lacks.

use std::{collections::BTreeSet, path::Path};

use serde_json::Value;

/// Every key of a translation file, as `settings.general.title`.
fn keys(value: &Value, prefix: &str, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                let path = if prefix.is_empty() { key.clone() } else { format!("{}.{}", prefix, key) };
                keys(value, &path, out);
            }
        }
        _ => {
            out.insert(prefix.to_string());
        }
    }
}

#[test]
fn every_language_has_every_text() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/shared/i18n/translations");
    let read = |language: &str| {
        let text = std::fs::read_to_string(dir.join(format!("{}.json", language))).unwrap();
        let mut out = BTreeSet::new();
        keys(&serde_json::from_str(&text).unwrap(), "", &mut out);
        out
    };
    let english = read("en");
    for entry in std::fs::read_dir(&dir).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(language) = name.strip_suffix(".json") else { continue };
        let other = read(language);
        let missing: Vec<&String> = english.difference(&other).collect();
        let extra: Vec<&String> = other.difference(&english).collect();
        assert!(missing.is_empty() && extra.is_empty(), "{}: missing {:?}, not in English {:?}", language, missing, extra);
    }
}
