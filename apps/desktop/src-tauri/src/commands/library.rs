use rand::Rng;
use std::collections::HashMap;
use tauri::State;
use url::Url;
use crate::{AppState, commands::profiles::{get_active_profile, ActiveProfile}};
use super::media::{Song, Album, Playlist, PlaylistDetail, PlaylistMeta, AlbumDetail, AlbumFull};

const API_VERSION: &str = "1.16.1";
const CLIENT_NAME: &str = "naviarr";

// ── Auth helpers ─────────────────────────────────────────────────────────────

fn auth_params(p: &ActiveProfile) -> Vec<(String, String)> {
    let mut base = vec![
        ("u".to_string(),  p.username.clone()),
        ("v".to_string(),  API_VERSION.to_string()),
        ("c".to_string(),  CLIENT_NAME.to_string()),
        ("f".to_string(),  "json".to_string()),
    ];
    if p.server_type == "subsonic_legacy" || p.password.starts_with("enc:") {
        base.push(("p".to_string(), p.password.clone()));
    } else {
        let salt: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(12)
            .map(char::from)
            .collect();
        let token = format!("{:x}", md5::compute(format!("{}{}", p.password, salt).as_bytes()));
        base.push(("t".to_string(), token));
        base.push(("s".to_string(), salt));
    }
    base
}

pub(crate) fn build_url(p: &ActiveProfile, path: &str, extra: &[(&str, &str)]) -> String {
    let base = format!("{}/rest/{}", p.url, path);
    let mut url = Url::parse(&base).expect("invalid profile url");
    {
        let mut q = url.query_pairs_mut();
        for (k, v) in auth_params(p) {
            q.append_pair(&k, &v);
        }
        for (k, v) in extra {
            q.append_pair(k, v);
        }
    }
    url.to_string()
}

pub(crate) async fn request(
    http: &reqwest::Client,
    p: &ActiveProfile,
    path: &str,
    params: &[(&str, &str)],
) -> Result<serde_json::Value, String> {
    let url = build_url(p, path, params);
    let resp = http.get(&url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Subsonic: HTTP {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let body = json
        .get("subsonic-response")
        .ok_or_else(|| "Invalid Subsonic response".to_string())?;
    if body.get("status").and_then(|s| s.as_str()) != Some("ok") {
        let msg = body
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("Subsonic request failed")
            .to_string();
        return Err(msg);
    }
    Ok(body.clone())
}

fn cover_url(p: &ActiveProfile, id: &str, size: u32) -> String {
    if id.is_empty() { return String::new(); }
    build_url(p, "getCoverArt", &[("id", id), ("size", &size.to_string())])
}

fn stream_url(p: &ActiveProfile, id: &str) -> String {
    if id.is_empty() { return String::new(); }
    build_url(p, "stream", &[("id", id), ("maxBitRate", "320")])
}

fn is_jf(p: &ActiveProfile) -> bool {
    matches!(p.server_type.as_str(), "jellyfin" | "emby")
}

// ── JSON helpers ─────────────────────────────────────────────────────────────

fn s(v: Option<&serde_json::Value>) -> String {
    v.and_then(|x| x.as_str()).unwrap_or("").to_string()
}
fn n(v: Option<&serde_json::Value>) -> f64 {
    v.and_then(|x| x.as_f64()).unwrap_or(0.0)
}
fn arr(v: Option<&serde_json::Value>) -> Vec<serde_json::Value> {
    match v {
        Some(serde_json::Value::Array(a)) => a.clone(),
        Some(item) => vec![item.clone()],
        None => vec![],
    }
}

// ── Output types (canonical — defined in commands::media) ───────────────────
// Song, Album, Playlist, AlbumDetail, AlbumFull, PlaylistDetail, PlaylistMeta
// are all imported from super::media above.

fn map_song(v: &serde_json::Value, p: &ActiveProfile) -> Song {
    let id = s(v.get("id"));
    let cover = s(v.get("coverArt"));
    Song {
        cover_art_url: cover_url(p, &cover, 240),
        stream_url: stream_url(p, &id),
        id,
        title: s(v.get("title")),
        artist: s(v.get("artist")),
        album: s(v.get("album")),
        album_id: s(v.get("albumId")),
        cover_art: cover,
        duration: n(v.get("duration")),
    }
}

fn map_album(v: &serde_json::Value, p: &ActiveProfile, art_size: u32) -> Album {
    let id = s(v.get("id"));
    let cover = { let c = s(v.get("coverArt")); if c.is_empty() { id.clone() } else { c } };
    Album {
        cover_art_url: cover_url(p, &cover, art_size),
        id,
        name: s(v.get("name")),
        artist: s(v.get("artist")),
        artist_id: s(v.get("artistId")),
        cover_art: cover,
        song_count: n(v.get("songCount")),
        duration: n(v.get("duration")),
        year: v.get("year").and_then(|y| y.as_f64()),
    }
}

// ── Commands ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn library_search(
    state: State<'_, AppState>,
    query: String,
    count: Option<u32>,
) -> Result<Vec<Song>, String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_jf(&p) { return crate::commands::jellyfin::search(&state.http, &p, &query, count.unwrap_or(20)).await; }
    let cnt = count.unwrap_or(20).to_string();
    let body = request(&state.http, &p, "search3", &[
        ("query", &query), ("songCount", &cnt), ("artistCount", "0"), ("albumCount", "0"),
    ]).await?;
    Ok(arr(body.get("searchResult3").and_then(|r| r.get("song"))).iter().map(|v| map_song(v, &p)).collect())
}

#[tauri::command]
pub async fn library_similar(
    state: State<'_, AppState>,
    song_id: String,
    count: Option<u32>,
) -> Result<Vec<Song>, String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_jf(&p) { return crate::commands::jellyfin::similar(&state.http, &p, &song_id, count.unwrap_or(20)).await; }
    let cnt = count.unwrap_or(20).to_string();
    let body = request(&state.http, &p, "getSimilarSongs2", &[("id", &song_id), ("count", &cnt)]).await?;
    Ok(arr(body.get("similarSongs2").and_then(|r| r.get("song"))).iter().map(|v| map_song(v, &p)).collect())
}

#[tauri::command]
pub async fn library_playlists(state: State<'_, AppState>) -> Result<Vec<Playlist>, String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_jf(&p) { return crate::commands::jellyfin::playlists(&state.http, &p).await; }
    let body = request(&state.http, &p, "getPlaylists", &[]).await?;
    Ok(arr(body.get("playlists").and_then(|r| r.get("playlist")))
        .iter()
        .map(|pl| {
            let id = s(pl.get("id"));
            let cover = { let c = s(pl.get("coverArt")); if c.is_empty() { id.clone() } else { c } };
            Playlist {
                cover_art_url: cover_url(&p, &cover, 240),
                id,
                name: s(pl.get("name")),
                song_count: n(pl.get("songCount")),
                duration: n(pl.get("duration")),
                cover_art: cover,
            }
        })
        .collect())
}

#[tauri::command]
pub async fn library_playlist(
    state: State<'_, AppState>,
    id: String,
) -> Result<PlaylistDetail, String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_jf(&p) { return crate::commands::jellyfin::playlist(&state.http, &p, &id).await; }
    let body = request(&state.http, &p, "getPlaylist", &[("id", &id)]).await?;
    let pl = body.get("playlist").cloned().unwrap_or(serde_json::Value::Null);
    let songs = arr(pl.get("entry")).iter().map(|v| map_song(v, &p)).collect();
    let pl_id = { let i = s(pl.get("id")); if i.is_empty() { id } else { i } };
    let cover = s(pl.get("coverArt"));
    Ok(PlaylistDetail {
        songs,
        playlist: PlaylistMeta {
            id: pl_id,
            name: s(pl.get("name")),
            song_count: n(pl.get("songCount")),
            duration: n(pl.get("duration")),
            cover_art_url: cover_url(&p, &cover, 240),
        },
    })
}

#[tauri::command]
pub async fn library_artist_albums(
    state: State<'_, AppState>,
    query: String,
    count: Option<u32>,
) -> Result<Vec<Album>, String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_jf(&p) { return crate::commands::jellyfin::artist_albums(&state.http, &p, &query, count.unwrap_or(20)).await; }
    let cnt = count.unwrap_or(20).to_string();
    let body = request(&state.http, &p, "search3", &[
        ("query", &query), ("artistCount", "0"), ("albumCount", &cnt), ("songCount", "0"),
    ]).await?;
    Ok(arr(body.get("searchResult3").and_then(|r| r.get("album"))).iter().map(|v| map_album(v, &p, 300)).collect())
}

#[tauri::command]
pub async fn library_album_songs(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<Song>, String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_jf(&p) { return crate::commands::jellyfin::album_songs(&state.http, &p, &id).await; }
    let body = request(&state.http, &p, "getAlbum", &[("id", &id)]).await?;
    Ok(arr(body.get("album").and_then(|a| a.get("song")))
        .iter()
        .map(|v| {
            let mut song = map_song(v, &p);
            if song.album_id.is_empty() { song.album_id = id.clone(); }
            song
        })
        .collect())
}

#[tauri::command]
pub async fn library_album(
    state: State<'_, AppState>,
    id: String,
) -> Result<AlbumDetail, String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_jf(&p) { return crate::commands::jellyfin::album(&state.http, &p, &id).await; }
    let body = request(&state.http, &p, "getAlbum", &[("id", &id)]).await?;
    let al = body.get("album").cloned().unwrap_or(serde_json::Value::Null);
    let al_id = { let i = s(al.get("id")); if i.is_empty() { id } else { i } };
    let al_name = s(al.get("name"));
    let al_cover = s(al.get("coverArt"));
    let songs: Vec<Song> = arr(al.get("song"))
        .iter()
        .map(|v| {
            let mut song = map_song(v, &p);
            if song.album.is_empty() { song.album = al_name.clone(); }
            if song.album_id.is_empty() { song.album_id = al_id.clone(); }
            if song.cover_art.is_empty() { song.cover_art = al_cover.clone(); }
            let c = if song.cover_art.is_empty() { al_cover.clone() } else { song.cover_art.clone() };
            song.cover_art_url = cover_url(&p, &c, 240);
            song
        })
        .collect();
    let cover = if al_cover.is_empty() { al_id.clone() } else { al_cover.clone() };
    Ok(AlbumDetail {
        songs,
        album: AlbumFull {
            id: al_id,
            name: al_name,
            artist: s(al.get("artist")),
            artist_id: s(al.get("artistId")),
            cover_art_url: cover_url(&p, &cover, 400),
            cover_art: cover,
            song_count: n(al.get("songCount")),
            duration: n(al.get("duration")),
            year: al.get("year").and_then(|y| y.as_f64()),
            genre: al.get("genre").and_then(|g| g.as_str()).map(String::from),
        },
    })
}

#[tauri::command]
pub async fn library_album_list(
    state: State<'_, AppState>,
    kind: Option<String>,
    count: Option<u32>,
) -> Result<Vec<Album>, String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_jf(&p) { return crate::commands::jellyfin::album_list(&state.http, &p, &kind.unwrap_or_else(|| "newest".to_string()), count.unwrap_or(20).min(100)).await; }
    let kind = kind.unwrap_or_else(|| "newest".to_string());
    let cnt = count.unwrap_or(20).min(100).to_string();
    let body = request(&state.http, &p, "getAlbumList2", &[("type", &kind), ("size", &cnt)]).await?;
    Ok(arr(body.get("albumList2").and_then(|r| r.get("album"))).iter().map(|v| map_album(v, &p, 240)).collect())
}

#[tauri::command]
pub async fn library_starred(state: State<'_, AppState>) -> Result<Vec<Song>, String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_jf(&p) { return crate::commands::jellyfin::starred(&state.http, &p).await; }
    let body = request(&state.http, &p, "getStarred2", &[]).await?;
    Ok(arr(body.get("starred2").and_then(|r| r.get("song"))).iter().map(|v| map_song(v, &p)).collect())
}

#[tauri::command]
pub async fn library_star(
    state: State<'_, AppState>,
    id: String,
    unstar: Option<bool>,
    artist: Option<String>,
    title: Option<String>,
) -> Result<(), String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_jf(&p) {
        crate::commands::jellyfin::star(&state.http, &p, &id, unstar.unwrap_or(false)).await?;
        // Mirror to Last.fm regardless of server type
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
    if id.is_empty() { return Err("Song id is required.".to_string()); }
    let method = if unstar.unwrap_or(false) { "unstar" } else { "star" };
    request(&state.http, &p, method, &[("id", &id)]).await?;

    // Mirror to Last.fm track.love / track.unlove
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
) -> Result<(), String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_jf(&p) { return crate::commands::jellyfin::add_to_playlist(&state.http, &p, &playlist_id, &song_id).await; }
    if playlist_id.is_empty() || song_id.is_empty() {
        return Err("playlistId and songId are required.".to_string());
    }
    request(&state.http, &p, "updatePlaylist", &[
        ("playlistId", &playlist_id), ("songIdToAdd", &song_id),
    ]).await?;
    Ok(())
}
