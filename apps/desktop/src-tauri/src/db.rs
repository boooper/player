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
        );",
    )?;
    Ok(conn)
}
