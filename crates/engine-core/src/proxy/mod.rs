pub mod forward_proxy;
pub mod transparent_proxy;

pub struct ProxyShutdownHandle {
    pub shutdown_tx: tokio::sync::oneshot::Sender<()>,
}
