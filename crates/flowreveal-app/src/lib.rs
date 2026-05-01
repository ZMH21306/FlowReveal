mod commands;
mod state;
mod events;

use state::AppState;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::capture::start_capture,
            commands::capture::stop_capture,
            commands::traffic::get_requests,
            commands::traffic::get_sessions,
            commands::cert::install_cert,
            commands::cert::uninstall_cert,
        ])
        .run(tauri::generate_context!())
        .expect("error while running FlowReveal");
}
