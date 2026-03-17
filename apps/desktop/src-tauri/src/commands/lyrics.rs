use std::collections::HashSet;

use genius_lyrics::get_lyrics_from_url;
use musixmatch_inofficial::{
    models::{SortOrder, SubtitleFormat, TrackId},
    Error as MusixmatchError, Musixmatch,
};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::AppState;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LyricsResult {
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
    pub instrumental: bool,
    pub provider: Option<String>,
    pub cached: bool,
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_artist(value: &str) -> String {
    let lower = value.to_lowercase();
    let primary = lower.split(&[',', ';', '&'][..]).next().unwrap_or("");
    normalize_whitespace(primary)
}

fn normalize_title(value: &str) -> String {
    let lower = value.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    let chars: Vec<char> = lower.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        if ch == '(' || ch == '[' {
            let closing = if ch == '(' { ')' } else { ']' };
            let mut j = i + 1;
            let mut inner = String::new();
            while j < chars.len() && chars[j] != closing {
                inner.push(chars[j]);
                j += 1;
            }

            let trimmed = inner.trim();
            if trimmed.contains("remaster")
                || trimmed.contains("deluxe")
                || trimmed.contains("edition")
                || trimmed.contains("version")
                || trimmed.contains("live")
                || trimmed.contains("mono")
                || trimmed.contains("stereo")
                || trimmed.contains("bonus")
            {
                i = if j < chars.len() { j + 1 } else { chars.len() };
                continue;
            }
        }

        out.push(ch);
        i += 1;
    }

    let out = out
        .replace(" feat. ", " ")
        .replace(" featuring ", " ")
        .replace(" ft. ", " ")
        .replace(" - remastered", " ")
        .replace(" - live", " ")
        .replace(" - mono", " ")
        .replace(" - stereo", " ");

    normalize_whitespace(&out)
}

fn normalize_album(value: &str) -> String {
    normalize_title(value)
}

fn cache_key(artist: &str, title: &str, album: &str, duration: f64) -> String {
    format!(
        "{}|{}|{}|{}",
        normalize_artist(artist),
        normalize_title(title),
        normalize_album(album),
        duration.round() as i64
    )
}

fn read_cached_result(state: &AppState, key: &str) -> Option<LyricsResult> {
    let (bytes, _) = state.lyrics_cache.read(key).ok()??;
    let mut cached = serde_json::from_slice::<LyricsResult>(&bytes).ok()?;
    cached.cached = true;
    Some(cached)
}

fn write_cached_result(state: &AppState, key: &str, value: &LyricsResult) {
    let Ok(bytes) = serde_json::to_vec(value) else {
        return;
    };
    let _ = state
        .lyrics_cache
        .write(key, "lyrics", "lyrics-cache", "application/json", &bytes);
}

fn clean_plain_lyrics(value: &str) -> Option<String> {
    let trimmed = value
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn musixmatch_error_is_miss(error: &MusixmatchError) -> bool {
    matches!(error, MusixmatchError::NotAvailable | MusixmatchError::NotFound)
}

async fn fetch_from_musixmatch(
    artist: &str,
    title: &str,
    album: &str,
    duration: f64,
) -> Result<Option<LyricsResult>, String> {
    let mxm = Musixmatch::builder()
        .no_storage()
        .build()
        .map_err(|e| e.to_string())?;

    let matcher_track = mxm
        .matcher_track(title, artist, album, false, false, false)
        .await
        .ok();
    let search_tracks = mxm
        .track_search()
        .q_track(title)
        .q_artist(artist)
        .f_has_lyrics()
        .s_track_rating(SortOrder::Desc)
        .send(5, 1)
        .await
        .unwrap_or_default();

    let track = matcher_track.or_else(|| {
        search_tracks.into_iter().find(|track| {
            let artist_ok = normalize_artist(&track.artist_name) == normalize_artist(artist);
            let title_ok = normalize_title(&track.track_name) == normalize_title(title);
            let album_ok = album.is_empty()
                || normalize_album(&track.album_name) == normalize_album(album);
            let duration_ok = duration <= 0.0
                || (track.track_length as f64 - duration).abs() <= 3.0
                || (track.track_length as f64 - duration).abs() <= 8.0 && title_ok;
            artist_ok && title_ok && album_ok && duration_ok
        })
    });

    let Some(track) = track else {
        return Ok(None);
    };

    let plain_lyrics = match mxm.track_lyrics(TrackId::TrackId(track.track_id)).await {
        Ok(lyrics) => clean_plain_lyrics(&lyrics.lyrics_body),
        Err(error) if musixmatch_error_is_miss(&error) => None,
        Err(error) => return Err(error.to_string()),
    };

    let synced_lyrics = match mxm
        .track_subtitle(
            TrackId::TrackId(track.track_id),
            SubtitleFormat::Lrc,
            if duration > 0.0 { Some(duration as f32) } else { None },
            if duration > 0.0 { Some(2.0) } else { None },
        )
        .await
    {
        Ok(subtitle) => clean_plain_lyrics(&subtitle.subtitle_body),
        Err(error) if musixmatch_error_is_miss(&error) => None,
        Err(error) => return Err(error.to_string()),
    };

    if plain_lyrics.is_none() && synced_lyrics.is_none() && !track.instrumental {
        return Ok(None);
    }

    Ok(Some(LyricsResult {
        plain_lyrics,
        synced_lyrics,
        instrumental: track.instrumental,
        provider: Some("Musixmatch".to_string()),
        cached: false,
    }))
}

fn slugify_genius_part(value: &str) -> String {
    let normalized = normalize_title(value)
        .replace('\'', "")
        .replace('.', "")
        .replace('/', " ")
        .replace(':', " ")
        .replace('&', " and ");
    let mut out = String::new();
    let mut last_dash = false;
    for ch in normalized.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn build_genius_url_candidates(artist: &str, title: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    let artist_slug = slugify_genius_part(artist);
    let title_slug = slugify_genius_part(title);

    for candidate_title in [
        title_slug.clone(),
        slugify_genius_part(&normalize_title(title).replace(" and ", " ")),
    ] {
        if artist_slug.is_empty() || candidate_title.is_empty() {
            continue;
        }
        let url = format!("https://genius.com/{}-{}-lyrics", artist_slug, candidate_title);
        if seen.insert(url.clone()) {
            candidates.push(url);
        }
    }

    candidates
}

async fn fetch_from_genius(artist: &str, title: &str) -> Option<LyricsResult> {
    for url in build_genius_url_candidates(artist, title) {
        let Ok(lyrics) = get_lyrics_from_url(&url).await else {
            continue;
        };
        let Some(plain_lyrics) = clean_plain_lyrics(&lyrics) else {
            continue;
        };
        return Some(LyricsResult {
            plain_lyrics: Some(plain_lyrics),
            synced_lyrics: None,
            instrumental: false,
            provider: Some("Genius".to_string()),
            cached: false,
        });
    }

    None
}

#[tauri::command]
pub async fn fetch_lyrics(
    state: State<'_, AppState>,
    artist: String,
    title: String,
    album: String,
    duration: f64,
) -> Result<Option<LyricsResult>, String> {
    let key = cache_key(&artist, &title, &album, duration);
    if let Some(cached) = read_cached_result(&state, &key) {
        return Ok(Some(cached));
    }

    let result = match fetch_from_musixmatch(&artist, &title, &album, duration).await {
        Ok(Some(result)) => Some(result),
        Ok(None) => fetch_from_genius(&artist, &title).await,
        Err(_) => fetch_from_genius(&artist, &title).await,
    };

    if let Some(result) = &result {
        write_cached_result(&state, &key, result);
    }

    Ok(result)
}
