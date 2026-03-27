use serde::Serialize;
use tauri::State;
use crate::AppState;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LikedArtist {
    pub id: i64,
    pub name: String,
    pub source: Option<String>,
    pub external_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn row_to_liked_artist(row: &rusqlite::Row) -> rusqlite::Result<LikedArtist> {
    Ok(LikedArtist {
        id: row.get(0)?,
        name: row.get(1)?,
        source: row.get(2)?,
        external_id: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

#[tauri::command]
pub fn get_liked_artists(state: State<'_, AppState>) -> Result<Vec<LikedArtist>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, name, source, external_id, created_at, updated_at
             FROM liked_artists ORDER BY name ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_liked_artist).map_err(|e| e.to_string())?;
    rows.map(|r| r.map_err(|e| e.to_string())).collect()
}

#[tauri::command]
pub fn save_liked_artist(
    state: State<'_, AppState>,
    name: String,
    source: Option<String>,
    external_id: Option<String>,
) -> Result<LikedArtist, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Artist name is required.".to_string());
    }

    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO liked_artists (name, source, external_id, updated_at)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(name) DO UPDATE SET
             source = excluded.source,
             external_id = excluded.external_id,
             updated_at = datetime('now')",
        rusqlite::params![name, source, external_id],
    )
    .map_err(|e| e.to_string())?;

    db.query_row(
        "SELECT id, name, source, external_id, created_at, updated_at
         FROM liked_artists WHERE name = ?1",
        rusqlite::params![name],
        row_to_liked_artist,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_liked_artist(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Artist name is required.".to_string());
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM liked_artists WHERE name = ?1", rusqlite::params![name])
        .map_err(|e| e.to_string())?;
    Ok(())
}
