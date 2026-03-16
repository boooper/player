use serde::Serialize;
use tauri::State;
use crate::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsResult {
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
    pub instrumental: bool,
}

fn from_json(v: &serde_json::Value) -> LyricsResult {
    LyricsResult {
        plain_lyrics:  v.get("plainLyrics").and_then(|x| x.as_str()).filter(|s| !s.is_empty()).map(String::from),
        synced_lyrics: v.get("syncedLyrics").and_then(|x| x.as_str()).filter(|s| !s.is_empty()).map(String::from),
        instrumental:  v.get("instrumental").and_then(|x| x.as_bool()).unwrap_or(false),
    }
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_artist(value: &str) -> String {
    let lower = value.to_lowercase();
    let primary = lower
        .split(&[',', ';', '&'][..])
        .next()
        .unwrap_or("");
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

fn duration_matches(candidate: &serde_json::Value, duration: f64, tolerance: f64) -> bool {
    let Some(candidate_duration) = candidate.get("duration").and_then(|v| v.as_f64()) else {
        return false;
    };
    (candidate_duration - duration).abs() <= tolerance
}

fn title_similarity(a: &str, b: &str) -> bool {
    a == b || a.contains(b) || b.contains(a)
}

fn score_candidate(
    candidate: &serde_json::Value,
    artist: &str,
    title: &str,
    album: &str,
    duration: f64,
) -> i32 {
    let candidate_artist = normalize_artist(
        candidate.get("artistName").and_then(|v| v.as_str()).unwrap_or("")
    );
    let candidate_title = normalize_title(
        candidate.get("trackName").and_then(|v| v.as_str()).unwrap_or("")
    );
    let candidate_album = normalize_album(
        candidate.get("albumName").and_then(|v| v.as_str()).unwrap_or("")
    );

    let mut score = 0;

    if candidate_artist == artist {
        score += 45;
    } else if !candidate_artist.is_empty() && (candidate_artist.contains(artist) || artist.contains(&candidate_artist)) {
        score += 25;
    } else {
        score -= 40;
    }

    if candidate_title == title {
        score += 55;
    } else if !candidate_title.is_empty() && title_similarity(&candidate_title, title) {
        score += 25;
    } else {
        score -= 60;
    }

    if !album.is_empty() && !candidate_album.is_empty() {
        if candidate_album == album {
            score += 20;
        } else if title_similarity(&candidate_album, album) {
            score += 8;
        } else {
            score -= 10;
        }
    }

    if duration > 0.0 {
        if duration_matches(candidate, duration, 2.0) {
            score += 35;
        } else if duration_matches(candidate, duration, 5.0) {
            score += 15;
        } else if duration_matches(candidate, duration, 10.0) {
            score += 5;
        } else {
            score -= 25;
        }
    }

    if candidate.get("syncedLyrics").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false) {
        score += 6;
    }
    if candidate.get("plainLyrics").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false) {
        score += 3;
    }
    if candidate.get("instrumental").and_then(|v| v.as_bool()).unwrap_or(false) {
        score += 2;
    }

    score
}

fn best_search_result(
    results: &[serde_json::Value],
    artist: &str,
    title: &str,
    album: &str,
    duration: f64,
) -> Option<LyricsResult> {
    let normalized_artist = normalize_artist(artist);
    let normalized_title = normalize_title(title);
    let normalized_album = normalize_album(album);

    let best = results
        .iter()
        .filter_map(|candidate| {
            let score = score_candidate(
                candidate,
                &normalized_artist,
                &normalized_title,
                &normalized_album,
                duration,
            );
            (score >= 55).then_some((score, candidate))
        })
        .max_by_key(|(score, _)| *score)?;

    Some(from_json(best.1))
}

#[tauri::command]
pub async fn fetch_lyrics(
    state: State<'_, AppState>,
    artist: String,
    title: String,
    album: String,
    duration: f64,
) -> Result<Option<LyricsResult>, String> {
    let dur = (duration.round() as i64).to_string();

    // Fire exact-match and search in parallel
    let (exact, search) = tokio::join!(
        async {
            if album.is_empty() || duration <= 0.0 { return None; }
            let r = state.http
                .get("https://lrclib.net/api/get")
                .query(&[
                    ("artist_name", artist.as_str()),
                    ("track_name",  title.as_str()),
                    ("album_name",  album.as_str()),
                    ("duration",    dur.as_str()),
                ])
                .header("Lrclib-Client", "Madrify")
                .timeout(std::time::Duration::from_secs(10))
                .send().await.ok()?;
            if !r.status().is_success() { return None; }
            r.json::<serde_json::Value>().await.ok().map(|j| from_json(&j))
        },
        async {
            let r = state.http
                .get("https://lrclib.net/api/search")
                .query(&[("track_name", title.as_str()), ("artist_name", artist.as_str())])
                .header("Lrclib-Client", "Madrify")
                .timeout(std::time::Duration::from_secs(10))
                .send().await.ok()?;
            if !r.status().is_success() { return None; }
            let results: Vec<serde_json::Value> = r.json().await.ok()?;
            best_search_result(&results, &artist, &title, &album, duration)
        }
    );

    // Prefer exact match (duration-aligned for synced lyrics), fall back to search
    Ok(exact.or(search))
}
