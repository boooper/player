use serde::Deserialize;

use crate::lyrics::{clean_plain_lyrics, LyricsResult};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrclibResponse {
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
    #[serde(default)]
    instrumental: bool,
}

pub async fn fetch_from_lrclib(
    http: &reqwest::Client,
    artist: &str,
    title: &str,
    album: &str,
    duration: f64,
) -> Option<LyricsResult> {
    let mut req = http
        .get("https://lrclib.net/api/get")
        .header("Lrclib-Client", "Madrify (https://github.com/madrify/madrify)")
        .query(&[("artist_name", artist), ("track_name", title)]);

    if !album.is_empty() {
        req = req.query(&[("album_name", album)]);
    }
    if duration > 0.0 {
        req = req.query(&[("duration", &duration.round().to_string())]);
    }

    let resp = req.send().await.ok()?;
    if resp.status() == 404 {
        return None;
    }
    let data: LrclibResponse = resp.json().await.ok()?;

    if data.instrumental {
        return Some(LyricsResult {
            plain_lyrics: None,
            synced_lyrics: None,
            instrumental: true,
            provider: Some("LRCLIB".to_string()),
            cached: false,
        });
    }

    let plain_lyrics = data.plain_lyrics.as_deref().and_then(clean_plain_lyrics);
    let synced_lyrics = data.synced_lyrics.filter(|s| !s.trim().is_empty());

    if plain_lyrics.is_none() && synced_lyrics.is_none() {
        return None;
    }

    Some(LyricsResult {
        plain_lyrics,
        synced_lyrics,
        instrumental: false,
        provider: Some("LRCLIB".to_string()),
        cached: false,
    })
}
