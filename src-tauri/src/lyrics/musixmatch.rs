use std::sync::LazyLock;

use base64::Engine;
use hmac::{Hmac, Mac};
use rand::Rng;
use serde::Deserialize;
use sha1::Sha1;
use url::Url;

use crate::lyrics::{clean_plain_lyrics, normalize_album, normalize_artist, normalize_title, LyricsResult};

const APP_ID: &str = "android-player-v1.0";
const API_URL: &str = "https://apic.musixmatch.com/ws/1.1/";
const SIG_SECRET: &[u8] = b"mNdca@6W7TeEcFn6*3.s97sJ*yPMd";
const USER_AGENT: &str = "Dalvik/2.1.0 (Linux; U; Android 13; Pixel 6 Build/T3B2.230316.003)";

static MXM_TOKEN: LazyLock<tokio::sync::Mutex<Option<String>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(None));

// ── Response types ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct MxmHeader {
    status_code: u16,
    #[serde(default)]
    hint: String,
}

#[derive(Deserialize)]
struct MxmResp<T> {
    message: MxmMsg<T>,
}

#[derive(Deserialize)]
struct MxmMsg<T> {
    header: MxmHeader,
    body: Option<T>,
}

#[derive(Deserialize)]
struct TokenBody {
    user_token: String,
}

#[derive(Deserialize)]
struct TrackBody {
    track: Track,
}

#[derive(Deserialize)]
struct TrackListBody {
    track_list: Vec<TrackBody>,
}

#[derive(Deserialize)]
struct Track {
    track_id: u64,
    track_name: String,
    artist_name: String,
    #[serde(default)]
    album_name: String,
    #[serde(default)]
    track_length: u32,
    #[serde(default, deserialize_with = "bool_from_int")]
    instrumental: bool,
}

#[derive(Deserialize)]
struct LyricsOuter {
    lyrics: LyricsInner,
}

#[derive(Deserialize)]
struct LyricsInner {
    #[serde(default)]
    lyrics_body: String,
    #[serde(default, deserialize_with = "bool_from_int")]
    restricted: bool,
}

#[derive(Deserialize)]
struct SubtitleOuter {
    subtitle: SubtitleInner,
}

#[derive(Deserialize)]
struct SubtitleInner {
    #[serde(default)]
    subtitle_body: String,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn bool_from_int<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    struct V;
    impl serde::de::Visitor<'_> for V {
        type Value = bool;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("bool or integer")
        }
        fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<bool, E> { Ok(v) }
        fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<bool, E> { Ok(v != 0) }
        fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<bool, E> { Ok(v != 0) }
    }
    d.deserialize_any(V)
}

/// Returns current UTC date as `YYYYMMDD` without any external date crate.
/// Uses Howard Hinnant's civil_from_days algorithm.
fn today_yyyymmdd() -> String {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86400;
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe as i64 + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}{m:02}{d:02}")
}

fn sign_and_finish(url: &mut Url) {
    let mut mac = Hmac::<Sha1>::new_from_slice(SIG_SECRET).expect("HMAC accepts any key length");
    mac.update(url.as_str().as_bytes());
    mac.update(today_yyyymmdd().as_bytes());
    let sig = mac.finalize().into_bytes();
    let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig) + "\n";
    url.query_pairs_mut()
        .append_pair("signature", &sig_b64)
        .append_pair("signature_protocol", "sha1");
}

fn api_url(endpoint: &str, token: &str, extra: &[(&str, &str)]) -> String {
    let mut url = Url::parse_with_params(
        &format!("{API_URL}{endpoint}"),
        &[("app_id", APP_ID), ("usertoken", token), ("format", "json")],
    )
    .unwrap();
    for (k, v) in extra {
        url.query_pairs_mut().append_pair(k, v);
    }
    sign_and_finish(&mut url);
    url.to_string()
}

fn parse_mxm<T: serde::de::DeserializeOwned>(text: &str) -> Result<Option<T>, String> {
    let resp: MxmResp<T> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let h = resp.message.header;
    match h.status_code {
        200..=299 => Ok(resp.message.body),
        404 | 400 => Ok(None),
        401 if h.hint == "renew" => Err("__token_expired__".into()),
        code => Err(format!("musixmatch {code}: {}", h.hint)),
    }
}

// ── Token management ─────────────────────────────────────────────────────────

async fn fetch_new_token(http: &reqwest::Client) -> Result<String, String> {
    let (adv_id, guid) = {
        let mut rng = rand::rng();
        let adv = format!(
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            rng.random::<u32>(),
            rng.random::<u16>(),
            rng.random::<u16>(),
            rng.random::<u16>(),
            rng.random::<u64>() & 0xffffffffffff,
        );
        let guid = format!("{:016x}", rng.random::<u64>());
        (adv, guid)
    };

    let mut url = Url::parse_with_params(
        &format!("{API_URL}token.get"),
        &[
            ("app_id", APP_ID),
            ("format", "json"),
            ("adv_id", adv_id.as_str()),
            ("guid", guid.as_str()),
            ("root", "0"),
            ("sideloaded", "0"),
            ("build_number", "2022090901"),
            ("lang", "en_US"),
        ],
    )
    .map_err(|e| e.to_string())?;
    sign_and_finish(&mut url);

    let text = http
        .get(url.as_str())
        .header("User-Agent", USER_AGENT)
        .header("Cookie", "AWSELBCORS=0; AWSELB=0")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    let resp: MxmResp<TokenBody> =
        serde_json::from_str(&text).map_err(|e| e.to_string())?;
    resp.message
        .body
        .ok_or_else(|| "no token in response".into())
        .map(|b| b.user_token)
}

async fn ensure_token(http: &reqwest::Client) -> Result<String, String> {
    let mut guard = MXM_TOKEN.lock().await;
    if let Some(t) = guard.as_ref() {
        return Ok(t.clone());
    }
    let token = fetch_new_token(http).await?;
    *guard = Some(token.clone());
    Ok(token)
}

async fn get<T: serde::de::DeserializeOwned>(
    http: &reqwest::Client,
    endpoint: &str,
    params: &[(&str, &str)],
) -> Result<Option<T>, String> {
    let token = ensure_token(http).await?;
    let url = api_url(endpoint, &token, params);

    let text = http
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Cookie", "AWSELBCORS=0; AWSELB=0")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    match parse_mxm::<T>(&text) {
        Err(e) if e == "__token_expired__" => {
            let new_token = {
                let mut guard = MXM_TOKEN.lock().await;
                let t = fetch_new_token(http).await?;
                *guard = Some(t.clone());
                t
            };
            let url = api_url(endpoint, &new_token, params);
            let text = http
                .get(&url)
                .header("User-Agent", USER_AGENT)
                .header("Cookie", "AWSELBCORS=0; AWSELB=0")
                .send()
                .await
                .map_err(|e| e.to_string())?
                .text()
                .await
                .map_err(|e| e.to_string())?;
            parse_mxm::<T>(&text)
        }
        other => other,
    }
}

// ── Public entry point ───────────────────────────────────────────────────────

pub async fn fetch_from_musixmatch(
    http: &reqwest::Client,
    artist: &str,
    title: &str,
    album: &str,
    duration: f64,
) -> Result<Option<LyricsResult>, String> {
    // Try the smart matcher first, then fall back to search
    let matcher_track: Option<Track> = {
        let mut p = vec![("q_track", title), ("q_artist", artist)];
        if !album.is_empty() {
            p.push(("q_album", album));
        }
        get::<TrackBody>(http, "matcher.track.get", &p)
            .await
            .ok()
            .flatten()
            .map(|b| b.track)
    };

    let track = if let Some(t) = matcher_track {
        Some(t)
    } else {
        get::<TrackListBody>(http, "track.search", &[
            ("q_track", title),
            ("q_artist", artist),
            ("f_has_lyrics", "1"),
            ("s_track_rating", "desc"),
            ("page_size", "5"),
            ("page", "1"),
        ])
        .await
        .unwrap_or(None)
        .and_then(|body| {
            body.track_list.into_iter().map(|b| b.track).find(|t| {
                let artist_ok = normalize_artist(&t.artist_name) == normalize_artist(artist);
                let title_ok = normalize_title(&t.track_name) == normalize_title(title);
                let album_ok = album.is_empty()
                    || normalize_album(&t.album_name) == normalize_album(album);
                let dur_ok = duration <= 0.0
                    || (t.track_length as f64 - duration).abs() <= 3.0
                    || ((t.track_length as f64 - duration).abs() <= 8.0 && title_ok);
                artist_ok && title_ok && album_ok && dur_ok
            })
        })
    };

    let Some(track) = track else {
        return Ok(None);
    };

    let id = track.track_id.to_string();

    let plain_lyrics = get::<LyricsOuter>(http, "track.lyrics.get", &[("track_id", &id)])
        .await
        .unwrap_or(None)
        .and_then(|o| {
            if o.lyrics.restricted {
                None
            } else {
                clean_plain_lyrics(&o.lyrics.lyrics_body)
            }
        });

    let dur_str;
    let synced_lyrics = if duration > 0.0 {
        dur_str = format!("{}", duration as u32);
        get::<SubtitleOuter>(http, "track.subtitle.get", &[
            ("track_id", &id),
            ("subtitle_format", "lrc"),
            ("f_subtitle_length", &dur_str),
            ("f_subtitle_length_max_deviation", "2"),
        ])
        .await
        .unwrap_or(None)
        .and_then(|o| clean_plain_lyrics(&o.subtitle.subtitle_body))
    } else {
        None
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
