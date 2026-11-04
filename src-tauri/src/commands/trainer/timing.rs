//! The tempo of a map, as the tasks, the naming and the analysis read it.

use rosu_map::Beatmap;

/// Timing points up to this many ms after a note still apply to it: notes a
/// couple of ms off the grid.
const GRID_LENIENCY_MS: f64 = 2.0;

/// BPM of the timing point that lasts the longest, like the one osu! shows on
/// song select. Expects the timing points to be already rate-scaled.
pub fn main_bpm(beatmap: &Beatmap) -> Option<f64> {
    let points = &beatmap.control_points.timing_points;
    let end = beatmap.hit_objects.last().map_or(0.0, |h| h.start_time);

    points
        .iter()
        .enumerate()
        .map(|(i, tp)| {
            let next = points.get(i + 1).map_or(end, |n| n.time);
            ((next - tp.time).max(0.0), tp.beat_len)
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .filter(|&(_, beat_len)| beat_len > 0.0)
        .map(|(_, beat_len)| 60_000.0 / beat_len)
}

/// Beat length (ms) of [`main_bpm`].
pub fn main_beat_len(beatmap: &Beatmap) -> Option<f64> {
    main_bpm(beatmap).map(|bpm| 60_000.0 / bpm)
}

/// Whether `beat_len` is a real tempo of a map whose main beat lasts `main`:
/// gimmick maps use fake BPMs (0.001, 100000...) that are off by far more.
pub fn is_plausible_beat_len(beat_len: f64, main: f64) -> bool {
    beat_len.is_finite() && (main / 10.0..=main * 10.0).contains(&beat_len)
}

/// The beat grid of a map: its timing points without the fake BPMs of gimmick
/// maps, as `(time, beat length)`.
pub struct BeatGrid(Vec<(f64, f64)>);

impl BeatGrid {
    pub fn new(beatmap: &Beatmap) -> Self {
        let main = main_beat_len(beatmap).unwrap_or(500.0);
        let mut points: Vec<(f64, f64)> = beatmap
            .control_points
            .timing_points
            .iter()
            .filter(|tp| is_plausible_beat_len(tp.beat_len, main))
            .map(|tp| (tp.time, tp.beat_len))
            .collect();
        if points.is_empty() {
            points.push((0.0, main));
        }
        Self(points)
    }

    /// The timing point in effect at `time`, as `(time, beat length)`.
    pub fn at(&self, time: f64) -> (f64, f64) {
        let i = self.0.partition_point(|&(t, _)| t <= time + GRID_LENIENCY_MS).saturating_sub(1);
        self.0[i]
    }

    /// Every `1 / division` of a beat from `from` to `to`, as the grid puts
    /// them (the first timing point's grid before it).
    pub fn ticks(&self, from: f64, to: f64, division: f64) -> Vec<f64> {
        let mut ticks = Vec::new();
        for (i, &(time, beat_len)) in self.0.iter().enumerate() {
            let step = beat_len / division;
            let start = if i == 0 { from } else { from.max(time) };
            let end = self.0.get(i + 1).map_or(to, |&(next, _)| to.min(next - GRID_LENIENCY_MS));
            let mut tick = time + ((start - time) / step - 1e-6).ceil() * step;
            while tick <= end + 1e-6 {
                ticks.push(tick);
                tick += step;
            }
        }
        ticks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_follow_each_timing_point() {
        // 120 BPM, then 150 BPM from 1000 ms.
        let osu = "osu file format v14\n\n[General]\nMode: 3\n\n[Difficulty]\nCircleSize:4\n\n[TimingPoints]\n0,500,4,1,0,60,1,0\n1000,400,4,1,0,60,1,0\n\n[HitObjects]\n64,192,0,1,0,0:0:0:0:\n64,192,2000,1,0,0:0:0:0:\n";
        let beatmap = rosu_map::from_str::<Beatmap>(osu).unwrap();
        let grid = BeatGrid::new(&beatmap);
        assert_eq!(grid.ticks(-300.0, 1900.0, 2.0), [-250.0, 0.0, 250.0, 500.0, 750.0, 1000.0, 1200.0, 1400.0, 1600.0, 1800.0]);
    }
}
