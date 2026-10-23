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
//! - `corpus_style`: learns the style tables of the key converter's Auto
//!   mode (see `style`), from every set but the test ones. `WRITE_STYLE=1`
//!   writes them over `style.json`, `ALL_SETS=1` learns from the test sets
//!   too.
//! - `corpus_pairs`: for the test sets with diffs of both key counts of
//!   `PAIR` (`4,7` by default), what the patterns of the source become in the
//!   hand-made target (rows at the same time) and in the key converter's
//!   output, and how dense each zone of the source gets.

use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    sync::{Mutex, atomic::{AtomicUsize, Ordering}},
};

use rosu_map::Beatmap;
use serde::Serialize;

use super::{
    layout::{Layout, layout},
    patterns::{Labelled, Pattern, key_count, label, link_ms, modifier},
    style::StyleTable,
    zones::Zone,
};
use crate::commands::trainer::{
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
/// which would teach the style what the converter already does.
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

/// Sets whose folder name hashes to 0 mod 5 are kept out of the learned
/// style, to evaluate it on maps it has not seen.
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

fn groups_of(keys: usize) -> Vec<u32> {
    layout(keys, None).groups().iter().map(|g| g.mask).collect()
}

/// The converter's key counts.
const CONVERTER_KEYS: std::ops::RangeInclusive<usize> = 1..=10;
/// Tables need at least this many maps, or sets for a pair of key counts.
const MIN_MAPS: u32 = 50;
const MIN_SETS: u32 = 30;

#[derive(Default)]
struct StyleAcc {
    own: BTreeMap<usize, StyleTable>,
    pairs: BTreeMap<(usize, usize), StyleTable>,
    sets: BTreeMap<(usize, usize), u64>,
}

#[test]
#[ignore = "reads a whole Songs folder; run by hand"]
fn corpus_style() {
    let all = std::env::var_os("ALL_SETS").is_some();
    let sets: Vec<Vec<PathBuf>> = mania_sets(&songs_dir()).into_iter().filter(|s| all || !is_test_set(s)).collect();
    println!("learning from {} sets", sets.len());

    let acc: StyleAcc = parallel(
        &sets,
        |acc: &mut StyleAcc, files| {
            let diffs: Vec<Beatmap> =
                files.iter().filter_map(|p| parse(p)).filter(|b| CONVERTER_KEYS.contains(&key_count(b))).collect();
            let labelled: Vec<Labelled> = diffs.iter().map(|b| label(b, &layout(key_count(b), None))).collect();

            for l in &labelled {
                let groups = groups_of(l.keys);
                let table = acc.own.entry(l.keys).or_insert_with(|| StyleTable { dst_keys: l.keys as u8, ..Default::default() });
                table.maps += 1;
                for j in 0..l.rows.len() {
                    table.add_row(l.labels[j].pattern, 0, l.zones[j], l.rows[j].beat, l, j, &groups);
                }
            }

            let mut keys: Vec<usize> = labelled.iter().map(|l| l.keys).collect();
            keys.sort();
            keys.dedup();
            for &a in &keys {
                for &b in keys.iter().filter(|&&b| b != a) {
                    let groups = groups_of(b);
                    let table = acc
                        .pairs
                        .entry((a, b))
                        .or_insert_with(|| StyleTable { src_keys: a as u8, dst_keys: b as u8, ..Default::default() });
                    for (si, di) in paired(&diffs, a, b) {
                        let (src, dst) = (&labelled[si], &labelled[di]);
                        table.maps += 1;
                        for (i, j) in aligned(src, dst) {
                            let s = &src.rows[i];
                            table.add_row(src.labels[i].pattern, s.mask.count_ones(), src.zones[i], s.beat, dst, j, &groups);
                        }
                    }
                    *acc.sets.entry((a, b)).or_insert(0) += 1;
                }
            }
        },
        |total, part| {
            for (k, t) in part.own {
                total.own.entry(k).or_default().merge(t);
                let mine = total.own.get_mut(&k).unwrap();
                mine.dst_keys = k as u8;
            }
            for (k, t) in part.pairs {
                let mine = total.pairs.entry(k).or_default();
                mine.merge(t);
                (mine.src_keys, mine.dst_keys) = (k.0 as u8, k.1 as u8);
            }
            add_counts(&mut total.sets, part.sets);
        },
    );

    let mut tables: Vec<StyleTable> = Vec::new();
    for (keys, mut t) in acc.own {
        if t.maps >= MIN_MAPS {
            println!("{}K: {} maps", keys, t.maps);
            t.prune();
            tables.push(t);
        }
    }
    for ((a, b), mut t) in acc.pairs {
        let sets = acc.sets.get(&(a, b)).copied().unwrap_or(0);
        if sets >= MIN_SETS as u64 {
            println!("{}K -> {}K: {} sets, {} diff pairs", a, b, sets, t.maps);
            t.prune();
            tables.push(t);
        }
    }

    let json = serde_json::to_string(&tables).unwrap();
    let path = out_dir().join("style.json");
    std::fs::write(&path, &json).unwrap();
    println!("written to {} ({} KB)", path.display(), json.len() / 1024);
    if std::env::var_os("WRITE_STYLE").is_some() {
        let dst = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands/trainer/analysis/style.json");
        std::fs::write(&dst, &json).unwrap();
        println!("written to {}", dst.display());
    }
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
        ("converted Auto", Some(Density::Auto)),
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
    let modes = [("None", Density::None), ("Full", Density::Full), ("Auto", Density::Auto)];

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
    let modes = [("hand-made 7K", None), ("4K -> 7K Full", Some(Density::Full)), ("4K -> 7K Auto", Some(Density::Auto))];
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
