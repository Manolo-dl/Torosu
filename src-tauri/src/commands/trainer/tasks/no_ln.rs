use anyhow::Result;
use rosu_map::{
    Beatmap,
    section::{
        general::GameMode,
        hit_objects::{HitObjectCircle, HitObjectKind},
    },
    util::Pos,
};

use crate::{commands::trainer::mania, error::Error};

/// Turns every long note into a normal note where it starts, keeping its
/// column and hitsounds: the map as a rice map.
pub fn apply(beatmap: &mut Beatmap) -> Result<()> {
    if beatmap.mode != GameMode::Mania {
        return Err(Error::ManiaOnly { task: "NoLn" }.into());
    }

    let mut converted = 0;
    for obj in &mut beatmap.hit_objects {
        let HitObjectKind::Hold(hold) = &obj.kind else { continue };
        obj.kind = HitObjectKind::Circle(HitObjectCircle { pos: Pos::new(hold.pos_x, 192.0), new_combo: false, combo_offset: 0 });
        converted += 1;
    }

    log::debug!("No LN: {} long note(s) turned into normal notes", converted);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // 4K: a long note in column 0 with a whistle, a note in column 1, and a
    // long note in column 3.
    const OSU: &str = "osu file format v14

[General]
Mode: 3

[Difficulty]
CircleSize:4

[TimingPoints]
0,500,4,1,0,60,1,0

[HitObjects]
64,192,0,128,2,500:0:0:0:0:
192,192,250,1,0,0:0:0:0:
448,192,1000,128,0,1750:0:0:0:0:
";

    #[test]
    fn long_notes_become_notes_where_they_start() {
        let mut beatmap = rosu_map::from_str::<Beatmap>(OSU).unwrap();
        let before: Vec<(f64, usize, usize)> = beatmap
            .hit_objects
            .iter()
            .map(|o| (o.start_time, mania::column(mania::note(o).unwrap().0, 4), o.samples.len()))
            .collect();
        apply(&mut beatmap).unwrap();

        assert!(beatmap.hit_objects.iter().all(|o| matches!(o.kind, HitObjectKind::Circle(_))));
        let after: Vec<(f64, usize, usize)> = beatmap
            .hit_objects
            .iter()
            .map(|o| (o.start_time, mania::column(mania::note(o).unwrap().0, 4), o.samples.len()))
            .collect();
        assert_eq!(after, before, "same times, columns and hitsounds");
    }

    #[test]
    fn mania_only() {
        let mut beatmap = rosu_map::from_str::<Beatmap>(&OSU.replace("Mode: 3", "Mode: 0")).unwrap();
        assert!(apply(&mut beatmap).is_err());
    }
}
