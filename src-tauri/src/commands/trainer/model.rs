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
    /// Also scales the `.osb` storyboard. It is shared by every diff in the set,
    /// so it is merged into the new diff and the .osz is exported as a new set.
    #[serde(default)]
    pub scale_storyboard: bool,
    /// Re-encodes the background video at the new speed (needs ffmpeg).
    #[serde(default)]
    pub scale_video: bool,
}

/// A length either in beats, as `1/value` of a beat at the BPM in effect, or
/// in milliseconds.
#[derive(Clone, Copy, Deserialize)]
pub struct NoteLength {
    pub unit: LengthUnit,
    pub value: f64,
}

#[derive(Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LengthUnit {
    Beat,
    Ms,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlnConverterParams {
    /// Time each long note is released before the next note in its column.
    pub gap: NoteLength,
    /// Long notes shorter than this are left as normal notes.
    pub min_length: NoteLength,
    /// Also reshapes the long notes the map already has (FLN); otherwise they
    /// are kept as they are (INV).
    pub rework_existing_ln: bool,
}

/// Extra notes the key converter adds to fill the new keys.
#[derive(Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Density {
    None,
    Dense,
    MoreDense,
    /// Chords grow with the key count, so handstreams stay handstreams.
    Full,
    /// Rows as big as hand-made maps make them for each pattern and part of
    /// the map, learned from real maps. Also applies when going down in keys.
    Auto,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyConverterParams {
    pub target_keys: u8,
    pub density: Density,
    /// Keeps the map's jacks on a single key instead of splitting them.
    pub keep_jacks: bool,
    /// How the target keys are played (see `analysis::layout`); the usual
    /// playstyle of the key count when missing.
    #[serde(default)]
    pub playstyle: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum TrainerTaskParams {
    RateChange(RateChangeParams),
    SVNormalizer(EmptyParams),
    FlnConverter(FlnConverterParams),
    KeyConverter(KeyConverterParams),
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