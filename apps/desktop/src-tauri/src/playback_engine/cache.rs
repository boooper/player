use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use base64::Engine;
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use url::Url;

const DEFAULT_CACHE_LIMIT_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Clone)]
pub struct DiskCache {
    root: PathBuf,
    limit_bytes: u64,
}

#[derive(Serialize, Deserialize)]
struct CacheMetadata {
    cache_key: String,
    media_kind: String,
    source_url: String,
    content_type: String,
    #[serde(default)]
    file_extension: String,
    size_bytes: u64,
    cached_at_unix_ms: u128,
    last_accessed_unix_ms: u128,
}

struct CacheEntry {
    meta_path: PathBuf,
    data_path: PathBuf,
    metadata: CacheMetadata,
}

impl DiskCache {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        Ok(Self {
            root,
            limit_bytes: DEFAULT_CACHE_LIMIT_BYTES,
        })
    }

    pub fn read(&self, song_id: &str) -> Result<Option<(Vec<u8>, String)>, String> {
        let (_, meta_path) = self.paths_for(song_id);
        if !meta_path.is_file() {
            return Ok(None);
        }

        let mut metadata = match self.read_metadata(&meta_path) {
            Ok(metadata) => metadata,
            Err(_) => {
                let _ = fs::remove_file(&meta_path);
                return Ok(None);
            }
        };
        let data_path = self.resolve_data_path(song_id, &metadata);
        if !data_path.is_file() {
            let _ = fs::remove_file(&meta_path);
            return Ok(None);
        }

        let bytes = match fs::read(&data_path) {
            Ok(bytes) => bytes,
            Err(_) => {
                let _ = fs::remove_file(&data_path);
                let _ = fs::remove_file(&meta_path);
                return Ok(None);
            }
        };

        metadata.last_accessed_unix_ms = now_unix_ms();
        metadata.size_bytes = bytes.len() as u64;
        self.write_metadata(&meta_path, &metadata)?;

        Ok(Some((bytes, metadata.content_type)))
    }

    pub fn write(
        &self,
        cache_key: &str,
        media_kind: &str,
        source_url: &str,
        content_type: &str,
        bytes: &[u8],
    ) -> Result<(), String> {
        let extension = extension_for_content_type(content_type);
        let (_, meta_path) = self.paths_for(cache_key);
        let data_path = self.data_path_for(cache_key, &extension);
        fs::write(&data_path, bytes).map_err(|error| error.to_string())?;

        let now = now_unix_ms();
        let metadata = CacheMetadata {
            cache_key: cache_key.to_string(),
            media_kind: media_kind.to_string(),
            source_url: source_url.to_string(),
            content_type: content_type.to_string(),
            file_extension: extension,
            size_bytes: bytes.len() as u64,
            cached_at_unix_ms: now,
            last_accessed_unix_ms: now,
        };
        self.write_metadata(&meta_path, &metadata)?;
        self.evict_if_needed()
    }

    pub async fn get_or_fetch_remote(
        &self,
        http: &reqwest::Client,
        cache_key: &str,
        media_kind: &str,
        source_url: &str,
    ) -> Result<String, String> {
        if let Some(url) = self.local_file_url(cache_key)? {
            return Ok(url);
        }

        let response = http
            .get(source_url)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?;

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        let bytes = response.bytes().await.map_err(|error| error.to_string())?;
        self.write(cache_key, media_kind, source_url, &content_type, &bytes)?;
        self.local_file_url(cache_key)?
            .ok_or_else(|| "Cached file was written but could not be resolved".to_string())
    }

    pub fn local_file_url(&self, cache_key: &str) -> Result<Option<String>, String> {
        let (_, meta_path) = self.paths_for(cache_key);
        if !meta_path.is_file() {
            return Ok(None);
        }

        let metadata = match self.read_metadata(&meta_path) {
            Ok(metadata) => metadata,
            Err(_) => {
                let _ = fs::remove_file(&meta_path);
                return Ok(None);
            }
        };
        let data_path = self.resolve_data_path(cache_key, &metadata);
        if !data_path.is_file() {
            let _ = fs::remove_file(&meta_path);
            return Ok(None);
        }

        if metadata.media_kind == "artwork" {
            let bytes = fs::read(&data_path).map_err(|error| error.to_string())?;
            return Ok(Some(bytes_to_data_url(&metadata.content_type, &bytes)));
        }

        Url::from_file_path(&data_path)
            .map(|url| Some(url.to_string()))
            .map_err(|_| "Failed to convert cached file path into URL".to_string())
    }

    pub fn clear(&self) -> Result<(), String> {
        if self.root.is_dir() {
            for entry in fs::read_dir(&self.root).map_err(|error| error.to_string())? {
                let entry = entry.map_err(|error| error.to_string())?;
                let path = entry.path();
                if path.is_dir() {
                    fs::remove_dir_all(&path).map_err(|error| error.to_string())?;
                } else {
                    fs::remove_file(&path).map_err(|error| error.to_string())?;
                }
            }
        }

        fs::create_dir_all(&self.root).map_err(|error| error.to_string())
    }

    fn evict_if_needed(&self) -> Result<(), String> {
        let mut entries = self.entries()?;
        let mut total_bytes: u64 = entries.iter().map(|entry| entry.metadata.size_bytes).sum();
        if total_bytes <= self.limit_bytes {
            return Ok(());
        }

        entries.sort_by_key(|entry| entry.metadata.last_accessed_unix_ms);
        for entry in entries {
            if total_bytes <= self.limit_bytes {
                break;
            }

            let _ = fs::remove_file(&entry.data_path);
            let _ = fs::remove_file(&entry.meta_path);
            total_bytes = total_bytes.saturating_sub(entry.metadata.size_bytes);
        }

        Ok(())
    }

    fn entries(&self) -> Result<Vec<CacheEntry>, String> {
        let mut entries = Vec::new();
        for item in fs::read_dir(&self.root).map_err(|error| error.to_string())? {
            let item = match item {
                Ok(item) => item,
                Err(_) => continue,
            };
            let path = item.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }

            let metadata = match self.read_metadata(&path) {
                Ok(metadata) => metadata,
                Err(_) => {
                    let _ = fs::remove_file(&path);
                    continue;
                }
            };
            let data_path = self.resolve_data_path(&metadata.cache_key, &metadata);
            if !data_path.is_file() {
                let _ = fs::remove_file(&path);
                continue;
            }

            entries.push(CacheEntry {
                meta_path: path,
                data_path,
                metadata,
            });
        }

        Ok(entries)
    }

    fn read_metadata(&self, path: &Path) -> Result<CacheMetadata, String> {
        let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
        serde_json::from_str(&text).map_err(|error| error.to_string())
    }

    fn write_metadata(&self, path: &Path, metadata: &CacheMetadata) -> Result<(), String> {
        let text = serde_json::to_string(metadata).map_err(|error| error.to_string())?;
        fs::write(path, text).map_err(|error| error.to_string())
    }

    fn paths_for(&self, song_id: &str) -> (String, PathBuf) {
        let key = format!("{:x}", md5::compute(song_id.as_bytes()));
        let meta_path = self.root.join(format!("{key}.json"));
        (key, meta_path)
    }

    fn data_path_for(&self, cache_key: &str, extension: &str) -> PathBuf {
        self.root.join(format!("{cache_key}.{extension}"))
    }

    fn resolve_data_path(&self, cache_key: &str, metadata: &CacheMetadata) -> PathBuf {
        let ext = metadata.file_extension.trim();
        if !ext.is_empty() {
            let preferred = self.data_path_for(cache_key, ext);
            if preferred.is_file() {
                return preferred;
            }
        }
        self.data_path_for(cache_key, "bin")
    }
}

fn extension_for_content_type(content_type: &str) -> String {
    let normalized = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    match normalized.as_str() {
        "image/jpeg" | "image/jpg" => "jpg".to_string(),
        "image/png" => "png".to_string(),
        "image/webp" => "webp".to_string(),
        "image/gif" => "gif".to_string(),
        "image/bmp" => "bmp".to_string(),
        "image/svg+xml" => "svg".to_string(),
        "audio/mpeg" | "audio/mp3" => "mp3".to_string(),
        "audio/flac" => "flac".to_string(),
        "audio/aac" => "aac".to_string(),
        "audio/ogg" | "application/ogg" => "ogg".to_string(),
        "audio/wav" | "audio/x-wav" => "wav".to_string(),
        "audio/mp4" | "audio/aacp" => "m4a".to_string(),
        _ => "bin".to_string(),
    }
}

fn bytes_to_data_url(content_type: &str, bytes: &[u8]) -> String {
    format!(
        "data:{};base64,{}",
        if content_type.trim().is_empty() {
            "application/octet-stream"
        } else {
            content_type.trim()
        },
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
