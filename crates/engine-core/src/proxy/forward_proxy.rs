use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};

use hyper::Request;
use hyper_util::rt::TokioIo;
use http_body_util::{BodyExt, Full};
use bytes::Bytes;
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};

use crate::capture_config::CaptureConfig;
use crate::engine_error::EngineError;
use crate::http_message::{HttpMessage, HttpProtocol, MessageDirection, Scheme};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

pub struct ForwardProxyHandle {
    pub shutdown_tx: oneshot::Sender<()>,
}

pub struct ForwardProxy;

impl ForwardProxy {
    pub fn new() -> Self {
        Self
    }

    pub async fn start(
        port: u16,
        config: &CaptureConfig,
        engine_tx: mpsc::Sender<HttpMessage>,
    ) -> Result<ForwardProxyHandle, EngineError> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let max_body_size = config.max_body_size;

        let listener = TcpListener::bind(addr).await?;

        tracing::info!("Forward proxy listening on {}", addr);

        tokio::spawn(async move {
            let shutdown_rx = shutdown_rx;
            tokio::pin!(shutdown_rx);
            loop {
                let accept_result = tokio::select! {
                    result = listener.accept() => result,
                    _ = &mut shutdown_rx => {
                        tracing::info!("Forward proxy shutting down");
                        return;
                    }
                };

                match accept_result {
                    Ok((stream, client_addr)) => {
                        let engine_tx = engine_tx.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_raw_connection(stream, client_addr, engine_tx, max_body_size).await {
                                tracing::debug!("Connection error from {}: {}", client_addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Accept error: {}", e);
                    }
                }
            }
        });

        Ok(ForwardProxyHandle { shutdown_tx })
    }
}

fn extract_header(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

fn now_us() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}

async fn handle_raw_connection(
    stream: tokio::net::TcpStream,
    client_addr: SocketAddr,
    engine_tx: mpsc::Sender<HttpMessage>,
    max_body_size: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut reader = BufReader::new(stream);
    let mut first_line = String::new();
    let n = reader.read_line(&mut first_line).await?;
    if n == 0 {
        return Ok(());
    }

    let first_line = first_line.trim_end().to_string();
    let parts: Vec<&str> = first_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Ok(());
    }

    let method = parts[0];
    let request_target = parts[1];

    let mut raw_headers = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let line = line.trim_end().to_string();
        if line.is_empty() {
            break;
        }
        if let Some(colon_pos) = line.find(':') {
            let name = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            raw_headers.push((name, value));
        }
    }

    let source_ip = client_addr.ip().to_string();
    let timestamp = now_us();
    let session_id = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);

    if method.eq_ignore_ascii_case("CONNECT") {
        handle_connect_tunnel(
            reader,
            session_id,
            request_target,
            &raw_headers,
            &source_ip,
            timestamp,
            engine_tx,
        ).await
    } else {
        handle_http_request(
            reader,
            session_id,
            method,
            request_target,
            raw_headers,
            source_ip,
            timestamp,
            engine_tx,
            max_body_size,
        ).await
    }
}

async fn handle_connect_tunnel(
    reader: BufReader<tokio::net::TcpStream>,
    session_id: u64,
    request_target: &str,
    req_headers: &[(String, String)],
    source_ip: &str,
    timestamp: u64,
    engine_tx: mpsc::Sender<HttpMessage>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (host, port) = parse_host_port(request_target, 443);

    let req_msg = HttpMessage {
        id: session_id,
        session_id,
        direction: MessageDirection::Request,
        protocol: HttpProtocol::HTTP1_1,
        scheme: Scheme::Https,
        method: Some("CONNECT".to_string()),
        url: Some(request_target.to_string()),
        status_code: None,
        status_text: None,
        headers: req_headers.to_vec(),
        body: None,
        body_size: 0,
        body_truncated: false,
        content_type: None,
        process_name: None,
        process_id: None,
        process_path: None,
        source_ip: Some(source_ip.to_string()),
        dest_ip: Some(host.clone()),
        source_port: None,
        dest_port: Some(port),
        timestamp,
        duration_us: None,
        cookies: vec![],
        raw_tls_info: None,
    };

    let _ = engine_tx.send(req_msg).await;

    let start = std::time::Instant::now();

    let remote_stream = match tokio::net::TcpStream::connect((host.as_str(), port)).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("CONNECT: Failed to connect to {}:{} - {}", host, port, e);
            let stream = reader.into_inner();
            let mut stream = stream;
            let _ = stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
            return Ok(());
        }
    };

    let duration_us = start.elapsed().as_micros() as u64;

    let resp_msg = HttpMessage {
        id: session_id + 1,
        session_id,
        direction: MessageDirection::Response,
        protocol: HttpProtocol::HTTP1_1,
        scheme: Scheme::Https,
        method: None,
        url: None,
        status_code: Some(200),
        status_text: Some("Connection Established".to_string()),
        headers: vec![],
        body: None,
        body_size: 0,
        body_truncated: false,
        content_type: None,
        process_name: None,
        process_id: None,
        process_path: None,
        source_ip: Some(host.clone()),
        dest_ip: Some(source_ip.to_string()),
        source_port: Some(port),
        dest_port: None,
        timestamp: now_us(),
        duration_us: Some(duration_us),
        cookies: vec![],
        raw_tls_info: None,
    };

    let _ = engine_tx.send(resp_msg).await;

    let mut client_stream = reader.into_inner();
    let _ = client_stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await;

    tracing::debug!("CONNECT tunnel established to {}:{}", host, port);

    let (mut cr, mut cw) = client_stream.split();
    let (mut rr, mut rw) = tokio::io::split(remote_stream);

    let client_to_remote = tokio::io::copy(&mut cr, &mut rw);
    let remote_to_client = tokio::io::copy(&mut rr, &mut cw);

    tokio::select! {
        r = client_to_remote => {
            if let Err(e) = r {
                tracing::debug!("CONNECT tunnel client->remote error: {}", e);
            }
        }
        r = remote_to_client => {
            if let Err(e) = r {
                tracing::debug!("CONNECT tunnel remote->client error: {}", e);
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_http_request(
    mut reader: BufReader<tokio::net::TcpStream>,
    session_id: u64,
    method: &str,
    request_target: &str,
    req_headers: Vec<(String, String)>,
    source_ip: String,
    timestamp: u64,
    engine_tx: mpsc::Sender<HttpMessage>,
    max_body_size: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let host = extract_header(&req_headers, "host").unwrap_or_else(|| "unknown".to_string());
    let forward_url = if request_target.starts_with("http://") || request_target.starts_with("https://") {
        request_target.to_string()
    } else {
        format!("http://{}{}", host, request_target)
    };

    let content_length: usize = extract_header(&req_headers, "content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let mut req_body_bytes = Vec::new();
    if content_length > 0 {
        let to_read = content_length.min(max_body_size + 1);
        req_body_bytes.resize(to_read, 0u8);
        reader.read_exact(&mut req_body_bytes).await.ok();
        if content_length > max_body_size + 1 {
            let mut discard = vec![0u8; 4096];
            let mut remaining = content_length - to_read;
            while remaining > 0 {
                let chunk = remaining.min(4096);
                let n = reader.read(&mut discard[..chunk]).await.unwrap_or(0);
                if n == 0 { break; }
                remaining -= n;
            }
        }
    }

    let req_content_type = extract_header(&req_headers, "content-type");
    let req_body_size = req_body_bytes.len();
    let (req_body_captured, req_body_truncated) = if req_body_size > max_body_size {
        (Some(req_body_bytes[..max_body_size].to_vec()), true)
    } else if req_body_size > 0 {
        (Some(req_body_bytes.clone()), false)
    } else {
        (None, false)
    };

    let req_msg = HttpMessage {
        id: session_id,
        session_id,
        direction: MessageDirection::Request,
        protocol: HttpProtocol::HTTP1_1,
        scheme: Scheme::Http,
        method: Some(method.to_string()),
        url: Some(forward_url.clone()),
        status_code: None,
        status_text: None,
        headers: req_headers.clone(),
        body: req_body_captured,
        body_size: content_length,
        body_truncated: req_body_truncated,
        content_type: req_content_type,
        process_name: None,
        process_id: None,
        process_path: None,
        source_ip: Some(source_ip.clone()),
        dest_ip: Some(host.clone()),
        source_port: None,
        dest_port: None,
        timestamp,
        duration_us: None,
        cookies: vec![],
        raw_tls_info: None,
    };

    let _ = engine_tx.send(req_msg).await;

    let start = std::time::Instant::now();

    let forward_uri: hyper::Uri = forward_url.parse().unwrap_or_else(|_| {
        format!("http://{}", request_target).parse().unwrap()
    });

    let port = forward_uri.port_u16().unwrap_or(80);
    let forward_host = forward_uri.host().unwrap_or("unknown").to_string();

    let upstream_stream = match tokio::net::TcpStream::connect((forward_host.as_str(), port)).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Failed to connect to {}: {}", forward_host, e);
            let mut client_stream = reader.into_inner();
            let resp = format!("HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\n\r\nFailed to connect to upstream: {}", e.to_string().len(), e);
            let _ = client_stream.write_all(resp.as_bytes()).await;
            return Ok(());
        }
    };

    let io = TokioIo::new(upstream_stream);

    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Handshake failed: {}", e);
            let mut client_stream = reader.into_inner();
            let resp = format!("HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\n\r\nUpstream handshake failed: {}", e.to_string().len(), e);
            let _ = client_stream.write_all(resp.as_bytes()).await;
            return Ok(());
        }
    };

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::debug!("Upstream connection error: {}", e);
        }
    });

    let mut forward_req_builder = Request::builder()
        .method(method)
        .uri(forward_uri);

    for (key, value) in &req_headers {
        let kl = key.to_lowercase();
        if kl != "host" && kl != "connection" && kl != "proxy-connection" && kl != "proxy-authorization" {
            forward_req_builder = forward_req_builder.header(key.as_str(), value.as_str());
        }
    }
    forward_req_builder = forward_req_builder.header("Host", &forward_host);
    forward_req_builder = forward_req_builder.header("Connection", "close");

    let forward_req = forward_req_builder.body(Full::new(Bytes::from(req_body_bytes))).unwrap();

    let upstream_resp = match sender.send_request(forward_req).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Forward request failed: {}", e);
            let mut client_stream = reader.into_inner();
            let resp = format!("HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\n\r\nUpstream request failed: {}", e.to_string().len(), e);
            let _ = client_stream.write_all(resp.as_bytes()).await;
            return Ok(());
        }
    };

    let duration_us = start.elapsed().as_micros() as u64;
    let status_code = upstream_resp.status().as_u16();
    let status_reason = upstream_resp.status().canonical_reason().map(|s| s.to_string());
    let resp_headers: Vec<(String, String)> = upstream_resp
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let resp_content_type = extract_header(&resp_headers, "content-type");
    let resp_body_bytes = upstream_resp.into_body()
        .collect()
        .await
        .map(|b| b.to_bytes().to_vec())
        .unwrap_or_default();
    let resp_body_size = resp_body_bytes.len();
    let (resp_body_captured, resp_body_truncated) = if resp_body_size > max_body_size {
        (Some(resp_body_bytes[..max_body_size].to_vec()), true)
    } else if resp_body_size > 0 {
        (Some(resp_body_bytes.clone()), false)
    } else {
        (None, false)
    };

    let resp_msg = HttpMessage {
        id: session_id + 1,
        session_id,
        direction: MessageDirection::Response,
        protocol: HttpProtocol::HTTP1_1,
        scheme: Scheme::Http,
        method: None,
        url: None,
        status_code: Some(status_code),
        status_text: None,
        headers: resp_headers.clone(),
        body: resp_body_captured,
        body_size: resp_body_size,
        body_truncated: resp_body_truncated,
        content_type: resp_content_type,
        process_name: None,
        process_id: None,
        process_path: None,
        source_ip: Some(forward_host.clone()),
        dest_ip: Some(source_ip.clone()),
        source_port: Some(port),
        dest_port: None,
        timestamp: now_us(),
        duration_us: Some(duration_us),
        cookies: vec![],
        raw_tls_info: None,
    };

    let _ = engine_tx.send(resp_msg).await;

    let mut client_stream = reader.into_inner();

    let mut response_line = format!("HTTP/1.1 {} {}\r\n", status_code, status_reason.unwrap_or_else(|| "Unknown".to_string()));
    for (key, value) in &resp_headers {
        let kl = key.to_lowercase();
        if kl != "connection" && kl != "transfer-encoding" && kl != "content-length" {
            response_line.push_str(&format!("{}: {}\r\n", key, value));
        }
    }
    response_line.push_str(&format!("Content-Length: {}\r\n", resp_body_size));
    response_line.push_str("Connection: close\r\n");
    response_line.push_str("\r\n");

    client_stream.write_all(response_line.as_bytes()).await?;
    if !resp_body_bytes.is_empty() {
        client_stream.write_all(&resp_body_bytes).await?;
    }
    let _ = client_stream.shutdown().await;

    Ok(())
}

fn parse_host_port(target: &str, default_port: u16) -> (String, u16) {
    if let Some(bracket_end) = target.find(']') {
        let host = target[..=bracket_end].to_string();
        let port = target[bracket_end + 1..]
            .strip_prefix(':')
            .and_then(|p| p.parse().ok())
            .unwrap_or(default_port);
        (host, port)
    } else if let Some(colon_pos) = target.rfind(':') {
        let host = target[..colon_pos].to_string();
        let port = target[colon_pos + 1..].parse().unwrap_or(default_port);
        (host, port)
    } else {
        (target.to_string(), default_port)
    }
}
