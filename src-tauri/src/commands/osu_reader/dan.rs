//! Dan estimate of the selected beatmap, from Mania Tracker's calculator
//! (`dan-calc`, in aleju03/mania-hub).

use std::path::Path;

use osu_memory_reader::{BeatmapAttributes, Ruleset};
use serde::Serialize;

/// The keymodes `dan-calc` rates; it returns no dan for the others.
const SUPPORTED_KEYS: [u32; 3] = [4, 6, 7];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Family {
    Rice,
    Ln,
}

/// A dan, e.g. `6++` (6.37).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dan {
    pub family: Family,
    /// As Mania Tracker writes it: `6++`, `zeta-`.
    pub name: String,
    /// The dan as a number, for comparing and colouring.
    pub value: f64,
    /// How sure the estimator is, from 0 to 1.
    pub confidence: f64,
}

/// The dans of a beatmap at 1.0x and its own OD.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatmapDan {
    /// The one Mania Tracker shows first: the rice or the LN dan.
    pub primary: Dan,
    pub rice: Option<Dan>,
    /// A hybrid map has both a rice and an LN dan.
    pub ln: Option<Dan>,
}

/// Whether [`estimate`] can rate a map with these attributes: only 4K, 6K
/// and 7K mania maps.
pub fn supported(attributes: &BeatmapAttributes) -> bool {
    attributes.mode == Ruleset::Mania && attributes.key_count.is_some_and(|keys| SUPPORTED_KEYS.contains(&keys))
}

/// Reads `osu` and estimates its dan. `None` if it can't be read or rated.
///
/// Takes ~0.5 s the first time (it compiles MinaCalc's wasm), less after,
/// so call it once per selected beatmap, not on every poll.
pub fn estimate(osu: &Path) -> Option<BeatmapDan> {
    let bytes = std::fs::read(osu).map_err(|e| log::warn!("could not read {} for its dan: {e}", osu.display())).ok()?;
    let text = String::from_utf8_lossy(&bytes);

    let estimate = dan_calc::estimate(&text, &dan_calc::Options::default())
        .map_err(|e| log::warn!("could not estimate the dan of {}: {e}", osu.display()))
        .ok()?;

    Some(BeatmapDan { primary: dan(estimate.primary.as_ref()?), rice: estimate.rice.as_ref().map(dan), ln: estimate.ln.as_ref().map(dan) })
}

fn dan(dan: &dan_calc::Dan) -> Dan {
    let family = match dan.family {
        dan_calc::Family::Rice => Family::Rice,
        dan_calc::Family::Ln => Family::Ln,
    };
    Dan { family, name: dan.display_name.clone(), value: dan.raw_dan, confidence: dan.confidence }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;
    use std::time::Instant;

    use super::*;
    use crate::test_utils::TestDir;

    /// A `keys`K mania map: a 180 BPM 1/4 stream rolling over the columns
    /// for about a minute.
    fn stream_map(keys: u32) -> String {
        let mut osu = format!(
            "osu file format v14\n\n[General]\nMode: 3\n\n[Metadata]\nTitle:Stream\nVersion:Test\n\n\
             [Difficulty]\nHPDrainRate:8\nCircleSize:{keys}\nOverallDifficulty:8\nApproachRate:5\n\
             SliderMultiplier:1.4\nSliderTickRate:1\n\n[TimingPoints]\n1000,333.333333333333,4,2,0,100,1,0\n\n[HitObjects]\n"
        );
        for i in 0..720u32 {
            let column = i % keys;
            let x = (column * 512 + 256) / keys;
            let time = 1000.0 + f64::from(i) * 333.333_333_333_333 / 4.0;
            writeln!(osu, "{x},192,{},1,0,0:0:0:0:", time.round()).unwrap();
        }
        osu
    }

    fn write(dir: &TestDir, keys: u32) -> std::path::PathBuf {
        let name = format!("{keys}k.osu");
        dir.write(&name, &stream_map(keys));
        dir.0.join(name)
    }

    #[test]
    fn rates_4k_maps() {
        let dir = TestDir::new("dan-4k");
        let osu = write(&dir, 4);
        assert!(supported(&osu_memory_reader::attributes_from_path(&osu, None).unwrap()));

        let start = Instant::now();
        let dan = estimate(&osu).expect("a 4K stream gets a dan");
        let first = start.elapsed();
        let start = Instant::now();
        assert_eq!(estimate(&osu).as_ref(), Some(&dan));
        println!("{dan:?}\nfirst estimate {first:?}, second {:?}", start.elapsed());

        assert_eq!(dan.primary.family, Family::Rice);
        assert_eq!(dan.rice.as_ref(), Some(&dan.primary));
        assert!(dan.ln.is_none());
        assert!(dan.primary.value > 0.0 && !dan.primary.name.is_empty());
    }

    #[test]
    fn skips_other_keymodes() {
        let dir = TestDir::new("dan-5k");
        let osu = write(&dir, 5);
        assert!(!supported(&osu_memory_reader::attributes_from_path(&osu, None).unwrap()));
        assert!(estimate(&osu).is_none());
    }

    #[test]
    fn missing_file() {
        assert!(estimate(Path::new("/nonexistent/map.osu")).is_none());
    }
}
