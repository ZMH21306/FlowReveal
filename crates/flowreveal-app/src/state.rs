use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use engine_core::http_message::{HttpMessage, HttpSession};
use engine_core::capture_config::CaptureConfig;
use engine_core::engine_stats::{CaptureStatus, EngineStats};
use engine_core::mitm::CaManager;
use engine_core::platform_integration::windows::ProxySettings;

pub struct AppState {
    pub sessions: Arc<RwLock<Vec<HttpSession>>>,
    pub capture_status: Arc<RwLock<CaptureStatus>>,
    pub stats: Arc<RwLock<EngineStats>>,
    pub event_tx: Arc<Mutex<Option<mpsc::Sender<HttpMessage>>>>,
    pub config: Arc<RwLock<Option<CaptureConfig>>>,
    pub shutdown_handle: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    pub ca_manager: Arc<RwLock<Option<CaManager>>>,
    pub original_proxy_settings: Arc<Mutex<Option<ProxySettings>>>,
    pub proxy_was_set: Arc<RwLock<bool>>,
    pub cert_was_installed: Arc<RwLock<bool>>,
    pub hook_injected_pids: Arc<Mutex<Vec<u32>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(Vec::new())),
            capture_status: Arc::new(RwLock::new(CaptureStatus::Idle)),
            stats: Arc::new(RwLock::new(EngineStats::default())),
            event_tx: Arc::new(Mutex::new(None)),
            config: Arc::new(RwLock::new(None)),
            shutdown_handle: Arc::new(Mutex::new(None)),
            ca_manager: Arc::new(RwLock::new(None)),
            original_proxy_settings: Arc::new(Mutex::new(None)),
            proxy_was_set: Arc::new(RwLock::new(false)),
            cert_was_installed: Arc::new(RwLock::new(false)),
            hook_injected_pids: Arc::new(Mutex::new(Vec::new())),
        }
    }
}
