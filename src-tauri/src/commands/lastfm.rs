use std::collections::HashMap;
use serde::Serialize;
use tauri::State;
use crate::{AppState, commands::settings};

fn get_lfm_tokens(state: &AppState) -> (String, String, String) {
    let Ok(db) = state.db.lock() else { return (String::new(), String::new(), String::new()) };
    let Ok(s) = settings::read_all(&db) else { return (String::new(), String::new(), String::new()) };
    (
        s.get("LASTFM_API_KEY").cloned().unwrap_or_default(),
        s.get("LASTFM_SHARED_SECRET").cloned().unwrap_or_default(),
        s.get("LASTFM_SESSION_KEY").cloned().unwrap_or_default(),
    )
}

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

fn read_session_credentials(s: &HashMap<String, String>) -> (String, String, String) {
    let key = s.get("LASTFM_API_KEY").cloned().unwrap_or_default();
    let secret = s.get("LASTFM_SHARED_SECRET").cloned().unwrap_or_default();
    let sk = s.get("LASTFM_SESSION_KEY").cloned().unwrap_or_default();
    (key, secret, sk)
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
        let (k, s, _) = get_lfm_tokens(&state);
        if !k.is_empty() {
            (k, s)
        } else {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            require_credentials(&settings::read_all(&db)?)?
        }
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
        let (k, s, _) = get_lfm_tokens(&state);
        if !k.is_empty() {
            (k, s)
        } else {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            require_credentials(&settings::read_all(&db)?)?
        }
    };
    let mut params = HashMap::new();
    params.insert("method".to_string(), "auth.getSession".to_string());
    params.insert("token".to_string(), token.trim().to_string());
    let json = sign_and_get(&state.http, params, &api_key, &secret).await?;
    let session = json.get("session").ok_or_else(|| "No session in Last.fm response".to_string())?;
    let username = session.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
    let sk = session.get("key").and_then(|k| k.as_str()).unwrap_or("").to_string();
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
        let tokens = get_lfm_tokens(&state);
        if !tokens.0.is_empty() {
            tokens
        } else {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            read_session_credentials(&settings::read_all(&db)?)
        }
    };
    if sk.is_empty() || api_key.is_empty() || secret.is_empty() {
        return Ok(());
    }
    let mut params = HashMap::new();
    params.insert("method".to_string(), "track.updateNowPlaying".to_string());
    params.insert("artist".to_string(), artist.trim().to_string());
    params.insert("track".to_string(), track.trim().to_string());
    if let Some(album) = album.filter(|a| !a.trim().is_empty()) {
        params.insert("album".to_string(), album.trim().to_string());
    }
    if let Some(dur) = duration.filter(|d| *d > 0.0) {
        params.insert("duration".to_string(), (dur as u64).to_string());
    }
    params.insert("sk".to_string(), sk);
    let _ = sign_and_post(&state.http, params, &api_key, &secret).await;
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
        let tokens = get_lfm_tokens(&state);
        if !tokens.0.is_empty() {
            tokens
        } else {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            read_session_credentials(&settings::read_all(&db)?)
        }
    };
    if sk.is_empty() || api_key.is_empty() || secret.is_empty() {
        return Ok(());
    }
    let mut params = HashMap::new();
    params.insert("method".to_string(), "track.scrobble".to_string());
    params.insert("artist".to_string(), artist.trim().to_string());
    params.insert("track".to_string(), track.trim().to_string());
    params.insert("timestamp".to_string(), timestamp.to_string());
    if let Some(album) = album.filter(|a| !a.trim().is_empty()) {
        params.insert("album".to_string(), album.trim().to_string());
    }
    if let Some(dur) = duration.filter(|d| *d > 0.0) {
        params.insert("duration".to_string(), (dur as u64).to_string());
    }
    params.insert("sk".to_string(), sk);
    let _ = sign_and_post(&state.http, params, &api_key, &secret).await;
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
    let (api_key, _secret, sk, username) = {
        let (k, sec, sk) = get_lfm_tokens(&state);
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let s = settings::read_all(&db)?;
        let u = s.get("LASTFM_USERNAME").cloned().unwrap_or_default();
        if !k.is_empty() {
            (k, sec, sk, u)
        } else {
            let (k2, sec2, sk2) = read_session_credentials(&s);
            (k2, sec2, sk2, u)
        }
    };
    if sk.is_empty() || api_key.is_empty() {
        return Ok(UserTaste { connected: false, username, artists: vec![] });
    }
    let result = state.http
        .get(LASTFM_API)
        .query(&[
            ("method", "user.getTopTracks"),
            ("user", username.as_str()),
            ("period", "3month"),
            ("limit", "100"),
            ("api_key", api_key.as_str()),
            ("format", "json"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string());

    match result {
        Ok(json) => {
            let tracks = json
                .get("toptracks")
                .and_then(|tt| tt.get("track"))
                .and_then(|t| t.as_array())
                .cloned()
                .unwrap_or_default();
            let mut artists: Vec<String> = Vec::new();
            for t in tracks {
                let artist = t.get("artist")
                    .and_then(|a| a.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !artist.is_empty() && !artists.contains(&artist) {
                    artists.push(artist);
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
