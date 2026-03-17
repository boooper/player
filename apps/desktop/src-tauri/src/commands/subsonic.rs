use opensubsonic::data::{AlbumId3, AlbumWithSongsId3, Child, Playlist as SubsonicPlaylist, PlaylistWithSongs};
use opensubsonic::{Auth, Client};

use crate::commands::media::{Album, AlbumDetail, AlbumFull, Playlist, PlaylistDetail, PlaylistMeta, Song};
use crate::commands::profiles::ActiveProfile;

const CLIENT_NAME: &str = "madrify";

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

fn auth_for_profile(p: &ActiveProfile) -> Auth {
    if p.server_type == "subsonic_legacy" || p.password.starts_with("enc:") {
        Auth::plain(decode_legacy_password(&p.password))
    } else {
        Auth::token(p.password.clone())
    }
}

fn client_from_profile(http: &reqwest::Client, p: &ActiveProfile) -> Result<Client, String> {
    Client::new(&p.url, &p.username, auth_for_profile(p))
        .map(|client| client.with_client_name(CLIENT_NAME).with_http_client(http.clone()))
        .map_err(|error| error.to_string())
}

fn cover_url(client: &Client, id: &str, size: i32) -> String {
    if id.is_empty() {
        return String::new();
    }

    client
        .cover_art_url(id, Some(size))
        .map(|url| url.to_string())
        .unwrap_or_default()
}

fn download_url(client: &Client, id: &str) -> String {
    if id.is_empty() {
        return String::new();
    }

    client
        .stream_url(id, None, None)
        .map(|url| url.to_string())
        .unwrap_or_default()
}

fn child_format(child: &Child) -> Option<String> {
    child
        .transcoded_suffix
        .clone()
        .or_else(|| child.suffix.clone())
        .or_else(|| {
            child
                .content_type
                .as_deref()
                .and_then(|value| value.rsplit('/').next())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
}

fn map_song(client: &Client, child: &Child) -> Song {
    let album_id = child.album_id.clone().or_else(|| child.parent.clone()).unwrap_or_default();
    let cover_art = child
        .cover_art
        .clone()
        .unwrap_or_else(|| if album_id.is_empty() { child.id.clone() } else { album_id.clone() });

    Song {
        id: child.id.clone(),
        title: child.title.clone(),
        artist: child
            .artist
            .clone()
            .or_else(|| child.display_artist.clone())
            .unwrap_or_default(),
        album: child.album.clone().unwrap_or_default(),
        album_id,
        cover_art: cover_art.clone(),
        cover_art_url: cover_url(client, &cover_art, 240),
        stream_url: download_url(client, &child.id),
        duration: child.duration.unwrap_or_default() as f64,
        audio_format: child_format(child),
        bitrate_kbps: child.bit_rate.and_then(|value| u32::try_from(value).ok()).filter(|value| *value > 0),
    }
}

fn map_album(client: &Client, album: &AlbumId3, art_size: i32) -> Album {
    let cover_art = album.cover_art.clone().unwrap_or_else(|| album.id.clone());
    Album {
        id: album.id.clone(),
        name: album.name.clone(),
        artist: album
            .artist
            .clone()
            .or_else(|| album.display_artist.clone())
            .unwrap_or_default(),
        artist_id: album.artist_id.clone().unwrap_or_default(),
        cover_art: cover_art.clone(),
        cover_art_url: cover_url(client, &cover_art, art_size),
        song_count: album.song_count.unwrap_or_default() as f64,
        duration: album.duration.unwrap_or_default() as f64,
        year: album.year.map(f64::from),
    }
}

fn map_playlist(client: &Client, playlist: &SubsonicPlaylist) -> Playlist {
    let cover_art = playlist.cover_art.clone().unwrap_or_else(|| playlist.id.clone());
    Playlist {
        id: playlist.id.clone(),
        name: playlist.name.clone(),
        song_count: playlist.song_count.unwrap_or_default() as f64,
        duration: playlist.duration.unwrap_or_default() as f64,
        cover_art: cover_art.clone(),
        cover_art_url: cover_url(client, &cover_art, 240),
    }
}

pub(crate) async fn ping(http: &reqwest::Client, p: &ActiveProfile) -> Result<bool, String> {
    client_from_profile(http, p)?
        .ping()
        .await
        .map(|_| true)
        .map_err(|error| error.to_string())
}

pub(crate) async fn search(
    http: &reqwest::Client,
    p: &ActiveProfile,
    query: &str,
    count: u32,
) -> Result<Vec<Song>, String> {
    let client = client_from_profile(http, p)?;
    let results = client
        .search3(query, Some(0), None, Some(0), None, Some(count.min(i32::MAX as u32) as i32), None, None)
        .await
        .map_err(|error| error.to_string())?;
    Ok(results.song.iter().map(|child| map_song(&client, child)).collect())
}

pub(crate) async fn similar(
    http: &reqwest::Client,
    p: &ActiveProfile,
    song_id: &str,
    count: u32,
) -> Result<Vec<Song>, String> {
    let client = client_from_profile(http, p)?;
    let songs = client
        .get_similar_songs2(song_id, Some(count.min(i32::MAX as u32) as i32))
        .await
        .map_err(|error| error.to_string())?;
    Ok(songs.iter().map(|child| map_song(&client, child)).collect())
}

pub(crate) async fn playlists(
    http: &reqwest::Client,
    p: &ActiveProfile,
) -> Result<Vec<Playlist>, String> {
    let client = client_from_profile(http, p)?;
    let playlists = client.get_playlists(None).await.map_err(|error| error.to_string())?;
    Ok(playlists.iter().map(|playlist| map_playlist(&client, playlist)).collect())
}

pub(crate) async fn playlist(
    http: &reqwest::Client,
    p: &ActiveProfile,
    id: &str,
) -> Result<PlaylistDetail, String> {
    let client = client_from_profile(http, p)?;
    let playlist = client.get_playlist(id).await.map_err(|error| error.to_string())?;
    Ok(map_playlist_detail(&client, &playlist))
}

fn map_playlist_detail(client: &Client, playlist: &PlaylistWithSongs) -> PlaylistDetail {
    let cover_art = playlist.cover_art.clone().unwrap_or_else(|| playlist.id.clone());
    PlaylistDetail {
        playlist: PlaylistMeta {
            id: playlist.id.clone(),
            name: playlist.name.clone(),
            song_count: playlist.song_count.unwrap_or(playlist.entry.len() as i64) as f64,
            duration: playlist
                .duration
                .unwrap_or_else(|| playlist.entry.iter().map(|song| song.duration.unwrap_or_default()).sum()) as f64,
            cover_art_url: cover_url(client, &cover_art, 240),
        },
        songs: playlist.entry.iter().map(|child| map_song(client, child)).collect(),
    }
}

pub(crate) async fn artist_albums(
    http: &reqwest::Client,
    p: &ActiveProfile,
    query: &str,
    count: u32,
) -> Result<Vec<Album>, String> {
    let client = client_from_profile(http, p)?;
    let results = client
        .search3(query, Some(0), None, Some(count.min(i32::MAX as u32) as i32), None, Some(0), None, None)
        .await
        .map_err(|error| error.to_string())?;
    Ok(results.album.iter().map(|album| map_album(&client, album, 300)).collect())
}

pub(crate) async fn album_songs(
    http: &reqwest::Client,
    p: &ActiveProfile,
    id: &str,
) -> Result<Vec<Song>, String> {
    let client = client_from_profile(http, p)?;
    let album = client.get_album(id).await.map_err(|error| error.to_string())?;
    Ok(album
        .song
        .iter()
        .map(|child| {
            let mut song = map_song(&client, child);
            if song.album_id.is_empty() {
                song.album_id = album.id.clone();
            }
            song
        })
        .collect())
}

pub(crate) async fn album(
    http: &reqwest::Client,
    p: &ActiveProfile,
    id: &str,
) -> Result<AlbumDetail, String> {
    let client = client_from_profile(http, p)?;
    let album = client.get_album(id).await.map_err(|error| error.to_string())?;
    Ok(map_album_detail(&client, &album))
}

fn map_album_detail(client: &Client, album: &AlbumWithSongsId3) -> AlbumDetail {
    let cover_art = album.cover_art.clone().unwrap_or_else(|| album.id.clone());
    let album_name = album.name.clone();
    let album_id = album.id.clone();
    let cover_art_url = cover_url(client, &cover_art, 400);
    let songs: Vec<Song> = album
        .song
        .iter()
        .map(|child| {
            let mut song = map_song(client, child);
            if song.album.is_empty() {
                song.album = album_name.clone();
            }
            if song.album_id.is_empty() {
                song.album_id = album_id.clone();
            }
            if song.cover_art.is_empty() {
                song.cover_art = cover_art.clone();
                song.cover_art_url = cover_url(client, &cover_art, 240);
            }
            song
        })
        .collect();

    AlbumDetail {
        album: AlbumFull {
            id: album.id.clone(),
            name: album.name.clone(),
            artist: album
                .artist
                .clone()
                .or_else(|| album.display_artist.clone())
                .unwrap_or_default(),
            artist_id: album.artist_id.clone().unwrap_or_default(),
            cover_art: cover_art.clone(),
            cover_art_url,
            song_count: album.song_count.unwrap_or(songs.len() as i64) as f64,
            duration: album
                .duration
                .unwrap_or_else(|| songs.iter().map(|song| song.duration as i64).sum()) as f64,
            year: album.year.map(f64::from),
            genre: album.genre.clone(),
        },
        songs,
    }
}

fn album_list_type(kind: &str) -> opensubsonic::AlbumListType {
    match kind {
        "recent" => opensubsonic::AlbumListType::Recent,
        "frequent" => opensubsonic::AlbumListType::Frequent,
        "highest" => opensubsonic::AlbumListType::Highest,
        "random" => opensubsonic::AlbumListType::Random,
        _ => opensubsonic::AlbumListType::Newest,
    }
}

pub(crate) async fn album_list(
    http: &reqwest::Client,
    p: &ActiveProfile,
    kind: &str,
    count: u32,
) -> Result<Vec<Album>, String> {
    let client = client_from_profile(http, p)?;
    let albums = client
        .get_album_list2(album_list_type(kind), Some(count.min(i32::MAX as u32) as i32), None, None, None, None, None)
        .await
        .map_err(|error| error.to_string())?;
    Ok(albums.iter().map(|album| map_album(&client, album, 240)).collect())
}

pub(crate) async fn starred(
    http: &reqwest::Client,
    p: &ActiveProfile,
) -> Result<Vec<Song>, String> {
    let client = client_from_profile(http, p)?;
    let starred = client.get_starred2(None).await.map_err(|error| error.to_string())?;
    Ok(starred.song.iter().map(|child| map_song(&client, child)).collect())
}

pub(crate) async fn star(
    http: &reqwest::Client,
    p: &ActiveProfile,
    id: &str,
    unstar: bool,
) -> Result<(), String> {
    let client = client_from_profile(http, p)?;
    let ids = [id];
    if unstar {
        client.unstar(&ids, &[], &[]).await.map_err(|error| error.to_string())
    } else {
        client.star(&ids, &[], &[]).await.map_err(|error| error.to_string())
    }
}

pub(crate) async fn add_to_playlist(
    http: &reqwest::Client,
    p: &ActiveProfile,
    playlist_id: &str,
    song_id: &str,
) -> Result<(), String> {
    let client = client_from_profile(http, p)?;
    let ids = [song_id];
    client
        .update_playlist(playlist_id, None, None, None, &ids, &[])
        .await
        .map_err(|error| error.to_string())
}

pub(crate) async fn create_playlist(
    http: &reqwest::Client,
    p: &ActiveProfile,
    name: &str,
    song_ids: &[String],
) -> Result<Playlist, String> {
    let client = client_from_profile(http, p)?;
    let refs: Vec<&str> = song_ids.iter().map(String::as_str).collect();
    let playlist = client
        .create_playlist(None, Some(name), &refs)
        .await
        .map_err(|error| error.to_string())?;
    Ok(map_playlist_detail(&client, &playlist).playlist.into())
}

pub(crate) async fn rename_playlist(
    http: &reqwest::Client,
    p: &ActiveProfile,
    playlist_id: &str,
    name: &str,
) -> Result<(), String> {
    let client = client_from_profile(http, p)?;
    client
        .update_playlist(playlist_id, Some(name), None, None, &[], &[])
        .await
        .map_err(|error| error.to_string())
}

pub(crate) async fn materialize_song(
    http: &reqwest::Client,
    p: &ActiveProfile,
    song_id: &str,
) -> Result<(), String> {
    client_from_profile(http, p)?
        .stream(song_id, Some(320), None, None, None)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub(crate) async fn library_counts(
    http: &reqwest::Client,
    p: &ActiveProfile,
) -> Result<(i64, i64, i64), String> {
    let client = client_from_profile(http, p)?;
    let playlists = client.get_playlists(None).await.map_err(|error| error.to_string())?;
    let starred = client.get_starred2(None).await.map_err(|error| error.to_string())?;

    let playlist_count = playlists.len() as i64;
    let total_playlist_songs = playlists
        .iter()
        .map(|playlist| playlist.song_count.unwrap_or_default())
        .sum();
    let starred_songs = starred.song.len() as i64;

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
