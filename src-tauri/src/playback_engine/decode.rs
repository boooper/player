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
