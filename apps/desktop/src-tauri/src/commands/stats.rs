use serde::Serialize;
use tauri::State;
use crate::AppState;
use crate::commands::profiles::get_active_profile;
use crate::commands::library::request;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStats {
    pub liked_artists: i64,
    pub playlist_count: Option<i64>,
    pub total_playlist_songs: Option<i64>,
    pub starred_songs: Option<i64>,
    pub last_fm_configured: bool,
}

#[tauri::command]
pub async fn get_library_stats(state: State<'_, AppState>) -> Result<LibraryStats, String> {
    // Grab DB values while holding the lock briefly
    let (liked_artists, last_fm_configured, profile) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        let liked: i64 = db
            .query_row("SELECT COUNT(*) FROM liked_artists", [], |row| row.get(0))
            .unwrap_or(0);

        let lfm: bool = db
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE key = 'LASTFM_API_KEY' AND value != ''",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;

        let profile = get_active_profile(&db).ok();
        (liked, lfm, profile)
    };

    // Fetch live counts if a profile is configured
    let (playlist_count, total_playlist_songs, starred_songs) = if let Some(ref p) = profile {
        if p.server_type == "jellyfin" || p.server_type == "emby" {
            match crate::commands::jellyfin::library_counts(&state.http, p).await {
                Ok((pc, tps, ss)) => (Some(pc), Some(tps), Some(ss)),
                Err(_) => (None, None, None),
            }
        } else {
        let playlists_result = request(&state.http, p, "getPlaylists", &[]).await;
        let (pc, tps) = playlists_result
            .ok()
            .and_then(|body| body.get("playlists").cloned())
            .and_then(|pl| {
                let arr = match pl.get("playlist") {
                    Some(serde_json::Value::Array(a)) => a.clone(),
                    Some(item) => vec![item.clone()],
                    None => vec![],
                };
                let count = arr.len() as i64;
                let total: i64 = arr.iter()
                    .filter_map(|p| p.get("songCount").and_then(|v| v.as_i64()))
                    .sum();
                Some((count, total))
            })
            .map(|(c, t)| (Some(c), Some(t)))
            .unwrap_or((None, None));

        let starred_result = request(&state.http, p, "getStarred2", &[]).await;
        let sc = starred_result
            .ok()
            .and_then(|body| body.get("starred2").cloned())
            .map(|s2| {
                let songs = match s2.get("song") {
                    Some(serde_json::Value::Array(a)) => a.len() as i64,
                    Some(_) => 1,
                    None => 0,
                };
                songs
            });

        (pc, tps, sc)
        }
    } else {
        (None, None, None)
    };

    Ok(LibraryStats {
        liked_artists,
        playlist_count,
        total_playlist_songs,
        starred_songs,
        last_fm_configured,
    })
}
