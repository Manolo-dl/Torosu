//! How hard a mania map is, from Mania Tracker's calculator (`dan-calc`, in
//! aleju03/mania-hub): its dan, and its MSD, from MinaCalc on 4K and from
//! Prism on 6K-8K. 10K maps get their dan from `tenk-calc` instead, on CT's
//! 10K Dan Course.

use std::path::Path;

use osu_memory_reader::{BeatmapAttributes, Ruleset};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Family {
    Rice,
    Ln,
}

/// What gave a dan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Estimator {
    /// Mania Tracker's `dan-calc`: 4K, 6K and 7K.
    ManiaTracker,
    /// `tenk-calc`: 10K, on CT's 10K Dan Course (Level 0 to 10, Expecialist,
    /// Virtuoso).
    TenkCalc,
}

/// A dan, e.g. `6++` (6.37).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dan {
    pub estimator: Estimator,
    pub family: Family,
    /// As Mania Tracker writes it: `6++`, `zeta-`; on 10K `Level 7+`,
    /// `Virtuoso-`.
    pub name: String,
    /// The dan without its variant: `6`, `zeta`, `azimuth` (7K), `terra` (6K),
    /// `expecialist` (10K).
    pub label: String,
    /// `++`, `+`, `-` or `--`; `None` for the dan itself.
    pub variant: Option<String>,
    /// The dan as a number, for comparing (on 10K the CT level: Expecialist
    /// 11, Virtuoso 12).
    pub value: f64,
    /// How sure the estimator is, from 0 to 1; `None` from `tenk-calc`,
    /// which does not say.
    pub confidence: Option<f64>,
}

/// The dans of a map. Only 4K, 6K, 7K and 10K maps have them.
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

/// What gives a map's MSD. Their numbers are on scales of their own, so two
/// MSDs only compare when the same one gives both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Calculator {
    /// Etterna's: 0.72.3 on 4K, 0.75.0 told the key count on the others.
    MinaCalc,
    /// Mania Tracker's, for the 6K, 7K and 8K MinaCalc was never built for.
    Prism,
}

impl Calculator {
    /// The one dan-calc rates `key_count` keys with (`uses_prism` in its
    /// chart-pipeline).
    fn of(key_count: i32) -> Self {
        if (6..=8).contains(&key_count) { Calculator::Prism } else { Calculator::MinaCalc }
    }
}

/// A map's MSD.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Msd {
    pub calculator: Calculator,
    pub overall: f64,
    /// Every skillset but the overall, in MinaCalc's order.
    pub skillsets: Vec<Skillset>,
}

/// What the calculator gives for a map at a rate; each part can be missing
/// (a 5K map has an MSD but no dan).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    /// Of the map rated: a convert can change it.
    pub key_count: i32,
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
        .ok();

    let mut dan = estimate.as_ref().and_then(|estimate| {
        estimate.primary.as_ref().map(|primary| Dans {
            primary: dan(primary),
            rice: estimate.rice.as_ref().map(dan),
            ln: estimate.ln.as_ref().map(dan),
        })
    });
    let mut key_count = estimate.as_ref().map(|e| e.key_count);
    // dan-calc has no 10K dans; if it could not read the map at all, it may
    // still be a 10K one.
    if dan.is_none() && key_count.is_none_or(|keys| keys == tenk_calc::KEYS as i32) {
        dan = tenk_dans(osu, rate);
        key_count = key_count.or(dan.is_some().then_some(tenk_calc::KEYS as i32));
    }
    let msd = estimate
        .as_ref()
        .and_then(|e| e.msd.as_deref().and_then(|values| msd(values, Calculator::of(e.key_count))));
    (dan.is_some() || msd.is_some()).then_some(Skill { key_count: key_count?, dan, msd })
}

/// The CT level of a 10K map, as its only dan (CT's courses are hybrid: no
/// rice and LN apart). `None` if it is not a 10K map.
fn tenk_dans(osu: &str, rate: f64) -> Option<Dans> {
    let estimate = match tenk_calc::estimate(osu, &tenk_calc::Options { rate, ..Default::default() }) {
        Ok(estimate) => estimate,
        Err(tenk_calc::Error::Keys(_) | tenk_calc::Error::NotMania) => return None,
        Err(e) => {
            log::warn!("could not rate the 10K map: {e}");
            return None;
        }
    };
    let primary = Dan {
        estimator: Estimator::TenkCalc,
        family: Family::Rice,
        name: estimate.label,
        label: estimate.dan.to_lowercase(),
        variant: estimate.variant.map(str::to_string),
        value: estimate.level,
        confidence: None,
    };
    Some(Dans { rice: Some(primary.clone()), primary, ln: None })
}

fn dan(dan: &dan_calc::Dan) -> Dan {
    let family = match dan.family {
        dan_calc::Family::Rice => Family::Rice,
        dan_calc::Family::Ln => Family::Ln,
    };
    Dan {
        estimator: Estimator::ManiaTracker,
        family,
        name: dan.display_name.clone(),
        label: dan.label.clone(),
        variant: dan.variant.map(str::to_string),
        value: dan.raw_dan,
        confidence: Some(dan.confidence),
    }
}

fn msd(values: &[(String, f64)], calculator: Calculator) -> Option<Msd> {
    let overall = values.iter().find(|(name, _)| name == "Overall").map(|(_, v)| *v).filter(|v| v.is_finite())?;
    let skillsets = values
        .iter()
        .filter(|(name, value)| name != "Overall" && value.is_finite())
        .map(|(name, value)| Skillset { name: name.clone(), value: *value })
        .collect();
    Some(Msd { calculator, overall, skillsets })
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

        assert_eq!(skill.key_count, 4);
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
    fn rates_10k_maps_on_ct() {
        let skill = estimate(&stream_map(10), 1.0).expect("a 10K stream is rated");
        assert_eq!(skill.key_count, 10);
        let dan = skill.dan.expect("10K maps have a dan");
        assert_eq!(dan.primary.estimator, Estimator::TenkCalc);
        assert_eq!(dan.primary.family, Family::Rice);
        assert_eq!(dan.rice.as_ref(), Some(&dan.primary));
        assert!(dan.ln.is_none() && dan.primary.confidence.is_none());
        let fast = estimate(&stream_map(10), 1.5).unwrap().dan.unwrap();
        assert!(fast.primary.value > dan.primary.value);
    }

    #[test]
    fn no_dan_on_5k() {
        let osu = stream_map(5);
        assert!(estimate(&osu, 1.0).is_none_or(|skill| skill.dan.is_none()));
    }

    #[test]
    fn msd_calculator_by_keymode() {
        for (keys, calculator) in [(4, Calculator::MinaCalc), (7, Calculator::Prism), (9, Calculator::MinaCalc)] {
            let msd = estimate(&stream_map(keys), 1.0).and_then(|skill| skill.msd).expect("the stream has an MSD");
            assert_eq!(msd.calculator, calculator, "{keys}K");
        }
    }

    #[test]
    fn missing_file() {
        assert!(estimate_file(Path::new("/nonexistent/map.osu")).is_none());
    }
}
