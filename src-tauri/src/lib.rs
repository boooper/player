use std::sync::Mutex;
use tauri::Manager;

struct NodeServer(Mutex<Option<std::process::Child>>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(NodeServer(Mutex::new(None)))
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }            app.handle().plugin(tauri_plugin_opener::init())?;
            // In a production build, spawn the SvelteKit Node.js server
            // Dev mode uses the Vite dev server at http://localhost:5173 instead
            #[cfg(not(debug_assertions))]
            {
                // Store the database in the user's app data directory so it persists across updates
                let app_data_dir = app.path().app_data_dir()?;
                std::fs::create_dir_all(&app_data_dir)?;
                let db_path = app_data_dir.join("naviarr.db");
                let database_url = format!("file:{}", db_path.to_string_lossy());

                let exe_dir = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                    .unwrap_or_default();
                let server_js = exe_dir.join("build").join("index.js");

                let child = std::process::Command::new("node")
                    .arg(&server_js)
                    .current_dir(&exe_dir)
                    .env("PORT", "3000")
                    .env("DATABASE_URL", &database_url)
                    .spawn()
                    .expect("Failed to start the SvelteKit server. Is Node.js installed?");

                *app.state::<NodeServer>().0.lock().unwrap() = Some(child);

                // Give the server a moment to become ready before the window opens
                std::thread::sleep(std::time::Duration::from_millis(800));
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // Kill the Node server when the last window closes
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if let Ok(mut guard) = window.state::<NodeServer>().0.lock() {
                    if let Some(mut child) = guard.take() as Option<std::process::Child> {
                        let _ = child.kill();
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
