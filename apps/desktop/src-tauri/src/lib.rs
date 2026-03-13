use std::sync::Mutex;
use tauri::Manager;

/// Holds the API sidecar process handle so it can be killed on exit.
struct ApiServerProcess(Mutex<Option<tauri_plugin_shell::process::CommandChild>>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Focus the existing window when a second instance is launched
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // In production, spawn the bundled API server sidecar.
            // In dev, beforeDevCommand already starts the API via `npm run dev:web+api`.
            #[cfg(not(debug_assertions))]
            {
                use tauri_plugin_shell::ShellExt;

                let app_data_dir = app.path().app_data_dir()?;
                std::fs::create_dir_all(&app_data_dir)?;
                let db_url = format!(
                    "file:{}/player.db",
                    app_data_dir.to_string_lossy().replace('\\', "/")
                );

                let (_, child) = app
                    .shell()
                    .sidecar("api-server")?
                    .env("DATABASE_URL", db_url)
                    .env("PORT", "8787")
                    .spawn()?;

                app.manage(ApiServerProcess(Mutex::new(Some(child))));
            }

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
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if let Some(state) = window.try_state::<ApiServerProcess>() {
                    if let Ok(mut guard) = state.0.lock() {
                        if let Some(child) = guard.take() {
                            let _ = child.kill();
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

