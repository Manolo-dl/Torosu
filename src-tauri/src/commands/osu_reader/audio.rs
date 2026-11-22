//! The music osu! is playing, for the visualizer in the title bar: where the
//! song is, often, and the spectrum osu! itself computes when it shows it
//! (only on the main menu). Read by a thread of its own, faster than the
//! beatmap's, and only while the frontend asks for it.

use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

use osu_memory_reader::OsuReader;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// How often the song's position is read: the frontend fills in between.
/// Each reading is an event the window's main thread hands to the page: at
/// 50 ms they cost it ~1.5% more of a core, with no difference in the bars.
const AUDIO_INTERVAL: Duration = Duration::from_millis(100);
/// While the song stands still (paused, or no song), how often to say so anyway.
const STILL_INTERVAL: Duration = Duration::from_millis(500);
/// How often to look for osu! while it is closed.
const ATTACH_INTERVAL: Duration = Duration::from_secs(2);
/// How often to look again while no one wants the music.
const IDLE_INTERVAL: Duration = Duration::from_millis(500);

/// Whether the frontend wants `osu-audio`, and whether its thread started.
#[derive(Default)]
pub struct OsuAudioState {
    pub wanted: AtomicBool,
    pub started: AtomicBool,
}

/// Sent as `osu-audio`.
#[derive(Clone, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AudioFrame {
    /// Where the song is, in its own milliseconds (negative in a lead-in).
    pub play_time: i32,
    /// osu!'s own spectrum, low to high: on the main menu only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spectrum: Option<Vec<f32>>,
}

/// Starts or stops sending `osu-audio`. Its thread starts the first time.
#[tauri::command]
pub fn watch_osu_audio(app: AppHandle, state: tauri::State<'_, OsuAudioState>, enabled: bool) -> crate::error::Result<()> {
    state.wanted.store(enabled, Ordering::SeqCst);
    if enabled && !state.started.swap(true, Ordering::SeqCst) {
        thread::Builder::new()
            .name("osu-audio".into())
            .spawn(move || run(app))
            .map_err(|e| format!("failed to start the osu! audio reader: {e}"))?;
    }
    Ok(())
}

fn wanted<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.state::<OsuAudioState>().wanted.load(Ordering::SeqCst)
}

fn run<R: Runtime>(app: AppHandle<R>) {
    loop {
        if !wanted(&app) {
            thread::sleep(IDLE_INTERVAL);
            continue;
        }
        let reader = match OsuReader::attach() {
            Ok(reader) => reader,
            Err(_) => {
                thread::sleep(ATTACH_INTERVAL);
                continue;
            }
        };

        let mut last: Option<AudioFrame> = None;
        let mut last_sent = Instant::now();
        while reader.is_running() && wanted(&app) {
            // Isolated read errors are normal while osu! changes screens.
            if let Ok(play_time) = reader.play_time() {
                let spectrum = reader.audio_spectrum().ok().flatten().map(|bands| bands.to_vec());
                let frame = AudioFrame { play_time, spectrum };
                if should_send(last.as_ref(), &frame, last_sent.elapsed()) {
                    let _ = app.emit("osu-audio", &frame);
                    last_sent = Instant::now();
                }
                last = Some(frame);
            }
            thread::sleep(AUDIO_INTERVAL);
        }
    }
}

/// Every reading while the song moves or osu! gives its spectrum; while it
/// stands still, now and then only.
fn should_send(last: Option<&AudioFrame>, frame: &AudioFrame, since_last: Duration) -> bool {
    match last {
        Some(last) if last == frame && frame.spectrum.is_none() => since_last >= STILL_INTERVAL,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(play_time: i32) -> AudioFrame {
        AudioFrame { play_time, spectrum: None }
    }

    #[test]
    fn sends_every_reading_while_the_song_moves() {
        assert!(should_send(None, &frame(0), Duration::ZERO));
        assert!(should_send(Some(&frame(100)), &frame(150), Duration::ZERO));
    }

    #[test]
    fn sends_now_and_then_while_it_stands_still() {
        assert!(!should_send(Some(&frame(100)), &frame(100), Duration::from_millis(50)));
        assert!(should_send(Some(&frame(100)), &frame(100), STILL_INTERVAL));
    }
}
