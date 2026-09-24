use anyhow::Result;
use rosu_map::{Beatmap, section::hit_objects::HitObjectKind};

use crate::commands::trainer::model::RateChangeParams;

pub fn apply(beatmap: &mut Beatmap, params: &RateChangeParams) -> Result<()> {
    let rate = params.rate;

    // Control points: los 4 tipos necesitan su `time` escalado
    for tp in beatmap.control_points.timing_points.iter_mut() {
        tp.time = (tp.time / rate).round();
        tp.beat_len /= rate;
    }

    for dp in beatmap.control_points.difficulty_points.iter_mut() {
        dp.time = (dp.time / rate).round();
    }

    for ep in beatmap.control_points.effect_points.iter_mut() {
        ep.time = (ep.time / rate).round();
    }

    for sp in beatmap.control_points.sample_points.iter_mut() {
        sp.time = (sp.time / rate).round();
    }

    // Hit objects
    for obj in beatmap.hit_objects.iter_mut() {
        obj.start_time = (obj.start_time / rate).round();

        match &mut obj.kind {
            HitObjectKind::Circle(_) => {}
            HitObjectKind::Slider(_) => {}
            HitObjectKind::Spinner(spinner) => {
                spinner.duration = (spinner.duration / rate).round();
            }
            HitObjectKind::Hold(hold) => {
                hold.duration = (hold.duration / rate).round();
            }
        }
    }

    // Break periods
    for br in beatmap.breaks.iter_mut() {
        br.start_time = (br.start_time / rate).round();
        br.end_time = (br.end_time / rate).round();
    }

    let suffix = if params.adjust_pitch { " AP" } else { "" };
    beatmap.version = format!("{} ({}x{})", beatmap.version, rate, suffix);

    Ok(())
}