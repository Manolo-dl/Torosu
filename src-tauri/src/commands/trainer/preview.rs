//! The skill (dan and MSD) each convert would have, shown before making them.

use std::{path::PathBuf, sync::Arc};

use futures_util::future::join_all;
use rosu_map::section::general::GameMode;
use tauri::{AppHandle, Manager};

use crate::{
    commands::trainer::{
        model::{RateChangeParams, RunTrainerRequest, TrainerTask, TrainerTaskParams},
        pipeline,
        run::{self, Source},
        tasks::rate_change,
    },
    error::Result,
    skill::{self, Skill},
    state::AppConfigState,
};

/// The skill of each convert of `request`, in order: `None` for one that
/// can't be rated (a map of another mode, or a task that fails).
///
/// Each convert is made as `run_trainer` makes it, without its audio, and
/// rated at its rate on the map at 1.0x, as Mania Tracker rates a map at a
/// rate: the same numbers it shows.
#[tauri::command]
pub async fn preview_converts(app: AppHandle, request: RunTrainerRequest) -> Result<Vec<Option<Skill>>> {
    let config = Arc::new(app.state::<AppConfigState>().config.lock().unwrap().trainer.clone());
    // Not shown, not rated: the frontend doesn't ask then either.
    if !config.shows_skill() {
        return Ok(vec![None; request.converts.len()]);
    }
    let path = PathBuf::from(&request.osu_file_path);
    let source = Arc::new(tauri::async_runtime::spawn_blocking(move || Source::read(&path, request.difficulty)).await??);
    if source.beatmap.mode != GameMode::Mania {
        return Ok(vec![None; request.converts.len()]);
    }

    let skills = join_all(request.converts.into_iter().enumerate().map(|(i, convert)| {
        let (source, config) = (source.clone(), config.clone());
        async move {
            tauri::async_runtime::spawn_blocking(move || {
                let mut tasks = convert.tasks;
                run::adjust_tasks(&source, &mut tasks, &config);
                rate(&source, &tasks)
                    .inspect_err(|e| log::warn!("could not preview convert {}: {e:#}", i + 1))
                    .ok()
                    .flatten()
            })
            .await
            .ok()
            .flatten()
        }
    }))
    .await;
    Ok(skills)
}

/// The skill of `source` with `tasks` applied.
fn rate(source: &Source, tasks: &[TrainerTask]) -> anyhow::Result<Option<Skill>> {
    // Rated at its rate on the map at 1.0x, as Mania Tracker does.
    let rate: f64 = tasks
        .iter()
        .filter_map(|t| match &t.task {
            TrainerTaskParams::RateChange(params) => Some(params.rate),
            _ => None,
        })
        .product();
    // The FLN gaps in ms and the key converter's accents depend on the times
    // a rate change before them gives: then the tasks run in their order and
    // the rate is taken back out after, which can move a note by a ms.
    // Otherwise the rate changes are left out, so the map is the original.
    let first_rate = tasks.iter().position(|t| matches!(t.task, TrainerTaskParams::RateChange(_)));
    let timed_after_rate = first_rate.is_some_and(|first| {
        tasks[first..].iter().any(|t| matches!(t.task, TrainerTaskParams::FlnConverter(_) | TrainerTaskParams::KeyConverter(_)))
    });
    let tasks: Vec<&TrainerTask> =
        tasks.iter().filter(|t| timed_after_rate || !matches!(t.task, TrainerTaskParams::RateChange(_))).collect();

    let mut beatmap = source.beatmap.clone();
    pipeline::run(&mut beatmap, tasks, || source.accents())?;
    if timed_after_rate {
        let params = RateChangeParams { rate: 1.0 / rate, adjust_pitch: false, scale_storyboard: false, scale_video: false };
        rate_change::apply(&mut beatmap, &params)?;
    }

    Ok(skill::estimate(&beatmap.encode_to_string()?, rate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::{
            config::model::TrainerConfig,
            trainer::model::{DifficultySettings, FlnConverterParams, LengthUnit, NoteLength},
        },
        skill::tests::stream_map,
        test_utils::TestDir,
    };

    fn source(dir: &TestDir) -> Source {
        dir.write("map.osu", &stream_map(4));
        Source::read(&dir.0.join("map.osu"), DifficultySettings { hp: 8.0, cs: 4.0, od: 8.0, ar: 5.0 }).unwrap()
    }

    fn rate_change(rate: f64) -> TrainerTask {
        TrainerTask {
            task: TrainerTaskParams::RateChange(RateChangeParams { rate, adjust_pitch: false, scale_storyboard: false, scale_video: false }),
        }
    }

    fn preview(source: &Source, mut tasks: Vec<TrainerTask>) -> Skill {
        run::adjust_tasks(source, &mut tasks, &TrainerConfig::default());
        rate(source, &tasks).unwrap().unwrap()
    }

    #[test]
    fn rate_change_as_mania_tracker_rates_it() {
        let dir = TestDir::new("preview-rate");
        let source = source(&dir);
        assert_eq!(preview(&source, vec![rate_change(1.3)]), skill::estimate(&stream_map(4), 1.3).unwrap());
        assert_eq!(preview(&source, vec![]), skill::estimate(&stream_map(4), 1.0).unwrap());
    }

    #[test]
    fn fln_makes_an_ln_map() {
        let dir = TestDir::new("preview-fln");
        let source = source(&dir);
        let fln = TrainerTask {
            task: TrainerTaskParams::FlnConverter(FlnConverterParams {
                gap: NoteLength { unit: LengthUnit::Beat, value: 8.0 },
                min_length: NoteLength { unit: LengthUnit::Ms, value: 0.0 },
                rework_existing_ln: true,
            }),
        };
        let rice = preview(&source, vec![]);
        let ln = preview(&source, vec![fln]);
        assert!(rice.dan.unwrap().ln.is_none());
        assert!(ln.dan.as_ref().unwrap().ln.is_some(), "{ln:?}");
    }
}
