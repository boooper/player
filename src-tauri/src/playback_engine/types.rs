use std::sync::Arc;

use serde::Serialize;

use crate::commands::media::Song;

use super::dsp::EQ_BANDS;

#[derive(Clone, Debug)]
pub struct TrackPayload {
    pub song: Song,
    pub samples: Arc<Vec<f32>>,
    /// Pre-computed linear gain that normalises this track to the configured
    /// normalization target (–14 dBFS for LUFS mode, –18 dBFS for RMS mode).
    /// Applied during playback when normalisation is enabled.
    pub norm_gain: f32,
    /// Time (seconds) of the last audible frame — where the track naturally
    /// fades to silence. Used by smart crossfade to trigger at the right moment.
    pub smart_end_secs: f64,
    /// Estimated BPM of this track (onset autocorrelation, folded to 80–160).
    pub bpm: f32,
}

#[derive(Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackStatus {
    pub loaded: bool,
    pub playing: bool,
    pub position: f64,
    pub duration: f64,
    pub ended: bool,
    pub track_id: Option<String>,
    /// Set when the engine autonomously advanced to the preloaded track for
    /// gapless playback. Contains the new track's song ID. Cleared on the
    /// next explicit `load()` or `stop()`.
    pub gapless_advanced_to: Option<String>,
    pub eq_enabled: bool,
    pub eq_frequencies: [f32; EQ_BANDS],
    pub eq_bands: [f32; EQ_BANDS],
    pub volume: f32,
    /// Natural end of the current track (seconds) — where audio fades to silence.
    /// None when no track is loaded.
    pub smart_crossfade_point: Option<f64>,
    /// Estimated BPM of the currently loaded track. None when no track is loaded.
    pub current_track_bpm: Option<f32>,
    /// Estimated BPM of the preloaded (next) track. None when not preloaded yet.
    pub preloaded_track_bpm: Option<f32>,
}
