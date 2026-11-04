use anyhow::Result;
use rosu_map::Beatmap;

use crate::commands::trainer::{
    accents::Accents,
    model::{TrainerTask, TrainerTaskParams},
    tasks::{fln_converter, key_converter, rate_change, sv_normalizer},
};

/// Applies `tasks` to the map in order. `accents` gives the accents of its
/// audio, for the tasks that follow the music; only those call it.
pub fn run<'a>(beatmap: &mut Beatmap, tasks: &[TrainerTask], accents: impl Fn() -> Option<&'a Accents>) -> Result<()> {
    // The map's times are the audio's divided by this: the rate changes so far.
    let mut rate = 1.0;

    for entry in tasks {
        match &entry.task {
            TrainerTaskParams::RateChange(params) => {
                rate_change::apply(beatmap, params)?;
                rate *= params.rate;
            }
            TrainerTaskParams::SVNormalizer(_) => sv_normalizer::apply(beatmap)?,
            TrainerTaskParams::FlnConverter(params) => fln_converter::apply(beatmap, params)?,
            TrainerTaskParams::KeyConverter(params) => {
                let audio = if key_converter::uses_audio(params) { accents().map(|a| a.at_rate(rate)) } else { None };
                key_converter::apply_with(beatmap, params, audio)?
            }
        }
    }

    Ok(())
}
