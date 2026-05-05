use tauri::{command, State, AppHandle, Emitter};
use crate::state::{AppState, DiverterStatus};
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

    let mode = config.mode;
    let capture_https = config.capture_https;
    let forward_port = config.proxy_port;
    let transparent_port = config.transparent_proxy_port;

    tracing::info!(
        mode = ?mode,
        forward_port,
        transparent_port,
        capture_https,
        "[Capture] 开始初始化"
    );

    {
        let is_elevated = engine_core::divert::elevation::is_elevated();
        *state.is_elevated.write().await = is_elevated;
        tracing::info!(is_elevated, "[Capture] 管理员权限检查");

        let is_wifi = engine_core::divert::wifi_detect::is_wifi_adapter();
        *state.is_wifi.write().await = is_wifi;
        if is_wifi {
            tracing::warn!("[Capture] ⚠ 检测到 Wi-Fi 适配器，fast-path 可能导致全局捕获不可用");
        }
    }

    *state.capture_status.write().await = CaptureStatus::Running;

    let (tx, mut rx) = mpsc::channel::<HttpMessage>(4096);
    {
        let mut event_tx = state.event_tx.lock().await;
        *event_tx = Some(tx.clone());
    }

    let should_start_forward = true;
    let should_start_transparent = mode == CaptureMode::Global;
    let mut actual_mode = mode;

    let ca_ready: bool;

    if capture_https && should_start_forward {
        tracing::info!("[Capture] 初始化 MITM CA 证书");

        let app_data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("FlowReveal");

        let ca_manager = match CaManager::load_or_create(&app_data_dir) {
            Ok(m) => m,
            Err(e) => {
                *state.capture_status.write().await = CaptureStatus::Idle;
                tracing::error!(error = %e, "[Capture] CA 管理器初始化失败");
                return Err(format!("CA 证书初始化失败: {}", e));
            }
        };

        let cert_pem = ca_manager.ca_cert_pem().to_string();

        tracing::info!("[Capture] 确保 CA 证书已安装到系统受信任根存储");
        match windows::uninstall_ca_certificate() {
            Ok(()) => tracing::debug!("[Capture] 已清除旧版 CA 证书"),
            Err(_) => tracing::debug!("[Capture] 无旧版 CA 证书需要清除"),
        }
        match windows::install_ca_certificate(&cert_pem) {
            Ok(()) => {
                *state.cert_was_installed.write().await = true;
                tracing::info!("[Capture] ✓ CA 证书已安装，HTTPS 解密就绪");
            }
            Err(e) => {
                tracing::warn!(error = %e, "[Capture] CA 证书安装失败，HTTPS 可能无法解密");
            }
        }

        {
            let mut ca_guard = state.ca_manager.write().await;
            *ca_guard = Some(CaManager::from_pem(&cert_pem, ca_manager.ca_key_pem()).expect("CA from_pem after new"));
        }

        ca_ready = true;

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    } else {
        ca_ready = false;
    }

    let mut forward_shutdown_handle: Option<tokio::sync::oneshot::Sender<()>> = None;
    let mut transparent_shutdown_handle: Option<tokio::sync::oneshot::Sender<()>> = None;
    let mut diverter_shutdown_handle: Option<tokio::sync::oneshot::Sender<()>> = None;

    if should_start_forward {
        tracing::info!(port = forward_port, "[Capture] 启动正向代理");

        let start_config = if ca_ready {
            config.clone()
        } else {
            let mut cfg = config.clone();
            cfg.capture_https = false;
            cfg
        };

        match ForwardProxy::start(forward_port, &start_config, tx.clone(), state.rule_engine.clone()).await {
            Ok(handle) => {
                tracing::info!(port = forward_port, "[Capture] 正向代理启动成功");
                forward_shutdown_handle = Some(handle.shutdown_tx);

                if !ca_ready {
                    if let Some(ca) = handle.ca_manager {
                        let mut ca_guard = state.ca_manager.write().await;
                        let pem = ca.ca_cert_pem().to_string();
                        let key = ca.ca_key_pem().to_string();
                        if let Ok(manager) = CaManager::from_pem(&pem, &key) {
                            *ca_guard = Some(manager);
                            tracing::debug!("[Capture] CA 管理器已共享到 AppState");
                        }
                    }
                }
            }
            Err(e) => {
                *state.capture_status.write().await = CaptureStatus::Idle;
                tracing::error!(port = forward_port, error = %e, "[Capture] 正向代理启动失败");
                return Err(format!("正向代理启动失败: {}", e));
            }
        }
    }

    if should_start_transparent {
        #[cfg(target_os = "windows")]
        {
            use engine_core::proxy::transparent_proxy::TransparentProxy;
            use engine_core::divert::nat_table::NatTable;
            use std::sync::Arc;

            tracing::info!(port = transparent_port, "[Capture] 启动透明代理");

            let nat_table = Arc::new(NatTable::new(65536));
            let ca_manager = {
                let ca_guard = state.ca_manager.read().await;
                match ca_guard.as_ref() {
                    Some(ca) => {
                        let pem = ca.ca_cert_pem().to_string();
                        let key = ca.ca_key_pem().to_string();
                        Some(Arc::new(CaManager::from_pem(&pem, &key).expect("CA from_pem")))
                    }
                    None => None,
                }
            };
            let rule_engine = state.rule_engine.clone();

            match TransparentProxy::start(transparent_port, &config, tx.clone(), nat_table.clone(), ca_manager, rule_engine).await {
                Ok(handle) => {
                    tracing::info!(port = transparent_port, "[Capture] 透明代理启动成功");
                    transparent_shutdown_handle = Some(handle.shutdown_tx);
                }
                Err(e) => {
                    tracing::warn!(port = transparent_port, error = %e, "[Capture] 透明代理启动失败，回退到仅代理模式");
                    actual_mode = CaptureMode::ProxyOnly;
                    *state.diverter_status.write().await = DiverterStatus::Error;
                }
            }

            if transparent_shutdown_handle.is_some() {
                #[cfg(feature = "windivert")]
                {
                    use engine_core::divert::diverter::{PacketDiverter, DivertConfig};

                    let divert_config = DivertConfig {
                        proxy_port: transparent_port,
                        capture_ports: config.capture_ports.clone(),
                        exclude_pids: config.exclude_pids.clone(),
                        include_pids: config.include_pids.clone(),
                        capture_localhost: config.capture_localhost,
                    };

                    let mut diverter = match PacketDiverter::new(divert_config, nat_table) {
                        Ok(d) => d,
                        Err(e) => {
                            tracing::warn!(error = %e, "[Capture] PacketDiverter 创建失败，回退到仅代理模式");
                            *state.diverter_status.write().await = DiverterStatus::Error;
                            actual_mode = CaptureMode::ProxyOnly;
                            PacketDiverter::new(DivertConfig::default(), Arc::new(NatTable::new(1))).unwrap()
                        }
                    };

                    match diverter.start() {
                        Ok(()) => {
                            tracing::info!("[Capture] ✓ WinDivert 数据包重定向器已启动");
                            *state.diverter_status.write().await = DiverterStatus::Running;

                            let (div_shutdown_tx, div_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
                            diverter_shutdown_handle = Some(div_shutdown_tx);

                            tokio::spawn(async move {
                                let _ = div_shutdown_rx.await;
                                tracing::info!("[Capture] WinDivert 关闭信号已收到");
                            });

                            std::mem::forget(diverter);
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "[Capture] WinDivert 启动失败，回退到仅代理模式");
                            *state.diverter_status.write().await = DiverterStatus::Error;
                            actual_mode = CaptureMode::ProxyOnly;
                        }
                    }
                }

                #[cfg(not(feature = "windivert"))]
                {
                    tracing::warn!("[Capture] WinDivert 功能未编译，全局捕获不可用，回退到仅代理模式");
                    *state.diverter_status.write().await = DiverterStatus::NotAvailable;
                    actual_mode = CaptureMode::ProxyOnly;
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            tracing::warn!("[Capture] 透明代理仅支持 Windows 平台，回退到仅代理模式");
            *state.diverter_status.write().await = DiverterStatus::NotAvailable;
            actual_mode = CaptureMode::ProxyOnly;
        }
    } else {
        *state.diverter_status.write().await = DiverterStatus::Stopped;
    }

    *state.config.write().await = Some(config.clone());

    {
        let mut handles = state.proxy_handles.lock().await;
        handles.forward_shutdown = forward_shutdown_handle;
        handles.transparent_shutdown = transparent_shutdown_handle;
        handles.diverter_shutdown = diverter_shutdown_handle;
    }

    if should_start_forward {
        let proxy_addr = format!("127.0.0.1:{}", forward_port);
        match windows::set_system_proxy(&proxy_addr) {
            Ok(original) => {
                *state.proxy_was_set.write().await = true;
                *state.original_proxy_settings.lock().await = Some(original);
                tracing::info!(addr = %proxy_addr, "[Capture] 系统代理已配置");
            }
            Err(e) => {
                tracing::warn!(error = %e, "[Capture] 系统代理配置失败，请手动设置");
            }
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
                tracing::debug!(
                    count = processed_count,
                    direction = ?msg.direction,
                    session_id = msg.session_id,
                    method = ?msg.method,
                    status = ?msg.status_code,
                    "[TrafficProcessor] 消息处理"
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
        tracing::info!(count = processed_count, "[TrafficProcessor] 流量处理线程已退出");
    });

    tracing::info!(
        actual_mode = ?actual_mode,
        forward_port,
        transparent_port,
        https = capture_https,
        "[Capture] 抓包已启动"
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
        if let Some(shutdown) = handles.diverter_shutdown.take() {
            let _ = shutdown.send(());
            tracing::debug!("[Capture] WinDivert 关闭信号已发送");
        }
        if let Some(shutdown) = handles.transparent_shutdown.take() {
            let _ = shutdown.send(());
            tracing::debug!("[Capture] 透明代理关闭信号已发送");
        }
        if let Some(shutdown) = handles.forward_shutdown.take() {
            let _ = shutdown.send(());
            tracing::debug!("[Capture] 正向代理关闭信号已发送");
        }
    }

    *state.diverter_status.write().await = DiverterStatus::Stopped;

    if *state.cert_was_installed.read().await {
        tracing::info!("[Capture] 卸载 CA 证书");
        match windows::uninstall_ca_certificate() {
            Ok(()) => tracing::info!("[Capture] CA 证书已移除"),
            Err(e) => tracing::warn!(error = %e, "[Capture] CA 证书移除失败"),
        }
        *state.cert_was_installed.write().await = false;
    }

    if *state.proxy_was_set.read().await {
        tracing::info!("[Capture] 恢复系统代理");
        let original = state.original_proxy_settings.lock().await;
        if let Some(ref orig) = *original {
            match windows::restore_system_proxy(orig) {
                Ok(()) => tracing::info!("[Capture] 系统代理已恢复"),
                Err(e) => tracing::warn!(error = %e, "[Capture] 系统代理恢复失败"),
            }
        } else {
            match windows::clear_system_proxy() {
                Ok(()) => tracing::info!("[Capture] 系统代理已清除"),
                Err(e) => tracing::warn!(error = %e, "[Capture] 系统代理清除失败"),
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

    tracing::info!("[Capture] 抓包已停止");
    Ok(())
}

#[command]
pub fn reset_session_counter() -> Result<(), String> {
    engine_core::proxy::utils::reset_session_counter();
    tracing::debug!("[Capture] Session 计数器已重置");
    Ok(())
}

#[command]
pub fn get_diverter_status(state: State<'_, AppState>) -> String {
    let status = state.diverter_status.try_read();
    match status {
        Ok(s) => match *s {
            DiverterStatus::NotAvailable => "NotAvailable".to_string(),
            DiverterStatus::Stopped => "Stopped".to_string(),
            DiverterStatus::Running => "Running".to_string(),
            DiverterStatus::Error => "Error".to_string(),
        },
        Err(_) => "Unknown".to_string(),
    }
}

#[command]
pub fn is_elevated(state: State<'_, AppState>) -> bool {
    let elevated = state.is_elevated.try_read();
    match elevated {
        Ok(s) => *s,
        Err(_) => engine_core::divert::elevation::is_elevated(),
    }
}

#[command]
pub fn is_wifi_adapter() -> bool {
    engine_core::divert::wifi_detect::is_wifi_adapter()
}

#[command]
pub fn request_elevation() -> Result<(), String> {
    engine_core::divert::elevation::request_elevation()
        .map_err(|e| format!("提权请求失败: {}", e))
}
