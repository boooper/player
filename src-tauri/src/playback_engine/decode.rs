use std::io::Cursor;

use symphonia::{
    core::{
        audio::{AudioBufferRef, SampleBuffer},
        codecs::DecoderOptions,
        errors::Error as SymphoniaError,
        formats::FormatOptions,
        io::MediaSourceStream,
        meta::MetadataOptions,
        probe::Hint,
    },
    default::{get_codecs, get_probe},
};

pub fn decode_audio(
    bytes: Vec<u8>,
    content_type: &str,
    audio_format: &str,
    target_channels: u16,
    target_sample_rate: u32,
) -> Result<(Vec<f32>, u16, u32), String> {
    let mut hint = Hint::new();
    let mime_ext = content_type
        .split('/')
        .nth(1)
        .map(|value| value.split(';').next().unwrap_or_default().trim())
        .unwrap_or_default();
    let ext_hint = if !mime_ext.is_empty() {
        mime_ext
    } else {
        audio_format.trim()
    };
    if !ext_hint.is_empty() {
        hint.with_extension(ext_hint);
    }

    let media_source = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
    let probed = get_probe()
        .format(
            &hint,
            media_source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|error| error.to_string())?;
    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| "No default audio track found".to_string())?;
    let track_id = track.id;
    let mut decoder = get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|error| error.to_string())?;

    let mut source_channels = 0u16;
    let mut source_sample_rate = 0u32;
    let mut decoded = Vec::<f32>::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::ResetRequired) => {
                return Err("Symphonia requested decoder reset".to_string());
            }
            Err(SymphoniaError::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(error) => return Err(error.to_string()),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded_packet = decoder.decode(&packet).map_err(|error| error.to_string())?;
        match decoded_packet {
            AudioBufferRef::F32(buffer) => {
                source_channels = buffer.spec().channels.count() as u16;
                source_sample_rate = buffer.spec().rate;
                let mut sample_buffer =
                    SampleBuffer::<f32>::new(buffer.capacity() as u64, *buffer.spec());
                sample_buffer.copy_interleaved_ref(AudioBufferRef::F32(buffer));
                decoded.extend_from_slice(sample_buffer.samples());
            }
            other => {
                source_channels = other.spec().channels.count() as u16;
                source_sample_rate = other.spec().rate;
                let mut sample_buffer =
                    SampleBuffer::<f32>::new(other.capacity() as u64, *other.spec());
                sample_buffer.copy_interleaved_ref(other);
                decoded.extend_from_slice(sample_buffer.samples());
            }
        }
    }

    if decoded.is_empty() || source_channels == 0 || source_sample_rate == 0 {
        return Err("No audio samples decoded".to_string());
    }

    let resampled = resample_interleaved(
        &decoded,
        source_channels,
        source_sample_rate,
        target_channels,
        target_sample_rate,
    );
    Ok((resampled, target_channels, target_sample_rate))
}

/// Compute a linear gain factor that brings the RMS of `samples` to
/// `target_dbfs` dBFS. The result is clamped to [0.1, 10.0] (~±20 dB) so
/// silence and extremely loud tracks don't get extreme adjustments.
/// Returns 1.0 for tracks with no audible energy.
pub fn compute_norm_gain(samples: &[f32], target_dbfs: f32) -> f32 {
    if samples.is_empty() {
        return 1.0;
    }
    let mean_sq = samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32;
    let rms = mean_sq.sqrt();
    if rms < 1e-9 {
        return 1.0;
    }
    let target_linear = 10.0_f32.powf(target_dbfs / 20.0);
    (target_linear / rms).clamp(0.1, 10.0)
}

/// Estimates the BPM of `samples` using energy-onset autocorrelation.
///
/// Strategy:
/// 1. Compute a per-frame RMS energy envelope (≈10 ms frames, mono mix).
/// 2. Derive an onset-strength signal (half-wave rectified first derivative).
/// 3. Autocorrelate the onset signal over the lag range corresponding to 60–200 BPM.
/// 4. The lag with the highest correlation → raw BPM.
/// 5. Fold into the 80–160 range to resolve octave ambiguity.
///
/// Only the first 90 seconds are analysed to keep decode overhead low.
/// Returns 120.0 as a safe default when detection fails.
pub fn estimate_bpm(samples: &[f32], channels: usize, sample_rate: u32) -> f32 {
    if samples.is_empty() || channels == 0 || sample_rate == 0 {
        return 120.0;
    }

    // Analyse up to 90 s.
    let max_samples = (sample_rate as usize * 90 * channels).min(samples.len());

    // Hop ≈ 10 ms of frames.
    let hop = ((sample_rate as f64 * 0.01) as usize).max(1);
    let num_frames = (max_samples / channels) / hop;
    if num_frames < 60 {
        return 120.0;
    }

    // Energy envelope (mono mix, RMS per hop).
    let energy: Vec<f32> = (0..num_frames)
        .map(|i| {
            let start = i * hop * channels;
            let end = (start + hop * channels).min(samples.len());
            let sum_sq: f32 = samples[start..end]
                .chunks(channels)
                .map(|ch| ch.iter().map(|&s| s * s).sum::<f32>() / channels as f32)
                .sum();
            (sum_sq / hop as f32).sqrt()
        })
        .collect();

    // Onset strength: half-wave rectified first derivative.
    let onset: Vec<f32> = energy.windows(2).map(|w| (w[1] - w[0]).max(0.0)).collect();

    // Lag range for 60–200 BPM.
    let frame_secs = hop as f32 / sample_rate as f32;
    let min_lag = ((60.0_f32 / 200.0) / frame_secs).round() as usize;
    let max_lag = ((60.0_f32 / 60.0) / frame_secs).round() as usize;
    let n = onset.len();

    let mut best_corr = 0.0_f32;
    let mut best_lag = (min_lag + max_lag) / 2; // ~120 BPM default

    for lag in min_lag..=max_lag.min(n / 2) {
        let corr: f32 = onset[..n - lag]
            .iter()
            .zip(&onset[lag..])
            .map(|(a, b)| a * b)
            .sum();
        if corr > best_corr {
            best_corr = corr;
            best_lag = lag;
        }
    }

    if best_lag == 0 {
        return 120.0;
    }

    let mut bpm = 60.0 / (best_lag as f32 * frame_secs);

    // Fold into 80–160 BPM to resolve octave errors.
    while bpm < 80.0 { bpm *= 2.0; }
    while bpm > 160.0 { bpm /= 2.0; }

    bpm.clamp(60.0, 200.0)
}

/// Scans from the end of `samples` to find the last frame where the audio is
/// audible (RMS above −40 dBFS over a ~50 ms window). Returns the frame index
/// of that natural end. Used to let smart crossfade trigger at the right moment
/// rather than relying solely on the nominal track duration.
pub fn find_natural_end_frame(samples: &[f32], channels: usize, sample_rate: u32) -> usize {
    let total_frames = samples.len() / channels;
    if total_frames == 0 || channels == 0 {
        return 0;
    }
    let window_frames = ((sample_rate as f64 * 0.05) as usize).max(1);
    // −40 dBFS linear threshold
    let threshold = 10.0_f32.powf(-40.0 / 20.0);

    let mut frame = total_frames;
    while frame > 0 {
        let end_frame = frame;
        let start_frame = end_frame.saturating_sub(window_frames);
        let slice = &samples[start_frame * channels..end_frame * channels];
        let rms = (slice.iter().map(|&s| s * s).sum::<f32>() / slice.len() as f32).sqrt();
        if rms > threshold {
            return end_frame;
        }
        frame = start_frame;
    }
    // Completely silent track — return full duration so crossfade still works
    total_frames
}

fn resample_interleaved(
    input: &[f32],
    input_channels: u16,
    input_sample_rate: u32,
    output_channels: u16,
    output_sample_rate: u32,
) -> Vec<f32> {
    let input_channels = input_channels as usize;
    let output_channels = output_channels as usize;
    let input_frames = input.len() / input_channels;
    if input_frames == 0 {
        return Vec::new();
    }

    if input_sample_rate == output_sample_rate && input_channels == output_channels {
        return input.to_vec();
    }

    let output_frames =
        ((input_frames as f64) * output_sample_rate as f64 / input_sample_rate as f64).ceil()
            as usize;
    let mut output = vec![0.0; output_frames * output_channels];

    for out_frame in 0..output_frames {
        let src_position =
            out_frame as f64 * input_sample_rate as f64 / output_sample_rate as f64;
        let left_index = src_position.floor() as usize;
        let right_index = left_index
            .min(input_frames - 1)
            .saturating_add(1)
            .min(input_frames - 1);
        let frac = (src_position - left_index as f64) as f32;

        for out_channel in 0..output_channels {
            let in_channel = if input_channels == 1 {
                0
            } else {
                out_channel.min(input_channels - 1)
            };
            let left = input[left_index * input_channels + in_channel];
            let right = input[right_index * input_channels + in_channel];
            output[out_frame * output_channels + out_channel] = left + (right - left) * frac;
        }
    }

    output
}
