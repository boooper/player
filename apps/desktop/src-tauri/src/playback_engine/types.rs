use std::sync::Arc;

use serde::Serialize;

use crate::commands::media::Song;

use super::dsp::EQ_BANDS;

#[derive(Clone, Debug)]
pub struct TrackPayload {
    pub song: Song,
    pub samples: Arc<Vec<f32>>,
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
    pub eq_enabled: bool,
    pub eq_bands: [f32; EQ_BANDS],
    pub volume: f32,
}
