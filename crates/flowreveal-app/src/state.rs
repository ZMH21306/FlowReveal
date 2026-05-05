use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use engine_core::http_message::{HttpMessage, HttpSession};
use engine_core::capture_config::CaptureConfig;
use engine_core::engine_stats::{CaptureStatus, EngineStats};
use engine_core::mitm::CaManager;
use engine_core::platform_integration::windows::ProxySettings;
use engine_core::rules::RuleEngine;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DiverterStatus {
    NotAvailable,
    Stopped,
    Running,
    Error,
}

pub struct ProxyHandles {
    pub forward_shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    pub transparent_shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    pub diverter_shutdown: Option<tokio::sync::oneshot::Sender<()>>,
}

pub struct AppState {
    pub sessions: Arc<RwLock<Vec<HttpSession>>>,
    pub capture_status: Arc<RwLock<CaptureStatus>>,
    pub stats: Arc<RwLock<EngineStats>>,
    pub event_tx: Arc<Mutex<Option<mpsc::Sender<HttpMessage>>>>,
    pub config: Arc<RwLock<Option<CaptureConfig>>>,
    pub proxy_handles: Arc<Mutex<ProxyHandles>>,
    pub ca_manager: Arc<RwLock<Option<CaManager>>>,
    pub original_proxy_settings: Arc<Mutex<Option<ProxySettings>>>,
    pub proxy_was_set: Arc<RwLock<bool>>,
    pub cert_was_installed: Arc<RwLock<bool>>,
    pub rule_engine: Arc<RuleEngine>,
    pub diverter_status: Arc<RwLock<DiverterStatus>>,
    pub is_elevated: Arc<RwLock<bool>>,
    pub is_wifi: Arc<RwLock<bool>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(Vec::new())),
            capture_status: Arc::new(RwLock::new(CaptureStatus::Idle)),
            stats: Arc::new(RwLock::new(EngineStats::default())),
            event_tx: Arc::new(Mutex::new(None)),
            config: Arc::new(RwLock::new(None)),
            proxy_handles: Arc::new(Mutex::new(ProxyHandles {
                forward_shutdown: None,
                transparent_shutdown: None,
                diverter_shutdown: None,
            })),
            ca_manager: Arc::new(RwLock::new(None)),
            original_proxy_settings: Arc::new(Mutex::new(None)),
            proxy_was_set: Arc::new(RwLock::new(false)),
            cert_was_installed: Arc::new(RwLock::new(false)),
            rule_engine: Arc::new(RuleEngine::new()),
            diverter_status: Arc::new(RwLock::new(DiverterStatus::NotAvailable)),
            is_elevated: Arc::new(RwLock::new(false)),
            is_wifi: Arc::new(RwLock::new(false)),
        }
    }
}
