use std::collections::HashMap;
use std::time::Duration;
use lastfm_client::LastFmClient;
use lastfm_client::api::Period;
use rustfm_scrobble::{Scrobble, Scrobbler};
use serde::Serialize;
use tauri::State;
use crate::{AppState, commands::settings};

const LASTFM_API: &str = "https://ws.audioscrobbler.com/2.0/";

// ── Signing ──────────────────────────────────────────────────────────────────

fn sign(params: &HashMap<String, String>, secret: &str) -> String {
    let mut pairs: Vec<(&String, &String)> = params
        .iter()
        .filter(|(k, _)| *k != "format" && *k != "callback")
        .collect();
    pairs.sort_by_key(|(k, _)| k.as_str());
    let mut s = String::new();
    for (k, v) in pairs {
        s.push_str(k);
        s.push_str(v);
    }
    s.push_str(secret);
    format!("{:x}", md5::compute(s.as_bytes()))
}

/// Public so `subsonic.rs` can call it for track.love / track.unlove.
pub async fn sign_and_post(
    http: &reqwest::Client,
    mut params: HashMap<String, String>,
    api_key: &str,
    secret: &str,
) -> Result<serde_json::Value, String> {
    params.insert("api_key".to_string(), api_key.to_string());
    let sig = sign(&params, secret);
    params.insert("api_sig".to_string(), sig);
    params.insert("format".to_string(), "json".to_string());

    let resp = http
        .post(LASTFM_API)
        .form(&params)
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

async fn sign_and_get(
    http: &reqwest::Client,
    mut params: HashMap<String, String>,
    api_key: &str,
    secret: &str,
) -> Result<serde_json::Value, String> {
    params.insert("api_key".to_string(), api_key.to_string());
    let sig = sign(&params, secret);
    params.insert("api_sig".to_string(), sig);
    params.insert("format".to_string(), "json".to_string());

    let resp = http.get(LASTFM_API).query(&params).send().await.map_err(|e| e.to_string())?;
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if let Some(code) = json.get("error") {
        let msg = json.get("message").and_then(|m| m.as_str()).unwrap_or("Last.fm error");
        return Err(format!("Last.fm error {code}: {msg}"));
    }
    Ok(json)
}

fn require_credentials(s: &HashMap<String, String>) -> Result<(String, String), String> {
    let key = s.get("LASTFM_API_KEY").cloned().unwrap_or_default();
    let secret = s.get("LASTFM_SHARED_SECRET").cloned().unwrap_or_default();
    if key.is_empty() || secret.is_empty() {
        return Err("Last.fm API key and shared secret are required".to_string());
    }
    Ok((key, secret))
}

fn build_scrobbler(api_key: &str, secret: &str, session_key: Option<&str>) -> Scrobbler {
    let mut scrobbler = Scrobbler::new(api_key, secret);
    if let Some(session_key) = session_key.filter(|value| !value.is_empty()) {
        scrobbler.authenticate_with_session_key(session_key);
    }
    scrobbler
}

fn build_scrobble(
    artist: String,
    track: String,
    album: Option<String>,
    _duration: Option<f64>,
    timestamp: Option<i64>,
) -> Scrobble {
    let album = album.unwrap_or_default();
    let mut scrobble = Scrobble::new(artist.trim(), track.trim(), album.trim());
    if let Some(timestamp) = timestamp.filter(|value| *value > 0) {
        scrobble.with_timestamp(timestamp as u64);
    }
    scrobble
}

fn build_lastfm_client(api_key: &str) -> Result<LastFmClient, String> {
    LastFmClient::builder()
        .api_key(api_key)
        .timeout(Duration::from_secs(20))
        .build()
        .map(LastFmClient::from_config)
        .map_err(|e| e.to_string())
}

// ── Commands ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LfmAuthToken {
    pub token: String,
    pub auth_url: String,
}

#[tauri::command]
pub async fn lfm_begin_auth(state: State<'_, AppState>) -> Result<LfmAuthToken, String> {
    let (api_key, secret) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        require_credentials(&settings::read_all(&db)?)?
    };
    let mut params = HashMap::new();
    params.insert("method".to_string(), "auth.getToken".to_string());
    let json = sign_and_get(&state.http, params, &api_key, &secret).await?;
    let token = json
        .get("token")
        .and_then(|t| t.as_str())
        .ok_or_else(|| "Failed to get auth token".to_string())?
        .to_string();
    Ok(LfmAuthToken {
        auth_url: format!("https://www.last.fm/api/auth/?api_key={api_key}&token={token}"),
        token,
    })
}

#[derive(Serialize)]
pub struct LfmSession {
    pub username: String,
}

#[tauri::command]
pub async fn lfm_complete_auth(
    state: State<'_, AppState>,
    token: String,
) -> Result<LfmSession, String> {
    let (api_key, secret) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        require_credentials(&settings::read_all(&db)?)?
    };
    let token = token.trim().to_string();
    let session = tokio::task::spawn_blocking(move || {
        let mut scrobbler = Scrobbler::new(&api_key, &secret);
        scrobbler
            .authenticate_with_token(&token)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    let username = session.name;
    let sk = session.key;
    if sk.is_empty() {
        return Err("No session key returned from Last.fm".to_string());
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    settings::upsert(&db, "LASTFM_SESSION_KEY", &sk)?;
    settings::upsert(&db, "LASTFM_USERNAME", &username)?;
    Ok(LfmSession { username })
}

#[tauri::command]
pub fn lfm_disconnect(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    settings::upsert(&db, "LASTFM_SESSION_KEY", "")?;
    settings::upsert(&db, "LASTFM_USERNAME", "")?;
    Ok(())
}

#[tauri::command]
pub async fn lfm_now_playing(
    state: State<'_, AppState>,
    artist: String,
    track: String,
    album: Option<String>,
    duration: Option<f64>,
) -> Result<(), String> {
    let (api_key, secret, sk) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let s = settings::read_all(&db)?;
        let k = s.get("LASTFM_API_KEY").cloned().unwrap_or_default();
        let sec = s.get("LASTFM_SHARED_SECRET").cloned().unwrap_or_default();
        let sk = s.get("LASTFM_SESSION_KEY").cloned().unwrap_or_default();
        (k, sec, sk)
    };
    if sk.is_empty() || api_key.is_empty() || secret.is_empty() {
        return Ok(());
    }
    let scrobble = build_scrobble(artist, track, album, duration, None);
    let _ = tokio::task::spawn_blocking(move || {
        let scrobbler = build_scrobbler(&api_key, &secret, Some(&sk));
        scrobbler.now_playing(&scrobble).map_err(|e| e.to_string())
    })
    .await;
    Ok(())
}

#[tauri::command]
pub async fn lfm_scrobble(
    state: State<'_, AppState>,
    artist: String,
    track: String,
    timestamp: i64,
    album: Option<String>,
    duration: Option<f64>,
) -> Result<(), String> {
    let (api_key, secret, sk) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let s = settings::read_all(&db)?;
        let k = s.get("LASTFM_API_KEY").cloned().unwrap_or_default();
        let sec = s.get("LASTFM_SHARED_SECRET").cloned().unwrap_or_default();
        let sk = s.get("LASTFM_SESSION_KEY").cloned().unwrap_or_default();
        (k, sec, sk)
    };
    if sk.is_empty() || api_key.is_empty() || secret.is_empty() {
        return Ok(());
    }
    let scrobble = build_scrobble(artist, track, album, duration, Some(timestamp));
    let _ = tokio::task::spawn_blocking(move || {
        let scrobbler = build_scrobbler(&api_key, &secret, Some(&sk));
        scrobbler.scrobble(&scrobble).map_err(|e| e.to_string())
    })
    .await;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTaste {
    pub connected: bool,
    pub username: String,
    pub artists: Vec<String>,
}

#[tauri::command]
pub async fn lfm_user_taste(state: State<'_, AppState>) -> Result<UserTaste, String> {
    let (api_key, secret, sk, username) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let s = settings::read_all(&db)?;
        let k = s.get("LASTFM_API_KEY").cloned().unwrap_or_default();
        let sec = s.get("LASTFM_SHARED_SECRET").cloned().unwrap_or_default();
        let sk = s.get("LASTFM_SESSION_KEY").cloned().unwrap_or_default();
        let u = s.get("LASTFM_USERNAME").cloned().unwrap_or_default();
        (k, sec, sk, u)
    };
    if sk.is_empty() || api_key.is_empty() || secret.is_empty() {
        return Ok(UserTaste { connected: false, username, artists: vec![] });
    }
    let client = build_lastfm_client(&api_key)?;
    match client
        .top_tracks(username.clone())
        .period(Period::ThreeMonth)
        .limit(100)
        .fetch()
        .await
    {
        Ok(tracks) => {
            let mut artists = Vec::new();
            for track in tracks {
                let artist = track.artist.name.trim();
                if !artist.is_empty() && !artists.iter().any(|value: &String| value == artist) {
                    artists.push(artist.to_string());
                }
                if artists.len() >= 50 {
                    break;
                }
            }
            Ok(UserTaste { connected: true, username, artists })
        }
        Err(_) => Ok(UserTaste { connected: true, username, artists: vec![] }),
    }
}

#[derive(Serialize)]
pub struct LfmStatus {
    pub connected: bool,
    pub username: String,
}

#[tauri::command]
pub fn lfm_status(state: State<'_, AppState>) -> Result<LfmStatus, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let s = settings::read_all(&db)?;
    let connected = s.get("LASTFM_SESSION_KEY").map(|v| !v.is_empty()).unwrap_or(false);
    let username = s.get("LASTFM_USERNAME").cloned().unwrap_or_default();
    Ok(LfmStatus { connected, username })
}
