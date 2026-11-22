use std::path::{Path, PathBuf};

use tauri::ipc::Response;

use crate::{commands::trainer::audio_processing::PacketDecoder, error::Result, paths};

/// Reads a file inside the Songs folder (e.g. a beatmap background) and sends
/// it as raw bytes, which the frontend turns into a `blob:` URL. `path` can be
/// absolute or relative to the Songs folder.
///
/// The `asset:` protocol is not used for this: WebKitGTK treats it as another
/// origin than the dev server (`http://localhost:1420`) and blocks the load
/// ("Unsafe attempt to load URL asset://localhost/..."). A blob URL is always
/// same-origin.
#[tauri::command]
pub async fn read_songs_file(app: tauri::AppHandle, path: String) -> Result<Response> {
    let file = songs_path(&app, &path).await?;
    let bytes = tokio::fs::read(&file).await.map_err(|e| {
        log::warn!("failed to read {:?}: {}", file, e);
        e
    })?;

    Ok(Response::new(bytes))
}

/// The rate of `read_song_samples`: enough for the visualizer's bands, up to 10 kHz.
const VISUALIZER_RATE: u32 = 22050;

/// The song `path` (as in `read_songs_file`), decoded for the title bar's
/// visualizer: one channel at `VISUALIZER_RATE`, as 16-bit little-endian
/// samples. Decoded here and not by the webview: WebKitGTK decodes with the
/// system's GStreamer plugins, and without gst-plugins-base it can't read
/// Ogg Vorbis, the format of many maps' songs.
#[tauri::command]
pub async fn read_song_samples(app: tauri::AppHandle, path: String) -> Result<Response> {
    let file = songs_path(&app, &path).await?;
    let bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        let mono = decode_mono(&file).map_err(|e| {
            log::warn!("failed to decode {:?}: {}", file, e);
            e
        })?;
        Ok(mono.iter().flat_map(|s| ((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()).collect())
    })
    .await
    .map_err(|e| format!("decoding the song stopped: {e}"))??;

    Ok(Response::new(bytes))
}

/// `path` inside the Songs folder, absolute or relative to it.
async fn songs_path(app: &tauri::AppHandle, path: &str) -> Result<PathBuf> {
    let songs = tokio::fs::canonicalize(paths::songs_dir(app)?)
        .await
        .map_err(|e| format!("Songs folder not found: {}", e))?;

    let file = tokio::fs::canonicalize(songs.join(path)).await.map_err(|e| {
        log::warn!("songs file {:?} not found: {}", path, e);
        e
    })?;

    // `path` comes from the frontend, but `..` or a symlink must not read outside Songs.
    if !file.starts_with(&songs) {
        log::warn!("refused to read {:?}: outside the Songs folder", file);
        return Err("path is outside the Songs folder".into());
    }
    Ok(file)
}

/// The song at `path` in one channel at `VISUALIZER_RATE`, made a packet at
/// a time as it is decoded: the whole song in stereo at its own rate is never
/// kept (187 MB at the most for a 6:40 song, against ~40 MB).
fn decode_mono(path: &Path) -> Result<Vec<f32>> {
    let mut decoder = PacketDecoder::open(path)?;
    let mut mono = MonoDownsampler::new(decoder.channels, decoder.sample_rate, VISUALIZER_RATE);
    while let Some(samples) = decoder.next_samples()? {
        mono.push(samples);
    }
    Ok(mono.finish())
}

/// Interleaved samples of `channels` at `from` Hz, averaged as they come into
/// one channel at `to` Hz: each sample made is the mean of the frames it
/// covers (from frame ⌊i·step⌋ to ⌊(i+1)·step⌋), which also keeps what is
/// above the new rate from folding back down. Frames past the last whole
/// sample are left out.
struct MonoDownsampler {
    channels: usize,
    /// Frames in, per sample out.
    step: f64,
    /// Frames taken so far, and the one the sample being made ends at.
    frames: u64,
    end: u64,
    sum: f32,
    summed: usize,
    out: Vec<f32>,
}

impl MonoDownsampler {
    fn new(channels: usize, from: u32, to: u32) -> Self {
        let step = from as f64 / to as f64;
        Self { channels: channels.max(1), step, frames: 0, end: (step as u64).max(1), sum: 0.0, summed: 0, out: Vec::new() }
    }

    fn push(&mut self, samples: &[f32]) {
        for frame in samples.chunks_exact(self.channels) {
            self.sum += frame.iter().sum::<f32>();
            self.summed += self.channels;
            self.frames += 1;
            if self.frames >= self.end {
                self.out.push(self.sum / self.summed as f32);
                self.sum = 0.0;
                self.summed = 0;
                let made = self.out.len() as u64;
                self.end = (((made + 1) as f64 * self.step) as u64).max(self.frames + 1);
            }
        }
    }

    fn finish(self) -> Vec<f32> {
        self.out
    }
}

/// All of `samples` at once through a `MonoDownsampler`.
#[cfg(test)]
fn mono_at(samples: &[f32], channels: usize, from: u32, to: u32) -> Vec<f32> {
    let mut mono = MonoDownsampler::new(channels, from, to);
    mono.push(samples);
    mono.finish()
}

#[cfg(test)]
mod tests {
    use super::{mono_at, MonoDownsampler};

    #[test]
    fn stereo_is_averaged_into_one_channel() {
        assert_eq!(mono_at(&[1.0, 0.0, 0.5, 0.5], 2, 100, 100), vec![0.5, 0.5]);
    }

    #[test]
    fn halving_the_rate_averages_pairs_of_frames() {
        assert_eq!(mono_at(&[1.0, 0.0, 0.25, 0.75, 0.5], 1, 44100, 22050), vec![0.5, 0.5]);
    }

    #[test]
    fn a_song_keeps_its_length() {
        let second = vec![0.0; 44100 * 2];
        assert_eq!(mono_at(&second, 2, 44100, 22050).len(), 22050);
        // 48 kHz: 2.18 frames a sample, so 2 or 3 frames each.
        assert_eq!(mono_at(&vec![0.0; 48000 * 2], 2, 48000, 22050).len(), 22050);
    }

    #[test]
    fn packets_of_any_size_make_the_same_samples() {
        let song: Vec<f32> = (0..48000 * 2).map(|i| ((i * 7919) % 1000) as f32 / 1000.0 - 0.5).collect();
        let whole = mono_at(&song, 2, 48000, 22050);
        let mut mono = MonoDownsampler::new(2, 48000, 22050);
        for packet in song.chunks(1152 * 2 + 6) {
            mono.push(packet);
        }
        assert_eq!(mono.finish(), whole);
    }
}
