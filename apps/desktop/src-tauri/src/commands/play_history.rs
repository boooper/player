use std::collections::HashMap;
use serde::Serialize;
use tauri::State;
use crate::AppState;

// ── Scoring thresholds ────────────────────────────────────────────────────────
/// A play where less than this fraction was heard counts as a skip.
const SKIP_THRESHOLD: f64 = 0.20;
/// A play where at least this fraction was heard counts as a full play.
const FULL_THRESHOLD: f64 = 0.80;

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListeningProfile {
    pub total_plays: i64,
    pub total_listen_secs: f64,
    pub unique_artists: i64,
    pub top_songs: Vec<SongStat>,
    pub top_artists: Vec<ArtistStat>,
    pub top_genres: Vec<GenreStat>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongStat {
    pub song_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub cover_art_url: Option<String>,
    pub play_count: i64,
    pub full_play_count: i64,
    pub skip_count: i64,
    pub total_listen_secs: f64,
    pub avg_completion: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistStat {
    pub artist: String,
    pub cover_art_url: Option<String>,
    pub play_count: i64,
    pub full_play_count: i64,
    pub skip_count: i64,
    pub total_listen_secs: f64,
    pub avg_completion: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenreStat {
    pub genre: String,
    pub play_count: i64,
    pub full_play_count: i64,
    pub skip_count: i64,
    pub avg_completion: f64,
}

/// Per-song reputation returned by `get_song_affinities`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongAffinity {
    pub song_id: String,
    pub play_count: i64,
    pub full_play_count: i64,
    pub skip_count: i64,
    /// Total seconds actually listened across all plays.
    pub total_listen_secs: f64,
    /// skip_count / play_count  (0–1).
    pub skip_rate: f64,
    /// Normalised quality score 0–1 (higher = liked, 0 = disliked/unknown).
    pub score: f64,
}

// ── Helper: compute artist/genre score from raw counters ─────────────────────
//
//  quality  = full_rate − skip_rate × 0.5          (0 → 1)
//  volume   = ln(listen_minutes + 1)               rewards total listening time
//  recency  = 0.5 + 0.5 × (recent_plays / total)   rewards recent activity
//  raw      = quality × (1 + volume × 0.3) × recency
//
fn artist_raw_score(
    total_plays: i64,
    full_plays: i64,
    skip_count: i64,
    total_listen_secs: f64,
    recency_ratio: f64,
) -> f64 {
    if total_plays == 0 { return 0.0; }
    let full_rate  = full_plays  as f64 / total_plays as f64;
    let skip_rate  = skip_count  as f64 / total_plays as f64;
    let quality    = (full_rate - skip_rate * 0.5).max(0.0);
    let volume     = (total_listen_secs / 60.0 + 1.0).ln();
    quality * (1.0 + volume * 0.3) * (0.5 + 0.5 * recency_ratio)
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// Record a single play event.  `completed_fraction` is 0.0–1.0.
/// Fire-and-forget — errors are silently ignored on the frontend.
#[tauri::command]
pub fn record_play(
    state: State<'_, AppState>,
    song_id: String,
    artist: String,
    title: String,
    album: Option<String>,
    cover_art_url: Option<String>,
    duration_secs: Option<f64>,
    completed_fraction: f64,
) -> Result<(), String> {
    let artist = artist.trim().to_string();
    let title  = title.trim().to_string();
    if artist.is_empty() || title.is_empty() { return Ok(()); }
    let fraction = completed_fraction.clamp(0.0, 1.0);
    let cover = cover_art_url.filter(|s| !s.is_empty());
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO play_history
             (song_id, artist, title, album, cover_art_url, duration_secs, completed_fraction)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![song_id, artist, title, album, cover, duration_secs, fraction],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Cache a song's genre for genre-affinity scoring.  Fire-and-forget.
#[tauri::command]
pub fn cache_song_genre(
    state: State<'_, AppState>,
    song_id: String,
    artist: String,
    genre: String,
) -> Result<(), String> {
    let genre = genre.trim().to_string();
    if genre.is_empty() { return Ok(()); }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO song_genres (song_id, artist, genre, cached_at)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(song_id) DO UPDATE
             SET genre = excluded.genre, cached_at = datetime('now')",
        rusqlite::params![song_id, artist.trim(), genre],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// 0–1 affinity score per artist.
///
/// Factors in:
///   - Full plays (completed ≥ 80 %)      — strong positive signal
///   - Partial plays (20–80 %)            — mild positive signal
///   - Skips (< 20 %)                     — negative signal (×0.5 penalty)
///   - Total listen time                  — rewards artists you've spent hours on
///   - Recency (last 30 days)             — recent plays weighted more heavily
#[tauri::command]
pub fn get_artist_affinities(
    state: State<'_, AppState>,
    artists: Vec<String>,
) -> Result<HashMap<String, f64>, String> {
    if artists.is_empty() { return Ok(HashMap::new()); }
    let requested: std::collections::HashSet<String> =
        artists.iter().map(|a| a.trim().to_lowercase()).collect();

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db.prepare(
        "SELECT
           LOWER(artist)                                                   AS artist_key,
           COUNT(*)                                                        AS total_plays,
           SUM(CASE WHEN completed_fraction >= ?1 THEN 1 ELSE 0 END)      AS full_plays,
           SUM(CASE WHEN completed_fraction <  ?2 THEN 1 ELSE 0 END)      AS skip_count,
           COALESCE(
             SUM(completed_fraction * duration_secs),
             SUM(completed_fraction * 180.0)
           )                                                               AS total_listen_secs,
           CAST(
             SUM(CASE WHEN played_at > datetime('now','-30 days') THEN 1 ELSE 0 END)
             AS REAL
           ) / COUNT(*)                                                    AS recency_ratio
         FROM play_history
         GROUP BY LOWER(artist)",
    ).map_err(|e| e.to_string())?;

    let mut raw: Vec<(String, f64)> = stmt
        .query_map(rusqlite::params![FULL_THRESHOLD, SKIP_THRESHOLD], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, f64>(4)?,
                row.get::<_, f64>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .filter(|(artist, ..)| requested.contains(artist.as_str()))
        .map(|(artist, total, full, skips, listen_secs, recency)| {
            (artist, artist_raw_score(total, full, skips, listen_secs, recency))
        })
        .collect();

    let max = raw.iter().map(|(_, s)| *s).fold(0.0_f64, f64::max);
    let mut result = HashMap::new();
    for (artist, score) in raw.drain(..) {
        result.insert(artist, if max > 0.0 { score / max } else { 0.0 });
    }
    Ok(result)
}

/// 0–1 affinity score per genre (requires prior `cache_song_genre` calls).
#[tauri::command]
pub fn get_genre_affinities(
    state: State<'_, AppState>,
    genres: Vec<String>,
) -> Result<HashMap<String, f64>, String> {
    if genres.is_empty() { return Ok(HashMap::new()); }
    let requested: std::collections::HashSet<String> =
        genres.iter().map(|g| g.trim().to_lowercase()).collect();

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db.prepare(
        "SELECT
           LOWER(sg.genre)                                                 AS genre_key,
           COUNT(*)                                                        AS total_plays,
           SUM(CASE WHEN ph.completed_fraction >= ?1 THEN 1 ELSE 0 END)   AS full_plays,
           SUM(CASE WHEN ph.completed_fraction <  ?2 THEN 1 ELSE 0 END)   AS skip_count,
           COALESCE(
             SUM(ph.completed_fraction * ph.duration_secs),
             SUM(ph.completed_fraction * 180.0)
           )                                                               AS total_listen_secs,
           CAST(
             SUM(CASE WHEN ph.played_at > datetime('now','-30 days') THEN 1 ELSE 0 END)
             AS REAL
           ) / COUNT(*)                                                    AS recency_ratio
         FROM play_history ph
         JOIN song_genres sg ON ph.song_id = sg.song_id
         GROUP BY LOWER(sg.genre)",
    ).map_err(|e| e.to_string())?;

    let mut raw: Vec<(String, f64)> = stmt
        .query_map(rusqlite::params![FULL_THRESHOLD, SKIP_THRESHOLD], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, f64>(4)?,
                row.get::<_, f64>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .filter(|(genre, ..)| requested.contains(genre.as_str()))
        .map(|(genre, total, full, skips, listen_secs, recency)| {
            (genre, artist_raw_score(total, full, skips, listen_secs, recency))
        })
        .collect();

    let max = raw.iter().map(|(_, s)| *s).fold(0.0_f64, f64::max);
    let mut result = HashMap::new();
    for (genre, score) in raw.drain(..) {
        result.insert(genre, if max > 0.0 { score / max } else { 0.0 });
    }
    Ok(result)
}

/// Per-song reputation for a list of song IDs.
///
/// Used to:
///   - Filter out songs the user repeatedly skips (skip_rate > 0.6, play_count ≥ 3)
///   - Boost songs the user has enjoyed before in the ranking
#[tauri::command]
pub fn get_song_affinities(
    state: State<'_, AppState>,
    song_ids: Vec<String>,
) -> Result<Vec<SongAffinity>, String> {
    if song_ids.is_empty() { return Ok(vec![]); }
    let requested: std::collections::HashSet<String> =
        song_ids.iter().map(|id| id.trim().to_string()).collect();

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db.prepare(
        "SELECT
           song_id,
           COUNT(*)                                                        AS total_plays,
           SUM(CASE WHEN completed_fraction >= ?1 THEN 1 ELSE 0 END)      AS full_plays,
           SUM(CASE WHEN completed_fraction <  ?2 THEN 1 ELSE 0 END)      AS skip_count,
           COALESCE(
             SUM(completed_fraction * duration_secs),
             SUM(completed_fraction * 180.0)
           )                                                               AS total_listen_secs
         FROM play_history
         GROUP BY song_id",
    ).map_err(|e| e.to_string())?;

    let rows: Vec<(String, i64, i64, i64, f64)> = stmt
        .query_map(rusqlite::params![FULL_THRESHOLD, SKIP_THRESHOLD], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, f64>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .filter(|(id, ..)| requested.contains(id.as_str()))
        .collect();

    // Compute a normalised score so callers can rank known songs.
    let max_raw = rows.iter().map(|(_, total, full, skips, secs)| {
        artist_raw_score(*total, *full, *skips, *secs, 1.0)
    }).fold(0.0_f64, f64::max);

    let result = rows.into_iter().map(|(song_id, total, full, skips, secs)| {
        let skip_rate = if total > 0 { skips as f64 / total as f64 } else { 0.0 };
        let raw   = artist_raw_score(total, full, skips, secs, 1.0);
        let score = if max_raw > 0.0 { raw / max_raw } else { 0.0 };
        SongAffinity {
            song_id,
            play_count: total,
            full_play_count: full,
            skip_count: skips,
            total_listen_secs: secs,
            skip_rate,
            score,
        }
    }).collect();

    Ok(result)
}

/// User's overall listening profile — top songs, artists, genres, and totals.
#[tauri::command]
pub fn get_listening_profile(
    state: State<'_, AppState>,
) -> Result<ListeningProfile, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let (total_plays, total_listen_secs, unique_artists): (i64, f64, i64) = db
        .query_row(
            "SELECT
               COUNT(*),
               COALESCE(SUM(completed_fraction * COALESCE(duration_secs, 180.0)), 0.0),
               COUNT(DISTINCT LOWER(artist))
             FROM play_history",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap_or((0, 0.0, 0));

    let mut stmt = db.prepare(
        "SELECT
           song_id,
           title,
           artist,
           album,
           MAX(cover_art_url),
           COUNT(*)                                                       AS plays,
           SUM(CASE WHEN completed_fraction >= ?1 THEN 1 ELSE 0 END)     AS full_plays,
           SUM(CASE WHEN completed_fraction <  ?2 THEN 1 ELSE 0 END)     AS skip_count,
           COALESCE(SUM(completed_fraction * duration_secs), SUM(completed_fraction * 180.0)) AS listen_secs,
           AVG(completed_fraction)
         FROM play_history
         GROUP BY song_id
         ORDER BY plays DESC
         LIMIT 25",
    ).map_err(|e| e.to_string())?;
    let top_songs: Vec<SongStat> = stmt
        .query_map(rusqlite::params![FULL_THRESHOLD, SKIP_THRESHOLD], |row| {
            Ok(SongStat {
                song_id:           row.get(0)?,
                title:             row.get(1)?,
                artist:            row.get(2)?,
                album:             row.get(3)?,
                cover_art_url:     row.get(4)?,
                play_count:        row.get(5)?,
                full_play_count:   row.get(6)?,
                skip_count:        row.get(7)?,
                total_listen_secs: row.get(8)?,
                avg_completion:    row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut stmt = db.prepare(
        "SELECT
           artist,
           MAX(cover_art_url),
           COUNT(*)                                                       AS plays,
           SUM(CASE WHEN completed_fraction >= ?1 THEN 1 ELSE 0 END)     AS full_plays,
           SUM(CASE WHEN completed_fraction <  ?2 THEN 1 ELSE 0 END)     AS skip_count,
           COALESCE(SUM(completed_fraction * duration_secs), SUM(completed_fraction * 180.0)) AS listen_secs,
           AVG(completed_fraction)
         FROM play_history
         GROUP BY LOWER(artist)
         ORDER BY plays DESC
         LIMIT 25",
    ).map_err(|e| e.to_string())?;
    let top_artists: Vec<ArtistStat> = stmt
        .query_map(rusqlite::params![FULL_THRESHOLD, SKIP_THRESHOLD], |row| {
            Ok(ArtistStat {
                artist:            row.get(0)?,
                cover_art_url:     row.get(1)?,
                play_count:        row.get(2)?,
                full_play_count:   row.get(3)?,
                skip_count:        row.get(4)?,
                total_listen_secs: row.get(5)?,
                avg_completion:    row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut stmt = db.prepare(
        "SELECT
           sg.genre,
           COUNT(*)                                                       AS plays,
           SUM(CASE WHEN ph.completed_fraction >= ?1 THEN 1 ELSE 0 END)  AS full_plays,
           SUM(CASE WHEN ph.completed_fraction <  ?2 THEN 1 ELSE 0 END)  AS skip_count,
           AVG(ph.completed_fraction)
         FROM play_history ph
         JOIN song_genres sg ON ph.song_id = sg.song_id
         GROUP BY LOWER(sg.genre)
         ORDER BY plays DESC
         LIMIT 25",
    ).map_err(|e| e.to_string())?;
    let top_genres: Vec<GenreStat> = stmt
        .query_map(rusqlite::params![FULL_THRESHOLD, SKIP_THRESHOLD], |row| {
            Ok(GenreStat {
                genre:           row.get(0)?,
                play_count:      row.get(1)?,
                full_play_count: row.get(2)?,
                skip_count:      row.get(3)?,
                avg_completion:  row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(ListeningProfile { total_plays, total_listen_secs, unique_artists, top_songs, top_artists, top_genres })
}
