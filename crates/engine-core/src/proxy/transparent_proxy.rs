use std::net::SocketAddr;
use std::sync::Arc;

use hyper::Request;
use hyper_util::rt::TokioIo;
use http_body_util::{BodyExt, Full};
use bytes::Bytes;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};

use crate::capture_config::CaptureConfig;
use crate::divert::nat_table::NatTable;
use crate::divert::sni_parser::extract_sni_from_client_hello;
use crate::engine_error::EngineError;
use crate::http_message::{HttpMessage, HttpProtocol, Scheme, TlsInfo};
use crate::mitm::{CaManager, MitmConfig, build_tls_client_config, build_tls_server_config, pem_to_der, private_key_pem_to_der};
use crate::protocol::websocket;
use crate::proxy::utils::{extract_header, now_us, truncate_body, is_hop_by_hop_header, next_session_id, read_headers_from_buf, read_body_from_buf};
use crate::rules::{RuleEngine, RuleExecutionResult};

pub struct TransparentProxyHandle {
    pub shutdown_tx: oneshot::Sender<()>,
}

pub struct TransparentProxy;

impl TransparentProxy {
    pub async fn start(
        port: u16,
        config: &CaptureConfig,
        engine_tx: mpsc::Sender<HttpMessage>,
        nat_table: Arc<NatTable>,
        ca_manager: Option<Arc<CaManager>>,
        rule_engine: Arc<RuleEngine>,
    ) -> Result<TransparentProxyHandle, EngineError> {
        let addr_v4 = SocketAddr::from(([127, 0, 0, 1], port));
        let addr_v6 = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], port));

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let max_body_size = config.max_body_size;
        let capture_https = config.capture_https;

        let mitm_config = MitmConfig {
            enabled: capture_https,
            bypass_hosts: config.mitm_bypass_hosts.clone(),
            bypass_port_ranges: vec![],
            max_cert_cache_entries: 1024,
            cert_validity_days: 365,
        };

        tracing::info!(port, capture_https, "[TransparentProxy] 初始化中");

        let listener_v4 = match TcpListener::bind(addr_v4).await {
            Ok(l) => {
                tracing::info!(%addr_v4, "[TransparentProxy] IPv4 监听已启动");
                Some(l)
            }
            Err(e) => {
                tracing::warn!(%addr_v4, error = %e, "[TransparentProxy] IPv4 端口绑定失败");
                None
            }
        };

        let listener_v6 = match TcpListener::bind(addr_v6).await {
            Ok(l) => {
                tracing::info!(%addr_v6, "[TransparentProxy] IPv6 监听已启动");
                Some(l)
            }
            Err(e) => {
                tracing::warn!(%addr_v6, error = %e, "[TransparentProxy] IPv6 端口绑定失败");
                None
            }
        };

        if listener_v4.is_none() && listener_v6.is_none() {
            return Err(EngineError::Io(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                format!("Transparent proxy port {} bind failed for both IPv4 and IPv6", port),
            )));
        }

        tokio::spawn(async move {
            tokio::pin!(shutdown_rx);
            let mut conn_count: u64 = 0;

            loop {
                let accept_v4 = async {
                    match &listener_v4 {
                        Some(l) => l.accept().await.map(|(s, a)| (s, a)),
                        None => std::future::pending().await,
                    }
                };
                let accept_v6 = async {
                    match &listener_v6 {
                        Some(l) => l.accept().await.map(|(s, a)| (s, a)),
                        None => std::future::pending().await,
                    }
                };

                let accept_result = tokio::select! {
                    result = accept_v4 => Some(result),
                    result = accept_v6 => Some(result),
                    _ = &mut shutdown_rx => {
                        tracing::info!(conn_count, "[TransparentProxy] 收到关闭信号");
                        return;
                    }
                };

                match accept_result {
                    Some(Ok((stream, client_addr))) => {
                        conn_count += 1;
                        tracing::trace!(conn_count, %client_addr, "[TransparentProxy] 新连接");
                        let engine_tx = engine_tx.clone();
                        let nat_table = nat_table.clone();
                        let ca_manager = ca_manager.clone();
                        let mitm_config = mitm_config.clone();
                        let rule_engine = rule_engine.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_transparent_connection(
                                stream, client_addr, engine_tx, max_body_size,
                                nat_table, ca_manager, &mitm_config, rule_engine,
                            ).await {
                                tracing::debug!(%client_addr, error = %e, "[TransparentProxy] 连接处理错误");
                            }
                        });
                    }
                    Some(Err(e)) => {
                        tracing::error!(error = %e, "[TransparentProxy] Accept 错误");
                    }
                    None => {
                        tracing::warn!("[TransparentProxy] 所有监听器已关闭");
                        return;
                    }
                }
            }
        });

        Ok(TransparentProxyHandle { shutdown_tx })
    }
}

async fn handle_transparent_connection(
    stream: tokio::net::TcpStream,
    client_addr: SocketAddr,
    engine_tx: mpsc::Sender<HttpMessage>,
    max_body_size: usize,
    nat_table: Arc<NatTable>,
    ca_manager: Option<Arc<CaManager>>,
    mitm_config: &MitmConfig,
    rule_engine: Arc<RuleEngine>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let original_dest = nat_table.get_original_dest(client_addr.ip(), client_addr.port());

    let (dest_ip, dest_port) = match original_dest {
        Some(dest) => (dest.ip.to_string(), dest.port),
        None => {
            tracing::debug!(%client_addr, "[TransparentProxy] 无 NAT 映射，回退到 Host 头解析");
            handle_fallback_no_nat(stream, client_addr, engine_tx, max_body_size, rule_engine).await?;
            return Ok(());
        }
    };

    let proc_info = crate::platform_integration::windows::find_process_by_connection(
        &client_addr.ip().to_string(), client_addr.port(),
    );

    tracing::debug!(%client_addr, %dest_ip, dest_port, "[TransparentProxy] 原始目标");

    match dest_port {
        80 => {
            handle_http_transparent(
                stream, client_addr, &dest_ip, dest_port,
                engine_tx, max_body_size, rule_engine, proc_info.as_ref(),
            ).await
        }
        443 => {
            handle_https_transparent(
                stream, client_addr, &dest_ip, dest_port,
                engine_tx, max_body_size, ca_manager, mitm_config, rule_engine, proc_info.as_ref(),
            ).await
        }
        _ => {
            handle_tunnel_transparent(
                stream, &dest_ip, dest_port,
                engine_tx, proc_info.as_ref(),
            ).await
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_http_transparent(
    stream: tokio::net::TcpStream,
    client_addr: SocketAddr,
    dest_ip: &str,
    dest_port: u16,
    engine_tx: mpsc::Sender<HttpMessage>,
    max_body_size: usize,
    rule_engine: Arc<RuleEngine>,
    proc_info: Option<&crate::process_info::ProcessInfo>,
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

    let host = extract_header(&raw_headers, "host").unwrap_or_else(|| dest_ip.to_string());
    let forward_url = if request_target.starts_with("http://") || request_target.starts_with("https://") {
        request_target.to_string()
    } else {
        format!("http://{}:{}", host, dest_port)
    };

    let source_ip = client_addr.ip().to_string();
    let source_port = client_addr.port();
    let timestamp = now_us();
    let session_id = next_session_id();

    let content_length: usize = extract_header(&raw_headers, "content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let req_body_bytes = read_body_from_buf(&mut reader, content_length, max_body_size).await;
    let (req_body_captured, req_body_truncated) = truncate_body(&req_body_bytes, max_body_size);

    let req_msg = HttpMessage::request(session_id, HttpProtocol::HTTP1_1, Scheme::Http, method, &forward_url, raw_headers.clone(), timestamp)
        .process_info(proc_info)
        .source_ip(&source_ip)
        .dest_ip(&host)
        .source_port(source_port)
        .dest_port(dest_port)
        .body(req_body_captured)
        .body_size(content_length)
        .body_truncated(req_body_truncated)
        .content_type(extract_header(&raw_headers, "content-type").unwrap_or_default())
        .build();

    if let Some(result) = rule_engine.apply(&req_msg, None).await {
        match result {
            RuleExecutionResult::AutoReply { status_code, status_text, headers, body, delay_ms } => {
                tracing::info!(status_code, "[TransparentProxy] 自动回复规则触发");
                let _ = engine_tx.send(req_msg).await;
                if delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
                let mut client_stream = reader.into_inner();
                write_response_to_client(&mut client_stream, status_code, &Some(status_text), &headers, &body).await?;
                return Ok(());
            }
            RuleExecutionResult::Redirected { new_url, redirect_type, .. } => {
                let redirect_code = match redirect_type {
                    crate::rules::RedirectType::Permanent301 => 301,
                    crate::rules::RedirectType::Temporary302 => 302,
                    crate::rules::RedirectType::Temporary307 => 307,
                    crate::rules::RedirectType::Permanent308 => 308,
                };
                let _ = engine_tx.send(req_msg).await;
                let redirect_headers = vec![
                    ("Location".to_string(), new_url.clone()),
                    ("Content-Length".to_string(), "0".to_string()),
                ];
                let mut client_stream = reader.into_inner();
                write_response_to_client(&mut client_stream, redirect_code, &None, &redirect_headers, &[]).await?;
                return Ok(());
            }
            RuleExecutionResult::HeaderModified { .. } => {}
        }
    }

    let _ = engine_tx.send(req_msg).await;

    let start = std::time::Instant::now();

    let forward_host = host.clone();
    let forward_port = extract_header(&raw_headers, "host")
        .and_then(|h| h.split(':').last().and_then(|p| p.parse().ok()))
        .unwrap_or(dest_port);

    let upstream_stream = match tokio::net::TcpStream::connect((forward_host.as_str(), forward_port)).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(host = %forward_host, forward_port, error = %e, "[TransparentProxy] 连接上游失败");
            let mut client_stream = reader.into_inner();
            let body = format!("Failed to connect to upstream: {}", e);
            write_response_to_client(&mut client_stream, 502, &Some("Bad Gateway".to_string()), &[], body.as_bytes()).await?;
            return Ok(());
        }
    };

    let io = TokioIo::new(upstream_stream);
    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "[TransparentProxy] 上游握手失败");
            let mut client_stream = reader.into_inner();
            write_response_to_client(&mut client_stream, 502, &Some("Bad Gateway".to_string()), &[], b"Upstream handshake failed").await?;
            return Ok(());
        }
    };

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::trace!(error = %e, "[TransparentProxy] 上游连接关闭");
        }
    });

    let forward_uri: hyper::Uri = forward_url.parse().unwrap_or_else(|_| format!("http://{}:{}", dest_ip, dest_port).parse().unwrap());
    let mut forward_req_builder = Request::builder().method(method).uri(&forward_uri);
    for (key, value) in &raw_headers {
        if !is_hop_by_hop_header(key) && !key.eq_ignore_ascii_case("host") {
            forward_req_builder = forward_req_builder.header(key.as_str(), value.as_str());
        }
    }
    forward_req_builder = forward_req_builder.header("Host", &forward_host);
    forward_req_builder = forward_req_builder.header("Connection", "close");

    let forward_req = forward_req_builder.body(Full::new(Bytes::from(req_body_bytes)))?;

    let upstream_resp = match sender.send_request(forward_req).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "[TransparentProxy] 转发请求失败");
            let mut client_stream = reader.into_inner();
            write_response_to_client(&mut client_stream, 502, &Some("Bad Gateway".to_string()), &[], b"Upstream request failed").await?;
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

    let resp_body_bytes = upstream_resp.into_body()
        .collect()
        .await
        .map(|b| b.to_bytes().to_vec())
        .unwrap_or_default();
    let resp_body_size = resp_body_bytes.len();
    let (resp_body_captured, resp_body_truncated) = truncate_body(&resp_body_bytes, max_body_size);

    let resp_msg = HttpMessage::response(session_id, HttpProtocol::HTTP1_1, Scheme::Http, status_code, resp_headers.clone(), now_us(), duration_us)
        .status_text(status_reason.clone().unwrap_or_default())
        .body(resp_body_captured)
        .body_size(resp_body_size)
        .body_truncated(resp_body_truncated)
        .content_type(extract_header(&resp_headers, "content-type").unwrap_or_default())
        .source_ip(&forward_host)
        .dest_ip(&source_ip)
        .source_port(forward_port)
        .build();

    let _ = engine_tx.send(resp_msg).await;

    let mut client_stream = reader.into_inner();
    write_response_to_client(&mut client_stream, status_code, &status_reason, &resp_headers, &resp_body_bytes).await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_https_transparent(
    stream: tokio::net::TcpStream,
    client_addr: SocketAddr,
    dest_ip: &str,
    dest_port: u16,
    engine_tx: mpsc::Sender<HttpMessage>,
    max_body_size: usize,
    ca_manager: Option<Arc<CaManager>>,
    mitm_config: &MitmConfig,
    rule_engine: Arc<RuleEngine>,
    proc_info: Option<&crate::process_info::ProcessInfo>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ca = match ca_manager {
        Some(ref ca) => ca.clone(),
        None => {
            tracing::debug!(dest_ip, "[TransparentProxy] 无 CA 管理器，回退到隧道模式");
            return handle_tunnel_transparent(stream, dest_ip, dest_port, engine_tx, proc_info).await;
        }
    };

    if mitm_config.should_bypass(dest_ip) {
        tracing::debug!(dest_ip, "[TransparentProxy] MITM 绕过，回退到隧道模式");
        return handle_tunnel_transparent(stream, dest_ip, dest_port, engine_tx, proc_info).await;
    }

    let mut peek_buf = [0u8; 4096];
    let (n, stream) = {
        let mut reader = BufReader::new(stream);
        let n = reader.read(&mut peek_buf).await?;
        (n, reader.into_inner())
    };

    if n < 5 || peek_buf[0] != 0x16 {
        tracing::debug!(dest_ip, "[TransparentProxy] 非 TLS 流量，回退到隧道模式");
        return handle_tunnel_transparent(stream, dest_ip, dest_port, engine_tx, proc_info).await;
    }

    let sni_hostname = extract_sni_from_client_hello(&peek_buf[..n])
        .unwrap_or_else(|| dest_ip.to_string());

    tracing::info!(host = %sni_hostname, %dest_ip, "[TransparentProxy] HTTPS MITM 拦截");

    let host_cert = match ca.get_or_generate_cert(&sni_hostname).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(host = %sni_hostname, error = %e, "[TransparentProxy] 生成证书失败");
            return handle_tunnel_transparent(stream, dest_ip, dest_port, engine_tx, proc_info).await;
        }
    };

    let cert_der = pem_to_der(&host_cert.cert_pem)?;
    let key_der = private_key_pem_to_der(&host_cert.key_pem)?;

    let server_config = match build_tls_server_config(cert_der, key_der) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(host = %sni_hostname, error = %e, "[TransparentProxy] 构建 TLS 服务端配置失败");
            return handle_tunnel_transparent(stream, dest_ip, dest_port, engine_tx, proc_info).await;
        }
    };

    let tls_acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));
    let tls_client = match tls_acceptor.accept(stream).await {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!(host = %sni_hostname, error = %e, "[TransparentProxy] TLS 握手失败");
            return Ok(());
        }
    };

    tracing::debug!(host = %sni_hostname, "[TransparentProxy] TLS 握手完成");

    let (tls_read, tls_write) = tokio::io::split(tls_client);
    let mut reader = BufReader::new(tls_read);

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

    let req_host = extract_header(&raw_headers, "host").unwrap_or_else(|| sni_hostname.clone());
    let forward_url = if request_target.starts_with("http://") || request_target.starts_with("https://") {
        request_target.to_string()
    } else {
        format!("https://{}{}", req_host, request_target)
    };

    let source_ip = client_addr.ip().to_string();
    let timestamp = now_us();
    let session_id = next_session_id();

    let content_length: usize = extract_header(&raw_headers, "content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let req_body_bytes = read_body_from_buf(&mut reader, content_length, max_body_size).await;
    let (req_body_captured, req_body_truncated) = truncate_body(&req_body_bytes, max_body_size);

    let is_ws_upgrade = websocket::is_websocket_upgrade(&raw_headers);

    let tls_info = TlsInfo {
        version: "TLS 1.3".to_string(),
        cipher_suite: "AES_256_GCM_SHA384".to_string(),
        server_name: Some(sni_hostname.clone()),
        cert_chain: vec![],
    };

    let protocol = if is_ws_upgrade { HttpProtocol::WebSocket } else { HttpProtocol::HTTP1_1 };

    let req_msg = HttpMessage::request(session_id, protocol, Scheme::Https, method, &forward_url, raw_headers.clone(), timestamp)
        .process_info(proc_info)
        .source_ip(&source_ip)
        .dest_ip(&req_host)
        .dest_port(dest_port)
        .body(req_body_captured)
        .body_size(content_length)
        .body_truncated(req_body_truncated)
        .content_type(extract_header(&raw_headers, "content-type").unwrap_or_default())
        .raw_tls_info(tls_info.clone())
        .build();

    let _ = engine_tx.send(req_msg).await;

    let tls_read = reader.into_inner();
    let mut tls_client = tls_read.unsplit(tls_write);

    if is_ws_upgrade {
        tracing::info!(url = %forward_url, "[TransparentProxy] WebSocket Upgrade 检测到");
        let _ = tls_client.write_all(b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n").await;
        let (ws_frame_tx, _) = mpsc::channel::<crate::http_message::WebSocketFrame>(1024);
        let ws_result = websocket::handle_websocket_proxy(
            tls_client, sni_hostname, dest_port, source_ip, session_id, engine_tx, ws_frame_tx, max_body_size,
        ).await;
        if let Err(e) = ws_result {
            tracing::warn!(error = %e, "[TransparentProxy] WebSocket 代理错误");
        }
        return Ok(());
    }

    if let Some(result) = rule_engine.apply(&HttpMessage::request(session_id, HttpProtocol::HTTP1_1, Scheme::Https, method, &forward_url, raw_headers.clone(), now_us())
        .process_info(proc_info)
        .source_ip(&source_ip)
        .dest_ip(&req_host)
        .dest_port(dest_port)
        .build(), None).await {
        match result {
            RuleExecutionResult::AutoReply { status_code, status_text, headers, body, delay_ms } => {
                if delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
                write_raw_tls_response(&mut tls_client, status_code, &status_text, &headers, &body).await?;
                return Ok(());
            }
            RuleExecutionResult::Redirected { new_url, redirect_type, .. } => {
                let redirect_code = match redirect_type {
                    crate::rules::RedirectType::Permanent301 => 301,
                    crate::rules::RedirectType::Temporary302 => 302,
                    crate::rules::RedirectType::Temporary307 => 307,
                    crate::rules::RedirectType::Permanent308 => 308,
                };
                let redirect_resp = format!(
                    "HTTP/1.1 {} Found\r\nLocation: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    redirect_code, new_url
                );
                tls_client.write_all(redirect_resp.as_bytes()).await?;
                tls_client.flush().await.ok();
                return Ok(());
            }
            RuleExecutionResult::HeaderModified { .. } => {}
        }
    }

    forward_https_transparent(
        tls_client, session_id, method, request_target,
        &raw_headers, &req_body_bytes, &sni_hostname, dest_port,
        &source_ip, &forward_url, engine_tx, max_body_size, tls_info,
    ).await
}

#[allow(clippy::too_many_arguments)]
async fn forward_https_transparent(
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

    let remote_tcp = match tokio::net::TcpStream::connect((forward_host.as_str(), forward_port)).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(host, error = %e, "[TransparentProxy] 上游连接失败");
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
            tracing::warn!(host, error = %e, "[TransparentProxy] 上游 TLS 握手失败");
            let _ = tls_client.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n").await;
            return Ok(());
        }
    };

    let io = TokioIo::new(tls_remote);

    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(host, error = %e, "[TransparentProxy] 上游 HTTP 握手失败");
            let _ = tls_client.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n").await;
            return Ok(());
        }
    };

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::trace!(error = %e, "[TransparentProxy] 上游连接关闭");
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
            tracing::warn!(host, error = %e, "[TransparentProxy] 转发请求失败");
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
        .content_type(extract_header(&resp_headers, "content-type").unwrap_or_default())
        .source_ip(&forward_host)
        .dest_ip(source_ip)
        .source_port(port)
        .raw_tls_info(tls_info)
        .build();

    let _ = engine_tx.send(resp_msg).await;

    write_raw_tls_response(&mut tls_client, status_code, &status_reason.unwrap_or_else(|| "Unknown".to_string()), &resp_headers, &resp_body_bytes).await?;

    tracing::info!(host, status_code, "[TransparentProxy] HTTPS 拦截完成");
    Ok(())
}

async fn handle_tunnel_transparent(
    stream: tokio::net::TcpStream,
    dest_ip: &str,
    dest_port: u16,
    engine_tx: mpsc::Sender<HttpMessage>,
    proc_info: Option<&crate::process_info::ProcessInfo>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let session_id = next_session_id();
    let timestamp = now_us();

    let req_msg = HttpMessage::request(session_id, HttpProtocol::HTTP1_1, Scheme::Https, "TUNNEL", format!("{}:{}", dest_ip, dest_port), vec![], timestamp)
        .process_info(proc_info)
        .dest_ip(dest_ip)
        .dest_port(dest_port)
        .build();
    let _ = engine_tx.send(req_msg).await;

    let remote_stream = match tokio::net::TcpStream::connect((dest_ip, dest_port)).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(dest_ip, dest_port, error = %e, "[TransparentProxy] 隧道: 连接上游失败");
            return Ok(());
        }
    };

    let start = std::time::Instant::now();
    let duration_us = start.elapsed().as_micros() as u64;

    let resp_msg = HttpMessage::response(session_id, HttpProtocol::HTTP1_1, Scheme::Https, 200, vec![], now_us(), duration_us)
        .status_text("Tunnel Established")
        .source_ip(dest_ip)
        .dest_port(dest_port)
        .build();
    let _ = engine_tx.send(resp_msg).await;

    let (mut cr, mut cw) = tokio::io::split(stream);
    let (mut rr, mut rw) = tokio::io::split(remote_stream);

    tokio::select! {
        r = tokio::io::copy(&mut cr, &mut rw) => {
            if let Err(e) = r { tracing::trace!(error = %e, "[TransparentProxy] 隧道 c->r 关闭"); }
        }
        r = tokio::io::copy(&mut rr, &mut cw) => {
            if let Err(e) = r { tracing::trace!(error = %e, "[TransparentProxy] 隧道 r->c 关闭"); }
        }
    }

    Ok(())
}

async fn handle_fallback_no_nat(
    stream: tokio::net::TcpStream,
    client_addr: SocketAddr,
    engine_tx: mpsc::Sender<HttpMessage>,
    max_body_size: usize,
    _rule_engine: Arc<RuleEngine>,
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

    let host = extract_header(&raw_headers, "host").unwrap_or_else(|| "unknown".to_string());
    let dest_port: u16 = extract_header(&raw_headers, "host")
        .and_then(|h| h.split(':').last().and_then(|p| p.parse().ok()))
        .unwrap_or(80);

    let source_ip = client_addr.ip().to_string();
    let source_port = client_addr.port();
    let timestamp = now_us();
    let session_id = next_session_id();

    let proc_info = crate::platform_integration::windows::find_process_by_connection(&source_ip, source_port);

    let content_length: usize = extract_header(&raw_headers, "content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let req_body_bytes = read_body_from_buf(&mut reader, content_length, max_body_size).await;
    let (req_body_captured, req_body_truncated) = truncate_body(&req_body_bytes, max_body_size);

    let forward_url = if request_target.starts_with("http://") || request_target.starts_with("https://") {
        request_target.to_string()
    } else {
        format!("http://{}{}", host, request_target)
    };

    let req_msg = HttpMessage::request(session_id, HttpProtocol::HTTP1_1, Scheme::Http, method, &forward_url, raw_headers, timestamp)
        .process_info(proc_info.as_ref())
        .source_ip(&source_ip)
        .dest_ip(&host)
        .source_port(source_port)
        .dest_port(dest_port)
        .body(req_body_captured)
        .body_size(content_length)
        .body_truncated(req_body_truncated)
        .build();

    let _ = engine_tx.send(req_msg).await;

    let mut client_stream = reader.into_inner();
    let _ = client_stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;

    Ok(())
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

async fn write_raw_tls_response(
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
