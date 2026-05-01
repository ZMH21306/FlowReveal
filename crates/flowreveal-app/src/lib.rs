mod commands;
mod state;
mod events;

use state::AppState;
use tauri::Manager;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let app_state = AppState::new();
    let cleanup_proxy = app_state.proxy_was_set.clone();
    let cleanup_cert = app_state.cert_was_installed.clone();
    let cleanup_original = app_state.original_proxy_settings.clone();

    let cleanup_on_exit = move || {
        if *cleanup_proxy.blocking_read() {
            let original = cleanup_original.blocking_lock();
            if let Some(ref orig) = *original {
                match engine_core::platform_integration::windows::restore_system_proxy(orig) {
                    Ok(()) => tracing::info!("Cleanup: System proxy restored on exit"),
                    Err(e) => tracing::warn!("Cleanup: Failed to restore system proxy on exit: {}", e),
                }
            } else {
                match engine_core::platform_integration::windows::clear_system_proxy() {
                    Ok(()) => tracing::info!("Cleanup: System proxy cleared on exit"),
                    Err(e) => tracing::warn!("Cleanup: Failed to clear system proxy on exit: {}", e),
                }
            }
        }

        if *cleanup_cert.blocking_read() {
            match engine_core::platform_integration::windows::uninstall_ca_certificate() {
                Ok(()) => tracing::info!("Cleanup: CA certificate removed on exit"),
                Err(e) => tracing::warn!("Cleanup: Failed to remove CA certificate on exit: {}", e),
            }
        }
    };

    ctrlc::set_handler(move || {
        tracing::info!("Received Ctrl+C, cleaning up...");
        cleanup_on_exit();
        std::process::exit(0);
    }).ok();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::capture::start_capture,
            commands::capture::stop_capture,
            commands::traffic::get_requests,
            commands::traffic::get_sessions,
            commands::cert::install_cert,
            commands::cert::uninstall_cert,
            commands::cert::get_ca_cert_pem,
            commands::cert::get_ca_info,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                let state = window.state::<AppState>();
                let proxy_was_set = *state.proxy_was_set.blocking_read();
                let cert_was_installed = *state.cert_was_installed.blocking_read();

                if cert_was_installed {
                    match engine_core::platform_integration::windows::uninstall_ca_certificate() {
                        Ok(()) => tracing::info!("Cleanup: CA certificate removed on window close"),
                        Err(e) => tracing::warn!("Cleanup: Failed to remove CA certificate: {}", e),
                    }
                }

                if proxy_was_set {
                    let original = state.original_proxy_settings.blocking_lock();
                    if let Some(ref orig) = *original {
                        match engine_core::platform_integration::windows::restore_system_proxy(orig) {
                            Ok(()) => tracing::info!("Cleanup: System proxy restored on window close"),
                            Err(e) => tracing::warn!("Cleanup: Failed to restore system proxy: {}", e),
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running FlowReveal");
}
