//! Scratches: 10K2S as a 10K map with scratches of its own on the accents,
//! and 7K+1 as the 7K map with a scratch.

use super::*;

#[test]
fn scratches_are_accents() {
    // 10K2S from 4K, 7K, 10K and 7K FLN: the scratches get about as many
    // notes as a key, and the pinky shares its notes with the key next to
    // them, but never for bars on end. With FLN a column can leave the
    // scratch when its long note ends.
    let sources = [("4K", map(4)), ("7K", map(7)), ("10K", map(10)), ("7K FLN", fln_map())];
    for (src, osu) in &sources {
        for (name, density) in [("None", Density::None), ("MoreDense", Density::MoreDense), ("Full", Density::Full)] {
            let beatmap = convert_to(osu, "12K", 12, density);
            let (share, pinky, longest) = scratch_use(&beatmap);
            assert!(share >= 0.35, "{} -> 12K {}: {:.2}x a key", src, name, share);
            assert!((0.6..=1.6).contains(&(share / pinky)), "{} -> 12K {}: {:.2}x a key, the key next to it {:.2}x", src, name, share, pinky);
            assert!(longest <= 8, "{} -> 12K {}: a run of {}", src, name, longest);
        }
    }
}

#[test]
fn ten_k_two_s_is_a_10k_map_and_scratches() {
    // 10K2S: the 10K map as it is on the inner keys but for the notes the
    // scratches took (their hand's one in a chord, or their pinky's, and
    // on the strong accents all of them, one moved out to the scratch),
    // and notes of their own on the scratches, plain, never with the key
    // of their pinky nor 3/8 beat around it, mostly on the beats. Their
    // hand is free: nothing in their row, nothing held across it.
    let layout = layout::layout(12, Some("10K2S"));
    let original = rosu_map::from_str::<Beatmap>(&map(10)).unwrap();
    let sorted = |mut notes: Vec<(f64, f64, usize)>| {
        notes.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.cmp(&b.2)));
        notes
    };
    let original_notes = sorted(notes_of(&original, 10));
    for density in [Density::None, Density::Full] {
        let beatmap = convert_to(&map(10), "10K2S", 12, density);
        check(&beatmap, 12);
        check_fingers(&beatmap, &layout, "10K -> 10K2S");
        check_scratch_hands(&beatmap, &original, "10K -> 10K2S");
        let notes = notes_of(&beatmap, 12);
        let inner: Vec<(f64, f64, usize)> =
            notes.iter().filter(|n| n.2 != 0 && n.2 != 11).map(|&(s, e, k)| (s, e, k - 1)).collect();
        let scratch: Vec<&(f64, f64, usize)> = notes.iter().filter(|n| n.2 == 0 || n.2 == 11).collect();
        // The notes of the map not on their key: ones a scratch of their
        // hand took, at their time.
        assert!(inner.iter().all(|n| original_notes.contains(n)));
        let mut taken = Vec::new();
        for &(start, _, key) in original_notes.iter().filter(|n| !inner.contains(n)) {
            let side = if key < 5 { 0 } else { 11 };
            let full = density == Density::Full;
            assert!(
                scratch.iter().any(|n| n.2 == side && n.0 == start),
                "full {}: the note at {} on key {} went without a scratch",
                full,
                start,
                key
            );
            taken.push((start, side));
        }
        // 25 bars at 120 BPM of rows on every 1/4, mostly on both hands,
        // and no audio.
        assert!(scratch.len() >= 10, "{} scratch notes", scratch.len());
        for &&(start, end, key) in &scratch {
            assert!(original_notes.iter().any(|n| n.0 == start), "a scratch at {}, where the map has no note", start);
            // A long one is one of the map's, moved out.
            let hand = if key == 0 { 0..5 } else { 5..10 };
            assert!(
                start == end || original_notes.iter().any(|n| n.0 == start && n.1 == end && hand.contains(&n.2)),
                "a long scratch at {} of its own",
                start
            );
            let pinky = if key == 0 { 1 } else { 10 };
            let gap = 500.0 * JACK_BEATS;
            assert!(
                notes.iter().filter(|n| n.2 == pinky).all(|&(s, e, _)| s - start > gap || start - e > gap),
                "a scratch at {} next to its pinky's key",
                start
            );
        }
        // Hand-made maps: 3% on the 1/4s and 2% elsewhere.
        let on_half = scratch.iter().filter(|n| (n.0 / 250.0 - (n.0 / 250.0).round()).abs() < 0.01).count();
        assert!(on_half * 10 >= scratch.len() * 9, "{} of {} on the 1/2s", on_half, scratch.len());
        // Those of their own, plain: the hit sound alone.
        for obj in beatmap.hit_objects.iter().filter(|o| matches!(column(o, 12), 0 | 11)) {
            if !taken.contains(&(obj.start_time, column(obj, 12))) {
                assert!(obj.samples.iter().all(|s| s.name == HitSampleInfo::HIT_NORMAL), "{:?}", obj.samples);
            }
        }
    }
    // From 7K, the inner keys hold the 7K map converted to 10K: every note
    // of it but the ones the scratches took, and the density's extra
    // notes but those in a scratch's way.
    let original = rosu_map::from_str::<Beatmap>(&map(7)).unwrap();
    let (_, ten) = convert(&map(7), 10, Density::Full);
    let beatmap = convert_to(&map(7), "10K2S", 12, Density::Full);
    check_scratch_hands(&beatmap, &ten, "7K -> 10K2S");
    let ten = notes_of(&ten, 10);
    let notes = notes_of(&beatmap, 12);
    let inner: Vec<(f64, f64, usize)> =
        notes.iter().filter(|n| n.2 != 0 && n.2 != 11).map(|&(s, e, k)| (s, e, k - 1)).collect();
    let scratched = notes.len() - inner.len();
    assert!(inner.iter().all(|n| ten.contains(n)));
    assert!(inner.len() + scratched >= original.hit_objects.len());
    assert!(inner.len() > ten.len() * 8 / 10, "{} of {}", inner.len(), ten.len());
}

#[test]
fn ten_k_two_s_marks_the_big_chords() {
    // A 10K stream on the index fingers with a chord on the second beat
    // of every bar, of the left pinky and the right middle finger and
    // pinky: the chords are the accents. Their left scratch takes the
    // pinky's note, sounds and all, the hand having nothing else there;
    // the right hand keeps its two notes.
    let (beat, bars) = (500.0, 16usize);
    let mut osu = String::from(
        "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
    );
    for q in 0..bars * 16 {
        let time = q as f64 * beat / 4.0;
        let (cols, sound): (&[usize], u8) = if q % 16 == 4 { (&[0, 7, 9], 2) } else { (&[3 + 3 * (q % 2)], 0) };
        for &col in cols {
            osu.push_str(&format!("{},192,{},1,{},0:0:0:0:\n", mania::column_x(col, 10) as i32, time, sound));
        }
    }
    let original = rosu_map::from_str::<Beatmap>(&osu).unwrap();
    let beatmap = convert_to(&osu, "10K2S", 12, Density::None);
    check(&beatmap, 12);
    check_fingers(&beatmap, &layout::layout(12, Some("10K2S")), "10K -> 10K2S");
    check_scratch_hands(&beatmap, &original, "10K -> 10K2S");
    let chords: Vec<f64> = (0..bars).map(|bar| (bar as f64 * 4.0 + 1.0) * beat).collect();
    let at = |time: f64, col: usize| beatmap.hit_objects.iter().find(|o| o.start_time == time && column(o, 12) == col);
    let mut scratched = 0;
    for &time in &chords {
        let Some(note) = at(time, 0) else { continue };
        scratched += 1;
        assert!((1..=5).all(|k| at(time, k).is_none()), "the scratch at {} has its hand's notes too", time);
        assert!([8, 10].iter().all(|&k| at(time, k).is_some()), "the chord at {} lost its right hand", time);
        assert!(note.samples.iter().any(|s| s.name == HitSampleInfo::HIT_WHISTLE), "{:?}", note.samples);
    }
    assert!(scratched * 10 >= chords.len() * 9, "{} of {} chords", scratched, chords.len());
    // The rest of the map is as it was: every note on its key, or at a
    // scratch of its hand.
    let scratch_at = |time: f64, side: usize| beatmap.hit_objects.iter().any(|o| o.start_time == time && column(o, 12) == side);
    for o in &original.hit_objects {
        let key = column(o, 10);
        let on_key = beatmap.hit_objects.iter().any(|n| n.start_time == o.start_time && column(n, 12) == key + 1);
        assert!(on_key || scratch_at(o.start_time, if key < 5 { 0 } else { 11 }), "the note at {} on key {} went", o.start_time, key);
    }
}

#[test]
fn ten_k_two_s_frees_the_hand_by_difficulty() {
    // 10K maps at 120 BPM, an easy one (a note a beat, the hands taking
    // turns) and a hard one (jumps on every 1/4), each with a chord on
    // every bar. No scratch has its hand's notes with it, and the easy
    // map's hand stays free for longer after it: past the other hand's
    // next note at least. The hard map's comes back on the next notes.
    let header = "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n";
    let note = |col: usize, time: f64| format!("{},192,{},1,0,0:0:0:0:\n", mania::column_x(col, 10) as i32, time);
    let mut easy = String::from(header);
    for b in 0..256usize {
        let cols: Vec<usize> = if b % 4 == 0 { vec![1, 3, 6, 8] } else if b % 2 == 0 { vec![1 + b % 3] } else { vec![6 + b % 3] };
        for col in cols {
            easy.push_str(&note(col, b as f64 * 500.0));
        }
    }
    let mut hard = String::from(header);
    for q in 0..512usize {
        let cols: Vec<usize> = if q % 16 == 0 { vec![0, 2, 7, 9] } else { vec![1 + q % 3, 8 - q % 3] };
        for col in cols {
            hard.push_str(&note(col, q as f64 * 125.0));
        }
    }
    let layout = layout::layout(12, Some("10K2S"));
    let mut soonest = Vec::new();
    for (name, osu) in [("easy", &easy), ("hard", &hard)] {
        let original = rosu_map::from_str::<Beatmap>(osu).unwrap();
        let beatmap = convert_to(osu, "10K2S", 12, Density::None);
        check(&beatmap, 12);
        check_fingers(&beatmap, &layout, name);
        let scratches = beatmap.hit_objects.iter().filter(|o| matches!(column(o, 12), 0 | 11)).count();
        assert!(scratches >= 10, "{}: {} scratch notes", name, scratches);
        soonest.push(check_scratch_hands(&beatmap, &original, name));
    }
    assert!(soonest[0] >= 500.0 - 1.0, "easy: the hand back {} ms after a scratch", soonest[0]);
    assert!(soonest[1] <= 250.0 + 1.0, "hard: the hand back {} ms after a scratch at the soonest", soonest[1]);
}

#[test]
fn ten_k_two_s_rests_by_difficulty() {
    // An easy 10K map and a hard one at 120 BPM (a note a beat, and rows
    // of one to three notes on every 1/4), with no density and with
    // Full: the easy map gets fewer scratches close together, and with
    // Full no fewer notes than without, its scratches added to the map's
    // notes rather than taking them.
    let header = "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n";
    let mut easy = String::from(header);
    for b in 0..256usize {
        let cols: Vec<usize> = if b % 4 == 0 { vec![1, 3, 6, 8] } else { vec![[2, 7, 1, 8, 3, 6][b % 6]] };
        for col in cols {
            easy.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", mania::column_x(col, 10) as i32, b as f64 * 500.0));
        }
    }
    let mut busiest = Vec::new();
    for (name, osu) in [("easy", easy), ("hard", map(10))] {
        let original = rosu_map::from_str::<Beatmap>(&osu).unwrap();
        let none = convert_to(&osu, "10K2S", 12, Density::None);
        let full = convert_to(&osu, "10K2S", 12, Density::Full);
        for beatmap in [&none, &full] {
            check(beatmap, 12);
        }
        assert!(full.hit_objects.len() >= none.hit_objects.len(), "{}: Full has fewer notes", name);
        let scratches = none.hit_objects.iter().filter(|o| matches!(column(o, 12), 0 | 11)).count();
        assert!(scratches >= 10, "{}: {} scratch notes", name, scratches);
        busiest.push(check_scratch_spans(&none, &original, 500.0, name).max(check_scratch_spans(&full, &original, 500.0, name)));
    }
    assert!(busiest[0] < busiest[1], "{:?}", busiest);
}

#[test]
fn ten_k_two_s_follows_the_sounds() {
    // A 10K map of a note a beat on the thumbs, over a quiet song with a
    // click on the second beat of every other bar: the scratches go on
    // the clicks.
    let (beat, bars) = (500.0, 32usize);
    let mut osu = String::from(
        "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
    );
    for b in 0..bars * 4 {
        osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", mania::column_x(4 + b % 2, 10) as i32, b as f64 * beat));
    }
    let clicks: Vec<f64> = (0..bars).step_by(2).map(|bar| (bar as f64 * 4.0 + 1.0) * beat).collect();
    let sample_rate = 44_100.0;
    let mut samples = vec![0.0f32; ((bars as f64 * 4.0 * beat / 1000.0 + 1.0) * sample_rate) as usize];
    let mut seed = 3u32;
    let mut noise = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / (1 << 24) as f32 - 0.5
    };
    for sample in samples.iter_mut() {
        *sample = noise() * 0.002;
    }
    for &click in &clicks {
        let first = (click / 1000.0 * sample_rate) as usize;
        for sample in &mut samples[first..first + (0.03 * sample_rate) as usize] {
            *sample = noise() * 0.5;
        }
    }
    let accents = crate::commands::trainer::accents::Accents::from_mono(&samples, sample_rate);
    let mut beatmap = rosu_map::from_str::<Beatmap>(&osu).unwrap();
    let params =
        KeyConverterParams { target_keys: 12, density: Density::None, keep_jacks: true, playstyle: Some("10K2S".to_string()) };
    apply_with(&mut beatmap, &params, Some(accents.at_rate(1.0))).unwrap();
    check_fingers(&beatmap, &layout::layout(12, Some("10K2S")), "10K -> 10K2S");
    let scratches: Vec<f64> =
        beatmap.hit_objects.iter().filter(|o| matches!(column(o, 12), 0 | 11)).map(|o| o.start_time).collect();
    let heard = clicks.iter().filter(|&&c| scratches.iter().any(|&t| (t - c).abs() < 1.0)).count();
    assert!(heard * 10 >= clicks.len() * 9, "{} of {} clicks: {:?}", heard, clicks.len(), scratches);
    // Without the audio they go on the first beats of the bars instead.
    let mut beatmap = rosu_map::from_str::<Beatmap>(&osu).unwrap();
    apply(&mut beatmap, &params).unwrap();
    let unheard: Vec<f64> =
        beatmap.hit_objects.iter().filter(|o| matches!(column(o, 12), 0 | 11)).map(|o| o.start_time).collect();
    let on_bars = unheard.iter().filter(|&&t| t % (4.0 * beat) == 0.0).count();
    assert!(!unheard.is_empty() && on_bars * 10 >= unheard.len() * 8, "{} of {} on the bars", on_bars, unheard.len());
}

#[test]
fn ten_k_two_s_marks_the_strong_sounds() {
    // A 10K map of a note a beat on the thumbs over a song of a click
    // every 1/2 beat, quiet for 16 bars and loud for 16: the scratches
    // mark the loud ones alone, as many as the map asks for or not.
    let (beat, bars) = (500.0, 32usize);
    let mut osu = String::from(
        "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
    );
    for b in 0..bars * 4 {
        osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", mania::column_x(4 + b % 2, 10) as i32, b as f64 * beat));
    }
    let sample_rate = 44_100.0;
    let mut samples = vec![0.0f32; ((bars as f64 * 4.0 * beat / 1000.0 + 1.0) * sample_rate) as usize];
    let mut seed = 9u32;
    let mut noise = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / (1 << 24) as f32 - 0.5
    };
    for sample in samples.iter_mut() {
        *sample = noise() * 0.002;
    }
    for half in 0..bars * 8 {
        let first = (half as f64 * beat / 2000.0 * sample_rate) as usize;
        let volume = if half < bars * 4 { 0.03 } else { 0.5 };
        for sample in &mut samples[first..first + (0.03 * sample_rate) as usize] {
            *sample = noise() * volume;
        }
    }
    let accents = crate::commands::trainer::accents::Accents::from_mono(&samples, sample_rate);
    let mut beatmap = rosu_map::from_str::<Beatmap>(&osu).unwrap();
    let params =
        KeyConverterParams { target_keys: 12, density: Density::Full, keep_jacks: true, playstyle: Some("10K2S".to_string()) };
    apply_with(&mut beatmap, &params, Some(accents.at_rate(1.0))).unwrap();
    check_fingers(&beatmap, &layout::layout(12, Some("10K2S")), "10K -> 10K2S");
    let scratches: Vec<f64> =
        beatmap.hit_objects.iter().filter(|o| matches!(column(o, 12), 0 | 11)).map(|o| o.start_time).collect();
    let loud = scratches.iter().filter(|&&t| t >= bars as f64 * 2.0 * beat - 1.0).count();
    assert!(scratches.len() >= 10 && loud == scratches.len(), "{} of {} in the loud half: {:?}", loud, scratches.len(), scratches);
    // On the map's notes, the beats, not on the clicks between them.
    assert!(scratches.iter().all(|&t| (t / beat).fract() == 0.0), "{:?}", scratches);
}

#[test]
fn ten_k_two_s_moves_the_accent_sounds_out() {
    // A 10K map of a note every 1/2 beat on the right hand, with a water
    // drop of its own on the left middle finger's key 1/2 beat into every
    // 8th bar, and a hi-hat on every beat of a bar: the water drops go to
    // the left scratch, sound and all, and the hi-hats, a part to play,
    // stay where they are.
    let (beat, bars) = (500.0, 32usize);
    let mut osu = String::from(
        "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
    );
    let mut drops = Vec::new();
    for half in 0..bars * 8 {
        let time = half as f64 * beat / 2.0;
        let (col, sample) = if half % 64 == 1 {
            drops.push(time);
            (2, "water-drop.wav")
        } else if half / 8 == 21 && half % 2 == 0 {
            (7, "hihat.wav")
        } else {
            ([6, 8][half % 2], "")
        };
        osu.push_str(&format!("{},192,{},1,0,0:0:0:0:{}\n", mania::column_x(col, 10) as i32, time, sample));
    }
    let beatmap = convert_to(&osu, "10K2S", 12, Density::None);
    check(&beatmap, 12);
    let at = |time: f64, col: usize| beatmap.hit_objects.iter().find(|o| o.start_time == time && column(o, 12) == col);
    let sample = |o: &HitObject, name: &str| o.samples.iter().any(|s| format!("{:?}", s.name).contains(name));
    let moved = drops.iter().filter(|&&t| at(t, 0).is_some_and(|o| sample(o, "water-drop"))).count();
    assert!(moved * 10 >= drops.len() * 8, "{} of {} water drops on the scratch", moved, drops.len());
    let hats = beatmap.hit_objects.iter().filter(|o| sample(o, "hihat")).collect::<Vec<_>>();
    assert!(hats.len() == 4 && hats.iter().all(|o| column(o, 12) == 8), "{:?}", hats.iter().map(|o| column(o, 12)).collect::<Vec<_>>());
}

#[test]
fn seven_k_plus_one_is_the_7k_map_and_a_scratch() {
    // From 7K: every note of the map as it was, a key to the right, and
    // the scratch's own notes on the first key: plain, on rows of the map,
    // never while the left hand (the next three keys) holds a long note,
    // and about as few as hand-made 7K+1 maps have (3% of their notes).
    let original = rosu_map::from_str::<Beatmap>(&map(7)).unwrap();
    let beatmap = convert_to(&map(7), "7K+1", 8, Density::None);
    let seven = notes_of(&original, 7);
    let notes = notes_of(&beatmap, 8);
    let mut inner: Vec<(f64, f64, usize)> = notes.iter().filter(|n| n.2 != 0).map(|&(s, e, k)| (s, e, k - 1)).collect();
    let mut expected = seven.clone();
    inner.sort_by(|a, b| a.partial_cmp(b).unwrap());
    expected.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(inner, expected);

    let scratches: Vec<(f64, f64, usize)> = notes.iter().copied().filter(|n| n.2 == 0).collect();
    assert!(!scratches.is_empty());
    assert!(scratches.len() * 10 < seven.len(), "{} scratches for {} notes", scratches.len(), seven.len());
    for &(start, end, _) in &scratches {
        assert!(end - start < 1.0, "a long scratch at {}", start);
        assert!(seven.iter().any(|n| (n.0 - start).abs() < 1.0), "a scratch at {} off the map's rows", start);
        assert!(
            !seven.iter().any(|n| n.2 <= 2 && n.0 < start - 1.0 && n.1 > start + 1.0),
            "a scratch at {} while the left hand holds a note",
            start
        );
    }
    for obj in beatmap.hit_objects.iter().filter(|o| column(o, 8) == 0) {
        assert!(obj.samples.iter().all(|s| s.name == HitSampleInfo::HIT_NORMAL), "{:?}", obj.samples);
    }
}

#[test]
fn seven_k_plus_one_from_4k_converts_to_7k_first() {
    // The inner keys hold the 4K map converted to 7K, as it would be alone.
    let (_, seven) = convert(&map(4), 7, Density::None);
    let beatmap = convert_to(&map(4), "7K+1", 8, Density::None);
    let mut inner: Vec<(f64, f64, usize)> =
        notes_of(&beatmap, 8).into_iter().filter(|n| n.2 != 0).map(|(s, e, k)| (s, e, k - 1)).collect();
    let mut expected = notes_of(&seven, 7);
    inner.sort_by(|a, b| a.partial_cmp(b).unwrap());
    expected.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(inner, expected);
    assert!(notes_of(&beatmap, 8).iter().any(|n| n.2 == 0), "no scratch");
}
