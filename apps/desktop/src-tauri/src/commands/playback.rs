use tauri::State;

use crate::{
    playback_engine::{load_track_payload, normalize_eq_bands, PlaybackStatus},
    AppState,
};

use super::media::Song;

fn playback_handle<'a>(
    state: &'a State<'_, AppState>,
) -> &'a crate::playback_engine::PlaybackHandle {
    &state.playback
}

fn is_song_cached(state: &State<'_, AppState>, song: &Song) -> Result<bool, String> {
    if playback_handle(state).cached_track(&song.id)?.is_some() {
        return Ok(true);
    }

    if song.stream_url.starts_with("file:") || song.stream_url.starts_with("data:") {
        return Ok(true);
    }

    Ok(state.playback_cache.local_file_url(&song.id)?.is_some())
}

#[tauri::command]
pub async fn playback_load(
    state: State<'_, AppState>,
    song: Song,
    autoplay: Option<bool>,
) -> Result<(), String> {
    let preloaded = playback_handle(&state).take_preloaded(&song.id)?;
    let payload = if let Some(preloaded) = preloaded {
        preloaded
    } else {
        load_track_payload(&state, song).await?
    };

    playback_handle(&state).load(payload, autoplay.unwrap_or(false))
}

#[tauri::command]
pub async fn playback_preload(state: State<'_, AppState>, song: Song) -> Result<(), String> {
    if playback_handle(&state).cached_track(&song.id)?.is_some() {
        return Ok(());
    }

    let payload = load_track_payload(&state, song).await?;
    playback_handle(&state).preload(payload)
}

#[tauri::command]
pub fn playback_play(state: State<'_, AppState>) -> Result<(), String> {
    playback_handle(&state).play()
}

#[tauri::command]
pub fn playback_pause(state: State<'_, AppState>) -> Result<(), String> {
    playback_handle(&state).pause()
}

#[tauri::command]
pub fn playback_stop(state: State<'_, AppState>) -> Result<(), String> {
    playback_handle(&state).stop()
}

#[tauri::command]
pub fn playback_seek(state: State<'_, AppState>, position: f64) -> Result<(), String> {
    playback_handle(&state).seek(position)
}

#[tauri::command]
pub fn playback_set_volume(state: State<'_, AppState>, volume: f32) -> Result<(), String> {
    playback_handle(&state).set_volume(volume)
}

#[tauri::command]
pub fn playback_set_eq(
    state: State<'_, AppState>,
    enabled: bool,
    bands: Vec<f32>,
) -> Result<(), String> {
    playback_handle(&state).set_eq(enabled, normalize_eq_bands(&bands))
}

#[tauri::command]
pub fn playback_status(state: State<'_, AppState>) -> Result<PlaybackStatus, String> {
    playback_handle(&state).status()
}

#[tauri::command]
pub fn playback_is_cached(state: State<'_, AppState>, song: Song) -> Result<bool, String> {
    is_song_cached(&state, &song)
}

#[tauri::command]
pub fn playback_cached_ids(state: State<'_, AppState>, songs: Vec<Song>) -> Result<Vec<String>, String> {
    let mut cached_ids = Vec::new();
    for song in songs {
        if is_song_cached(&state, &song)? {
            cached_ids.push(song.id);
        }
    }
    Ok(cached_ids)
}
