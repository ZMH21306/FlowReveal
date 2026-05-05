use std::net::SocketAddr;
use std::sync::Arc;

use hyper::Request;
use hyper_util::rt::TokioIo;
use http_body_util::{BodyExt, Full};
use bytes::Bytes;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};

use crate::capture_config::CaptureConfig;
use crate::engine_error::EngineError;
use crate::http_message::{HttpMessage, HttpProtocol, Scheme};
use crate::mitm::{CaManager, MitmConfig};
use crate::proxy::mitm_proxy;
use crate::proxy::utils::{extract_header, now_us, parse_host_port, truncate_body, is_hop_by_hop_header, next_session_id, read_headers_from_buf, read_body_from_buf};
use crate::rules::{RuleEngine, RuleExecutionResult, executor::RuleExecutor};

pub struct ForwardProxyHandle {
    pub shutdown_tx: oneshot::Sender<()>,
    pub ca_manager: Option<Arc<CaManager>>,
}

pub struct ForwardProxy;

impl ForwardProxy {
    pub async fn start(
        port: u16,
        config: &CaptureConfig,
        engine_tx: mpsc::Sender<HttpMessage>,
        rule_engine: Arc<RuleEngine>,
    ) -> Result<ForwardProxyHandle, EngineError> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let max_body_size = config.max_body_size;
        let capture_https = config.capture_https;

        tracing::info!(port, capture_https, max_body_size, "[ForwardProxy] 初始化中");

        let mitm_config = MitmConfig {
            enabled: capture_https,
            bypass_hosts: config.mitm_bypass_hosts.clone(),
            bypass_port_ranges: vec![],
            max_cert_cache_entries: 1024,
            cert_validity_days: 365,
        };

        let ca_manager = if capture_https {
            match load_ca_manager(config) {
                Ok(m) => {
                    tracing::info!("[ForwardProxy] MITM CA 管理器初始化成功，HTTPS 解密已启用");
                    Some(Arc::new(m))
                }
                Err(e) => {
                    tracing::error!(error = %e, "[ForwardProxy] CA 管理器初始化失败，HTTPS 抓取已禁用");
                    None
                }
            }
        } else {
            tracing::info!("[ForwardProxy] HTTPS 解密未启用");
            None
        };

        let listener = TcpListener::bind(addr).await.map_err(|e| {
            tracing::error!(port, error = %e, "[ForwardProxy] 端口绑定失败");
            e
        })?;
        tracing::info!(%addr, "[ForwardProxy] 监听已启动");

        let ca_manager_clone = ca_manager.clone();
        let mitm_config_clone = mitm_config.clone();

        tokio::spawn(async move {
            tokio::pin!(shutdown_rx);
            let mut conn_count: u64 = 0;
            loop {
                let accept_result = tokio::select! {
                    result = listener.accept() => result,
                    _ = &mut shutdown_rx => {
                        tracing::info!(conn_count, "[ForwardProxy] 收到关闭信号");
                        return;
                    }
                };

                match accept_result {
                    Ok((stream, client_addr)) => {
                        conn_count += 1;
                        tracing::debug!(conn_count, %client_addr, "[ForwardProxy] 新连接");
                        let engine_tx = engine_tx.clone();
                        let ca_manager = ca_manager_clone.clone();
                        let mitm_config = mitm_config_clone.clone();
                        let rule_engine = rule_engine.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_raw_connection(
                                stream, client_addr, engine_tx, max_body_size, ca_manager, &mitm_config, rule_engine,
                            ).await {
                                tracing::debug!(%client_addr, error = %e, "[ForwardProxy] 连接处理错误");
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "[ForwardProxy] Accept 错误");
                    }
                }
            }
        });

        Ok(ForwardProxyHandle { shutdown_tx, ca_manager })
    }
}

fn load_ca_manager(config: &CaptureConfig) -> Result<CaManager, EngineError> {
    if let (Some(cert_path), Some(key_path)) = (&config.ca_cert_path, &config.ca_key_path) {
        let cert_pem = std::fs::read_to_string(cert_path)?;
        let key_pem = std::fs::read_to_string(key_path)?;
        CaManager::from_pem(&cert_pem, &key_pem).map_err(|e| EngineError::CertError(e.to_string()))
    } else {
        let app_data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("FlowReveal");
        CaManager::load_or_create(&app_data_dir).map_err(|e| EngineError::CertError(e.to_string()))
    }
}

async fn handle_raw_connection(
    stream: tokio::net::TcpStream,
    client_addr: SocketAddr,
    engine_tx: mpsc::Sender<HttpMessage>,
    max_body_size: usize,
    ca_manager: Option<Arc<CaManager>>,
    mitm_config: &MitmConfig,
    rule_engine: Arc<RuleEngine>,
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

    let raw_headers = read_headers_from_buf(&mut reader).await?;

    let source_ip = client_addr.ip().to_string();
    let source_port = client_addr.port();
    let timestamp = now_us();
    let session_id = next_session_id();

    let proc_info = crate::platform_integration::windows::find_process_by_connection(&source_ip, source_port);
    tracing::debug!(source_ip, source_port, pid = proc_info.as_ref().map(|p| p.pid), "[ForwardProxy] 进程查找");

    if method.eq_ignore_ascii_case("CONNECT") {
        tracing::info!(target = request_target, source = %source_ip, "[ForwardProxy] CONNECT 请求");
        if let Some(ref ca) = ca_manager {
            let client_stream = reader.into_inner();
            mitm_proxy::handle_mitm_connect(
                client_stream, request_target, &raw_headers, &source_ip,
                engine_tx, ca.clone(), mitm_config, max_body_size, rule_engine, proc_info.as_ref(),
            ).await
        } else {
            tracing::debug!(target = request_target, "[ForwardProxy] CONNECT 隧道模式（无 MITM）");
            handle_connect_tunnel(
                reader, session_id, request_target, &raw_headers,
                &source_ip, timestamp, engine_tx, proc_info.as_ref(),
            ).await
        }
    } else {
        tracing::info!(method, target = request_target, source = %source_ip, "[ForwardProxy] HTTP 请求");
        handle_http_request(
            reader, session_id, method, request_target, raw_headers,
            source_ip, timestamp, engine_tx, max_body_size, rule_engine, proc_info.as_ref(),
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
    proc_info: Option<&crate::process_info::ProcessInfo>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (host, port) = parse_host_port(request_target, 443);

    let req_msg = HttpMessage::request(session_id, HttpProtocol::HTTP1_1, Scheme::Https, "CONNECT", request_target, req_headers.to_vec(), timestamp)
        .process_info(proc_info)
        .source_ip(source_ip)
        .dest_ip(&host)
        .dest_port(port)
        .build();
    let _ = engine_tx.send(req_msg).await;

    let start = std::time::Instant::now();

    let remote_stream = match tokio::net::TcpStream::connect((host.as_str(), port)).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(host, port, error = %e, "[ForwardProxy] CONNECT: 连接上游失败");
            let mut stream = reader.into_inner();
            let _ = stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
            return Ok(());
        }
    };

    let duration_us = start.elapsed().as_micros() as u64;

    let resp_msg = HttpMessage::response(session_id, HttpProtocol::HTTP1_1, Scheme::Https, 200, vec![], now_us(), duration_us)
        .status_text("Connection Established")
        .source_ip(&host)
        .dest_ip(source_ip)
        .source_port(port)
        .build();
    let _ = engine_tx.send(resp_msg).await;

    let mut client_stream = reader.into_inner();
    let _ = client_stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await;

    tracing::debug!(host, port, "[ForwardProxy] CONNECT 隧道建立");

    let (mut cr, mut cw) = client_stream.split();
    let (mut rr, mut rw) = tokio::io::split(remote_stream);

    tokio::select! {
        r = tokio::io::copy(&mut cr, &mut rw) => {
            if let Err(e) = r { tracing::trace!(error = %e, "[ForwardProxy] 隧道 c->r 关闭"); }
        }
        r = tokio::io::copy(&mut rr, &mut cw) => {
            if let Err(e) = r { tracing::trace!(error = %e, "[ForwardProxy] 隧道 r->c 关闭"); }
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
    mut req_headers: Vec<(String, String)>,
    source_ip: String,
    timestamp: u64,
    engine_tx: mpsc::Sender<HttpMessage>,
    max_body_size: usize,
    rule_engine: Arc<RuleEngine>,
    proc_info: Option<&crate::process_info::ProcessInfo>,
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

    let req_body_bytes = read_body_from_buf(&mut reader, content_length, max_body_size).await;
    let (req_body_captured, req_body_truncated) = truncate_body(&req_body_bytes, max_body_size);

    let req_msg = HttpMessage::request(session_id, HttpProtocol::HTTP1_1, Scheme::Http, method, &forward_url, req_headers.clone(), timestamp)
        .process_info(proc_info)
        .source_ip(&source_ip)
        .dest_ip(&host)
        .body(req_body_captured)
        .body_size(content_length)
        .body_truncated(req_body_truncated)
        .content_type(extract_header(&req_headers, "content-type").unwrap_or_default())
        .build();

    if let Some(result) = rule_engine.apply(&req_msg, None).await {
        match result {
            RuleExecutionResult::AutoReply { status_code, status_text, headers, body, delay_ms } => {
                tracing::info!(status_code, delay_ms, "[ForwardProxy] 自动回复规则触发");
                let _ = engine_tx.send(req_msg).await;
                if delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
                let mock_resp = HttpMessage::response(session_id, HttpProtocol::HTTP1_1, Scheme::Http, status_code, headers.clone(), now_us(), 0)
                    .status_text(&status_text)
                    .body(if body.is_empty() { None } else { Some(body.clone()) })
                    .body_size(body.len())
                    .content_type(extract_header(&headers, "content-type").unwrap_or_default())
                    .source_ip(&host)
                    .dest_ip(&source_ip)
                    .build();
                let _ = engine_tx.send(mock_resp).await;
                let mut client_stream = reader.into_inner();
                write_response_to_client(&mut client_stream, status_code, &Some(status_text), &headers, &body).await?;
                return Ok(());
            }
            RuleExecutionResult::HeaderModified { request_actions, response_actions } => {
                tracing::info!(req_actions = request_actions.len(), resp_actions = response_actions.len(), "[ForwardProxy] 消息头修改规则触发");
                RuleExecutor::apply_header_actions(&mut req_headers, &request_actions);
                let _ = engine_tx.send(req_msg).await;
                return handle_http_request_forward(
                    reader, session_id, method, request_target, req_headers,
                    source_ip, forward_url, req_body_bytes, engine_tx, max_body_size,
                    rule_engine, response_actions,
                ).await;
            }
            RuleExecutionResult::Redirected { new_url, redirect_type, preserve_query, preserve_path } => {
                let final_url = RuleExecutor::build_redirect_url(&forward_url, &new_url, preserve_query, preserve_path);
                let redirect_code = match redirect_type {
                    crate::rules::RedirectType::Permanent301 => 301,
                    crate::rules::RedirectType::Temporary302 => 302,
                    crate::rules::RedirectType::Temporary307 => 307,
                    crate::rules::RedirectType::Permanent308 => 308,
                };
                tracing::info!(from = %forward_url, to = %final_url, redirect_code, "[ForwardProxy] 重定向规则触发");
                let _ = engine_tx.send(req_msg).await;
                let redirect_headers = vec![
                    ("Location".to_string(), final_url.clone()),
                    ("Content-Length".to_string(), "0".to_string()),
                ];
                let redirect_resp = HttpMessage::response(session_id, HttpProtocol::HTTP1_1, Scheme::Http, redirect_code, redirect_headers.clone(), now_us(), 0)
                    .source_ip(&host)
                    .dest_ip(&source_ip)
                    .build();
                let _ = engine_tx.send(redirect_resp).await;
                let mut client_stream = reader.into_inner();
                write_response_to_client(&mut client_stream, redirect_code, &None, &redirect_headers, &[]).await?;
                return Ok(());
            }
        }
    }

    let _ = engine_tx.send(req_msg).await;

    handle_http_request_forward(
        reader, session_id, method, request_target, req_headers,
        source_ip, forward_url, req_body_bytes, engine_tx, max_body_size,
        rule_engine, vec![],
    ).await
}

#[allow(clippy::too_many_arguments)]
async fn handle_http_request_forward(
    reader: BufReader<tokio::net::TcpStream>,
    session_id: u64,
    method: &str,
    request_target: &str,
    req_headers: Vec<(String, String)>,
    source_ip: String,
    forward_url: String,
    req_body_bytes: Vec<u8>,
    engine_tx: mpsc::Sender<HttpMessage>,
    max_body_size: usize,
    _rule_engine: Arc<RuleEngine>,
    pending_response_actions: Vec<crate::rules::HeaderAction>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let start = std::time::Instant::now();

    let forward_uri: hyper::Uri = forward_url.parse().unwrap_or_else(|_| {
        format!("http://{}", request_target).parse().unwrap()
    });

    let port = forward_uri.port_u16().unwrap_or(80);
    let forward_host = forward_uri.host().unwrap_or("unknown").to_string();

    let upstream_stream = match tokio::net::TcpStream::connect((forward_host.as_str(), port)).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(host = %forward_host, port, error = %e, "[ForwardProxy] 连接上游失败");
            let mut client_stream = reader.into_inner();
            let body = format!("Failed to connect to upstream: {}", e);
            let resp = format!("HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\n\r\n{}", body.len(), body);
            let _ = client_stream.write_all(resp.as_bytes()).await;
            return Ok(());
        }
    };

    let io = TokioIo::new(upstream_stream);

    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "[ForwardProxy] 上游握手失败");
            let mut client_stream = reader.into_inner();
            let body = format!("Upstream handshake failed: {}", e);
            let resp = format!("HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\n\r\n{}", body.len(), body);
            let _ = client_stream.write_all(resp.as_bytes()).await;
            return Ok(());
        }
    };

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::trace!(error = %e, "[ForwardProxy] 上游连接关闭");
        }
    });

    let forward_req = build_forward_request(method, &forward_uri, &req_headers, &forward_host, req_body_bytes)?;

    let upstream_resp = match sender.send_request(forward_req).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "[ForwardProxy] 转发请求失败");
            let mut client_stream = reader.into_inner();
            let body = format!("Upstream request failed: {}", e);
            let resp = format!("HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\n\r\n{}", body.len(), body);
            let _ = client_stream.write_all(resp.as_bytes()).await;
            return Ok(());
        }
    };

    let duration_us = start.elapsed().as_micros() as u64;
    let status_code = upstream_resp.status().as_u16();
    let status_reason = upstream_resp.status().canonical_reason().map(|s| s.to_string());
    let mut resp_headers: Vec<(String, String)> = upstream_resp
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let resp_body_bytes = upstream_resp.into_body()
        .collect()
        .await
        .map(|b| b.to_bytes().to_vec())
        .unwrap_or_default();
    let resp_body_size = resp_body_bytes.len();
    let (resp_body_captured, resp_body_truncated) = truncate_body(&resp_body_bytes, max_body_size);

    if !pending_response_actions.is_empty() {
        RuleExecutor::apply_header_actions(&mut resp_headers, &pending_response_actions);
        tracing::debug!(actions = pending_response_actions.len(), "[ForwardProxy] 响应头修改规则应用");
    }

    let resp_msg = HttpMessage::response(session_id, HttpProtocol::HTTP1_1, Scheme::Http, status_code, resp_headers.clone(), now_us(), duration_us)
        .status_text(status_reason.clone().unwrap_or_default())
        .body(resp_body_captured)
        .body_size(resp_body_size)
        .body_truncated(resp_body_truncated)
        .content_type(extract_header(&resp_headers, "content-type").unwrap_or_default())
        .source_ip(&forward_host)
        .dest_ip(&source_ip)
        .source_port(port)
        .build();

    let _ = engine_tx.send(resp_msg).await;

    let mut client_stream = reader.into_inner();
    write_response_to_client(&mut client_stream, status_code, &status_reason, &resp_headers, &resp_body_bytes).await?;

    Ok(())
}

fn build_forward_request(
    method: &str,
    uri: &hyper::Uri,
    req_headers: &[(String, String)],
    forward_host: &str,
    body: Vec<u8>,
) -> Result<Request<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    let mut builder = Request::builder().method(method).uri(uri);

    for (key, value) in req_headers {
        if !is_hop_by_hop_header(key) && !key.eq_ignore_ascii_case("host") {
            builder = builder.header(key.as_str(), value.as_str());
        }
    }
    builder = builder.header("Host", forward_host);
    builder = builder.header("Connection", "close");

    Ok(builder.body(Full::new(Bytes::from(body)))?)
}

async fn write_response_to_client(
    client: &mut tokio::net::TcpStream,
    status_code: u16,
    status_reason: &Option<String>,
    resp_headers: &[(String, String)],
    resp_body: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut response_line = format!(
        "HTTP/1.1 {} {}\r\n",
        status_code,
        status_reason.as_deref().unwrap_or("Unknown")
    );
    for (key, value) in resp_headers {
        let kl = key.to_lowercase();
        if kl != "connection" && kl != "transfer-encoding" && kl != "content-length" {
            response_line.push_str(&format!("{}: {}\r\n", key, value));
        }
    }
    response_line.push_str(&format!("Content-Length: {}\r\n", resp_body.len()));
    response_line.push_str("Connection: close\r\n");
    response_line.push_str("\r\n");

    client.write_all(response_line.as_bytes()).await?;
    if !resp_body.is_empty() {
        client.write_all(resp_body).await?;
    }
    let _ = client.shutdown().await;
    Ok(())
}
