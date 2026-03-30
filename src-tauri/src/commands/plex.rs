//! Plex music library implementation.
//!
//! Authentication uses the Plex token stored in `profile.password` and sent as
//! `X-Plex-Token` on API requests. Direct media/image URLs also embed the token
//! as a query param so the webview can load them directly.

use crate::commands::media::{Album, AlbumDetail, AlbumFull, Playlist, PlaylistDetail, PlaylistMeta, Song};
use crate::commands::profiles::ActiveProfile;
use crate::commands::settings;
use crate::AppState;
use rand::{distr::Alphanumeric, Rng};
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tauri::State;
use url::Url;
use url::form_urlencoded;

const PLEX_CLIENT_ID_KEY: &str = "PLEX_CLIENT_ID";
const PLEX_PRODUCT_NAME: &str = "Madrify";

static SECTION_KEY_CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn section_key_cache() -> &'static Mutex<HashMap<String, String>> {
    SECTION_KEY_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlexAuthStart {
    pub pin_id: i64,
    pub code: String,
    pub auth_url: String,
    pub expires_at: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlexServerResource {
    pub name: String,
    pub uri: String,
    pub machine_identifier: String,
    pub access_token: String,
    pub owned: bool,
    pub source_title: Option<String>,
    pub local: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlexAuthPollResult {
    pub status: String,
    pub username: Option<String>,
    pub auth_token: Option<String>,
    pub expires_at: Option<String>,
    pub servers: Vec<PlexServerResource>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlexPinResponse {
    id: i64,
    code: String,
    auth_token: Option<String>,
    expires_at: Option<String>,
}

fn attr(node: Node<'_, '_>, name: &str) -> String {
    node.attribute(name).unwrap_or("").to_string()
}

fn attr_f64(node: Node<'_, '_>, name: &str) -> f64 {
    node.attribute(name)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0)
}

fn attr_u32(node: Node<'_, '_>, name: &str) -> Option<u32> {
    node.attribute(name)
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
}

fn millis_to_seconds(value: f64) -> f64 {
    if value <= 0.0 { 0.0 } else { value / 1000.0 }
}

fn join_authed_url(p: &ActiveProfile, path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }

    let base = format!("{}/", p.url.trim_end_matches('/'));
    let Ok(base_url) = Url::parse(&base) else {
        return String::new();
    };
    let Ok(mut url) = base_url.join(path.trim_start_matches('/')) else {
        return String::new();
    };
    url.query_pairs_mut().append_pair("X-Plex-Token", &p.password);
    url.to_string()
}

fn cover_url(p: &ActiveProfile, path: &str) -> String {
    join_authed_url(p, path)
}

fn stream_url(p: &ActiveProfile, path: &str) -> String {
    join_authed_url(p, path)
}

fn platform_name() -> &'static str {
    match std::env::consts::OS {
        "windows" => "Windows",
        "macos" => "macOS",
        "linux" => "Linux",
        other => other,
    }
}

fn with_plex_headers(
    request: reqwest::RequestBuilder,
    client_id: &str,
) -> reqwest::RequestBuilder {
    request
        .header("X-Plex-Product", PLEX_PRODUCT_NAME)
        .header("X-Plex-Version", env!("CARGO_PKG_VERSION"))
        .header("X-Plex-Client-Identifier", client_id)
        .header("X-Plex-Platform", platform_name())
        .header("X-Plex-Platform-Version", std::env::consts::ARCH)
        .header("X-Plex-Device", format!("{PLEX_PRODUCT_NAME} Desktop"))
        .header("X-Plex-Device-Name", format!("{PLEX_PRODUCT_NAME} Desktop"))
}

fn get_or_create_client_id(db: &rusqlite::Connection) -> Result<String, String> {
    let settings_map = settings::read_all(db)?;
    if let Some(existing) = settings_map.get(PLEX_CLIENT_ID_KEY).map(String::as_str) {
        let trimmed = existing.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    let client_id: String = rand::rng()
        .sample_iter(Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    settings::upsert(db, PLEX_CLIENT_ID_KEY, &client_id)?;
    Ok(client_id)
}

fn build_auth_url(client_id: &str, code: &str) -> String {
    let query = form_urlencoded::Serializer::new(String::new())
        .append_pair("clientID", client_id)
        .append_pair("code", code)
        .append_pair("context[device][product]", PLEX_PRODUCT_NAME)
        .finish();
    format!("https://app.plex.tv/auth#?{query}")
}

async fn plex_json<T: serde::de::DeserializeOwned>(
    _http: &reqwest::Client,
    client_id: &str,
    request: reqwest::RequestBuilder,
) -> Result<T, String> {
    let resp = with_plex_headers(request, client_id)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Plex auth: HTTP {}", resp.status()));
    }

    resp.json::<T>().await.map_err(|e| e.to_string())
}

async fn plex_text(
    _http: &reqwest::Client,
    client_id: &str,
    request: reqwest::RequestBuilder,
) -> Result<String, String> {
    let resp = with_plex_headers(request, client_id)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Plex auth: HTTP {}", resp.status()));
    }

    resp.text().await.map_err(|e| e.to_string())
}

async fn fetch_plex_servers(
    http: &reqwest::Client,
    client_id: &str,
    account_token: &str,
) -> Result<Vec<PlexServerResource>, String> {
    let xml = plex_text(
        http,
        client_id,
        http.get("https://plex.tv/api/v2/resources?includeHttps=1")
            .header("X-Plex-Token", account_token)
            .header("Accept", "application/xml"),
    )
    .await?;
    let doc = parse_doc(&xml)?;
    let mut servers: Vec<PlexServerResource> = doc
        .descendants()
        .filter(|node| node.has_tag_name("Device"))
        .filter_map(|node| {
            let provides = node.attribute("provides").unwrap_or("");
            if !provides.split(',').any(|value| value.trim() == "server") {
                return None;
            }

            let access_token = node
                .attribute("accessToken")
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(account_token)
                .to_string();

            let best_connection = node
                .children()
                .filter(|child| child.has_tag_name("Connection"))
                .filter_map(|child| {
                    let uri = child.attribute("uri")?.trim();
                    if uri.is_empty() {
                        return None;
                    }
                    let local = child.attribute("local") == Some("1");
                    let secure = child.attribute("protocol") == Some("https")
                        || uri.starts_with("https://");
                    Some((uri.to_string(), local, secure))
                })
                .max_by_key(|(_, local, secure)| (*local as u8, *secure as u8));

            let (uri, local, _) = best_connection?;
            Some(PlexServerResource {
                name: node.attribute("name").unwrap_or("Plex Server").to_string(),
                uri,
                machine_identifier: node.attribute("clientIdentifier").unwrap_or("").to_string(),
                access_token,
                owned: node.attribute("owned") == Some("1"),
                source_title: node.attribute("sourceTitle").map(ToOwned::to_owned),
                local,
            })
        })
        .collect();

    servers.sort_by(|a, b| {
        b.owned
            .cmp(&a.owned)
            .then_with(|| b.local.cmp(&a.local))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(servers)
}

async fn fetch_plex_username(
    http: &reqwest::Client,
    client_id: &str,
    account_token: &str,
) -> Result<Option<String>, String> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct PlexUserResponse {
        username: Option<String>,
        title: Option<String>,
        friendly_name: Option<String>,
    }

    let user = plex_json::<PlexUserResponse>(
        http,
        client_id,
        http.get("https://plex.tv/api/v2/user").header("X-Plex-Token", account_token),
    )
    .await?;

    Ok(user
        .username
        .or(user.title)
        .or(user.friendly_name)
        .filter(|value| !value.trim().is_empty()))
}

#[tauri::command]
pub async fn plex_begin_auth(state: State<'_, AppState>) -> Result<PlexAuthStart, String> {
    let client_id = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        get_or_create_client_id(&db)?
    };

    let pin = plex_json::<PlexPinResponse>(
        &state.http,
        &client_id,
        state
            .http
            .post("https://plex.tv/api/v2/pins?strong=true"),
    )
    .await?;

    Ok(PlexAuthStart {
        pin_id: pin.id,
        code: pin.code.clone(),
        auth_url: build_auth_url(&client_id, &pin.code),
        expires_at: pin.expires_at,
    })
}

#[tauri::command]
pub async fn plex_poll_auth(
    state: State<'_, AppState>,
    pin_id: i64,
) -> Result<PlexAuthPollResult, String> {
    let client_id = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        get_or_create_client_id(&db)?
    };

    let pin = plex_json::<PlexPinResponse>(
        &state.http,
        &client_id,
        state.http.get(format!("https://plex.tv/api/v2/pins/{pin_id}")),
    )
    .await?;

    let auth_token = pin.auth_token.filter(|value| !value.trim().is_empty());
    let Some(auth_token) = auth_token else {
        return Ok(PlexAuthPollResult {
            status: if pin.expires_at.is_some() { "pending".to_string() } else { "expired".to_string() },
            username: None,
            auth_token: None,
            expires_at: pin.expires_at,
            servers: Vec::new(),
        });
    };

    let (username, servers) = tokio::join!(
        fetch_plex_username(&state.http, &client_id, &auth_token),
        fetch_plex_servers(&state.http, &client_id, &auth_token),
    );

    Ok(PlexAuthPollResult {
        status: "complete".to_string(),
        username: username?,
        auth_token: Some(auth_token),
        expires_at: pin.expires_at,
        servers: servers?,
    })
}

async fn get_xml(
    http: &reqwest::Client,
    p: &ActiveProfile,
    path: &str,
    params: &[(&str, &str)],
) -> Result<String, String> {
    let base = format!("{}{}", p.url.trim_end_matches('/'), path);
    let mut url = Url::parse(&base).map_err(|e| e.to_string())?;
    {
        let mut pairs = url.query_pairs_mut();
        for (k, v) in params {
            pairs.append_pair(k, v);
        }
    }

    let resp = http
        .get(url.as_str())
        .header("X-Plex-Token", &p.password)
        .header("Accept", "application/xml")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Plex: HTTP {}", resp.status()));
    }

    resp.text().await.map_err(|e| e.to_string())
}

fn parse_doc<'a>(xml: &'a str) -> Result<Document<'a>, String> {
    Document::parse(xml).map_err(|e| e.to_string())
}

fn first_music_section_key(doc: &Document<'_>) -> Option<String> {
    doc.descendants()
        .find(|node| {
            node.has_tag_name("Directory")
                && matches!(node.attribute("type"), Some("artist"))
        })
        .and_then(|node| node.attribute("key"))
        .map(ToOwned::to_owned)
}

async fn music_section_key(http: &reqwest::Client, p: &ActiveProfile) -> Result<String, String> {
    if let Some(key) = section_key_cache().lock().unwrap().get(&p.url).cloned() {
        return Ok(key);
    }
    let xml = get_xml(http, p, "/library/sections", &[]).await?;
    let doc = parse_doc(&xml)?;
    let key = first_music_section_key(&doc).ok_or_else(|| "No Plex music library section found.".to_string())?;
    section_key_cache().lock().unwrap().insert(p.url.clone(), key.clone());
    Ok(key)
}

fn map_song(node: Node<'_, '_>, p: &ActiveProfile) -> Song {
    let album_id = attr(node, "parentRatingKey");
    let thumb = node
        .attribute("thumb")
        .or_else(|| node.attribute("parentThumb"))
        .or_else(|| node.attribute("grandparentThumb"))
        .unwrap_or("");
    let media = node.children().find(|child| child.has_tag_name("Media"));
    let part = media.and_then(|media_node| media_node.children().find(|child| child.has_tag_name("Part")));
    let stream_key = part
        .and_then(|part_node| part_node.attribute("key"))
        .or_else(|| node.attribute("key"))
        .unwrap_or("");

    Song {
        id: attr(node, "ratingKey"),
        title: attr(node, "title"),
        artist: node
            .attribute("grandparentTitle")
            .or_else(|| node.attribute("originalTitle"))
            .unwrap_or("")
            .to_string(),
        album: attr(node, "parentTitle"),
        album_id,
        cover_art: thumb.to_string(),
        cover_art_url: cover_url(p, thumb),
        stream_url: stream_url(p, stream_key),
        duration: millis_to_seconds(attr_f64(node, "duration")),
        audio_format: media
            .and_then(|media_node| media_node.attribute("container"))
            .map(ToOwned::to_owned)
            .filter(|value| !value.is_empty()),
        bitrate_kbps: media.and_then(|media_node| attr_u32(media_node, "bitrate")),
        genre: node.attribute("parentGenre")
            .or_else(|| node.attribute("Genre"))
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned),
    }
}

fn map_album(node: Node<'_, '_>, p: &ActiveProfile) -> Album {
    let thumb = node
        .attribute("thumb")
        .or_else(|| node.attribute("parentThumb"))
        .unwrap_or("");

    Album {
        id: attr(node, "ratingKey"),
        name: attr(node, "title"),
        artist: attr(node, "parentTitle"),
        artist_id: attr(node, "parentRatingKey"),
        cover_art: thumb.to_string(),
        cover_art_url: cover_url(p, thumb),
        song_count: attr_f64(node, "leafCount"),
        duration: millis_to_seconds(attr_f64(node, "duration")),
        year: node.attribute("year").and_then(|value| value.parse::<f64>().ok()),
    }
}

fn map_playlist(node: Node<'_, '_>, p: &ActiveProfile) -> Playlist {
    let art = node
        .attribute("composite")
        .or_else(|| node.attribute("thumb"))
        .unwrap_or("");

    Playlist {
        id: attr(node, "ratingKey"),
        name: attr(node, "title"),
        song_count: attr_f64(node, "leafCount"),
        duration: millis_to_seconds(attr_f64(node, "duration")),
        cover_art: art.to_string(),
        cover_art_url: cover_url(p, art),
    }
}

pub(crate) async fn ping(http: &reqwest::Client, p: &ActiveProfile) -> Result<bool, String> {
    let xml = get_xml(http, p, "/", &[]).await?;
    let doc = parse_doc(&xml)?;
    Ok(doc.descendants().any(|node| node.has_tag_name("MediaContainer")))
}

pub(crate) async fn search(
    http: &reqwest::Client,
    p: &ActiveProfile,
    query: &str,
    count: u32,
) -> Result<Vec<Song>, String> {
    let section = music_section_key(http, p).await?;
    let limit = count.to_string();
    let xml = get_xml(
        http,
        p,
        &format!("/library/sections/{section}/search"),
        &[("query", query), ("limit", &limit)],
    )
    .await?;
    let doc = parse_doc(&xml)?;
    Ok(doc
        .descendants()
        .filter(|node| node.has_tag_name("Track"))
        .take(count as usize)
        .map(|node| map_song(node, p))
        .collect())
}

pub(crate) async fn similar(
    http: &reqwest::Client,
    p: &ActiveProfile,
    song_id: &str,
    count: u32,
) -> Result<Vec<Song>, String> {
    let xml = get_xml(http, p, &format!("/library/metadata/{song_id}"), &[]).await?;
    let doc = parse_doc(&xml)?;
    let artist = doc
        .descendants()
        .find(|node| node.has_tag_name("Track"))
        .and_then(|node| node.attribute("grandparentTitle"))
        .unwrap_or("")
        .to_string();

    if artist.is_empty() {
        return Ok(Vec::new());
    }

    let mut songs = search(http, p, &artist, count.saturating_mul(2).max(8)).await?;
    songs.retain(|song| song.id != song_id);
    songs.truncate(count as usize);
    Ok(songs)
}

pub(crate) async fn playlists(
    http: &reqwest::Client,
    p: &ActiveProfile,
) -> Result<Vec<Playlist>, String> {
    let xml = get_xml(http, p, "/playlists/all", &[("playlistType", "audio")]).await?;
    let doc = parse_doc(&xml)?;
    Ok(doc
        .descendants()
        .filter(|node| node.has_tag_name("Playlist"))
        .map(|node| map_playlist(node, p))
        .collect())
}

pub(crate) async fn playlist(
    http: &reqwest::Client,
    p: &ActiveProfile,
    id: &str,
) -> Result<PlaylistDetail, String> {
    let playlist_path = format!("/playlists/{id}");
    let playlist_items_path = format!("/playlists/{id}/items");
    let (meta_xml, items_xml) = tokio::join!(
        get_xml(http, p, &playlist_path, &[]),
        get_xml(http, p, &playlist_items_path, &[]),
    );
    let meta_xml = meta_xml?;
    let items_xml = items_xml?;
    let meta_doc = parse_doc(&meta_xml)?;
    let items_doc = parse_doc(&items_xml)?;
    let songs: Vec<Song> = items_doc
        .descendants()
        .filter(|node| node.has_tag_name("Track"))
        .map(|node| map_song(node, p))
        .collect();
    let playlist_node = meta_doc
        .descendants()
        .find(|node| node.has_tag_name("Playlist"))
        .ok_or_else(|| "Plex playlist not found.".to_string())?;
    let playlist_meta = map_playlist(playlist_node, p);

    Ok(PlaylistDetail {
        playlist: PlaylistMeta {
            id: playlist_meta.id,
            name: playlist_meta.name,
            song_count: if playlist_meta.song_count > 0.0 {
                playlist_meta.song_count
            } else {
                songs.len() as f64
            },
            duration: if playlist_meta.duration > 0.0 {
                playlist_meta.duration
            } else {
                songs.iter().map(|song| song.duration).sum()
            },
            cover_art_url: playlist_meta.cover_art_url,
        },
        songs,
    })
}

pub(crate) async fn artist_albums(
    http: &reqwest::Client,
    p: &ActiveProfile,
    query: &str,
    count: u32,
) -> Result<Vec<Album>, String> {
    let section = music_section_key(http, p).await?;
    let limit = count.to_string();
    let xml = get_xml(
        http,
        p,
        &format!("/library/sections/{section}/search"),
        &[("query", query), ("limit", &limit)],
    )
    .await?;
    let doc = parse_doc(&xml)?;
    Ok(doc
        .descendants()
        .filter(|node| node.has_tag_name("Directory") && node.attribute("type") == Some("album"))
        .take(count as usize)
        .map(|node| map_album(node, p))
        .collect())
}

pub(crate) async fn album_songs(
    http: &reqwest::Client,
    p: &ActiveProfile,
    id: &str,
) -> Result<Vec<Song>, String> {
    let xml = get_xml(http, p, &format!("/library/metadata/{id}/children"), &[]).await?;
    let doc = parse_doc(&xml)?;
    Ok(doc
        .descendants()
        .filter(|node| node.has_tag_name("Track"))
        .map(|node| map_song(node, p))
        .collect())
}

pub(crate) async fn album(
    http: &reqwest::Client,
    p: &ActiveProfile,
    id: &str,
) -> Result<AlbumDetail, String> {
    let album_path = format!("/library/metadata/{id}");
    let album_children_path = format!("/library/metadata/{id}/children");
    let (meta_xml, songs_xml) = tokio::join!(
        get_xml(http, p, &album_path, &[]),
        get_xml(http, p, &album_children_path, &[]),
    );
    let meta_xml = meta_xml?;
    let songs_xml = songs_xml?;
    let meta_doc = parse_doc(&meta_xml)?;
    let songs_doc = parse_doc(&songs_xml)?;
    let album_node = meta_doc
        .descendants()
        .find(|node| node.has_tag_name("Directory") && node.attribute("type") == Some("album"))
        .ok_or_else(|| "Plex album not found.".to_string())?;
    let base_album = map_album(album_node, p);
    let songs: Vec<Song> = songs_doc
        .descendants()
        .filter(|node| node.has_tag_name("Track"))
        .map(|node| map_song(node, p))
        .collect();

    Ok(AlbumDetail {
        album: AlbumFull {
            id: base_album.id,
            name: base_album.name,
            artist: base_album.artist,
            artist_id: base_album.artist_id,
            cover_art: base_album.cover_art,
            cover_art_url: base_album.cover_art_url,
            song_count: if base_album.song_count > 0.0 { base_album.song_count } else { songs.len() as f64 },
            duration: if base_album.duration > 0.0 { base_album.duration } else { songs.iter().map(|song| song.duration).sum() },
            year: base_album.year,
            genre: album_node
                .children()
                .find(|child| child.has_tag_name("Genre"))
                .and_then(|genre| genre.attribute("tag"))
                .map(ToOwned::to_owned),
        },
        songs,
    })
}

pub(crate) async fn album_list(
    http: &reqwest::Client,
    p: &ActiveProfile,
    kind: &str,
    count: u32,
) -> Result<Vec<Album>, String> {
    let section = music_section_key(http, p).await?;
    let size = count.to_string();
    let sort = match kind {
        "recent" => "lastViewedAt:desc",
        "frequent" => "viewCount:desc",
        "highest" => "userRating:desc",
        "random" => "random",
        _ => "addedAt:desc",
    };

    let xml = get_xml(
        http,
        p,
        &format!("/library/sections/{section}/all"),
        &[
            ("type", "9"),
            ("sort", sort),
            ("X-Plex-Container-Start", "0"),
            ("X-Plex-Container-Size", &size),
        ],
    )
    .await?;
    let doc = parse_doc(&xml)?;
    Ok(doc
        .descendants()
        .filter(|node| node.has_tag_name("Directory") && node.attribute("type") == Some("album"))
        .take(count as usize)
        .map(|node| map_album(node, p))
        .collect())
}

pub(crate) async fn starred(
    _http: &reqwest::Client,
    _p: &ActiveProfile,
) -> Result<Vec<Song>, String> {
    Ok(Vec::new())
}

pub(crate) async fn library_counts(
    http: &reqwest::Client,
    p: &ActiveProfile,
) -> Result<(i64, i64, i64), String> {
    let playlists = playlists(http, p).await?;
    let playlist_count = playlists.len() as i64;
    let total_playlist_songs = playlists
        .iter()
        .map(|playlist| playlist.song_count.round() as i64)
        .sum();
    Ok((playlist_count, total_playlist_songs, 0))
}
