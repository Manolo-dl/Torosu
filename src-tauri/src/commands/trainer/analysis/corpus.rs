//! Pattern statistics over a whole osu! Songs folder, run by hand:
//!
//! ```sh
//! OSU_SONGS=~/.local/share/osu-wine/osu!/Songs OUT=/tmp/patterns \
//!     cargo test --release --lib corpus -- --ignored --nocapture
//! ```
//!
//! - `corpus_patterns`: how each key count is mapped, pattern by pattern:
//!   chord sizes, how the notes split between the hands, the most common
//!   rows, and how many keys a row shares with the one before it and the one
//!   two rows back.
//! - `corpus_pairs`: for the test sets with diffs of both key counts of
//!   `PAIR` (`4,7` by default), what the patterns of the source become in the
//!   hand-made target (rows at the same time) and in the key converter's
//!   output, and how dense each zone of the source gets.
//! - `corpus_feel`: hand-made 7K and 10K maps next to 7K -> 10K converts, on
//!   what players notice: pills, hands taking turns or playing their own
//!   patterns, the outer keys. `CONVERTS_ONLY=1` skips the hand-made maps.
//! - `corpus_compare`: a passage of `MAP` next to its 10K converts, as text.
//! - `corpus_columns`: how often hand-made 9K and 10K maps use each column,
//!   the outer ones above all, next to 4K and 7K converts to them.

use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    sync::{Mutex, atomic::{AtomicUsize, Ordering}},
};

use rosu_map::Beatmap;
use serde::Serialize;

use super::{
    layout::{Layout, layout},
    patterns::{Labelled, Pattern, label, link_ms, modifier},
    zones::Zone,
};
use crate::commands::trainer::{
    mania::key_count,
    model::{Density, KeyConverterParams},
    tasks::key_converter,
};

/// How many of the most common rows and splits are listed per pattern.
const TOP: usize = 15;
/// Rows of two diffs closer than this (in ms) are the same row.
const ALIGN_MS: f64 = 3.0;

fn songs_dir() -> PathBuf {
    std::env::var_os("OSU_SONGS").map(PathBuf::from).unwrap_or_else(|| {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        home.join(".local/share/osu-wine/osu!/Songs")
    })
}

fn out_dir() -> PathBuf {
    let dir = std::env::var_os("OUT").map(PathBuf::from).unwrap_or_else(|| std::env::temp_dir().join("patterns"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The mania diffs of each set folder.
fn mania_sets(songs: &Path) -> Vec<Vec<PathBuf>> {
    let mut sets = Vec::new();
    for entry in std::fs::read_dir(songs).into_iter().flatten().flatten() {
        let files: Vec<PathBuf> = std::fs::read_dir(entry.path())
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("osu")))
            .filter(|p| is_mania(p))
            .collect();
        if !files.is_empty() {
            sets.push(files);
        }
    }
    sets
}

fn is_mania(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else { return false };
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]);
    head.lines().any(|l| {
        let l = l.trim();
        l.strip_prefix("Mode:").is_some_and(|m| m.trim() == "3")
    })
}

fn parse(path: &Path) -> Option<Beatmap> {
    let beatmap = rosu_map::from_path::<Beatmap>(path).ok()?;
    (beatmap.hit_objects.len() >= 50 && !is_edit(&beatmap.version)).then_some(beatmap)
}

/// Diffs made from another diff of the set: rate edits (`1.1x`, `x1.2`,
/// `[0,9x Rate]`) and the trainer's own output (`(7K Auto)`, `(trainer)`),
/// which would mix what the converter already does into the statistics.
fn is_edit(version: &str) -> bool {
    let b = version.as_bytes();
    let rate = (0..b.len()).any(|i| {
        let x = b[i] == b'x' || b[i] == b'X';
        let digit_before = i > 0 && b[i - 1].is_ascii_digit();
        let digit_after = b.get(i + 1).is_some_and(|c| c.is_ascii_digit()) && (i == 0 || !b[i - 1].is_ascii_alphabetic());
        x && (digit_before || digit_after)
    });
    let keys = (0..b.len()).filter(|&i| b[i] == b'(').any(|i| {
        let digits = b[i + 1..].iter().take_while(|c| c.is_ascii_digit()).count();
        digits > 0 && b.get(i + 1 + digits) == Some(&b'K') && matches!(b.get(i + 2 + digits), Some(b' ' | b')' | b','))
    });
    rate || keys || version.contains("(trainer)") || version.contains("(SV normalized)")
}

/// Runs `f` on every item, on every core.
fn parallel<T: Sync, A: Send + Default>(items: &[T], f: impl Fn(&mut A, &T) + Sync, merge: impl Fn(&mut A, A)) -> A {
    let next = AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let done = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let mut acc = A::default();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(item) = items.get(i) else { break };
                    f(&mut acc, item);
                }
                done.lock().unwrap().push(acc);
            });
        }
    });
    let mut total = A::default();
    for acc in done.into_inner().unwrap() {
        merge(&mut total, acc);
    }
    total
}

fn add_counts<K: Ord>(a: &mut BTreeMap<K, u64>, b: BTreeMap<K, u64>) {
    for (k, v) in b {
        *a.entry(k).or_insert(0) += v;
    }
}

/// What the rows of one pattern look like.
#[derive(Default)]
struct PatternAgg {
    rows: u64,
    notes: u64,
    sizes: BTreeMap<u32, u64>,
    /// Notes per hand, left to right (`2-0-1`).
    splits: BTreeMap<String, u64>,
    shapes: BTreeMap<u32, u64>,
    /// Keys shared with the previous row / the row two back, in a phrase.
    overlap1: BTreeMap<u32, u64>,
    overlap2: BTreeMap<u32, u64>,
    modifiers: BTreeMap<&'static str, u64>,
    /// When comparing diffs: the pattern of the same row in the other diff.
    targets: BTreeMap<Pattern, u64>,
}

impl PatternAgg {
    fn merge(&mut self, b: PatternAgg) {
        self.rows += b.rows;
        self.notes += b.notes;
        add_counts(&mut self.sizes, b.sizes);
        add_counts(&mut self.splits, b.splits);
        add_counts(&mut self.shapes, b.shapes);
        add_counts(&mut self.overlap1, b.overlap1);
        add_counts(&mut self.overlap2, b.overlap2);
        add_counts(&mut self.modifiers, b.modifiers);
        add_counts(&mut self.targets, b.targets);
    }

    /// Adds row `i` of a labelled map.
    fn add(&mut self, l: &Labelled, i: usize, groups: &[u32]) {
        let row = l.rows[i];
        self.rows += 1;
        self.notes += row.mask.count_ones() as u64;
        *self.sizes.entry(row.mask.count_ones()).or_insert(0) += 1;
        let split = groups.iter().map(|g| (row.mask & g).count_ones().to_string()).collect::<Vec<_>>().join("-");
        *self.splits.entry(split).or_insert(0) += 1;
        *self.shapes.entry(row.mask).or_insert(0) += 1;

        let linked = |a: usize, b: usize| l.rows[b].time - l.rows[a].time <= link_ms(l.rows[b].beat_len);
        if i >= 1 && linked(i - 1, i) {
            *self.overlap1.entry((l.rows[i - 1].mask & row.mask).count_ones()).or_insert(0) += 1;
            if i >= 2 && linked(i - 2, i - 1) {
                *self.overlap2.entry((l.rows[i - 2].mask & row.mask).count_ones()).or_insert(0) += 1;
            }
        }
        for (bit, name) in modifier::NAMES {
            if l.labels[i].modifiers & bit != 0 {
                *self.modifiers.entry(name).or_insert(0) += 1;
            }
        }
    }
}

/// Everything about one key count.
#[derive(Default)]
struct KeyAgg {
    maps: u64,
    rows: u64,
    notes: u64,
    patterns: BTreeMap<Pattern, PatternAgg>,
    /// Per hand: rows and notes of each pattern.
    groups: BTreeMap<String, BTreeMap<Pattern, (u64, u64)>>,
}

impl KeyAgg {
    fn merge(&mut self, b: KeyAgg) {
        self.maps += b.maps;
        self.rows += b.rows;
        self.notes += b.notes;
        for (p, agg) in b.patterns {
            self.patterns.entry(p).or_default().merge(agg);
        }
        for (g, counts) in b.groups {
            let mine = self.groups.entry(g).or_default();
            for (p, (r, n)) in counts {
                let e = mine.entry(p).or_default();
                e.0 += r;
                e.1 += n;
            }
        }
    }

    fn add(&mut self, l: &Labelled, layout: &Layout) {
        let groups: Vec<u32> = layout.groups().iter().map(|g| g.mask).collect();
        self.maps += 1;
        self.rows += l.rows.len() as u64;
        for (i, label) in l.labels.iter().enumerate() {
            self.notes += l.rows[i].mask.count_ones() as u64;
            self.patterns.entry(label.pattern).or_default().add(l, i, &groups);
        }
        for g in &l.groups {
            let counts = self.groups.entry(g.group.name.clone()).or_default();
            for (&i, label) in g.rows.iter().zip(&g.labels) {
                let e = counts.entry(label.pattern).or_default();
                e.0 += 1;
                e.1 += (l.rows[i].mask & g.group.mask).count_ones() as u64;
            }
        }
    }
}

/// A row as text, with `|` between the hands: `O.O|.|..O`.
fn shape(mask: u32, layout: &Layout) -> String {
    let groups = layout.groups();
    let group_of = |k: usize| groups.iter().position(|g| g.mask & 1 << k != 0);
    let mut s = String::new();
    for k in 0..layout.keys() {
        if k > 0 && group_of(k) != group_of(k - 1) {
            s.push('|');
        }
        s.push(if mask & 1 << k != 0 { 'O' } else { '.' });
    }
    s
}

fn top<K: Clone>(counts: &BTreeMap<K, u64>, n: usize) -> Vec<(K, u64)> {
    let mut v: Vec<(K, u64)> = counts.iter().map(|(k, &c)| (k.clone(), c)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    v.truncate(n);
    v
}

fn dist<K: Clone + ToString>(counts: &BTreeMap<K, u64>, n: usize) -> Vec<(String, f64)> {
    let total: u64 = counts.values().sum();
    top(counts, n).into_iter().map(|(k, c)| (k.to_string(), c as f64 / total.max(1) as f64)).collect()
}

fn mean(counts: &BTreeMap<u32, u64>) -> f64 {
    let total: u64 = counts.values().sum();
    counts.iter().fold(0.0, |acc, (&k, &c)| acc + k as f64 * c as f64) / total.max(1) as f64
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PatternReport {
    pattern: Pattern,
    rows: u64,
    /// Share of the notes.
    share: f64,
    mean_size: f64,
    sizes: Vec<(String, f64)>,
    splits: Vec<(String, f64)>,
    shapes: Vec<(String, f64)>,
    shared_with_previous: f64,
    shared_with_two_back: f64,
    modifiers: Vec<(String, f64)>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    targets: Vec<(String, f64)>,
}

fn pattern_report(pattern: Pattern, agg: &PatternAgg, total_notes: u64, layout: &Layout) -> PatternReport {
    let shapes: BTreeMap<String, u64> = agg.shapes.iter().map(|(&m, &c)| (shape(m, layout), c)).collect();
    PatternReport {
        pattern,
        rows: agg.rows,
        share: agg.notes as f64 / total_notes.max(1) as f64,
        mean_size: mean(&agg.sizes),
        sizes: dist(&agg.sizes, 8),
        splits: dist(&agg.splits, TOP),
        shapes: dist(&shapes, TOP),
        shared_with_previous: mean(&agg.overlap1),
        shared_with_two_back: mean(&agg.overlap2),
        modifiers: agg.modifiers.iter().map(|(k, &c)| (k.to_string(), c as f64 / agg.rows.max(1) as f64)).collect(),
        targets: dist(&agg.targets.iter().map(|(p, &c)| (format!("{:?}", p), c)).collect(), 6),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KeyReport {
    keys: usize,
    layout: String,
    maps: u64,
    rows: u64,
    notes: u64,
    patterns: Vec<PatternReport>,
    /// Per hand: share of that hand's notes in each pattern.
    groups: BTreeMap<String, Vec<(String, f64)>>,
}

fn key_report(keys: usize, agg: &KeyAgg) -> KeyReport {
    let layout = layout(keys, None);
    let mut patterns: Vec<PatternReport> =
        agg.patterns.iter().map(|(&p, a)| pattern_report(p, a, agg.notes, &layout)).collect();
    patterns.sort_by(|a, b| b.share.total_cmp(&a.share));
    let groups = agg
        .groups
        .iter()
        .map(|(name, counts)| {
            let notes: BTreeMap<String, u64> = counts.iter().map(|(p, &(_, n))| (format!("{:?}", p), n)).collect();
            (name.clone(), dist(&notes, 8))
        })
        .collect();
    KeyReport { keys, layout: layout.name.to_string(), maps: agg.maps, rows: agg.rows, notes: agg.notes, patterns, groups }
}

fn pct(x: f64) -> String {
    format!("{:.0}%", x * 100.0)
}

fn print_key(report: &KeyReport) {
    println!(
        "\n=== {}K ({}) — {} maps, {} rows, {} notes",
        report.keys, report.layout, report.maps, report.rows, report.notes
    );
    for p in report.patterns.iter().filter(|p| p.share >= 0.01) {
        let shapes = p.shapes.iter().take(4).map(|(s, x)| format!("{} {}", s, pct(*x))).collect::<Vec<_>>().join(", ");
        let splits = p.splits.iter().take(3).map(|(s, x)| format!("{} {}", s, pct(*x))).collect::<Vec<_>>().join(", ");
        println!(
            "  {:<15} {:>4}  size {:.2}  shared {:.2}/{:.2}  split [{}]  rows [{}]",
            format!("{:?}", p.pattern),
            pct(p.share),
            p.mean_size,
            p.shared_with_previous,
            p.shared_with_two_back,
            splits,
            shapes
        );
    }
    for (name, dist) in &report.groups {
        let d = dist.iter().take(5).map(|(p, x)| format!("{} {}", p, pct(*x))).collect::<Vec<_>>().join(", ");
        println!("  hand {:<10} {}", name, d);
    }
}

#[test]
#[ignore = "reads a whole Songs folder; run by hand"]
fn corpus_patterns() {
    let songs = songs_dir();
    let files: Vec<PathBuf> = mania_sets(&songs).into_iter().flatten().collect();
    println!("{} mania diffs in {}", files.len(), songs.display());

    let aggs: HashMap<usize, KeyAgg> = parallel(
        &files,
        |acc: &mut HashMap<usize, KeyAgg>, path| {
            let Some(beatmap) = parse(path) else { return };
            let keys = key_count(&beatmap);
            let layout = layout(keys, None);
            let l = label(&beatmap, &layout);
            acc.entry(keys).or_default().add(&l, &layout);
        },
        |total, part| {
            for (k, agg) in part {
                total.entry(k).or_default().merge(agg);
            }
        },
    );

    let mut keys: Vec<usize> = aggs.keys().copied().collect();
    keys.sort();
    let reports: Vec<KeyReport> = keys.iter().map(|&k| key_report(k, &aggs[&k])).collect();
    for report in &reports {
        print_key(report);
    }
    let path = out_dir().join("corpus_patterns.json");
    std::fs::write(&path, serde_json::to_string_pretty(&reports).unwrap()).unwrap();
    println!("\nwritten to {}", path.display());
}

/// A fifth of the sets, picked by folder name: a fixed sample for the runs
/// that convert maps, which would take long on every set.
fn is_test_set(files: &[PathBuf]) -> bool {
    let name = files.first().and_then(|p| p.parent()).and_then(|p| p.file_name()).unwrap_or_default();
    let hash = name.as_encoded_bytes().iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
    hash % 5 == 0
}

/// The diffs of `keys` keys, from the most notes per second to the least.
fn by_difficulty(diffs: &[Beatmap], keys: usize) -> Vec<usize> {
    let nps = |b: &Beatmap| {
        let first = b.hit_objects.first().map_or(0.0, |h| h.start_time);
        let last = b.hit_objects.last().map_or(0.0, |h| h.start_time);
        b.hit_objects.len() as f64 / ((last - first) / 1000.0).max(1.0)
    };
    let mut idx: Vec<usize> = (0..diffs.len()).filter(|&i| key_count(&diffs[i]) == keys).collect();
    idx.sort_by(|&a, &b| nps(&diffs[b]).total_cmp(&nps(&diffs[a])));
    idx
}

/// The diffs of two key counts paired by difficulty: hardest with hardest,
/// second with second...
fn paired(diffs: &[Beatmap], a: usize, b: usize) -> Vec<(usize, usize)> {
    by_difficulty(diffs, a).into_iter().zip(by_difficulty(diffs, b)).collect()
}

/// Rows of `src` and `dst` at the same time.
fn aligned(src: &Labelled, dst: &Labelled) -> Vec<(usize, usize)> {
    aligned_by(src, dst, 0.0)
}

/// Rows of `src` and `dst` at the same time, `dst` being `shift` ms later.
fn aligned_by(src: &Labelled, dst: &Labelled, shift: f64) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut j = 0;
    for (i, row) in src.rows.iter().enumerate() {
        let time = row.time + shift;
        while j < dst.rows.len() && dst.rows[j].time < time - ALIGN_MS {
            j += 1;
        }
        if j < dst.rows.len() && (dst.rows[j].time - time).abs() <= ALIGN_MS {
            out.push((i, j));
        }
    }
    out
}

/// How much later `dst` is than `src` (in ms), for diffs of different sets
/// whose audio starts elsewhere: the shift that lines up the most rows.
fn offset(src: &Labelled, dst: &Labelled) -> f64 {
    const RANGE_MS: f64 = 3000.0;
    let mut votes: HashMap<i64, u32> = HashMap::new();
    let mut lo = 0;
    for row in src.rows.iter().step_by(4) {
        while lo < dst.rows.len() && dst.rows[lo].time < row.time - RANGE_MS {
            lo += 1;
        }
        for d in dst.rows[lo..].iter().take_while(|d| d.time <= row.time + RANGE_MS) {
            *votes.entry((d.time - row.time).round() as i64).or_insert(0) += 1;
        }
    }
    // Rounding splits a shift over two ms: count each with its neighbours.
    let score = |k: i64| (-1..=1).map(|d| votes.get(&(k + d)).copied().unwrap_or(0)).sum::<u32>();
    votes.keys().copied().max_by_key(|&k| (score(k), -k.abs())).unwrap_or(0) as f64
}

/// What each pattern of `src` becomes in `dst`, for the rows they share.
#[derive(Default)]
struct Translation {
    pairs: u64,
    /// Rows of the source found in the target.
    aligned: u64,
    source_rows: u64,
    /// Notes per row of the target against the source, for aligned rows.
    ratio_sum: f64,
    patterns: BTreeMap<Pattern, PatternAgg>,
    /// Per zone of the source: its notes, and the target's in the same time.
    zones: BTreeMap<Zone, (u64, u64)>,
    target: KeyAgg,
}

impl Translation {
    fn merge(&mut self, b: Translation) {
        self.pairs += b.pairs;
        self.aligned += b.aligned;
        self.source_rows += b.source_rows;
        self.ratio_sum += b.ratio_sum;
        for (p, agg) in b.patterns {
            self.patterns.entry(p).or_default().merge(agg);
        }
        for (z, (s, d)) in b.zones {
            let e = self.zones.entry(z).or_default();
            e.0 += s;
            e.1 += d;
        }
        self.target.merge(b.target);
    }

    fn add(&mut self, src: &Labelled, dst: &Labelled, dst_layout: &Layout) {
        let groups: Vec<u32> = dst_layout.groups().iter().map(|g| g.mask).collect();
        self.pairs += 1;
        self.source_rows += src.rows.len() as u64;
        self.target.add(dst, dst_layout);
        for (i, j) in aligned(src, dst) {
            self.aligned += 1;
            self.ratio_sum += dst.rows[j].mask.count_ones() as f64 / src.rows[i].mask.count_ones() as f64;
            let agg = self.patterns.entry(src.labels[i].pattern).or_default();
            agg.add(dst, j, &groups);
            *agg.targets.entry(dst.labels[j].pattern).or_insert(0) += 1;
        }

        // The target's notes from each source row to the next.
        let mut k = 0;
        for (i, row) in src.rows.iter().enumerate() {
            let end = src.rows.get(i + 1).map_or(f64::INFINITY, |r| r.time) - ALIGN_MS;
            while k < dst.rows.len() && dst.rows[k].time < row.time - ALIGN_MS {
                k += 1;
            }
            let mut notes = 0;
            while k < dst.rows.len() && dst.rows[k].time < end {
                notes += dst.rows[k].mask.count_ones() as u64;
                k += 1;
            }
            let e = self.zones.entry(src.zones[i]).or_default();
            e.0 += row.mask.count_ones() as u64;
            e.1 += notes;
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TranslationReport {
    name: String,
    pairs: u64,
    aligned: f64,
    notes_per_row_ratio: f64,
    /// Per zone of the source: target notes per source note.
    zones: Vec<(String, f64)>,
    /// By the pattern of the source row: what the target row looks like.
    from: Vec<PatternReport>,
    /// The target diffs as a whole.
    target: KeyReport,
}

fn translation_report(name: &str, t: &Translation, layout: &Layout) -> TranslationReport {
    let total: u64 = t.patterns.values().map(|a| a.notes).sum();
    let mut from: Vec<PatternReport> = t.patterns.iter().map(|(&p, a)| pattern_report(p, a, total, layout)).collect();
    from.sort_by(|a, b| b.rows.cmp(&a.rows));
    TranslationReport {
        name: name.to_string(),
        pairs: t.pairs,
        aligned: t.aligned as f64 / t.source_rows.max(1) as f64,
        notes_per_row_ratio: t.ratio_sum / t.aligned.max(1) as f64,
        zones: t.zones.iter().map(|(z, &(s, d))| (format!("{:?}", z), d as f64 / s.max(1) as f64)).collect(),
        from,
        target: key_report(layout.keys(), &t.target),
    }
}

/// Total variation distance between two distributions given as top lists.
fn distance(a: &[(String, f64)], b: &[(String, f64)]) -> f64 {
    let mut keys: Vec<&String> = a.iter().chain(b).map(|(k, _)| k).collect();
    keys.sort();
    keys.dedup();
    let get = |v: &[(String, f64)], k: &String| v.iter().find(|(x, _)| x == k).map_or(0.0, |(_, p)| *p);
    keys.iter().map(|k| (get(a, k) - get(b, k)).abs()).sum::<f64>() / 2.0
}

#[test]
#[ignore = "reads a whole Songs folder; run by hand"]
fn corpus_pairs() {
    let pair = std::env::var("PAIR").unwrap_or_else(|_| "4,7".into());
    let (src_keys, dst_keys) = match pair.split(',').map(|k| k.trim().parse::<usize>()).collect::<Vec<_>>()[..] {
        [Ok(a), Ok(b)] => (a, b),
        _ => panic!("PAIR must look like 4,7"),
    };
    let all = std::env::var_os("ALL_SETS").is_some();
    let sets: Vec<Vec<PathBuf>> = mania_sets(&songs_dir()).into_iter().filter(|s| all || is_test_set(s)).collect();
    let src_layout = layout(src_keys, None);
    let dst_layout = layout(dst_keys, None);
    let modes = [
        ("hand-made", None),
        ("converted", Some(Density::None)),
        ("converted Full", Some(Density::Full)),
    ];

    let results: Vec<Translation> = parallel(
        &sets,
        |acc: &mut Vec<Translation>, files| {
            if acc.is_empty() {
                acc.resize_with(modes.len(), Translation::default);
            }
            let diffs: Vec<Beatmap> = files.iter().filter_map(|p| parse(p)).collect();
            for (si, di) in paired(&diffs, src_keys, dst_keys) {
                let (src, real) = (&diffs[si], &diffs[di]);
                let src_l = label(src, &src_layout);
                for (slot, (_, density)) in modes.iter().enumerate() {
                    let converted;
                    let dst = match density {
                        None => real,
                        Some(density) => {
                            let mut b = src.clone();
                            let params = KeyConverterParams {
                                target_keys: dst_keys as u8,
                                density: *density,
                                keep_jacks: true,
                                playstyle: None,
                            };
                            if key_converter::apply(&mut b, &params).is_err() {
                                continue;
                            }
                            converted = b;
                            &converted
                        }
                    };
                    acc[slot].add(&src_l, &label(dst, &dst_layout), &dst_layout);
                }
            }
        },
        |total: &mut Vec<Translation>, part| {
            if total.is_empty() {
                total.resize_with(modes.len(), Translation::default);
            }
            for (t, p) in total.iter_mut().zip(part) {
                t.merge(p);
            }
        },
    );

    let reports: Vec<TranslationReport> =
        modes.iter().zip(&results).map(|((name, _), t)| translation_report(name, t, &dst_layout)).collect();
    let real = &reports[0];
    let fmt = |v: &[(String, f64)], n: usize| v.iter().take(n).map(|(s, x)| format!("{} {}", s, pct(*x))).collect::<Vec<_>>().join(", ");

    println!(
        "\n{}K -> {}K: {} diff pairs from {} test sets, {} of the source rows found in the hand-made target",
        src_keys,
        dst_keys,
        real.pairs,
        sets.len(),
        pct(real.aligned)
    );
    println!("\n=== target notes per source note, by zone of the source");
    for r in &reports {
        let zones = r.zones.iter().map(|(z, x)| format!("{} {:.2}", z, x)).collect::<Vec<_>>().join(", ");
        println!("  {:<15} {}", r.name, zones);
    }

    println!("\n=== source pattern -> target row, aligned rows");
    // Distances weighted by rows, per mode.
    let mut score = vec![(0.0, 0.0, 0.0, 0.0, 0.0); reports.len()];
    for from in &real.from {
        let detail = from.rows >= 500;
        if detail {
            println!(
                "\n  {:?} ({} rows)  hand-made: size {:.2} shared {:.2}/{:.2}",
                from.pattern, from.rows, from.mean_size, from.shared_with_previous, from.shared_with_two_back
            );
            println!("    becomes   {}", fmt(&from.targets, 4));
            println!("    split     {}", fmt(&from.splits, 4));
            println!("    rows      {}", fmt(&from.shapes, 5));
        }
        for (m, conv) in reports.iter().enumerate().skip(1) {
            let Some(c) = conv.from.iter().find(|c| c.pattern == from.pattern) else { continue };
            let d = (distance(&from.targets, &c.targets), distance(&from.sizes, &c.sizes), distance(&from.splits, &c.splits));
            let w = from.rows as f64;
            score[m].0 += w * d.0;
            score[m].1 += w * d.1;
            score[m].2 += w * d.2;
            score[m].3 += w * ((from.shared_with_previous - c.shared_with_previous).abs() + (from.shared_with_two_back - c.shared_with_two_back).abs());
            score[m].4 += w;
            if detail {
                println!(
                    "    {:<15} size {:.2} shared {:.2}/{:.2}  distance: patterns {:.2} sizes {:.2} splits {:.2}",
                    conv.name, c.mean_size, c.shared_with_previous, c.shared_with_two_back, d.0, d.1, d.2
                );
                println!("      becomes {}", fmt(&c.targets, 4));
            }
        }
    }

    println!("\n=== distance to hand-made, weighted by rows (lower is better)");
    for (r, s) in reports.iter().zip(&score).skip(1) {
        let w = s.4.max(1.0);
        println!(
            "  {:<15} patterns {:.3}  sizes {:.3}  splits {:.3}  shared keys {:.3}",
            r.name,
            s.0 / w,
            s.1 / w,
            s.2 / w,
            s.3 / w
        );
    }

    let path = out_dir().join(format!("corpus_{}k_to_{}k.json", src_keys, dst_keys));
    std::fs::write(&path, serde_json::to_string_pretty(&reports).unwrap()).unwrap();
    println!("\nwritten to {}", path.display());
}

/// Problems players notice in a conversion, counted against its source.
#[derive(Default, Debug)]
struct Faults {
    /// Source rows alternating hands (jumptrills, brackets, split chords).
    alternating: usize,
    /// ...where a hand of the target plays both rows: it runs at twice the
    /// speed of the other.
    hand_twice: usize,
    /// ...where the middle key is in both rows: a jack at full speed.
    middle_twice: usize,
    /// Keys repeated in the next row (a minijack) where the source has none.
    new_minijacks: usize,
    rows: usize,
}

fn hands_of(mask: u32, groups: &[(u32, bool)]) -> (u32, bool) {
    // Bit per hand group, and whether the middle (either-hand) key is used.
    let mut hands = 0;
    let mut middle = false;
    for (g, &(gm, either)) in groups.iter().enumerate() {
        if mask & gm != 0 {
            if either {
                middle = true;
            } else {
                hands |= 1 << g;
            }
        }
    }
    (hands, middle)
}

fn faults(src: &Labelled, src_layout: &Layout, dst: &Labelled, dst_layout: &Layout, shift: f64) -> Faults {
    let groups = |l: &Layout| -> Vec<(u32, bool)> {
        let either = l.fingers.iter().enumerate().filter(|(_, f)| f.hand == super::layout::Hand::Either).fold(0u32, |m, (c, _)| m | 1 << c);
        l.groups().iter().map(|g| (g.mask, g.mask & either == g.mask)).collect()
    };
    let (sg, dg) = (groups(src_layout), groups(dst_layout));
    let mut f = Faults { rows: src.rows.len(), ..Default::default() };
    let pairs = aligned_by(src, dst, shift);
    for w in pairs.windows(2) {
        let ((i0, j0), (i1, j1)) = (w[0], w[1]);
        if i1 != i0 + 1 || j1 != j0 + 1 {
            continue;
        }
        let (s0, s1, d0, d1) = (src.rows[i0], src.rows[i1], dst.rows[j0], dst.rows[j1]);
        if s1.time - s0.time > link_ms(s1.beat_len) {
            continue;
        }
        let jack_ms = (s1.beat_len * 0.375).clamp(77.0, 250.0);
        if s0.mask & s1.mask == 0 && d0.mask & d1.mask != 0 && d1.time - d0.time <= jack_ms {
            f.new_minijacks += 1;
        }
        let (h0, m0) = hands_of(s0.mask, &sg);
        let (h1, m1) = hands_of(s1.mask, &sg);
        let chords = s0.mask.count_ones() > 1 || s1.mask.count_ones() > 1;
        if chords && h0 & h1 == 0 && h0 != 0 && h1 != 0 && !(m0 && m1) {
            f.alternating += 1;
            let (t0, n0) = hands_of(d0.mask, &dg);
            let (t1, n1) = hands_of(d1.mask, &dg);
            if t0 & t1 != 0 {
                f.hand_twice += 1;
            }
            if n0 && n1 {
                f.middle_twice += 1;
            }
        }
    }
    f
}

/// [`faults`], only on the source's jumptrills, trills and brackets.
fn trill_faults(src: &Labelled, src_layout: &Layout, dst: &Labelled, dst_layout: &Layout, shift: f64) -> Faults {
    let mut only = Labelled { keys: src.keys, rows: src.rows.clone(), labels: src.labels.clone(), zones: src.zones.clone(), groups: Vec::new() };
    for (row, label) in only.rows.iter_mut().zip(&src.labels) {
        if !matches!(label.pattern, Pattern::Jumptrill | Pattern::Trill | Pattern::Bracket) {
            // Far away from everything: never a pair.
            row.time = f64::NAN;
        }
    }
    let mut f = faults(&only, src_layout, dst, dst_layout, shift);
    f.new_minijacks = 0;
    f
}

fn row_text(mask: u32, layout: &Layout) -> String {
    shape(mask, layout)
}

/// The user's reference maps: a 4K, and a 7K of the same song mapped by hand.
const REFERENCES: [(&str, &str); 3] = [
    ("H.A.R.D.C.O.R.E. (DannyPX) [Hell].osu", "H.A.R.D.C.O.R.E. (Enie) [Extreme].osu"),
    ("Lolit Speed (IcyWorld) [Marathon].osu", "Lolit speed (tyrcs) [Genocide].osu"),
    ("KLASSIKER EINHEIT (IcyWorld) [Challenge].osu", "KLASSIKER EINHEIT (Arona) [BMS LV.23 [Yasashiku]].osu"),
];

fn find_diff(songs: &Path, suffix: &str) -> Option<PathBuf> {
    std::fs::read_dir(songs).ok()?.flatten().find_map(|set| {
        std::fs::read_dir(set.path()).ok()?.flatten().map(|e| e.path()).find(|p| {
            p.file_name().is_some_and(|n| n.to_string_lossy().ends_with(suffix) && !n.to_string_lossy().contains('_'))
        })
    })
}

#[test]
#[ignore = "reads maps from a Songs folder; run by hand"]
fn corpus_references() {
    let songs = songs_dir();
    let (src_layout, dst_layout) = (layout(4, None), layout(7, None));
    let modes = [("None", Density::None), ("Full", Density::Full)];

    for (src_name, ref_name) in REFERENCES {
        let (Some(src_path), Some(ref_path)) = (find_diff(&songs, src_name), find_diff(&songs, ref_name)) else {
            println!("\nmissing {} or {}", src_name, ref_name);
            continue;
        };
        let src = rosu_map::from_path::<Beatmap>(&src_path).unwrap();
        let reference = rosu_map::from_path::<Beatmap>(&ref_path).unwrap();
        let src_l = label(&src, &src_layout);
        let ref_l = label(&reference, &dst_layout);
        println!("\n===== {}  vs  {}", src_name, ref_name);
        println!("  hand-made: {} notes, {} rows", ref_l.rows.iter().map(|r| r.mask.count_ones()).sum::<u32>(), ref_l.rows.len());
        // On the rows the hand-made map shares with the source, once their
        // audio is lined up.
        let shift = offset(&src_l, &ref_l);
        let found = aligned_by(&src_l, &ref_l, shift).len();
        println!("  hand-made is {:+.0} ms off the 4K; {} of the 4K rows found in it", shift, pct(found as f64 / src_l.rows.len().max(1) as f64));
        let f = faults(&src_l, &src_layout, &ref_l, &dst_layout, shift);
        let jt = trill_faults(&src_l, &src_layout, &ref_l, &dst_layout, shift);
        println!(
            "  hand  alternating rows {}: a hand twice {} ({}), middle twice {} | in jumptrills {}: a hand twice {} ({})",
            f.alternating,
            f.hand_twice,
            pct(f.hand_twice as f64 / f.alternating.max(1) as f64),
            f.middle_twice,
            jt.alternating,
            jt.hand_twice,
            pct(jt.hand_twice as f64 / jt.alternating.max(1) as f64),
        );

        let mut converted = Vec::new();
        for (name, density) in modes {
            let mut b = src.clone();
            let params = KeyConverterParams { target_keys: 7, density, keep_jacks: true, playstyle: None };
            key_converter::apply(&mut b, &params).unwrap();
            let l = label(&b, &dst_layout);
            let f = faults(&src_l, &src_layout, &l, &dst_layout, 0.0);
            let jt = trill_faults(&src_l, &src_layout, &l, &dst_layout, 0.0);
            println!(
                "  {:<5} {} notes | alternating rows {}: a hand twice {} ({}), middle twice {} ({}) | in jumptrills {}: a hand twice {} ({}) | new minijacks {} per 1000 rows",
                name,
                l.rows.iter().map(|r| r.mask.count_ones()).sum::<u32>(),
                f.alternating,
                f.hand_twice,
                pct(f.hand_twice as f64 / f.alternating.max(1) as f64),
                f.middle_twice,
                pct(f.middle_twice as f64 / f.alternating.max(1) as f64),
                jt.alternating,
                jt.hand_twice,
                pct(jt.hand_twice as f64 / jt.alternating.max(1) as f64),
                f.new_minijacks * 1000 / f.rows.max(1),
            );
            converted.push((name, l, 0.0));
        }
        converted.push(("hand-made", ref_l, shift));

        // The first jumptrill of the source, side by side.
        let start = (0..src_l.labels.len().saturating_sub(12))
            .find(|&i| src_l.labels[i..i + 12].iter().all(|x| x.pattern == Pattern::Jumptrill));
        if let Some(start) = start {
            println!("\n  first jumptrill at {:.0} ms:", src_l.rows[start].time);
            println!("    {:<6} {}", "4K", converted.iter().map(|(n, _, _)| format!("{:<12}", n)).collect::<String>());
            for i in start..start + 12 {
                let time = src_l.rows[i].time;
                let cells: String = converted
                    .iter()
                    .map(|(_, l, shift)| {
                        let j = l.rows.iter().position(|r| (r.time - time - shift).abs() <= ALIGN_MS);
                        format!("{:<12}", j.map_or("-".to_string(), |j| row_text(l.rows[j].mask, &dst_layout)))
                    })
                    .collect();
                println!("    {:<6} {}", row_text(src_l.rows[i].mask, &src_layout), cells);
            }
        }
    }
}

/// Runs of rows alternating hands (each row on one hand, the middle key with
/// either), mostly chords: jumptrills as 7K maps write them, varying the
/// chords of each hand.
fn hand_trills(l: &Labelled, layout: &Layout, min_rows: usize) -> Vec<std::ops::Range<usize>> {
    let groups = layout.groups();
    let either = layout.fingers.iter().enumerate().filter(|(_, f)| f.hand == super::layout::Hand::Either).fold(0u32, |m, (c, _)| m | 1 << c);
    let side = |mask: u32| -> Option<usize> {
        let used: Vec<usize> = groups.iter().enumerate().filter(|(_, g)| g.mask & !either != 0 && mask & g.mask & !either != 0).map(|(i, _)| i).collect();
        (used.len() == 1).then(|| used[0])
    };
    let mut runs = Vec::new();
    let mut start = 0;
    for i in 1..=l.rows.len() {
        let continues = i < l.rows.len()
            && l.rows[i].time - l.rows[i - 1].time <= link_ms(l.rows[i].beat_len)
            && matches!((side(l.rows[i - 1].mask), side(l.rows[i].mask)), (Some(a), Some(b)) if a != b)
            && l.rows[i].mask & l.rows[i - 1].mask & either == 0;
        if !continues {
            let chords = l.rows[start..i].iter().filter(|r| r.mask.count_ones() > 1).count();
            if i - start >= min_rows && chords * 2 >= i - start {
                runs.push(start..i);
            }
            start = i;
        }
    }
    runs
}

#[test]
#[ignore = "reads a map from a Songs folder; run by hand"]
fn corpus_show() {
    let name = std::env::var("MAP").expect("MAP: the end of the .osu file name");
    let path = find_diff(&songs_dir(), &name).expect("map not found");
    let beatmap = rosu_map::from_path::<Beatmap>(&path).unwrap();
    let layout = layout(key_count(&beatmap), None);
    let l = label(&beatmap, &layout);
    let runs = hand_trills(&l, &layout, 8);
    let rows: usize = runs.iter().map(|r| r.len()).sum();
    println!("\n{}: {} rows, {} hand-alternating runs ({} rows)", name, l.rows.len(), runs.len(), rows);

    // What the classifier calls them, and what they look like.
    let mut labels: BTreeMap<String, u64> = BTreeMap::new();
    let mut sizes: BTreeMap<u32, u64> = BTreeMap::new();
    let (mut thumb, mut repeats2, mut pairs) = (0, 0, 0);
    let thumb_mask = layout.digit_mask(super::layout::Digit::Thumb);
    for run in &runs {
        for i in run.clone() {
            *labels.entry(format!("{:?}", l.labels[i].pattern)).or_insert(0) += 1;
            *sizes.entry(l.rows[i].mask.count_ones()).or_insert(0) += 1;
            thumb += (l.rows[i].mask & thumb_mask != 0) as usize;
            if i >= run.start + 2 {
                pairs += 1;
                repeats2 += (l.rows[i].mask == l.rows[i - 2].mask) as usize;
            }
        }
    }
    let fmt = |v: &[(String, f64)]| v.iter().map(|(s, x)| format!("{} {}", s, pct(*x))).collect::<Vec<_>>().join(", ");
    println!("  labelled  {}", fmt(&dist(&labels, 8)));
    println!("  sizes     {}", fmt(&dist(&sizes, 8)));
    println!("  middle key in {} of the rows; same row as 2 back {}", pct(thumb as f64 / rows.max(1) as f64), pct(repeats2 as f64 / pairs.max(1) as f64));
    for run in runs.iter().take(6) {
        println!("\n  at {:.0} ms, {} rows:", l.rows[run.start].time, run.len());
        for i in run.clone().take(16) {
            println!("    {}  {:?}", shape(l.rows[i].mask, &layout), l.labels[i].pattern);
        }
    }
}

/// What hand-alternating runs (see [`hand_trills`]) look like.
#[derive(Default)]
struct HandTrillAgg {
    maps: u64,
    rows: u64,
    pairs: u64,
    /// The row mirrors the previous one (the middle key aside).
    mirrored: u64,
    /// The row repeats the one 2 back.
    repeated: u64,
    middle: u64,
    sizes: BTreeMap<u32, u64>,
}

impl HandTrillAgg {
    fn add(&mut self, l: &Labelled, layout: &Layout) {
        let middle = layout.digit_mask(super::layout::Digit::Thumb);
        let keys = layout.keys();
        let mirror = |m: u32| (0..keys).filter(|&k| m & 1 << k != 0).fold(0u32, |a, k| a | 1 << (keys - 1 - k));
        self.maps += 1;
        for run in hand_trills(l, layout, 8) {
            for i in run.clone() {
                let m = l.rows[i].mask;
                self.rows += 1;
                self.middle += (m & middle != 0) as u64;
                *self.sizes.entry(m.count_ones()).or_insert(0) += 1;
                if i > run.start {
                    self.pairs += 1;
                    self.mirrored += (mirror(m & !middle) == l.rows[i - 1].mask & !middle) as u64;
                }
                if i >= run.start + 2 {
                    self.repeated += (m == l.rows[i - 2].mask) as u64;
                }
            }
        }
    }

    fn merge(&mut self, b: HandTrillAgg) {
        self.maps += b.maps;
        self.rows += b.rows;
        self.pairs += b.pairs;
        self.mirrored += b.mirrored;
        self.repeated += b.repeated;
        self.middle += b.middle;
        add_counts(&mut self.sizes, b.sizes);
    }

    fn print(&self, name: &str) {
        let sizes = dist(&self.sizes, 6).iter().map(|(s, x)| format!("{} {}", s, pct(*x))).collect::<Vec<_>>().join(", ");
        println!(
            "  {:<18} {} maps, {} rows | mirrors the previous row {} | repeats 2 back {} | middle key {} | sizes {}",
            name,
            self.maps,
            self.rows,
            pct(self.mirrored as f64 / self.pairs.max(1) as f64),
            pct(self.repeated as f64 / self.pairs.max(1) as f64),
            pct(self.middle as f64 / self.rows.max(1) as f64),
            sizes
        );
    }
}

#[test]
#[ignore = "reads a whole Songs folder; run by hand"]
fn corpus_hand_trills() {
    let sets = mania_sets(&songs_dir());
    let seven = layout(7, None);
    let modes = [("hand-made 7K", None), ("4K -> 7K Full", Some(Density::Full))];
    let aggs: Vec<HandTrillAgg> = parallel(
        &sets,
        |acc: &mut Vec<HandTrillAgg>, files| {
            if acc.is_empty() {
                acc.resize_with(modes.len(), HandTrillAgg::default);
            }
            for beatmap in files.iter().filter_map(|p| parse(p)) {
                match key_count(&beatmap) {
                    7 => acc[0].add(&label(&beatmap, &seven), &seven),
                    // Converting every 4K would take long: one in four.
                    4 if beatmap.hit_objects.len() % 4 == 0 => {
                        for (slot, (_, density)) in modes.iter().enumerate().skip(1) {
                            let mut b = beatmap.clone();
                            let params = KeyConverterParams { target_keys: 7, density: density.unwrap(), keep_jacks: true, playstyle: None };
                            if key_converter::apply(&mut b, &params).is_ok() {
                                acc[slot].add(&label(&b, &seven), &seven);
                            }
                        }
                    }
                    _ => {}
                }
            }
        },
        |total: &mut Vec<HandTrillAgg>, part| {
            if total.is_empty() {
                total.resize_with(modes.len(), HandTrillAgg::default);
            }
            for (t, p) in total.iter_mut().zip(part) {
                t.merge(p);
            }
        },
    );
    println!("\nhand-alternating runs of 8+ rows, mostly chords:");
    for ((name, _), agg) in modes.iter().zip(&aggs) {
        agg.print(name);
    }
}

/// What a row is part of, for [`Feel`]: single-note streams, chordstreams,
/// jumptrills and brackets, chordjacks.
const FAMILIES: [&str; 4] = ["streams", "chordstreams", "jumptrills", "chordjacks"];

fn family(pattern: Pattern) -> Option<usize> {
    match pattern {
        Pattern::Stream | Pattern::Roll | Pattern::Trill => Some(0),
        Pattern::Jumpstream | Pattern::Handstream | Pattern::Chordstream => Some(1),
        Pattern::Jumptrill | Pattern::Bracket => Some(2),
        Pattern::Jack | Pattern::Chordjack | Pattern::DenseChordjack => Some(3),
        _ => None,
    }
}

#[derive(Default, Clone, Copy)]
struct FeelCounts {
    rows: u64,
    notes: u64,
    /// Rows of 2+ notes.
    chords: u64,
    /// Chords with two next keys on the same hand (a pill).
    pills: u64,
    /// Chords on both hands.
    split: u64,
    /// A hand playing 2+ notes in a row, and how often they are all next to
    /// each other (a closed chord, `OO.` rather than `O.O`).
    hand_chords: u64,
    closed: u64,
    outer: u64,
    pinky: u64,
    /// Linked rows, both on some hand: how often the busier hand changes (the
    /// hands taking turns, as 7K chordstreams do), and how often both hands
    /// play in both rows (each hand with its own pattern).
    pairs: u64,
    flips: u64,
    both: u64,
}

impl FeelCounts {
    fn merge(&mut self, b: FeelCounts) {
        self.rows += b.rows;
        self.notes += b.notes;
        self.chords += b.chords;
        self.pills += b.pills;
        self.split += b.split;
        self.hand_chords += b.hand_chords;
        self.closed += b.closed;
        self.outer += b.outer;
        self.pinky += b.pinky;
        self.pairs += b.pairs;
        self.flips += b.flips;
        self.both += b.both;
    }
}

/// How a map uses the hands and the keys, measured for the feedback on
/// 7K -> 10K converts: pills, hands taking turns, outer keys left aside.
#[derive(Default)]
struct Feel {
    maps: u64,
    families: [FeelCounts; 4],
    /// Rows and notes in each density zone.
    zones: [(u64, u64); 5],
    /// Runs of 4 rows of one hand that stay on its outer three keys (pinky,
    /// ring, middle), its middle three, its inner three, or go wider.
    windows: [u64; 4],
    /// Chordstream chords of 2, 3 and 4 notes: rows, pills, closed hands.
    by_size: [(u64, u64, u64); 3],
    /// In chordstreams, the pattern each hand plays on its own.
    hand_patterns: BTreeMap<Pattern, u64>,
    /// In chordstreams, how far a hand moves from one of its rows to the next
    /// (in keys, between the middles of its notes: 0, 1, 2, 3+), and how often
    /// it keeps going the same way.
    steps: [u64; 4],
    same_way: u64,
    turns_back: u64,
    /// Chordstream rows, and how many of them each hand plays in.
    duty_rows: u64,
    duty: u64,
}

impl Feel {
    fn merge(&mut self, b: Feel) {
        self.maps += b.maps;
        for (a, b) in self.families.iter_mut().zip(b.families) {
            a.merge(b);
        }
        for (a, b) in self.zones.iter_mut().zip(b.zones) {
            a.0 += b.0;
            a.1 += b.1;
        }
        for (a, b) in self.windows.iter_mut().zip(b.windows) {
            *a += b;
        }
        for (a, b) in self.by_size.iter_mut().zip(b.by_size) {
            a.0 += b.0;
            a.1 += b.1;
            a.2 += b.2;
        }
        add_counts(&mut self.hand_patterns, b.hand_patterns);
        for (a, b) in self.steps.iter_mut().zip(b.steps) {
            *a += b;
        }
        self.same_way += b.same_way;
        self.turns_back += b.turns_back;
        self.duty_rows += b.duty_rows;
        self.duty += b.duty;
    }

    fn add(&mut self, l: &Labelled, layout: &Layout) {
        use super::layout::{Digit, Hand};
        self.maps += 1;

        // The hands (the middle key aside) and how far each key is from the
        // outside of its hand: 0 pinky ... 4 thumb.
        let hands: Vec<u32> = layout
            .groups()
            .iter()
            .map(|g| g.mask)
            .filter(|&m| (0..layout.keys()).any(|k| m & 1 << k != 0 && layout.fingers[k].hand != Hand::Either))
            .collect();
        let place = |k: usize| match layout.fingers[k].digit {
            Digit::Pinky | Digit::Scratch => 0u32,
            Digit::Ring => 1,
            Digit::Middle => 2,
            Digit::Index => 3,
            Digit::Thumb => 4,
        };
        let outer = layout.digit_mask(Digit::Pinky) | layout.digit_mask(Digit::Ring);
        let pinky = layout.digit_mask(Digit::Pinky);
        let linked = |a: usize, b: usize| l.rows[b].time - l.rows[a].time <= link_ms(l.rows[b].beat_len);
        let busier = |mask: u32| -> i32 {
            match hands.as_slice() {
                [left, right] => ((mask & left).count_ones() as i32 - (mask & right).count_ones() as i32).signum(),
                _ => 0,
            }
        };

        for (i, row) in l.rows.iter().enumerate() {
            let size = row.mask.count_ones() as u64;
            let z = Zone::ALL.iter().position(|&z| z == l.zones[i]).unwrap_or(0);
            self.zones[z].0 += 1;
            self.zones[z].1 += size;

            let in_phrase = (i > 0 && linked(i - 1, i)) || (i + 1 < l.rows.len() && linked(i, i + 1));
            let Some(f) = family(l.labels[i].pattern).filter(|_| in_phrase) else { continue };
            let pill = hands.iter().any(|&h| (row.mask & h) & ((row.mask & h) >> 1) != 0);
            let closed_hand = hands.iter().any(|&h| {
                let m = row.mask & h;
                m.count_ones() >= 2 && (31 - m.leading_zeros()) - m.trailing_zeros() + 1 == m.count_ones()
            });
            if f == 1 {
                if (2..=4).contains(&size) {
                    let b = &mut self.by_size[size as usize - 2];
                    b.0 += 1;
                    b.1 += pill as u64;
                    b.2 += closed_hand as u64;
                }
                self.duty_rows += 1;
                self.duty += hands.iter().filter(|&&h| row.mask & h != 0).count() as u64;
            }
            let c = &mut self.families[f];
            c.rows += 1;
            c.notes += size;
            c.outer += (row.mask & outer).count_ones() as u64;
            c.pinky += (row.mask & pinky).count_ones() as u64;
            if size >= 2 {
                c.chords += 1;
                c.pills += pill as u64;
                c.split += (hands.iter().filter(|&&h| row.mask & h != 0).count() > 1) as u64;
            }
            for &h in &hands {
                let m = row.mask & h;
                if m.count_ones() >= 2 {
                    c.hand_chords += 1;
                    let (lo, hi) = (m.trailing_zeros(), 31 - m.leading_zeros());
                    c.closed += (hi - lo + 1 == m.count_ones()) as u64;
                }
            }
            if i > 0 && linked(i - 1, i) {
                let prev = l.rows[i - 1].mask;
                let (a, b) = (busier(prev), busier(row.mask));
                c.pairs += 1;
                c.flips += (a != 0 && b != 0 && a != b) as u64;
                c.both += (hands.len() == 2 && hands.iter().all(|&h| prev & h != 0 && row.mask & h != 0)) as u64;
            }
        }

        // What each hand plays on its own, in chordstreams.
        let chordstream = |i: usize| family(l.labels[i].pattern) == Some(1);
        for g in &l.groups {
            if !hands.contains(&g.group.mask) {
                continue;
            }
            for (&i, label) in g.rows.iter().zip(&g.labels) {
                if chordstream(i) {
                    *self.hand_patterns.entry(label.pattern).or_insert(0) += 1;
                }
            }
            let middle = |i: usize| {
                let keys: Vec<f64> = (0..layout.keys()).filter(|&k| l.rows[i].mask & g.group.mask & 1 << k != 0).map(|k| k as f64).collect();
                keys.iter().sum::<f64>() / keys.len() as f64
            };
            let mut last_dir = 0.0f64;
            for w in g.rows.windows(2) {
                let (a, b) = (w[0], w[1]);
                if !chordstream(b) || !linked(a, b) {
                    last_dir = 0.0;
                    continue;
                }
                let d = middle(b) - middle(a);
                self.steps[(d.abs().round() as usize).min(3)] += 1;
                if d.abs() > 0.25 && last_dir.abs() > 0.25 {
                    if d.signum() == last_dir.signum() {
                        self.same_way += 1;
                    } else {
                        self.turns_back += 1;
                    }
                }
                last_dir = d;
            }
        }

        // Where each hand's patterns sit, on hands of 5 keys.
        for &h in &hands {
            let places: u32 = (0..layout.keys()).filter(|k| h & 1 << k != 0).fold(0, |m, k| m | 1 << place(k));
            if places != 0b11111 {
                continue;
            }
            let rows: Vec<usize> = (0..l.rows.len()).filter(|&i| l.rows[i].mask & h != 0).collect();
            for w in rows.windows(4) {
                if !w.windows(2).all(|p| linked(p[0], p[1])) {
                    continue;
                }
                let used = w.iter().flat_map(|&i| (0..layout.keys()).filter(move |&k| l.rows[i].mask & h & 1 << k != 0)).fold(0u32, |m, k| m | 1 << place(k));
                let slot = if used & !0b00111 == 0 && used & 1 != 0 {
                    0
                } else if used & !0b11100 == 0 && used & 0b10000 != 0 {
                    2
                } else if used & !0b01110 == 0 {
                    1
                } else {
                    3
                };
                self.windows[slot] += 1;
            }
        }
    }

    fn print(&self, name: &str) {
        let r = |a: u64, b: u64| pct(a as f64 / b.max(1) as f64);
        println!("\n  {} ({} maps)", name, self.maps);
        let total: u64 = self.families.iter().map(|c| c.rows).sum();
        for (f, c) in FAMILIES.iter().zip(&self.families) {
            println!(
                "    {:<13} {:>4} of rows | size {:.2} | pills {:>4} split {:>4} closed {:>4} of chords | outer {:>4} pinky {:>4} of notes | turns {:>4} both hands {:>4}",
                f,
                r(c.rows, total),
                c.notes as f64 / c.rows.max(1) as f64,
                r(c.pills, c.chords),
                r(c.split, c.chords),
                r(c.closed, c.hand_chords),
                r(c.outer, c.notes),
                r(c.pinky, c.notes),
                r(c.flips, c.pairs),
                r(c.both, c.pairs),
            );
        }
        let zones = Zone::ALL
            .iter()
            .zip(&self.zones)
            .map(|(z, &(rows, notes))| format!("{:?} {:.2}", z, notes as f64 / rows.max(1) as f64))
            .collect::<Vec<_>>()
            .join(", ");
        println!("    row size by zone: {}", zones);
        let sizes = self
            .by_size
            .iter()
            .enumerate()
            .map(|(i, &(rows, pills, closed))| format!("{} notes: pills {} closed {}", i + 2, r(pills, rows), r(closed, rows)))
            .collect::<Vec<_>>()
            .join(" | ");
        println!("    chordstream chords by size: {}", sizes);
        let hp = dist(&self.hand_patterns.iter().map(|(p, &c)| (format!("{:?}", p), c)).collect(), 6);
        println!("    a hand on its own in chordstreams: {}", hp.iter().map(|(p, x)| format!("{} {}", p, pct(*x))).collect::<Vec<_>>().join(", "));
        let steps: u64 = self.steps.iter().sum();
        println!(
            "    a hand's steps: 0 {} 1 {} 2 {} 3+ {} | keeps its way {} | hands per row {:.2}",
            r(self.steps[0], steps),
            r(self.steps[1], steps),
            r(self.steps[2], steps),
            r(self.steps[3], steps),
            r(self.same_way, self.same_way + self.turns_back),
            self.duty as f64 / self.duty_rows.max(1) as f64,
        );
        let w = self.windows.iter().sum::<u64>();
        if w > 0 {
            println!(
                "    4-row runs of a hand: outer keys {} middle {} inner {} wider {}",
                r(self.windows[0], w),
                r(self.windows[1], w),
                r(self.windows[2], w),
                r(self.windows[3], w)
            );
        }
    }
}

/// What hand-made 7K and 10K maps look like next to 7K -> 10K converts, on
/// what the feedback on them talks about (see [`Feel`]). The converts are of
/// the test sets' 7K maps, `LIMIT` of them (300 by default). `MAP` also shows
/// that map's converts.
#[test]
#[ignore = "reads a whole Songs folder; run by hand"]
fn corpus_feel() {
    let sets = mania_sets(&songs_dir());
    let limit: usize = std::env::var("LIMIT").ok().and_then(|s| s.parse().ok()).unwrap_or(300);
    let (seven, ten) = (layout(7, None), layout(10, None));
    let modes = [("None", Density::None), ("MoreDense", Density::MoreDense), ("Full", Density::Full)];

    let both = sets
        .iter()
        .filter(|files| {
            let keys: Vec<usize> = files.iter().filter_map(|p| parse(p)).map(|b| key_count(&b)).collect();
            keys.contains(&7) && keys.contains(&10)
        })
        .count();
    println!("{} sets with hand-made 7K and 10K diffs", both);

    let sevens: Vec<PathBuf> = sets.iter().filter(|f| is_test_set(f)).flatten().cloned().collect();
    let sevens: Vec<Beatmap> = sevens.iter().filter_map(|p| parse(p)).filter(|b| key_count(b) == 7).take(limit).collect();

    // `CONVERTS_ONLY=1` skips the hand-made maps, which do not change between runs.
    let hand_sets: &[Vec<PathBuf>] = if std::env::var_os("CONVERTS_ONLY").is_some() { &[] } else { &sets };
    let hand_made: Vec<Feel> = parallel(
        hand_sets,
        |acc: &mut Vec<Feel>, files| {
            if acc.is_empty() {
                acc.resize_with(2, Feel::default);
            }
            for beatmap in files.iter().filter_map(|p| parse(p)) {
                match key_count(&beatmap) {
                    7 => acc[0].add(&label(&beatmap, &seven), &seven),
                    10 => acc[1].add(&label(&beatmap, &ten), &ten),
                    _ => {}
                }
            }
        },
        |total: &mut Vec<Feel>, part| {
            if total.is_empty() {
                total.resize_with(2, Feel::default);
            }
            for (t, p) in total.iter_mut().zip(part) {
                t.merge(p);
            }
        },
    );

    let converts: Vec<Feel> = parallel(
        &sevens,
        |acc: &mut Vec<Feel>, beatmap| {
            if acc.is_empty() {
                acc.resize_with(modes.len(), Feel::default);
            }
            for (slot, (_, density)) in modes.iter().enumerate() {
                let mut b = beatmap.clone();
                let params = KeyConverterParams { target_keys: 10, density: *density, keep_jacks: true, playstyle: None };
                if key_converter::apply(&mut b, &params).is_ok() {
                    acc[slot].add(&label(&b, &ten), &ten);
                }
            }
        },
        |total: &mut Vec<Feel>, part| {
            if total.is_empty() {
                total.resize_with(modes.len(), Feel::default);
            }
            for (t, p) in total.iter_mut().zip(part) {
                t.merge(p);
            }
        },
    );

    if let [seven, ten] = hand_made.as_slice() {
        seven.print("hand-made 7K");
        ten.print("hand-made 10K");
    }
    for ((name, _), feel) in modes.iter().zip(&converts) {
        feel.print(&format!("7K -> 10K {}", name));
    }

    if let Ok(name) = std::env::var("MAP") {
        let path = find_diff(&songs_dir(), &name).expect("map not found");
        let beatmap = rosu_map::from_path::<Beatmap>(&path).unwrap();
        let mut feel = Feel::default();
        feel.add(&label(&beatmap, &seven), &seven);
        feel.print(&name);
        for (mode, density) in modes {
            let mut b = beatmap.clone();
            let params = KeyConverterParams { target_keys: 10, density, keep_jacks: true, playstyle: None };
            key_converter::apply(&mut b, &params).unwrap();
            let mut feel = Feel::default();
            feel.add(&label(&b, &ten), &ten);
            feel.print(&format!("{} -> 10K {}", name, mode));
        }
    }
}

/// A passage of `MAP` (`FROM` ms, `ROWS` rows) next to its 10K converts and,
/// with `OLD`, another 10K diff of the set (an earlier convert), as text, the
/// first row at the bottom as in the editor.
#[test]
#[ignore = "reads a map from a Songs folder; run by hand"]
fn corpus_compare() {
    let songs = songs_dir();
    let name = std::env::var("MAP").expect("MAP: the end of the .osu file name");
    let from: f64 = std::env::var("FROM").ok().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let count: usize = std::env::var("ROWS").ok().and_then(|s| s.parse().ok()).unwrap_or(32);
    let source = rosu_map::from_path::<Beatmap>(&find_diff(&songs, &name).expect("map not found")).unwrap();
    let keys = key_count(&source);

    let mut columns: Vec<(String, Beatmap)> = vec![(format!("{}K", keys), source.clone())];
    if let Ok(old) = std::env::var("OLD") {
        columns.push(("old".into(), rosu_map::from_path::<Beatmap>(&find_diff(&songs, &old).expect("OLD not found")).unwrap()));
    }
    for (mode, density) in [("None", Density::None), ("MoreDense", Density::MoreDense), ("Full", Density::Full)] {
        let mut b = source.clone();
        let params = KeyConverterParams { target_keys: 10, density, keep_jacks: true, playstyle: None };
        key_converter::apply(&mut b, &params).unwrap();
        columns.push((mode.into(), b));
    }

    let labelled: Vec<(String, Labelled, Layout)> = columns
        .into_iter()
        .map(|(n, b)| {
            let layout = layout(key_count(&b), None);
            (n, label(&b, &layout), layout)
        })
        .collect();
    let times: Vec<f64> = labelled[0].1.rows.iter().map(|r| r.time).filter(|&t| t >= from).take(count).collect();
    let header = labelled.iter().map(|(n, _, l)| format!("{:<w$}", n, w = l.keys() + 2)).collect::<Vec<_>>().join("  ");
    println!("\n{:>8}  {}", "ms", header);
    for &t in times.iter().rev() {
        let cells = labelled
            .iter()
            .map(|(_, l, layout)| {
                let row = l.rows.iter().find(|r| (r.time - t).abs() < ALIGN_MS);
                let text = row.map_or_else(|| " ".repeat(layout.keys() + layout.groups().len() - 1), |r| shape(r.mask, layout));
                format!("{:<w$}", text, w = layout.keys() + 2)
            })
            .collect::<Vec<_>>()
            .join("  ");
        println!("{:>8.0}  {}", t, cells);
    }
}

/// How a set of maps uses each column.
#[derive(Default, Clone)]
struct ColumnAgg {
    maps: u64,
    keys: usize,
    /// Notes per column, and the sum over the maps of each column's share of
    /// its map (so every map weighs the same).
    notes: [u64; 18],
    shares: [f64; 18],
    /// Notes in the outer columns (first and last) in single-note rows and in
    /// chords, against all the notes in each.
    outer_single: u64,
    single: u64,
    outer_chord: u64,
    chord: u64,
    /// Maps whose outer columns get over 1.3 times the notes of an average
    /// column.
    heavy: u64,
}

/// See [`ColumnAgg::heavy`].
const HEAVY_OUTER: f64 = 1.3;

impl ColumnAgg {
    fn add(&mut self, l: &Labelled) {
        let keys = l.keys;
        self.keys = keys;
        self.maps += 1;
        let mut notes = [0u64; 18];
        let outer = 1u32 | 1 << (keys - 1);
        for row in &l.rows {
            for (k, n) in notes.iter_mut().enumerate().take(keys) {
                *n += (row.mask >> k & 1) as u64;
            }
            let size = row.mask.count_ones() as u64;
            let o = (row.mask & outer).count_ones() as u64;
            if size == 1 {
                self.single += 1;
                self.outer_single += o;
            } else {
                self.chord += size;
                self.outer_chord += o;
            }
        }
        let total: u64 = notes.iter().sum();
        for k in 0..keys {
            self.notes[k] += notes[k];
            self.shares[k] += notes[k] as f64 / total.max(1) as f64;
        }
        let outer = (notes[0] + notes[keys - 1]) as f64 / 2.0;
        self.heavy += (outer > HEAVY_OUTER * total as f64 / keys as f64) as u64;
    }

    fn merge(&mut self, b: ColumnAgg) {
        self.maps += b.maps;
        self.keys = self.keys.max(b.keys);
        for k in 0..18 {
            self.notes[k] += b.notes[k];
            self.shares[k] += b.shares[k];
        }
        self.outer_single += b.outer_single;
        self.single += b.single;
        self.outer_chord += b.outer_chord;
        self.chord += b.chord;
        self.heavy += b.heavy;
    }

    fn print(&self, name: &str) {
        let keys = self.keys;
        let shares: Vec<f64> = (0..keys).map(|k| self.shares[k] / self.maps.max(1) as f64).collect();
        let even = 1.0 / keys as f64;
        let cols = shares.iter().map(|s| format!("{:>4.1}", s * 100.0)).collect::<Vec<_>>().join(" ");
        let outer = (shares[0] + shares[keys - 1]) / 2.0;
        println!(
            "  {:<22} {:>5} maps | columns % [{}] | outer columns {:.2}x a column | maps over {}x {} | outer in single notes {} / in chords {}",
            name,
            self.maps,
            cols,
            outer / even,
            HEAVY_OUTER,
            pct(self.heavy as f64 / self.maps.max(1) as f64),
            pct(self.outer_single as f64 / self.single.max(1) as f64),
            pct(self.outer_chord as f64 / self.chord.max(1) as f64),
        );
    }
}

/// How often hand-made 9K and 10K maps use each column, the outer ones
/// above all, next to 4K and 7K converts to 9K and 10K (`LIMIT` source maps
/// of each key count from the test sets, 200 by default).
#[test]
#[ignore = "reads a whole Songs folder; run by hand"]
fn corpus_columns() {
    let sets = mania_sets(&songs_dir());
    let limit: usize = std::env::var("LIMIT").ok().and_then(|s| s.parse().ok()).unwrap_or(200);
    let targets = [9usize, 10];
    let modes = [("None", Density::None), ("MoreDense", Density::MoreDense), ("Full", Density::Full)];

    let hand_made: BTreeMap<usize, ColumnAgg> = parallel(
        &sets,
        |acc: &mut BTreeMap<usize, ColumnAgg>, files| {
            for beatmap in files.iter().filter_map(|p| parse(p)) {
                let keys = key_count(&beatmap);
                if targets.contains(&keys) {
                    acc.entry(keys).or_default().add(&label(&beatmap, &layout(keys, None)));
                }
            }
        },
        |total, part| {
            for (k, agg) in part {
                total.entry(k).or_default().merge(agg);
            }
        },
    );

    let test_maps: Vec<Beatmap> = sets.iter().filter(|f| is_test_set(f)).flatten().filter_map(|p| parse(p)).collect();
    let sources: Vec<Beatmap> = [4usize, 7]
        .iter()
        .flat_map(|&keys| test_maps.iter().filter(move |b| key_count(b) == keys).take(limit).cloned())
        .collect();

    // Keyed by (source keys, target keys, mode).
    let converts: BTreeMap<(usize, usize, usize), ColumnAgg> = parallel(
        &sources,
        |acc: &mut BTreeMap<(usize, usize, usize), ColumnAgg>, beatmap| {
            let src = key_count(beatmap);
            for &dst in &targets {
                for (m, (_, density)) in modes.iter().enumerate() {
                    let mut b = beatmap.clone();
                    let params = KeyConverterParams { target_keys: dst as u8, density: *density, keep_jacks: true, playstyle: None };
                    if key_converter::apply(&mut b, &params).is_ok() {
                        acc.entry((src, dst, m)).or_default().add(&label(&b, &layout(dst, None)));
                    }
                }
            }
        },
        |total, part| {
            for (k, agg) in part {
                total.entry(k).or_default().merge(agg);
            }
        },
    );

    for &dst in &targets {
        println!("\n=== {}K ({})", dst, layout(dst, None).name);
        if let Some(agg) = hand_made.get(&dst) {
            agg.print("hand-made");
        }
        for ((src, d, m), agg) in &converts {
            if *d == dst {
                agg.print(&format!("{}K -> {}K {}", src, dst, modes[*m].0));
            }
        }
    }
}
