use std::path::PathBuf;

use tauri::ipc::Response;

use crate::{commands::trainer::audio_processing::decode_audio, error::Result, paths};

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
        let audio = decode_audio(&file).map_err(|e| {
            log::warn!("failed to decode {:?}: {}", file, e);
            e
        })?;
        let mono = mono_at(&audio.samples, audio.channels, audio.sample_rate, VISUALIZER_RATE);
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

/// Interleaved `samples` of `channels` at `from` Hz, averaged into one channel
/// at `to` Hz: each output sample is the mean of the input frames it covers,
/// which also keeps what is above the new rate from folding back down.
fn mono_at(samples: &[f32], channels: usize, from: u32, to: u32) -> Vec<f32> {
    let channels = channels.max(1);
    let frames = samples.len() / channels;
    let step = from as f64 / to as f64;
    let count = (frames as f64 / step) as usize;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let start = (i as f64 * step) as usize;
        let end = (((i + 1) as f64 * step) as usize).clamp(start + 1, frames);
        let sum: f32 = samples[start * channels..end * channels].iter().sum();
        out.push(sum / ((end - start) * channels) as f32);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::mono_at;

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
    }
}
