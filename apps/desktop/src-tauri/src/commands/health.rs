use rand::Rng;
use serde::Serialize;
use tauri::State;
use url::Url;
use crate::AppState;
use crate::commands::profiles::get_active_profile;

#[derive(Serialize)]
pub struct ServiceHealth {
    pub subsonic: String,
    pub lastfm: String,
}

#[tauri::command]
pub async fn get_service_health(state: State<'_, AppState>) -> Result<ServiceHealth, String> {
    // ── Grab everything we need from the DB while holding the lock briefly ───
    let (profile, lfm_key, lfm_session) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let profile = get_active_profile(&db).ok();
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
        (profile, lfm_key, lfm_session)
    };

    // ── Subsonic: ping the active profile ────────────────────────────────────
    let subsonic = match profile {
        None => "missing".to_string(),
        Some(p) => {
            let salt: String = rand::thread_rng()
                .sample_iter(&rand::distributions::Alphanumeric)
                .take(12)
                .map(char::from)
                .collect();
            let token = format!("{:x}", md5::compute(format!("{}{}", p.password, salt).as_bytes()));

            let ping_base = format!("{}/rest/ping.view", p.url.trim_end_matches('/'));
            let mut url = Url::parse(&ping_base).unwrap_or_else(|_| Url::parse("http://localhost").unwrap());
            {
                let mut q = url.query_pairs_mut();
                q.append_pair("u", &p.username);
                q.append_pair("v", "1.16.1");
                q.append_pair("c", "naviarr");
                q.append_pair("f", "json");
                if p.use_password_auth {
                    q.append_pair("p", &p.password);
                } else {
                    q.append_pair("t", &token);
                    q.append_pair("s", &salt);
                }
            }

            let result = state.http
                .get(url.as_str())
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await;

            match result {
                Ok(resp) if resp.status().is_success() => {
                    // Also verify the Subsonic response status field
                    let ok = resp.json::<serde_json::Value>().await
                        .ok()
                        .and_then(|j| j.get("subsonic-response").cloned())
                        .and_then(|r| r.get("status").and_then(|s| s.as_str()).map(|s| s == "ok"))
                        .unwrap_or(false);
                    if ok { "online".to_string() } else { "offline".to_string() }
                }
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
