use crate::error::Result;

/// Writes `content` to `path`, a file the user picked (an export).
#[tauri::command]
pub async fn write_text_file(path: String, content: String) -> Result<()> {
    tokio::fs::write(&path, content).await?;
    Ok(())
}
