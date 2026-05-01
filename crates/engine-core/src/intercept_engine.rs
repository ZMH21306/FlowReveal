use async_trait::async_trait;
use crate::capture_config::CaptureConfig;
use crate::engine_error::EngineError;
use crate::engine_stats::{EngineCapabilities, EngineStats};
use crate::http_message::HttpMessage;
use tokio::sync::mpsc;

#[async_trait]
pub trait InterceptEngine: Send + Sync {
    async fn start(&mut self, config: CaptureConfig, event_tx: mpsc::Sender<HttpMessage>) -> Result<(), EngineError>;
    async fn stop(&mut self) -> Result<(), EngineError>;
    async fn update_config(&mut self, config: CaptureConfig) -> Result<(), EngineError>;
    async fn stats(&self) -> EngineStats;
    async fn install_ca_cert(&self, cert_pem: &str) -> Result<(), EngineError>;
    async fn uninstall_ca_cert(&self) -> Result<(), EngineError>;
    fn capabilities(&self) -> EngineCapabilities;
}
