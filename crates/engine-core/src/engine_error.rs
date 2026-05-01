use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("Proxy error: {0}")]
    ProxyError(String),

    #[error("WFP error: {0}")]
    WfpError(String),

    #[error("MITM error: {0}")]
    MitmError(String),

    #[error("Certificate error: {0}")]
    CertError(String),

    #[error("Hook error: {0}")]
    HookError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Engine not running")]
    NotRunning,
}
