//! The error the commands fail with. The frontend gets its message, a string,
//! as when the commands returned `Result<_, String>`.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),

    #[error(transparent)]
    OsuDb(#[from] osynic_osudb::error::Error),

    #[error(transparent)]
    Toml(#[from] toml::ser::Error),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Updater(#[from] tauri_plugin_updater::Error),

    /// Also what a `spawn_blocking` task that panicked gives.
    #[error(transparent)]
    Tauri(#[from] tauri::Error),

    /// The trainer's tasks report with anyhow.
    #[error(transparent)]
    Other(#[from] anyhow::Error),

    #[error("{0}")]
    Message(String),
}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

impl From<&str> for Error {
    fn from(message: &str) -> Self {
        Self::Message(message.to_string())
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
