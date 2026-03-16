use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::SystemTime,
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use lofty::{
    picture::PictureType,
    prelude::{Accessor, AudioFile, TaggedFileExt},
    probe::Probe,
};
use url::Url;
use walkdir::WalkDir;

use crate::commands::{
    media::{Album, AlbumDetail, AlbumFull, Playlist, PlaylistDetail, PlaylistMeta, Song},
    profiles::ActiveProfile,
};

type AlbumBuild = (Album, Vec<Song>, SystemTime, Option<String>);

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

fn supported_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_lowercase()),
        Some(ext) if matches!(ext.as_str(), "mp3" | "flac" | "m4a" | "aac" | "ogg" | "oga" | "opus" | "wav")
    )
}

fn normalize_local_root(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    #[cfg(windows)]
    {
        let normalized = trimmed.replace('/', r"\");
        if normalized.starts_with(r"\\") {
            let without_trailing = normalized.trim_end_matches('\\');
            let parts: Vec<&str> = without_trailing.split('\\').filter(|part| !part.is_empty()).collect();
            if parts.len() <= 2 {
                return if without_trailing.is_empty() { normalized } else { without_trailing.to_string() };
            }
            return without_trailing.to_string();
        }

        if normalized.len() <= 3 && normalized.as_bytes().get(1) == Some(&b':') {
            return if normalized.ends_with('\\') { normalized } else { format!(r"{}\",
                normalized.trim_end_matches('\\')) };
        }

        return normalized.trim_end_matches('\\').to_string();
    }

    #[cfg(not(windows))]
    {
        if trimmed == "/" {
            return "/".to_string();
        }
        trimmed.trim_end_matches('/').to_string()
    }
}

fn relative_parts(root: &Path, file: &Path) -> Vec<String> {
    if let Ok(relative) = file.strip_prefix(root) {
        return relative
            .components()
            .filter_map(|component| component.as_os_str().to_str().map(|value| value.to_string()))
            .collect();
    }

    let root_components: Vec<String> = root
        .components()
        .filter_map(|component| component.as_os_str().to_str().map(|value| value.to_string()))
        .collect();
    let file_components: Vec<String> = file
        .components()
        .filter_map(|component| component.as_os_str().to_str().map(|value| value.to_string()))
        .collect();

    if file_components.len() >= root_components.len()
        && file_components
            .iter()
            .zip(root_components.iter())
            .all(|(left, right)| normalize(left) == normalize(right))
    {
        return file_components[root_components.len()..].to_vec();
    }

    file_components
}

fn path_fallback(root: &Path, file: &Path) -> (String, String, String) {
    let parts = relative_parts(root, file);
    let title = file
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Unknown Track")
        .trim()
        .to_string();

    let (artist, album) = match parts.as_slice() {
        [artist_dir, _, _file_name, ..] => (artist_dir.trim().to_string(), parts[1].trim().to_string()),
        [artist_dir, _file_name] => (artist_dir.trim().to_string(), "Singles".to_string()),
        [_file_name] => ("Unknown Artist".to_string(), "Singles".to_string()),
        _ => ("Unknown Artist".to_string(), "Unknown Album".to_string()),
    };

    (artist, album, title)
}

fn album_id(artist: &str, album: &str) -> String {
    format!("local-album-{:x}", md5::compute(format!("{}::{}", normalize(artist), normalize(album))))
}

fn artist_id(artist: &str) -> String {
    format!("local-artist-{:x}", md5::compute(normalize(artist)))
}

fn song_id(path: &Path) -> String {
    format!("local-song-{:x}", md5::compute(path.to_string_lossy().as_bytes()))
}

fn file_url(path: &Path) -> String {
    Url::from_file_path(path).map(|url| url.to_string()).unwrap_or_else(|_| path.to_string_lossy().to_string())
}

fn sidecar_cover_url(file: &Path) -> Option<String> {
    let parent = file.parent()?;
    let candidates = [
        "cover.jpg",
        "cover.jpeg",
        "cover.png",
        "folder.jpg",
        "folder.jpeg",
        "folder.png",
        "front.jpg",
        "front.jpeg",
        "front.png",
        "album.jpg",
        "album.jpeg",
        "album.png",
    ];

    candidates
        .iter()
        .map(|name| parent.join(name))
        .find(|path| path.is_file())
        .map(|path| file_url(&path))
}

fn embedded_cover_data_url(tagged: &impl TaggedFileExt) -> Option<String> {
    let picture = tagged
        .primary_tag()
        .or_else(|| tagged.first_tag())
        .and_then(|tag| {
            tag.pictures()
                .iter()
                .find(|picture| picture.pic_type() == PictureType::CoverFront)
                .or_else(|| tag.pictures().first())
        })?;

    let mime = picture.mime_type()?.as_str();
    let encoded = BASE64.encode(picture.data());
    Some(format!("data:{mime};base64,{encoded}"))
}

fn song_cover_url(file: &Path, tagged: Option<&impl TaggedFileExt>) -> String {
    if let Some(tagged) = tagged {
        if let Some(url) = embedded_cover_data_url(tagged) {
            return url;
        }
    }

    sidecar_cover_url(file).unwrap_or_default()
}

fn read_song(root: &Path, file: &Path) -> Option<(Song, Option<String>, SystemTime)> {
    let (fallback_artist, fallback_album, fallback_title) = path_fallback(root, file);
    let mut title = fallback_title;
    let mut artist = fallback_artist;
    let mut album = fallback_album;
    let mut duration = 0.0;
    let mut year = None;
    let mut cover_art_url = sidecar_cover_url(file).unwrap_or_default();

    if let Ok(tagged) = Probe::open(file).and_then(|probe| probe.read()) {
        let properties = tagged.properties();
        duration = properties.duration().as_secs_f64();
        cover_art_url = song_cover_url(file, Some(&tagged));

        if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
            if let Some(value) = tag.title() {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    title = trimmed.to_string();
                }
            }
            if let Some(value) = tag.artist() {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    artist = trimmed.to_string();
                }
            }
            if let Some(value) = tag.album() {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    album = trimmed.to_string();
                }
            }
            year = tag.year().map(|value| value.to_string());
        }
    }

    let artist = if artist.is_empty() { "Unknown Artist".to_string() } else { artist };
    let album = if album.is_empty() { "Unknown Album".to_string() } else { album };
    let modified = file.metadata().and_then(|meta| meta.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
    let song = Song {
        id: song_id(file),
        title,
        artist: artist.clone(),
        album: album.clone(),
        album_id: album_id(&artist, &album),
        cover_art: String::new(),
        cover_art_url,
        stream_url: file_url(file),
        duration,
        audio_format: file
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::trim)
            .filter(|ext| !ext.is_empty())
            .map(ToOwned::to_owned),
        bitrate_kbps: None,
    };

    Some((song, year, modified))
}

fn scan_local_library(root: &Path) -> Result<(Vec<Song>, HashMap<String, AlbumBuild>), String> {
    if !root.exists() {
        return Err("Local library folder does not exist.".to_string());
    }
    if !root.is_dir() {
        return Err("Local library path must be a folder.".to_string());
    }

    let mut songs = Vec::new();
    let mut albums: HashMap<String, AlbumBuild> = HashMap::new();

    for entry in WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let path = entry.path();
        if !entry.file_type().is_file() || !supported_extension(path) {
            continue;
        }

        let Some((song, year, modified)) = read_song(root, path) else {
            continue;
        };

        let key = song.album_id.clone();
        let album_entry = albums.entry(key.clone()).or_insert_with(|| {
            (
                Album {
                    id: key.clone(),
                    name: song.album.clone(),
                    artist: song.artist.clone(),
                    artist_id: artist_id(&song.artist),
                    cover_art: String::new(),
                    cover_art_url: song.cover_art_url.clone(),
                    song_count: 0.0,
                    duration: 0.0,
                    year: year.as_deref().and_then(|value| value.parse::<f64>().ok()),
                },
                Vec::new(),
                modified,
                year.clone(),
            )
        });

        album_entry.0.song_count += 1.0;
        album_entry.0.duration += song.duration;
        if album_entry.0.year.is_none() {
            album_entry.0.year = year.as_deref().and_then(|value| value.parse::<f64>().ok());
        }
        if album_entry.0.cover_art_url.is_empty() && !song.cover_art_url.is_empty() {
            album_entry.0.cover_art_url = song.cover_art_url.clone();
        }
        if modified > album_entry.2 {
            album_entry.2 = modified;
        }
        album_entry.1.push(song.clone());
        songs.push(song);
    }

    songs.sort_by(|left, right| {
        normalize(&left.artist)
            .cmp(&normalize(&right.artist))
            .then_with(|| normalize(&left.album).cmp(&normalize(&right.album)))
            .then_with(|| normalize(&left.title).cmp(&normalize(&right.title)))
    });

    Ok((songs, albums))
}

pub fn root_path(profile: &ActiveProfile) -> PathBuf {
    let normalized_root = normalize_local_root(&profile.url);
    let raw = normalized_root.trim();

    if let Ok(url) = Url::parse(raw) {
        if url.scheme() == "file" {
            if let Ok(path) = url.to_file_path() {
                return path;
            }
        }
    }

    #[cfg(windows)]
    {
        if raw.starts_with("//") {
            return PathBuf::from(format!(r"\{}", &raw[1..]).replace('/', r"\"));
        }

        return PathBuf::from(raw.replace('/', r"\"));
    }

    #[cfg(not(windows))]
    {
        PathBuf::from(raw)
    }
}

pub async fn search(profile: &ActiveProfile, query: &str, count: u32) -> Result<Vec<Song>, String> {
    let (songs, _) = scan_local_library(&root_path(profile))?;
    let needle = normalize(query);

    let filtered = if needle.is_empty() {
        songs
    } else {
        songs
            .into_iter()
            .filter(|song| {
                normalize(&song.title).contains(&needle)
                    || normalize(&song.artist).contains(&needle)
                    || normalize(&song.album).contains(&needle)
            })
            .collect()
    };

    Ok(filtered.into_iter().take(count as usize).collect())
}

pub async fn similar(_profile: &ActiveProfile, _song_id: &str, _count: u32) -> Result<Vec<Song>, String> {
    Ok(Vec::new())
}

pub async fn playlists(_profile: &ActiveProfile) -> Result<Vec<Playlist>, String> {
    Ok(Vec::new())
}

pub async fn playlist(_profile: &ActiveProfile, _id: &str) -> Result<PlaylistDetail, String> {
    Ok(PlaylistDetail {
        playlist: PlaylistMeta {
            id: String::new(),
            name: String::new(),
            song_count: 0.0,
            duration: 0.0,
            cover_art_url: String::new(),
        },
        songs: Vec::new(),
    })
}

pub async fn artist_albums(profile: &ActiveProfile, query: &str, count: u32) -> Result<Vec<Album>, String> {
    let (_, albums_map) = scan_local_library(&root_path(profile))?;
    let needle = normalize(query);
    let mut albums: Vec<Album> = albums_map
        .into_values()
        .map(|(album, _, _, _)| album)
        .filter(|album| {
            needle.is_empty()
                || normalize(&album.name).contains(&needle)
                || normalize(&album.artist).contains(&needle)
        })
        .collect();

    albums.sort_by(|left, right| {
        normalize(&left.artist)
            .cmp(&normalize(&right.artist))
            .then_with(|| normalize(&left.name).cmp(&normalize(&right.name)))
    });

    Ok(albums.into_iter().take(count as usize).collect())
}

pub async fn album_songs(profile: &ActiveProfile, id: &str) -> Result<Vec<Song>, String> {
    let (_, albums_map) = scan_local_library(&root_path(profile))?;
    let mut songs = albums_map
        .into_iter()
        .find(|(album_id, _)| album_id == id)
        .map(|(_, (_, songs, _, _))| songs)
        .unwrap_or_default();

    songs.sort_by(|left, right| normalize(&left.title).cmp(&normalize(&right.title)));
    Ok(songs)
}

pub async fn album(profile: &ActiveProfile, id: &str) -> Result<AlbumDetail, String> {
    let (_, albums_map) = scan_local_library(&root_path(profile))?;
    let Some((album, songs, _, _)) = albums_map.into_values().find(|(album, _, _, _)| album.id == id) else {
        return Err("Album not found".to_string());
    };

    Ok(AlbumDetail {
        album: AlbumFull {
            id: album.id,
            name: album.name,
            artist: album.artist,
            artist_id: album.artist_id,
            cover_art: album.cover_art,
            cover_art_url: album.cover_art_url,
            song_count: album.song_count,
            duration: album.duration,
            year: album.year,
            genre: None,
        },
        songs,
    })
}

pub async fn album_list(profile: &ActiveProfile, kind: &str, count: u32) -> Result<Vec<Album>, String> {
    let (_, albums_map) = scan_local_library(&root_path(profile))?;
    let mut albums: Vec<AlbumBuild> = albums_map.into_values().collect();

    match kind {
        "random" => {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            albums.shuffle(&mut rng);
        }
        _ => {
            albums.sort_by(|left, right| right.2.cmp(&left.2));
        }
    }

    Ok(albums.into_iter().map(|(album, _, _, _)| album).take(count as usize).collect())
}

pub async fn starred(_profile: &ActiveProfile) -> Result<Vec<Song>, String> {
    Ok(Vec::new())
}

pub async fn ping(profile: &ActiveProfile) -> Result<bool, String> {
    let root = root_path(profile);
    Ok(root.exists() && root.is_dir())
}
