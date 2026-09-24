use std::sync::LazyLock;

use regex::Regex;
use tauri::Manager;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    timestamp: String,
    level: String,
    target: String,
    message: String,
}

static LOG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\[(\S+\s+\S+)\s+(\w+)\s+([\w:]+)\]\s+(.*)$").expect("invalid log regex")
});

#[tauri::command]
pub async fn read_logs(app: tauri::AppHandle) -> Result<Vec<LogEntry>, String> {

    // Get path to the log file
    let log_dir = app.path().app_log_dir().map_err(|e| {
        log::error!("could not find app log dir: {}", e);
        e.to_string()
    })?;

    let mut log_files: Vec<_> = std::fs::read_dir(&log_dir)
        .map_err(|e| {
            log::error!("could not read log dir: {}", e);
            e.to_string()
        })?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "log"))
        .collect();

    log_files.sort_by_key(|e| {
        e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });

    let latest_file = log_files.last().ok_or("No log file found")?;

    let content = std::fs::read_to_string(latest_file.path()).map_err(|e| {
        log::error!("could not read log file: {}", e);
        e.to_string()
    })?;

    let entries: Vec<LogEntry> = content
        .lines()
        .filter_map(parse_log_line)
        .collect();

    Ok(entries)
}

fn parse_log_line(line: &str) -> Option<LogEntry> {
    let caps = LOG_REGEX.captures(line)?;

    Some(LogEntry {
        timestamp: caps.get(1)?.as_str().to_string(),
        level: caps.get(2)?.as_str().to_string(),
        target: caps.get(3)?.as_str().to_string(),
        message: caps.get(4)?.as_str().to_string(),
    })
}