//! Density zones: where a map rests and where it peaks, against its own
//! density, so a conversion can be denser where the map already is and leave
//! the calm parts calm.

use serde::{Deserialize, Serialize};

use super::patterns::Row;

/// The density of a row: notes this many beats around it.
const WINDOW_BEATS: f64 = 2.0;
/// Zone limits, against the median density of the map.
const BREAK: f64 = 0.25;
const CALM: f64 = 0.6;
const DENSE: f64 = 1.25;
const PEAK: f64 = 1.6;
/// Zone sections shorter than this many rows are not listed.
const SECTION_MIN_ROWS: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum Zone {
    Break,
    Calm,
    Normal,
    Dense,
    Peak,
}

impl Zone {
    pub const ALL: [Zone; 5] = [Zone::Break, Zone::Calm, Zone::Normal, Zone::Dense, Zone::Peak];
}

/// Notes per second around each row.
pub fn density(rows: &[Row]) -> Vec<f64> {
    let (mut lo, mut hi, mut notes) = (0, 0, 0u32);
    rows.iter()
        .map(|row| {
            let half = row.beat_len * WINDOW_BEATS;
            while hi < rows.len() && rows[hi].time <= row.time + half {
                notes += rows[hi].mask.count_ones();
                hi += 1;
            }
            while rows[lo].time < row.time - half {
                notes -= rows[lo].mask.count_ones();
                lo += 1;
            }
            notes as f64 * 1000.0 / (2.0 * half)
        })
        .collect()
}

pub fn zones(rows: &[Row]) -> Vec<Zone> {
    let density = density(rows);
    let mut sorted = density.clone();
    sorted.sort_by(f64::total_cmp);
    let Some(&median) = sorted.get(sorted.len() / 2) else { return Vec::new() };

    density
        .iter()
        .map(|&d| {
            let x = d / median.max(1e-9);
            if x < BREAK {
                Zone::Break
            } else if x < CALM {
                Zone::Calm
            } else if x < DENSE {
                Zone::Normal
            } else if x < PEAK {
                Zone::Dense
            } else {
                Zone::Peak
            }
        })
        .collect()
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoneSection {
    pub zone: Zone,
    pub start: f64,
    pub end: f64,
    pub rows: usize,
}

pub fn sections(rows: &[Row], zones: &[Zone]) -> Vec<ZoneSection> {
    let mut out = Vec::new();
    let mut start = 0;
    for i in 1..=rows.len() {
        if i < rows.len() && zones[i] == zones[start] {
            continue;
        }
        if i - start >= SECTION_MIN_ROWS {
            out.push(ZoneSection { zone: zones[start], start: rows[start].time, end: rows[i - 1].time, rows: i - start });
        }
        start = i;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(time: f64, notes: u32) -> Row {
        Row { time, beat_len: 500.0, mask: (1 << notes) - 1, ln: 0, held: 0, inverse: 0, beat: 0 }
    }

    #[test]
    fn finds_rests_and_peaks() {
        // 1/2s, then 1/4 jumps, then 1/4 quads, then a note every 2 beats.
        let mut rows = Vec::new();
        rows.extend((0..32).map(|i| row(i as f64 * 250.0, 1)));
        rows.extend((0..64).map(|i| row(8000.0 + i as f64 * 125.0, 2)));
        rows.extend((0..64).map(|i| row(16000.0 + i as f64 * 125.0, 4)));
        rows.extend((0..8).map(|i| row(24000.0 + i as f64 * 1000.0, 1)));
        let z = zones(&rows);
        assert_eq!(z[16], Zone::Calm);
        assert_eq!(z[32 + 32], Zone::Normal);
        assert_eq!(z[96 + 32], Zone::Peak);
        assert_eq!(z[164], Zone::Break);
    }
}
