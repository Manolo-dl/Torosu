//! Replay analysis: the plays osu! keeps on this computer and how they went,
//! column by column.

pub mod frames;
pub mod graph;
pub mod judge;
pub mod scores;

use crate::{error::Result, state::AppConfigState};

use scores::LocalScore;

/// The mania plays osu! keeps on this computer, newest first, with the
/// replay of each one when osu! still has it.
#[tauri::command]
pub async fn list_local_scores(config_state: tauri::State<'_, AppConfigState>) -> Result<Vec<LocalScore>> {
    let osu = config_state.osu_path()?;
    let scores = tauri::async_runtime::spawn_blocking(move || scores::read(&osu)).await??;
    log::info!("read {} local mania scores", scores.len());
    Ok(scores)
}
