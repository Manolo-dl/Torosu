use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize)]
pub struct TrainerProgress {
    pub percent: u8,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateChangeParams {
    pub rate: f64,
    pub adjust_pitch: bool,
}

#[allow(dead_code)] // Mirror task is not implemented yet
#[derive(Deserialize)]
pub struct MirrorParams {
    pub axis: MirrorAxis,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MirrorAxis {
    Horizontal,
    Vertical,
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum TrainerTaskParams {
    RateChange(RateChangeParams),
    #[allow(dead_code)]
    Mirror(MirrorParams),
    RemoveSV(EmptyParams),
    NoSpinner(EmptyParams),
}

#[derive(Deserialize)]
pub struct EmptyParams {}

#[derive(Deserialize)]
pub struct TrainerTask {
    pub task: TrainerTaskParams,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DifficultySettings {
    pub hp: f64,
    pub cs: f64,
    pub od: f64,
    pub ar: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunTrainerRequest {
    pub osu_file_path: String,
    pub difficulty: DifficultySettings,
    pub tasks: Vec<TrainerTask>,
}