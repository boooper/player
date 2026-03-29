use rand::{distr::Alphanumeric, Rng};
use serde_json::Value;

use crate::commands::media::{Album, AlbumDetail, AlbumFull, Playlist, PlaylistDetail, PlaylistMeta, Song};
use crate::commands::profiles::ActiveProfile;

const CLIENT_NAME: &str = "madrify";
const API_VERSION: &str = "1.16.1";

fn decode_legacy_password(password: &str) -> String {
    let Some(hex) = password.strip_prefix("enc:") else {
        return password.to_string();
    };

    if hex.len() % 2 != 0 {
        return hex.to_string();
    }

    let bytes: Option<Vec<u8>> = hex
        .as_bytes()
        .chunks(2)
        .map(|chunk| {
            let pair = std::str::from_utf8(chunk).ok()?;
            u8::from_str_radix(pair, 16).ok()
        })
        .collect();

    bytes
        .and_then(|value| String::from_utf8(value).ok())
        .unwrap_or_else(|| hex.to_string())
}

enum SubsonicAuth {
    Plain(String),
    Token(String),
}

struct SubsonicClient {
    base_url: String,
    username: String,
    auth: SubsonicAuth,
}

impl SubsonicClient {
    fn new(p: &ActiveProfile) -> Self {
        let auth = if p.server_type == "subsonic_legacy" || p.password.starts_with("enc:") {
            SubsonicAuth::Plain(decode_legacy_password(&p.password))
        } else {
            SubsonicAuth::Token(p.password.clone())
        };
        Self {
            base_url: p.url.trim_end_matches('/').to_string(),
            username: p.username.clone(),
            auth,
        }
    }

    fn auth_params(&self) -> Vec<(String, String)> {
        let mut params = vec![
            ("v".to_string(), API_VERSION.to_string()),
            ("c".to_string(), CLIENT_NAME.to_string()),
            ("f".to_string(), "json".to_string()),
            ("u".to_string(), self.username.clone()),
        ];
        match &self.auth {
            SubsonicAuth::Plain(password) => {
                params.push(("p".to_string(), password.clone()));
            }
            SubsonicAuth::Token(password) => {
                let salt: String = rand::rng().sample_iter(Alphanumeric).take(16).map(char::from).collect();
                let token = format!("{:x}", md5::compute(format!("{}{}", password, salt)));
                params.push(("t".to_string(), token));
                params.push(("s".to_string(), salt));
            }
        }
        params
    }

    fn build_url(&self, endpoint: &str, params: &[(&str, &str)]) -> String {
        let base = format!("{}/rest/{}", self.base_url, endpoint);
        let Ok(mut url) = url::Url::parse(&base) else {
            return base;
        };
        {
            let mut q = url.query_pairs_mut();
            for (k, v) in self.auth_params() {
                q.append_pair(&k, &v);
            }
            for (k, v) in params {
                q.append_pair(k, v);
            }
        }
        url.to_string()
    }

    fn cover_art_url(&self, id: &str, size: i32) -> String {
        if id.is_empty() {
            return String::new();
        }
        let size_str = size.to_string();
        self.build_url("getCoverArt", &[("id", id), ("size", &size_str)])
    }

    fn stream_url(&self, id: &str) -> String {
        if id.is_empty() {
            return String::new();
        }
        self.build_url("stream", &[("id", id)])
    }

    async fn call(&self, http: &reqwest::Client, endpoint: &str, params: &[(&str, &str)]) -> Result<Value, String> {
        let url = self.build_url(endpoint, params);
        self.fetch(http, &url).await
    }

    async fn fetch(&self, http: &reqwest::Client, url: &str) -> Result<Value, String> {
        let resp = http
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<Value>()
            .await
            .map_err(|e| e.to_string())?;

        let inner = resp
            .get("subsonic-response")
            .ok_or_else(|| "Invalid response: missing subsonic-response".to_string())?
            .clone();

        if inner.get("status").and_then(Value::as_str) != Some("ok") {
            let msg = inner
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("Unknown subsonic error")
                .to_string();
            return Err(msg);
        }

        Ok(inner)
    }
}

fn get_str(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn get_opt_str(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

fn get_f64(v: &Value, key: &str) -> f64 {
    v.get(key).and_then(Value::as_f64).unwrap_or_default()
}

fn child_format(v: &Value) -> Option<String> {
    get_opt_str(v, "transcodedSuffix")
        .or_else(|| get_opt_str(v, "suffix"))
        .or_else(|| {
            get_opt_str(v, "contentType").and_then(|ct| {
                ct.rsplit('/')
                    .next()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
            })
        })
}

fn map_song(client: &SubsonicClient, v: &Value) -> Song {
    let id = get_str(v, "id");
    let album_id = get_opt_str(v, "albumId")
        .or_else(|| get_opt_str(v, "parent"))
        .unwrap_or_default();
    let cover_art = get_opt_str(v, "coverArt")
        .unwrap_or_else(|| if album_id.is_empty() { id.clone() } else { album_id.clone() });

    let cover_art_url = client.cover_art_url(&cover_art, 240);
    let stream_url = client.stream_url(&id);

    Song {
        id,
        title: get_str(v, "title"),
        artist: get_opt_str(v, "artist")
            .or_else(|| get_opt_str(v, "displayArtist"))
            .unwrap_or_default(),
        album: get_opt_str(v, "album").unwrap_or_default(),
        album_id,
        cover_art,
        cover_art_url,
        stream_url,
        duration: get_f64(v, "duration"),
        audio_format: child_format(v),
        bitrate_kbps: v
            .get("bitRate")
            .and_then(Value::as_u64)
            .and_then(|b| u32::try_from(b).ok())
            .filter(|&b| b > 0),
    }
}

fn map_album(client: &SubsonicClient, v: &Value, art_size: i32) -> Album {
    let id = get_str(v, "id");
    let cover_art = get_opt_str(v, "coverArt").unwrap_or_else(|| id.clone());
    Album {
        id,
        name: get_str(v, "name"),
        artist: get_opt_str(v, "artist")
            .or_else(|| get_opt_str(v, "displayArtist"))
            .unwrap_or_default(),
        artist_id: get_opt_str(v, "artistId").unwrap_or_default(),
        cover_art: cover_art.clone(),
        cover_art_url: client.cover_art_url(&cover_art, art_size),
        song_count: get_f64(v, "songCount"),
        duration: get_f64(v, "duration"),
        year: v.get("year").and_then(Value::as_f64),
    }
}

fn map_playlist(client: &SubsonicClient, v: &Value) -> Playlist {
    let id = get_str(v, "id");
    let cover_art = get_opt_str(v, "coverArt").unwrap_or_else(|| id.clone());
    Playlist {
        id,
        name: get_str(v, "name"),
        song_count: get_f64(v, "songCount"),
        duration: get_f64(v, "duration"),
        cover_art: cover_art.clone(),
        cover_art_url: client.cover_art_url(&cover_art, 240),
    }
}

fn map_playlist_detail(client: &SubsonicClient, v: &Value) -> PlaylistDetail {
    let id = get_str(v, "id");
    let cover_art = get_opt_str(v, "coverArt").unwrap_or_else(|| id.clone());
    let songs: Vec<Song> = v
        .get("entry")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().map(|s| map_song(client, s)).collect())
        .unwrap_or_default();

    let song_count = v.get("songCount").and_then(Value::as_f64).unwrap_or(songs.len() as f64);
    let duration = v
        .get("duration")
        .and_then(Value::as_f64)
        .unwrap_or_else(|| songs.iter().map(|s| s.duration).sum());

    PlaylistDetail {
        playlist: PlaylistMeta {
            id,
            name: get_str(v, "name"),
            song_count,
            duration,
            cover_art_url: client.cover_art_url(&cover_art, 240),
        },
        songs,
    }
}

fn map_album_detail(client: &SubsonicClient, v: &Value) -> AlbumDetail {
    let id = get_str(v, "id");
    let cover_art = get_opt_str(v, "coverArt").unwrap_or_else(|| id.clone());
    let album_name = get_str(v, "name");
    let cover_art_url = client.cover_art_url(&cover_art, 400);

    let songs: Vec<Song> = v
        .get("song")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|child| {
                    let mut song = map_song(client, child);
                    if song.album.is_empty() {
                        song.album = album_name.clone();
                    }
                    if song.album_id.is_empty() {
                        song.album_id = id.clone();
                    }
                    if song.cover_art.is_empty() {
                        song.cover_art = cover_art.clone();
                        song.cover_art_url = client.cover_art_url(&cover_art, 240);
                    }
                    song
                })
                .collect()
        })
        .unwrap_or_default();

    AlbumDetail {
        album: AlbumFull {
            id: id.clone(),
            name: album_name,
            artist: get_opt_str(v, "artist")
                .or_else(|| get_opt_str(v, "displayArtist"))
                .unwrap_or_default(),
            artist_id: get_opt_str(v, "artistId").unwrap_or_default(),
            cover_art: cover_art.clone(),
            cover_art_url,
            song_count: v.get("songCount").and_then(Value::as_f64).unwrap_or(songs.len() as f64),
            duration: v
                .get("duration")
                .and_then(Value::as_f64)
                .unwrap_or_else(|| songs.iter().map(|s| s.duration).sum()),
            year: v.get("year").and_then(Value::as_f64),
            genre: get_opt_str(v, "genre"),
        },
        songs,
    }
}

pub(crate) async fn ping(http: &reqwest::Client, p: &ActiveProfile) -> Result<bool, String> {
    SubsonicClient::new(p).call(http, "ping", &[]).await.map(|_| true)
}

pub(crate) async fn search(
    http: &reqwest::Client,
    p: &ActiveProfile,
    query: &str,
    count: u32,
) -> Result<Vec<Song>, String> {
    let client = SubsonicClient::new(p);
    let count_str = count.to_string();
    let resp = client
        .call(http, "search3", &[("query", query), ("artistCount", "0"), ("albumCount", "0"), ("songCount", &count_str)])
        .await?;
    Ok(resp
        .get("searchResult3")
        .and_then(|r| r.get("song"))
        .and_then(Value::as_array)
        .map(|arr| arr.iter().map(|s| map_song(&client, s)).collect())
        .unwrap_or_default())
}

pub(crate) async fn similar(
    http: &reqwest::Client,
    p: &ActiveProfile,
    song_id: &str,
    count: u32,
) -> Result<Vec<Song>, String> {
    let client = SubsonicClient::new(p);
    let count_str = count.to_string();
    let resp = client.call(http, "getSimilarSongs2", &[("id", song_id), ("count", &count_str)]).await?;
    Ok(resp
        .get("similarSongs2")
        .and_then(|r| r.get("song"))
        .and_then(Value::as_array)
        .map(|arr| arr.iter().map(|s| map_song(&client, s)).collect())
        .unwrap_or_default())
}

pub(crate) async fn playlists(http: &reqwest::Client, p: &ActiveProfile) -> Result<Vec<Playlist>, String> {
    let client = SubsonicClient::new(p);
    let resp = client.call(http, "getPlaylists", &[]).await?;
    Ok(resp
        .get("playlists")
        .and_then(|r| r.get("playlist"))
        .and_then(Value::as_array)
        .map(|arr| arr.iter().map(|pl| map_playlist(&client, pl)).collect())
        .unwrap_or_default())
}

pub(crate) async fn playlist(
    http: &reqwest::Client,
    p: &ActiveProfile,
    id: &str,
) -> Result<PlaylistDetail, String> {
    let client = SubsonicClient::new(p);
    let resp = client.call(http, "getPlaylist", &[("id", id)]).await?;
    let pl = resp.get("playlist").ok_or_else(|| "Missing playlist in response".to_string())?;
    Ok(map_playlist_detail(&client, pl))
}

pub(crate) async fn artist_albums(
    http: &reqwest::Client,
    p: &ActiveProfile,
    query: &str,
    count: u32,
) -> Result<Vec<Album>, String> {
    let client = SubsonicClient::new(p);
    let count_str = count.to_string();
    let resp = client
        .call(http, "search3", &[("query", query), ("artistCount", "0"), ("albumCount", &count_str), ("songCount", "0")])
        .await?;
    Ok(resp
        .get("searchResult3")
        .and_then(|r| r.get("album"))
        .and_then(Value::as_array)
        .map(|arr| arr.iter().map(|a| map_album(&client, a, 300)).collect())
        .unwrap_or_default())
}

pub(crate) async fn album_songs(
    http: &reqwest::Client,
    p: &ActiveProfile,
    id: &str,
) -> Result<Vec<Song>, String> {
    let client = SubsonicClient::new(p);
    let resp = client.call(http, "getAlbum", &[("id", id)]).await?;
    let album = resp.get("album").ok_or_else(|| "Missing album in response".to_string())?;
    let album_id = get_str(album, "id");
    Ok(album
        .get("song")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|child| {
                    let mut song = map_song(&client, child);
                    if song.album_id.is_empty() {
                        song.album_id = album_id.clone();
                    }
                    song
                })
                .collect()
        })
        .unwrap_or_default())
}

pub(crate) async fn album(
    http: &reqwest::Client,
    p: &ActiveProfile,
    id: &str,
) -> Result<AlbumDetail, String> {
    let client = SubsonicClient::new(p);
    let resp = client.call(http, "getAlbum", &[("id", id)]).await?;
    let album = resp.get("album").ok_or_else(|| "Missing album in response".to_string())?;
    Ok(map_album_detail(&client, album))
}

pub(crate) async fn album_list(
    http: &reqwest::Client,
    p: &ActiveProfile,
    kind: &str,
    count: u32,
) -> Result<Vec<Album>, String> {
    let client = SubsonicClient::new(p);
    let count_str = count.to_string();
    let resp = client.call(http, "getAlbumList2", &[("type", kind), ("size", &count_str)]).await?;
    Ok(resp
        .get("albumList2")
        .and_then(|r| r.get("album"))
        .and_then(Value::as_array)
        .map(|arr| arr.iter().map(|a| map_album(&client, a, 240)).collect())
        .unwrap_or_default())
}

pub(crate) async fn starred(http: &reqwest::Client, p: &ActiveProfile) -> Result<Vec<Song>, String> {
    let client = SubsonicClient::new(p);
    let resp = client.call(http, "getStarred2", &[]).await?;
    Ok(resp
        .get("starred2")
        .and_then(|r| r.get("song"))
        .and_then(Value::as_array)
        .map(|arr| arr.iter().map(|s| map_song(&client, s)).collect())
        .unwrap_or_default())
}

pub(crate) async fn star(
    http: &reqwest::Client,
    p: &ActiveProfile,
    id: &str,
    unstar: bool,
) -> Result<(), String> {
    let client = SubsonicClient::new(p);
    let endpoint = if unstar { "unstar" } else { "star" };
    client.call(http, endpoint, &[("id", id)]).await.map(|_| ())
}

pub(crate) async fn add_to_playlist(
    http: &reqwest::Client,
    p: &ActiveProfile,
    playlist_id: &str,
    song_id: &str,
) -> Result<(), String> {
    let client = SubsonicClient::new(p);
    client
        .call(http, "updatePlaylist", &[("playlistId", playlist_id), ("songIdToAdd", song_id)])
        .await
        .map(|_| ())
}

pub(crate) async fn create_playlist(
    http: &reqwest::Client,
    p: &ActiveProfile,
    name: &str,
    song_ids: &[String],
) -> Result<Playlist, String> {
    let client = SubsonicClient::new(p);

    // Build URL manually to support multiple songId params
    let base = format!("{}/rest/createPlaylist", client.base_url);
    let Ok(mut url) = url::Url::parse(&base) else {
        return Err(format!("Invalid base URL: {}", client.base_url));
    };
    {
        let mut q = url.query_pairs_mut();
        for (k, v) in client.auth_params() {
            q.append_pair(&k, &v);
        }
        q.append_pair("name", name);
        for id in song_ids {
            q.append_pair("songId", id);
        }
    }

    let resp = client.fetch(http, url.as_str()).await?;
    let pl = resp.get("playlist").ok_or_else(|| "Missing playlist in response".to_string())?;
    Ok(map_playlist_detail(&client, pl).playlist.into())
}

pub(crate) async fn rename_playlist(
    http: &reqwest::Client,
    p: &ActiveProfile,
    playlist_id: &str,
    name: &str,
) -> Result<(), String> {
    let client = SubsonicClient::new(p);
    client
        .call(http, "updatePlaylist", &[("playlistId", playlist_id), ("name", name)])
        .await
        .map(|_| ())
}

pub(crate) async fn delete_playlist(
    http: &reqwest::Client,
    p: &ActiveProfile,
    playlist_id: &str,
) -> Result<(), String> {
    let client = SubsonicClient::new(p);
    client.call(http, "deletePlaylist", &[("id", playlist_id)]).await.map(|_| ())
}

pub(crate) async fn materialize_song(
    http: &reqwest::Client,
    p: &ActiveProfile,
    song_id: &str,
) -> Result<(), String> {
    let url = SubsonicClient::new(p).build_url("stream", &[("id", song_id), ("maxBitRate", "320")]);
    http.get(&url).send().await.map(|_| ()).map_err(|e| e.to_string())
}

pub(crate) async fn library_counts(
    http: &reqwest::Client,
    p: &ActiveProfile,
) -> Result<(i64, i64, i64), String> {
    let client = SubsonicClient::new(p);

    let pl_resp = client.call(http, "getPlaylists", &[]).await?;
    let starred_resp = client.call(http, "getStarred2", &[]).await?;

    let playlists = pl_resp
        .get("playlists")
        .and_then(|r| r.get("playlist"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let playlist_count = playlists.len() as i64;
    let total_playlist_songs: i64 = playlists
        .iter()
        .map(|pl| pl.get("songCount").and_then(Value::as_i64).unwrap_or_default())
        .sum();
    let starred_songs = starred_resp
        .get("starred2")
        .and_then(|r| r.get("song"))
        .and_then(Value::as_array)
        .map(|arr| arr.len() as i64)
        .unwrap_or_default();

    Ok((playlist_count, total_playlist_songs, starred_songs))
}

impl From<PlaylistMeta> for Playlist {
    fn from(value: PlaylistMeta) -> Self {
        Self {
            id: value.id,
            name: value.name,
            song_count: value.song_count,
            duration: value.duration,
            cover_art: String::new(),
            cover_art_url: value.cover_art_url,
        }
    }
}
