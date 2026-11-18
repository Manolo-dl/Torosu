use anyhow::Result;
use rosu_map::{
    Beatmap,
    section::{
        general::GameMode,
        hit_objects::{HitObjectCircle, HitObjectHold, HitObjectKind},
    },
    util::Pos,
};

use crate::{
    commands::trainer::{mania, model::ShortLnParams, timing},
    error::Error,
};

/// A long note is released at least this part of a beat (1/8) before the
/// next note of its column, so the two can be told apart and played.
const MIN_RELEASE_GAP: f64 = 8.0;
/// As in the FLN converter: 1-2 ms of rounding must not turn some notes into
/// long notes and not their neighbours.
const MIN_LENGTH_LENIENCY: f64 = 2.0;

/// Turns every note, normal or long, into a short long note: as long as the
/// room before the next note of its column lets it, up to `max_length`, and
/// released [`MIN_RELEASE_GAP`] before that note. One without room for
/// `min_length` becomes a normal note. Dense parts get the shorter ones,
/// open ones `max_length`; the last note of each column has all the room.
pub fn apply(beatmap: &mut Beatmap, params: &ShortLnParams) -> Result<()> {
    if beatmap.mode != GameMode::Mania {
        return Err(Error::ManiaOnly { task: "ShortLn" }.into());
    }

    let keys = mania::key_count(beatmap);
    let beat_len_at = timing::beat_len_at(beatmap);

    let mut columns: Vec<Vec<usize>> = vec![Vec::new(); keys];
    for (i, obj) in beatmap.hit_objects.iter().enumerate() {
        let Some((x, _)) = mania::note(obj) else { continue };
        columns[mania::column(x, keys)].push(i);
    }

    // (index, new length): `None` turns it into a normal note.
    let mut changes: Vec<(usize, Option<f64>)> = Vec::new();
    for column in columns.iter_mut() {
        column.sort_by(|&a, &b| beatmap.hit_objects[a].start_time.total_cmp(&beatmap.hit_objects[b].start_time));

        for (pos, &i) in column.iter().enumerate() {
            let start = beatmap.hit_objects[i].start_time;
            let next = column[pos + 1..].iter().map(|&n| beatmap.hit_objects[n].start_time).find(|&t| t > start);

            let beat_len = beat_len_at(start);
            let min_length = params.min_length.ms(beat_len);
            // A maximum under the minimum is the minimum.
            let max_length = params.max_length.ms(beat_len).max(min_length);
            let room = next.map_or(f64::INFINITY, |next| next - beat_len_at(next) / MIN_RELEASE_GAP - start);
            let length = (start + max_length.min(room)).round() - start;

            if length <= 0.0 || length < min_length.round() - MIN_LENGTH_LENIENCY {
                changes.push((i, None));
            } else {
                changes.push((i, Some(length)));
            }
        }
    }

    // It reads the map's timing, which the changes leave as it is.
    drop(beat_len_at);
    let mut converted = 0;
    for (i, length) in changes {
        let obj = &mut beatmap.hit_objects[i];
        let Some((x, _)) = mania::note(obj) else { continue };
        obj.kind = match length {
            Some(duration) => {
                converted += 1;
                HitObjectKind::Hold(HitObjectHold { pos_x: x, duration })
            }
            None => HitObjectKind::Circle(HitObjectCircle { pos: Pos::new(x, 192.0), new_combo: false, combo_offset: 0 }),
        };
    }

    log::debug!("Short LN: {} note(s) turned into short long notes", converted);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::trainer::model::{LengthUnit, NoteLength};

    // 4K at 120 BPM (500 ms per beat). Column 0: notes at 0, 1000, 1125 and
    // 1250, then a long note at 3000. Column 1: a note at 0, second once
    // sorted.
    const OSU: &str = "osu file format v14

[General]
Mode: 3

[Difficulty]
CircleSize:4

[TimingPoints]
0,500,4,1,0,60,1,0

[HitObjects]
64,192,0,1,0,0:0:0:0:
64,192,1000,1,0,0:0:0:0:
64,192,1125,1,0,0:0:0:0:
64,192,1250,1,0,0:0:0:0:
64,192,3000,128,0,4500:0:0:0:0:
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
    fn as_long_as_the_room_lets_up_to_the_maximum() {
        let mut beatmap = rosu_map::from_str::<Beatmap>(OSU).unwrap();
        apply(&mut beatmap, &ShortLnParams { min_length: beat(16.0), max_length: beat(2.0) }).unwrap();

        // 0: 1/2 beat, with room for it. 1000: 125 ms to the next note, less
        // its 1/8 gap, is 62.5 -> 63, over 1/16 (31). 1250: 1/2 beat, the last
        // ones too; the 1.5 s long note gets short.
        assert_eq!(durations(&beatmap), vec![Some(250.0), Some(250.0), Some(63.0), Some(63.0), Some(250.0), Some(250.0)]);
    }

    #[test]
    fn notes_without_room_for_the_minimum_stay_notes() {
        let mut beatmap = rosu_map::from_str::<Beatmap>(OSU).unwrap();
        apply(&mut beatmap, &ShortLnParams { min_length: beat(4.0), max_length: NoteLength { unit: LengthUnit::Ms, value: 200.0 } }).unwrap();

        // The 1/4 notes have 62.5 ms of room, under 1/4 beat (125).
        assert_eq!(durations(&beatmap), vec![Some(200.0), Some(200.0), None, None, Some(200.0), Some(200.0)]);
    }

    #[test]
    fn a_maximum_under_the_minimum_is_the_minimum() {
        let mut beatmap = rosu_map::from_str::<Beatmap>(OSU).unwrap();
        apply(&mut beatmap, &ShortLnParams { min_length: beat(4.0), max_length: beat(8.0) }).unwrap();
        assert_eq!(durations(&beatmap)[0], Some(125.0));
    }
}
