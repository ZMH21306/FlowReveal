use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use engine_core::http_message::{HttpMessage, HttpSession};
use engine_core::capture_config::CaptureConfig;
use engine_core::engine_stats::{CaptureStatus, EngineStats};
use engine_core::proxy::ProxyShutdownHandle;

pub struct AppState {
    pub sessions: Arc<RwLock<Vec<HttpSession>>>,
    pub capture_status: Arc<RwLock<CaptureStatus>>,
    pub stats: Arc<RwLock<EngineStats>>,
    pub event_tx: Arc<Mutex<Option<mpsc::Sender<HttpMessage>>>>,
    pub config: Arc<RwLock<Option<CaptureConfig>>>,
    pub shutdown_handle: Arc<Mutex<Option<ProxyShutdownHandle>>>,
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
        }
    }
}
