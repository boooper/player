mod db;
mod commands;

use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub http: reqwest::Client,
}

// AppState is Send + Sync because:
//   - Mutex<rusqlite::Connection>: Connection is Send, Mutex makes it Sync
//   - reqwest::Client: Send + Sync
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            // Initialise SQLite database in app data directory
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("player.db");
            let conn = db::open(&db_path).expect("failed to open database");

            app.manage(AppState {
                db: Mutex::new(conn),
                http: reqwest::Client::new(),
            });

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            app.handle().plugin(tauri_plugin_opener::init())?;
            app.handle().plugin(tauri_plugin_drpc::init())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // settings
            commands::settings::get_settings,
            commands::settings::update_settings,
            // profiles
            commands::profiles::get_profiles,
            commands::profiles::create_profile,
            commands::profiles::update_profile,
            commands::profiles::delete_profile,
            commands::profiles::activate_profile,
            // liked artists
            commands::liked_artists::get_liked_artists,
            commands::liked_artists::save_liked_artist,
            commands::liked_artists::remove_liked_artist,
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
            // subsonic
            commands::subsonic::subsonic_search,
            commands::subsonic::subsonic_similar,
            commands::subsonic::subsonic_playlists,
            commands::subsonic::subsonic_playlist,
            commands::subsonic::subsonic_artist_albums,
            commands::subsonic::subsonic_album_songs,
            commands::subsonic::subsonic_album,
            commands::subsonic::subsonic_album_list,
            commands::subsonic::subsonic_starred,
            commands::subsonic::subsonic_star,
            commands::subsonic::subsonic_add_to_playlist,
            // lyrics
            commands::lyrics::fetch_lyrics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

