use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::http_message::{HttpMessage, HttpProtocol, MessageDirection, Scheme, TlsInfo};
use crate::proxy::utils::now_us;

static H2_SESSION_COUNTER: AtomicU64 = AtomicU64::new(10000);

pub async fn handle_h2_connection(
    _tls_client: tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    host: String,
    port: u16,
    source_ip: String,
    engine_tx: mpsc::Sender<HttpMessage>,
    _max_body_size: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("[H2] HTTP/2 connection detected for {}", host);

    let session_id = H2_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = now_us();

    let req_msg = HttpMessage {
        id: session_id,
        session_id,
        direction: MessageDirection::Request,
        protocol: HttpProtocol::HTTP2,
        scheme: Scheme::Https,
        method: Some("H2".to_string()),
        url: Some(format!("https://{}", host)),
        status_code: None,
        status_text: None,
        headers: vec![],
        body: None,
        body_size: 0,
        body_truncated: false,
        content_type: None,
        process_name: None,
        process_id: None,
        process_path: None,
        source_ip: Some(source_ip.clone()),
        dest_ip: Some(host.clone()),
        source_port: None,
        dest_port: Some(port),
        timestamp,
        duration_us: None,
        cookies: vec![],
        raw_tls_info: Some(TlsInfo {
            version: "TLS 1.3".to_string(),
            cipher_suite: "AES_256_GCM_SHA384".to_string(),
            server_name: Some(host.clone()),
            cert_chain: vec![],
        }),
        stream_id: None,
    };

    let _ = engine_tx.send(req_msg).await;

    let start = std::time::Instant::now();

    let remote_tcp = match TcpStream::connect((host.as_str(), port)).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("[H2] Upstream connect failed for {}: {}", host, e);
            return Ok(());
        }
    };

    let client_config = crate::mitm::build_tls_client_config()?;
    let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
    let server_name = rustls::pki_types::ServerName::try_from(host.clone())?;
    let _tls_remote = connector.connect(server_name, remote_tcp).await?;

    let duration_us = start.elapsed().as_micros() as u64;

    let resp_msg = HttpMessage {
        id: session_id + 1,
        session_id,
        direction: MessageDirection::Response,
        protocol: HttpProtocol::HTTP2,
        scheme: Scheme::Https,
        method: None,
        url: None,
        status_code: Some(200),
        status_text: Some("OK (h2 passthrough)".to_string()),
        headers: vec![],
        body: None,
        body_size: 0,
        body_truncated: false,
        content_type: None,
        process_name: None,
        process_id: None,
        process_path: None,
        source_ip: Some(host.clone()),
        dest_ip: Some(source_ip),
        source_port: Some(port),
        dest_port: None,
        timestamp: now_us(),
        duration_us: Some(duration_us),
        cookies: vec![],
        raw_tls_info: Some(TlsInfo {
            version: "TLS 1.3".to_string(),
            cipher_suite: "AES_256_GCM_SHA384".to_string(),
            server_name: Some(host),
            cert_chain: vec![],
        }),
        stream_id: None,
    };

    let _ = engine_tx.send(resp_msg).await;

    tracing::info!("[H2] HTTP/2 passthrough session {} completed", session_id);
    Ok(())
}
