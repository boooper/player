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
        plain_lyrics: v.get("plainLyrics").and_then(|x| x.as_str()).map(String::from),
        synced_lyrics: v.get("syncedLyrics").and_then(|x| x.as_str()).map(String::from),
        instrumental: v.get("instrumental").and_then(|x| x.as_bool()).unwrap_or(false),
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
    // Try exact match first when we have album + duration
    if !album.is_empty() && duration > 0.0 {
        let dur = (duration.round() as i64).to_string();
        let resp = state
            .http
            .get("https://lrclib.net/api/get")
            .query(&[
                ("artist_name", artist.as_str()),
                ("track_name", title.as_str()),
                ("album_name", album.as_str()),
                ("duration", dur.as_str()),
            ])
            .header("Lrclib-Client", "Naviarr")
            .send()
            .await;
        if let Ok(r) = resp {
            if r.status().is_success() {
                if let Ok(json) = r.json::<serde_json::Value>().await {
                    return Ok(Some(from_json(&json)));
                }
            }
        }
    }

    // Fall back to search
    let resp = state
        .http
        .get("https://lrclib.net/api/search")
        .query(&[("track_name", title.as_str()), ("artist_name", artist.as_str())])
        .header("Lrclib-Client", "Naviarr")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let results: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;
    if results.is_empty() {
        return Ok(None);
    }

    let best = results
        .iter()
        .find(|r| r.get("syncedLyrics").map(|v| !v.is_null() && v.as_str().map(|s| !s.is_empty()).unwrap_or(false)).unwrap_or(false))
        .or_else(|| results.first())
        .cloned();

    Ok(best.map(|b| from_json(&b)))
}
