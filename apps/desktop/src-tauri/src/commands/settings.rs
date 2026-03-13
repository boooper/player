use std::collections::HashMap;
use tauri::State;
use crate::AppState;

const ALLOWED_KEYS: &[&str] = &[
    "LASTFM_API_KEY",
    "LASTFM_SHARED_SECRET",
    "RECOMMENDATION_PROVIDER",
    "METADATA_PROVIDER",
    "SUBSONIC_BASE_URL",
    "SUBSONIC_USERNAME",
    "SUBSONIC_USE_PASSWORD_AUTH",
    "SUBSONIC_PASSWORD",
    "LASTFM_SESSION_KEY",
    "LASTFM_USERNAME",
];

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<HashMap<String, String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for row in rows {
        let (k, v) = row.map_err(|e| e.to_string())?;
        map.insert(k, v);
    }
    Ok(map)
}

#[tauri::command]
pub fn update_settings(
    state: State<'_, AppState>,
    updates: HashMap<String, String>,
) -> Result<Vec<String>, String> {
    let valid: Vec<(String, String)> = updates
        .into_iter()
        .filter(|(k, _)| ALLOWED_KEYS.contains(&k.as_str()))
        .collect();

    if valid.is_empty() {
        return Err("No valid setting keys provided".to_string());
    }

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let updated: Vec<String> = valid.iter().map(|(k, _)| k.clone()).collect();
    for (key, value) in &valid {
        db.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(updated)
}

/// Internal helper shared by other command modules.
pub fn read_all(db: &rusqlite::Connection) -> Result<HashMap<String, String>, String> {
    let mut stmt = db
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for row in rows {
        let (k, v) = row.map_err(|e| e.to_string())?;
        map.insert(k, v);
    }
    Ok(map)
}

/// Upsert a single key/value pair.
pub fn upsert(db: &rusqlite::Connection, key: &str, value: &str) -> Result<(), String> {
    db.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
