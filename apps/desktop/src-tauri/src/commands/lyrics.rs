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
                .header("Lrclib-Client", "Naviarr")
                .timeout(std::time::Duration::from_secs(10))
                .send().await.ok()?;
            if !r.status().is_success() { return None; }
            r.json::<serde_json::Value>().await.ok().map(|j| from_json(&j))
        },
        async {
            let r = state.http
                .get("https://lrclib.net/api/search")
                .query(&[("track_name", title.as_str()), ("artist_name", artist.as_str())])
                .header("Lrclib-Client", "Naviarr")
                .timeout(std::time::Duration::from_secs(10))
                .send().await.ok()?;
            if !r.status().is_success() { return None; }
            let results: Vec<serde_json::Value> = r.json().await.ok()?;
            results.iter()
                .find(|r| r.get("syncedLyrics").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false))
                .or_else(|| results.first())
                .map(from_json)
        }
    );

    // Prefer exact match (duration-aligned for synced lyrics), fall back to search
    Ok(exact.or(search))
}
