use std::net::SocketAddr;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};

use crate::capture_config::CaptureConfig;
use crate::engine_error::EngineError;
use crate::http_message::{HttpMessage, HttpProtocol, Scheme};
use crate::proxy::utils::{extract_header, now_us, next_session_id, read_headers_from_buf, read_body_from_buf, truncate_body};

pub struct TransparentProxyHandle {
    pub shutdown_tx: oneshot::Sender<()>,
}

pub struct TransparentProxy;

impl TransparentProxy {
    pub async fn start(
        port: u16,
        config: &CaptureConfig,
        engine_tx: mpsc::Sender<HttpMessage>,
    ) -> Result<TransparentProxyHandle, EngineError> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let max_body_size = config.max_body_size;

        tracing::info!(port, "[TransparentProxy] 初始化中");

        let listener = TcpListener::bind(addr).await.map_err(|e| {
            tracing::error!(port, error = %e, "[TransparentProxy] 端口绑定失败");
            e
        })?;
        tracing::info!(%addr, "[TransparentProxy] 监听已启动");

        tokio::spawn(async move {
            tokio::pin!(shutdown_rx);
            let mut conn_count: u64 = 0;
            loop {
                let accept_result = tokio::select! {
                    result = listener.accept() => result,
                    _ = &mut shutdown_rx => {
                        tracing::info!(conn_count, "[TransparentProxy] 收到关闭信号");
                        return;
                    }
                };

                match accept_result {
                    Ok((stream, client_addr)) => {
                        conn_count += 1;
                        tracing::trace!(conn_count, %client_addr, "[TransparentProxy] 新连接");
                        let engine_tx = engine_tx.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_transparent_connection(
                                stream, client_addr, engine_tx, max_body_size,
                            ).await {
                                tracing::debug!(%client_addr, error = %e, "[TransparentProxy] 连接处理错误");
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "[TransparentProxy] Accept 错误");
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

    let host = extract_header(&raw_headers, "host").unwrap_or_else(|| "unknown".to_string());
    let dest_port: u16 = extract_header(&raw_headers, "host")
        .and_then(|h| h.split(':').last().and_then(|p| p.parse().ok()))
        .unwrap_or(80);

    let content_length: usize = extract_header(&raw_headers, "content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let req_body_bytes = read_body_from_buf(&mut reader, content_length, max_body_size).await;
    let (req_body_captured, req_body_truncated) = truncate_body(&req_body_bytes, max_body_size);

    let req_msg = HttpMessage::request(session_id, HttpProtocol::HTTP1_1, Scheme::Http, method, request_target, raw_headers.clone(), timestamp)
        .process_info(proc_info.as_ref())
        .source_ip(&source_ip)
        .dest_ip(&host)
        .source_port(source_port)
        .dest_port(dest_port)
        .body(req_body_captured)
        .body_size(content_length)
        .body_truncated(req_body_truncated)
        .content_type(extract_header(&raw_headers, "content-type").unwrap_or_default())
        .build();

    let _ = engine_tx.send(req_msg).await;

    tracing::debug!(method, host, source = %source_ip, "[TransparentProxy] 请求已记录");

    let mut client_stream = reader.into_inner();
    let _ = client_stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;

    Ok(())
}
