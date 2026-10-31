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