use serde::{Deserialize, Serialize};
use crate::AppState;

mod musixmatch;
mod genius;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LyricsResult {
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
    pub instrumental: bool,
    pub provider: Option<String>,
    pub cached: bool,
}

pub(crate) fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn normalize_artist(value: &str) -> String {
    let lower = value.to_lowercase();
    let primary = lower.split(&[',', ';', '&'][..]).next().unwrap_or("");
    normalize_whitespace(primary)
}

pub(crate) fn normalize_title(value: &str) -> String {
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

pub(crate) fn normalize_album(value: &str) -> String {
    normalize_title(value)
}

pub(crate) fn cache_key(artist: &str, title: &str, album: &str, duration: f64) -> String {
    format!(
        "{}|{}|{}|{}",
        normalize_artist(artist),
        normalize_title(title),
        normalize_album(album),
        duration.round() as i64
    )
}

pub(crate) fn read_cached_result(state: &AppState, key: &str) -> Option<LyricsResult> {
    let (bytes, _) = state.lyrics_cache.read(key).ok()??;
    let mut cached = serde_json::from_slice::<LyricsResult>(&bytes).ok()?;
    cached.cached = true;
    Some(cached)
}

pub(crate) fn write_cached_result(state: &AppState, key: &str, value: &LyricsResult) {
    let Ok(bytes) = serde_json::to_vec(value) else {
        return;
    };
    let _ = state
        .lyrics_cache
        .write(key, "lyrics", "lyrics-cache", "application/json", &bytes);
}

pub(crate) fn clean_plain_lyrics(value: &str) -> Option<String> {
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

pub async fn fetch_lyrics_inner(
    state: &AppState,
    artist: &str,
    title: &str,
    album: &str,
    duration: f64,
    provider: Option<&str>,
) -> Result<Option<LyricsResult>, String> {
    let key = cache_key(artist, title, album, duration);
    if let Some(cached) = read_cached_result(state, &key) {
        return Ok(Some(cached));
    }

    let result = match provider {
        Some("musixmatch") => musixmatch::fetch_from_musixmatch(artist, title, album, duration).await.map_err(|e| e.to_string())?,
        Some("genius") => match genius::fetch_from_genius(artist, title, album, duration).await {
            Some(r) => Some(r),
            None => None,
        },
        _ => {
            // default: try musixmatch then fallback to genius
            match musixmatch::fetch_from_musixmatch(artist, title, album, duration).await {
                Ok(Some(r)) => Some(r),
                Ok(None) => genius::fetch_from_genius(artist, title, album, duration).await,
                Err(_) => genius::fetch_from_genius(artist, title, album, duration).await,
            }
        }
    };

    if let Some(result) = &result {
        write_cached_result(state, &key, result);
    }

    Ok(result)
}
