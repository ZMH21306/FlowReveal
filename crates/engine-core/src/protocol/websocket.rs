use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::http_message::{HttpMessage, MessageDirection, WebSocketFrame, WsOpcode};
use crate::proxy::utils::now_us;

static WS_FRAME_COUNTER: AtomicU64 = AtomicU64::new(1);

pub async fn handle_websocket_proxy(
    mut tls_client: tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    host: String,
    port: u16,
    _source_ip: String,
    session_id: u64,
    _engine_tx: mpsc::Sender<HttpMessage>,
    ws_frame_tx: mpsc::Sender<WebSocketFrame>,
    max_body_size: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("[WS] WebSocket proxy starting for session {} -> {}:{}", session_id, host, port);

    let remote_tcp = match TcpStream::connect((host.as_str(), port)).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("[WS] Upstream connect failed for {}: {}", host, e);
            return Err(e.into());
        }
    };

    let client_config = crate::mitm::build_tls_client_config()?;
    let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
    let server_name = rustls::pki_types::ServerName::try_from(host.clone())?;
    let mut tls_remote = connector.connect(server_name, remote_tcp).await?;

    let upgrade_request = format!(
        "GET / HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n",
        host
    );
    tls_remote.write_all(upgrade_request.as_bytes()).await?;

    let mut resp_buf = [0u8; 4096];
    let n = tls_remote.read(&mut resp_buf).await?;
    if n == 0 {
        tracing::warn!("[WS] No response from upstream for {}", host);
        return Ok(());
    }

    let _ = tls_client.write_all(b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n").await;

    let (mut cr, mut cw) = tokio::io::split(tls_client);
    let (mut rr, mut rw) = tokio::io::split(tls_remote);

    let frame_tx_c = ws_frame_tx.clone();
    let frame_tx_s = ws_frame_tx.clone();

    let client_to_server = async move {
        let mut buf = [0u8; 8192];
        loop {
            match cr.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let frame = WebSocketFrame {
                        id: WS_FRAME_COUNTER.fetch_add(1, Ordering::Relaxed),
                        session_id,
                        direction: MessageDirection::Request,
                        opcode: WsOpcode::Binary,
                        payload: Some(buf[..n.min(max_body_size)].to_vec()),
                        payload_size: n,
                        payload_truncated: n > max_body_size,
                        timestamp: now_us(),
                    };
                    let _ = frame_tx_c.send(frame).await;
                    if rw.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };

    let server_to_client = async move {
        let mut buf = [0u8; 8192];
        loop {
            match rr.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let frame = WebSocketFrame {
                        id: WS_FRAME_COUNTER.fetch_add(1, Ordering::Relaxed),
                        session_id,
                        direction: MessageDirection::Response,
                        opcode: WsOpcode::Binary,
                        payload: Some(buf[..n.min(max_body_size)].to_vec()),
                        payload_size: n,
                        payload_truncated: n > max_body_size,
                        timestamp: now_us(),
                    };
                    let _ = frame_tx_s.send(frame).await;
                    if cw.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };

    tokio::select! {
        _ = client_to_server => {},
        _ = server_to_client => {},
    }

    tracing::info!("[WS] WebSocket connection closed for session {}", session_id);
    Ok(())
}

pub fn is_websocket_upgrade(headers: &[(String, String)]) -> bool {
    let has_upgrade = headers.iter().any(|(k, v)| {
        k.eq_ignore_ascii_case("upgrade") && v.eq_ignore_ascii_case("websocket")
    });
    let has_connection = headers.iter().any(|(k, v)| {
        k.eq_ignore_ascii_case("connection") && v.to_lowercase().contains("upgrade")
    });
    has_upgrade && has_connection
}
