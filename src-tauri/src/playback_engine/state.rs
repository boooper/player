use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use biquad::DirectForm1;

use super::{
    dsp::{build_filter_bank, compute_loudness_compensation, EQ_BANDS, EQ_FREQUENCIES},
    types::{PlaybackStatus, TrackPayload},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NormalizationMode {
    /// Target –14 dBFS (matches streaming services' LUFS target).
    Lufs,
    /// Target –18 dBFS (legacy RMS).
    Rms,
}

impl NormalizationMode {
    pub fn target_dbfs(self) -> f32 {
        match self {
            NormalizationMode::Lufs => -14.0,
            NormalizationMode::Rms => -18.0,
        }
    }
}

const CACHE_LIMIT: usize = 3;

struct CrossfadeState {
    outgoing_samples: Arc<Vec<f32>>,
    outgoing_frame: usize,
    duration_frames: usize,
    elapsed_frames: usize,
    /// norm_gain of the track that is fading out.
    outgoing_norm_gain: f32,
}

pub struct PlaybackState {
    track: Option<TrackPayload>,
    preloaded: Option<TrackPayload>,
    cache: HashMap<String, TrackPayload>,
    cache_order: VecDeque<String>,
    frame_index: usize,
    playing: bool,
    ended: bool,
    /// Song ID of the track the engine autonomously advanced to for gapless
    /// playback. Reported in `snapshot()` so the frontend can sync the queue.
    /// Cleared by an explicit `load()` or `stop()`.
    gapless_next_id: Option<String>,
    normalization_enabled: bool,
    normalization_mode: NormalizationMode,
    loudness_compensation: bool,
    volume: f32,
    eq_enabled: bool,
    eq_frequencies: [f32; EQ_BANDS],
    eq_bands: [f32; EQ_BANDS],
    eq_filters: Vec<Vec<DirectForm1<f32>>>,
    output_channels: u16,
    output_sample_rate: u32,
    crossfade: Option<CrossfadeState>,
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
            gapless_next_id: None,
            normalization_enabled: false,
            normalization_mode: NormalizationMode::Lufs,
            loudness_compensation: false,
            volume: 0.8,
            eq_enabled: false,
            eq_frequencies: EQ_FREQUENCIES,
            eq_bands: [0.0; EQ_BANDS],
            eq_filters: build_filter_bank(output_sample_rate, output_channels, EQ_FREQUENCIES, [0.0; EQ_BANDS])?,
            output_channels,
            output_sample_rate,
            crossfade: None,
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
            gapless_advanced_to: self.gapless_next_id.clone(),
            eq_enabled: self.eq_enabled,
            eq_frequencies: self.eq_frequencies,
            eq_bands: self.eq_bands,
            volume: self.volume,
            smart_crossfade_point: self.track.as_ref().map(|t| t.smart_end_secs),
            current_track_bpm: self.track.as_ref().map(|t| t.bpm),
            preloaded_track_bpm: self.preloaded.as_ref().map(|t| t.bpm),
        }
    }

    pub fn output_config(&self) -> (u16, u32) {
        (self.output_channels, self.output_sample_rate)
    }

    pub fn output_sample_rate(&self) -> u32 {
        self.output_sample_rate
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

    /// True when any filter processing should run (user EQ or loudness compensation).
    pub fn filter_active(&self) -> bool {
        self.eq_enabled || self.loudness_compensation
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn normalization_enabled(&self) -> bool {
        self.normalization_enabled
    }

    pub fn set_normalization(&mut self, enabled: bool) {
        self.normalization_enabled = enabled;
    }

    pub fn normalization_mode(&self) -> NormalizationMode {
        self.normalization_mode
    }

    pub fn set_normalization_mode(&mut self, mode: NormalizationMode) {
        self.normalization_mode = mode;
    }

    /// Returns the norm_gain of the outgoing crossfade track, or 1.0.
    pub fn crossfade_outgoing_norm_gain(&self) -> f32 {
        self.crossfade.as_ref().map(|cf| cf.outgoing_norm_gain).unwrap_or(1.0)
    }

    pub fn filters_mut(&mut self) -> &mut [Vec<DirectForm1<f32>>] {
        &mut self.eq_filters
    }

    pub fn advance_frame(&mut self) {
        self.frame_index += 1;
        if let Some(cf) = &mut self.crossfade {
            cf.elapsed_frames += 1;
            if cf.elapsed_frames >= cf.duration_frames {
                self.crossfade = None;
            }
        }
    }

    /// Returns `(outgoing_sample, progress)` when a crossfade is active.
    /// `progress` goes from 0.0 (full outgoing) → 1.0 (full incoming).
    pub fn crossfade_outgoing_sample(&self, channel: usize) -> Option<(f32, f32)> {
        let cf = self.crossfade.as_ref()?;
        let progress = (cf.elapsed_frames as f32 / cf.duration_frames as f32).clamp(0.0, 1.0);
        let frame = cf.outgoing_frame + cf.elapsed_frames;
        let channels = self.output_channels as usize;
        let sample = cf.outgoing_samples
            .get(frame * channels + channel)
            .copied()
            .unwrap_or(0.0);
        Some((sample, progress))
    }

    pub fn mark_ended(&mut self) {
        self.playing = false;
        self.ended = true;
    }

    /// If a preloaded track is waiting, swap it in as the current track and
    /// start playing it immediately — no gap in audio output.
    /// Returns `true` if the switch happened, `false` if there was nothing to
    /// switch to (caller should fall back to `mark_ended`).
    /// NOTE: EQ filters are intentionally NOT reset so the IIR state flows
    /// continuously across the track boundary.
    pub fn advance_to_preloaded_if_ready(&mut self) -> bool {
        let Some(next) = self.preloaded.take() else {
            return false;
        };
        self.gapless_next_id = Some(next.song.id.clone());
        self.track = Some(next);
        self.frame_index = 0;
        self.ended = false;
        self.crossfade = None;
        // playing stays true — we keep the audio stream running without a stop
        true
    }

    pub fn clear_playing(&mut self) {
        self.playing = false;
    }

    fn reset_filters(&mut self) -> Result<(), String> {
        let effective_bands = if self.loudness_compensation {
            let comp = compute_loudness_compensation(self.volume);
            let mut bands = self.eq_bands;
            for (b, c) in bands.iter_mut().zip(comp.iter()) {
                *b = (*b + c).clamp(-12.0, 12.0);
            }
            bands
        } else {
            self.eq_bands
        };
        self.eq_filters = build_filter_bank(
            self.output_sample_rate,
            self.output_channels,
            self.eq_frequencies,
            effective_bands,
        )?;
        Ok(())
    }

    pub fn load(&mut self, track: TrackPayload, autoplay: bool) -> Result<(), String> {
        self.track = Some(track);
        self.preloaded = None;
        self.frame_index = 0;
        self.playing = autoplay;
        self.ended = false;
        self.gapless_next_id = None;
        self.crossfade = None;
        self.reset_filters()
    }

    /// Starts playing `incoming` immediately while fading out the current track over
    /// `duration_frames` frames. If no track is currently playing, falls back to a normal load.
    pub fn load_with_crossfade(
        &mut self,
        incoming: TrackPayload,
        autoplay: bool,
        duration_frames: usize,
    ) -> Result<(), String> {
        if duration_frames > 0 {
            if let Some(current) = &self.track {
                self.crossfade = Some(CrossfadeState {
                    outgoing_samples: Arc::clone(&current.samples),
                    outgoing_frame: self.frame_index,
                    duration_frames,
                    elapsed_frames: 0,
                    outgoing_norm_gain: current.norm_gain,
                });
            }
        }
        self.track = Some(incoming);
        self.preloaded = None;
        self.frame_index = 0;
        self.playing = autoplay;
        self.ended = false;
        self.gapless_next_id = None;
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

    pub fn clear_cached_tracks(&mut self) {
        self.preloaded = None;
        self.cache.clear();
        self.cache_order.clear();
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
        self.gapless_next_id = None;
        self.crossfade = None;
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

    pub fn set_volume(&mut self, volume: f32) -> Result<(), String> {
        self.volume = volume.clamp(0.0, 1.0);
        if self.loudness_compensation {
            self.reset_filters()?;
        }
        Ok(())
    }

    pub fn set_loudness_compensation(&mut self, enabled: bool) -> Result<(), String> {
        self.loudness_compensation = enabled;
        self.reset_filters()
    }

    pub fn set_eq(&mut self, enabled: bool, frequencies: [f32; EQ_BANDS], bands: [f32; EQ_BANDS]) -> Result<(), String> {
        self.eq_enabled = enabled;
        self.eq_frequencies = frequencies;
        self.eq_bands = bands;
        self.reset_filters()
    }
}

pub fn samples_for_frame(track: &Arc<Vec<f32>>, frame_index: usize, channels: usize) -> Option<&[f32]> {
    let frame_start = frame_index.checked_mul(channels)?;
    let frame_end = frame_start.checked_add(channels)?;
    track.get(frame_start..frame_end)
}
