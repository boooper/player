use tauri::{Manager, State};

use crate::AppState;

#[tauri::command]
pub async fn fetch_lyrics(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    artist: String,
    title: String,
    album: String,
    duration: f64,
    provider: Option<String>,
) -> Result<Option<crate::lyrics::LyricsResult>, String> {
    // Build cache key used by the lyrics module
    let key = crate::lyrics::cache_key(&artist, &title, &album, duration);

    // Try plugin cache first
    if let Ok(Some(value)) = app_handle.cache().get(&key) {
        if let Ok(cached) = serde_json::from_value::<crate::lyrics::LyricsResult>(value) {
            return Ok(Some(crate::lyrics::LyricsResult { cached: true, ..cached }));
        }
    }

    // Fallback to existing provider logic
    let result = crate::lyrics::fetch_lyrics_inner(&*state, &artist, &title, &album, duration, provider.as_deref()).await?;

    // Store successful results in plugin cache for faster subsequent fetches
    if let Some(ref res) = result {
        let options = tauri_plugin_cache::SetItemOptions {
            ttl: Some(60 * 60), // 1 hour default
            compress: Some(true),
            compression_method: None,
        };
        let _ = app_handle
            .cache()
            .set(key, serde_json::to_value(res).map_err(|e| e.to_string())?, Some(options));
    }

    Ok(result)
}
