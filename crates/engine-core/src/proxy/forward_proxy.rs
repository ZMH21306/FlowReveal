use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::pin::pin;

use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request, Response, Method, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};

use crate::capture_config::CaptureConfig;
use crate::engine_error::EngineError;
use crate::http_message::{HttpMessage, HttpProtocol, MessageDirection, Scheme};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

pub struct ForwardProxy {
    addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    config: CaptureConfig,
}

impl ForwardProxy {
    pub fn new(port: u16, config: CaptureConfig) -> Self {
        Self {
            addr: SocketAddr::from(([127, 0, 0, 1], port)),
            shutdown_tx: None,
            config,
        }
    }

    pub async fn start(&mut self, engine_tx: mpsc::Sender<HttpMessage>) -> Result<(), EngineError> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        let listener = TcpListener::bind(self.addr).await?;
        let max_body_size = self.config.max_body_size;

        tracing::info!("Forward proxy listening on {}", self.addr);

        tokio::spawn(async move {
            let mut shutdown_rx = pin!(shutdown_rx);
            loop {
                let accept_result = tokio::select! {
                    result = listener.accept() => result,
                    _ = &mut shutdown_rx => {
                        tracing::info!("Forward proxy shutting down");
                        return;
                    }
                };

                match accept_result {
                    Ok((stream, _remote_addr)) => {
                        let io = TokioIo::new(stream);
                        let engine_tx = engine_tx.clone();

                        tokio::spawn(async move {
                            let service = service_fn(move |req| {
                                handle_request(req, engine_tx.clone(), max_body_size)
                            });

                            if let Err(e) = hyper::server::conn::http1::Builder::new()
                                .serve_connection(io, service)
                                .await
                            {
                                tracing::debug!("Connection error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Accept error: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), EngineError> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        Ok(())
    }
}

fn extract_content_type(headers: &[(String, String)]) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.clone())
}

async fn handle_request(
    req: Request<Incoming>,
    engine_tx: mpsc::Sender<HttpMessage>,
    _max_body_size: usize,
) -> Result<Response<String>, hyper::Error> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let req_headers: Vec<(String, String)> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let host = uri.host().unwrap_or("unknown").to_string();
    let path = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64;

    let session_id = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);

    if method == Method::CONNECT {
        let msg = HttpMessage {
            id: session_id,
            session_id,
            direction: MessageDirection::Request,
            protocol: HttpProtocol::HTTP1_1,
            scheme: Scheme::Https,
            method: Some("CONNECT".to_string()),
            url: Some(uri.to_string()),
            status_code: None,
            status_text: None,
            headers: req_headers,
            body: None,
            body_size: 0,
            body_truncated: false,
            content_type: None,
            process_name: None,
            process_id: None,
            process_path: None,
            source_ip: None,
            dest_ip: Some(host.clone()),
            source_port: None,
            dest_port: uri.port_u16(),
            timestamp,
            duration_us: None,
            cookies: vec![],
            raw_tls_info: None,
        };

        let _ = engine_tx.send(msg).await;

        return Ok(Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body("HTTPS tunneling not yet supported in this version".to_string())
            .unwrap());
    }

    let forward_url = if uri.scheme_str().is_some() {
        uri.to_string()
    } else {
        format!("http://{}{}", host, path)
    };

    let start = std::time::Instant::now();

    let forward_uri: hyper::Uri = forward_url.parse().unwrap_or_else(|_| {
        format!("http://{}{}", host, path).parse().unwrap()
    });

    let port = forward_uri.port_u16().unwrap_or(80);
    let forward_host = forward_uri.host().unwrap_or("unknown").to_string();

    let stream = match tokio::net::TcpStream::connect((forward_host.as_str(), port)).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Failed to connect to {}: {}", forward_host, e);
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(format!("Failed to connect to upstream: {}", e))
                .unwrap());
        }
    };

    let io = TokioIo::new(stream);

    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Handshake failed: {}", e);
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(format!("Upstream handshake failed: {}", e))
                .unwrap());
        }
    };

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::debug!("Upstream connection error: {}", e);
        }
    });

    let mut forward_req = Request::builder()
        .method(method.clone())
        .uri(forward_uri);

    for (key, value) in &req_headers {
        if key.to_lowercase() != "host" && key.to_lowercase() != "connection" {
            forward_req = forward_req.header(key.as_str(), value.as_str());
        }
    }
    forward_req = forward_req.header("Host", &forward_host);
    forward_req = forward_req.header("Connection", "close");

    let forward_req = forward_req.body(String::new()).unwrap();

    let resp = match sender.send_request(forward_req).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Forward request failed: {}", e);
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(format!("Upstream request failed: {}", e))
                .unwrap());
        }
    };

    let duration_us = start.elapsed().as_micros() as u64;
    let status_code = resp.status().as_u16();
    let resp_headers: Vec<(String, String)> = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let content_type = extract_content_type(&resp_headers);

    let mut response = Response::builder().status(status_code);
    for (key, value) in &resp_headers {
        if key.to_lowercase() != "connection" && key.to_lowercase() != "transfer-encoding" {
            response = response.header(key.as_str(), value.as_str());
        }
    }

    let msg = HttpMessage {
        id: session_id,
        session_id,
        direction: MessageDirection::Request,
        protocol: HttpProtocol::HTTP1_1,
        scheme: Scheme::Http,
        method: Some(method.to_string()),
        url: Some(forward_url),
        status_code: Some(status_code),
        status_text: None,
        headers: resp_headers,
        body: None,
        body_size: 0,
        body_truncated: false,
        content_type,
        process_name: None,
        process_id: None,
        process_path: None,
        source_ip: None,
        dest_ip: Some(forward_host),
        source_port: None,
        dest_port: Some(port),
        timestamp,
        duration_us: Some(duration_us),
        cookies: vec![],
        raw_tls_info: None,
    };

    let _ = engine_tx.send(msg).await;

    Ok(response.body("OK".to_string()).unwrap())
}
