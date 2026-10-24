use serde::{Deserialize, Serialize};

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

    #[serde(default = "default_true")]
    pub sqlite_db: bool,

    #[serde(default = "default_false")]
    pub open_file: bool,
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
}

fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_max_concurrent_downloads() -> u32 { 1 }
fn default_max_sets_per_pack() -> u32 { 0 }

// Implemented by hand so the defaults match the serde ones above: a derived
// Default would set `sqlite_db` to false, and that value is then written to
// configuration.toml on close, silently disabling the cache for good.
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            osu_path: None,
            excluded_tools: Vec::new(),
            downloader: DownloaderConfig::default(),
            packs: PacksConfig::default(),
            sqlite_db: default_true(),
            open_file: default_false(),
        }
    }
}

impl Default for PacksConfig {
    fn default() -> Self {
        Self {
            keep_background: default_true(),
            max_sets_per_pack: default_max_sets_per_pack(),
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