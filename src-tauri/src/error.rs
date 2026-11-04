//! The error the commands fail with. The frontend gets it as
//! `{ code, params, message }`: it shows the text of `errors.<code>` in the
//! language in use, filled with `params`, and `message` (English, what the
//! logs say too) when it has none.

use serde::{Serialize, Serializer};
use serde_json::{json, Map, Value};

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

    /// The trainer's tasks report with anyhow. Not `#[from]`: an `Error`
    /// they returned through anyhow comes back as itself (see the `From`).
    #[error(transparent)]
    Other(anyhow::Error),

    /// A message nobody is expected to see; not translated.
    #[error("{0}")]
    Message(String),

    // ── osu! folder ─────────────────────────────────────────────────────────
    #[error("osu! folder not found")]
    OsuFolderNotFound,

    #[error("{path} is not an osu! folder: it has no osu!.db")]
    NotOsuFolder { path: String },

    #[error("osu!.db appears to be corrupted at {path} and could not be parsed")]
    OsuDbCorrupted { path: String },

    #[error("failed to read collection.db: {detail}")]
    CollectionDbRead { detail: String },

    #[error("failed to save collection.db: {detail}")]
    CollectionDbWrite { detail: String },

    #[error("no beatmap selected in osu!, or its folder could not be found")]
    NoBeatmapSelected,

    // ── Downloader ──────────────────────────────────────────────────────────
    #[error("osu_session cookie not set")]
    NoSession,

    #[error("output folder not found: {path}")]
    OutputFolderNotFound { path: String },

    #[error("HTTP {status}")]
    HttpStatus { status: u16 },

    #[error("beatmap set {id} is too large ({bytes} bytes)")]
    SetTooLarge { id: u64, bytes: u64 },

    // ── Packs ───────────────────────────────────────────────────────────────
    #[error("could not create {path}: {detail}")]
    CreateFolder { path: String, detail: String },

    #[error("failed to read {file}: {detail}")]
    ReadDiff { file: String, detail: String },

    // ── Trainer ─────────────────────────────────────────────────────────────
    #[error("there is nothing to convert")]
    NothingToConvert,

    #[error("converts {first} and {second} make the same difficulty, \"{name}\": change one of them")]
    SameDifficulty { first: usize, second: usize, name: String },

    /// What went wrong in one of the converts.
    #[error("convert {number}: {cause}")]
    Convert { number: usize, cause: Box<Error> },

    /// `task` is the frontend's name of the task: `KeyConverter` or `FlnConverter`.
    #[error("{task} only works on osu!mania maps")]
    ManiaOnly { task: &'static str },

    #[error("the key converter supports 1K to 10K and 12K, 14K, 16K and 18K, not {keys}K")]
    UnsupportedKeys { keys: usize },

    #[error("the key converter maps for one player, not {layout}")]
    OnePlayer { layout: String },

    #[error("no supported audio track found")]
    NoAudio,

    #[error("video processing failed: {detail}")]
    Video { detail: String },

    // ── App ─────────────────────────────────────────────────────────────────
    #[error("no update to install")]
    NoUpdate,
}

#[cfg(test)]
impl Error {
    /// Every code [`Error::code`] gives: each one has its text in the
    /// frontend's translations (`errors.<code>`), which a test checks.
    pub const CODES: &'static [&'static str] = &[
        "io", "notFound", "permissionDenied", "zip", "osuDb", "config", "http", "offline", "timeout", "updater",
        "internal", "message", "osuFolderNotFound", "notOsuFolder", "osuDbCorrupted", "collectionDbRead",
        "collectionDbWrite", "noBeatmapSelected", "noSession", "outputFolderNotFound", "httpStatus", "setTooLarge",
        "createFolder", "readDiff", "nothingToConvert", "sameDifficulty", "convert", "maniaOnly", "unsupportedKeys",
        "onePlayer", "noAudio", "video", "noUpdate",
    ];
}

impl Error {
    /// What the frontend looks the text up by, and what fills it. The errors
    /// of the system and the libraries keep their own text as `detail`.
    pub fn code(&self) -> (&'static str, Map<String, Value>) {
        let detail = || params(json!({ "detail": self.to_string() }));
        match self {
            Error::Io(e) => match e.kind() {
                std::io::ErrorKind::NotFound => ("notFound", detail()),
                std::io::ErrorKind::PermissionDenied => ("permissionDenied", detail()),
                _ => ("io", detail()),
            },
            Error::Zip(_) => ("zip", detail()),
            Error::OsuDb(_) => ("osuDb", detail()),
            Error::Toml(_) => ("config", detail()),
            Error::Http(e) if e.is_timeout() => ("timeout", detail()),
            Error::Http(e) if e.is_connect() => ("offline", detail()),
            Error::Http(_) => ("http", detail()),
            Error::Updater(_) => ("updater", detail()),
            Error::Tauri(_) | Error::Other(_) => ("internal", detail()),
            Error::Message(_) => ("message", detail()),
            Error::OsuFolderNotFound => ("osuFolderNotFound", Map::new()),
            Error::NotOsuFolder { path } => ("notOsuFolder", params(json!({ "path": path }))),
            Error::OsuDbCorrupted { path } => ("osuDbCorrupted", params(json!({ "path": path }))),
            Error::CollectionDbRead { detail } => ("collectionDbRead", params(json!({ "detail": detail }))),
            Error::CollectionDbWrite { detail } => ("collectionDbWrite", params(json!({ "detail": detail }))),
            Error::NoBeatmapSelected => ("noBeatmapSelected", Map::new()),
            Error::NoSession => ("noSession", Map::new()),
            Error::OutputFolderNotFound { path } => ("outputFolderNotFound", params(json!({ "path": path }))),
            Error::HttpStatus { status } => ("httpStatus", params(json!({ "status": status }))),
            Error::SetTooLarge { id, bytes } => ("setTooLarge", params(json!({ "id": id, "bytes": bytes }))),
            Error::CreateFolder { path, detail } => ("createFolder", params(json!({ "path": path, "detail": detail }))),
            Error::ReadDiff { file, detail } => ("readDiff", params(json!({ "file": file, "detail": detail }))),
            Error::NothingToConvert => ("nothingToConvert", Map::new()),
            Error::SameDifficulty { first, second, name } => {
                ("sameDifficulty", params(json!({ "first": first, "second": second, "name": name })))
            }
            // The frontend translates the cause too.
            Error::Convert { number, cause } => ("convert", params(json!({ "number": number, "cause": cause }))),
            Error::ManiaOnly { task } => ("maniaOnly", params(json!({ "task": task }))),
            Error::UnsupportedKeys { keys } => ("unsupportedKeys", params(json!({ "keys": keys }))),
            Error::OnePlayer { layout } => ("onePlayer", params(json!({ "layout": layout }))),
            Error::NoAudio => ("noAudio", Map::new()),
            Error::Video { detail } => ("video", params(json!({ "detail": detail }))),
            Error::NoUpdate => ("noUpdate", Map::new()),
        }
    }
}

fn params(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        error.downcast::<Error>().unwrap_or_else(Error::Other)
    }
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
        let (code, params) = self.code();
        json!({ "code": code, "params": params, "message": self.to_string() }).serialize(serializer)
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(test)]
mod tests {
    use super::*;

    fn sent(error: &Error) -> Value {
        serde_json::to_value(error).unwrap()
    }

    #[test]
    fn is_sent_with_its_code_and_params() {
        let value = sent(&Error::NotOsuFolder { path: "/games/osu".into() });
        assert_eq!(value["code"], "notOsuFolder");
        assert_eq!(value["params"]["path"], "/games/osu");
        assert_eq!(value["message"], "/games/osu is not an osu! folder: it has no osu!.db");
    }

    #[test]
    fn a_convert_sends_its_cause() {
        let error = Error::Convert { number: 2, cause: Box::new(Error::UnsupportedKeys { keys: 11 }) };
        let value = sent(&error);
        assert_eq!(value["code"], "convert");
        assert_eq!(value["params"]["number"], 2);
        assert_eq!(value["params"]["cause"]["code"], "unsupportedKeys");
        assert_eq!(value["params"]["cause"]["params"]["keys"], 11);
    }

    #[test]
    fn comes_back_through_anyhow() {
        let through: anyhow::Error = Error::ManiaOnly { task: "KeyConverter" }.into();
        assert!(matches!(Error::from(through), Error::ManiaOnly { task: "KeyConverter" }));
        assert!(matches!(Error::from(anyhow::anyhow!("something else")), Error::Other(_)));
    }

    #[test]
    fn system_errors_keep_their_text() {
        let error = Error::Io(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "no way"));
        let value = sent(&error);
        assert_eq!(value["code"], "permissionDenied");
        assert_eq!(value["params"]["detail"], "no way");
    }

    #[test]
    fn every_code_is_listed() {
        let errors = [
            Error::Message("x".into()),
            Error::OsuFolderNotFound,
            Error::NotOsuFolder { path: String::new() },
            Error::OsuDbCorrupted { path: String::new() },
            Error::CollectionDbRead { detail: String::new() },
            Error::CollectionDbWrite { detail: String::new() },
            Error::NoBeatmapSelected,
            Error::NoSession,
            Error::OutputFolderNotFound { path: String::new() },
            Error::HttpStatus { status: 404 },
            Error::SetTooLarge { id: 1, bytes: 1 },
            Error::CreateFolder { path: String::new(), detail: String::new() },
            Error::ReadDiff { file: String::new(), detail: String::new() },
            Error::NothingToConvert,
            Error::SameDifficulty { first: 1, second: 2, name: String::new() },
            Error::Convert { number: 1, cause: Box::new(Error::NoAudio) },
            Error::ManiaOnly { task: "FlnConverter" },
            Error::UnsupportedKeys { keys: 11 },
            Error::OnePlayer { layout: String::new() },
            Error::NoAudio,
            Error::Video { detail: String::new() },
            Error::NoUpdate,
            Error::Io(std::io::Error::other("x")),
            Error::Other(anyhow::anyhow!("x")),
        ];
        for error in &errors {
            let (code, _) = error.code();
            assert!(Error::CODES.contains(&code), "{code} is not in Error::CODES");
        }
    }
}
