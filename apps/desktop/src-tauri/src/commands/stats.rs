use serde::Serialize;
use tauri::State;
use crate::AppState;

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
pub fn get_library_stats(state: State<'_, AppState>) -> Result<LibraryStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let liked_artists: i64 = db
        .query_row("SELECT COUNT(*) FROM liked_artists", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let last_fm_configured: bool = db
        .query_row(
            "SELECT COUNT(*) FROM settings WHERE key = 'LASTFM_API_KEY' AND value != ''",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|e| e.to_string())?
        > 0;

    Ok(LibraryStats {
        liked_artists,
        playlist_count: None,
        total_playlist_songs: None,
        starred_songs: None,
        last_fm_configured,
    })
}
