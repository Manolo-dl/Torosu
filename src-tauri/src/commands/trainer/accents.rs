//! The accents of a song: how strongly a sound starts at each moment of its
//! audio, and how loud it is, every 10 ms. The key converter puts the notes
//! that mark accents (the scratches of 10K2S) where the music has them.
//!
//! A sound starting is measured as the spectral flux: how much louder each
//! frequency gets from one frame to the next, kicks to cymbals, compared with
//! the loudest of it and the frequencies next to it a little before
//! (SuperFlux), so a vibrato or a slide does not count as a new sound.
//! Against hand-made 10K2S maps, that and the loudness tell where their
//! mappers hit a scratch (see `corpus_scratch_truth`).

use std::path::Path;

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

pub struct Accents {
    /// The length of a frame in ms; frame `i` is centred on `i * hop_ms`.
    hop_ms: f64,
    /// Per frame, how strongly sounds start (the spectral flux).
    onset: Vec<f32>,
    /// Per frame, the loudness in dB.
    loudness: Vec<f32>,
}

impl Accents {
    pub fn read(path: &Path) -> Result<Self> {
        Ok(Self::new(&audio_processing::decode_audio(path)?))
    }

    pub fn new(audio: &DecodedAudio) -> Self {
        let channels = audio.channels.max(1);
        let mono: Vec<f32> =
            audio.samples.chunks_exact(channels).map(|frame| frame.iter().sum::<f32>() / channels as f32).collect();
        Self::from_mono(&mono, audio.sample_rate as f64)
    }

    pub(crate) fn from_mono(samples: &[f32], sample_rate: f64) -> Self {
        let n = 1usize << (WINDOW_MS / 1000.0 * sample_rate).log2().round().clamp(6.0, 16.0) as u32;
        let hop = (sample_rate * HOP_MS / 1000.0).round().max(1.0) as usize;
        let bin_hz = sample_rate / n as f64;
        let bins = n / 2 + 1;
        // The bins of the onsets, and the ones the loudness is measured over.
        let low = ((LOW_HZ / bin_hz).ceil() as usize).min(bins);
        let high = ((HIGH_HZ / bin_hz).ceil() as usize).clamp(low, bins);
        let audible = ((HIGH_HZ / bin_hz).ceil() as usize).clamp(1, bins);
        // As loud as a window of 2048 samples would make it, whatever the size.
        let scale = 2048.0 / n as f32;
        let window: Vec<f32> =
            (0..n).map(|i| (0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos()) as f32).collect();

        let frames = samples.len() / hop + 1;
        let mut fft = Fft::new(n);
        let mut buffer = vec![Complex::new(0.0f32, 0.0); n];
        let mut spectrum = vec![0.0f32; bins];
        // The spectra of the last frames, each frequency the loudest of it and
        // its neighbours: frame `f` in `past[f % LAG_FRAMES]`.
        let mut past = vec![vec![0.0f32; bins]; LAG_FRAMES];
        let mut onset = Vec::with_capacity(frames);
        let mut loudness = Vec::with_capacity(frames);

        for frame in 0..frames {
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
            loudness.push((10.0 * (power / audible as f64 + 1e-10).log10()) as f32);

            // The slot of this frame holds the one LAG_FRAMES before it.
            let before = &mut past[frame % LAG_FRAMES];
            let flux = if frame >= LAG_FRAMES {
                (low..high).map(|k| (spectrum[k] - before[k]).max(0.0)).sum::<f32>() / (high - low).max(1) as f32
            } else {
                0.0
            };
            onset.push(flux);
            for (k, value) in before.iter_mut().enumerate() {
                *value = spectrum[k].max(spectrum[k.saturating_sub(1)]).max(spectrum[(k + 1).min(bins - 1)]);
            }
        }

        Self { hop_ms: hop as f64 / sample_rate * 1000.0, onset, loudness }
    }

    fn frame(&self, ms: f64) -> isize {
        (ms / self.hop_ms).round() as isize
    }

    /// The frames from `from` to `to` ms, both included, within the song.
    fn frames(&self, from: f64, to: f64) -> std::ops::Range<usize> {
        let len = self.onset.len() as isize;
        let a = self.frame(from).clamp(0, len) as usize;
        let b = (self.frame(to) + 1).clamp(0, len) as usize;
        a..b.max(a)
    }

    /// How strongly a sound starts at `ms`: the strongest onset around it.
    pub fn onset(&self, ms: f64) -> f32 {
        self.onset[self.frames(ms - ONSET_MS, ms + ONSET_MS)].iter().copied().fold(0.0, f32::max)
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

/// The accents of a map's audio, in the map's times.
#[derive(Clone, Copy)]
pub struct MapAccents<'a> {
    accents: &'a Accents,
    rate: f64,
}

impl MapAccents<'_> {
    pub fn onset(&self, time: f64) -> f32 {
        self.accents.onset(time * self.rate)
    }

    pub fn loudness(&self, from: f64, to: f64) -> Option<f32> {
        self.accents.loudness(from * self.rate, to * self.rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            let quiet = accents.onset(700.0);
            let onsets: Vec<f32> = [1000.0, 1500.0, 2000.0, 2500.0, 3000.0].iter().map(|&t| accents.onset(t)).collect();
            assert!(onsets.iter().all(|&o| o > quiet * 10.0 + 1e-3), "{} Hz: {:?} against {}", sample_rate, onsets, quiet);
            // Between the clicks nothing starts.
            assert!(accents.onset(1250.0) < onsets[0] / 4.0, "{} Hz", sample_rate);
            // The louder clicks are louder.
            let loud = |t: f64| accents.loudness(t, t + 20.0).unwrap();
            assert!(loud(3000.0) > loud(1000.0), "{} Hz", sample_rate);
            assert!(loud(1000.0) > loud(700.0) + 20.0, "{} Hz", sample_rate);
        }
    }

    /// Writes the onsets and loudness of `AUDIO` to `OUT`, a frame a line, to
    /// look at them next to the map.
    #[test]
    #[ignore = "reads an audio file; run by hand"]
    fn dump() {
        let path = std::env::var("AUDIO").expect("AUDIO");
        let accents = Accents::read(Path::new(&path)).unwrap();
        let text: String = accents
            .onset
            .iter()
            .zip(&accents.loudness)
            .enumerate()
            .map(|(i, (o, l))| format!("{:.2} {} {}\n", i as f64 * accents.hop_ms, o, l))
            .collect();
        std::fs::write(std::env::var("OUT").expect("OUT"), text).unwrap();
    }

    #[test]
    fn follows_a_rate_change() {
        let accents = Accents::from_mono(&clicks(44_100.0), 44_100.0);
        // At 1.5x the click at 3 s of the audio is at 2 s of the map.
        let fast = accents.at_rate(1.5);
        assert_eq!(fast.onset(2000.0), accents.onset(3000.0));
        assert!(accents.loudness(10_000.0, 11_000.0).is_none());
    }
}
