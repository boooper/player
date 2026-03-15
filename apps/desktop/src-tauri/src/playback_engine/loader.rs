use std::{collections::HashMap, sync::Arc};

use reqwest::header::CONTENT_TYPE;
use tauri::State;
use tokio::sync::{Mutex as AsyncMutex, Notify};

use crate::{
    commands::{library::resolve_playback_song, media::Song},
    playback_engine::TrackPayload,
    AppState,
};

use super::{decode::decode_audio, output::PlaybackHandle};

async fn begin_inflight(
    inflight: &AsyncMutex<HashMap<String, Arc<Notify>>>,
    song_id: &str,
) -> Option<Arc<Notify>> {
    let mut inflight = inflight.lock().await;
    if let Some(notify) = inflight.get(song_id) {
        return Some(notify.clone());
    }

    inflight.insert(song_id.to_string(), Arc::new(Notify::new()));
    None
}

async fn finish_inflight(inflight: &AsyncMutex<HashMap<String, Arc<Notify>>>, song_id: &str) {
    let notify = {
        let mut inflight = inflight.lock().await;
        inflight.remove(song_id)
    };

    if let Some(notify) = notify {
        notify.notify_waiters();
    }
}

fn playback_handle<'a>(state: &'a State<'_, AppState>) -> Result<&'a PlaybackHandle, String> {
    Ok(&state.playback)
}

pub async fn load_track_payload(
    state: &State<'_, AppState>,
    song: Song,
) -> Result<TrackPayload, String> {
    let requested_song = song;
    let playback_song = resolve_playback_song(state, &requested_song).await?;
    let inflight = playback_handle(state)?.inflight.clone();

    loop {
        if let Some(cached) = playback_handle(state)?.cached_track(&requested_song.id)? {
            return Ok(cached);
        }

        if let Some(notify) = begin_inflight(&inflight, &requested_song.id).await {
            notify.notified().await;
            continue;
        }

        let result = async {
            let response = state
                .http
                .get(&playback_song.stream_url)
                .send()
                .await
                .map_err(|error| error.to_string())?
                .error_for_status()
                .map_err(|error| error.to_string())?;
            let content_type = response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("")
                .to_string();
            let bytes = response.bytes().await.map_err(|error| error.to_string())?;

            if bytes.len() >= 16 {
                eprintln!(
                    "Playback load {} content-type='{}' bytes={} head={:02x?}",
                    requested_song.id,
                    content_type,
                    bytes.len(),
                    &bytes[..16]
                );
            }

            if content_type.contains("application/json") {
                let snippet = String::from_utf8_lossy(&bytes[..bytes.len().min(160)]);
                return Err(format!("Expected audio stream for playback, got JSON response: {snippet}"));
            }

            let (output_channels, output_sample_rate) = playback_handle(state)?.output_config()?;
            let (samples, _channels, _sample_rate) =
                decode_audio(bytes.to_vec(), &content_type, output_channels, output_sample_rate)?;

            let payload = TrackPayload {
                song: requested_song.clone(),
                samples: Arc::new(samples),
            };
            playback_handle(state)?.cache_track(payload.clone())?;
            Ok(payload)
        }
        .await;

        finish_inflight(&inflight, &requested_song.id).await;
        return result;
    }
}
