use sqlx::prelude::FromRow;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCollection {
    pub name: String,
    pub md5s: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct OsuCollection {
    pub name: String,
    pub md5s: Vec<String>,
}

#[derive(FromRow)]
pub struct CollectionMeta {
    pub last_modified: i64,
    pub file_size: i64,
}

#[derive(FromRow)]
pub struct CollectionRow {
    pub name: String,
}