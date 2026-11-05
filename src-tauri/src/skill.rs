//! How hard a mania map is, from Mania Tracker's calculator (`dan-calc`, in
//! aleju03/mania-hub): its dan, and its MSD, from MinaCalc on 4K and from
//! Prism on 6K-8K.

use std::path::Path;

use osu_memory_reader::{BeatmapAttributes, Ruleset};
use serde::Serialize;

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
    /// The dan without its variant: `6`, `zeta`, `azimuth` (7K), `terra` (6K).
    pub label: String,
    /// `++`, `+`, `-` or `--`; `None` for the dan itself.
    pub variant: Option<String>,
    /// The dan as a number, for comparing.
    pub value: f64,
    /// How sure the estimator is, from 0 to 1.
    pub confidence: f64,
}

/// The dans of a map. Only 4K, 6K and 7K maps have them.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dans {
    /// The one Mania Tracker shows first: the rice or the LN dan.
    pub primary: Dan,
    pub rice: Option<Dan>,
    /// A hybrid map has both a rice and an LN dan.
    pub ln: Option<Dan>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Skillset {
    /// `Stream`, `Jumpstream`, `Handstream`, `Stamina`, `JackSpeed`,
    /// `Chordjack`, `Technical` or `LN`.
    pub name: String,
    pub value: f64,
}

/// A map's MSD.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Msd {
    pub overall: f64,
    /// Every skillset but the overall, in MinaCalc's order.
    pub skillsets: Vec<Skillset>,
}

/// What the calculator gives for a map at a rate; each part can be missing
/// (a 5K map has an MSD but no dan).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub dan: Option<Dans>,
    pub msd: Option<Msd>,
}

/// Whether [`estimate_file`] can rate a map with these attributes: mania maps.
pub fn supported(attributes: &BeatmapAttributes) -> bool {
    attributes.mode == Ruleset::Mania
}

/// Reads `osu` and rates it at 1.0x and its own OD. `None` if it can't be
/// read or rated.
///
/// Takes ~0.5 s the first time (it compiles MinaCalc's wasm), tens of ms
/// after, so call it once per selected beatmap, not on every poll.
pub fn estimate_file(osu: &Path) -> Option<Skill> {
    let bytes = std::fs::read(osu).map_err(|e| log::warn!("could not read {} to rate it: {e}", osu.display())).ok()?;
    estimate(&String::from_utf8_lossy(&bytes), 1.0)
}

/// Rates the `.osu` text `osu` at `rate`, at the OD in the file. `None` if it
/// can't be rated.
pub fn estimate(osu: &str, rate: f64) -> Option<Skill> {
    let estimate = dan_calc::estimate(osu, &dan_calc::Options { rate, ..Default::default() })
        .map_err(|e| log::warn!("could not rate the map: {e}"))
        .ok()?;

    let dan = estimate.primary.as_ref().map(|primary| Dans {
        primary: dan(primary),
        rice: estimate.rice.as_ref().map(dan),
        ln: estimate.ln.as_ref().map(dan),
    });
    let msd = estimate.msd.as_deref().and_then(msd);
    (dan.is_some() || msd.is_some()).then_some(Skill { dan, msd })
}

fn dan(dan: &dan_calc::Dan) -> Dan {
    let family = match dan.family {
        dan_calc::Family::Rice => Family::Rice,
        dan_calc::Family::Ln => Family::Ln,
    };
    Dan {
        family,
        name: dan.display_name.clone(),
        label: dan.label.clone(),
        variant: dan.variant.map(str::to_string),
        value: dan.raw_dan,
        confidence: dan.confidence,
    }
}

fn msd(values: &[(String, f64)]) -> Option<Msd> {
    let overall = values.iter().find(|(name, _)| name == "Overall").map(|(_, v)| *v).filter(|v| v.is_finite())?;
    let skillsets = values
        .iter()
        .filter(|(name, value)| name != "Overall" && value.is_finite())
        .map(|(name, value)| Skillset { name: name.clone(), value: *value })
        .collect();
    Some(Msd { overall, skillsets })
}

#[cfg(test)]
pub mod tests {
    use std::fmt::Write;
    use std::time::Instant;

    use super::*;
    use crate::test_utils::TestDir;

    /// A `keys`K mania map: a 180 BPM 1/4 stream rolling over the columns
    /// for about a minute.
    pub fn stream_map(keys: u32) -> String {
        let mut osu = format!(
            "osu file format v14\n\n[General]\nAudioFilename: audio.mp3\nMode: 3\n\n[Metadata]\nTitle:Stream\nArtist:Test\n\
             Creator:Test\nVersion:Test\n\n[Difficulty]\nHPDrainRate:8\nCircleSize:{keys}\nOverallDifficulty:8\n\
             ApproachRate:5\nSliderMultiplier:1.4\nSliderTickRate:1\n\n[TimingPoints]\n1000,333.333333333333,4,2,0,100,1,0\n\n\
             [HitObjects]\n"
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
        let dir = TestDir::new("skill-4k");
        let osu = write(&dir, 4);
        assert!(supported(&osu_memory_reader::attributes_from_path(&osu, None).unwrap()));

        let start = Instant::now();
        let skill = estimate_file(&osu).expect("a 4K stream is rated");
        let first = start.elapsed();
        let start = Instant::now();
        assert_eq!(estimate_file(&osu).as_ref(), Some(&skill));
        println!("{skill:?}\nfirst estimate {first:?}, second {:?}", start.elapsed());

        let dan = skill.dan.expect("4K maps have a dan");
        assert_eq!(dan.primary.family, Family::Rice);
        assert_eq!(dan.rice.as_ref(), Some(&dan.primary));
        assert!(dan.ln.is_none());
        assert!(dan.primary.value > 0.0 && !dan.primary.name.is_empty());
        assert_eq!(dan.primary.name, format!("{}{}", dan.primary.label, dan.primary.variant.as_deref().unwrap_or("")));

        let msd = skill.msd.expect("4K maps have an MSD");
        assert!(msd.overall > 0.0);
        assert!(msd.skillsets.iter().any(|s| s.name == "Stream"));
        assert!(msd.skillsets.iter().all(|s| s.name != "Overall"));
    }

    #[test]
    fn harder_at_a_higher_rate() {
        let osu = stream_map(4);
        let normal = estimate(&osu, 1.0).unwrap();
        let fast = estimate(&osu, 1.3).unwrap();
        assert!(fast.msd.unwrap().overall > normal.msd.unwrap().overall);
        assert!(fast.dan.unwrap().primary.value > normal.dan.unwrap().primary.value);
    }

    #[test]
    fn no_dan_on_5k() {
        let osu = stream_map(5);
        assert!(estimate(&osu, 1.0).is_none_or(|skill| skill.dan.is_none()));
    }

    #[test]
    fn missing_file() {
        assert!(estimate_file(Path::new("/nonexistent/map.osu")).is_none());
    }
}
