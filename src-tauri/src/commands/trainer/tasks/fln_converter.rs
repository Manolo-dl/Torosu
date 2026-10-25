use anyhow::{Result, bail};
use rosu_map::{
    Beatmap,
    section::{
        general::GameMode,
        hit_objects::{HitObjectCircle, HitObjectHold, HitObjectKind},
    },
    util::Pos,
};

use crate::commands::trainer::{
    model::{FlnConverterParams, LengthUnit, NoteLength},
    naming,
};

/// Avoids 1-2 ms of rounding turning some notes into LNs and not their
/// neighbours, when they should all be the same (from osuLNmaster).
const MIN_LENGTH_LENIENCY: f64 = 2.0;

/// Turns every note into a long note held until shortly before the next note
/// in its column (full LN), like osuLNmaster and cosu-trainer's invert.
///
/// The last note of each column has nothing to be held until, so it stays as
/// it is.
pub fn apply(beatmap: &mut Beatmap, params: &FlnConverterParams) -> Result<()> {
    if beatmap.mode != GameMode::Mania {
        bail!("the FLN converter only works on osu!mania maps");
    }

    let keys = beatmap.circle_size.round().max(1.0) as usize;
    let main_beat_len = naming::main_bpm(beatmap).map(|bpm| 60_000.0 / bpm);

    // Gimmick maps use fake BPMs (0.001, 100000...) that would make a beat
    // based gap absurd, so those fall back to the main BPM.
    let beat_len_at = |time: f64| {
        let beat_len = beatmap.control_points.timing_point_at(time).map(|tp| tp.beat_len);
        match (beat_len, main_beat_len) {
            (Some(b), Some(main)) if !(b.is_finite() && b > 0.0 && (main / 10.0..=main * 10.0).contains(&b)) => main,
            (Some(b), _) if b.is_finite() && b > 0.0 => b,
            (_, main) => main.unwrap_or(500.0),
        }
    };

    let mut columns: Vec<Vec<usize>> = vec![Vec::new(); keys];
    for (i, obj) in beatmap.hit_objects.iter().enumerate() {
        let x = match &obj.kind {
            HitObjectKind::Circle(circle) => circle.pos.x,
            HitObjectKind::Hold(hold) => hold.pos_x,
            _ => continue,
        };
        let column = ((x * keys as f32 / 512.0).floor().max(0.0) as usize).min(keys - 1);
        columns[column].push(i);
    }

    // (index, new length): `None` turns it into a normal note.
    let mut changes: Vec<(usize, Option<f64>)> = Vec::new();

    for column in columns.iter_mut() {
        column.sort_by(|&a, &b| beatmap.hit_objects[a].start_time.total_cmp(&beatmap.hit_objects[b].start_time));

        for (pos, &i) in column.iter().enumerate() {
            let obj = &beatmap.hit_objects[i];

            if !params.rework_existing_ln && matches!(obj.kind, HitObjectKind::Hold(_)) {
                continue;
            }

            let start = obj.start_time;
            let Some(next) = column[pos + 1..]
                .iter()
                .map(|&n| beatmap.hit_objects[n].start_time)
                .find(|&t| t > start)
            else {
                continue;
            };

            let gap = length_ms(params.gap, beat_len_at(next));
            let min_length = length_ms(params.min_length, beat_len_at(start));
            let length = (next - gap).round() - start;

            if length < min_length.round() - MIN_LENGTH_LENIENCY || length <= 0.0 {
                changes.push((i, None));
            } else {
                changes.push((i, Some(length)));
            }
        }
    }

    let mut converted = 0;

    for (i, length) in changes {
        let obj = &mut beatmap.hit_objects[i];
        let x = match &obj.kind {
            HitObjectKind::Circle(circle) => circle.pos.x,
            HitObjectKind::Hold(hold) => hold.pos_x,
            _ => continue,
        };

        obj.kind = match length {
            Some(duration) => {
                converted += 1;
                HitObjectKind::Hold(HitObjectHold { pos_x: x, duration })
            }
            None => HitObjectKind::Circle(HitObjectCircle {
                pos: Pos::new(x, 192.0),
                new_combo: false,
                combo_offset: 0,
            }),
        };
    }

    log::debug!("FLN converter: {} note(s) turned into long notes", converted);

    Ok(())
}

fn length_ms(length: NoteLength, beat_len: f64) -> f64 {
    match length.unit {
        LengthUnit::Beat if length.value > 0.0 => beat_len / length.value,
        LengthUnit::Beat => 0.0,
        LengthUnit::Ms => length.value.max(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 4K at 120 BPM (500 ms per beat). Column 0: notes at 0, 500, 625, 1000
    // (an LN) and 2000. Column 1: a single note at 0, second once sorted.
    const OSU: &str = "osu file format v14

[General]
Mode: 3

[Difficulty]
CircleSize:4

[TimingPoints]
0,500,4,1,0,60,1,0

[HitObjects]
64,192,0,1,0,0:0:0:0:
64,192,500,1,0,0:0:0:0:
64,192,625,1,0,0:0:0:0:
64,192,1000,128,0,1500:0:0:0:0:
64,192,2000,1,0,0:0:0:0:
192,192,0,1,0,0:0:0:0:
";

    fn beat(value: f64) -> NoteLength {
        NoteLength { unit: LengthUnit::Beat, value }
    }

    fn durations(beatmap: &Beatmap) -> Vec<Option<f64>> {
        beatmap
            .hit_objects
            .iter()
            .map(|h| match &h.kind {
                HitObjectKind::Hold(hold) => Some(hold.duration),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn holds_until_the_gap_before_the_next_note() {
        let mut beatmap = rosu_map::from_str::<Beatmap>(OSU).unwrap();
        let params = FlnConverterParams { gap: beat(4.0), min_length: beat(4.0), rework_existing_ln: true };
        apply(&mut beatmap, &params).unwrap();

        // 0 -> 500 - 125. 500 -> 625 - 125 is empty, so a note. The last note
        // of each column is left as it was.
        assert_eq!(
            durations(&beatmap),
            vec![Some(375.0), None, None, Some(250.0), Some(875.0), None]
        );
    }

    #[test]
    fn keeps_existing_lns_unless_reworked() {
        let mut beatmap = rosu_map::from_str::<Beatmap>(OSU).unwrap();
        let params = FlnConverterParams {
            gap: NoteLength { unit: LengthUnit::Ms, value: 50.0 },
            min_length: beat(2.0),
            rework_existing_ln: false,
        };
        apply(&mut beatmap, &params).unwrap();

        // 0 -> 450, 500 -> 75 is under half a beat, the LN keeps its 500 ms.
        assert_eq!(
            durations(&beatmap),
            vec![Some(450.0), None, None, Some(325.0), Some(500.0), None]
        );
    }
}
