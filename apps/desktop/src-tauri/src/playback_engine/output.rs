use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig,
};
use tokio::sync::{Mutex as AsyncMutex, Notify};

use super::{
    state::{samples_for_frame, PlaybackState},
    types::{PlaybackStatus, TrackPayload},
};

pub struct PlaybackHandle {
    state: Arc<Mutex<PlaybackState>>,
    pub inflight: Arc<AsyncMutex<HashMap<String, Arc<Notify>>>>,
    _stream: Stream,
}

impl PlaybackHandle {
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "No default audio output device found".to_string())?;
        let supported_config = device.default_output_config().map_err(|error| error.to_string())?;
        let stream_config: StreamConfig = supported_config.clone().into();
        let state = Arc::new(Mutex::new(PlaybackState::new(
            stream_config.channels,
            stream_config.sample_rate.0,
        )?));
        let error_callback = |error| eprintln!("Desktop playback stream error: {error}");

        let stream = match supported_config.sample_format() {
            SampleFormat::F32 => {
                build_stream::<f32>(&device, &stream_config, Arc::clone(&state), error_callback)?
            }
            SampleFormat::I16 => {
                build_stream::<i16>(&device, &stream_config, Arc::clone(&state), error_callback)?
            }
            SampleFormat::U16 => {
                build_stream::<u16>(&device, &stream_config, Arc::clone(&state), error_callback)?
            }
            other => return Err(format!("Unsupported output sample format: {other:?}")),
        };

        stream.play().map_err(|error| error.to_string())?;

        Ok(Self {
            state,
            inflight: Arc::new(AsyncMutex::new(HashMap::new())),
            _stream: stream,
        })
    }

    fn with_state<R>(
        &self,
        mutator: impl FnOnce(&mut PlaybackState) -> Result<R, String>,
    ) -> Result<R, String> {
        let mut state = self.state.lock().map_err(|error| error.to_string())?;
        mutator(&mut state)
    }

    pub fn load(&self, track: TrackPayload, autoplay: bool) -> Result<(), String> {
        self.with_state(|state| state.load(track, autoplay))
    }

    pub fn load_with_crossfade(
        &self,
        track: TrackPayload,
        autoplay: bool,
        crossfade_ms: u32,
    ) -> Result<(), String> {
        self.with_state(|state| {
            let duration_frames =
                (crossfade_ms as u64 * state.output_sample_rate() as u64 / 1000) as usize;
            state.load_with_crossfade(track, autoplay, duration_frames)
        })
    }

    pub fn play(&self) -> Result<(), String> {
        self.with_state(|state| {
            state.play();
            Ok(())
        })
    }

    pub fn pause(&self) -> Result<(), String> {
        self.with_state(|state| {
            state.pause();
            Ok(())
        })
    }

    pub fn stop(&self) -> Result<(), String> {
        self.with_state(|state| state.stop())
    }

    pub fn seek(&self, position: f64) -> Result<(), String> {
        self.with_state(|state| state.seek(position))
    }

    pub fn set_volume(&self, volume: f32) -> Result<(), String> {
        self.with_state(|state| {
            state.set_volume(volume);
            Ok(())
        })
    }

    pub fn set_eq(
        &self,
        enabled: bool,
        bands: [f32; super::dsp::EQ_BANDS],
    ) -> Result<(), String> {
        self.with_state(|state| state.set_eq(enabled, bands))
    }

    pub fn status(&self) -> Result<PlaybackStatus, String> {
        self.state
            .lock()
            .map(|state| state.snapshot())
            .map_err(|error| error.to_string())
    }

    pub fn output_config(&self) -> Result<(u16, u32), String> {
        self.state
            .lock()
            .map(|state| state.output_config())
            .map_err(|error| error.to_string())
    }

    pub fn preload(&self, track: TrackPayload) -> Result<(), String> {
        self.with_state(|state| {
            state.cache_track(track.clone());
            state.store_preloaded(track);
            Ok(())
        })
    }

    pub fn take_preloaded(&self, song_id: &str) -> Result<Option<TrackPayload>, String> {
        self.with_state(|state| Ok(state.take_preloaded(song_id)))
    }

    pub fn cached_track(&self, song_id: &str) -> Result<Option<TrackPayload>, String> {
        self.with_state(|state| Ok(state.get_cached(song_id)))
    }

    pub fn cache_track(&self, track: TrackPayload) -> Result<(), String> {
        self.with_state(|state| {
            state.cache_track(track);
            Ok(())
        })
    }

    pub fn clear_cached_tracks(&self) -> Result<(), String> {
        self.with_state(|state| {
            state.clear_cached_tracks();
            Ok(())
        })
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    state: Arc<Mutex<PlaybackState>>,
    error_callback: impl Fn(cpal::StreamError) + Send + 'static,
) -> Result<Stream, String>
where
    T: Sample + FromSample<f32> + SizedSample,
{
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| write_output_data(data, &state),
            error_callback,
            None,
        )
        .map_err(|error| error.to_string())
}

fn write_output_data<T>(output: &mut [T], shared: &Arc<Mutex<PlaybackState>>)
where
    T: Sample + FromSample<f32>,
{
    let Ok(mut state) = shared.lock() else {
        write_silence(output);
        return;
    };

    if !state.is_playing() {
        write_silence(output);
        return;
    }

    let channels = state.output_channels();
    let Some(track) = state.current_track() else {
        write_silence(output);
        state.clear_playing();
        return;
    };

    let total_frames = track.samples.len() / channels;
    for frame in output.chunks_mut(channels) {
        if state.current_frame_index() >= total_frames {
            state.mark_ended();
            for sample in frame.iter_mut() {
                *sample = T::from_sample(0.0);
            }
            continue;
        }

        let Some(input_frame) = samples_for_frame(&track.samples, state.current_frame_index(), channels) else {
            state.mark_ended();
            for sample in frame.iter_mut() {
                *sample = T::from_sample(0.0);
            }
            continue;
        };

        let volume = state.volume();
        let eq_enabled = state.eq_enabled();
        for (channel, sample) in frame.iter_mut().enumerate() {
            let incoming = input_frame[channel];
            let mixed = match state.crossfade_outgoing_sample(channel) {
                Some((outgoing, progress)) => incoming * progress + outgoing * (1.0 - progress),
                None => incoming,
            };
            let mut value = mixed * volume;
            if eq_enabled {
                value = state.filters_mut()[channel]
                    .iter_mut()
                    .fold(value, |acc, filter| biquad::Biquad::run(filter, acc));
            }
            *sample = T::from_sample(value);
        }

        state.advance_frame();
    }
}

fn write_silence<T>(output: &mut [T])
where
    T: Sample + FromSample<f32>,
{
    for sample in output.iter_mut() {
        *sample = T::from_sample(0.0);
    }
}
