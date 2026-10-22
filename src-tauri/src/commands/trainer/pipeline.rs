use anyhow::Result;
use rosu_map::Beatmap;

use crate::commands::trainer::{model::{TrainerTask, TrainerTaskParams}, tasks::{fln_converter, rate_change, sv_normalizer}};

pub fn run(beatmap: &mut Beatmap, tasks: &[TrainerTask]) -> Result<()> {

    for entry in tasks {
        match &entry.task {
            TrainerTaskParams::RateChange(params) => rate_change::apply(beatmap, params)?,
            TrainerTaskParams::SVNormalizer(_) => sv_normalizer::apply(beatmap)?,
            TrainerTaskParams::FlnConverter(params) => fln_converter::apply(beatmap, params)?,
        }
    }

    Ok(())
}