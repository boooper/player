mod db;
mod commands;
mod lyrics;
mod playback_engine;

use std::sync::Mutex;
use tauri::Manager;

use commands::cast::CastActorHandle;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub http: reqwest::Client,
    pub cast_actor: Mutex<Option<CastActorHandle>>,
    pub playback: playback_engine::PlaybackHandle,
    pub playback_cache: playback_engine::DiskCache,
    pub artwork_cache: playback_engine::DiskCache,
    pub lyrics_cache: playback_engine::DiskCache,
}

// AppState is Send + Sync because:
//   - Mutex<rusqlite::Connection>: Connection is Send, Mutex makes it Sync
//   - reqwest::Client: Send + Sync
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let updater_pubkey = option_env!("TAURI_UPDATER_PUBKEY")
        .map(str::trim)
        .filter(|value| !value.is_empty());

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Show a message then bring the existing window to front
            use tauri_plugin_dialog::DialogExt;
            app.dialog()
                .message("Player is already running.")
                .title("Already Running")
                .blocking_show();
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .setup(move |app| {
            // Initialise SQLite database in app data directory
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let cache_dir = app.path().app_cache_dir()?;
            std::fs::create_dir_all(&cache_dir)?;
            let db_path = data_dir.join("player.db");
            let conn = db::open(&db_path).expect("failed to open database");

            app.manage(AppState {
                db: Mutex::new(conn),
                http: reqwest::Client::new(),
                cast_actor: Mutex::new(None),
                playback: playback_engine::PlaybackHandle::new()
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?,
                playback_cache: playback_engine::DiskCache::new(cache_dir.join("audio"))
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?,
                artwork_cache: playback_engine::DiskCache::new(cache_dir.join("artwork"))
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?,
                lyrics_cache: playback_engine::DiskCache::new(cache_dir.join("lyrics"))
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?,
            });

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            app.handle().plugin(tauri_plugin_opener::init())?;
            app.handle().plugin(tauri_plugin_dialog::init())?;
            app.handle().plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))?;
            app.handle().plugin(tauri_plugin_os::init())?;
            app.handle().plugin(tauri_plugin_positioner::init())?;
            app.handle().plugin(tauri_plugin_store::Builder::default().build())?;
            app.handle().plugin(tauri_plugin_drpc::init())?;
            app.handle().plugin(tauri_plugin_process::init())?;
            if let Some(pubkey) = updater_pubkey {
                app.handle().plugin(
                    tauri_plugin_updater::Builder::new()
                        .pubkey(pubkey)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // settings
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::clear_database,
            commands::settings::clear_cache,
            // profiles
            commands::profiles::get_profiles,
            commands::profiles::create_profile,
            commands::profiles::update_profile,
            commands::profiles::delete_profile,
            commands::profiles::activate_profile,
            commands::profiles::get_active_server_type,
            commands::plex::plex_begin_auth,
            commands::plex::plex_poll_auth,
            // liked artists
            commands::liked_artists::get_liked_artists,
            commands::liked_artists::save_liked_artist,
            commands::liked_artists::remove_liked_artist,
            // play history & recommendation signals
            commands::play_history::record_play,
            commands::play_history::cache_song_genre,
            commands::play_history::get_artist_affinities,
            commands::play_history::get_genre_affinities,
            commands::play_history::get_song_affinities,
            commands::play_history::get_listening_profile,
            // stats + health
            commands::stats::get_library_stats,
            commands::health::get_service_health,
            // last.fm
            commands::lastfm::lfm_begin_auth,
            commands::lastfm::lfm_complete_auth,
            commands::lastfm::lfm_disconnect,
            commands::lastfm::lfm_now_playing,
            commands::lastfm::lfm_scrobble,
            commands::lastfm::lfm_user_taste,
            commands::lastfm::lfm_status,
            // library (provider-agnostic — dispatches to Subsonic or Jellyfin)
            commands::library::library_search,
            commands::library::library_search_bundle,
            commands::library::library_similar,
            commands::library::library_playlists,
            commands::library::library_playlist,
            commands::library::library_artist_albums,
            commands::library::library_album_songs,
            commands::library::library_album,
            commands::library::library_album_list,
            commands::library::library_starred,
            commands::library::library_star,
            commands::library::library_add_to_playlist,
            commands::library::library_create_playlist,
            commands::library::library_rename_playlist,
            commands::library::library_delete_playlist,
            commands::library::library_materialize_song,
            // lyrics
            commands::lyrics::fetch_lyrics,
            // chromecast
            commands::cast::cast_discover,
            commands::cast::cast_connect,
            commands::cast::cast_play,
            commands::cast::cast_pause,
            commands::cast::cast_resume,
            commands::cast::cast_stop,
            commands::cast::cast_get_session,
            commands::cast::cast_set_volume,
            commands::cast::cast_seek,
            commands::cast::cast_get_status,
            // desktop playback
            commands::playback::playback_load,
            commands::playback::playback_preload,
            commands::playback::playback_play,
            commands::playback::playback_pause,
            commands::playback::playback_stop,
            commands::playback::playback_seek,
            commands::playback::playback_set_volume,
            commands::playback::playback_set_eq,
            commands::playback::playback_status,
            commands::playback::playback_is_cached,
            commands::playback::playback_cached_ids,
            // external metadata & recommendation providers
            commands::providers::audiodb_artist,
            commands::providers::lfm_chart_top_artists,
            commands::providers::lfm_artist_search,
            commands::providers::lfm_chart_top_tracks,
            commands::providers::lfm_track_search,
            commands::providers::lfm_artist_top_tracks,
            commands::providers::lfm_tag_top_tags,
            commands::providers::lfm_track_top_tags,
            commands::providers::lfm_artist_info,
            commands::providers::lfm_similar_tracks,
            commands::providers::lfm_tag_top_tracks,
            commands::providers::lbz_cf_recommendations,
            commands::providers::lbz_recording_metadata,
            commands::providers::lbz_now_playing,
            commands::providers::lbz_scrobble,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

