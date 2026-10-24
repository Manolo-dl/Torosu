use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct AppConfig {
    #[serde(default)]
    pub osu_path: Option<String>,

    #[serde(default)]
    pub excluded_tools: Vec<String>,

    #[serde(default)]
    pub downloader: DownloaderConfig,

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

    #[serde(default)]
    pub osu_session: Option<String>,

    #[serde(default)]
    pub output_path: Option<String>,

    #[serde(default = "default_false")]
    pub load_library: bool,

    #[serde(default = "default_false")]
    pub skip_video: bool,

    #[serde(default = "default_max_concurrent_downloads")]
    pub max_concurrent_downloads: u32,
}

fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_max_concurrent_downloads() -> u32 { 1 }

// Implemented by hand so the defaults match the serde ones above: a derived
// Default would set `sqlite_db` to false, and that value is then written to
// configuration.toml on close, silently disabling the cache for good.
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            osu_path: None,
            excluded_tools: Vec::new(),
            downloader: DownloaderConfig::default(),
            sqlite_db: default_true(),
            open_file: default_false(),
        }
    }
}

impl Default for DownloaderConfig {
    fn default() -> Self {
        Self {
            notify: false,
            error_file: true,
            osu_session: None,
            output_path: None,
            load_library: false,
            skip_video: false,
            max_concurrent_downloads: default_max_concurrent_downloads(),
        }
    }
}