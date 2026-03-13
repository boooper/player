use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::AppState;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub username: String,
    pub use_password_auth: bool,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDraft {
    pub name: String,
    pub url: String,
    pub username: String,
    pub password: Option<String>,
    pub use_password_auth: bool,
}

fn row_to_profile(row: &rusqlite::Row) -> rusqlite::Result<Profile> {
    Ok(Profile {
        id: row.get(0)?,
        name: row.get(1)?,
        url: row.get(2)?,
        username: row.get(3)?,
        use_password_auth: row.get::<_, i64>(4)? != 0,
        is_active: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

const SELECT: &str =
    "SELECT id, name, url, username, use_password_auth, is_active, created_at, updated_at FROM profiles";

#[tauri::command]
pub fn get_profiles(state: State<'_, AppState>) -> Result<Vec<Profile>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(&format!("{SELECT} ORDER BY id ASC"))
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_profile).map_err(|e| e.to_string())?;
    rows.map(|r| r.map_err(|e| e.to_string())).collect()
}

#[tauri::command]
pub fn create_profile(state: State<'_, AppState>, data: ProfileDraft) -> Result<Profile, String> {
    let name = data.name.trim().to_string();
    let url = data.url.trim().trim_end_matches('/').to_string();
    let username = data.username.trim().to_string();
    let password = data.password.unwrap_or_default().trim().to_string();

    if name.is_empty() { return Err("name is required".to_string()); }
    if url.is_empty() { return Err("url is required".to_string()); }
    if username.is_empty() { return Err("username is required".to_string()); }
    if password.is_empty() { return Err("password is required".to_string()); }

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let count: i64 = db
        .query_row("SELECT COUNT(*) FROM profiles", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    let is_active = count == 0;

    db.execute(
        "INSERT INTO profiles (name, url, username, password, use_password_auth, is_active, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
        rusqlite::params![name, url, username, password, data.use_password_auth as i64, is_active as i64],
    )
    .map_err(|e| e.to_string())?;

    let id = db.last_insert_rowid();
    db.query_row(
        &format!("{SELECT} WHERE id = ?1"),
        rusqlite::params![id],
        row_to_profile,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_profile(
    state: State<'_, AppState>,
    id: i64,
    data: ProfileDraft,
) -> Result<Profile, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let exists: bool = db
        .query_row("SELECT COUNT(*) FROM profiles WHERE id = ?1", rusqlite::params![id], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|e| e.to_string())?
        > 0;
    if !exists {
        return Err("Profile not found".to_string());
    }

    let name = data.name.trim().to_string();
    let url = data.url.trim().trim_end_matches('/').to_string();
    let username = data.username.trim().to_string();

    if !name.is_empty() {
        db.execute(
            "UPDATE profiles SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![name, id],
        )
        .map_err(|e| e.to_string())?;
    }
    if !url.is_empty() {
        db.execute(
            "UPDATE profiles SET url = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![url, id],
        )
        .map_err(|e| e.to_string())?;
    }
    if !username.is_empty() {
        db.execute(
            "UPDATE profiles SET username = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![username, id],
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(pw) = data.password {
        let pw = pw.trim().to_string();
        if !pw.is_empty() {
            db.execute(
                "UPDATE profiles SET password = ?1, updated_at = datetime('now') WHERE id = ?2",
                rusqlite::params![pw, id],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    db.execute(
        "UPDATE profiles SET use_password_auth = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![data.use_password_auth as i64, id],
    )
    .map_err(|e| e.to_string())?;

    db.query_row(&format!("{SELECT} WHERE id = ?1"), rusqlite::params![id], row_to_profile)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_profile(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let row = db
        .query_row(
            "SELECT is_active, (SELECT COUNT(*) FROM profiles) FROM profiles WHERE id = ?1",
            rusqlite::params![id],
            |row| Ok((row.get::<_, i64>(0)? != 0, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Profile not found".to_string())?;

    let (is_active, count) = row;
    if is_active && count == 1 {
        return Err("Cannot delete the only server profile".to_string());
    }

    db.execute("DELETE FROM profiles WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;

    if is_active {
        db.execute(
            "UPDATE profiles SET is_active = 1, updated_at = datetime('now')
             WHERE id = (SELECT id FROM profiles ORDER BY id ASC LIMIT 1)",
            [],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn activate_profile(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let exists: bool = db
        .query_row("SELECT COUNT(*) FROM profiles WHERE id = ?1", rusqlite::params![id], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|e| e.to_string())?
        > 0;
    if !exists {
        return Err("Profile not found".to_string());
    }

    db.execute("UPDATE profiles SET is_active = 0, updated_at = datetime('now')", [])
        .map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE profiles SET is_active = 1, updated_at = datetime('now') WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Internal helper used by subsonic commands.
pub struct ActiveProfile {
    pub url: String,
    pub username: String,
    pub password: String,
    pub use_password_auth: bool,
}

pub fn get_active_profile(db: &rusqlite::Connection) -> Result<ActiveProfile, String> {
    db.query_row(
        "SELECT url, username, password, use_password_auth FROM profiles WHERE is_active = 1 LIMIT 1",
        [],
        |row| {
            Ok(ActiveProfile {
                url: row.get::<_, String>(0)?.trim_end_matches('/').to_string(),
                username: row.get(1)?,
                password: row.get(2)?,
                use_password_auth: row.get::<_, i64>(3)? != 0,
            })
        },
    )
    .map_err(|_| "No active server profile configured. Add one in Settings.".to_string())
}
