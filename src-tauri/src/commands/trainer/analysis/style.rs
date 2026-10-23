//! The style of hand-made maps, learned from a Songs folder (see `corpus`):
//! for each pattern of the source, how big the rows of the target are, how
//! many keys they share with the rows before them and how they split between
//! the hands.
//!
//! A table learned from sets with both key counts (a 4K and a 7K diff of the
//! same song) says what mappers make of each 4K pattern on 7K. Where there are
//! not enough such sets, a table learned from the target maps alone says what
//! each pattern looks like on that key count.
//!
//! Row sizes are a distribution, not a choice: always picking the likeliest
//! size would turn every 4K stream into a 7K stream, while mappers make a
//! quarter of its rows jumps. So each row draws its size (see
//! [`Style::quota`]), and the conversion keeps close to it. Without sets of
//! both key counts, the row keeps its own size times how much bigger the
//! pattern's rows are on the target than on the source.

use std::{collections::BTreeMap, sync::OnceLock};

use serde::{Deserialize, Serialize};

use super::{
    layout::layout,
    patterns::{Labelled, Pattern, link_ms, MAX_KEYS},
    zones::Zone,
};

/// The learned tables, written by `corpus_style`.
const TABLES: &str = include_str!("style.json");

const SIZES: usize = MAX_KEYS + 1;
/// Sizes the conversion looks up: up to 10K.
const SMALL: usize = 11;
/// Beat positions (see `Row::beat`).
const BEATS: usize = 4;
/// Counts below this fall back to a less specific entry.
const MIN_COUNT: u32 = 40;
/// Cap of the cost of something never seen.
const MAX_COST: f64 = 8.0;

/// Counts learned from maps, as stored in `style.json`. Keys are
/// `pattern|source size|zone|beat` for sizes and `pattern|size` for the rest,
/// with `*` for the totals.
#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleTable {
    /// The key count of the source maps; 0 when learned from the target maps
    /// alone.
    pub src_keys: u8,
    pub dst_keys: u8,
    pub maps: u32,
    pub sizes: BTreeMap<String, Vec<u32>>,
    /// Keys shared with the previous row / the row two back, in a phrase.
    pub overlap1: BTreeMap<String, Vec<u32>>,
    pub overlap2: BTreeMap<String, Vec<u32>>,
    /// Notes per hand, as a [`split_code`].
    pub splits: BTreeMap<String, BTreeMap<u32, u32>>,
}

/// The notes of each hand, as one number.
pub fn split_code(mask: u32, groups: &[u32]) -> u32 {
    groups.iter().rev().fold(0, |code, g| code * SIZES as u32 + (mask & g).count_ones())
}

fn bump(map: &mut BTreeMap<String, Vec<u32>>, key: String, value: usize) {
    let v = map.entry(key).or_insert_with(|| vec![0; SIZES]);
    v[value.min(SIZES - 1)] += 1;
}

impl StyleTable {
    /// Adds row `j` of `dst`, the target of a row of pattern `pattern` with
    /// `src_size` notes (0 when learning from the target alone) in `zone`, on
    /// `beat`.
    #[allow(clippy::too_many_arguments)]
    pub fn add_row(&mut self, pattern: Pattern, src_size: u32, zone: Zone, beat: u8, dst: &Labelled, j: usize, groups: &[u32]) {
        let row = dst.rows[j];
        let size = row.mask.count_ones() as usize;
        let p = format!("{:?}", pattern);
        let z = format!("{:?}", zone);

        for key in [
            format!("{}|{}|{}|{}", p, src_size, z, beat),
            format!("{}|{}|{}|*", p, src_size, z),
            format!("{}|{}|*|*", p, src_size),
            format!("{}|*|*|*", p),
            "*|*|*|*".into(),
        ] {
            bump(&mut self.sizes, key, size);
        }

        let linked = |a: usize, b: usize| dst.rows[b].time - dst.rows[a].time <= link_ms(dst.rows[b].beat_len);
        let keys = [format!("{}|{}", p, size), format!("*|{}", size)];
        if j >= 1 && linked(j - 1, j) {
            let shared = (dst.rows[j - 1].mask & row.mask).count_ones() as usize;
            for key in &keys {
                bump(&mut self.overlap1, key.clone(), shared);
            }
            if j >= 2 && linked(j - 2, j - 1) {
                let shared = (dst.rows[j - 2].mask & row.mask).count_ones() as usize;
                for key in &keys {
                    bump(&mut self.overlap2, key.clone(), shared);
                }
            }
        }
        let code = split_code(row.mask, groups);
        for key in keys {
            *self.splits.entry(key).or_default().entry(code).or_insert(0) += 1;
        }
    }

    pub fn merge(&mut self, b: StyleTable) {
        self.maps += b.maps;
        let add = |a: &mut BTreeMap<String, Vec<u32>>, b: BTreeMap<String, Vec<u32>>| {
            for (k, v) in b {
                let mine = a.entry(k).or_insert_with(|| vec![0; SIZES]);
                for (x, y) in mine.iter_mut().zip(v) {
                    *x += y;
                }
            }
        };
        add(&mut self.sizes, b.sizes);
        add(&mut self.overlap1, b.overlap1);
        add(&mut self.overlap2, b.overlap2);
        for (k, v) in b.splits {
            let mine = self.splits.entry(k).or_default();
            for (code, c) in v {
                *mine.entry(code).or_insert(0) += c;
            }
        }
    }

    /// Drops entries too rare to be used, so the stored file stays small.
    pub fn prune(&mut self) {
        let total = |v: &Vec<u32>| v.iter().sum::<u32>();
        self.sizes.retain(|_, v| total(v) >= MIN_COUNT);
        self.overlap1.retain(|_, v| total(v) >= MIN_COUNT);
        self.overlap2.retain(|_, v| total(v) >= MIN_COUNT);
        self.splits.retain(|_, v| v.values().sum::<u32>() >= MIN_COUNT);
        for v in self.splits.values_mut() {
            v.retain(|_, c| *c >= 2);
        }
    }
}

fn tables() -> &'static [StyleTable] {
    static TABLES_CELL: OnceLock<Vec<StyleTable>> = OnceLock::new();
    TABLES_CELL.get_or_init(|| serde_json::from_str(TABLES).unwrap_or_default())
}

/// Costs (negative log probabilities) from a [`StyleTable`], ready to be
/// looked up in the conversion.
pub struct Style {
    groups: Vec<u32>,
    dst_keys: usize,
    /// Indexed by pattern, source size, zone and beat; the probability of
    /// each row size.
    sizes: Vec<[f64; SMALL]>,
    /// Indexed by pattern and size; a cost per shared key count.
    overlap1: Vec<[f64; SIZES]>,
    overlap2: Vec<[f64; SIZES]>,
    /// Indexed by pattern and size: the cost of each split, and of any other.
    splits: Vec<(BTreeMap<u32, f64>, f64)>,
    /// Without sets of both key counts: indexed by pattern, zone and beat, the
    /// mean row size on the target against the source.
    ratios: Option<Vec<f64>>,
}

fn pattern_index(p: Pattern) -> usize {
    Pattern::ALL.iter().position(|&x| x == p).unwrap_or(0)
}

fn zone_index(z: Zone) -> usize {
    Zone::ALL.iter().position(|&x| x == z).unwrap_or(0)
}

/// Probabilities of a histogram over `bins` values, smoothed.
fn probs(counts: &[u32], bins: usize) -> [f64; SMALL] {
    let total: u32 = counts.iter().take(bins).sum();
    let mut out = [0.0; SMALL];
    for (i, p) in out.iter_mut().enumerate().take(bins) {
        *p = (counts.get(i).copied().unwrap_or(0) as f64 + 0.5) / (total as f64 + 0.5 * bins as f64);
    }
    out
}

/// Costs of a histogram over `bins` values, smoothed.
fn costs(counts: &[u32], bins: usize) -> [f64; SIZES] {
    let total: u32 = counts.iter().take(bins).sum();
    let mut out = [MAX_COST; SIZES];
    for (i, cost) in out.iter_mut().enumerate().take(bins) {
        let p = (counts.get(i).copied().unwrap_or(0) as f64 + 0.5) / (total as f64 + 0.5 * bins as f64);
        *cost = (-p.ln()).min(MAX_COST);
    }
    out
}

impl Style {
    /// The style for converting `src_keys` to `dst_keys`: learned from sets
    /// with both, or else from `dst_keys` maps alone.
    pub fn load(src_keys: usize, dst_keys: usize) -> Option<Style> {
        let tables = tables();
        let own = |keys: usize| tables.iter().find(|t| t.src_keys == 0 && t.dst_keys as usize == keys);
        if let Some(pair) = tables.iter().find(|t| t.src_keys as usize == src_keys && t.dst_keys as usize == dst_keys) {
            return Some(Style::from_table(pair));
        }
        let mut style = Style::from_table(own(dst_keys)?);
        let source = Style::from_table(own(src_keys)?);
        let mean = |p: &[f64; SMALL]| p.iter().enumerate().map(|(i, x)| i as f64 * x).sum::<f64>() / p.iter().sum::<f64>().max(1e-9);
        let mut ratios = Vec::with_capacity(Pattern::ALL.len() * Zone::ALL.len() * BEATS);
        for p in Pattern::ALL {
            for z in Zone::ALL {
                for b in 0..BEATS as u8 {
                    let from = mean(source.size_probs(p, 0, z, b));
                    let to = mean(style.size_probs(p, 0, z, b));
                    ratios.push(if from > 0.0 { to / from } else { 1.0 });
                }
            }
        }
        style.ratios = Some(ratios);
        Some(style)
    }

    pub fn from_table(table: &StyleTable) -> Style {
        let dst_keys = table.dst_keys as usize;
        let bins = dst_keys + 1;
        let groups = layout(dst_keys, None).groups().iter().map(|g| g.mask).collect();
        let found = |map: &BTreeMap<String, Vec<u32>>, keys: &[String]| -> Option<Vec<u32>> {
            keys.iter().filter_map(|k| map.get(k)).find(|v| v.iter().sum::<u32>() >= MIN_COUNT).cloned()
        };

        let bins_small = bins.min(SMALL);
        let mut sizes = Vec::with_capacity(Pattern::ALL.len() * SMALL * Zone::ALL.len() * BEATS);
        for p in Pattern::ALL {
            for s in 0..SMALL {
                for z in Zone::ALL {
                    for b in 0..BEATS {
                        let keys = [
                            format!("{:?}|{}|{:?}|{}", p, s, z, b),
                            format!("{:?}|{}|{:?}|*", p, s, z),
                            format!("{:?}|{}|*|*", p, s),
                            format!("{:?}|*|*|*", p),
                            "*|*|*|*".to_string(),
                        ];
                        let counts = found(&table.sizes, &keys).unwrap_or_default();
                        sizes.push(probs(&counts, bins_small));
                    }
                }
            }
        }

        let by_size = |map: &BTreeMap<String, Vec<u32>>| -> Vec<[f64; SIZES]> {
            let mut out = Vec::with_capacity(Pattern::ALL.len() * SIZES);
            for p in Pattern::ALL {
                for size in 0..SIZES {
                    let keys = [format!("{:?}|{}", p, size), format!("*|{}", size)];
                    let counts = found(map, &keys).unwrap_or_default();
                    out.push(costs(&counts, (size + 1).min(bins)));
                }
            }
            out
        };

        let mut splits = Vec::with_capacity(Pattern::ALL.len() * SIZES);
        for p in Pattern::ALL {
            for size in 0..SIZES {
                let counts = [format!("{:?}|{}", p, size), format!("*|{}", size)]
                    .iter()
                    .filter_map(|k| table.splits.get(k))
                    .find(|v| v.values().sum::<u32>() >= MIN_COUNT);
                let entry = match counts {
                    Some(counts) => {
                        let total = counts.values().sum::<u32>() as f64 + 1.0;
                        let costs = counts.iter().map(|(&code, &c)| (code, (-(c as f64 / total).ln()).min(MAX_COST))).collect();
                        (costs, (-(1.0 / total).ln()).min(MAX_COST))
                    }
                    None => (BTreeMap::new(), 0.0),
                };
                splits.push(entry);
            }
        }

        Style {
            groups,
            dst_keys,
            sizes,
            overlap1: by_size(&table.overlap1),
            overlap2: by_size(&table.overlap2),
            splits,
            ratios: None,
        }
    }

    fn size_probs(&self, pattern: Pattern, src_size: usize, zone: Zone, beat: u8) -> &[f64; SMALL] {
        let i = ((pattern_index(pattern) * SMALL + src_size.min(SMALL - 1)) * Zone::ALL.len() + zone_index(zone)) * BEATS
            + (beat as usize).min(BEATS - 1);
        &self.sizes[i]
    }

    /// Cost of a target row of `size` notes for a source row of `pattern` with
    /// `src_size` notes in `zone`, on `beat`.
    pub fn size_cost(&self, pattern: Pattern, src_size: usize, zone: Zone, beat: u8, size: usize) -> f64 {
        let p = self.size_probs(pattern, src_size, zone, beat)[size.min(self.dst_keys).min(SMALL - 1)];
        (-p.ln()).min(MAX_COST)
    }

    /// The row size drawn for `u` in `[0, 1)`: sizes come out as often as in
    /// the maps the style was learned from.
    pub fn quota(&self, pattern: Pattern, src_size: usize, zone: Zone, beat: u8, u: f64) -> usize {
        if let Some(ratios) = &self.ratios {
            let i = (pattern_index(pattern) * Zone::ALL.len() + zone_index(zone)) * BEATS + (beat as usize).min(BEATS - 1);
            let target = src_size as f64 * ratios[i];
            let size = target.floor() as usize + usize::from(u < target.fract());
            return size.clamp(1, self.dst_keys);
        }
        let probs = self.size_probs(pattern, src_size, zone, beat);
        let total: f64 = probs.iter().skip(1).sum();
        let mut acc = 0.0;
        for (size, p) in probs.iter().enumerate().skip(1) {
            acc += p / total.max(1e-9);
            if u < acc {
                return size;
            }
        }
        self.dst_keys.min(SMALL - 1)
    }

    /// Cost of a target row of `size` notes sharing `shared` keys with the row
    /// `lag` (1 or 2) rows back.
    pub fn overlap_cost(&self, lag: usize, pattern: Pattern, size: usize, shared: usize) -> f64 {
        let table = if lag == 1 { &self.overlap1 } else { &self.overlap2 };
        table[pattern_index(pattern) * SIZES + size.min(SIZES - 1)][shared.min(SIZES - 1)]
    }

    /// Cost of how a target row splits between the hands.
    pub fn split_cost(&self, pattern: Pattern, mask: u32) -> f64 {
        let size = (mask.count_ones() as usize).min(SIZES - 1);
        let (costs, unseen) = &self.splits[pattern_index(pattern) * SIZES + size];
        costs.get(&split_code(mask, &self.groups)).copied().unwrap_or(*unseen)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{layout::layout, patterns::label_rows, patterns::Row};
    use super::*;

    #[test]
    fn learns_what_it_sees() {
        // Jumps on one hand, the same split every time.
        let rows: Vec<Row> = (0..200)
            .map(|i| Row {
                time: i as f64 * 125.0,
                beat_len: 500.0,
                mask: if i % 2 == 0 { 0b0011 } else { 0b1100 },
                ln: 0,
                held: 0,
                inverse: 0,
                beat: 0,
            })
            .collect();
        let layout = layout(4, None);
        let groups: Vec<u32> = layout.groups().iter().map(|g| g.mask).collect();
        let l = label_rows(rows, &layout);
        let mut table = StyleTable { dst_keys: 4, ..Default::default() };
        for j in 0..l.rows.len() {
            table.add_row(l.labels[j].pattern, 0, l.zones[j], 0, &l, j, &groups);
        }
        let style = Style::from_table(&table);
        let p = Pattern::Jumptrill;
        assert!(style.size_cost(p, 0, Zone::Normal, 0, 2) < style.size_cost(p, 0, Zone::Normal, 0, 3));
        assert_eq!(style.quota(p, 0, Zone::Normal, 0, 0.5), 2);
        assert!(style.overlap_cost(2, p, 2, 2) < style.overlap_cost(2, p, 2, 0));
        assert!(style.split_cost(p, 0b0011) < style.split_cost(p, 0b0101));
    }
}
