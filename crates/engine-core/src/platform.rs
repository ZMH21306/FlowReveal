use async_trait::async_trait;
use crate::capture_config::CaptureConfig;
use crate::engine_error::EngineError;
use crate::http_message::HttpMessage;
use crate::process_info::ProcessInfo;
use crate::engine_stats::EngineCapabilities;
use tokio::sync::mpsc;

#[async_trait]
pub trait PlatformCapture: Send + Sync {
    async fn start(&mut self, config: &CaptureConfig, event_tx: mpsc::Sender<HttpMessage>) -> Result<(), EngineError>;
    async fn stop(&mut self) -> Result<(), EngineError>;
    async fn is_running(&self) -> bool;
    fn capabilities(&self) -> EngineCapabilities;

    async fn resolve_process(&self, pid: u32) -> Option<ProcessInfo>;
    async fn resolve_process_by_connection(&self, local_port: u16, remote_addr: &str, remote_port: u16) -> Option<ProcessInfo>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookTarget {
    WinHttpSendRequest,
    WinHttpReceiveResponse,
    WinHttpWriteData,
    WinHttpReadData,
    WinHttpCloseHandle,
    SslEncryptPacket,
    SslDecryptPacket,
}

#[derive(Debug, Clone)]
pub struct HookPacket {
    pub target: HookTarget,
    pub direction: HookDirection,
    pub process_id: u32,
    pub thread_id: u32,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub data: Vec<u8>,
    pub sequence: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookDirection {
    Outgoing,
    Incoming,
}

#[async_trait]
pub trait PlatformHook: Send + Sync {
    async fn inject(&mut self, pid: u32, targets: &[HookTarget]) -> Result<(), EngineError>;
    async fn eject(&mut self, pid: u32) -> Result<(), EngineError>;
    async fn eject_all(&mut self) -> Result<(), EngineError>;
    async fn is_injected(&self, pid: u32) -> bool;
    async fn injected_pids(&self) -> Vec<u32>;
}
