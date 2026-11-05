//! The accents of a song: how strongly a sound starts at each moment of its
//! audio, and how loud it is, every 10 ms. The key converter puts the notes
//! that mark accents (the scratches of 10K2S) where the music has them.
//!
//! A sound starting is measured as the spectral flux: how much louder each
//! frequency gets from one frame to the next, compared with the loudest of
//! it and the frequencies next to it a little before (SuperFlux), so a
//! vibrato or a slide does not count as a new sound. It is measured in three
//! bands, kicks and bass (30 to 200 Hz), the middle (200 Hz to 2 kHz) and
//! cymbals and hats (2 to 11 kHz): over every frequency at once, the many
//! high ones drowned the kicks. Against hand-made 10K2S maps, the bands and
//! the loudness tell where their mappers hit a scratch (see
//! `corpus_scratch_sounds`).

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::SystemTime,
};

use anyhow::Result;
use symphonia::core::dsp::{complex::Complex, fft::Fft};

use super::audio_processing::{self, DecodedAudio};

/// A frame every this many ms.
const HOP_MS: f64 = 10.0;
/// Each frame looks at about this much audio: the nearest power of two of
/// samples.
const WINDOW_MS: f64 = 93.0;
/// The frequencies looked at, in Hz.
const LOW_HZ: f64 = 30.0;
const HIGH_HZ: f64 = 11_000.0;
/// A frame is compared with the one this many frames before.
const LAG_FRAMES: usize = 2;
/// The magnitudes count as log(1 + this x), so quiet sounds count too.
const COMPRESSION: f32 = 100.0;
/// An onset counts for a time this many ms around it: osu! rounds the notes
/// to the ms, and the flux of a sound peaks a frame or two after it starts.
const ONSET_MS: f64 = 30.0;
/// The edges of the bands (low, mid, high), in Hz.
const BANDS_HZ: [f64; 4] = [LOW_HZ, 200.0, 2000.0, HIGH_HZ];

pub struct Accents {
    /// The length of a frame in ms; frame `i` is centred on `i * hop_ms`.
    hop_ms: f64,
    /// Per frame, the loudness in dB.
    loudness: Vec<f32>,
    /// Per frame, how strongly sounds start in each band (the spectral flux,
    /// see [`BANDS_HZ`]).
    bands: Vec<[f32; 3]>,
}

impl Accents {
    pub fn read(path: &Path) -> Result<Self> {
        Ok(Self::new(&audio_processing::decode_audio(path)?))
    }

    /// [`read`](Self::read), kept for the last audio read: the preview of the
    /// converts asks again on every change of a task, and the converts made
    /// after it are of the same song. `None` if it can't be read.
    pub fn cached(path: &Path) -> Arc<Option<Accents>> {
        // The file as it was: a song replaced in its folder is read again.
        type Key = (PathBuf, u64, Option<SystemTime>);
        static CACHE: Mutex<Option<(Key, Arc<Option<Accents>>)>> = Mutex::new(None);

        let meta = std::fs::metadata(path).ok();
        let key: Key = (path.to_path_buf(), meta.as_ref().map_or(0, |m| m.len()), meta.and_then(|m| m.modified().ok()));
        if let Some((cached, accents)) = CACHE.lock().unwrap().as_ref() {
            if *cached == key {
                return accents.clone();
            }
        }

        let accents = Arc::new(
            Self::read(path).inspect_err(|e| log::warn!("could not read the accents of {path:?}: {e}")).ok(),
        );
        *CACHE.lock().unwrap() = Some((key, accents.clone()));
        accents
    }

    pub fn new(audio: &DecodedAudio) -> Self {
        let channels = audio.channels.max(1);
        let mono: Vec<f32> =
            audio.samples.chunks_exact(channels).map(|frame| frame.iter().sum::<f32>() / channels as f32).collect();
        Self::from_mono(&mono, audio.sample_rate as f64)
    }

    pub(crate) fn from_mono(samples: &[f32], sample_rate: f64) -> Self {
        let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
        Self::from_mono_on(samples, sample_rate, threads)
    }

    /// [`from_mono`](Self::from_mono) on up to `threads` threads, each over a
    /// part of the song: the frames don't depend on each other but for the
    /// [`LAG_FRAMES`] before them, which each part works out first. The
    /// result is the same, to the bit, on any number.
    fn from_mono_on(samples: &[f32], sample_rate: f64, threads: usize) -> Self {
        let analysis = Analysis::new(sample_rate);
        let frames = samples.len() / analysis.hop + 1;
        // Not worth a thread below this many frames (10 s of song).
        let parts = threads.min(frames / 1000).max(1);
        let size = frames.div_ceil(parts);

        let (mut loudness, mut bands) = (Vec::with_capacity(frames), Vec::with_capacity(frames));
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..parts)
                .map(|part| {
                    let analysis = &analysis;
                    let range = part * size..((part + 1) * size).min(frames);
                    scope.spawn(move || analysis.frames(samples, range))
                })
                .collect();
            for handle in handles {
                let (part_loudness, part_bands) = handle.join().expect("accents thread panicked");
                loudness.extend(part_loudness);
                bands.extend(part_bands);
            }
        });

        Self { hop_ms: analysis.hop as f64 / sample_rate * 1000.0, loudness, bands }
    }

    fn frame(&self, ms: f64) -> isize {
        (ms / self.hop_ms).round() as isize
    }

    /// The frames from `from` to `to` ms, both included, within the song.
    fn frames(&self, from: f64, to: f64) -> std::ops::Range<usize> {
        let len = self.bands.len() as isize;
        let a = self.frame(from).clamp(0, len) as usize;
        let b = (self.frame(to) + 1).clamp(0, len) as usize;
        a..b.max(a)
    }

    /// How strongly a sound starts at `ms` in each band (low, mid, high, see
    /// [`BANDS_HZ`]): the strongest onset around it.
    pub fn band_onsets(&self, ms: f64) -> [f32; 3] {
        self.bands[self.frames(ms - ONSET_MS, ms + ONSET_MS)].iter().fold([0.0f32; 3], |most, band| {
            [most[0].max(band[0]), most[1].max(band[1]), most[2].max(band[2])]
        })
    }

    /// How loud the song is from `from` to `to` ms, in dB; `None` outside it.
    pub fn loudness(&self, from: f64, to: f64) -> Option<f32> {
        let frames = &self.loudness[self.frames(from, to)];
        (!frames.is_empty()).then(|| frames.iter().sum::<f32>() / frames.len() as f32)
    }

    /// The accents for a map whose times are the audio's divided by `rate`
    /// (a rate change made before).
    pub fn at_rate(&self, rate: f64) -> MapAccents<'_> {
        MapAccents { accents: self, rate }
    }
}

/// What the frames of a song at a sample rate are worked out with.
struct Analysis {
    /// Samples per window, a power of two.
    n: usize,
    /// Samples from a frame to the next.
    hop: usize,
    bins: usize,
    /// The bins of each band, and the ones the loudness is measured over.
    edges: [usize; 4],
    audible: usize,
    scale: f32,
    window: Vec<f32>,
}

impl Analysis {
    fn new(sample_rate: f64) -> Self {
        let n = 1usize << (WINDOW_MS / 1000.0 * sample_rate).log2().round().clamp(6.0, 16.0) as u32;
        let hop = (sample_rate * HOP_MS / 1000.0).round().max(1.0) as usize;
        let bin_hz = sample_rate / n as f64;
        let bins = n / 2 + 1;
        let edges = BANDS_HZ.map(|hz| ((hz / bin_hz).ceil() as usize).clamp(1, bins));
        let audible = ((HIGH_HZ / bin_hz).ceil() as usize).clamp(1, bins);
        // As loud as a window of 2048 samples would make it, whatever the size.
        let scale = 2048.0 / n as f32;
        let window: Vec<f32> =
            (0..n).map(|i| (0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos()) as f32).collect();
        Self { n, hop, bins, edges, audible, scale, window }
    }

    /// The loudness and the band onsets of the frames in `range`.
    fn frames(&self, samples: &[f32], range: std::ops::Range<usize>) -> (Vec<f32>, Vec<[f32; 3]>) {
        let Self { n, hop, bins, edges, audible, scale, ref window } = *self;
        let mut fft = Fft::new(n);
        let mut buffer = vec![Complex::new(0.0f32, 0.0); n];
        let mut spectrum = vec![0.0f32; bins];
        // The spectra of the last frames, each frequency the loudest of it and
        // its neighbours: frame `f` in `past[f % LAG_FRAMES]`.
        let mut past = vec![vec![0.0f32; bins]; LAG_FRAMES];
        let mut loudness = Vec::with_capacity(range.len());
        let mut bands = Vec::with_capacity(range.len());

        // Of the frames before the range, only their spectra are needed.
        for frame in range.start.saturating_sub(LAG_FRAMES)..range.end {
            // Centred on its time: half a window before it, half after.
            let start = (frame * hop) as isize - (n / 2) as isize;
            buffer.fill(Complex::new(0.0, 0.0));
            let first = (-start).max(0) as usize;
            let last = (samples.len() as isize - start).clamp(0, n as isize) as usize;
            for i in first..last {
                buffer[i].re = samples[(start + i as isize) as usize] * window[i];
            }
            fft.fft_inplace(&mut buffer);

            let mut power = 0.0f64;
            for (k, value) in spectrum.iter_mut().enumerate() {
                let magnitude = buffer[k].norm() * scale;
                if k < audible {
                    power += (magnitude * magnitude) as f64;
                }
                *value = (COMPRESSION * magnitude).ln_1p();
            }

            // The slot of this frame holds the one LAG_FRAMES before it.
            let before = &mut past[frame % LAG_FRAMES];
            if frame >= range.start {
                loudness.push((10.0 * (power / audible as f64 + 1e-10).log10()) as f32);
                let mut band = [0.0f32; 3];
                if frame >= LAG_FRAMES {
                    for (b, flux) in band.iter_mut().enumerate() {
                        let (lo, hi) = (edges[b], edges[b + 1]);
                        *flux = (lo..hi).map(|k| (spectrum[k] - before[k]).max(0.0)).sum::<f32>() / (hi - lo).max(1) as f32;
                    }
                }
                bands.push(band);
            }
            for (k, value) in before.iter_mut().enumerate() {
                *value = spectrum[k].max(spectrum[k.saturating_sub(1)]).max(spectrum[(k + 1).min(bins - 1)]);
            }
        }
        (loudness, bands)
    }
}

/// The accents of a map's audio, in the map's times.
#[derive(Clone, Copy)]
pub struct MapAccents<'a> {
    accents: &'a Accents,
    rate: f64,
}

impl MapAccents<'_> {
    pub fn loudness(&self, from: f64, to: f64) -> Option<f32> {
        self.accents.loudness(from * self.rate, to * self.rate)
    }

    pub fn band_onsets(&self, time: f64) -> [f32; 3] {
        self.accents.band_onsets(time * self.rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether `a` and `b` are the same to the bit.
    fn same(a: &Accents, b: &Accents) -> bool {
        a.hop_ms == b.hop_ms
            && a.loudness.len() == b.loudness.len()
            && a.loudness.iter().zip(&b.loudness).all(|(x, y)| x.to_bits() == y.to_bits())
            && a.bands.len() == b.bands.len()
            && a.bands.iter().zip(&b.bands).all(|(x, y)| x.iter().zip(y).all(|(p, q)| p.to_bits() == q.to_bits()))
    }

    #[test]
    fn same_on_any_number_of_threads() {
        // 30 s of noise bursts of every loudness: 3000 frames, up to 3 parts.
        let sample_rate = 44_100.0;
        let mut seed = 7u32;
        let samples: Vec<f32> = (0..(sample_rate * 30.0) as usize)
            .map(|i| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let burst = (i / 4410) % 3 == 0;
                if burst { ((seed >> 8) as f32 / (1 << 24) as f32 - 0.5) * ((i / 44_100) as f32 / 30.0) } else { 0.0 }
            })
            .collect();
        let one = Accents::from_mono_on(&samples, sample_rate, 1);
        assert_eq!(one.bands.len(), samples.len() / 441 + 1);
        for threads in [2, 3, 7] {
            assert!(same(&one, &Accents::from_mono_on(&samples, sample_rate, threads)), "{threads} threads");
        }
    }

    /// One second of silence, then a click every half second: a burst of
    /// noise 20 ms long, louder each time.
    fn clicks(sample_rate: f64) -> Vec<f32> {
        let mut samples = vec![0.0f32; (sample_rate * 4.0) as usize];
        let mut seed = 1u32;
        for (c, start) in [1.0, 1.5, 2.0, 2.5, 3.0].iter().enumerate() {
            let first = (start * sample_rate) as usize;
            for sample in &mut samples[first..first + (0.02 * sample_rate) as usize] {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                *sample = ((seed >> 8) as f32 / (1 << 24) as f32 - 0.5) * 0.2 * (c + 1) as f32;
            }
        }
        samples
    }

    #[test]
    fn finds_sounds_where_they_start() {
        for sample_rate in [22_050.0, 44_100.0, 48_000.0] {
            let accents = Accents::from_mono(&clicks(sample_rate), sample_rate);
            // The clicks are noise: they start in every band.
            let onset = |t: f64| accents.band_onsets(t).into_iter().fold(f32::INFINITY, f32::min);
            let quiet = accents.band_onsets(700.0).into_iter().fold(0.0, f32::max);
            let onsets: Vec<f32> = [1000.0, 1500.0, 2000.0, 2500.0, 3000.0].iter().map(|&t| onset(t)).collect();
            assert!(onsets.iter().all(|&o| o > quiet * 10.0 + 1e-3), "{} Hz: {:?} against {}", sample_rate, onsets, quiet);
            // Between the clicks nothing starts.
            let between = accents.band_onsets(1250.0).into_iter().fold(0.0, f32::max);
            assert!(between < onsets[0] / 4.0, "{} Hz", sample_rate);
            // The louder clicks are louder.
            let loud = |t: f64| accents.loudness(t, t + 20.0).unwrap();
            assert!(loud(3000.0) > loud(1000.0), "{} Hz", sample_rate);
            assert!(loud(1000.0) > loud(700.0) + 20.0, "{} Hz", sample_rate);
        }
    }

    #[test]
    fn tells_kicks_from_hats() {
        // A kick (a 60 Hz tone) at 1 s and a hat (noise through a crude high
        // pass, each sample minus the last) at 2 s.
        let sample_rate = 44_100.0;
        let mut samples = vec![0.0f32; (sample_rate * 3.0) as usize];
        for (i, sample) in samples[(sample_rate as usize)..(sample_rate * 1.1) as usize].iter_mut().enumerate() {
            *sample = (2.0 * std::f64::consts::PI * 60.0 * i as f64 / sample_rate).sin() as f32 * 0.5;
        }
        let (mut seed, mut last) = (5u32, 0.0f32);
        for sample in &mut samples[(sample_rate * 2.0) as usize..(sample_rate * 2.03) as usize] {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let noise = (seed >> 8) as f32 / (1 << 24) as f32 - 0.5;
            *sample = (noise - last) * 0.5;
            last = noise;
        }
        let accents = Accents::from_mono(&samples, sample_rate);
        let (kick, hat) = (accents.band_onsets(1000.0), accents.band_onsets(2000.0));
        assert!(kick[0] > hat[0] * 2.0 && kick[0] > kick[2] * 2.0, "kick {:?}, hat {:?}", kick, hat);
        assert!(hat[2] > kick[2] * 2.0 && hat[2] > hat[0] * 2.0, "kick {:?}, hat {:?}", kick, hat);
    }

    /// Writes the onsets of each band and the loudness of `AUDIO` to `OUT`,
    /// a frame a line, to look at them next to the map.
    #[test]
    #[ignore = "reads an audio file; run by hand"]
    fn dump() {
        let path = std::env::var("AUDIO").expect("AUDIO");
        let accents = Accents::read(Path::new(&path)).unwrap();
        let text: String = accents
            .bands
            .iter()
            .zip(&accents.loudness)
            .enumerate()
            .map(|(i, (b, l))| format!("{:.2} {} {} {} {}\n", i as f64 * accents.hop_ms, b[0], b[1], b[2], l))
            .collect();
        std::fs::write(std::env::var("OUT").expect("OUT"), text).unwrap();
    }

    #[test]
    fn follows_a_rate_change() {
        let accents = Accents::from_mono(&clicks(44_100.0), 44_100.0);
        // At 1.5x the click at 3 s of the audio is at 2 s of the map.
        let fast = accents.at_rate(1.5);
        assert_eq!(fast.band_onsets(2000.0), accents.band_onsets(3000.0));
        assert!(accents.loudness(10_000.0, 11_000.0).is_none());
    }
}
