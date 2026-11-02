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
    /// 0 for diffs never uploaded: the trainer and the packs leave `BeatmapID` out.
    pub beatmap_id: u32,
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