use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize)]
pub struct TrainerProgress {
    pub percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateChangeParams {
    pub rate: f64,
    pub adjust_pitch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorParams {
    pub axis: MirrorAxis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MirrorAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum TrainerTaskParams {
    RateChange(RateChangeParams),
    Mirror(MirrorParams),
    RemoveSV(EmptyParams),
    NoSpinner(EmptyParams),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyParams {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainerTask {
    pub task: TrainerTaskParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DifficultySettings {
    pub hp: f64,
    pub cs: f64,
    pub od: f64,
    pub ar: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunTrainerRequest {
    pub osu_file_path: String,
    pub difficulty: DifficultySettings,
    pub tasks: Vec<TrainerTask>,
}