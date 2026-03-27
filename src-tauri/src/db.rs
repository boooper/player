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
        );
        CREATE TABLE IF NOT EXISTS play_history (
            id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            song_id             TEXT NOT NULL,
            artist              TEXT NOT NULL,
            title               TEXT NOT NULL,
            album               TEXT,
            duration_secs       REAL,
            completed_fraction  REAL NOT NULL DEFAULT 0.0,
            played_at           TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS play_history_artist ON play_history (LOWER(artist));
        CREATE INDEX IF NOT EXISTS play_history_played_at ON play_history (played_at);
        CREATE TABLE IF NOT EXISTS song_genres (
            song_id    TEXT NOT NULL PRIMARY KEY,
            artist     TEXT NOT NULL,
            genre      TEXT NOT NULL,
            cached_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS song_genres_genre ON song_genres (LOWER(genre));"
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
    conn.execute(
        "UPDATE profiles SET username = '' WHERE server_type = 'local' AND username IS NULL",
        [],
    ).ok();
    // Migrate: add cover_art_url column to play_history (ignored if already present)
    conn.execute(
        "ALTER TABLE play_history ADD COLUMN cover_art_url TEXT",
        [],
    ).ok();
    Ok(conn)
}
