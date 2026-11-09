//! Targets with few new keys (10K -> 12K, 7K -> 8K): the source's own
//! patterns stay as they are.

use super::*;

#[test]
fn near_key_counts_keep_the_source_patterns() {
    // 10K -> 12K has 2 new keys: a 10K column coming back two notes later
    // stays on its key, as its mapper meant it, instead of rolling over
    // the keys next to it. Two columns take turns in 1/2s at 174 BPM
    // (osu! rounds them to 172 or 173 ms), with a chord on every beat
    // pair, so they are no trill to copy.
    let beat = 60000.0 / 174.0;
    let mut osu = format!(
        "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,{},4,1,0,60,1,0\n\n[HitObjects]\n",
        beat
    );
    let mut odd = Vec::new();
    for step in 0..48 {
        let time = (step as f64 * beat / 2.0).round();
        let cols: &[usize] = match step % 4 {
            0 => &[1, 8],
            1 | 3 => &[4],
            _ => &[1],
        };
        if step % 2 == 1 {
            odd.push(time);
        }
        for &col in cols {
            osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", mania::column_x(col, 10) as i32, time));
        }
    }
    for density in [Density::None, Density::Full] {
        let (_, beatmap) = convert(&osu, 12, density);
        // The keys column 4 went to: mostly one.
        let mut keys = [0usize; 12];
        for obj in beatmap.hit_objects.iter().filter(|o| odd.contains(&o.start_time)) {
            keys[column(obj, 12)] += 1;
        }
        let most = keys.iter().max().copied().unwrap_or(0);
        assert!(most * 10 >= odd.len() * 8, "{:?}", keys);
    }
}

#[test]
fn near_key_counts_keep_columns_by_the_scratch() {
    // 10K -> 12K: a column by the edge comes back every beat, taking turns
    // with another, with a chord now and then. Its notes stay on one key
    // rather than going to the scratch for a few and then to the keys
    // around it as the scratch's costs and the pinky's balance change.
    let beat = 60000.0 / 174.0;
    let mut osu = format!(
        "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,{},4,1,0,60,1,0\n\n[HitObjects]\n",
        beat
    );
    let mut edge = Vec::new();
    for step in 0..48 {
        let time = (step as f64 * beat / 2.0).round();
        let cols: &[usize] = match step % 8 {
            1 | 3 => &[0, 8],
            s if s % 2 == 1 => &[8],
            _ => &[6],
        };
        if step % 2 == 1 {
            edge.push(time);
        }
        for &col in cols {
            osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", mania::column_x(col, 10) as i32, time));
        }
    }
    for density in [Density::None, Density::Full] {
        let (_, beatmap) = convert(&osu, 12, density);
        let mut keys = [0usize; 12];
        for obj in beatmap.hit_objects.iter().filter(|o| edge.contains(&o.start_time)) {
            let key = column(obj, 12);
            // The chords' other note goes to the left; this is the right.
            if key >= 6 {
                keys[key] += 1;
            }
        }
        let most = keys.iter().max().copied().unwrap_or(0);
        assert!(most * 10 >= edge.len() * 8, "{:?}", keys);
    }
}

#[test]
fn near_key_counts_add_notes_around_the_map() {
    // 10K -> 12K: the denser fills place the map's notes where the plain
    // conversion does, so its streams keep their direction, and add their
    // notes around them, still playable, not next to a note of their row
    // where the source has none next to each other, and not within a
    // jack of the map's (1/2 beat) of a note on their key: no jack of two
    // turns into three, no long note gets a shield.
    let (original, plain) = convert(&map(10), 12, Density::None);
    let adjacent = |r: &[usize]| r.windows(2).any(|w| w[1] == w[0] + 1);
    let source = key_rows(&original, 10);
    let rows = key_rows(&plain, 12);
    let times: Vec<f64> = {
        let mut t: Vec<f64> = plain.hit_objects.iter().map(|o| o.start_time).collect();
        t.dedup();
        t
    };
    for density in [Density::Dense, Density::MoreDense, Density::Full] {
        let (_, beatmap) = convert(&map(10), 12, density);
        assert!(beatmap.hit_objects.len() > plain.hit_objects.len());
        check(&beatmap, 12);
        check_fingers(&beatmap, &layout::layout(12, None), "10K -> 12K");
        for ((time, row), src) in times.iter().zip(&rows).zip(&source) {
            let mut keys: Vec<usize> =
                beatmap.hit_objects.iter().filter(|o| o.start_time == *time).map(|o| column(o, 12)).collect();
            keys.sort();
            assert!(row.iter().all(|k| keys.contains(k)), "at {}: {:?} then {:?}", time, row, keys);
            if keys.len() > row.len() && !adjacent(src) && !adjacent(row) {
                assert!(!adjacent(&keys), "at {}: {:?} from {:?}", time, keys, src);
            }
        }
        let placed: Vec<(f64, f64, usize)> = plain
            .hit_objects
            .iter()
            .map(|o| (o.start_time, mania::note(o).unwrap().1, column(o, 12)))
            .collect();
        let gap = 500.0 * SOURCE_JACK_BEATS;
        for obj in &beatmap.hit_objects {
            let (start, end, key) = (obj.start_time, mania::note(obj).unwrap().1, column(obj, 12));
            if placed.contains(&(start, end, key)) {
                continue;
            }
            let near = placed.iter().any(|&(s, e, k)| k == key && ((0.0..=gap).contains(&(start - e)) || (0.0..=gap).contains(&(s - end))));
            assert!(!near, "an extra note at {} in key {} next to a note of the map", start, key);
        }
    }
}

#[test]
fn near_key_counts_keep_trills_of_single_notes() {
    // A 10K trill of single notes on 12K: Full (x1.08) keeps it single
    // notes instead of drawing a jumptrill for the whole of it, and the
    // denser fills add their notes open, not as pills next to the trill.
    let beat = 60000.0 / 174.0;
    let mut osu = format!(
        "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:10\n\n[TimingPoints]\n0,{},4,1,0,60,1,0\n\n[HitObjects]\n",
        beat
    );
    for (trill, cols) in [(0, [5, 8]), (1, [6, 9]), (2, [1, 3]), (3, [2, 7])] {
        for step in 0..12 {
            let time = ((trill * 16 + step) as f64 * beat / 2.0).round();
            osu.push_str(&format!("{},192,{},1,0,0:0:0:0:\n", mania::column_x(cols[step % 2], 10) as i32, time));
        }
    }
    let (_, beatmap) = convert(&osu, 12, Density::Full);
    let rows = key_rows(&beatmap, 12);
    assert!(rows.iter().all(|r| r.len() == 1), "{:?}", rows);
    for density in [Density::Dense, Density::MoreDense] {
        let (_, beatmap) = convert(&osu, 12, density);
        let rows = key_rows(&beatmap, 12);
        let pills = rows.iter().filter(|r| r.windows(2).any(|w| w[1] == w[0] + 1 && (w[0] < 6) == (w[1] < 6))).count();
        assert!(pills * 10 <= rows.len(), "{} pills: {:?}", pills, rows);
    }
}
