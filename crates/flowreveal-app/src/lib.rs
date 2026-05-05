mod commands;
mod state;

use state::AppState;
use tauri::Manager;

pub fn run() {
    let log_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("FlowReveal")
        .join("logs");

    let file_appender = tracing_appender::rolling::daily(&log_dir, "flowreveal.log");
    let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);

    let _env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new("info,engine_core::proxy=info,engine_core::platform_integration=warn")
        });

    use tracing_subscriber::prelude::*;

    let stdout_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let file_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug"));

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_filter(stdout_filter);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false)
        .with_target(true)
        .with_filter(file_filter);

    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(file_layer)
        .init();

    tracing::info!(
        path = %log_dir.display(),
        "[Init] FlowReveal 日志系统已初始化"
    );

    let app_state = AppState::new();
    let cleanup_proxy = app_state.proxy_was_set.clone();
    let cleanup_cert = app_state.cert_was_installed.clone();
    let cleanup_original = app_state.original_proxy_settings.clone();

    let cleanup_on_exit = move || {
        if *cleanup_proxy.blocking_read() {
            let original = cleanup_original.blocking_lock();
            if let Some(ref orig) = *original {
                match engine_core::platform_integration::windows::restore_system_proxy(orig) {
                    Ok(()) => tracing::info!("[Cleanup] 系统代理已恢复"),
                    Err(e) => tracing::warn!(error = %e, "[Cleanup] 系统代理恢复失败"),
                }
            } else {
                match engine_core::platform_integration::windows::clear_system_proxy() {
                    Ok(()) => tracing::info!("[Cleanup] 系统代理已清除"),
                    Err(e) => tracing::warn!(error = %e, "[Cleanup] 系统代理清除失败"),
                }
            }
        }

        if *cleanup_cert.blocking_read() {
            match engine_core::platform_integration::windows::uninstall_ca_certificate() {
                Ok(()) => tracing::info!("[Cleanup] CA 证书已移除"),
                Err(e) => tracing::warn!(error = %e, "[Cleanup] CA 证书移除失败"),
            }
        }
    };

    ctrlc::set_handler(move || {
        tracing::info!("[Cleanup] 收到 Ctrl+C，正在清理...");
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
            commands::capture::reset_session_counter,
            commands::capture::get_diverter_status,
            commands::capture::is_elevated,
            commands::capture::is_wifi_adapter,
            commands::capture::request_elevation,
            commands::traffic::get_requests,
            commands::traffic::get_sessions,
            commands::traffic::export_har,
            commands::traffic::replay_request,
            commands::cert::install_cert,
            commands::cert::uninstall_cert,
            commands::cert::get_ca_cert_pem,
            commands::cert::get_ca_info,
            commands::rules::add_rule,
            commands::rules::remove_rule,
            commands::rules::toggle_rule,
            commands::rules::update_rule,
            commands::rules::get_rules,
            commands::rules::enable_preset_rule,
            commands::rules::export_rules,
            commands::rules::import_rules,
            commands::rules::clear_rules,
            commands::analysis::search_traffic,
            commands::analysis::filter_traffic_dsl,
            commands::analysis::get_traffic_stats,
            commands::ai::ai_analyze_traffic,
            commands::ai::ai_natural_language_query,
            commands::vuln_scan::scan_vulnerabilities,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                let state = window.state::<AppState>();
                let proxy_was_set = *state.proxy_was_set.blocking_read();
                let cert_was_installed = *state.cert_was_installed.blocking_read();

                if cert_was_installed {
                    match engine_core::platform_integration::windows::uninstall_ca_certificate() {
                        Ok(()) => tracing::info!("[Cleanup] CA 证书已移除"),
                        Err(e) => tracing::warn!(error = %e, "[Cleanup] CA 证书移除失败"),
                    }
                }

                if proxy_was_set {
                    let original = state.original_proxy_settings.blocking_lock();
                    if let Some(ref orig) = *original {
                        match engine_core::platform_integration::windows::restore_system_proxy(orig) {
                            Ok(()) => tracing::info!("[Cleanup] 系统代理已恢复"),
                            Err(e) => tracing::warn!(error = %e, "[Cleanup] 系统代理恢复失败"),
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running FlowReveal");
}
