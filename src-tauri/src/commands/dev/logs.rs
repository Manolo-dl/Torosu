use tauri::Manager;

use crate::error::Result;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    timestamp: String,
    level: String,
    target: String,
    message: String,
}

#[tauri::command]
pub async fn read_logs(app: tauri::AppHandle) -> Result<Vec<LogEntry>> {

    // Get path to the log file
    let log_dir = app.path().app_log_dir().map_err(|e| {
        log::error!("could not find app log dir: {}", e);
        e
    })?;

    let mut log_files: Vec<_> = std::fs::read_dir(&log_dir)
        .map_err(|e| {
            log::error!("could not read log dir: {}", e);
            e
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
        e
    })?;

    let entries: Vec<LogEntry> = content
        .lines()
        .filter_map(parse_log_line)
        .collect();

    Ok(entries)
}

/// Parses lines written by the log plugin format in `lib.rs`:
/// `[<date> <time> <level> <target>] <message>`
fn parse_log_line(line: &str) -> Option<LogEntry> {
    let (header, message) = line.strip_prefix('[')?.split_once(']')?;

    let mut parts = header.split_whitespace();
    let (date, time, level, target) = (parts.next()?, parts.next()?, parts.next()?, parts.next()?);

    if parts.next().is_some() {
        return None;
    }

    Some(LogEntry {
        timestamp: format!("{} {}", date, time),
        level: level.to_string(),
        target: target.to_string(),
        message: message.trim_start().to_string(),
    })
}