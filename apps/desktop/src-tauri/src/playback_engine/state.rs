use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use biquad::DirectForm1;

use super::{
    dsp::{build_filter_bank, EQ_BANDS},
    types::{PlaybackStatus, TrackPayload},
};

const CACHE_LIMIT: usize = 3;

pub struct PlaybackState {
    track: Option<TrackPayload>,
    preloaded: Option<TrackPayload>,
    cache: HashMap<String, TrackPayload>,
    cache_order: VecDeque<String>,
    frame_index: usize,
    playing: bool,
    ended: bool,
    volume: f32,
    eq_enabled: bool,
    eq_bands: [f32; EQ_BANDS],
    eq_filters: Vec<Vec<DirectForm1<f32>>>,
    output_channels: u16,
    output_sample_rate: u32,
}

impl PlaybackState {
    pub fn new(output_channels: u16, output_sample_rate: u32) -> Result<Self, String> {
        Ok(Self {
            track: None,
            preloaded: None,
            cache: HashMap::new(),
            cache_order: VecDeque::new(),
            frame_index: 0,
            playing: false,
            ended: false,
            volume: 0.8,
            eq_enabled: false,
            eq_bands: [0.0; EQ_BANDS],
            eq_filters: build_filter_bank(output_sample_rate, output_channels, [0.0; EQ_BANDS])?,
            output_channels,
            output_sample_rate,
        })
    }

    pub fn snapshot(&self) -> PlaybackStatus {
        PlaybackStatus {
            loaded: self.track.is_some(),
            playing: self.playing,
            position: self.frame_index as f64 / self.output_sample_rate as f64,
            duration: self
                .track
                .as_ref()
                .map(|track| track.song.duration)
                .unwrap_or(0.0),
            ended: self.ended,
            track_id: self.track.as_ref().map(|track| track.song.id.clone()),
            eq_enabled: self.eq_enabled,
            eq_bands: self.eq_bands,
            volume: self.volume,
        }
    }

    pub fn output_config(&self) -> (u16, u32) {
        (self.output_channels, self.output_sample_rate)
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn current_track(&self) -> Option<TrackPayload> {
        self.track.clone()
    }

    pub fn current_frame_index(&self) -> usize {
        self.frame_index
    }

    pub fn output_channels(&self) -> usize {
        self.output_channels as usize
    }

    pub fn eq_enabled(&self) -> bool {
        self.eq_enabled
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn filters_mut(&mut self) -> &mut [Vec<DirectForm1<f32>>] {
        &mut self.eq_filters
    }

    pub fn advance_frame(&mut self) {
        self.frame_index += 1;
    }

    pub fn mark_ended(&mut self) {
        self.playing = false;
        self.ended = true;
    }

    pub fn clear_playing(&mut self) {
        self.playing = false;
    }

    fn reset_filters(&mut self) -> Result<(), String> {
        self.eq_filters =
            build_filter_bank(self.output_sample_rate, self.output_channels, self.eq_bands)?;
        Ok(())
    }

    pub fn load(&mut self, track: TrackPayload, autoplay: bool) -> Result<(), String> {
        self.track = Some(track);
        self.preloaded = None;
        self.frame_index = 0;
        self.playing = autoplay;
        self.ended = false;
        self.reset_filters()
    }

    pub fn store_preloaded(&mut self, track: TrackPayload) {
        self.preloaded = Some(track);
    }

    pub fn take_preloaded(&mut self, song_id: &str) -> Option<TrackPayload> {
        if self
            .preloaded
            .as_ref()
            .map(|track| track.song.id.as_str() == song_id)
            .unwrap_or(false)
        {
            return self.preloaded.take();
        }
        None
    }

    pub fn cache_track(&mut self, track: TrackPayload) {
        let track_id = track.song.id.clone();
        self.cache.insert(track_id.clone(), track);
        self.cache_order.retain(|cached_id| cached_id != &track_id);
        self.cache_order.push_back(track_id);

        while self.cache_order.len() > CACHE_LIMIT {
            if let Some(evicted_id) = self.cache_order.pop_front() {
                self.cache.remove(&evicted_id);
            }
        }
    }

    pub fn get_cached(&self, song_id: &str) -> Option<TrackPayload> {
        if self
            .track
            .as_ref()
            .map(|track| track.song.id.as_str() == song_id)
            .unwrap_or(false)
        {
            return self.track.clone();
        }

        if self
            .preloaded
            .as_ref()
            .map(|track| track.song.id.as_str() == song_id)
            .unwrap_or(false)
        {
            return self.preloaded.clone();
        }

        self.cache.get(song_id).cloned()
    }

    pub fn play(&mut self) {
        if self.track.is_some() {
            self.playing = true;
            self.ended = false;
        }
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn stop(&mut self) -> Result<(), String> {
        self.track = None;
        self.frame_index = 0;
        self.playing = false;
        self.ended = false;
        self.reset_filters()
    }

    pub fn seek(&mut self, position: f64) -> Result<(), String> {
        if let Some(track) = &self.track {
            let total_frames = track.samples.len() / self.output_channels as usize;
            let target = (position.max(0.0) * self.output_sample_rate as f64).round() as usize;
            self.frame_index = target.min(total_frames);
            self.ended = false;
            self.reset_filters()?;
        }
        Ok(())
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    pub fn set_eq(&mut self, enabled: bool, bands: [f32; EQ_BANDS]) -> Result<(), String> {
        self.eq_enabled = enabled;
        self.eq_bands = bands;
        self.reset_filters()
    }
}

pub fn samples_for_frame(track: &Arc<Vec<f32>>, frame_index: usize, channels: usize) -> Option<&[f32]> {
    let frame_start = frame_index.checked_mul(channels)?;
    let frame_end = frame_start.checked_add(channels)?;
    track.get(frame_start..frame_end)
}
