use tauri::State;

use crate::{
    playback_engine::{load_track_payload, normalize_eq_bands, PlaybackStatus},
    AppState,
};

use super::media::Song;

fn playback_handle<'a>(
    state: &'a State<'_, AppState>,
) -> Result<&'a crate::playback_engine::PlaybackHandle, String> {
    state.playback.as_ref().ok_or_else(|| "Audio engine not initialized".to_string())
}

fn is_song_cached(state: &State<'_, AppState>, song: &Song) -> Result<bool, String> {
    if playback_handle(state)?.cached_track(&song.id)?.is_some() {
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
    crossfade_ms: Option<u32>,
) -> Result<(), String> {
    #[cfg(target_os = "android")]
    let android_meta = (
        song.title.clone(), song.artist.clone(), song.album.clone(),
        song.cover_art_url.clone(), song.duration,
    );

    let preloaded = playback_handle(&state)?.take_preloaded(&song.id)?;
    let payload = if let Some(preloaded) = preloaded {
        preloaded
    } else {
        load_track_payload(&state, song).await?
    };

    let autoplay = autoplay.unwrap_or(false);
    let result = match crossfade_ms.filter(|&ms| ms > 0) {
        Some(ms) => playback_handle(&state)?.load_with_crossfade(payload, autoplay, ms),
        None => playback_handle(&state)?.load(payload, autoplay),
    };

    #[cfg(target_os = "android")]
    if result.is_ok() {
        let (title, artist, album, cover_url, duration) = android_meta;
        crate::android_media::update_track(
            &title, &artist, &album, &cover_url, duration, autoplay, 0.0,
        );
    }

    result
}

#[tauri::command]
pub async fn playback_preload(state: State<'_, AppState>, song: Song) -> Result<(), String> {
    if playback_handle(&state)?.cached_track(&song.id)?.is_some() {
        return Ok(());
    }

    let payload = load_track_payload(&state, song).await?;
    playback_handle(&state)?.preload(payload)
}

#[tauri::command]
pub fn playback_play(state: State<'_, AppState>) -> Result<(), String> {
    let result = playback_handle(&state)?.play();
    #[cfg(target_os = "android")]
    if result.is_ok() {
        let pos = playback_handle(&state).ok()
            .and_then(|ph| ph.status().ok())
            .map(|s| s.position)
            .unwrap_or(0.0);
        crate::android_media::update_playback_state(true, pos);
    }
    result
}

#[tauri::command]
pub fn playback_pause(state: State<'_, AppState>) -> Result<(), String> {
    let result = playback_handle(&state)?.pause();
    #[cfg(target_os = "android")]
    if result.is_ok() {
        let pos = playback_handle(&state).ok()
            .and_then(|ph| ph.status().ok())
            .map(|s| s.position)
            .unwrap_or(0.0);
        crate::android_media::update_playback_state(false, pos);
    }
    result
}

#[tauri::command]
pub fn playback_stop(state: State<'_, AppState>) -> Result<(), String> {
    let result = playback_handle(&state)?.stop();
    #[cfg(target_os = "android")]
    if result.is_ok() {
        crate::android_media::stop();
    }
    result
}

#[tauri::command]
pub fn playback_seek(state: State<'_, AppState>, position: f64) -> Result<(), String> {
    playback_handle(&state)?.seek(position)
}

#[tauri::command]
pub fn playback_set_volume(state: State<'_, AppState>, volume: f32) -> Result<(), String> {
    playback_handle(&state)?.set_volume(volume)
}

#[tauri::command]
pub fn playback_set_eq(
    state: State<'_, AppState>,
    enabled: bool,
    bands: Vec<f32>,
) -> Result<(), String> {
    playback_handle(&state)?.set_eq(enabled, normalize_eq_bands(&bands))
}

#[tauri::command]
pub fn playback_status(state: State<'_, AppState>) -> Result<PlaybackStatus, String> {
    playback_handle(&state)?.status()
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
