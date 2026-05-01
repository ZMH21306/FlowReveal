use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineStats {
    pub total_sessions: u64,
    pub active_sessions: u64,
    pub bytes_captured: u64,
    pub tls_handshakes: u64,
    pub hook_injections: u64,
    pub http1_requests: u64,
    pub http2_requests: u64,
    pub ws_frames: u64,
    pub filtered_out: u64,
}

impl Default for EngineStats {
    fn default() -> Self {
        Self {
            total_sessions: 0,
            active_sessions: 0,
            bytes_captured: 0,
            tls_handshakes: 0,
            hook_injections: 0,
            http1_requests: 0,
            http2_requests: 0,
            ws_frames: 0,
            filtered_out: 0,
        }
    }
}

impl EngineStats {
    pub fn increment_sessions(&mut self) {
        self.total_sessions += 1;
        self.active_sessions += 1;
    }

    pub fn decrement_active(&mut self) {
        self.active_sessions = self.active_sessions.saturating_sub(1);
    }

    pub fn add_bytes(&mut self, n: usize) {
        self.bytes_captured += n as u64;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineCapabilities {
    pub supports_wfp: bool,
    pub supports_api_hook: bool,
    pub supports_tls_mitm: bool,
    pub supports_http2: bool,
    pub supports_websocket: bool,
    pub process_identification: bool,
}

impl Default for EngineCapabilities {
    fn default() -> Self {
        Self {
            supports_wfp: false,
            supports_api_hook: false,
            supports_tls_mitm: false,
            supports_http2: false,
            supports_websocket: false,
            process_identification: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureStatus {
    Idle,
    Running,
    Error,
}

impl std::fmt::Display for CaptureStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureStatus::Idle => write!(f, "Idle"),
            CaptureStatus::Running => write!(f, "Running"),
            CaptureStatus::Error => write!(f, "Error"),
        }
    }
}
