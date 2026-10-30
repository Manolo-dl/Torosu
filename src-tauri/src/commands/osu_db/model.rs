use sqlx::prelude::FromRow;

#[derive(FromRow)]
pub struct OsuDbMeta {
    pub last_modified: i64,
    pub file_size: i64,
}

#[derive(FromRow)]
pub struct BeatmapSetRow {
    pub folder_name: String,
    pub beatmapset_id: i64,
    pub title: String,
    pub artist: String,
}

#[derive(FromRow)]
pub struct DiffRow {
    pub folder_name: String,
    pub md5: String,
    pub diff_name: String,
    pub mode: i64,
    pub length: i64,
    pub stars: f64,
    pub last_played: i64,
    pub circle_size: f64,
    pub approach_rate: f64,
    pub hp_drain: f64,
    pub overall_difficulty: f64,
    pub file_name: String,
    pub audio: String,
    pub creator: String,
    pub status: String,
}

pub struct BeatmapsetFolder {
    pub folder_path: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsuDiff {
    pub md5: String,
    pub diff_name: String,
    pub mode: u8,
    pub length: u32,
    pub stars: f64,
    pub last_played: bool,
    pub circle_size: f32,
    pub approach_rate: f32,
    pub hp_drain: f32,
    pub overall_difficulty: f32,
    pub file_name: String,
    pub audio: String,
    pub creator: String,
    /// Per diff: a ranked set can hold diffs that were never uploaded (e.g. rates).
    pub status: String,
}

/// A folder of `Songs`. Unsubmitted sets have `beatmapset_id` 0, so the folder
/// is what identifies a set.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsuBeatmapSet {
    pub folder_name: String,
    pub beatmapset_id: u32,
    pub title: String,
    pub artist: String,
    pub diffs: Vec<OsuDiff>,
}