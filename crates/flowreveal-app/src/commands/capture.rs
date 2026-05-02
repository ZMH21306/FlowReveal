use tauri::{command, State, AppHandle, Emitter};
use crate::state::AppState;
use engine_core::capture_config::{CaptureConfig, CaptureMode};
use engine_core::engine_stats::CaptureStatus;
use engine_core::http_message::{HttpMessage, HttpSession};
use engine_core::proxy::forward_proxy::ForwardProxy;
use engine_core::mitm::CaManager;
use engine_core::platform_integration::windows;
use tokio::sync::mpsc;

#[command]
pub async fn start_capture(
    app: AppHandle,
    state: State<'_, AppState>,
    config: CaptureConfig,
) -> Result<(), String> {
    {
        let status = state.capture_status.read().await;
        if *status == CaptureStatus::Running {
            return Err("Capture is already running".to_string());
        }
    }

    let (tx, mut rx) = mpsc::channel::<HttpMessage>(1024);

    {
        let mut event_tx = state.event_tx.lock().await;
        *event_tx = Some(tx.clone());
    }

    let port = config.proxy_port;
    let mode = config.mode;
    let capture_https = config.capture_https;
    tracing::info!("Starting capture in {:?} mode on port {} (HTTPS={})", mode, port, capture_https);

    *state.capture_status.write().await = CaptureStatus::Running;

    let (shutdown_tx, ca_manager_from_proxy) = match mode {
        CaptureMode::ForwardProxy => {
            match ForwardProxy::start(port, &config, tx).await {
                Ok(handle) => (handle.shutdown_tx, handle.ca_manager),
                Err(e) => {
                    *state.capture_status.write().await = CaptureStatus::Idle;
                    tracing::error!("Failed to start forward proxy: {}", e);
                    return Err(format!("Failed to start forward proxy: {}", e));
                }
            }
        }
        CaptureMode::TransparentProxy => {
            #[cfg(target_os = "windows")]
            {
                use engine_core::proxy::transparent_proxy::TransparentProxy;
                match TransparentProxy::start(port, &config, tx).await {
                    Ok(handle) => (handle.shutdown_tx, None),
                    Err(e) => {
                        *state.capture_status.write().await = CaptureStatus::Idle;
                        tracing::error!("Failed to start transparent proxy: {}", e);
                        return Err(format!("Failed to start transparent proxy: {}", e));
                    }
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                *state.capture_status.write().await = CaptureStatus::Idle;
                return Err("Transparent proxy is only supported on Windows".to_string());
            }
        }
        CaptureMode::ApiHook => {
            *state.capture_status.write().await = CaptureStatus::Idle;
            return Err("API Hook mode is not yet implemented".to_string());
        }
    };

    tracing::info!("Proxy started successfully in {:?} mode on port {}", mode, port);

    *state.config.write().await = Some(config);

    {
        let mut shutdown = state.shutdown_handle.lock().await;
        *shutdown = Some(shutdown_tx);
    }

    if let Some(ca) = ca_manager_from_proxy {
        let mut ca_guard = state.ca_manager.write().await;
        let pem = ca.ca_cert_pem().to_string();
        let key = ca.ca_key_pem().to_string();
        match CaManager::from_pem(&pem, &key) {
            Ok(manager) => {
                *ca_guard = Some(manager);
                tracing::info!("CA manager shared with AppState");
            }
            Err(e) => {
                tracing::warn!("Failed to clone CA manager into AppState: {}", e);
            }
        }
    }

    if mode == CaptureMode::ForwardProxy {
        let proxy_addr = format!("127.0.0.1:{}", port);
        match windows::set_system_proxy(&proxy_addr) {
            Ok(original) => {
                *state.proxy_was_set.write().await = true;
                *state.original_proxy_settings.lock().await = Some(original);
                tracing::info!("System proxy auto-configured to {}", proxy_addr);
            }
            Err(e) => {
                tracing::warn!("Failed to auto-set system proxy: {} - please configure manually", e);
            }
        }
    }

    if capture_https {
        let ca_guard = state.ca_manager.read().await;
        if let Some(ca) = ca_guard.as_ref() {
            let cert_pem = ca.ca_cert_pem().to_string();
            drop(ca_guard);

            if !windows::is_ca_certificate_installed() {
                match windows::install_ca_certificate(&cert_pem) {
                    Ok(()) => {
                        *state.cert_was_installed.write().await = true;
                        tracing::info!("CA certificate auto-installed to Trusted Root store");
                    }
                    Err(e) => {
                        tracing::warn!("Failed to auto-install CA certificate: {} - HTTPS decryption may not work in browsers", e);
                    }
                }
            } else {
                tracing::info!("CA certificate already installed in Trusted Root store");
                *state.cert_was_installed.write().await = true;
            }
        }
    }

    let sessions = state.sessions.clone();
    let stats = state.stats.clone();
    let app_handle = app.clone();

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let is_request = matches!(msg.direction, engine_core::http_message::MessageDirection::Request);

            {
                let mut sess = sessions.write().await;
                if is_request {
                    let session = HttpSession::new(msg.clone());
                    sess.push(session);
                    {
                        let mut s = stats.write().await;
                        s.increment_sessions();
                        s.add_bytes(msg.body_size);
                    }
                } else if let Some(existing) = sess.iter_mut().find(|s| s.id == msg.session_id) {
                    existing.complete(msg.clone());
                    {
                        let mut s = stats.write().await;
                        s.decrement_active();
                        s.add_bytes(msg.body_size);
                    }
                }
            }

            let _ = app_handle.emit("traffic:request", &msg);
        }
    });

    tracing::info!("Capture started in {:?} mode on port {}", mode, port);
    Ok(())
}

#[command]
pub async fn stop_capture(state: State<'_, AppState>) -> Result<(), String> {
    {
        let status = state.capture_status.read().await;
        if *status != CaptureStatus::Running {
            return Err("Capture is not running".to_string());
        }
    }

    {
        let mut shutdown = state.shutdown_handle.lock().await;
        if let Some(handle) = shutdown.take() {
            let _ = handle.send(());
            tracing::info!("Proxy shutdown signal sent");
        }
    }

    #[cfg(target_os = "windows")]
    {
        let config = state.config.read().await;
        if let Some(cfg) = config.as_ref() {
            if cfg.mode == CaptureMode::TransparentProxy {
                use engine_core::proxy::transparent_proxy::wfp_engine;
                if let Err(e) = wfp_engine::uninstall_redirect_filters() {
                    tracing::warn!("Failed to uninstall WFP filters: {}", e);
                }
            }
        }
    }

    if *state.cert_was_installed.read().await {
        match windows::uninstall_ca_certificate() {
            Ok(()) => tracing::info!("CA certificate auto-removed from Trusted Root store"),
            Err(e) => tracing::warn!("Failed to auto-remove CA certificate: {}", e),
        }
        *state.cert_was_installed.write().await = false;
    }

    if *state.proxy_was_set.read().await {
        let original = state.original_proxy_settings.lock().await;
        if let Some(ref orig) = *original {
            match windows::restore_system_proxy(orig) {
                Ok(()) => tracing::info!("System proxy auto-restored"),
                Err(e) => tracing::warn!("Failed to auto-restore system proxy: {}", e),
            }
        } else {
            match windows::clear_system_proxy() {
                Ok(()) => tracing::info!("System proxy auto-cleared"),
                Err(e) => tracing::warn!("Failed to auto-clear system proxy: {}", e),
            }
        }
        *state.proxy_was_set.write().await = false;
        drop(original);
        *state.original_proxy_settings.lock().await = None;
    }

    {
        let mut event_tx = state.event_tx.lock().await;
        *event_tx = None;
    }

    *state.capture_status.write().await = CaptureStatus::Idle;
    *state.config.write().await = None;

    tracing::info!("Capture stopped");
    Ok(())
}
