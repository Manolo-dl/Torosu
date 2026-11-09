//! Tests of the key converter, and the maps and checks they share.

use super::*;

mod key_counts;
mod near;
mod scratches;
mod source_patterns;

/// A map at 120 BPM with streams, jacks, chords and LNs, generated
/// deterministically.
fn map(keys: usize) -> String {
    let mut osu = format!(
        "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:{}\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
        keys
    );
    let mut seed = 7u64;
    let mut busy = vec![-1i64; keys];
    for step in 0..400i64 {
        let time = step * 125;
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let chord = 1 + (seed >> 60) as usize % 3;
        for k in 0..chord {
            let col = ((seed >> (8 * k + 8)) as usize) % keys;
            if busy[col] >= time {
                continue;
            }
            let x = mania::column_x(col, keys) as i32;
            if (seed >> 40) % 5 == 0 {
                let end = time + 375;
                busy[col] = end;
                osu.push_str(&format!("{},192,{},128,0,{}:0:0:0:0:\n", x, time, end));
            } else {
                busy[col] = time;
                osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", x, time));
            }
        }
    }
    osu
}

fn stream(cols: &[usize]) -> String {
    let mut osu = String::from(
        "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:4\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
    );
    for (step, col) in cols.iter().enumerate() {
        osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", col * 128 + 64, step * 125));
    }
    osu
}

/// Columns of a converted map, one per note.
fn columns(beatmap: &Beatmap, keys: usize) -> Vec<usize> {
    beatmap.hit_objects.iter().map(|o| column(o, keys)).collect()
}

/// A 4K map from rows of columns, in 1/4s at 120 BPM.
fn rows_map(rows: &[&[usize]]) -> String {
    let mut osu = String::from(
        "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:4\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
    );
    for (step, row) in rows.iter().enumerate() {
        for col in row.iter() {
            osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", col * 128 + 64, step * 125));
        }
    }
    osu
}

/// The keys of each row of a converted map.
fn key_rows(beatmap: &Beatmap, keys: usize) -> Vec<Vec<usize>> {
    let mut rows: Vec<(f64, Vec<usize>)> = Vec::new();
    for obj in &beatmap.hit_objects {
        match rows.last_mut() {
            Some((time, cols)) if *time == obj.start_time => cols.push(column(obj, keys)),
            _ => rows.push((obj.start_time, vec![column(obj, keys)])),
        }
    }
    rows.into_iter().map(|(_, mut cols)| {
        cols.sort();
        cols
    }).collect()
}

/// A 4K stair, 0123 0123..., in 1/4s at 120 BPM.
fn stair() -> String {
    let mut osu = String::from(
        "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:4\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
    );
    for step in 0..64 {
        osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", (step % 4) * 128 + 64, step * 125));
    }
    osu
}

fn column(obj: &HitObject, keys: usize) -> usize {
    let (x, _) = mania::note(obj).expect("only notes");
    mania::column(x, keys)
}

/// Checks no two notes overlap or stack in a key, and returns how many
/// jacks (1/4 apart or closer) there are.
fn check(beatmap: &Beatmap, keys: usize) -> usize {
    let mut release = vec![f64::NEG_INFINITY; keys];
    let mut jacks = 0;
    for obj in &beatmap.hit_objects {
        let (x, end) = mania::note(obj).expect("only notes");
        let col = mania::column(x, keys);
        assert!(obj.start_time > release[col], "overlap in key {} at {}", col, obj.start_time);
        if obj.start_time - release[col] <= 500.0 * JACK_BEATS {
            jacks += 1;
        }
        release[col] = end;
    }
    jacks
}

fn convert(osu: &str, dst: usize, density: Density) -> (Beatmap, Beatmap) {
    let original = rosu_map::from_str::<Beatmap>(osu).unwrap();
    let mut beatmap = original.clone();
    let params = KeyConverterParams { target_keys: dst as u8, density, keep_jacks: true, playstyle: None };
    apply(&mut beatmap, &params).unwrap();
    (original, beatmap)
}

/// Share of the chords with two next keys on one hand of a 10K map.
fn pill_rate(beatmap: &Beatmap) -> f64 {
    let rows = key_rows(beatmap, 10);
    let chords: Vec<&Vec<usize>> = rows.iter().filter(|r| r.len() > 1).collect();
    let pills = chords.iter().filter(|r| r.windows(2).any(|w| w[1] == w[0] + 1 && (w[0] < 5) == (w[1] < 5))).count();
    pills as f64 / chords.len().max(1) as f64
}

fn convert_to(osu: &str, playstyle: &str, dst: usize, density: Density) -> Beatmap {
    let mut beatmap = rosu_map::from_str::<Beatmap>(osu).unwrap();
    let params = KeyConverterParams { target_keys: dst as u8, density, keep_jacks: true, playstyle: Some(playstyle.to_string()) };
    apply(&mut beatmap, &params).unwrap();
    beatmap
}

/// Notes a finger plays on another of its keys within a jack's time.
fn finger_jacks(beatmap: &Beatmap, layout: &Layout) -> usize {
    let keys = layout.keys();
    let shared = shared_fingers(layout);
    let mut release = vec![f64::NEG_INFINITY; keys];
    let mut jacks = 0;
    for obj in &beatmap.hit_objects {
        let (x, end) = mania::note(obj).expect("only notes");
        let col = mania::column(x, keys);
        let gap = |k: u8| obj.start_time - release[k as usize];
        if keys_of(shared[col] & !(1 << col)).any(|k| gap(k) > 0.0 && gap(k) <= 500.0 * JACK_BEATS) {
            jacks += 1;
        }
        release[col] = end;
    }
    jacks
}

/// Checks no finger plays two of its keys at once (see [`shared_fingers`]):
/// in one row, or while it holds a long note on the other.
fn check_fingers(beatmap: &Beatmap, layout: &Layout, what: &str) {
    let keys = layout.keys();
    let shared = shared_fingers(layout);
    let mut release = vec![f64::NEG_INFINITY; keys];
    for obj in &beatmap.hit_objects {
        let (x, end) = mania::note(obj).expect("only notes");
        let col = mania::column(x, keys);
        for k in keys_of(shared[col] & !(1 << col)) {
            assert!(obj.start_time > release[k as usize], "{}: keys {} and {} at {}", what, k, col, obj.start_time);
        }
        release[col] = end;
    }
}

/// A 7K full LN map at 120 BPM: rows of one or two notes in 1/4s, each
/// note held until 1/4 beat before the next of its column, as FLN maps do.
fn fln_map() -> String {
    let keys = 7;
    let mut seed = 11u64;
    let mut notes: Vec<(i64, usize)> = Vec::new();
    for step in 0..400i64 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        for k in 0..1 + (seed >> 62) as usize % 2 {
            let col = ((seed >> (8 * k + 8)) as usize) % keys;
            if !notes.contains(&(step * 125, col)) {
                notes.push((step * 125, col));
            }
        }
    }
    let mut osu = String::from(
        "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:7\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n\n[HitObjects]\n",
    );
    for &(time, col) in &notes {
        let x = mania::column_x(col, keys) as i32;
        match notes.iter().filter(|&&(t, c)| c == col && t > time).map(|&(t, _)| t).min() {
            Some(next) if next - 125 > time => osu.push_str(&format!("{},192,{},128,0,{}:0:0:0:0:\n", x, time, next - 125)),
            _ => osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", x, time)),
        }
    }
    osu
}

/// How many notes each scratch of a 12K map and the key next to it get
/// against an even key, and the scratch's longest run of notes a beat
/// apart or closer (at 120 BPM).
fn scratch_use(beatmap: &Beatmap) -> (f64, f64, usize) {
    let cols = columns(beatmap, 12);
    let share = |a, b| cols.iter().filter(|&&c| c == a || c == b).count() as f64 / 2.0 / cols.len() as f64 * 12.0;
    let (scratch, pinky) = (share(0, 11), share(1, 10));
    let mut longest = 0;
    for s in [0, 11] {
        let times: Vec<f64> = beatmap.hit_objects.iter().filter(|o| column(o, 12) == s).map(|o| o.start_time).collect();
        let mut run = 0;
        for (i, t) in times.iter().enumerate() {
            run = if i > 0 && t - times[i - 1] <= 505.0 { run + 1 } else { 1 };
            longest = longest.max(run);
        }
    }
    (scratch, pinky, longest)
}

/// The notes of a map as (start, end, key), in time.
fn notes_of(beatmap: &Beatmap, keys: usize) -> Vec<(f64, f64, usize)> {
    beatmap.hit_objects.iter().map(|o| (o.start_time, mania::note(o).unwrap().1, column(o, keys))).collect()
}

fn notes_per_second(beatmap: &Beatmap) -> f64 {
    let first = beatmap.hit_objects.first().map_or(0.0, |o| o.start_time);
    let last = beatmap.hit_objects.last().map_or(0.0, |o| o.start_time);
    beatmap.hit_objects.len() as f64 / ((last - first) / 1000.0).max(1.0)
}

/// Checks that no scratch of a 10K2S map has a note of its hand (the five
/// keys next to it) in its row or held across it, a muri, nor closer than
/// [`SCRATCH_FREE_MS`] lets it for a map as hard as `inner`, the 10K map
/// under the scratches. Returns how soon after a scratch its hand plays
/// again at the soonest, in ms.
fn check_scratch_hands(beatmap: &Beatmap, inner: &Beatmap, what: &str) -> f64 {
    let free_ms = scratch_free_ms(notes_per_second(inner));
    let (after, before) = (free_ms * SCRATCH_FREE_AFTER[0].0, free_ms * SCRATCH_FREE_BEFORE[0].0);
    let notes = notes_of(beatmap, 12);
    let mut soonest = f64::INFINITY;
    for &(time, release, scratch) in notes.iter().filter(|n| matches!(n.2, 0 | 11)) {
        let hand = if scratch == 0 { 1..=5 } else { 6..=10 };
        for &(start, end, key) in notes.iter().filter(|n| hand.contains(&n.2)) {
            assert!((start - time).abs() > 1.0, "{}: a muri, the scratch at {} with key {}", what, time, key);
            assert!(!(start < time && end > time), "{}: a muri, the scratch at {} while key {} holds", what, time, key);
            assert!(!(start > time && start < release), "{}: key {} while the scratch at {} holds", what, key, time);
            if start > time {
                assert!(start - time >= after, "{}: key {} {} ms after the scratch at {}", what, key, start - time, time);
                soonest = soonest.min(start - time);
            } else {
                assert!(time - end >= before, "{}: key {} {} ms before the scratch at {}", what, key, time - end, time);
            }
        }
    }
    soonest
}

/// Checks that no span of a 10K2S map has more scratches than
/// [`SCRATCH_SPANS`] lets it for a map as hard as `inner`, the 10K map
/// under the scratches. Returns the most there are in any 8 beats.
fn check_scratch_spans(beatmap: &Beatmap, inner: &Beatmap, beat_len: f64, what: &str) -> usize {
    let nps = notes_per_second(inner);
    let mut times: Vec<f64> =
        beatmap.hit_objects.iter().filter(|o| matches!(column(o, 12), 0 | 11)).map(|o| o.start_time).collect();
    times.dedup();
    let busiest = |beats: f64| {
        (0..times.len()).map(|i| times[i..].iter().take_while(|&&t| t - times[i] <= beats * beat_len + 1.0).count()).max()
    };
    for (beats, a, b) in SCRATCH_SPANS {
        let most = scratch_most(nps, a, b);
        let found = busiest(beats).unwrap_or(0);
        assert!(found <= most, "{}: {} scratches in {} beats, {} at most at {:.1} notes a second", what, found, beats, most, nps);
    }
    busiest(8.0).unwrap_or(0)
}
