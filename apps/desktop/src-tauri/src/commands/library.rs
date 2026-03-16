use rand::Rng;
use std::collections::HashMap;
use tauri::State;
use url::Url;
use crate::{AppState, commands::profiles::{get_active_profile, ActiveProfile}};
use super::media::{Song, Album, Playlist, PlaylistDetail, PlaylistMeta, AlbumDetail, AlbumFull, SearchBundle};

const API_VERSION: &str = "1.16.1";
const CLIENT_NAME: &str = "madrify";

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

pub(crate) async fn request_binary(
    http: &reqwest::Client,
    p: &ActiveProfile,
    path: &str,
    params: &[(&str, &str)],
) -> Result<(), String> {
    let url = build_url(p, path, params);
    let resp = http.get(&url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Subsonic: HTTP {}", resp.status()));
    }
    let _ = resp.bytes().await.map_err(|e| e.to_string())?;
    Ok(())
}

fn cover_url(p: &ActiveProfile, id: &str, size: u32) -> String {
    if id.is_empty() { return String::new(); }
    build_url(p, "getCoverArt", &[("id", id), ("size", &size.to_string())])
}

fn stream_url(p: &ActiveProfile, id: &str) -> String {
    if id.is_empty() { return String::new(); }
    build_url(p, "download", &[("id", id)])
}

fn is_jf(p: &ActiveProfile) -> bool {
    matches!(p.server_type.as_str(), "jellyfin" | "emby")
}

fn is_local(p: &ActiveProfile) -> bool {
    p.server_type == "local"
}

fn is_remote_cover_url(url: &str) -> bool {
    Url::parse(url)
        .map(|parsed| matches!(parsed.scheme(), "http" | "https"))
        .unwrap_or(false)
}

async fn cache_cover_url(
    state: &State<'_, AppState>,
    cache_key: &str,
    cover_art_url: &str,
) -> String {
    if cover_art_url.is_empty() || !is_remote_cover_url(cover_art_url) {
        return cover_art_url.to_string();
    }

    match state
        .artwork_cache
        .get_or_fetch_remote(&state.http, cache_key, "artwork", cover_art_url)
        .await
    {
        Ok(local_url) => local_url,
        Err(_) => cover_art_url.to_string(),
    }
}

async fn cache_song_covers(state: &State<'_, AppState>, songs: Vec<Song>) -> Vec<Song> {
    let mut cached = Vec::with_capacity(songs.len());
    for mut song in songs {
        let key = format!("song-cover-{}-240", if song.cover_art.is_empty() { &song.id } else { &song.cover_art });
        song.cover_art_url = cache_cover_url(state, &key, &song.cover_art_url).await;
        cached.push(song);
    }
    cached
}

async fn cache_album_covers(state: &State<'_, AppState>, albums: Vec<Album>) -> Vec<Album> {
    let mut cached = Vec::with_capacity(albums.len());
    for mut album in albums {
        let key = format!("album-cover-{}-400", if album.cover_art.is_empty() { &album.id } else { &album.cover_art });
        album.cover_art_url = cache_cover_url(state, &key, &album.cover_art_url).await;
        cached.push(album);
    }
    cached
}

async fn cache_playlist_covers(
    state: &State<'_, AppState>,
    playlists: Vec<Playlist>,
) -> Vec<Playlist> {
    let mut cached = Vec::with_capacity(playlists.len());
    for mut playlist in playlists {
        let key = format!("playlist-cover-{}-240", if playlist.cover_art.is_empty() { &playlist.id } else { &playlist.cover_art });
        playlist.cover_art_url = cache_cover_url(state, &key, &playlist.cover_art_url).await;
        cached.push(playlist);
    }
    cached
}

async fn cache_album_detail(state: &State<'_, AppState>, mut detail: AlbumDetail) -> AlbumDetail {
    let key = format!(
        "album-cover-{}-400",
        if detail.album.cover_art.is_empty() { &detail.album.id } else { &detail.album.cover_art }
    );
    detail.album.cover_art_url = cache_cover_url(state, &key, &detail.album.cover_art_url).await;
    detail.songs = cache_song_covers(state, detail.songs).await;
    detail
}

async fn cache_playlist_detail(
    state: &State<'_, AppState>,
    mut detail: PlaylistDetail,
) -> PlaylistDetail {
    let key = format!("playlist-cover-{}-240", detail.playlist.id);
    detail.playlist.cover_art_url = cache_cover_url(state, &key, &detail.playlist.cover_art_url).await;
    detail.songs = cache_song_covers(state, detail.songs).await;
    detail
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

fn normalize_match(value: &str) -> String {
    value.trim().to_lowercase()
}

fn best_materialized_match(candidates: Vec<Song>, song: &Song) -> Option<Song> {
    let title = normalize_match(&song.title);
    let artist = normalize_match(&song.artist);

    candidates
        .iter()
        .find(|candidate| {
            !candidate.id.starts_with("ext-")
                && normalize_match(&candidate.title) == title
                && normalize_match(&candidate.artist) == artist
        })
        .cloned()
        .or_else(|| {
            candidates
                .iter()
                .find(|candidate| {
                    !candidate.id.starts_with("ext-")
                        && normalize_match(&candidate.title).contains(&title)
                        && (artist.is_empty()
                            || normalize_match(&candidate.artist) == artist
                            || normalize_match(&candidate.artist).contains(&artist))
                })
                .cloned()
        })
}

pub(crate) async fn resolve_playback_song(
    state: &State<'_, AppState>,
    song: &Song,
) -> Result<Song, String> {
    if !song.id.starts_with("ext-") {
        return Ok(song.clone());
    }

    let p = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        get_active_profile(&db)?
    };

    if is_jf(&p) {
        return Err("External playback materialization is only supported for Subsonic-compatible servers.".to_string());
    }

    request_binary(
        &state.http,
        &p,
        "stream",
        &[("id", song.id.as_str()), ("maxBitRate", "320")],
    )
    .await?;

    let query = format!("{} {}", song.artist, song.title).trim().to_string();
    for _ in 0..8 {
        let body = request(
            &state.http,
            &p,
            "search3",
            &[("query", &query), ("songCount", "10"), ("artistCount", "0"), ("albumCount", "0")],
        )
        .await?;
        let candidates: Vec<Song> = arr(body.get("searchResult3").and_then(|r| r.get("song")))
            .iter()
            .map(|value| map_song(value, &p))
            .collect();

        if let Some(resolved) = best_materialized_match(candidates, song) {
            return Ok(resolved);
        }

        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    }

    Err(format!(
        "Could not materialize \"{}\" by {} for playback",
        song.title, song.artist
    ))
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
    };

    let resolved = resolve_playback_song(state, &lookup_song).await?;
    Ok(resolved.id)
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
    if is_local(&p) { return crate::commands::local::search(&p, &query, count.unwrap_or(20)).await; }
    if is_jf(&p) {
        let songs = crate::commands::jellyfin::search(&state.http, &p, &query, count.unwrap_or(20)).await?;
        return Ok(cache_song_covers(&state, songs).await);
    }
    let cnt = count.unwrap_or(20).to_string();
    let body = request(&state.http, &p, "search3", &[
        ("query", &query), ("songCount", &cnt), ("artistCount", "0"), ("albumCount", "0"),
    ]).await?;
    let songs = arr(body.get("searchResult3").and_then(|r| r.get("song"))).iter().map(|v| map_song(v, &p)).collect();
    Ok(cache_song_covers(&state, songs).await)
}

#[tauri::command]
pub async fn library_search_bundle(
    state: State<'_, AppState>,
    query: String,
    song_count: Option<u32>,
    album_count: Option<u32>,
    recommendation_count: Option<u32>,
) -> Result<SearchBundle, String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    let song_limit = song_count.unwrap_or(24);
    let album_limit = album_count.unwrap_or(12);
    let rec_limit = recommendation_count.unwrap_or(16);

    let songs = if is_local(&p) {
        crate::commands::local::search(&p, &query, song_limit).await?
    } else if is_jf(&p) {
        let songs = crate::commands::jellyfin::search(&state.http, &p, &query, song_limit).await?;
        cache_song_covers(&state, songs).await
    } else {
        let cnt = song_limit.to_string();
        let body = request(&state.http, &p, "search3", &[
            ("query", &query), ("songCount", &cnt), ("artistCount", "0"), ("albumCount", "0"),
        ]).await?;
        let songs: Vec<Song> = arr(body.get("searchResult3").and_then(|r| r.get("song")))
            .iter()
            .map(|v| map_song(v, &p))
            .collect();
        cache_song_covers(&state, songs).await
    };

    let albums = if is_local(&p) {
        crate::commands::local::artist_albums(&p, &query, album_limit).await?
    } else if is_jf(&p) {
        let albums = crate::commands::jellyfin::artist_albums(&state.http, &p, &query, album_limit).await?;
        cache_album_covers(&state, albums).await
    } else {
        let cnt = album_limit.to_string();
        let body = request(&state.http, &p, "search3", &[
            ("query", &query), ("artistCount", "0"), ("albumCount", &cnt), ("songCount", "0"),
        ]).await?;
        let albums: Vec<Album> = arr(body.get("searchResult3").and_then(|r| r.get("album")))
            .iter()
            .map(|v| map_album(v, &p, 300))
            .collect();
        cache_album_covers(&state, albums).await
    };

    let recommendations = if let Some(seed) = songs.first() {
        let recs = if is_local(&p) {
            crate::commands::local::similar(&p, &seed.id, rec_limit).await?
        } else if is_jf(&p) {
            crate::commands::jellyfin::similar(&state.http, &p, &seed.id, rec_limit).await?
        } else {
            let cnt = rec_limit.to_string();
            let body = request(&state.http, &p, "getSimilarSongs2", &[("id", &seed.id), ("count", &cnt)]).await?;
            arr(body.get("similarSongs2").and_then(|r| r.get("song")))
                .iter()
                .map(|v| map_song(v, &p))
                .collect()
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
            cache_song_covers(&state, deduped).await
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
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_local(&p) { return crate::commands::local::similar(&p, &song_id, count.unwrap_or(20)).await; }
    if is_jf(&p) {
        let songs = crate::commands::jellyfin::similar(&state.http, &p, &song_id, count.unwrap_or(20)).await?;
        return Ok(cache_song_covers(&state, songs).await);
    }
    let cnt = count.unwrap_or(20).to_string();
    let body = request(&state.http, &p, "getSimilarSongs2", &[("id", &song_id), ("count", &cnt)]).await?;
    let songs = arr(body.get("similarSongs2").and_then(|r| r.get("song"))).iter().map(|v| map_song(v, &p)).collect();
    Ok(cache_song_covers(&state, songs).await)
}

#[tauri::command]
pub async fn library_playlists(state: State<'_, AppState>) -> Result<Vec<Playlist>, String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_local(&p) { return crate::commands::local::playlists(&p).await; }
    if is_jf(&p) {
        let playlists = crate::commands::jellyfin::playlists(&state.http, &p).await?;
        return Ok(cache_playlist_covers(&state, playlists).await);
    }
    let body = request(&state.http, &p, "getPlaylists", &[]).await?;
    let playlists = arr(body.get("playlists").and_then(|r| r.get("playlist")))
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
        .collect();
    Ok(cache_playlist_covers(&state, playlists).await)
}

#[tauri::command]
pub async fn library_playlist(
    state: State<'_, AppState>,
    id: String,
) -> Result<PlaylistDetail, String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_local(&p) { return crate::commands::local::playlist(&p, &id).await; }
    if is_jf(&p) {
        let detail = crate::commands::jellyfin::playlist(&state.http, &p, &id).await?;
        return Ok(cache_playlist_detail(&state, detail).await);
    }
    let body = request(&state.http, &p, "getPlaylist", &[("id", &id)]).await?;
    let pl = body.get("playlist").cloned().unwrap_or(serde_json::Value::Null);
    let songs = arr(pl.get("entry")).iter().map(|v| map_song(v, &p)).collect();
    let pl_id = { let i = s(pl.get("id")); if i.is_empty() { id } else { i } };
    let cover = s(pl.get("coverArt"));
    let detail = PlaylistDetail {
        songs,
        playlist: PlaylistMeta {
            id: pl_id,
            name: s(pl.get("name")),
            song_count: n(pl.get("songCount")),
            duration: n(pl.get("duration")),
            cover_art_url: cover_url(&p, &cover, 240),
        },
    };
    Ok(cache_playlist_detail(&state, detail).await)
}

#[tauri::command]
pub async fn library_artist_albums(
    state: State<'_, AppState>,
    query: String,
    count: Option<u32>,
) -> Result<Vec<Album>, String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_local(&p) { return crate::commands::local::artist_albums(&p, &query, count.unwrap_or(20)).await; }
    if is_jf(&p) {
        let albums = crate::commands::jellyfin::artist_albums(&state.http, &p, &query, count.unwrap_or(20)).await?;
        return Ok(cache_album_covers(&state, albums).await);
    }
    let cnt = count.unwrap_or(20).to_string();
    let body = request(&state.http, &p, "search3", &[
        ("query", &query), ("artistCount", "0"), ("albumCount", &cnt), ("songCount", "0"),
    ]).await?;
    let albums = arr(body.get("searchResult3").and_then(|r| r.get("album"))).iter().map(|v| map_album(v, &p, 300)).collect();
    Ok(cache_album_covers(&state, albums).await)
}

#[tauri::command]
pub async fn library_album_songs(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<Song>, String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_local(&p) { return crate::commands::local::album_songs(&p, &id).await; }
    if is_jf(&p) {
        let songs = crate::commands::jellyfin::album_songs(&state.http, &p, &id).await?;
        return Ok(cache_song_covers(&state, songs).await);
    }
    let body = request(&state.http, &p, "getAlbum", &[("id", &id)]).await?;
    let songs = arr(body.get("album").and_then(|a| a.get("song")))
        .iter()
        .map(|v| {
            let mut song = map_song(v, &p);
            if song.album_id.is_empty() { song.album_id = id.clone(); }
            song
        })
        .collect();
    Ok(cache_song_covers(&state, songs).await)
}

#[tauri::command]
pub async fn library_album(
    state: State<'_, AppState>,
    id: String,
) -> Result<AlbumDetail, String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_local(&p) { return crate::commands::local::album(&p, &id).await; }
    if is_jf(&p) {
        let detail = crate::commands::jellyfin::album(&state.http, &p, &id).await?;
        return Ok(cache_album_detail(&state, detail).await);
    }
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
    let detail = AlbumDetail {
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
    };
    Ok(cache_album_detail(&state, detail).await)
}

#[tauri::command]
pub async fn library_album_list(
    state: State<'_, AppState>,
    kind: Option<String>,
    count: Option<u32>,
) -> Result<Vec<Album>, String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_local(&p) { return crate::commands::local::album_list(&p, &kind.clone().unwrap_or_else(|| "newest".to_string()), count.unwrap_or(20).min(100)).await; }
    if is_jf(&p) {
        let albums = crate::commands::jellyfin::album_list(&state.http, &p, &kind.unwrap_or_else(|| "newest".to_string()), count.unwrap_or(20).min(100)).await?;
        return Ok(cache_album_covers(&state, albums).await);
    }
    let kind = kind.unwrap_or_else(|| "newest".to_string());
    let cnt = count.unwrap_or(20).min(100).to_string();
    let body = request(&state.http, &p, "getAlbumList2", &[("type", &kind), ("size", &cnt)]).await?;
    let albums = arr(body.get("albumList2").and_then(|r| r.get("album"))).iter().map(|v| map_album(v, &p, 240)).collect();
    Ok(cache_album_covers(&state, albums).await)
}

#[tauri::command]
pub async fn library_starred(state: State<'_, AppState>) -> Result<Vec<Song>, String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_local(&p) { return crate::commands::local::starred(&p).await; }
    if is_jf(&p) {
        let songs = crate::commands::jellyfin::starred(&state.http, &p).await?;
        return Ok(cache_song_covers(&state, songs).await);
    }
    let body = request(&state.http, &p, "getStarred2", &[]).await?;
    let songs = arr(body.get("starred2").and_then(|r| r.get("song"))).iter().map(|v| map_song(v, &p)).collect();
    Ok(cache_song_covers(&state, songs).await)
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
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_local(&p) {
        return Err("Favorites are not supported for local libraries yet.".to_string());
    }
    let resolved_id = resolve_library_song_id(&state, &id, artist.as_deref(), title.as_deref(), album.as_deref()).await?;
    if is_jf(&p) {
        crate::commands::jellyfin::star(&state.http, &p, &resolved_id, unstar.unwrap_or(false)).await?;
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
    if resolved_id.is_empty() { return Err("Song id is required.".to_string()); }
    let method = if unstar.unwrap_or(false) { "unstar" } else { "star" };
    request(&state.http, &p, method, &[("id", &resolved_id)]).await?;

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
    artist: Option<String>,
    title: Option<String>,
    album: Option<String>,
) -> Result<(), String> {
    let p = { let db = state.db.lock().map_err(|e| e.to_string())?; get_active_profile(&db)? };
    if is_local(&p) {
        return Err("Playlists are not supported for local libraries yet.".to_string());
    }
    let resolved_id = resolve_library_song_id(&state, &song_id, artist.as_deref(), title.as_deref(), album.as_deref()).await?;
    if is_jf(&p) { return crate::commands::jellyfin::add_to_playlist(&state.http, &p, &playlist_id, &resolved_id).await; }
    if playlist_id.is_empty() || resolved_id.is_empty() {
        return Err("playlistId and songId are required.".to_string());
    }
    request(&state.http, &p, "updatePlaylist", &[
        ("playlistId", &playlist_id), ("songIdToAdd", &resolved_id),
    ]).await?;
    Ok(())
}

#[tauri::command]
pub async fn library_create_playlist(
    state: State<'_, AppState>,
    name: String,
    song_ids: Vec<String>,
) -> Result<Playlist, String> {
    let p = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        get_active_profile(&db)?
    };

    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err("Playlist name is required.".to_string());
    }
    if song_ids.is_empty() {
        return Err("At least one song is required.".to_string());
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

    let mut params: Vec<(&str, &str)> = vec![("name", trimmed_name)];
    for song_id in &song_ids {
        params.push(("songId", song_id.as_str()));
    }
    let body = request(&state.http, &p, "createPlaylist", &params).await?;
    let playlist = body.get("playlist").cloned().unwrap_or(serde_json::Value::Null);
    let id = s(playlist.get("id"));
    let cover = {
        let c = s(playlist.get("coverArt"));
        if c.is_empty() { id.clone() } else { c }
    };

    let playlist = Playlist {
        cover_art_url: cover_url(&p, &cover, 240),
        id,
        name: {
            let created_name = s(playlist.get("name"));
            if created_name.is_empty() { trimmed_name.to_string() } else { created_name }
        },
        song_count: n(playlist.get("songCount")).max(song_ids.len() as f64),
        duration: n(playlist.get("duration")),
        cover_art: cover,
    };
    Ok(cache_playlist_covers(&state, vec![playlist]).await.into_iter().next().unwrap())
}

#[tauri::command]
pub async fn library_materialize_song(
    state: State<'_, AppState>,
    song_id: String,
) -> Result<(), String> {
    let p = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        get_active_profile(&db)?
    };

    if is_jf(&p) {
        return Err("Materializing external tracks is only supported for Subsonic-compatible servers.".to_string());
    }
    if is_local(&p) {
        return Err("Local library tracks do not need materialization.".to_string());
    }
    if song_id.trim().is_empty() {
        return Err("Song id is required.".to_string());
    }

    request_binary(
        &state.http,
        &p,
        "stream",
        &[("id", song_id.as_str()), ("maxBitRate", "320")],
    )
    .await
}
