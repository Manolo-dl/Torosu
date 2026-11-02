use serde::{Deserialize, Serialize};

use crate::commands::{packs::naming::DEFAULT_PACK_NAME_FORMAT, trainer::naming::DEFAULT_NAME_FORMAT};

#[derive(Deserialize, Serialize, Clone)]
pub struct AppConfig {
    #[serde(default)]
    pub osu_path: Option<String>,

    #[serde(default)]
    pub excluded_tools: Vec<String>,

    #[serde(default)]
    pub downloader: DownloaderConfig,

    #[serde(default)]
    pub packs: PacksConfig,

    #[serde(default)]
    pub trainer: TrainerConfig,

    #[serde(default = "default_false")]
    pub open_file: bool,

    /// Shows the developer pages (logs, osu! data).
    #[serde(default = "default_false")]
    pub dev_mode: bool,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct DownloaderConfig {
    #[serde(default = "default_false")]
    pub notify: bool,

    #[serde(default = "default_true")]
    pub error_file: bool,

    /// The osu_session cookie as saved in the file, only with `save_session`;
    /// the one in use is in `SessionState`.
    #[serde(default)]
    pub osu_session: Option<String>,

    /// Keep the cookie between sessions. Off, it's forgotten on close.
    #[serde(default = "default_false")]
    pub save_session: bool,

    #[serde(default)]
    pub output_path: Option<String>,

    #[serde(default = "default_false")]
    pub load_library: bool,

    #[serde(default = "default_false")]
    pub skip_video: bool,

    #[serde(default = "default_max_concurrent_downloads")]
    pub max_concurrent_downloads: u32,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct PacksConfig {
    /// Copy each diff's background into the pack instead of leaving it without one.
    #[serde(default = "default_true")]
    pub keep_background: bool,

    /// Above this many original sets (each with its own audio and background)
    /// the pack is split into several; 0 never splits.
    #[serde(default = "default_max_sets_per_pack")]
    pub max_sets_per_pack: u32,

    /// Template of the name a diff gets in the pack: see `packs::naming`.
    #[serde(default = "default_pack_name_format")]
    pub name_format: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TrainerConfig {
    /// Encodes the generated audio at ~192 kbit/s instead of ~128 kbit/s.
    #[serde(default = "default_false")]
    pub high_quality_audio: bool,

    /// Re-encodes the scaled video at CRF 23 instead of 28.
    #[serde(default = "default_false")]
    pub high_quality_video: bool,

    /// Sets the rate change's pitch from the rate: kept below 1.0x, raised
    /// with the speed above. The frontend hides the pitch option then.
    #[serde(default = "default_false")]
    pub auto_pitch: bool,

    /// Opens each generated .osz, so osu! imports it right away.
    #[serde(default = "default_true")]
    pub open_osz: bool,

    /// Template of the difficulty name: see `naming::version`.
    #[serde(default = "default_name_format")]
    pub name_format: String,
}

fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_max_concurrent_downloads() -> u32 { 1 }
fn default_max_sets_per_pack() -> u32 { 0 }
fn default_name_format() -> String { DEFAULT_NAME_FORMAT.to_string() }
fn default_pack_name_format() -> String { DEFAULT_PACK_NAME_FORMAT.to_string() }

// Implemented by hand so the defaults match the serde ones above: a derived
// Default would set `error_file` and `keep_background` to false, and those
// values are then written to configuration.toml with the next change, for good.
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            osu_path: None,
            excluded_tools: Vec::new(),
            downloader: DownloaderConfig::default(),
            packs: PacksConfig::default(),
            trainer: TrainerConfig::default(),
            open_file: default_false(),
            dev_mode: default_false(),
        }
    }
}

impl Default for PacksConfig {
    fn default() -> Self {
        Self {
            keep_background: default_true(),
            max_sets_per_pack: default_max_sets_per_pack(),
            name_format: default_pack_name_format(),
        }
    }
}

impl Default for DownloaderConfig {
    fn default() -> Self {
        Self {
            notify: false,
            error_file: true,
            osu_session: None,
            save_session: default_false(),
            output_path: None,
            load_library: false,
            skip_video: false,
            max_concurrent_downloads: default_max_concurrent_downloads(),
        }
    }
}

impl Default for TrainerConfig {
    fn default() -> Self {
        Self {
            high_quality_audio: default_false(),
            high_quality_video: default_false(),
            auto_pitch: default_false(),
            open_osz: default_true(),
            name_format: default_name_format(),
        }
    }
}
