//! The patterns of the source kept on the target: trills, jumptrills,
//! chordjacks, stairs, and the outer notes of a chord.

use super::*;

#[test]
fn keeps_the_outer_notes_of_a_chord() {
    assert_eq!(kept(4, 4), vec![0, 1, 2, 3]);
    assert_eq!(kept(4, 2), vec![0, 3]);
    assert_eq!(kept(5, 3), vec![0, 2, 4]);
    assert_eq!(kept(3, 1), vec![1]);
}

#[test]
fn keeps_real_trills() {
    let (_, beatmap) = convert(&stream(&[0, 1, 0, 1, 0, 1, 0, 1, 0, 1]), 7, Density::None);
    let cols = columns(&beatmap, 7);
    assert!(cols.windows(3).all(|w| w[0] == w[2] && w[0] != w[1]), "{:?}", cols);
}

#[test]
fn keeps_jumptrills() {
    let trill: Vec<&[usize]> = (0..12).map(|i| if i % 2 == 0 { &[0, 1][..] } else { &[2, 3][..] }).collect();
    for density in [Density::None, Density::Full] {
        let (_, beatmap) = convert(&rows_map(&trill), 7, density);
        let rows = key_rows(&beatmap, 7);
        assert!(rows.windows(3).all(|w| w[0] == w[2]), "{:?}", rows);
        assert!(rows.windows(2).all(|w| w[0].iter().all(|c| !w[1].contains(c))), "{:?}", rows);
    }
}

#[test]
fn keeps_chordjacks() {
    // Quads every 1/2 beat.
    let osu = rows_map(&[&[0, 1, 2, 3], &[], &[0, 1, 2, 3], &[], &[0, 1, 2, 3], &[], &[0, 1, 2, 3]]);
    let (_, beatmap) = convert(&osu, 7, Density::Full);
    let rows = key_rows(&beatmap, 7);
    assert!(rows.windows(2).all(|w| w[0] == w[1]), "{:?}", rows);
    // A full row grows by a note or so, not into a wall.
    assert!(rows[0].len() <= 5, "{:?}", rows);
}

#[test]
fn rolls_short_back_and_forths() {
    // A 4K stream that comes back to the same columns, like the one in
    // Black dress: on 7K it should roll over the keys instead.
    let (_, beatmap) = convert(&stream(&[1, 2, 3, 0, 2, 1, 0, 3, 2, 1, 2, 1, 3, 2, 0, 1, 2, 0]), 7, Density::None);
    let cols = columns(&beatmap, 7);
    let back_and_forths = cols.windows(3).filter(|w| w[0] == w[2]).count();
    assert!(back_and_forths <= 1, "{:?}", cols);
}

#[test]
fn stairs_keep_their_direction() {
    let (_, beatmap) = convert(&stair(), 7, Density::None);
    let cols: Vec<usize> = beatmap.hit_objects.iter().map(|o| column(o, 7)).collect();
    // Within each 0123 the target goes right too.
    let rising = cols.chunks(4).filter(|c| c.windows(2).all(|w| w[0] < w[1])).count();
    assert!(rising >= 15, "{:?}", cols);
}

#[test]
fn anchors_of_the_source_are_shared_between_keys() {
    // A 4K jumpstream anchored on column 1 for 16 chords: hand-made 7K maps
    // seldom keep one key in all of them, and nor does Full.
    let lone = [&[3][..], &[0], &[2], &[3]];
    let chords = [&[0, 1][..], &[1, 2], &[1, 3], &[0, 1, 2]];
    let rows: Vec<&[usize]> = (0..32).map(|i| if i % 2 == 0 { chords[i / 2 % 4] } else { lone[i / 2 % 4] }).collect();
    let (_, beatmap) = convert(&rows_map(&rows), 7, Density::Full);
    let chord_rows: Vec<Vec<usize>> = key_rows(&beatmap, 7).into_iter().step_by(2).collect();
    let longest = (0..7)
        .map(|k| chord_rows.iter().fold((0, 0), |(run, best), r| if r.contains(&k) { (run + 1, best.max(run + 1)) } else { (0, best) }).1)
        .max()
        .unwrap();
    assert!(longest <= 6, "{} {:?}", longest, chord_rows);
}

#[test]
fn jumptrills_keep_a_hand_per_phase() {
    // A long 4K jumptrill: on 7K each phase stays on its own hand, the
    // middle key with one of them at most, repeating the same two rows.
    let trill: Vec<&[usize]> = (0..32).map(|i| if i % 2 == 0 { &[0, 1][..] } else { &[2, 3][..] }).collect();
    for density in [Density::None, Density::Full] {
        let (_, beatmap) = convert(&rows_map(&trill), 7, density);
        check(&beatmap, 7);
        let rows = key_rows(&beatmap, 7);
        assert!(rows.windows(3).all(|w| w[0] == w[2]), "{:?}", rows);
        let side = |r: &[usize]| r.iter().filter(|&&c| c != 3).fold(0, |m, &c| m | if c < 3 { 1 } else { 2 });
        for w in rows.windows(2) {
            assert!(side(&w[0]) & side(&w[1]) == 0, "{:?}", rows);
            assert!(!(w[0].contains(&3) && w[1].contains(&3)), "{:?}", rows);
        }
    }
}

#[test]
fn trills_speeding_up_add_no_jacks() {
    // A 4K jumptrill that starts at 1/2 and goes on at 1/4, like the one
    // in FUTURE DOMINATORS: its first two rows, 1/2 apart, may share the
    // middle key, but copied on at 1/4 that is a jack.
    let lead: [&[usize]; 5] = [&[0, 3], &[], &[0, 1], &[], &[2, 3]];
    let trill = (0..6).map(|i| if i % 2 == 0 { &[0, 1][..] } else { &[2, 3][..] });
    let rows: Vec<&[usize]> = lead.into_iter().chain(trill).chain([&[1][..], &[0, 3][..]]).collect();
    for density in [Density::None, Density::Dense, Density::MoreDense, Density::Full] {
        let (_, beatmap) = convert(&rows_map(&rows), 7, density);
        assert_eq!(check(&beatmap, 7), 0, "{:?}", key_rows(&beatmap, 7));
    }
}
