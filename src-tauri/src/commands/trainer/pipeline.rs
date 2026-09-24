use anyhow::Result;
use rosu_map::Beatmap;

use crate::commands::trainer::{model::{TrainerTask, TrainerTaskParams}, tasks::rate_change};

pub fn run(beatmap: &mut Beatmap, tasks: &[TrainerTask]) -> Result<()> {

    for entry in tasks {
        match &entry.task {
            TrainerTaskParams::RateChange(params) => rate_change::apply(beatmap, params)?,
            TrainerTaskParams::Mirror(_) => log::warn!("Mirror task is not implemented yet"),
            TrainerTaskParams::RemoveSV(_) => log::warn!("RemoveSV task is not implemented yet"),
            TrainerTaskParams::NoSpinner(_) => log::warn!("NoSpinner task is not implemented yet"),
        }
    }

    Ok(())
}