use std::collections::HashMap;
use serde::Serialize;
use tauri::State;
use crate::{AppState, commands::settings};

const AUDIODB_API: &str = "https://www.theaudiodb.com/api/v1/json/2";
const LASTFM_API: &str = "https://ws.audioscrobbler.com/2.0/";
const LBZ_API: &str = "https://api.listenbrainz.org/1";

// ── Helpers ───────────────────────────────────────────────────────────────────

fn get_lfm_api_key(state: &AppState) -> String {
    let Ok(db) = state.db.lock() else { return String::new() };
    let Ok(s) = settings::read_all(&db) else { return String::new() };
    s.get("LASTFM_API_KEY").cloned().unwrap_or_default()
}

fn get_lbz_credentials(state: &AppState) -> (String, String) {
    let Ok(db) = state.db.lock() else { return (String::new(), String::new()) };
    let Ok(s) = settings::read_all(&db) else { return (String::new(), String::new()) };
    let token = s.get("LISTENBRAINZ_TOKEN").cloned().unwrap_or_default();
    let username = s.get("LISTENBRAINZ_USERNAME").cloned().unwrap_or_default();
    (token, username)
}

async fn lfm_get(
    http: &reqwest::Client,
    mut params: HashMap<String, String>,
    api_key: &str,
) -> Result<serde_json::Value, String> {
    params.insert("api_key".to_string(), api_key.to_string());
    params.insert("format".to_string(), "json".to_string());
    let resp = http
        .get(LASTFM_API)
        .query(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if let Some(code) = json.get("error") {
        let msg = json.get("message").and_then(|m| m.as_str()).unwrap_or("Last.fm error");
        return Err(format!("Last.fm error {code}: {msg}"));
    }
    Ok(json)
}

// ── AudioDB ───────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDbArtist {
    pub name: String,
    pub thumb: String,
    pub fanart: String,
    pub banner: String,
    pub biography: String,
    pub genre: String,
    pub country: String,
    pub formed_year: String,
}

#[tauri::command]
pub async fn audiodb_artist(
    state: State<'_, AppState>,
    name: String,
) -> Result<Option<AudioDbArtist>, String> {
    if name.trim().is_empty() {
        return Ok(None);
    }
    let url = format!("{}/search.php", AUDIODB_API);
    let resp = state.http
        .get(&url)
        .query(&[("s", name.trim())])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let a = match json.get("artists").and_then(|arr| arr.get(0)) {
        Some(v) => v.clone(),
        None => return Ok(None),
    };
    let s = |key: &str| a.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string();
    Ok(Some(AudioDbArtist {
        name: s("strArtist"),
        thumb: s("strArtistThumb"),
        fanart: s("strArtistFanart"),
        banner: s("strArtistBanner"),
        biography: s("strBiographyEN"),
        genre: s("strGenre"),
        country: s("strCountry"),
        formed_year: s("intFormedYear"),
    }))
}

// ── Last.fm metadata ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn lfm_chart_top_artists(
    state: State<'_, AppState>,
    limit: u32,
) -> Result<serde_json::Value, String> {
    let key = get_lfm_api_key(&state);
    if key.is_empty() { return Err("No Last.fm API key configured".to_string()); }
    let mut p = HashMap::new();
    p.insert("method".to_string(), "chart.gettopartists".to_string());
    p.insert("limit".to_string(), limit.to_string());
    lfm_get(&state.http, p, &key).await
}

#[tauri::command]
pub async fn lfm_artist_search(
    state: State<'_, AppState>,
    artist: String,
    limit: u32,
) -> Result<serde_json::Value, String> {
    let key = get_lfm_api_key(&state);
    if key.is_empty() { return Err("No Last.fm API key configured".to_string()); }
    let mut p = HashMap::new();
    p.insert("method".to_string(), "artist.search".to_string());
    p.insert("artist".to_string(), artist);
    p.insert("limit".to_string(), limit.to_string());
    lfm_get(&state.http, p, &key).await
}

#[tauri::command]
pub async fn lfm_chart_top_tracks(
    state: State<'_, AppState>,
    limit: u32,
) -> Result<serde_json::Value, String> {
    let key = get_lfm_api_key(&state);
    if key.is_empty() { return Err("No Last.fm API key configured".to_string()); }
    let mut p = HashMap::new();
    p.insert("method".to_string(), "chart.gettoptracks".to_string());
    p.insert("limit".to_string(), limit.to_string());
    lfm_get(&state.http, p, &key).await
}

#[tauri::command]
pub async fn lfm_track_search(
    state: State<'_, AppState>,
    track: String,
    limit: u32,
) -> Result<serde_json::Value, String> {
    let key = get_lfm_api_key(&state);
    if key.is_empty() { return Err("No Last.fm API key configured".to_string()); }
    let mut p = HashMap::new();
    p.insert("method".to_string(), "track.search".to_string());
    p.insert("track".to_string(), track);
    p.insert("limit".to_string(), limit.to_string());
    lfm_get(&state.http, p, &key).await
}

#[tauri::command]
pub async fn lfm_artist_top_tracks(
    state: State<'_, AppState>,
    artist: String,
    limit: u32,
) -> Result<serde_json::Value, String> {
    let key = get_lfm_api_key(&state);
    if key.is_empty() { return Err("No Last.fm API key configured".to_string()); }
    let mut p = HashMap::new();
    p.insert("method".to_string(), "artist.getTopTracks".to_string());
    p.insert("artist".to_string(), artist);
    p.insert("autocorrect".to_string(), "1".to_string());
    p.insert("limit".to_string(), limit.to_string());
    lfm_get(&state.http, p, &key).await
}

#[tauri::command]
pub async fn lfm_tag_top_tags(
    state: State<'_, AppState>,
    limit: u32,
) -> Result<serde_json::Value, String> {
    let key = get_lfm_api_key(&state);
    if key.is_empty() { return Err("No Last.fm API key configured".to_string()); }
    let mut p = HashMap::new();
    p.insert("method".to_string(), "tag.getTopTags".to_string());
    p.insert("limit".to_string(), limit.to_string());
    lfm_get(&state.http, p, &key).await
}

#[tauri::command]
pub async fn lfm_track_top_tags(
    state: State<'_, AppState>,
    artist: String,
    track: String,
) -> Result<serde_json::Value, String> {
    let key = get_lfm_api_key(&state);
    if key.is_empty() { return Err("No Last.fm API key configured".to_string()); }
    let mut p = HashMap::new();
    p.insert("method".to_string(), "track.getTopTags".to_string());
    p.insert("artist".to_string(), artist);
    p.insert("track".to_string(), track);
    p.insert("autocorrect".to_string(), "1".to_string());
    lfm_get(&state.http, p, &key).await
}

#[tauri::command]
pub async fn lfm_artist_info(
    state: State<'_, AppState>,
    artist: String,
) -> Result<serde_json::Value, String> {
    let key = get_lfm_api_key(&state);
    if key.is_empty() { return Err("No Last.fm API key configured".to_string()); }
    let mut p = HashMap::new();
    p.insert("method".to_string(), "artist.getinfo".to_string());
    p.insert("artist".to_string(), artist);
    p.insert("autocorrect".to_string(), "1".to_string());
    lfm_get(&state.http, p, &key).await
}

#[tauri::command]
pub async fn lfm_similar_tracks(
    state: State<'_, AppState>,
    artist: String,
    track: String,
    limit: u32,
) -> Result<serde_json::Value, String> {
    let key = get_lfm_api_key(&state);
    if key.is_empty() { return Err("No Last.fm API key configured".to_string()); }
    let mut p = HashMap::new();
    p.insert("method".to_string(), "track.getSimilar".to_string());
    p.insert("artist".to_string(), artist);
    p.insert("track".to_string(), track);
    p.insert("autocorrect".to_string(), "1".to_string());
    p.insert("limit".to_string(), limit.to_string());
    lfm_get(&state.http, p, &key).await
}

#[tauri::command]
pub async fn lfm_tag_top_tracks(
    state: State<'_, AppState>,
    tag: String,
    limit: u32,
) -> Result<serde_json::Value, String> {
    let key = get_lfm_api_key(&state);
    if key.is_empty() { return Err("No Last.fm API key configured".to_string()); }
    let mut p = HashMap::new();
    p.insert("method".to_string(), "tag.gettoptracks".to_string());
    p.insert("tag".to_string(), tag);
    p.insert("limit".to_string(), limit.to_string());
    lfm_get(&state.http, p, &key).await
}

// ── ListenBrainz ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn lbz_cf_recommendations(
    state: State<'_, AppState>,
    count: u32,
) -> Result<serde_json::Value, String> {
    let (_, username) = get_lbz_credentials(&state);
    if username.trim().is_empty() {
        return Err("ListenBrainz username is not configured".to_string());
    }
    let url = format!(
        "{}/cf/recommendation/user/{}/recording?count={}",
        LBZ_API,
        username.trim(),
        count
    );
    let resp = state.http.get(&url).send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        if status == 404 {
            return Err(format!("ListenBrainz user \"{}\" not found", username.trim()));
        }
        return Err(format!("ListenBrainz request failed with status {status}"));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(json)
}

#[tauri::command]
pub async fn lbz_recording_metadata(
    state: State<'_, AppState>,
    mbids: Vec<String>,
) -> Result<serde_json::Value, String> {
    let url = format!("{}/metadata/recording/", LBZ_API);
    let body = serde_json::json!({ "recording_mbids": mbids });
    let resp = state.http
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("ListenBrainz metadata request failed with status {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(json)
}

#[tauri::command]
pub async fn lbz_now_playing(
    state: State<'_, AppState>,
    artist: String,
    title: String,
    album: Option<String>,
    duration_ms: Option<u64>,
) -> Result<(), String> {
    let (token, _) = get_lbz_credentials(&state);
    if token.trim().is_empty() {
        return Ok(());
    }
    let track_meta = build_lbz_track_meta(&artist, &title, album.as_deref(), duration_ms);
    let body = serde_json::json!({
        "listen_type": "playing_now",
        "payload": [{ "track_metadata": track_meta }]
    });
    let _ = state.http
        .post(format!("{}/submit-listens", LBZ_API))
        .header("Authorization", format!("Token {}", token.trim()))
        .json(&body)
        .send()
        .await;
    Ok(())
}

#[tauri::command]
pub async fn lbz_scrobble(
    state: State<'_, AppState>,
    artist: String,
    title: String,
    listened_at: i64,
    album: Option<String>,
    duration_ms: Option<u64>,
) -> Result<(), String> {
    let (token, _) = get_lbz_credentials(&state);
    if token.trim().is_empty() {
        return Ok(());
    }
    let track_meta = build_lbz_track_meta(&artist, &title, album.as_deref(), duration_ms);
    let body = serde_json::json!({
        "listen_type": "single",
        "payload": [{
            "listened_at": listened_at,
            "track_metadata": track_meta
        }]
    });
    let _ = state.http
        .post(format!("{}/submit-listens", LBZ_API))
        .header("Authorization", format!("Token {}", token.trim()))
        .json(&body)
        .send()
        .await;
    Ok(())
}

fn build_lbz_track_meta(
    artist: &str,
    title: &str,
    album: Option<&str>,
    duration_ms: Option<u64>,
) -> serde_json::Value {
    let mut meta = serde_json::json!({
        "artist_name": artist,
        "track_name": title,
    });
    if let Some(album) = album.filter(|a| !a.is_empty()) {
        meta["release_name"] = serde_json::Value::String(album.to_string());
    }
    if let Some(ms) = duration_ms {
        meta["additional_info"] = serde_json::json!({ "duration_ms": ms });
    }
    meta
}
