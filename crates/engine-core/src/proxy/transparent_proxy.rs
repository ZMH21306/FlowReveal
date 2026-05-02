use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::capture_config::CaptureConfig;
use crate::engine_error::EngineError;
use crate::http_message::{HttpMessage, HttpProtocol, MessageDirection, Scheme};

static TRANSPARENT_SESSION_COUNTER: AtomicU64 = AtomicU64::new(100000);

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
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let max_body_size = config.max_body_size;
        let pid_map: Arc<Mutex<PidMap>> = Arc::new(Mutex::new(PidMap::new()));

        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).await?;
        tracing::info!("Transparent proxy listening on 127.0.0.1:{}", port);

        #[cfg(target_os = "windows")]
        {
            match wfp_engine::install_redirect_filters(port) {
                Ok(()) => tracing::info!("WFP redirect filters installed successfully"),
                Err(e) => tracing::error!("WFP filter installation failed: {} - transparent proxy may not intercept traffic", e),
            }
        }

        let pid_map_clone = pid_map.clone();

        tokio::spawn(async move {
            let shutdown_rx = shutdown_rx;
            tokio::pin!(shutdown_rx);
            loop {
                let accept_result = tokio::select! {
                    result = listener.accept() => result,
                    _ = &mut shutdown_rx => {
                        tracing::info!("Transparent proxy shutting down");
                        return;
                    }
                };
                match accept_result {
                    Ok((stream, client_addr)) => {
                        let engine_tx = engine_tx.clone();
                        let pid_map = pid_map_clone.clone();
                        tokio::spawn(async move {
                            if let Err(e) =
                                handle_transparent_connection(stream, client_addr, engine_tx, max_body_size, pid_map).await
                            {
                                tracing::debug!("Transparent connection error from {}: {}", client_addr, e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Transparent proxy accept error: {}", e);
                    }
                }
            }
        });

        Ok(TransparentProxyHandle { shutdown_tx })
    }
}

struct PidMap {
    entries: Vec<PidMapEntry>,
}

#[allow(dead_code)]
struct PidMapEntry {
    source_port: u16,
    #[allow(dead_code)]
    dest_ip: String,
    #[allow(dead_code)]
    dest_port: u16,
    pid: u32,
    process_name: String,
}

impl PidMap {
    fn new() -> Self {
        Self { entries: Vec::new() }
    }

    fn lookup(&self, source_port: u16) -> Option<(u32, String)> {
        self.entries
            .iter()
            .find(|e| e.source_port == source_port)
            .map(|e| (e.pid, e.process_name.clone()))
    }

    #[allow(dead_code)]
    fn insert(&mut self, source_port: u16, dest_ip: String, dest_port: u16, pid: u32, process_name: String) {
        self.entries.retain(|e| e.source_port != source_port);
        self.entries.push(PidMapEntry {
            source_port,
            dest_ip,
            dest_port,
            pid,
            process_name,
        });
        if self.entries.len() > 65536 {
            self.entries.drain(..16384);
        }
    }
}

async fn handle_transparent_connection(
    stream: tokio::net::TcpStream,
    client_addr: SocketAddr,
    engine_tx: mpsc::Sender<HttpMessage>,
    max_body_size: usize,
    pid_map: Arc<Mutex<PidMap>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let original_dest = get_original_destination(&stream).await;

    let (dest_ip, dest_port) = match original_dest {
        Some(addr) => (addr.ip().to_string(), addr.port()),
        None => {
            tracing::debug!("Could not determine original destination for {}", client_addr);
            return Ok(());
        }
    };

    let (process_id, process_name) = {
        let map = pid_map.lock().await;
        map.lookup(client_addr.port()).unwrap_or((0, "-".to_string()))
    };

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
    let session_id = TRANSPARENT_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);

    let host_header = extract_header(&raw_headers, "host");
    let scheme = if dest_port == 443 { Scheme::Https } else { Scheme::Http };

    let forward_url = if request_target.starts_with("http://") || request_target.starts_with("https://") {
        request_target.to_string()
    } else {
        let host = host_header.as_deref().unwrap_or(&dest_ip);
        format!("{}://{}{}", scheme, host, request_target)
    };

    let content_length: usize = extract_header(&raw_headers, "content-length")
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

    let req_content_type = extract_header(&raw_headers, "content-type");
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
        scheme,
        method: Some(method.to_string()),
        url: Some(forward_url.clone()),
        status_code: None,
        status_text: None,
        headers: raw_headers.clone(),
        body: req_body_captured,
        body_size: content_length,
        body_truncated: req_body_truncated,
        content_type: req_content_type,
        process_name: if process_name != "-" { Some(process_name.clone()) } else { None },
        process_id: if process_id > 0 { Some(process_id) } else { None },
        process_path: None,
        source_ip: Some(source_ip.clone()),
        dest_ip: Some(dest_ip.clone()),
        source_port: Some(client_addr.port()),
        dest_port: Some(dest_port),
        timestamp,
        duration_us: None,
        cookies: vec![],
        raw_tls_info: None,
    };

    let _ = engine_tx.send(req_msg).await;

    let start = std::time::Instant::now();

    let upstream_stream = match tokio::net::TcpStream::connect((dest_ip.as_str(), dest_port)).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Transparent proxy: Failed to connect to {}:{} - {}", dest_ip, dest_port, e);
            let mut client_stream = reader.into_inner();
            let resp = format!("HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\n\r\nFailed to connect: {}", e.to_string().len(), e);
            let _ = client_stream.write_all(resp.as_bytes()).await;
            return Ok(());
        }
    };

    let (mut rh, mut wh) = tokio::io::split(upstream_stream);

    let forward_req = build_forward_request(method, &forward_url, &raw_headers, &req_body_bytes);
    let _ = wh.write_all(forward_req.as_bytes()).await;

    let mut resp_data = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = match rh.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        resp_data.extend_from_slice(&buf[..n]);
        if resp_data.len() > 10 * 1024 * 1024 {
            break;
        }
    }
    drop(rh);
    drop(wh);

    let duration_us = start.elapsed().as_micros() as u64;

    let (status_code, status_text, resp_headers, resp_body_bytes) = parse_http_response(&resp_data);

    let resp_content_type = extract_header(&resp_headers, "content-type");
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
        scheme,
        method: None,
        url: None,
        status_code,
        status_text: status_text.clone(),
        headers: resp_headers.clone(),
        body: resp_body_captured,
        body_size: resp_body_size,
        body_truncated: resp_body_truncated,
        content_type: resp_content_type,
        process_name: if process_name != "-" { Some(process_name) } else { None },
        process_id: if process_id > 0 { Some(process_id) } else { None },
        process_path: None,
        source_ip: Some(dest_ip.clone()),
        dest_ip: Some(source_ip.clone()),
        source_port: Some(dest_port),
        dest_port: Some(client_addr.port()),
        timestamp: now_us(),
        duration_us: Some(duration_us),
        cookies: vec![],
        raw_tls_info: None,
    };

    let _ = engine_tx.send(resp_msg).await;

    let mut client_stream = reader.into_inner();
    let _ = client_stream.write_all(&resp_data).await;
    let _ = client_stream.shutdown().await;

    Ok(())
}

async fn get_original_destination(stream: &tokio::net::TcpStream) -> Option<SocketAddr> {
    #[cfg(target_os = "windows")]
    {
        query_original_dst_windows(stream)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = stream;
        None
    }
}

#[cfg(target_os = "windows")]
fn query_original_dst_windows(stream: &tokio::net::TcpStream) -> Option<SocketAddr> {
    use std::os::windows::io::AsRawSocket;
    use windows::Win32::Networking::WinSock::*;

    let raw_socket = stream.as_raw_socket();

    let mut addr: SOCKADDR_IN = unsafe { std::mem::zeroed() };
    addr.sin_family = ADDRESS_FAMILY(AF_INET.0 as u16);
    let mut addr_len = std::mem::size_of::<SOCKADDR_IN>() as i32;

    let result = unsafe {
        getsockopt(
            SOCKET(raw_socket as usize),
            SOL_SOCKET,
            SO_ORIGINAL_DST as i32,
            windows::core::PSTR(&mut addr as *mut SOCKADDR_IN as *mut u8),
            &mut addr_len,
        )
    };

    if result == SOCKET_ERROR {
        let err = unsafe { WSAGetLastError() };
        tracing::debug!("getsockopt SO_ORIGINAL_DST failed: WSA error {:?}", err);
        return None;
    }

    let port = u16::from_be_bytes(addr.sin_port.to_ne_bytes());
    let ip_bytes = unsafe { addr.sin_addr.S_un.S_addr.to_ne_bytes() };
    let ip = std::net::Ipv4Addr::from(ip_bytes);

    Some(SocketAddr::new(std::net::IpAddr::V4(ip), port))
}

fn build_forward_request(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> String {
    let uri: hyper::Uri = url.parse().unwrap_or_else(|_| format!("http://{}", url).parse().unwrap());
    let path = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");

    let mut req = format!("{} {} HTTP/1.1\r\n", method, path);

    for (key, value) in headers {
        let kl = key.to_lowercase();
        if kl != "connection" && kl != "proxy-connection" && kl != "proxy-authorization" {
            req.push_str(&format!("{}: {}\r\n", key, value));
        }
    }

    if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("host")) {
        if let Some(host) = uri.host() {
            req.push_str(&format!("Host: {}\r\n", host));
        }
    }

    req.push_str(&format!("Content-Length: {}\r\n", body.len()));
    req.push_str("Connection: close\r\n");
    req.push_str("\r\n");

    req
}

fn parse_http_response(data: &[u8]) -> (Option<u16>, Option<String>, Vec<(String, String)>, Vec<u8>) {
    let header_end = data.windows(4).position(|w| w == b"\r\n\r\n").unwrap_or(data.len());
    let header_str = String::from_utf8_lossy(&data[..header_end]);

    let mut lines = header_str.lines();
    let status_line = lines.next().unwrap_or("");

    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse::<u16>().ok());

    let status_text = status_line
        .split_whitespace()
        .nth(2)
        .map(|s| s.to_string());

    let mut headers = Vec::new();
    for line in lines {
        if let Some(colon_pos) = line.find(':') {
            let name = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            headers.push((name, value));
        }
    }

    let body_start = header_end + 4;
    let body = if body_start < data.len() {
        data[body_start..].to_vec()
    } else {
        Vec::new()
    };

    (status_code, status_text, headers, body)
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

#[cfg(target_os = "windows")]
pub mod wfp_engine {
    use crate::engine_error::EngineError;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::NetworkManagement::WindowsFilteringPlatform::*;
    use windows::Win32::System::Rpc::RPC_C_AUTHN_WINNT;

    static PROVIDER_GUID: windows::core::GUID = windows::core::GUID::from_values(
        0xE1A2B3C4, 0xD5E6, 0xF789, [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF],
    );

    static SUBLAYER_GUID: windows::core::GUID = windows::core::GUID::from_values(
        0xF2A3B4C5, 0xD6E7, 0xF890, [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0],
    );

    static FILTER_REDIRECT_V4_GUID: windows::core::GUID = windows::core::GUID::from_values(
        0xA3B4C5D6, 0xE7F8, 0x9012, [0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x12],
    );

    static FILTER_REDIRECT_V6_GUID: windows::core::GUID = windows::core::GUID::from_values(
        0xB4C5D6E7, 0xF890, 0x1234, [0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x12, 0x34],
    );

    static FWPM_CALLOUT_ALE_CONNECT_REDIRECT_V4: windows::core::GUID = windows::core::GUID::from_values(
        0x784DDC32, 0x2E83, 0x4CF7, [0xA2, 0xB1, 0x6E, 0x88, 0xE6, 0xE4, 0xC3, 0xA0],
    );

    static FWPM_CALLOUT_ALE_CONNECT_REDIRECT_V6: windows::core::GUID = windows::core::GUID::from_values(
        0xB28D8F62, 0x1F5F, 0x4E1C, [0x9C, 0x81, 0x1B, 0x7A, 0x46, 0x5C, 0x8E, 0x26],
    );

    pub fn install_redirect_filters(_proxy_port: u16) -> Result<(), EngineError> {
        unsafe {
            let session = FWPM_SESSION0::default();

            let mut engine_handle: HANDLE = HANDLE::default();
            let result = FwpmEngineOpen0(
                None,
                RPC_C_AUTHN_WINNT,
                None,
                Some(&raw const session),
                &mut engine_handle,
            );
            if result != 0 {
                return Err(EngineError::WfpError(format!(
                    "FwpmEngineOpen0 failed: 0x{:08X}",
                    result
                )));
            }

            let provider = FWPM_PROVIDER0 {
                providerKey: PROVIDER_GUID,
                displayData: FWPM_DISPLAY_DATA0::default(),
                flags: 0,
                providerData: FWP_BYTE_BLOB::default(),
                serviceName: windows::core::PWSTR::null(),
            };

            let sublayer = FWPM_SUBLAYER0 {
                subLayerKey: SUBLAYER_GUID,
                displayData: FWPM_DISPLAY_DATA0::default(),
                flags: 0,
                providerKey: &raw const PROVIDER_GUID as *mut _,
                providerData: FWP_BYTE_BLOB::default(),
                weight: 0xFFFF,
            };

            let _ = FwpmProviderAdd0(engine_handle, &provider, None);
            let _ = FwpmSubLayerAdd0(engine_handle, &sublayer, None);

            let filter_v4 = FWPM_FILTER0 {
                filterKey: FILTER_REDIRECT_V4_GUID,
                displayData: FWPM_DISPLAY_DATA0::default(),
                flags: FWPM_FILTER_FLAGS(0),
                providerKey: &raw const PROVIDER_GUID as *mut _,
                providerData: FWP_BYTE_BLOB::default(),
                layerKey: FWPM_LAYER_ALE_CONNECT_REDIRECT_V4,
                subLayerKey: SUBLAYER_GUID,
                weight: FWP_VALUE0 {
                    r#type: FWP_UINT8,
                    Anonymous: FWP_VALUE0_0 { uint8: 5 },
                },
                numFilterConditions: 0,
                filterCondition: std::ptr::null_mut(),
                action: FWPM_ACTION0 {
                    r#type: FWP_ACTION_CALLOUT_UNKNOWN,
                    Anonymous: FWPM_ACTION0_0 {
                        calloutKey: FWPM_CALLOUT_ALE_CONNECT_REDIRECT_V4,
                    },
                },
                Anonymous: FWPM_FILTER0_0 { rawContext: 0 },
                reserved: std::ptr::null_mut(),
                filterId: 0,
                effectiveWeight: FWP_VALUE0::default(),
            };

            let mut filter_id: u64 = 0;
            let result = FwpmFilterAdd0(engine_handle, &filter_v4, None, Some(&raw mut filter_id));
            if result != 0 {
                let _ = FwpmEngineClose0(engine_handle);
                return Err(EngineError::WfpError(format!(
                    "FwpmFilterAdd0 (V4 redirect) failed: 0x{:08X}",
                    result
                )));
            }

            tracing::info!("WFP V4 connect redirect filter added, id={}", filter_id);

            let filter_v6 = FWPM_FILTER0 {
                filterKey: FILTER_REDIRECT_V6_GUID,
                layerKey: FWPM_LAYER_ALE_CONNECT_REDIRECT_V6,
                action: FWPM_ACTION0 {
                    r#type: FWP_ACTION_CALLOUT_UNKNOWN,
                    Anonymous: FWPM_ACTION0_0 {
                        calloutKey: FWPM_CALLOUT_ALE_CONNECT_REDIRECT_V6,
                    },
                },
                ..filter_v4.clone()
            };

            let mut filter_id_v6: u64 = 0;
            let result = FwpmFilterAdd0(engine_handle, &filter_v6, None, Some(&raw mut filter_id_v6));
            if result != 0 {
                tracing::warn!("WFP V6 filter add failed: 0x{:08X}", result);
            } else {
                tracing::info!("WFP V6 connect redirect filter added, id={}", filter_id_v6);
            }

            let _ = FwpmEngineClose0(engine_handle);
            Ok(())
        }
    }

    pub fn uninstall_redirect_filters() -> Result<(), EngineError> {
        unsafe {
            let session = FWPM_SESSION0::default();

            let mut engine_handle: HANDLE = HANDLE::default();
            let result = FwpmEngineOpen0(
                None,
                RPC_C_AUTHN_WINNT,
                None,
                Some(&raw const session),
                &mut engine_handle,
            );
            if result != 0 {
                return Err(EngineError::WfpError(format!(
                    "FwpmEngineOpen0 failed: 0x{:08X}",
                    result
                )));
            }

            let _ = FwpmFilterDeleteByKey0(engine_handle, &FILTER_REDIRECT_V4_GUID);
            let _ = FwpmFilterDeleteByKey0(engine_handle, &FILTER_REDIRECT_V6_GUID);
            let _ = FwpmSubLayerDeleteByKey0(engine_handle, &SUBLAYER_GUID);
            let _ = FwpmProviderDeleteByKey0(engine_handle, &PROVIDER_GUID);

            let _ = FwpmEngineClose0(engine_handle);
            Ok(())
        }
    }
}
