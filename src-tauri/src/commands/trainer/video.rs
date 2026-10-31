//! Video speed change for the rate change, through the system `ffmpeg`.
//!
//! ffmpeg is not bundled: it has to be in the PATH. The UI asks first with
//! [`is_ffmpeg_available`] and only offers the option when it is there.

use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{bail, Context, Result};

fn ffmpeg_command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);

    // Without this every call opens a console window on Windows.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd
}

#[tauri::command]
pub fn is_ffmpeg_available() -> bool {
    let available = ffmpeg_command("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());

    log::debug!("ffmpeg available: {}", available);
    available
}

#[derive(Default)]
struct Probe {
    /// Seconds, for the progress.
    duration: Option<f64>,
    /// Bits per second, to cap the output.
    bit_rate: Option<u64>,
}

/// What ffprobe can tell about the video; empty when ffprobe is missing.
fn probe(src: &Path) -> Probe {
    let Ok(output) = ffmpeg_command("ffprobe")
        .args(["-v", "error", "-show_entries", "format=duration,bit_rate", "-of", "default=noprint_wrappers=1"])
        .arg(src)
        .output()
    else {
        return Probe::default();
    };

    let mut probe = Probe::default();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        match line.split_once('=') {
            Some(("duration", v)) => probe.duration = v.trim().parse().ok().filter(|d: &f64| *d > 0.0),
            Some(("bit_rate", v)) => probe.bit_rate = v.trim().parse().ok().filter(|b: &u64| *b > 0),
            _ => {}
        }
    }
    probe
}

/// Re-encodes `src` into `dst` played `rate` times faster, as H.264 without
/// audio (osu! never plays the video's audio). The container follows the
/// extension of `dst`. The frame rate is kept, so speeding up drops frames
/// instead of raising it.
///
/// The quality is CRF 28, or CRF 23 when `high_quality`: nearly twice the
/// size, while osu! shows the video dimmed behind the map.
pub fn apply_rate(src: &Path, dst: &Path, rate: f64, high_quality: bool, on_progress: impl Fn(f64)) -> Result<()> {
    let probe = probe(src);
    let total = probe.duration.map(|d| d / rate);

    // Capped at the original bit rate: the result is never heavier per second
    // than the source, and being shorter, it ends up smaller.
    let crf = if high_quality { "23" } else { "28" };
    let mut rate_control = vec!["-crf".to_string(), crf.to_string()];
    if let Some(bit_rate) = probe.bit_rate {
        rate_control.extend(["-maxrate".to_string(), bit_rate.to_string()]);
        rate_control.extend(["-bufsize".to_string(), (bit_rate * 2).to_string()]);
    }

    let is_mp4 = dst
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mp4") || e.eq_ignore_ascii_case("m4v"));

    let mut child = ffmpeg_command("ffmpeg")
        .args(["-y", "-v", "error", "-nostats", "-progress", "pipe:1"])
        .arg("-i")
        .arg(src)
        .args(["-an", "-sn", "-dn"])
        .arg("-filter:v")
        .arg(format!("setpts=PTS/{}", rate))
        .args(["-c:v", "libx264", "-preset", "veryfast", "-pix_fmt", "yuv420p"])
        .args(&rate_control)
        .args(if is_mp4 { &["-movflags", "+faststart"][..] } else { &[] })
        .arg(dst)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to start ffmpeg")?;

    // With `-v error` stderr stays small, so reading it after stdout is safe.
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(|l| l.ok()) {
            let (Some(total), Some(us)) = (total, line.strip_prefix("out_time_us=")) else { continue };
            if let Ok(us) = us.trim().parse::<f64>() {
                on_progress((us / 1_000_000.0 / total).clamp(0.0, 1.0));
            }
        }
    }

    let output = child.wait_with_output().context("failed to wait for ffmpeg")?;
    if !output.status.success() {
        bail!("ffmpeg failed ({}): {}", output.status, String::from_utf8_lossy(&output.stderr).trim());
    }

    Ok(())
}
