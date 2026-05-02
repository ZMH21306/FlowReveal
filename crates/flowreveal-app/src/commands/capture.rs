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
            return Err("抓包已在运行中".to_string());
        }
    }

    let (tx, mut rx) = mpsc::channel::<HttpMessage>(4096);

    {
        let mut event_tx = state.event_tx.lock().await;
        *event_tx = Some(tx.clone());
    }

    let mode = config.mode;
    let capture_https = config.capture_https;
    let forward_port = config.proxy_port;
    let transparent_port = config.transparent_proxy_port;

    tracing::info!(
        "[Capture] 开始初始化抓包 | 模式={:?} | 正向代理端口={} | 透明代理端口={} | HTTPS={}",
        mode, forward_port, transparent_port, capture_https
    );

    *state.capture_status.write().await = CaptureStatus::Running;

    let mut forward_shutdown_handle: Option<tokio::sync::oneshot::Sender<()>> = None;
    let mut transparent_shutdown_handle: Option<tokio::sync::oneshot::Sender<()>> = None;
    let mut ca_manager_from_proxy: Option<std::sync::Arc<CaManager>> = None;

    let should_start_forward = mode == CaptureMode::ForwardProxy || mode == CaptureMode::DualProxy;
    let should_start_transparent = mode == CaptureMode::TransparentProxy || mode == CaptureMode::DualProxy;

    if should_start_forward {
        tracing::info!("[Capture] 正在启动正向代理 (端口={})...", forward_port);
        match ForwardProxy::start(forward_port, &config, tx.clone()).await {
            Ok(handle) => {
                tracing::info!("[Capture] ✓ 正向代理启动成功 (端口={})", forward_port);
                forward_shutdown_handle = Some(handle.shutdown_tx);
                if let Some(ca) = handle.ca_manager {
                    ca_manager_from_proxy = Some(ca);
                }
            }
            Err(e) => {
                *state.capture_status.write().await = CaptureStatus::Idle;
                tracing::error!("[Capture] ✗ 正向代理启动失败: {}", e);
                return Err(format!("正向代理启动失败: {}", e));
            }
        }
    }

    if should_start_transparent {
        #[cfg(target_os = "windows")]
        {
            use engine_core::proxy::transparent_proxy::TransparentProxy;
            tracing::info!("[Capture] 正在启动透明代理 (端口={})...", transparent_port);
            match TransparentProxy::start(transparent_port, &config, tx.clone()).await {
                Ok(handle) => {
                    tracing::info!("[Capture] ✓ 透明代理启动成功 (端口={})", transparent_port);
                    transparent_shutdown_handle = Some(handle.shutdown_tx);
                }
                Err(e) => {
                    if forward_shutdown_handle.is_some() {
                        if let Some(shutdown) = forward_shutdown_handle.take() {
                            let _ = shutdown.send(());
                            tracing::warn!("[Capture] 已关闭正向代理（因透明代理启动失败）");
                        }
                    }
                    *state.capture_status.write().await = CaptureStatus::Idle;
                    tracing::error!("[Capture] ✗ 透明代理启动失败: {}", e);
                    return Err(format!("透明代理启动失败: {}", e));
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            tracing::warn!("[Capture] 透明代理仅支持 Windows 平台，跳过");
        }
    }

    tracing::info!(
        "[Capture] 代理启动完成 | 正向代理={} | 透明代理={}",
        if should_start_forward { "✓" } else { "✗" },
        if should_start_transparent { "✓" } else { "✗" }
    );

    *state.config.write().await = Some(config.clone());

    {
        let mut handles = state.proxy_handles.lock().await;
        handles.forward_shutdown = forward_shutdown_handle;
        handles.transparent_shutdown = transparent_shutdown_handle;
    }

    if let Some(ca) = ca_manager_from_proxy {
        let mut ca_guard = state.ca_manager.write().await;
        let pem = ca.ca_cert_pem().to_string();
        let key = ca.ca_key_pem().to_string();
        match CaManager::from_pem(&pem, &key) {
            Ok(manager) => {
                *ca_guard = Some(manager);
                tracing::info!("[Capture] CA 管理器已共享到 AppState");
            }
            Err(e) => {
                tracing::warn!("[Capture] CA 管理器克隆到 AppState 失败: {}", e);
            }
        }
    }

    if should_start_forward {
        let proxy_addr = format!("127.0.0.1:{}", forward_port);
        match windows::set_system_proxy(&proxy_addr) {
            Ok(original) => {
                *state.proxy_was_set.write().await = true;
                *state.original_proxy_settings.lock().await = Some(original);
                tracing::info!("[Capture] 系统代理已自动配置为 {}", proxy_addr);
            }
            Err(e) => {
                tracing::warn!("[Capture] 系统代理自动配置失败: {} - 请手动配置", e);
            }
        }
    }

    if capture_https {
        let ca_guard = state.ca_manager.read().await;
        if let Some(ca) = ca_guard.as_ref() {
            let cert_pem = ca.ca_cert_pem().to_string();
            drop(ca_guard);

            if !windows::is_ca_certificate_installed() {
                tracing::info!("[Capture] CA 证书未安装，正在安装...");
                match windows::install_ca_certificate(&cert_pem) {
                    Ok(()) => {
                        *state.cert_was_installed.write().await = true;
                        tracing::info!("[Capture] ✓ CA 证书已安装到受信任根证书存储");
                    }
                    Err(e) => {
                        tracing::warn!("[Capture] ✗ CA 证书安装失败: {} - HTTPS 解密可能无法在浏览器中工作", e);
                    }
                }
            } else {
                tracing::info!("[Capture] CA 证书已存在于受信任根证书存储");
                *state.cert_was_installed.write().await = true;
            }
        } else {
            tracing::warn!("[Capture] CA 管理器未初始化，跳过证书安装");
        }
    }

    let sessions = state.sessions.clone();
    let stats = state.stats.clone();
    let app_handle = app.clone();

    tokio::spawn(async move {
        tracing::info!("[TrafficProcessor] 流量处理线程已启动");
        let mut processed_count: u64 = 0;
        while let Some(msg) = rx.recv().await {
            processed_count += 1;
            let is_request = matches!(msg.direction, engine_core::http_message::MessageDirection::Request);

            if processed_count <= 5 || processed_count % 100 == 0 {
                tracing::info!(
                    "[TrafficProcessor] 处理消息 #{} | 方向={:?} | session_id={} | method={:?} | url={:?} | status={:?}",
                    processed_count,
                    msg.direction,
                    msg.session_id,
                    msg.method,
                    msg.url.as_ref().map(|u| if u.len() > 80 { &u[..80] } else { u }).unwrap_or("-"),
                    msg.status_code
                );
            }

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

                if sess.len() > 10000 {
                    sess.drain(..1000);
                }
            }

            let _ = app_handle.emit("traffic:request", &msg);
        }
        tracing::info!("[TrafficProcessor] 流量处理线程已退出，共处理 {} 条消息", processed_count);
    });

    tracing::info!(
        "[Capture] ✓ 抓包已启动 | 模式={:?} | 正向端口={} | 透明端口={}",
        mode, forward_port, transparent_port
    );
    Ok(())
}

#[command]
pub async fn stop_capture(state: State<'_, AppState>) -> Result<(), String> {
    {
        let status = state.capture_status.read().await;
        if *status != CaptureStatus::Running {
            return Err("抓包未在运行".to_string());
        }
    }

    tracing::info!("[Capture] 正在停止抓包...");

    {
        let mut handles = state.proxy_handles.lock().await;
        if let Some(shutdown) = handles.forward_shutdown.take() {
            let _ = shutdown.send(());
            tracing::info!("[Capture] 正向代理关闭信号已发送");
        }
        if let Some(shutdown) = handles.transparent_shutdown.take() {
            let _ = shutdown.send(());
            tracing::info!("[Capture] 透明代理关闭信号已发送");
        }
    }

    #[cfg(target_os = "windows")]
    {
        let config = state.config.read().await;
        if let Some(cfg) = config.as_ref() {
            if cfg.mode == CaptureMode::TransparentProxy || cfg.mode == CaptureMode::DualProxy {
                use engine_core::proxy::transparent_proxy::wfp_engine;
                tracing::info!("[Capture] 正在卸载 WFP 过滤器...");
                match wfp_engine::uninstall_redirect_filters() {
                    Ok(()) => tracing::info!("[Capture] ✓ WFP 过滤器已卸载"),
                    Err(e) => tracing::warn!("[Capture] ✗ WFP 过滤器卸载失败: {}", e),
                }
            }
        }
    }

    if *state.cert_was_installed.read().await {
        tracing::info!("[Capture] 正在卸载 CA 证书...");
        match windows::uninstall_ca_certificate() {
            Ok(()) => tracing::info!("[Capture] ✓ CA 证书已从受信任根证书存储移除"),
            Err(e) => tracing::warn!("[Capture] ✗ CA 证书移除失败: {}", e),
        }
        *state.cert_was_installed.write().await = false;
    }

    if *state.proxy_was_set.read().await {
        tracing::info!("[Capture] 正在恢复系统代理设置...");
        let original = state.original_proxy_settings.lock().await;
        if let Some(ref orig) = *original {
            match windows::restore_system_proxy(orig) {
                Ok(()) => tracing::info!("[Capture] ✓ 系统代理已恢复"),
                Err(e) => tracing::warn!("[Capture] ✗ 系统代理恢复失败: {}", e),
            }
        } else {
            match windows::clear_system_proxy() {
                Ok(()) => tracing::info!("[Capture] ✓ 系统代理已清除"),
                Err(e) => tracing::warn!("[Capture] ✗ 系统代理清除失败: {}", e),
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

    tracing::info!("[Capture] ✓ 抓包已停止");
    Ok(())
}
