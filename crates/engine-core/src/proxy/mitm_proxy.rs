use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use hyper::Request;
use hyper_util::rt::TokioIo;
use http_body_util::{BodyExt, Full};
use bytes::Bytes;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::http_message::{HttpMessage, HttpProtocol, MessageDirection, Scheme, TlsInfo};
use crate::mitm::{CaManager, MitmConfig, build_tls_client_config, build_tls_server_config, pem_to_der, private_key_pem_to_der};
use crate::protocol::websocket;
use crate::proxy::utils::{extract_header, now_us, parse_host_port, truncate_body, is_hop_by_hop_header};
use crate::rules::{RuleEngine, RuleExecutionResult, executor::RuleExecutor};

static MITM_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

async fn read_request_from_tls(
    tls_stream: tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    max_body_size: usize,
) -> Result<(tokio_rustls::server::TlsStream<tokio::net::TcpStream>, String, String, Vec<(String, String)>, Vec<u8>), Box<dyn std::error::Error + Send + Sync>> {
    let (tls_read, tls_write) = tokio::io::split(tls_stream);
    let mut reader = BufReader::new(tls_read);

    let mut first_line = String::new();
    let n = reader.read_line(&mut first_line).await?;
    if n == 0 {
        return Err("Empty request".into());
    }

    let first_line = first_line.trim_end().to_string();
    let parts: Vec<&str> = first_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err(format!("Invalid request line: {}", first_line).into());
    }

    let method = parts[0].to_string();
    let request_target = parts[1].to_string();

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

    let content_length: usize = extract_header(&raw_headers, "content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let mut req_body_bytes = Vec::new();
    if content_length > 0 {
        let to_read = content_length.min(max_body_size + 1);
        req_body_bytes.resize(to_read, 0u8);
        reader.read_exact(&mut req_body_bytes).await.ok();
        if content_length > max_body_size + 1 {
            let mut discard = [0u8; 4096];
            let mut remaining = content_length - to_read;
            while remaining > 0 {
                let chunk = remaining.min(4096);
                let n = reader.read(&mut discard[..chunk]).await.unwrap_or(0);
                if n == 0 { break; }
                remaining -= n;
            }
        }
    }

    let tls_read = reader.into_inner();
    let tls_stream = tls_read.unsplit(tls_write);

    Ok((tls_stream, method, request_target, raw_headers, req_body_bytes))
}

pub async fn handle_mitm_connect(
    client_stream: TcpStream,
    request_target: &str,
    req_headers: &[(String, String)],
    source_ip: &str,
    engine_tx: mpsc::Sender<HttpMessage>,
    ca_manager: Arc<CaManager>,
    mitm_config: &MitmConfig,
    max_body_size: usize,
    rule_engine: Arc<RuleEngine>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (host, port) = parse_host_port(request_target, 443);

    if mitm_config.should_bypass(&host) {
        tracing::info!("[MITM] 绕过主机 {} (在绕过列表中)，回退到隧道模式", host);
        return handle_connect_fallback(
            client_stream, &host, port, request_target, req_headers, source_ip, engine_tx,
        ).await;
    }

    tracing::info!("[MITM] 拦截 CONNECT 到 {}:{}", host, port);
    let session_id = MITM_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = now_us();

    let req_msg = build_connect_message(session_id, request_target, req_headers, source_ip, &host, port, timestamp);
    let _ = engine_tx.send(req_msg).await;

    let mut client = client_stream;
    let _ = client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await;

    let host_cert = match ca_manager.get_or_generate_cert(&host).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("[MITM] 生成 {} 证书失败: {}", host, e);
            return Ok(());
        }
    };

    let cert_der = pem_to_der(&host_cert.cert_pem)?;
    let key_der = private_key_pem_to_der(&host_cert.key_pem)?;

    let server_config = match build_tls_server_config(cert_der, key_der) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("[MITM] 构建 {} TLS 服务端配置失败: {}", host, e);
            return Ok(());
        }
    };

    let tls_acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));
    let tls_client = match tls_acceptor.accept(client).await {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!("[MITM] 与客户端的 TLS 握手失败 {}: {}", host, e);
            return Ok(());
        }
    };

    tracing::debug!("[MITM] 与客户端的 TLS 握手完成 {}", host);

    let (mut tls_client, method, request_target, raw_headers, req_body_bytes) =
        match read_request_from_tls(tls_client, max_body_size).await {
            Ok(result) => result,
            Err(e) => {
                tracing::debug!("[MITM] 读取 {} 请求失败: {}", host, e);
                return Ok(());
            }
        };

    let req_host = extract_header(&raw_headers, "host").unwrap_or_else(|| host.clone());
    let forward_url = if request_target.starts_with("http://") || request_target.starts_with("https://") {
        request_target.clone()
    } else {
        format!("https://{}{}", req_host, request_target)
    };

    let req_content_type = extract_header(&raw_headers, "content-type");
    let content_length: usize = extract_header(&raw_headers, "content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let (req_body_captured, req_body_truncated) = truncate_body(&req_body_bytes, max_body_size);

    let is_ws_upgrade = websocket::is_websocket_upgrade(&raw_headers);

    let tls_info = TlsInfo {
        version: "TLS 1.3".to_string(),
        cipher_suite: "AES_256_GCM_SHA384".to_string(),
        server_name: Some(host.clone()),
        cert_chain: vec![],
    };

    let protocol = if is_ws_upgrade { HttpProtocol::WebSocket } else { HttpProtocol::HTTP1_1 };

    let https_req_msg = HttpMessage {
        id: session_id + 100,
        session_id,
        direction: MessageDirection::Request,
        protocol,
        scheme: Scheme::Https,
        method: Some(method.clone()),
        url: Some(forward_url.clone()),
        status_code: None,
        status_text: None,
        headers: raw_headers.clone(),
        body: req_body_captured.clone(),
        body_size: content_length,
        body_truncated: req_body_truncated,
        content_type: req_content_type.clone(),
        process_name: None,
        process_id: None,
        process_path: None,
        source_ip: Some(source_ip.to_string()),
        dest_ip: Some(req_host.clone()),
        source_port: None,
        dest_port: Some(port),
        timestamp: now_us(),
        duration_us: None,
        cookies: vec![],
        raw_tls_info: Some(tls_info),
        stream_id: None,
    };

    let _ = engine_tx.send(https_req_msg).await;

    if is_ws_upgrade {
        tracing::info!("[MITM] WebSocket Upgrade 检测到: {}", forward_url);
        let _ = tls_client.write_all(b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n").await;
        let (ws_frame_tx, _) = mpsc::channel::<crate::http_message::WebSocketFrame>(1024);
        let ws_result = websocket::handle_websocket_proxy(
            tls_client, host, port, source_ip.to_string(), session_id, engine_tx, ws_frame_tx, max_body_size,
        ).await;
        if let Err(e) = ws_result {
            tracing::warn!("[MITM] WebSocket 代理错误: {}", e);
        }
        return Ok(());
    }

    // === MITM 规则检查点（请求阶段） ===
    if let Some(result) = rule_engine.apply(&HttpMessage {
        id: session_id + 100,
        session_id,
        direction: MessageDirection::Request,
        protocol: HttpProtocol::HTTP1_1,
        scheme: Scheme::Https,
        method: Some(method.clone()),
        url: Some(forward_url.clone()),
        status_code: None,
        status_text: None,
        headers: raw_headers.clone(),
        body: req_body_captured.clone(),
        body_size: content_length,
        body_truncated: req_body_truncated,
        content_type: req_content_type.clone(),
        process_name: None,
        process_id: None,
        process_path: None,
        source_ip: Some(source_ip.to_string()),
        dest_ip: Some(req_host.clone()),
        source_port: None,
        dest_port: Some(port),
        timestamp: now_us(),
        duration_us: None,
        cookies: vec![],
        raw_tls_info: None,
        stream_id: None,
    }, None).await {
        match result {
            RuleExecutionResult::AutoReply { status_code, status_text, headers, body, delay_ms } => {
                tracing::info!("[MITM] 自动回复规则触发 | status={} | delay={}ms", status_code, delay_ms);
                if delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
                let mut resp_line = format!("HTTP/1.1 {} {}\r\n", status_code, status_text);
                for (key, value) in &headers {
                    resp_line.push_str(&format!("{}: {}\r\n", key, value));
                }
                resp_line.push_str(&format!("Content-Length: {}\r\n", body.len()));
                resp_line.push_str("Connection: close\r\n\r\n");
                tls_client.write_all(resp_line.as_bytes()).await?;
                if !body.is_empty() {
                    tls_client.write_all(&body).await?;
                }
                tls_client.flush().await.ok();
                let mock_resp_msg = HttpMessage {
                    id: session_id + 101,
                    session_id,
                    direction: MessageDirection::Response,
                    protocol: HttpProtocol::HTTP1_1,
                    scheme: Scheme::Https,
                    method: None,
                    url: None,
                    status_code: Some(status_code),
                    status_text: Some(status_text),
                    headers,
                    body: if body.is_empty() { None } else { Some(body) },
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
                    duration_us: Some(0),
                    cookies: vec![],
                    raw_tls_info: Some(TlsInfo {
                        version: "TLS 1.3".to_string(),
                        cipher_suite: "AES_256_GCM_SHA384".to_string(),
                        server_name: Some(host.clone()),
                        cert_chain: vec![],
                    }),
                    stream_id: None,
                };
                let _ = engine_tx.send(mock_resp_msg).await;
                return Ok(());
            }
            RuleExecutionResult::Redirected { new_url, redirect_type, preserve_query, preserve_path } => {
                let final_url = RuleExecutor::build_redirect_url(&forward_url, &new_url, preserve_query, preserve_path);
                let redirect_code = match redirect_type {
                    crate::rules::RedirectType::Permanent301 => 301,
                    crate::rules::RedirectType::Temporary302 => 302,
                    crate::rules::RedirectType::Temporary307 => 307,
                    crate::rules::RedirectType::Permanent308 => 308,
                };
                tracing::info!("[MITM] 重定向规则触发 | {} -> {} | status={}", forward_url, final_url, redirect_code);
                let redirect_resp = format!(
                    "HTTP/1.1 {} Found\r\nLocation: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    redirect_code, final_url
                );
                tls_client.write_all(redirect_resp.as_bytes()).await?;
                tls_client.flush().await.ok();
                let redirect_resp_msg = HttpMessage {
                    id: session_id + 101,
                    session_id,
                    direction: MessageDirection::Response,
                    protocol: HttpProtocol::HTTP1_1,
                    scheme: Scheme::Https,
                    method: None,
                    url: None,
                    status_code: Some(redirect_code),
                    status_text: None,
                    headers: vec![("Location".to_string(), final_url)],
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
                    duration_us: Some(0),
                    cookies: vec![],
                    raw_tls_info: Some(TlsInfo {
                        version: "TLS 1.3".to_string(),
                        cipher_suite: "AES_256_GCM_SHA384".to_string(),
                        server_name: Some(host.clone()),
                        cert_chain: vec![],
                    }),
                    stream_id: None,
                };
                let _ = engine_tx.send(redirect_resp_msg).await;
                return Ok(());
            }
            RuleExecutionResult::HeaderModified { request_actions, response_actions } => {
                if !request_actions.is_empty() {
                    let mut modified_headers = raw_headers.clone();
                    RuleExecutor::apply_header_actions(&mut modified_headers, &request_actions);
                    tracing::info!("[MITM] 请求头修改规则触发 | 修改了 {} 个动作", request_actions.len());
                }
                if !response_actions.is_empty() {
                    tracing::info!("[MITM] 响应头修改规则将在响应阶段应用 | {} 个动作", response_actions.len());
                }
            }
        }
    }

    let start = std::time::Instant::now();

    let forward_uri: hyper::Uri = forward_url.parse().unwrap_or_else(|_| {
        format!("https://{}", request_target).parse().unwrap()
    });

    let forward_host = forward_uri.host().unwrap_or(&host).to_string();
    let forward_port = forward_uri.port_u16().unwrap_or(443);

    let remote_tcp = match TcpStream::connect((forward_host.as_str(), forward_port)).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("[MITM] 上游连接 {} 失败: {}", host, e);
            let _ = tls_client.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n").await;
            return Ok(());
        }
    };

    let client_config2 = build_tls_client_config()?;
    let connector2 = tokio_rustls::TlsConnector::from(Arc::new(client_config2));
    let server_name2 = rustls::pki_types::ServerName::try_from(forward_host.clone())?;
    let tls_remote2 = match connector2.connect(server_name2, remote_tcp).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("[MITM] 上游 TLS 握手 {} 失败: {}", host, e);
            let _ = tls_client.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n").await;
            return Ok(());
        }
    };

    let io = TokioIo::new(tls_remote2);

    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("[MITM] 上游 HTTP 握手 {} 失败: {}", host, e);
            let _ = tls_client.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n").await;
            return Ok(());
        }
    };

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::debug!("[MITM] 上游连接错误: {}", e);
        }
    });

    let mut forward_req_builder = Request::builder()
        .method(method.as_str())
        .uri(forward_uri);

    for (key, value) in &raw_headers {
        if !is_hop_by_hop_header(key) && !key.eq_ignore_ascii_case("host") && !key.eq_ignore_ascii_case("accept-encoding") {
            forward_req_builder = forward_req_builder.header(key.as_str(), value.as_str());
        }
    }
    forward_req_builder = forward_req_builder.header("Host", &forward_host);
    forward_req_builder = forward_req_builder.header("Connection", "close");
    forward_req_builder = forward_req_builder.header("Accept-Encoding", "identity");

    let forward_req = forward_req_builder.body(Full::new(Bytes::from(req_body_bytes))).unwrap();

    let upstream_resp = match sender.send_request(forward_req).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("[MITM] 转发请求 {} 失败: {}", host, e);
            let _ = tls_client.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n").await;
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
    let (resp_body_captured, resp_body_truncated) = truncate_body(&resp_body_bytes, max_body_size);

    let resp_msg = HttpMessage {
        id: session_id + 101,
        session_id,
        direction: MessageDirection::Response,
        protocol: HttpProtocol::HTTP1_1,
        scheme: Scheme::Https,
        method: None,
        url: None,
        status_code: Some(status_code),
        status_text: status_reason.clone(),
        headers: resp_headers.clone(),
        body: resp_body_captured,
        body_size: resp_body_size,
        body_truncated: resp_body_truncated,
        content_type: resp_content_type,
        process_name: None,
        process_id: None,
        process_path: None,
        source_ip: Some(forward_host.clone()),
        dest_ip: Some(source_ip.to_string()),
        source_port: Some(port),
        dest_port: None,
        timestamp: now_us(),
        duration_us: Some(duration_us),
        cookies: vec![],
        raw_tls_info: Some(TlsInfo {
            version: "TLS 1.3".to_string(),
            cipher_suite: "AES_256_GCM_SHA384".to_string(),
            server_name: Some(host.clone()),
            cert_chain: vec![],
        }),
        stream_id: None,
    };

    let _ = engine_tx.send(resp_msg).await;

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

    tls_client.write_all(response_line.as_bytes()).await?;
    if !resp_body_bytes.is_empty() {
        tls_client.write_all(&resp_body_bytes).await?;
    }
    tls_client.flush().await.ok();
    tokio::time::timeout(std::time::Duration::from_secs(2), tls_client.shutdown()).await.ok();

    tracing::info!("[MITM] ✓ HTTPS 拦截完成 {} (status={})", host, status_code);
    Ok(())
}

fn build_connect_message(
    session_id: u64,
    request_target: &str,
    req_headers: &[(String, String)],
    source_ip: &str,
    host: &str,
    port: u16,
    timestamp: u64,
) -> HttpMessage {
    HttpMessage {
        id: session_id,
        session_id,
        direction: MessageDirection::Request,
        protocol: HttpProtocol::HTTP1_1,
        scheme: Scheme::Https,
        method: Some("CONNECT".to_string()),
        url: Some(format!("https://{}", request_target)),
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
        dest_ip: Some(host.to_string()),
        source_port: None,
        dest_port: Some(port),
        timestamp,
        duration_us: None,
        cookies: vec![],
        raw_tls_info: None,
        stream_id: None,
    }
}

async fn handle_connect_fallback(
    mut client_stream: TcpStream,
    host: &str,
    port: u16,
    request_target: &str,
    req_headers: &[(String, String)],
    source_ip: &str,
    engine_tx: mpsc::Sender<HttpMessage>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let session_id = MITM_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = now_us();

    let req_msg = build_connect_message(session_id, request_target, req_headers, source_ip, host, port, timestamp);
    let _ = engine_tx.send(req_msg).await;

    let remote_stream = match TcpStream::connect((host, port)).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("[MITM] CONNECT 回退: 连接 {}:{} 失败 - {}", host, port, e);
            let _ = client_stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
            return Ok(());
        }
    };

    let start = std::time::Instant::now();
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
        status_text: Some("Connection Established (tunnel)".to_string()),
        headers: vec![],
        body: None,
        body_size: 0,
        body_truncated: false,
        content_type: None,
        process_name: None,
        process_id: None,
        process_path: None,
        source_ip: Some(host.to_string()),
        dest_ip: Some(source_ip.to_string()),
        source_port: Some(port),
        dest_port: None,
        timestamp: now_us(),
        duration_us: Some(duration_us),
        cookies: vec![],
        raw_tls_info: None,
        stream_id: None,
    };

    let _ = engine_tx.send(resp_msg).await;

    let _ = client_stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await;

    let (mut cr, mut cw) = tokio::io::split(client_stream);
    let (mut rr, mut rw) = tokio::io::split(remote_stream);

    tokio::select! {
        r = tokio::io::copy(&mut cr, &mut rw) => {
            if let Err(e) = r { tracing::debug!("[MITM] 回退隧道 c->r 错误: {}", e); }
        }
        r = tokio::io::copy(&mut rr, &mut cw) => {
            if let Err(e) = r { tracing::debug!("[MITM] 回退隧道 r->c 错误: {}", e); }
        }
    }

    Ok(())
}
