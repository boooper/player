//! Jellyfin / Emby native REST API implementation.
//!
//! Authentication: `X-Emby-Authorization` header with the API key stored in `profile.password`.
//! The `username` field is the display name; user ID is fetched from `/Users/Me` when needed.

use url::Url;
use serde_json::Value;
use crate::commands::profiles::ActiveProfile;
use crate::commands::media::{AlbumDetail, AlbumFull, Album, Playlist, PlaylistDetail, PlaylistMeta, Song};

// ── Auth ─────────────────────────────────────────────────────────────────────

fn auth_header(p: &ActiveProfile) -> String {
    format!(
        r#"MediaBrowser Client="naviarr", Device="Desktop", DeviceId="naviarr-desktop", Version="1.0.0", Token="{}""#,
        p.password
    )
}

// ── URL helpers ───────────────────────────────────────────────────────────────

pub(crate) fn cover_url(p: &ActiveProfile, item_id: &str, size: u32) -> String {
    if item_id.is_empty() {
        return String::new();
    }
    format!(
        "{}/Items/{}/Images/Primary?fillHeight={}&fillWidth={}&quality=96",
        p.url.trim_end_matches('/'),
        item_id,
        size,
        size
    )
}

fn stream_url(p: &ActiveProfile, id: &str) -> String {
    if id.is_empty() {
        return String::new();
    }
    // Embed the API key as a query param so the browser's <audio> element can load it
    format!(
        "{}/Items/{}/Download?api_key={}",
        p.url.trim_end_matches('/'),
        id,
        p.password
    )
}

// ── HTTP helpers ──────────────────────────────────────────────────────────────

pub(crate) async fn get(
    http: &reqwest::Client,
    p: &ActiveProfile,
    path: &str,
    params: &[(&str, &str)],
) -> Result<Value, String> {
    let base = format!("{}{}", p.url.trim_end_matches('/'), path);
    let mut url = Url::parse(&base).map_err(|e| e.to_string())?;
    for (k, v) in params {
        url.query_pairs_mut().append_pair(k, v);
    }
    let resp = http
        .get(url.as_str())
        .header("X-Emby-Authorization", auth_header(p))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json::<Value>().await.map_err(|e| e.to_string())
}

async fn post_empty(http: &reqwest::Client, p: &ActiveProfile, path: &str) -> Result<(), String> {
    let url = format!("{}{}", p.url.trim_end_matches('/'), path);
    http.post(&url)
        .header("X-Emby-Authorization", auth_header(p))
        .header("Content-Length", "0")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn delete(http: &reqwest::Client, p: &ActiveProfile, path: &str) -> Result<(), String> {
    let url = format!("{}{}", p.url.trim_end_matches('/'), path);
    http.delete(&url)
        .header("X-Emby-Authorization", auth_header(p))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── User ID (needed for favourite endpoints) ──────────────────────────────────

pub(crate) async fn user_id(http: &reqwest::Client, p: &ActiveProfile) -> Result<String, String> {
    let json = get(http, p, "/Users/Me", &[]).await?;
    json.get("Id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Failed to get user ID from Jellyfin/Emby".to_string())
}

// ── JSON field helpers ────────────────────────────────────────────────────────

fn js(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn jduration(v: &Value) -> f64 {
    v.get("RunTimeTicks")
        .and_then(|t| t.as_f64())
        .unwrap_or(0.0)
        / 10_000_000.0
}

fn jitems(json: &Value) -> Vec<Value> {
    json.get("Items")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default()
}

fn jartist(v: &Value) -> String {
    v.get("Artists")
        .and_then(|a| a.as_array())
        .and_then(|a| a.first())
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string()
}

fn jartist_id(v: &Value) -> String {
    v.get("ArtistItems")
        .and_then(|a| a.as_array())
        .and_then(|a| a.first())
        .and_then(|o| o.get("Id"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string()
}

// ── Item mappers ──────────────────────────────────────────────────────────────

pub(crate) fn map_song(v: &Value, p: &ActiveProfile) -> Song {
    let id = js(v, "Id");
    let album_id = js(v, "AlbumId");
    // Prefer the song's own image; fall back to album cover
    let cover_id = if v
        .get("ImageTags")
        .and_then(|t| t.get("Primary"))
        .is_some()
    {
        id.clone()
    } else {
        album_id.clone()
    };
    Song {
        cover_art_url: cover_url(p, &cover_id, 240),
        stream_url: stream_url(p, &id),
        id: id.clone(),
        title: js(v, "Name"),
        artist: jartist(v),
        album: js(v, "Album"),
        album_id,
        cover_art: cover_id,
        duration: jduration(v),
    }
}

fn map_album(v: &Value, p: &ActiveProfile, size: u32) -> Album {
    let id = js(v, "Id");
    Album {
        cover_art_url: cover_url(p, &id, size),
        cover_art: id.clone(),
        id,
        name: js(v, "Name"),
        artist: jartist(v),
        artist_id: jartist_id(v),
        song_count: v.get("ChildCount").and_then(|n| n.as_f64()).unwrap_or(0.0),
        duration: jduration(v),
        year: v.get("ProductionYear").and_then(|y| y.as_f64()),
    }
}

fn map_playlist(v: &Value, p: &ActiveProfile) -> Playlist {
    let id = js(v, "Id");
    Playlist {
        cover_art_url: cover_url(p, &id, 240),
        cover_art: id.clone(),
        id,
        name: js(v, "Name"),
        song_count: v.get("ChildCount").and_then(|n| n.as_f64()).unwrap_or(0.0),
        duration: jduration(v),
    }
}

// ── Command implementations ───────────────────────────────────────────────────

const SONG_FIELDS: &str =
    "Artists,ArtistItems,Album,AlbumId,ImageTags,RunTimeTicks,AlbumPrimaryImageTag";
const ALBUM_FIELDS: &str =
    "Artists,ArtistItems,ChildCount,RunTimeTicks,ImageTags,ProductionYear";

pub(crate) async fn search(
    http: &reqwest::Client,
    p: &ActiveProfile,
    query: &str,
    count: u32,
) -> Result<Vec<Song>, String> {
    let cnt = count.to_string();
    let json = get(http, p, "/Items", &[
        ("searchTerm", query),
        ("IncludeItemTypes", "Audio"),
        ("Recursive", "true"),
        ("Limit", &cnt),
        ("Fields", SONG_FIELDS),
    ]).await?;
    Ok(jitems(&json).iter().map(|v| map_song(v, p)).collect())
}

pub(crate) async fn similar(
    http: &reqwest::Client,
    p: &ActiveProfile,
    song_id: &str,
    count: u32,
) -> Result<Vec<Song>, String> {
    let cnt = count.to_string();
    let json = get(http, p, &format!("/Items/{}/Similar", song_id), &[
        ("IncludeItemTypes", "Audio"),
        ("Limit", &cnt),
        ("Fields", SONG_FIELDS),
    ]).await?;
    Ok(jitems(&json).iter().map(|v| map_song(v, p)).collect())
}

pub(crate) async fn playlists(
    http: &reqwest::Client,
    p: &ActiveProfile,
) -> Result<Vec<Playlist>, String> {
    let json = get(http, p, "/Items", &[
        ("IncludeItemTypes", "Playlist"),
        ("Recursive", "true"),
        ("Fields", "ChildCount,RunTimeTicks,ImageTags"),
    ]).await?;
    Ok(jitems(&json).iter().map(|v| map_playlist(v, p)).collect())
}

pub(crate) async fn playlist(
    http: &reqwest::Client,
    p: &ActiveProfile,
    id: &str,
) -> Result<PlaylistDetail, String> {
    let pl_path = format!("/Items/{}", id);
    let pl_params: &[(&str, &str)] = &[("Fields", "ChildCount,ImageTags")];
    let items_params: &[(&str, &str)] = &[
        ("ParentId", id),
        ("IncludeItemTypes", "Audio"),
        ("Fields", SONG_FIELDS),
    ];
    let (pl_resp, items_resp) = tokio::join!(
        get(http, p, &pl_path, pl_params),
        get(http, p, "/Items", items_params)
    );
    let pl = pl_resp?;
    let songs: Vec<Song> = jitems(&items_resp?).iter().map(|v| map_song(v, p)).collect();
    let total_dur: f64 = songs.iter().map(|s| s.duration).sum();
    let pl_id = js(&pl, "Id");
    Ok(PlaylistDetail {
        songs: songs.clone(),
        playlist: PlaylistMeta {
            id: if pl_id.is_empty() { id.to_string() } else { pl_id },
            name: js(&pl, "Name"),
            song_count: songs.len() as f64,
            duration: total_dur,
            cover_art_url: cover_url(p, id, 240),
        },
    })
}

pub(crate) async fn artist_albums(
    http: &reqwest::Client,
    p: &ActiveProfile,
    query: &str,
    count: u32,
) -> Result<Vec<Album>, String> {
    let cnt = count.to_string();
    let json = get(http, p, "/Items", &[
        ("searchTerm", query),
        ("IncludeItemTypes", "MusicAlbum"),
        ("Recursive", "true"),
        ("Limit", &cnt),
        ("Fields", ALBUM_FIELDS),
    ]).await?;
    Ok(jitems(&json).iter().map(|v| map_album(v, p, 300)).collect())
}

pub(crate) async fn album_songs(
    http: &reqwest::Client,
    p: &ActiveProfile,
    id: &str,
) -> Result<Vec<Song>, String> {
    let json = get(http, p, "/Items", &[
        ("ParentId", id),
        ("IncludeItemTypes", "Audio"),
        ("SortBy", "IndexNumber"),
        ("Fields", SONG_FIELDS),
    ]).await?;
    Ok(jitems(&json)
        .iter()
        .map(|v| {
            let mut s = map_song(v, p);
            if s.album_id.is_empty() {
                s.album_id = id.to_string();
            }
            s
        })
        .collect())
}

pub(crate) async fn album(
    http: &reqwest::Client,
    p: &ActiveProfile,
    id: &str,
) -> Result<AlbumDetail, String> {
    let al_path = format!("/Items/{}", id);
    let al_params: &[(&str, &str)] = &[
        ("Fields", "Artists,ArtistItems,Genres,ChildCount,RunTimeTicks,ImageTags,ProductionYear"),
    ];
    let songs_params: &[(&str, &str)] = &[
        ("ParentId", id),
        ("IncludeItemTypes", "Audio"),
        ("SortBy", "IndexNumber"),
        ("Fields", SONG_FIELDS),
    ];
    let (al_resp, songs_resp) = tokio::join!(
        get(http, p, &al_path, al_params),
        get(http, p, "/Items", songs_params)
    );
    let al = al_resp?;
    let al_id = { let i = js(&al, "Id"); if i.is_empty() { id.to_string() } else { i } };
    let al_name = js(&al, "Name");
    let songs: Vec<Song> = jitems(&songs_resp?)
        .iter()
        .map(|v| {
            let mut s = map_song(v, p);
            if s.album.is_empty() { s.album = al_name.clone(); }
            if s.album_id.is_empty() { s.album_id = al_id.clone(); }
            s
        })
        .collect();
    let genre = al
        .get("Genres")
        .and_then(|g| g.as_array())
        .and_then(|g| g.first())
        .and_then(|s| s.as_str())
        .map(String::from);
    Ok(AlbumDetail {
        songs,
        album: AlbumFull {
            cover_art_url: cover_url(p, &al_id, 400),
            cover_art: al_id.clone(),
            id: al_id,
            name: al_name,
            artist: jartist(&al),
            artist_id: jartist_id(&al),
            song_count: al.get("ChildCount").and_then(|n| n.as_f64()).unwrap_or(0.0),
            duration: jduration(&al),
            year: al.get("ProductionYear").and_then(|y| y.as_f64()),
            genre,
        },
    })
}

pub(crate) async fn album_list(
    http: &reqwest::Client,
    p: &ActiveProfile,
    kind: &str,
    count: u32,
) -> Result<Vec<Album>, String> {
    let cnt = count.to_string();
    let (sort_by, sort_order) = match kind {
        "newest"   => ("DateCreated", "Descending"),
        "random"   => ("Random",      "Ascending"),
        "frequent" => ("PlayCount",   "Descending"),
        "recent"   => ("DatePlayed",  "Descending"),
        "highest"  => ("CommunityRating", "Descending"),
        _          => ("DateCreated", "Descending"),
    };
    let json = get(http, p, "/Items", &[
        ("IncludeItemTypes", "MusicAlbum"),
        ("Recursive", "true"),
        ("SortBy", sort_by),
        ("SortOrder", sort_order),
        ("Limit", &cnt),
        ("Fields", ALBUM_FIELDS),
    ]).await?;
    Ok(jitems(&json).iter().map(|v| map_album(v, p, 240)).collect())
}

pub(crate) async fn starred(
    http: &reqwest::Client,
    p: &ActiveProfile,
) -> Result<Vec<Song>, String> {
    let json = get(http, p, "/Items", &[
        ("Filters", "IsFavorite"),
        ("IncludeItemTypes", "Audio"),
        ("Recursive", "true"),
        ("Fields", SONG_FIELDS),
    ]).await?;
    Ok(jitems(&json).iter().map(|v| map_song(v, p)).collect())
}

pub(crate) async fn star(
    http: &reqwest::Client,
    p: &ActiveProfile,
    id: &str,
    unstar: bool,
) -> Result<(), String> {
    let uid = user_id(http, p).await?;
    let path = format!("/Users/{}/FavoriteItems/{}", uid, id);
    if unstar {
        delete(http, p, &path).await
    } else {
        post_empty(http, p, &path).await
    }
}

pub(crate) async fn add_to_playlist(
    http: &reqwest::Client,
    p: &ActiveProfile,
    playlist_id: &str,
    song_id: &str,
) -> Result<(), String> {
    post_empty(http, p, &format!("/Playlists/{}/Items?Ids={}", playlist_id, song_id)).await
}

/// Returns true if the server responds successfully to a ping.
pub(crate) async fn ping(http: &reqwest::Client, p: &ActiveProfile) -> Result<bool, String> {
    let url = format!("{}/System/Ping", p.url.trim_end_matches('/'));
    let resp = http
        .get(&url)
        .header("X-Emby-Authorization", auth_header(p))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp.status().is_success())
}

/// Playlists and starred song counts for library stats.
pub(crate) async fn library_counts(
    http: &reqwest::Client,
    p: &ActiveProfile,
) -> Result<(i64, i64, i64), String> {
    let pl_params: &[(&str, &str)] = &[
        ("IncludeItemTypes", "Playlist"),
        ("Recursive", "true"),
        ("Fields", "ChildCount"),
    ];
    let starred_params: &[(&str, &str)] = &[
        ("Filters", "IsFavorite"),
        ("IncludeItemTypes", "Audio"),
        ("Recursive", "true"),
        ("Limit", "1"),
    ];
    let (pl_resp, starred_resp) = tokio::join!(
        get(http, p, "/Items", pl_params),
        get(http, p, "/Items", starred_params)
    );
    let pl_items = jitems(&pl_resp.unwrap_or(serde_json::Value::Null));
    let playlist_count = pl_items.len() as i64;
    let total_songs: i64 = pl_items
        .iter()
        .filter_map(|pl| pl.get("ChildCount").and_then(|n| n.as_i64()))
        .sum();
    let starred = starred_resp
        .ok()
        .and_then(|j| j.get("TotalRecordCount").and_then(|n| n.as_i64()))
        .unwrap_or(0);
    Ok((playlist_count, total_songs, starred))
}
