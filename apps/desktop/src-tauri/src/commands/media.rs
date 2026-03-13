//! Canonical media types shared across all music providers (Subsonic, Jellyfin, etc.).
//!
//! Add support for a new provider by mapping its HTTP responses into these types,
//! then returning them from your command implementations.  Nothing outside of a
//! provider module should reference provider-specific raw shapes.

use serde::Serialize;

// ── Core media types ──────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_id: String,
    /// Provider-internal cover-art identifier (opaque string).
    pub cover_art: String,
    /// Fully-resolved URL ready for `<img>` / `<audio>`.
    pub cover_art_url: String,
    pub stream_url: String,
    pub duration: f64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub artist_id: String,
    pub cover_art: String,
    pub cover_art_url: String,
    pub song_count: f64,
    pub duration: f64,
    pub year: Option<f64>,
}

/// An `Album` extended with an optional genre field, used in `AlbumDetail`.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AlbumFull {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub artist_id: String,
    pub cover_art: String,
    pub cover_art_url: String,
    pub song_count: f64,
    pub duration: f64,
    pub year: Option<f64>,
    pub genre: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub song_count: f64,
    pub duration: f64,
    pub cover_art: String,
    pub cover_art_url: String,
}

// ── Composite types ───────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumDetail {
    pub album: AlbumFull,
    pub songs: Vec<Song>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistDetail {
    pub playlist: PlaylistMeta,
    pub songs: Vec<Song>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistMeta {
    pub id: String,
    pub name: String,
    pub song_count: f64,
    pub duration: f64,
    pub cover_art_url: String,
}
