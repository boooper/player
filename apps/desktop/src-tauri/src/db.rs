use rusqlite::{Connection, Result};
use std::path::Path;

pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS settings (
            key   TEXT NOT NULL PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS profiles (
            id                INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            name              TEXT NOT NULL,
            url               TEXT NOT NULL,
            username          TEXT NOT NULL,
            password          TEXT NOT NULL,
            use_password_auth INTEGER NOT NULL DEFAULT 0,
            is_active         INTEGER NOT NULL DEFAULT 0,
            server_type       TEXT NOT NULL DEFAULT 'subsonic',
            created_at        TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at        TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS liked_artists (
            id          INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL UNIQUE,
            source      TEXT,
            external_id TEXT,
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );"
    )?;
    // Migrate: add server_type column to existing DBs (ignored if already present)
    conn.execute(
        "ALTER TABLE profiles ADD COLUMN server_type TEXT NOT NULL DEFAULT 'subsonic'",
        [],
    ).ok();
    // Migrate: populate server_type from legacy use_password_auth flag
    conn.execute(
        "UPDATE profiles SET server_type = 'subsonic_legacy' WHERE use_password_auth = 1 AND server_type = 'subsonic'",
        [],
    )?;
    Ok(conn)
}
