#[cfg(desktop)]
use std::collections::HashMap;
use futures::future::join_all;
use tauri::State;
use url::Url;
use crate::{AppState, commands::profiles::{get_active_profile, ActiveProfile}};
use super::media::{Song, Album, Playlist, PlaylistDetail, AlbumDetail, SearchBundle};


fn is_jf(p: &ActiveProfile) -> bool {
    matches!(p.server_type.as_str(), "jellyfin" | "emby")
}

fn is_plex(p: &ActiveProfile) -> bool {
    p.server_type == "plex"
}

fn is_local(p: &ActiveProfile) -> bool {
    p.server_type == "local"
}

/// Jellyfin/Emby omit auth from image URLs so the webview cannot load them directly —
/// proxy those through the Rust backend. Subsonic and Plex embed auth as query params
/// so the webview can load cover art directly without proxying.
fn needs_artwork_proxy(p: &ActiveProfile) -> bool {
    is_jf(p)
}

fn is_remote_cover_url(url: &str) -> bool {
    Url::parse(url)
        .map(|parsed| matches!(parsed.scheme(), "http" | "https"))
        .unwrap_or(false)
}

async fn fetch_cover_url(
    http: &reqwest::Client,
    cache: &crate::playback_engine::DiskCache,
    cache_key: &str,
    cover_art_url: &str,
    proxy: bool,
) -> String {
    if !proxy || cover_art_url.is_empty() || !is_remote_cover_url(cover_art_url) {
        return cover_art_url.to_string();
    }
    match cache.get_or_fetch_remote(http, cache_key, "artwork", cover_art_url).await {
        Ok(local_url) => local_url,
        Err(_) => cover_art_url.to_string(),
    }
}

async fn cache_song_covers(state: &State<'_, AppState>, songs: Vec<Song>, proxy: bool) -> Vec<Song> {
    if !proxy { return songs; }
    let http = state.http.clone();
    let cache = state.artwork_cache.clone();
    join_all(songs.into_iter().map(|mut song| {
        let http = http.clone();
        let cache = cache.clone();
        async move {
            let key = format!("song-cover-{}-240", if song.cover_art.is_empty() { &song.id } else { &song.cover_art });
            song.cover_art_url = fetch_cover_url(&http, &cache, &key, &song.cover_art_url, true).await;
            song
        }
    })).await
}

async fn cache_album_covers(state: &State<'_, AppState>, albums: Vec<Album>, proxy: bool) -> Vec<Album> {
    if !proxy { return albums; }
    let http = state.http.clone();
    let cache = state.artwork_cache.clone();
    join_all(albums.into_iter().map(|mut album| {
        let http = http.clone();
        let cache = cache.clone();
        async move {
            let key = format!("album-cover-{}-400", if album.cover_art.is_empty() { &album.id } else { &album.cover_art });
            album.cover_art_url = fetch_cover_url(&http, &cache, &key, &album.cover_art_url, true).await;
            album
        }
    })).await
}

async fn cache_playlist_covers(
    state: &State<'_, AppState>,
    playlists: Vec<Playlist>,
    proxy: bool,
) -> Vec<Playlist> {
    if !proxy { return playlists; }
    let http = state.http.clone();
    let cache = state.artwork_cache.clone();
    join_all(playlists.into_iter().map(|mut playlist| {
        let http = http.clone();
        let cache = cache.clone();
        async move {
            let key = format!("playlist-cover-{}-240", if playlist.cover_art.is_empty() { &playlist.id } else { &playlist.cover_art });
            playlist.cover_art_url = fetch_cover_url(&http, &cache, &key, &playlist.cover_art_url, true).await;
            playlist
        }
    })).await
}

async fn cache_album_detail(state: &State<'_, AppState>, mut detail: AlbumDetail, proxy: bool) -> AlbumDetail {
    if !proxy { return detail; }
    let key = format!(
        "album-cover-{}-400",
        if detail.album.cover_art.is_empty() { &detail.album.id } else { &detail.album.cover_art }
    );
    detail.album.cover_art_url = fetch_cover_url(&state.http, &state.artwork_cache, &key, &detail.album.cover_art_url, true).await;
    detail.songs = cache_song_covers(state, detail.songs, true).await;
    detail
}

async fn cache_playlist_detail(
    state: &State<'_, AppState>,
    mut detail: PlaylistDetail,
    proxy: bool,
) -> PlaylistDetail {
    if !proxy { return detail; }
    let key = format!("playlist-cover-{}-240", detail.playlist.id);
    detail.playlist.cover_art_url = fetch_cover_url(&state.http, &state.artwork_cache, &key, &detail.playlist.cover_art_url, true).await;
    detail.songs = cache_song_covers(state, detail.songs, true).await;
    detail
}


pub(crate) async fn resolve_playback_song(
    state: &State<'_, AppState>,
    song: &Song,
) -> Result<Song, String> {
    if !song.id.starts_with("ext-") {
        return Ok(song.clone());
    }

    let p = get_active_profile(&state)?;

    if is_jf(&p) || is_plex(&p) {
        return Err("External tracks are not supported on this server type.".to_string());
    }

    // For octo-fiesta: the song's stream_url already points to /rest/stream?id=ext-...
    // Fetching it triggers octo-fiesta to download from the external provider and stream
    // the audio back directly. load_track_payload will fetch via stream_url — no separate
    // materialisation trigger or polling needed.
    Ok(song.clone())
}

async fn resolve_library_song_id(
    state: &State<'_, AppState>,
    id: &str,
    artist: Option<&str>,
    title: Option<&str>,
    album: Option<&str>,
) -> Result<String, String> {
    if !id.starts_with("ext-") {
        return Ok(id.to_string());
    }

    let lookup_song = Song {
        id: id.to_string(),
        title: title.unwrap_or_default().to_string(),
        artist: artist.unwrap_or_default().to_string(),
        album: album.unwrap_or_default().to_string(),
        album_id: String::new(),
        cover_art: String::new(),
        cover_art_url: String::new(),
        stream_url: String::new(),
        duration: 0.0,
        audio_format: None,
        bitrate_kbps: None,
        genre: None,
    };

    let resolved = resolve_playback_song(state, &lookup_song).await?;
    Ok(resolved.id)
}

// ── Commands ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn library_search(
    state: State<'_, AppState>,
    query: String,
    count: Option<u32>,
) -> Result<Vec<Song>, String> {
    let p = get_active_profile(&state)?;
    if is_local(&p) { return crate::commands::local::search(&p, &query, count.unwrap_or(20)).await; }
    let proxy = needs_artwork_proxy(&p);
    let songs = if is_jf(&p) {
        crate::commands::jellyfin::search(&state.http, &p, &query, count.unwrap_or(20)).await?
    } else if is_plex(&p) {
        crate::commands::plex::search(&state.http, &p, &query, count.unwrap_or(20)).await?
    } else {
        crate::commands::subsonic::search(&state.http, &p, &query, count.unwrap_or(20)).await?
    };
    Ok(cache_song_covers(&state, songs, proxy).await)
}

#[tauri::command]
pub async fn library_search_bundle(
    state: State<'_, AppState>,
    query: String,
    song_count: Option<u32>,
    album_count: Option<u32>,
    recommendation_count: Option<u32>,
) -> Result<SearchBundle, String> {
    let p = get_active_profile(&state)?;
    let song_limit = song_count.unwrap_or(24);
    let album_limit = album_count.unwrap_or(12);
    let rec_limit = recommendation_count.unwrap_or(16);

    let proxy = needs_artwork_proxy(&p);

    let songs_fut = async {
        if is_local(&p) {
            crate::commands::local::search(&p, &query, song_limit).await
        } else if is_jf(&p) {
            let songs = crate::commands::jellyfin::search(&state.http, &p, &query, song_limit).await?;
            Ok(cache_song_covers(&state, songs, proxy).await)
        } else if is_plex(&p) {
            let songs = crate::commands::plex::search(&state.http, &p, &query, song_limit).await?;
            Ok(cache_song_covers(&state, songs, proxy).await)
        } else {
            let songs = crate::commands::subsonic::search(&state.http, &p, &query, song_limit).await?;
            Ok(cache_song_covers(&state, songs, proxy).await)
        }
    };

    let albums_fut = async {
        let albums = if is_local(&p) {
            crate::commands::local::artist_albums(&p, &query, album_limit).await.unwrap_or_default()
        } else if is_jf(&p) {
            crate::commands::jellyfin::artist_albums(&state.http, &p, &query, album_limit).await.unwrap_or_default()
        } else if is_plex(&p) {
            crate::commands::plex::artist_albums(&state.http, &p, &query, album_limit).await.unwrap_or_default()
        } else {
            crate::commands::subsonic::artist_albums(&state.http, &p, &query, album_limit).await.unwrap_or_default()
        };
        if is_local(&p) { albums } else { cache_album_covers(&state, albums, proxy).await }
    };

    let (songs, albums) = tokio::join!(songs_fut, albums_fut);
    let songs = songs?;

    let recommendations = if let Some(seed) = songs.first() {
        let recs = if is_local(&p) {
            crate::commands::local::similar(&p, &seed.id, rec_limit).await.unwrap_or_default()
        } else if is_jf(&p) {
            crate::commands::jellyfin::similar(&state.http, &p, &seed.id, rec_limit).await.unwrap_or_default()
        } else if is_plex(&p) {
            crate::commands::plex::similar(&state.http, &p, &seed.id, rec_limit).await.unwrap_or_default()
        } else {
            crate::commands::subsonic::similar(&state.http, &p, &seed.id, rec_limit).await.unwrap_or_default()
        };

        let mut deduped = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for song in recs {
            if song.id == seed.id || !seen.insert(song.id.clone()) {
                continue;
            }
            deduped.push(song);
        }

        if is_local(&p) {
            deduped
        } else {
            cache_song_covers(&state, deduped, proxy).await
        }
    } else {
        Vec::new()
    };

    Ok(SearchBundle {
        songs,
        albums,
        recommendations,
    })
}

#[tauri::command]
pub async fn library_similar(
    state: State<'_, AppState>,
    song_id: String,
    count: Option<u32>,
) -> Result<Vec<Song>, String> {
    // ext- IDs are external/unmaterialized tracks unknown to the server's similar-songs index
    if song_id.starts_with("ext-") {
        return Ok(vec![]);
    }
    let p = get_active_profile(&state)?;
    if is_local(&p) { return crate::commands::local::similar(&p, &song_id, count.unwrap_or(20)).await; }
    let proxy = needs_artwork_proxy(&p);
    let songs = if is_jf(&p) {
        crate::commands::jellyfin::similar(&state.http, &p, &song_id, count.unwrap_or(20)).await?
    } else if is_plex(&p) {
        crate::commands::plex::similar(&state.http, &p, &song_id, count.unwrap_or(20)).await?
    } else {
        crate::commands::subsonic::similar(&state.http, &p, &song_id, count.unwrap_or(20)).await?
    };
    Ok(cache_song_covers(&state, songs, proxy).await)
}

#[tauri::command]
pub async fn library_playlists(state: State<'_, AppState>) -> Result<Vec<Playlist>, String> {
    let p = get_active_profile(&state)?;
    if is_local(&p) { return crate::commands::local::playlists(&p).await; }
    let proxy = needs_artwork_proxy(&p);
    let playlists = if is_jf(&p) {
        crate::commands::jellyfin::playlists(&state.http, &p).await?
    } else if is_plex(&p) {
        crate::commands::plex::playlists(&state.http, &p).await?
    } else {
        crate::commands::subsonic::playlists(&state.http, &p).await?
    };
    Ok(cache_playlist_covers(&state, playlists, proxy).await)
}

#[tauri::command]
pub async fn library_playlist(
    state: State<'_, AppState>,
    id: String,
) -> Result<PlaylistDetail, String> {
    let p = get_active_profile(&state)?;
    if is_local(&p) { return crate::commands::local::playlist(&p, &id).await; }
    let proxy = needs_artwork_proxy(&p);
    let detail = if is_jf(&p) {
        crate::commands::jellyfin::playlist(&state.http, &p, &id).await?
    } else if is_plex(&p) {
        crate::commands::plex::playlist(&state.http, &p, &id).await?
    } else {
        crate::commands::subsonic::playlist(&state.http, &p, &id).await?
    };
    Ok(cache_playlist_detail(&state, detail, proxy).await)
}

#[tauri::command]
pub async fn library_artist_albums(
    state: State<'_, AppState>,
    query: String,
    count: Option<u32>,
) -> Result<Vec<Album>, String> {
    let p = get_active_profile(&state)?;
    if is_local(&p) { return crate::commands::local::artist_albums(&p, &query, count.unwrap_or(20)).await; }
    let proxy = needs_artwork_proxy(&p);
    let albums = if is_jf(&p) {
        crate::commands::jellyfin::artist_albums(&state.http, &p, &query, count.unwrap_or(20)).await?
    } else if is_plex(&p) {
        crate::commands::plex::artist_albums(&state.http, &p, &query, count.unwrap_or(20)).await?
    } else {
        crate::commands::subsonic::artist_albums(&state.http, &p, &query, count.unwrap_or(20)).await?
    };
    Ok(cache_album_covers(&state, albums, proxy).await)
}

#[tauri::command]
pub async fn library_album_songs(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<Song>, String> {
    let p = get_active_profile(&state)?;
    if is_local(&p) { return crate::commands::local::album_songs(&p, &id).await; }
    let proxy = needs_artwork_proxy(&p);
    let songs = if is_jf(&p) {
        crate::commands::jellyfin::album_songs(&state.http, &p, &id).await?
    } else if is_plex(&p) {
        crate::commands::plex::album_songs(&state.http, &p, &id).await?
    } else {
        crate::commands::subsonic::album_songs(&state.http, &p, &id).await?
    };
    Ok(cache_song_covers(&state, songs, proxy).await)
}

#[tauri::command]
pub async fn library_album(
    state: State<'_, AppState>,
    id: String,
) -> Result<AlbumDetail, String> {
    let p = get_active_profile(&state)?;
    if is_local(&p) { return crate::commands::local::album(&p, &id).await; }
    let proxy = needs_artwork_proxy(&p);
    let detail = if is_jf(&p) {
        crate::commands::jellyfin::album(&state.http, &p, &id).await?
    } else if is_plex(&p) {
        crate::commands::plex::album(&state.http, &p, &id).await?
    } else {
        crate::commands::subsonic::album(&state.http, &p, &id).await?
    };
    Ok(cache_album_detail(&state, detail, proxy).await)
}

#[tauri::command]
pub async fn library_album_list(
    state: State<'_, AppState>,
    kind: Option<String>,
    count: Option<u32>,
) -> Result<Vec<Album>, String> {
    let p = get_active_profile(&state)?;
    if is_local(&p) { return crate::commands::local::album_list(&p, &kind.clone().unwrap_or_else(|| "newest".to_string()), count.unwrap_or(20).min(100)).await; }
    let proxy = needs_artwork_proxy(&p);
    let kind = kind.unwrap_or_else(|| "newest".to_string());
    let albums = if is_jf(&p) {
        crate::commands::jellyfin::album_list(&state.http, &p, &kind, count.unwrap_or(20).min(100)).await?
    } else if is_plex(&p) {
        crate::commands::plex::album_list(&state.http, &p, &kind, count.unwrap_or(20).min(100)).await?
    } else {
        crate::commands::subsonic::album_list(&state.http, &p, &kind, count.unwrap_or(20).min(100)).await?
    };
    Ok(cache_album_covers(&state, albums, proxy).await)
}

#[tauri::command]
pub async fn library_starred(state: State<'_, AppState>) -> Result<Vec<Song>, String> {
    let p = get_active_profile(&state)?;
    if is_local(&p) { return crate::commands::local::starred(&p).await; }
    let proxy = needs_artwork_proxy(&p);
    let songs = if is_jf(&p) {
        crate::commands::jellyfin::starred(&state.http, &p).await?
    } else if is_plex(&p) {
        crate::commands::plex::starred(&state.http, &p).await?
    } else {
        crate::commands::subsonic::starred(&state.http, &p).await?
    };
    Ok(cache_song_covers(&state, songs, proxy).await)
}

#[tauri::command]
pub async fn library_star(
    state: State<'_, AppState>,
    id: String,
    unstar: Option<bool>,
    artist: Option<String>,
    title: Option<String>,
    album: Option<String>,
) -> Result<(), String> {
    let p = get_active_profile(&state)?;
    if is_local(&p) {
        return Err("Favorites are not supported for local libraries yet.".to_string());
    }
    if is_plex(&p) {
        return Err("Favorites are not supported for Plex libraries yet.".to_string());
    }
    let resolved_id = resolve_library_song_id(&state, &id, artist.as_deref(), title.as_deref(), album.as_deref()).await?;
    if is_jf(&p) {
        crate::commands::jellyfin::star(&state.http, &p, &resolved_id, unstar.unwrap_or(false)).await?;
        // Mirror to Last.fm regardless of server type
        #[cfg(desktop)]
        if let (Some(a), Some(t)) = (artist.as_deref(), title.as_deref()) {
            if !a.is_empty() && !t.is_empty() {
                let (key, secret, sk) = {
                    let db = state.db.lock().map_err(|e| e.to_string())?;
                    let s = crate::commands::settings::read_all(&db)?;
                    (s.get("LASTFM_API_KEY").cloned().unwrap_or_default(), s.get("LASTFM_SHARED_SECRET").cloned().unwrap_or_default(), s.get("LASTFM_SESSION_KEY").cloned().unwrap_or_default())
                };
                if !sk.is_empty() && !key.is_empty() && !secret.is_empty() {
                    let lfm_method = if unstar.unwrap_or(false) { "track.unlove" } else { "track.love" };
                    let mut params = HashMap::new();
                    params.insert("method".to_string(), lfm_method.to_string());
                    params.insert("sk".to_string(), sk);
                    params.insert("artist".to_string(), a.to_string());
                    params.insert("track".to_string(), t.to_string());
                    let _ = crate::commands::lastfm::sign_and_post(&state.http, params, &key, &secret).await;
                }
            }
        }
        return Ok(());
    }
    if resolved_id.is_empty() { return Err("Song id is required.".to_string()); }
    crate::commands::subsonic::star(&state.http, &p, &resolved_id, unstar.unwrap_or(false)).await?;

    // Mirror to Last.fm track.love / track.unlove
    #[cfg(desktop)]
    if let (Some(a), Some(t)) = (artist.as_deref(), title.as_deref()) {
        if !a.is_empty() && !t.is_empty() {
            let (key, secret, sk) = {
                let db = state.db.lock().map_err(|e| e.to_string())?;
                let s = crate::commands::settings::read_all(&db)?;
                (
                    s.get("LASTFM_API_KEY").cloned().unwrap_or_default(),
                    s.get("LASTFM_SHARED_SECRET").cloned().unwrap_or_default(),
                    s.get("LASTFM_SESSION_KEY").cloned().unwrap_or_default(),
                )
            };
            if !sk.is_empty() && !key.is_empty() && !secret.is_empty() {
                let lfm_method =
                    if unstar.unwrap_or(false) { "track.unlove" } else { "track.love" };
                let mut params = HashMap::new();
                params.insert("method".to_string(), lfm_method.to_string());
                params.insert("sk".to_string(), sk);
                params.insert("artist".to_string(), a.to_string());
                params.insert("track".to_string(), t.to_string());
                let _ = crate::commands::lastfm::sign_and_post(&state.http, params, &key, &secret).await;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn library_add_to_playlist(
    state: State<'_, AppState>,
    playlist_id: String,
    song_id: String,
    artist: Option<String>,
    title: Option<String>,
    album: Option<String>,
) -> Result<(), String> {
    let p = get_active_profile(&state)?;
    if is_local(&p) {
        return Err("Playlists are not supported for local libraries yet.".to_string());
    }
    if is_plex(&p) {
        return Err("Editing Plex playlists is not supported yet.".to_string());
    }
    let resolved_id = resolve_library_song_id(&state, &song_id, artist.as_deref(), title.as_deref(), album.as_deref()).await?;
    if is_jf(&p) { return crate::commands::jellyfin::add_to_playlist(&state.http, &p, &playlist_id, &resolved_id).await; }
    if playlist_id.is_empty() || resolved_id.is_empty() {
        return Err("playlistId and songId are required.".to_string());
    }
    crate::commands::subsonic::add_to_playlist(&state.http, &p, &playlist_id, &resolved_id).await
}

#[tauri::command]
pub async fn library_create_playlist(
    state: State<'_, AppState>,
    name: String,
    song_ids: Vec<String>,
) -> Result<Playlist, String> {
    let p = get_active_profile(&state)?;

    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err("Playlist name is required.".to_string());
    }

    if is_jf(&p) {
        return crate::commands::jellyfin::create_playlist(
            &state.http,
            &p,
            trimmed_name,
            &song_ids,
        )
        .await;
    }
    if is_local(&p) {
        return Err("Playlists are not supported for local libraries yet.".to_string());
    }
    if is_plex(&p) {
        return Err("Creating Plex playlists is not supported yet.".to_string());
    }

    let playlist = crate::commands::subsonic::create_playlist(&state.http, &p, trimmed_name, &song_ids).await?;
    Ok(cache_playlist_covers(&state, vec![playlist], needs_artwork_proxy(&p)).await.into_iter().next().unwrap())
}

#[tauri::command]
pub async fn library_rename_playlist(
    state: State<'_, AppState>,
    playlist_id: String,
    name: String,
) -> Result<(), String> {
    let p = get_active_profile(&state)?;

    let trimmed_name = name.trim();
    if playlist_id.trim().is_empty() {
        return Err("Playlist id is required.".to_string());
    }
    if trimmed_name.is_empty() {
        return Err("Playlist name is required.".to_string());
    }
    if is_local(&p) {
        return Err("Playlists are not supported for local libraries yet.".to_string());
    }
    if is_jf(&p) {
        return crate::commands::jellyfin::rename_playlist(
            &state.http,
            &p,
            &playlist_id,
            trimmed_name,
        )
        .await;
    }
    if is_plex(&p) {
        return Err("Renaming Plex playlists is not supported yet.".to_string());
    }

    crate::commands::subsonic::rename_playlist(&state.http, &p, &playlist_id, trimmed_name).await
}

#[tauri::command]
pub async fn library_delete_playlist(
    state: State<'_, AppState>,
    playlist_id: String,
) -> Result<(), String> {
    let p = get_active_profile(&state)?;

    if playlist_id.trim().is_empty() {
        return Err("Playlist id is required.".to_string());
    }
    if is_local(&p) {
        return Err("Playlists are not supported for local libraries yet.".to_string());
    }
    if is_jf(&p) {
        return crate::commands::jellyfin::delete_playlist(&state.http, &p, &playlist_id).await;
    }
    if is_plex(&p) {
        return Err("Deleting Plex playlists is not supported yet.".to_string());
    }

    crate::commands::subsonic::delete_playlist(&state.http, &p, &playlist_id).await
}

#[tauri::command]
pub async fn library_materialize_song(
    state: State<'_, AppState>,
    song_id: String,
) -> Result<(), String> {
    let p = get_active_profile(&state)?;

    if is_jf(&p) || is_plex(&p) {
        return Err("Materializing external tracks is only supported for Subsonic-compatible servers.".to_string());
    }
    if is_local(&p) {
        return Err("Local library tracks do not need materialization.".to_string());
    }
    if song_id.trim().is_empty() {
        return Err("Song id is required.".to_string());
    }

    crate::commands::subsonic::materialize_song(&state.http, &p, song_id.as_str()).await
}
