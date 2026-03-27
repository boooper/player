use tauri::State;

use crate::AppState;

#[tauri::command]
pub async fn fetch_lyrics(
    state: State<'_, AppState>,
    artist: String,
    title: String,
    album: String,
    duration: f64,
    provider: Option<String>,
) -> Result<Option<crate::lyrics::LyricsResult>, String> {
    // Delegate the fetching logic to the `lyrics` module.
    // `State<T>` derefs to `T`, so pass a reference to the inner `AppState`.
    // Optional `provider` selection is not exposed here yet; default behavior is used.
    crate::lyrics::fetch_lyrics_inner(&*state, &artist, &title, &album, duration, provider.as_deref()).await
}
