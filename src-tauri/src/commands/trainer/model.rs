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

impl NoteLength {
    /// In ms, where a beat lasts `beat_len`; never negative.
    pub fn ms(self, beat_len: f64) -> f64 {
        match self.unit {
            LengthUnit::Beat if self.value > 0.0 => beat_len / self.value,
            LengthUnit::Beat => 0.0,
            LengthUnit::Ms => self.value.max(0.0),
        }
    }
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

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortLnParams {
    /// Notes with less room than this before the next of their column stay
    /// normal notes.
    pub min_length: NoteLength,
    /// The longest a long note gets, with room for it.
    pub max_length: NoteLength,
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
    /// Every long note turned into a normal note.
    NoLn(EmptyParams),
    /// Every note turned into a short long note.
    ShortLn(ShortLnParams),
}

impl TrainerTaskParams {
    /// Whether it decides which notes are long notes: FLN, No LN and Short
    /// LN. A convert
    /// has one of them at most.
    pub fn shapes_lns(&self) -> bool {
        matches!(self, Self::FlnConverter(_) | Self::NoLn(_) | Self::ShortLn(_))
    }
}

#[derive(Deserialize)]
pub struct EmptyParams {}

#[derive(Deserialize)]
pub struct TrainerTask {
    pub task: TrainerTaskParams,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DifficultySettings {
    pub hp: f64,
    pub cs: f64,
    pub od: f64,
    pub ar: f64,
}

/// A diff to generate: the tasks it applies, in order.
#[derive(Deserialize)]
pub struct TrainerConvert {
    pub tasks: Vec<TrainerTask>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunTrainerRequest {
    pub osu_file_path: String,
    /// Shared by every convert.
    pub difficulty: DifficultySettings,
    pub converts: Vec<TrainerConvert>,
}