use tauri::{command, State, AppHandle, Emitter};
use crate::state::AppState;
use engine_core::capture_config::CaptureConfig;
use engine_core::engine_stats::CaptureStatus;
use engine_core::http_message::{HttpMessage, HttpSession};
use engine_core::proxy::forward_proxy::ForwardProxy;
use tokio::sync::mpsc;

#[command]
pub async fn start_capture(
    app: AppHandle,
    state: State<'_, AppState>,
    config: CaptureConfig,
) -> Result<(), String> {
    let status = state.capture_status.read().await;
    if *status == CaptureStatus::Running {
        return Err("Capture is already running".to_string());
    }
    drop(status);

    let (tx, mut rx) = mpsc::channel::<HttpMessage>(1024);

    {
        let mut event_tx = state.event_tx.lock().await;
        *event_tx = Some(tx.clone());
    }

    let port = config.proxy_port;
    tracing::info!("Starting capture on port {}", port);

    let handle = ForwardProxy::start(port, &config, tx)
        .await
        .map_err(|e| format!("Failed to start proxy: {}", e))?;

    tracing::info!("Proxy started successfully on port {}", port);

    *state.capture_status.write().await = CaptureStatus::Running;
    *state.config.write().await = Some(config);

    {
        let mut shutdown = state.shutdown_handle.lock().await;
        *shutdown = Some(handle);
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

    tracing::info!("Capture started on port {}", port);
    Ok(())
}

#[command]
pub async fn stop_capture(state: State<'_, AppState>) -> Result<(), String> {
    let mut status = state.capture_status.write().await;
    if *status != CaptureStatus::Running {
        return Err("Capture is not running".to_string());
    }
    *status = CaptureStatus::Idle;
    drop(status);

    {
        let mut shutdown = state.shutdown_handle.lock().await;
        if let Some(handle) = shutdown.take() {
            let _ = handle.shutdown_tx.send(());
            tracing::info!("Proxy shutdown signal sent");
        }
    }

    {
        let mut event_tx = state.event_tx.lock().await;
        *event_tx = None;
    }

    tracing::info!("Capture stopped");
    Ok(())
}
