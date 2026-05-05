use std::sync::Arc;

use hyper::Request;
use hyper_util::rt::TokioIo;
use http_body_util::{BodyExt, Full};
use bytes::Bytes;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::http_message::{HttpMessage, HttpProtocol, Scheme, TlsInfo};
use crate::mitm::{CaManager, MitmConfig, build_tls_client_config, build_tls_server_config, pem_to_der, private_key_pem_to_der};
use crate::protocol::websocket;
use crate::proxy::utils::{extract_header, now_us, parse_host_port, truncate_body, is_hop_by_hop_header, next_session_id, read_headers_from_buf, read_body_from_buf};
use crate::rules::{RuleEngine, RuleExecutionResult, executor::RuleExecutor};

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

    let raw_headers = read_headers_from_buf(&mut reader).await?;

    let content_length: usize = extract_header(&raw_headers, "content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let req_body_bytes = read_body_from_buf(&mut reader, content_length, max_body_size).await;

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
    proc_info: Option<&crate::process_info::ProcessInfo>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (host, port) = parse_host_port(request_target, 443);

    if mitm_config.should_bypass(&host) {
        tracing::info!(host, "[MITM] 绕过主机，回退到隧道模式");
        return handle_connect_fallback(
            client_stream, &host, port, request_target, req_headers, source_ip, engine_tx,
        ).await;
    }

    tracing::info!(host, port, "[MITM] 拦截 CONNECT");
    let connect_session_id = next_session_id();
    let timestamp = now_us();

    let req_msg = HttpMessage::request(connect_session_id, HttpProtocol::HTTP1_1, Scheme::Https, "CONNECT", format!("https://{}", request_target), req_headers.to_vec(), timestamp)
        .process_info(proc_info)
        .source_ip(source_ip)
        .dest_ip(&host)
        .dest_port(port)
        .build();
    let _ = engine_tx.send(req_msg).await;

    let mut client = client_stream;
    let _ = client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await;

    let connect_resp = HttpMessage::response(connect_session_id, HttpProtocol::HTTP1_1, Scheme::Https, 200, vec![], now_us(), 0)
        .status_text("Connection Established")
        .source_ip(&host)
        .dest_ip(source_ip)
        .source_port(port)
        .build();
    let _ = engine_tx.send(connect_resp).await;

    let host_cert = match ca_manager.get_or_generate_cert(&host).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(host, error = %e, "[MITM] 生成证书失败");
            return Ok(());
        }
    };

    let cert_der = pem_to_der(&host_cert.cert_pem)?;
    let key_der = private_key_pem_to_der(&host_cert.key_pem)?;

    let server_config = match build_tls_server_config(cert_der, key_der) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(host, error = %e, "[MITM] 构建 TLS 服务端配置失败");
            return Ok(());
        }
    };

    let tls_acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));
    let tls_client = match tls_acceptor.accept(client).await {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!(host, error = %e, "[MITM] 与客户端的 TLS 握手失败");
            return Ok(());
        }
    };

    tracing::debug!(host, "[MITM] 与客户端的 TLS 握手完成");

    let (mut tls_client, method, request_target, raw_headers, req_body_bytes) =
        match read_request_from_tls(tls_client, max_body_size).await {
            Ok(result) => result,
            Err(e) => {
                tracing::debug!(host, error = %e, "[MITM] 读取请求失败");
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

    let https_req_msg = HttpMessage::request(connect_session_id, protocol, Scheme::Https, &method, &forward_url, raw_headers.clone(), now_us())
        .process_info(proc_info)
        .source_ip(source_ip)
        .dest_ip(&req_host)
        .dest_port(port)
        .body(req_body_captured.clone())
        .body_size(content_length)
        .body_truncated(req_body_truncated)
        .content_type(req_content_type.clone().unwrap_or_default())
        .raw_tls_info(tls_info.clone())
        .build();

    let _ = engine_tx.send(https_req_msg).await;

    if is_ws_upgrade {
        tracing::info!(url = %forward_url, "[MITM] WebSocket Upgrade 检测到");
        let _ = tls_client.write_all(b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n").await;
        let (ws_frame_tx, _) = mpsc::channel::<crate::http_message::WebSocketFrame>(1024);
        let ws_result = websocket::handle_websocket_proxy(
            tls_client, host, port, source_ip.to_string(), connect_session_id, engine_tx, ws_frame_tx, max_body_size,
        ).await;
        if let Err(e) = ws_result {
            tracing::warn!(error = %e, "[MITM] WebSocket 代理错误");
        }
        return Ok(());
    }

    if let Some(result) = rule_engine.apply(&HttpMessage::request(connect_session_id, HttpProtocol::HTTP1_1, Scheme::Https, &method, &forward_url, raw_headers.clone(), now_us())
        .process_info(proc_info)
        .source_ip(source_ip)
        .dest_ip(&req_host)
        .dest_port(port)
        .body(req_body_captured.clone())
        .body_size(content_length)
        .body_truncated(req_body_truncated)
        .content_type(req_content_type.clone().unwrap_or_default())
        .build(), None).await {
        match result {
            RuleExecutionResult::AutoReply { status_code, status_text, headers, body, delay_ms } => {
                tracing::info!(status_code, delay_ms, "[MITM] 自动回复规则触发");
                if delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
                write_raw_response(&mut tls_client, status_code, &status_text, &headers, &body).await?;
                let mock_resp = HttpMessage::response(connect_session_id, HttpProtocol::HTTP1_1, Scheme::Https, status_code, headers, now_us(), 0)
                    .status_text(&status_text)
                    .body(if body.is_empty() { None } else { Some(body) })
                    .source_ip(&host)
                    .dest_ip(source_ip)
                    .source_port(port)
                    .raw_tls_info(tls_info.clone())
                    .build();
                let _ = engine_tx.send(mock_resp).await;
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
                tracing::info!(from = %forward_url, to = %final_url, redirect_code, "[MITM] 重定向规则触发");
                let redirect_resp = format!(
                    "HTTP/1.1 {} Found\r\nLocation: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    redirect_code, final_url
                );
                tls_client.write_all(redirect_resp.as_bytes()).await?;
                tls_client.flush().await.ok();
                let redirect_msg = HttpMessage::response(connect_session_id, HttpProtocol::HTTP1_1, Scheme::Https, redirect_code, vec![("Location".to_string(), final_url)], now_us(), 0)
                    .source_ip(&host)
                    .dest_ip(source_ip)
                    .source_port(port)
                    .raw_tls_info(tls_info.clone())
                    .build();
                let _ = engine_tx.send(redirect_msg).await;
                return Ok(());
            }
            RuleExecutionResult::HeaderModified { request_actions, response_actions } => {
                if !request_actions.is_empty() {
                    tracing::debug!(actions = request_actions.len(), "[MITM] 请求头修改规则触发");
                }
                if !response_actions.is_empty() {
                    tracing::debug!(actions = response_actions.len(), "[MITM] 响应头修改规则将在响应阶段应用");
                }
                let _ = response_actions;
            }
        }
    }

    forward_https_request(
        tls_client, connect_session_id, &method, &request_target,
        &raw_headers, &req_body_bytes, &host, port, source_ip,
        &forward_url, engine_tx, max_body_size, tls_info,
    ).await
}

async fn forward_https_request(
    mut tls_client: tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    session_id: u64,
    method: &str,
    request_target: &str,
    raw_headers: &[(String, String)],
    req_body_bytes: &[u8],
    host: &str,
    port: u16,
    source_ip: &str,
    forward_url: &str,
    engine_tx: mpsc::Sender<HttpMessage>,
    max_body_size: usize,
    tls_info: TlsInfo,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let start = std::time::Instant::now();

    let forward_uri: hyper::Uri = forward_url.parse().unwrap_or_else(|_| {
        format!("https://{}", request_target).parse().unwrap()
    });

    let forward_host = forward_uri.host().unwrap_or(host).to_string();
    let forward_port = forward_uri.port_u16().unwrap_or(443);

    let remote_tcp = match TcpStream::connect((forward_host.as_str(), forward_port)).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(host, error = %e, "[MITM] 上游连接失败");
            let _ = tls_client.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n").await;
            return Ok(());
        }
    };

    let client_config = build_tls_client_config()?;
    let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
    let server_name = rustls::pki_types::ServerName::try_from(forward_host.clone())?;
    let tls_remote = match connector.connect(server_name, remote_tcp).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(host, error = %e, "[MITM] 上游 TLS 握手失败");
            let _ = tls_client.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n").await;
            return Ok(());
        }
    };

    let io = TokioIo::new(tls_remote);

    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(host, error = %e, "[MITM] 上游 HTTP 握手失败");
            let _ = tls_client.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n").await;
            return Ok(());
        }
    };

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::trace!(error = %e, "[MITM] 上游连接关闭");
        }
    });

    let mut forward_req_builder = Request::builder()
        .method(method)
        .uri(&forward_uri);

    for (key, value) in raw_headers {
        if !is_hop_by_hop_header(key) && !key.eq_ignore_ascii_case("host") && !key.eq_ignore_ascii_case("accept-encoding") {
            forward_req_builder = forward_req_builder.header(key.as_str(), value.as_str());
        }
    }
    forward_req_builder = forward_req_builder.header("Host", &forward_host);
    forward_req_builder = forward_req_builder.header("Connection", "close");
    forward_req_builder = forward_req_builder.header("Accept-Encoding", "identity");

    let forward_req = forward_req_builder.body(Full::new(Bytes::from(req_body_bytes.to_vec())))?;

    let upstream_resp = match sender.send_request(forward_req).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(host, error = %e, "[MITM] 转发请求失败");
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

    let resp_msg = HttpMessage::response(session_id, HttpProtocol::HTTP1_1, Scheme::Https, status_code, resp_headers.clone(), now_us(), duration_us)
        .status_text(status_reason.clone().unwrap_or_default())
        .body(resp_body_captured)
        .body_size(resp_body_size)
        .body_truncated(resp_body_truncated)
        .content_type(resp_content_type.unwrap_or_default())
        .source_ip(&forward_host)
        .dest_ip(source_ip)
        .source_port(port)
        .raw_tls_info(tls_info)
        .build();

    let _ = engine_tx.send(resp_msg).await;

    write_raw_response(&mut tls_client, status_code, &status_reason.unwrap_or_else(|| "Unknown".to_string()), &resp_headers, &resp_body_bytes).await?;

    tracing::info!(host, status_code, "[MITM] HTTPS 拦截完成");
    Ok(())
}

async fn write_raw_response(
    tls_client: &mut tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    status_code: u16,
    status_reason: &str,
    resp_headers: &[(String, String)],
    resp_body: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut response_line = format!("HTTP/1.1 {} {}\r\n", status_code, status_reason);
    for (key, value) in resp_headers {
        let kl = key.to_lowercase();
        if kl != "connection" && kl != "transfer-encoding" && kl != "content-length" {
            response_line.push_str(&format!("{}: {}\r\n", key, value));
        }
    }
    response_line.push_str(&format!("Content-Length: {}\r\n", resp_body.len()));
    response_line.push_str("Connection: close\r\n");
    response_line.push_str("\r\n");

    tls_client.write_all(response_line.as_bytes()).await?;
    if !resp_body.is_empty() {
        tls_client.write_all(resp_body).await?;
    }
    tls_client.flush().await.ok();
    tokio::time::timeout(std::time::Duration::from_secs(2), tls_client.shutdown()).await.ok();
    Ok(())
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
    let session_id = next_session_id();
    let timestamp = now_us();

    let req_msg = HttpMessage::request(session_id, HttpProtocol::HTTP1_1, Scheme::Https, "CONNECT", format!("https://{}", request_target), req_headers.to_vec(), timestamp)
        .source_ip(source_ip)
        .dest_ip(host)
        .dest_port(port)
        .build();
    let _ = engine_tx.send(req_msg).await;

    let remote_stream = match TcpStream::connect((host, port)).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(host, port, error = %e, "[MITM] CONNECT 回退: 连接上游失败");
            let _ = client_stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
            return Ok(());
        }
    };

    let start = std::time::Instant::now();
    let duration_us = start.elapsed().as_micros() as u64;

    let resp_msg = HttpMessage::response(session_id, HttpProtocol::HTTP1_1, Scheme::Https, 200, vec![], now_us(), duration_us)
        .status_text("Connection Established (tunnel)")
        .source_ip(host)
        .dest_ip(source_ip)
        .source_port(port)
        .build();

    let _ = engine_tx.send(resp_msg).await;

    let _ = client_stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await;

    let (mut cr, mut cw) = tokio::io::split(client_stream);
    let (mut rr, mut rw) = tokio::io::split(remote_stream);

    tokio::select! {
        r = tokio::io::copy(&mut cr, &mut rw) => {
            if let Err(e) = r { tracing::trace!(error = %e, "[MITM] 回退隧道 c->r 关闭"); }
        }
        r = tokio::io::copy(&mut rr, &mut cw) => {
            if let Err(e) = r { tracing::trace!(error = %e, "[MITM] 回退隧道 r->c 关闭"); }
        }
    }

    Ok(())
}
