use serde::Serialize;
use tauri::State;
use crate::AppState;
use crate::commands::profiles::get_active_profile;

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
        if p.server_type == "local" {
            (None, None, None)
        } else if p.server_type == "jellyfin" || p.server_type == "emby" {
            match crate::commands::jellyfin::library_counts(&state.http, p).await {
                Ok((pc, tps, ss)) => (Some(pc), Some(tps), Some(ss)),
                Err(_) => (None, None, None),
            }
        } else if p.server_type == "plex" {
            match crate::commands::plex::library_counts(&state.http, p).await {
                Ok((pc, tps, ss)) => (Some(pc), Some(tps), Some(ss)),
                Err(_) => (None, None, None),
            }
        } else {
            match crate::commands::subsonic::library_counts(&state.http, p).await {
                Ok((pc, tps, ss)) => (Some(pc), Some(tps), Some(ss)),
                Err(_) => (None, None, None),
            }
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
