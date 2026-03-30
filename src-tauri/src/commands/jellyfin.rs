//! Jellyfin / Emby native REST API implementation.
//!
//! Authentication: `X-Emby-Authorization` header with the API key stored in `profile.password`.
//! The `username` field is the display name; user ID is fetched from `/Users/Me` when needed.

use serde_json::Value;
use crate::commands::profiles::ActiveProfile;
use crate::commands::media::{AlbumDetail, AlbumFull, Album, Playlist, PlaylistDetail, PlaylistMeta, Song};

// ── Auth / URL helpers ────────────────────────────────────────────────────────

fn auth_header(p: &ActiveProfile) -> String {
    format!(
        "MediaBrowser Client=\"madrify\", Device=\"Desktop\", DeviceId=\"madrify-desktop\", Version=\"{}\", Token=\"{}\"",
        env!("CARGO_PKG_VERSION"),
        p.password
    )
}

fn endpoint(p: &ActiveProfile, path: &str) -> String {
    format!("{}/{}", p.url.trim_end_matches('/'), path.trim_start_matches('/'))
}

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
    format!(
        "{}/Items/{}/Download?api_key={}",
        p.url.trim_end_matches('/'),
        id,
        p.password
    )
}

// ── HTTP helpers ──────────────────────────────────────────────────────────────

async fn check_json(resp: reqwest::Response) -> Result<Value, String> {
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Jellyfin error {status}: {body}"));
    }
    resp.json::<Value>().await.map_err(|e| e.to_string())
}

async fn check_unit(resp: reqwest::Response) -> Result<(), String> {
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Jellyfin error {status}: {body}"));
    }
    Ok(())
}

pub(crate) async fn get(
    http: &reqwest::Client,
    p: &ActiveProfile,
    path: &str,
    params: &[(&str, &str)],
) -> Result<Value, String> {
    let resp = http
        .get(endpoint(p, path))
        .header("Authorization", auth_header(p))
        .query(params)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    check_json(resp).await
}

async fn post_empty(http: &reqwest::Client, p: &ActiveProfile, path: &str) -> Result<(), String> {
    let resp = http
        .post(endpoint(p, path))
        .header("Authorization", auth_header(p))
        .header("Content-Length", "0")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    check_unit(resp).await
}

async fn post_json(http: &reqwest::Client, p: &ActiveProfile, path: &str, body: &Value) -> Result<Value, String> {
    let resp = http
        .post(endpoint(p, path))
        .header("Authorization", auth_header(p))
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    check_json(resp).await
}

async fn delete(http: &reqwest::Client, p: &ActiveProfile, path: &str) -> Result<(), String> {
    let resp = http
        .delete(endpoint(p, path))
        .header("Authorization", auth_header(p))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    check_unit(resp).await
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

fn j_audio_format(v: &Value) -> Option<String> {
    v.get("Container")
        .and_then(|value| value.as_str())
        .or_else(|| {
            v.get("MediaSources")
                .and_then(|value| value.as_array())
                .and_then(|sources| sources.first())
                .and_then(|source| source.get("Container"))
                .and_then(|value| value.as_str())
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn j_bitrate_kbps(v: &Value) -> Option<u32> {
    let bits_per_second = v
        .get("Bitrate")
        .and_then(|value| value.as_u64())
        .or_else(|| {
            v.get("MediaSources")
                .and_then(|value| value.as_array())
                .and_then(|sources| sources.first())
                .and_then(|source| source.get("Bitrate"))
                .and_then(|value| value.as_u64())
        })?;

    let kbps = bits_per_second / 1000;
    u32::try_from(kbps).ok().filter(|value| *value > 0)
}

// ── Item mappers ──────────────────────────────────────────────────────────────

pub(crate) fn map_song(v: &Value, p: &ActiveProfile) -> Song {
    let id = js(v, "Id");
    let album_id = js(v, "AlbumId");
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
        audio_format: j_audio_format(v),
        bitrate_kbps: j_bitrate_kbps(v),
        genre: v.get("Genres")
            .and_then(|g| g.as_array())
            .and_then(|a| a.first())
            .and_then(|g| g.as_str())
            .map(|s| s.to_string()),
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
    "Artists,ArtistItems,Album,AlbumId,ImageTags,RunTimeTicks,AlbumPrimaryImageTag,Container,Bitrate,MediaSources";
const ALBUM_FIELDS: &str =
    "Artists,ArtistItems,ChildCount,RunTimeTicks,ImageTags,ProductionYear";

pub(crate) async fn ping(http: &reqwest::Client, p: &ActiveProfile) -> Result<bool, String> {
    get(http, p, "/System/Ping", &[]).await.map(|_| true)
}

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
        "newest"   => ("DateCreated",     "Descending"),
        "random"   => ("Random",          "Ascending"),
        "frequent" => ("PlayCount",       "Descending"),
        "recent"   => ("DatePlayed",      "Descending"),
        "highest"  => ("CommunityRating", "Descending"),
        _          => ("DateCreated",     "Descending"),
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

pub(crate) async fn create_playlist(
    http: &reqwest::Client,
    p: &ActiveProfile,
    name: &str,
    song_ids: &[String],
) -> Result<Playlist, String> {
    if name.trim().is_empty() {
        return Err("Playlist name is required.".to_string());
    }

    let user_id = user_id(http, p).await?;
    let body = serde_json::json!({
        "Name": name,
        "Ids": song_ids,
        "UserId": user_id,
        "MediaType": "Audio"
    });
    let created = post_json(http, p, "/Playlists", &body).await?;

    Ok(Playlist {
        cover_art_url: String::new(),
        cover_art: String::new(),
        id: js(&created, "Id"),
        name: js(&created, "Name"),
        song_count: song_ids.len() as f64,
        duration: 0.0,
    })
}

pub(crate) async fn delete_playlist(
    http: &reqwest::Client,
    p: &ActiveProfile,
    playlist_id: &str,
) -> Result<(), String> {
    delete(http, p, &format!("/Items/{}", playlist_id)).await
}

pub(crate) async fn rename_playlist(
    http: &reqwest::Client,
    p: &ActiveProfile,
    playlist_id: &str,
    name: &str,
) -> Result<(), String> {
    if playlist_id.trim().is_empty() {
        return Err("Playlist id is required.".to_string());
    }
    if name.trim().is_empty() {
        return Err("Playlist name is required.".to_string());
    }

    let body = serde_json::json!({ "Name": name.trim() });
    post_json(http, p, &format!("/Playlists/{}", playlist_id), &body).await?;
    Ok(())
}

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
    let pl_items = jitems(&pl_resp.unwrap_or(Value::Null));
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
