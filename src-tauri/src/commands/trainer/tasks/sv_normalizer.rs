use anyhow::Result;
use rosu_map::{
    Beatmap,
    section::{general::GameMode, hit_objects::HitObjectKind, timing_points::DifficultyPoint},
};

use crate::commands::trainer::timing;

/// osu! clamps the slider velocity to this range.
const MIN_SV: f64 = 0.1;
const MAX_SV: f64 = 10.0;

/// Replaces every SV change (green line) with one per BPM change (red line),
/// so the scroll speed, which follows BPM × SV, stays the one of the main BPM
/// for the whole map. Red lines, kiai and hitsounds are kept: rosu-map keeps
/// them in their own control points.
///
/// Gimmick maps use fake BPMs (0.001, 100000...) that no SV within osu!'s
/// range can compensate. In mania a red line only sets the scroll speed and
/// the barlines, so those get the main BPM instead; in the other modes the BPM
/// also times the sliders, so they are kept and the SV is clamped.
///
/// Slider and drumroll lengths are rescaled to the new SV, otherwise their
/// duration (pixel length / SV) would change.
pub fn apply(beatmap: &mut Beatmap) -> Result<()> {
    let Some(main_bpm) = timing::main_bpm(beatmap) else {
        log::warn!("SV normalizer: the beatmap has no valid timing point, left as is");
        return Ok(());
    };

    let original = beatmap.control_points.difficulty_points.clone();

    let is_mania = beatmap.mode == GameMode::Mania;

    let mut normalized: Vec<DifficultyPoint> = Vec::new();
    let mut flattened = 0;

    for tp in beatmap.control_points.timing_points.iter_mut() {
        if !(tp.beat_len.is_finite() && tp.beat_len > 0.0) {
            continue;
        }

        let mut sv = main_bpm * tp.beat_len / 60_000.0;

        if is_mania && !(MIN_SV..=MAX_SV).contains(&sv) {
            tp.beat_len = 60_000.0 / main_bpm;
            sv = 1.0;
            flattened += 1;
        }

        let point = DifficultyPoint {
            time: tp.time,
            slider_velocity: sv.clamp(MIN_SV, MAX_SV),
            generate_ticks: DifficultyPoint::DEFAULT_GENERATE_TICKS,
        };

        // Two red lines at the same time: the last one wins, like in osu!.
        match normalized.last_mut() {
            Some(last) if last.time == point.time => *last = point,
            _ => normalized.push(point),
        }
    }

    normalized.sort_by(|a, b| a.time.total_cmp(&b.time));

    log::debug!(
        "SV normalizer: main BPM {:.2}, {} SV change(s) replaced by {}",
        main_bpm,
        original.len(),
        normalized.len()
    );

    if flattened > 0 {
        log::info!("SV normalizer: {} fake BPM(s) out of the SV range set to the main BPM", flattened);
    }

    beatmap.control_points.difficulty_points = normalized;

    let sv_at = |points: &[DifficultyPoint], time: f64| {
        let i = points.partition_point(|p| p.time <= time);
        i.checked_sub(1).map_or(DifficultyPoint::DEFAULT_SLIDER_VELOCITY, |i| points[i].slider_velocity)
    };

    for obj in beatmap.hit_objects.iter_mut() {
        let HitObjectKind::Slider(slider) = &mut obj.kind else { continue };

        let old_sv = sv_at(&original, obj.start_time);
        let new_sv = sv_at(&beatmap.control_points.difficulty_points, obj.start_time);

        if (old_sv - new_sv).abs() < f64::EPSILON {
            continue;
        }

        let dist = slider
            .path
            .expected_dist()
            .unwrap_or_else(|| slider.path.curve().dist());

        *slider.path.expected_dist_mut() = Some(dist * new_sv / old_sv);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const OSU: &str = "osu file format v14

[Difficulty]
SliderMultiplier:1.4

[TimingPoints]
0,500,4,1,0,60,1,0
1000,-50,4,1,0,60,0,1
8000,250,4,1,0,60,1,0
9000,-200,4,1,0,60,0,0

[HitObjects]
256,192,2000,2,0,L|356:192,1,140
256,192,8500,1,0,0:0:0:0:
";

    #[test]
    fn replaces_sv_changes_with_one_per_bpm_change() {
        let mut beatmap = rosu_map::from_str::<Beatmap>(OSU).unwrap();
        apply(&mut beatmap).unwrap();

        let svs: Vec<(f64, f64)> = beatmap
            .control_points
            .difficulty_points
            .iter()
            .map(|p| (p.time, p.slider_velocity))
            .collect();

        // 120 BPM lasts the longest, so 240 BPM gets 0.5x.
        assert_eq!(svs, vec![(0.0, 1.0), (8000.0, 0.5)]);
        // Kiai is kept.
        assert_eq!(beatmap.control_points.effect_points.len(), 2);
    }

    #[test]
    fn flattens_fake_bpms_in_mania() {
        let osu = "osu file format v14

[General]
Mode: 3

[TimingPoints]
0,500,4,1,0,60,1,0
1000,60000000,4,1,0,60,1,0
1100,0.6,4,1,0,60,1,0
1200,500,4,1,0,60,1,0

[HitObjects]
64,192,2000,1,0,0:0:0:0:
";
        let mut beatmap = rosu_map::from_str::<Beatmap>(osu).unwrap();
        apply(&mut beatmap).unwrap();

        let beat_lens: Vec<f64> = beatmap.control_points.timing_points.iter().map(|tp| tp.beat_len).collect();
        assert_eq!(beat_lens, vec![500.0; 4]);
        assert!(beatmap.control_points.difficulty_points.iter().all(|p| p.slider_velocity == 1.0));
    }

    #[test]
    fn keeps_slider_duration() {
        let mut beatmap = rosu_map::from_str::<Beatmap>(OSU).unwrap();
        apply(&mut beatmap).unwrap();

        let HitObjectKind::Slider(slider) = &beatmap.hit_objects[0].kind else { panic!() };
        // It was at 2x, now at 1x: half the length keeps the same duration.
        assert_eq!(slider.path.expected_dist(), Some(70.0));
    }
}
