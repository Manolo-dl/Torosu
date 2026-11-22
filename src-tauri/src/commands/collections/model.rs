#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCollection {
    pub name: String,
    pub md5s: Vec<String>,
}

/// What an import changed in collection.db.
#[derive(Debug, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    /// Maps added, to new collections or to those there already.
    pub maps: usize,
    /// Collections made, and those there already that got maps.
    pub created: usize,
    pub merged: usize,
}

#[derive(serde::Serialize)]
pub struct OsuCollection {
    pub name: String,
    pub md5s: Vec<String>,
}