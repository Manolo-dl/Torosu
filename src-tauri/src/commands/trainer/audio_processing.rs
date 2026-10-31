use std::{
    fs::File,
    num::{NonZeroU32, NonZeroU8},
    path::Path,
};

use anyhow::Result;
use soundtouch::{Setting, SoundTouch};
use symphonia::core::{
    codecs::{CodecParameters, audio::AudioDecoderOptions},
    formats::{FormatOptions, probe::Hint},
    io::MediaSourceStream,
    meta::MetadataOptions,
};
use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoderBuilder};

pub struct DecodedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: usize,
}

pub fn decode_audio(path: &Path) -> Result<DecodedAudio> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let fmt_opts: FormatOptions = Default::default();
    let meta_opts: MetadataOptions = Default::default();

    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, fmt_opts, meta_opts)?;

    let (track_id, audio_params) = format
        .tracks()
        .iter()
        .find_map(|t| {
            if let Some(CodecParameters::Audio(params)) = &t.codec_params {
                Some((t.id, params.clone()))
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("no supported audio track found"))?;

    let sample_rate = audio_params.sample_rate.unwrap_or(44100);
    let channels = audio_params
        .channels
        .as_ref()
        .map(|c| c.count())
        .unwrap_or(2);

    let dec_opts: AudioDecoderOptions = Default::default();
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&audio_params, &dec_opts)?;

    let mut samples: Vec<f32> = Vec::new();
    let mut packet_count = 0;
    let mut decoded_count = 0;

    // Reused across packets: copy_to_vec_interleaved resizes, so it overwrites
    // the contents while keeping the already reserved capacity.
    let mut packet_samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet()? {
            Some(packet) => packet,
            None => break,
        };
        packet_count += 1;

        if packet.track_id != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                decoded_count += 1;
                decoded.copy_to_vec_interleaved::<f32>(&mut packet_samples);
                samples.extend_from_slice(&packet_samples);
            }
            Err(e) => {
                log::warn!("decode error on packet: {:?}", e);
                continue;
            }
        }
    }

    log::debug!(
        "decode_audio finished: {} packets read, {} decoded ok, {} total samples",
        packet_count, decoded_count, samples.len()
    );

    Ok(DecodedAudio {
        samples,
        sample_rate,
        channels,
    })
}

/// Frames pushed into SoundTouch before draining its output.
const STRETCH_CHUNK_FRAMES: usize = 4096;

/// Changes the speed while preserving the original pitch.
///
/// Feeds SoundTouch in chunks and drains after each one instead of using
/// `generate_audio()`, which pushes the whole track at once and only drains at
/// the end. That grows SoundTouch's internal FIFO from 0 to ~85 MB in 4 KB
/// steps, each one a full memcpy: O(n^2). Draining per chunk keeps the FIFO at
/// a few KB and `ensureCapacity` never reallocates. On a 4 min track:
/// 57.7 s -> 0.19 s, with bit-identical output.
///
/// Borrows the decoded audio (~85 MB for 4 min of stereo): the converts of a
/// run share it.
pub fn apply_timestretch(
    audio: &DecodedAudio,
    rate: f64,
    mut on_progress: impl FnMut(f32),
) -> Result<Vec<f32>> {
    let channels = audio.channels;
    let total_frames = audio.samples.len() / channels;
    let mut processed_frames = 0usize;
    let mut last_pct = 0u8;

    let mut st = SoundTouch::new();
    // SequenceMs/SeekwindowMs are kept: dropping them for calcSeqParameters'
    // auto-tuning only saves ~5% once streaming, but changes the stretch
    // granulation, so the output would no longer be identical.
    st.set_channels(channels as u32)
        .set_sample_rate(audio.sample_rate)
        .set_tempo(rate)
        .set_setting(Setting::UseQuickseek, 1)
        .set_setting(Setting::SequenceMs, 40)
        .set_setting(Setting::SeekwindowMs, 15);

    let mut output: Vec<f32> = Vec::with_capacity((audio.samples.len() as f64 / rate) as usize + 4096);
    let mut recv = vec![0.0f32; STRETCH_CHUNK_FRAMES * channels * 2];
    let recv_frames = recv.len() / channels;

    for chunk in audio.samples.chunks(STRETCH_CHUNK_FRAMES * channels) {
        let chunk_frames = chunk.len() / channels;
        st.put_samples(chunk, chunk_frames);

        loop {
            let n = st.receive_samples(recv.as_mut_slice(), recv_frames);
            if n == 0 {
                break;
            }
            output.extend_from_slice(&recv[..n * channels]);
        }

        processed_frames += chunk_frames;
        let pct = ((processed_frames as f32 / total_frames as f32) * 100.0) as u8;
        if pct > last_pct {
            last_pct = pct;
            on_progress(processed_frames as f32 / total_frames as f32);
        }
    }

    // Drain the tail after flushing. `generate_audio()` calls flush() but never
    // reads again, dropping the last ~13 ms of the track.
    st.flush();
    loop {
        let n = st.receive_samples(recv.as_mut_slice(), recv_frames);
        if n == 0 {
            break;
        }
        output.extend_from_slice(&recv[..n * channels]);
    }

    log::debug!(
        "soundtouch output: {} samples (input was {})",
        output.len(),
        audio.samples.len()
    );

    Ok(output)
}

/// Changes the speed letting the pitch shift naturally: the same samples,
/// played at this sample rate, so they are encoded as they are.
pub fn resampled_rate(sample_rate: u32, rate: f64) -> u32 {
    (sample_rate as f64 * rate).round() as u32
}

/// Frames per channel submitted to libvorbis per call. The libvorbis docs
/// recommend ~1024; larger blocks don't speed it up and raise peak memory.
const ENCODE_BLOCK_FRAMES: usize = 1024;

/// Quality-based VBR (range -0.2..=1.0) for 44.1 kHz stereo: 0.4 is ~128 kbit/s,
/// 0.6 (`high`) ~192 kbit/s, on par with the MP3s osu! beatmaps usually ship.
pub fn vorbis_quality(high: bool) -> f32 {
    if high { 0.6 } else { 0.4 }
}

/// Encodes interleaved samples to Ogg Vorbis in memory so they can be added
/// straight into the .osz without going through a staging directory.
pub fn encode_audio(samples: &[f32], sample_rate: u32, channels: u16, quality: f32) -> Result<Vec<u8>> {
    log::debug!(
        "encode_audio: {} samples, sample_rate={}, channels={}, quality={}",
        samples.len(),
        sample_rate,
        channels,
        quality
    );

    let sample_rate_nz = NonZeroU32::new(sample_rate)
        .ok_or_else(|| anyhow::anyhow!("invalid sample rate: {}", sample_rate))?;
    let channels_nz = u8::try_from(channels)
        .ok()
        .and_then(NonZeroU8::new)
        .ok_or_else(|| anyhow::anyhow!("unsupported channel count: {}", channels))?;
    let channels = channels as usize;

    // ~24 KB/s covers q0.6 stereo, so the sink rarely has to reallocate.
    let duration_secs = samples.len() as u64 / (channels as u64 * sample_rate as u64);
    let sink = Vec::with_capacity((duration_secs as usize + 1) * 24 * 1024);

    let mut encoder = VorbisEncoderBuilder::new(sample_rate_nz, channels_nz, sink)?
        .bitrate_management_strategy(VorbisBitrateManagementStrategy::QualityVbr {
            target_quality: quality,
        })
        .build()?;

    // libvorbis takes planar input: deinterleave each block into reused buffers.
    let mut planar = vec![Vec::with_capacity(ENCODE_BLOCK_FRAMES); channels];

    for chunk in samples.chunks(ENCODE_BLOCK_FRAMES * channels) {
        for buf in planar.iter_mut() {
            buf.clear();
        }
        for frame in chunk.chunks_exact(channels) {
            for (buf, &sample) in planar.iter_mut().zip(frame) {
                buf.push(sample);
            }
        }
        encoder.encode_audio_block(&planar)?;
    }

    Ok(encoder.finish()?)
}


