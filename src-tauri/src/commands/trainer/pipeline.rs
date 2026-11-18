use anyhow::Result;
use rosu_map::Beatmap;

use crate::commands::trainer::{
    accents::Accents,
    model::{TrainerTask, TrainerTaskParams},
    tasks::{fln_converter, key_converter, no_ln, rate_change, sv_normalizer},
};
use crate::error::Error;

/// Applies `tasks` to the map in order. `accents` gives the accents of its
/// audio, for the tasks that follow the music; only those call it.
pub fn run<'a, 't>(
    beatmap: &mut Beatmap,
    tasks: impl IntoIterator<Item = &'t TrainerTask>,
    accents: impl Fn() -> Option<&'a Accents>,
) -> Result<()> {
    // One task decides the long notes: two would undo each other.
    let tasks: Vec<&TrainerTask> = tasks.into_iter().collect();
    if tasks.iter().filter(|t| t.task.shapes_lns()).count() > 1 {
        return Err(Error::LnTasksTogether.into());
    }

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
            TrainerTaskParams::NoLn(_) => no_ln::apply(beatmap)?,
            TrainerTaskParams::KeyConverter(params) => {
                let audio = if key_converter::uses_audio(params) { accents().map(|a| a.at_rate(rate)) } else { None };
                key_converter::apply_with(beatmap, params, audio)?
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::trainer::model::{EmptyParams, FlnConverterParams, LengthUnit, NoteLength};

    #[test]
    fn one_ln_task_per_convert() {
        let osu = "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:4\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n64,192,0,1,0,0:0:0:0:\n";
        let length = NoteLength { unit: LengthUnit::Beat, value: 4.0 };
        let fln = TrainerTask { task: TrainerTaskParams::FlnConverter(FlnConverterParams { gap: length, min_length: length, rework_existing_ln: true }) };
        let no_ln = TrainerTask { task: TrainerTaskParams::NoLn(EmptyParams {}) };
        let mut beatmap = rosu_map::from_str::<Beatmap>(osu).unwrap();
        let error = run(&mut beatmap, [&fln, &no_ln], || None).unwrap_err();
        assert!(matches!(Error::from(error), Error::LnTasksTogether));
        assert!(run(&mut beatmap, [&no_ln], || None).is_ok());
    }
}
