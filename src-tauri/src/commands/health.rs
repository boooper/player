use serde::{Deserialize, Serialize};
use tauri::State;
use crate::AppState;
use crate::commands::profiles::{get_active_profile, ActiveProfile};

#[derive(Serialize)]
pub struct ServiceHealth {
    pub subsonic: String,
    pub lastfm: String,
}

#[tauri::command]
pub async fn get_service_health(state: State<'_, AppState>) -> Result<ServiceHealth, String> {
    // ── Grab everything we need while holding locks briefly ──────────────────
    let profile = get_active_profile(&state).ok();
    let (lfm_key, lfm_session) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let lfm_key = db
            .query_row(
                "SELECT value FROM settings WHERE key = 'LASTFM_API_KEY'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .filter(|v| !v.is_empty());
        let lfm_session = db
            .query_row(
                "SELECT value FROM settings WHERE key = 'LASTFM_SESSION_KEY'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .filter(|v| !v.is_empty());
        (lfm_key, lfm_session)
    };

    // ── Subsonic: ping the active profile ────────────────────────────────────
    let subsonic = match profile {
        None => "missing".to_string(),
        Some(p) if p.server_type == "local" => {
            match crate::commands::local::ping(&p).await {
                Ok(true) => "online".to_string(),
                _ => "offline".to_string(),
            }
        }
        Some(p) if p.server_type == "jellyfin" || p.server_type == "emby" => {
            match crate::commands::jellyfin::ping(&state.http, &p).await {
                Ok(true) => "online".to_string(),
                _ => "offline".to_string(),
            }
        }
        Some(p) if p.server_type == "plex" => {
            match crate::commands::plex::ping(&state.http, &p).await {
                Ok(true) => "online".to_string(),
                _ => "offline".to_string(),
            }
        }
        Some(p) => {
            match crate::commands::subsonic::ping(&state.http, &p).await {
                Ok(true) => "online".to_string(),
                _ => "offline".to_string(),
            }
        }
    };

    // ── Last.fm: session key = online, api key only = offline, nothing = missing
    let lastfm = if lfm_session.is_some() {
        "online".to_string()
    } else if lfm_key.is_some() {
        "offline".to_string()
    } else {
        "missing".to_string()
    };

    Ok(ServiceHealth { subsonic, lastfm })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestProfilePayload {
    pub url: String,
    pub username: String,
    pub password: String,
    pub server_type: String,
}

#[tauri::command]
pub async fn test_profile_connection(
    state: State<'_, AppState>,
    data: TestProfilePayload,
) -> Result<(), String> {
    use std::time::Duration;

    let url = if data.server_type == "local" {
        data.url.trim().to_string()
    } else {
        data.url.trim().trim_end_matches('/').to_string()
    };

    let profile = ActiveProfile {
        url,
        username: data.username.trim().to_string(),
        password: data.password.trim().to_string(),
        server_type: data.server_type.clone(),
    };

    // Use a dedicated client with a 10-second timeout so the test never hangs.
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| state.http.clone());

    let ok = match data.server_type.as_str() {
        "local" => crate::commands::local::ping(&profile).await?,
        "jellyfin" | "emby" => crate::commands::jellyfin::ping(&http, &profile).await?,
        "plex" => crate::commands::plex::ping(&http, &profile).await?,
        _ => crate::commands::subsonic::ping(&http, &profile).await?,
    };

    if ok {
        Ok(())
    } else {
        Err("Server responded but reported an error — check your credentials.".to_string())
    }
}
