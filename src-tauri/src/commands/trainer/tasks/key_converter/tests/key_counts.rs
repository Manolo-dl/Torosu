//! Converting up and down in keys, with each density and playstyle: every
//! note placed, no new jacks, a finger on one key at a time.

use super::*;

#[test]
fn expanding_keeps_every_note_and_adds_no_jacks() {
    for (src, dst) in [(4, 7), (4, 10), (7, 8), (7, 10), (4, 5), (6, 9)] {
        let (original, beatmap) = convert(&map(src), dst, Density::None);
        assert_eq!(beatmap.circle_size, dst as f32);
        assert_eq!(beatmap.hit_objects.len(), original.hit_objects.len(), "{}K -> {}K", src, dst);
        assert!(check(&beatmap, dst) <= check(&original, src), "{}K -> {}K", src, dst);
    }
}

#[test]
fn expanding_uses_every_key_evenly() {
    let (_, beatmap) = convert(&map(4), 7, Density::None);
    let mut used = [0usize; 7];
    for obj in &beatmap.hit_objects {
        used[column(obj, 7)] += 1;
    }
    let even = beatmap.hit_objects.len() / 7;
    assert!(used.iter().all(|&n| n > even / 2), "{:?}", used);
}

#[test]
fn density_adds_notes_without_many_jacks() {
    let (original, beatmap) = convert(&map(4), 10, Density::MoreDense);
    assert!(beatmap.hit_objects.len() > original.hit_objects.len());
    // Chordjacks copy their extra notes too, so their jacks grow with
    // them; the rest adds none.
    assert!(check(&beatmap, 10) as f64 <= check(&original, 4) as f64 * 1.2);
}

#[test]
fn full_scales_chords_with_the_keys() {
    let (original, beatmap) = convert(&map(4), 7, Density::Full);
    let ratio = beatmap.hit_objects.len() as f64 / original.hit_objects.len() as f64;
    // 7/4 * 0.9, give or take what does not fit.
    assert!((1.4..=1.65).contains(&ratio), "{}", ratio);
    check(&beatmap, 7);
}

#[test]
fn full_grows_the_beat_more_than_the_quarters() {
    // A 4K stream of single notes in 1/4s: on 7K the notes on the beat
    // and its 1/2 become jumps and hands, the 1/4s between stay light.
    let (_, beatmap) = convert(&stair(), 7, Density::Full);
    let rows = key_rows(&beatmap, 7);
    let mean = |phase: &[usize]| {
        let sizes: Vec<usize> = rows.iter().enumerate().filter(|(i, _)| phase.contains(&(i % 4))).map(|(_, r)| r.len()).collect();
        sizes.iter().sum::<usize>() as f64 / sizes.len() as f64
    };
    let (beat, quarters) = (mean(&[0, 2]), mean(&[1, 3]));
    assert!(beat >= 2.0, "{} {:?}", beat, rows);
    assert!(quarters <= 1.7, "{} {:?}", quarters, rows);

    // 1/8s do not grow.
    let osu = stair().replace("TimingPoints]\n0,500", "TimingPoints]\n0,1000");
    let (_, beatmap) = convert(&osu, 7, Density::Full);
    let rows = key_rows(&beatmap, 7);
    assert!(rows.iter().skip(1).step_by(2).all(|r| r.len() == 1), "{:?}", rows);
}

#[test]
fn full_keeps_the_thumb_out_of_big_chords() {
    // Hand-made 7K maps build their big chords with the hands: the middle
    // key is in fewer of them than the others are.
    let (_, beatmap) = convert(&map(4), 7, Density::Full);
    let big: Vec<Vec<usize>> = key_rows(&beatmap, 7).into_iter().filter(|r| r.len() >= 4).collect();
    let share = |key: usize| big.iter().filter(|r| r.contains(&key)).count() as f64 / big.len() as f64;
    let others = [0, 1, 2, 4, 5, 6].iter().map(|&k| share(k)).sum::<f64>() / 6.0;
    assert!(big.len() >= 20, "{} big chords", big.len());
    assert!(share(3) < others, "middle {} others {}", share(3), others);
}

#[test]
fn reducing_shrinks_chords_without_jacks() {
    // A 7K hand trill, like the one in Ice Angel: on 4K it should become a
    // jumptrill, not three keys jacking every row.
    let mut osu = String::from(
        "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:7\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
    );
    for step in 0..16 {
        let cols: &[usize] = if step % 2 == 0 { &[0, 1, 2] } else { &[4, 5, 6] };
        for col in cols {
            osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", (col * 512 + 256) / 7, step * 125));
        }
    }
    let (_, beatmap) = convert(&osu, 4, Density::None);
    let rows = key_rows(&beatmap, 4);
    assert!(rows.iter().all(|r| r.len() == 2), "{:?}", rows);
    assert_eq!(check(&beatmap, 4), 0, "{:?}", rows);
}

#[test]
fn every_8k_playstyle_stays_playable() {
    // Every 8K playstyle moves the hands (3+2+3: the middle keys are the
    // thumbs'; 7K+1: the first key is a scratch), and all stay playable.
    let mut params = KeyConverterParams { target_keys: 8, density: Density::None, keep_jacks: true, playstyle: None };
    let original = rosu_map::from_str::<Beatmap>(&map(4)).unwrap();
    for playstyle in [None, Some("3+2+3".to_string()), Some("7K+1".to_string())] {
        params.playstyle = playstyle;
        let mut beatmap = original.clone();
        apply(&mut beatmap, &params).unwrap();
        check(&beatmap, 8);
    }
}

#[test]
fn wide_hands_keep_chords_open() {
    // 7K -> 10K: extra notes no longer grow every chord into a pill (More
    // dense made 99% of them pills), and the plain conversion makes few.
    for (name, density) in [("None", Density::None), ("MoreDense", Density::MoreDense), ("Full", Density::Full)] {
        let (_, beatmap) = convert(&map(7), 10, density);
        check(&beatmap, 10);
        let rate = pill_rate(&beatmap);
        assert!(rate < 0.4, "{}: {:.2}", name, rate);
    }
}

#[test]
fn reducing_stays_playable() {
    for (src, dst) in [(7, 4), (10, 7), (4, 1), (18, 7), (14, 4), (12, 10), (16, 14)] {
        let (_, beatmap) = convert(&map(src), dst, Density::MoreDense);
        assert_eq!(beatmap.circle_size, dst as f32);
        check(&beatmap, dst);
    }
}

#[test]
fn two_stages_stay_playable() {
    // 12K to 18K in every playstyle of the osu! wiki: every note kept and
    // no new jacks, and with Full no overlaps either. 10K2S adds the
    // scratches' own notes to them, or moves a pinky's out to its scratch.
    for dst in [12, 14, 16, 18] {
        for layout in layout::layouts(dst).iter().filter(|l| l.fingers.iter().all(|f| f.player == 0)) {
            let scratches = layout.inner_keys().map_or(0, |_| layout.digit_mask(Digit::Scratch));
            for src in [4, 7, 10] {
                let original = rosu_map::from_str::<Beatmap>(&map(src)).unwrap();
                let beatmap = convert_to(&map(src), layout.name, dst, Density::None);
                let scratched = beatmap.hit_objects.iter().filter(|o| scratches & 1 << column(o, dst) != 0).count();
                let kept = beatmap.hit_objects.len() - scratched;
                let notes = original.hit_objects.len();
                assert!(kept <= notes && kept + scratched >= notes, "{}K -> {}K {}", src, dst, layout.name);
                assert!(check(&beatmap, dst) <= check(&original, src), "{}K -> {}K {}", src, dst, layout.name);
                check(&convert_to(&map(src), layout.name, dst, Density::Full), dst);
            }
        }
    }
}

#[test]
fn fingers_do_not_jack_on_other_keys() {
    // A finger with two keys (DP, 9K9K, 10K8K) going from one to the
    // other in a stream is a jack too. Hand-made maps do it for 0 to 2.5%
    // of the notes; on this dense map converts did it for 3.5 to 22%
    // before fingers were shared, and for 0.2 to 3.3% after.
    for (dst, name) in [(16, "DP"), (18, "9K9K"), (18, "10K8K")] {
        let layout = layout::layout(dst, Some(name));
        for density in [Density::None, Density::Full] {
            let beatmap = convert_to(&map(7), name, dst, density);
            let rate = finger_jacks(&beatmap, &layout) as f64 / beatmap.hit_objects.len() as f64;
            assert!(rate < 0.04, "{}K {}: {:.3}", dst, name, rate);
        }
    }
}

#[test]
fn a_finger_plays_one_key_at_a_time() {
    // 10K2S: the pinky plays the scratch and the key next to it, so the
    // first two keys (and the last two) never have notes at once. The same
    // goes for the fingers with two keys on DP, 9K9K and 10K8K.
    let densities = [("None", Density::None), ("MoreDense", Density::MoreDense), ("Full", Density::Full)];
    for dst in [12, 14, 16, 18] {
        for layout in layout::layouts(dst).iter().filter(|l| l.fingers.iter().all(|f| f.player == 0)) {
            for src in [4, 7, 10] {
                for (name, density) in densities {
                    let beatmap = convert_to(&map(src), layout.name, dst, density);
                    check_fingers(&beatmap, layout, &format!("{}K -> {}K {} {}", src, dst, layout.name, name));
                }
            }
        }
    }
}

#[test]
fn shared_fingers_follow_the_playstyle() {
    for keys in 1..=10 {
        for layout in layout::layouts(keys) {
            let shared = shared_fingers(&layout);
            assert!((0..keys).all(|k| shared[k] == 1 << k), "{}K {}", keys, layout.name);
        }
    }
    let shared = |keys, name| shared_fingers(&layout::layout(keys, Some(name)));
    assert_eq!(shared(18, "10K8K")[0], 1 | 1 << 4);
    assert_eq!(shared(14, "DP")[1], 0b110);
    // The scratch goes with the pinky; the middle keys of EZ2AC with no one.
    assert_eq!(shared(12, "10K2S")[11], 0b11 << 10);
    assert_eq!(shared(14, "EZ2AC")[6], 1 << 6);
}

#[test]
fn converts_only_to_what_osu_plays() {
    let mut beatmap = rosu_map::from_str::<Beatmap>(&map(7)).unwrap();
    for (keys, playstyle) in [(11, None), (20, None), (14, Some("Co-op"))] {
        let params = KeyConverterParams { target_keys: keys, density: Density::None, keep_jacks: true, playstyle: playstyle.map(String::from) };
        assert!(apply(&mut beatmap, &params).is_err(), "{}K {:?}", keys, playstyle);
    }
}

#[test]
fn combos_go_in_the_order_of_their_masks() {
    let own: [u32; MAX_KEYS] = std::array::from_fn(|k| 1 << k);
    let mut seen = Vec::new();
    each_combo(&[&[0, 1, 2], &[1, 2, 3]], &own, &mut [0; 2], &mut |c| seen.push(c.to_vec()));
    assert_eq!(seen, [[0, 1], [0, 2], [1, 2], [0, 3], [1, 3], [2, 3]]);
    // Keys 0 and 1 of one finger (a scratch and its pinky) never go together.
    let mut shared = own;
    shared[0] = 0b11;
    shared[1] = 0b11;
    seen.clear();
    each_combo(&[&[0, 1, 2], &[1, 2, 3]], &shared, &mut [0; 2], &mut |c| seen.push(c.to_vec()));
    assert_eq!(seen, [[0, 2], [1, 2], [0, 3], [1, 3], [2, 3]]);
    assert_eq!(fingers_count(&shared, 4), 3);
    assert_eq!(fingers_count(&shared_fingers(&layout::layout(12, Some("10K2S"))), 12), 10);
    // And their tie breaks are the ones of the list of every mask.
    let mut next = [0usize; 11];
    for mask in 1u32..1 << 10 {
        let size = mask.count_ones() as usize;
        assert_eq!(rank(mask), next[size], "{:b}", mask);
        next[size] += 1;
    }
}
