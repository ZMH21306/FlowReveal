use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageDirection {
    Request,
    Response,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpProtocol {
    HTTP1_1,
    HTTP2,
    WebSocket,
}

impl std::fmt::Display for HttpProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpProtocol::HTTP1_1 => write!(f, "HTTP/1.1"),
            HttpProtocol::HTTP2 => write!(f, "HTTP/2"),
            HttpProtocol::WebSocket => write!(f, "WebSocket"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scheme {
    Http,
    Https,
}

impl std::fmt::Display for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scheme::Http => write!(f, "http"),
            Scheme::Https => write!(f, "https"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsInfo {
    pub version: String,
    pub cipher_suite: String,
    pub server_name: Option<String>,
    pub cert_chain: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<i64>,
    pub http_only: bool,
    pub secure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpMessage {
    pub id: u64,
    pub session_id: u64,
    pub direction: MessageDirection,
    pub protocol: HttpProtocol,
    pub scheme: Scheme,
    pub method: Option<String>,
    pub url: Option<String>,
    pub status_code: Option<u16>,
    pub status_text: Option<String>,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub body_size: usize,
    pub body_truncated: bool,
    pub content_type: Option<String>,
    pub process_name: Option<String>,
    pub process_id: Option<u32>,
    pub process_path: Option<String>,
    pub source_ip: Option<String>,
    pub dest_ip: Option<String>,
    pub source_port: Option<u16>,
    pub dest_port: Option<u16>,
    pub timestamp: u64,
    pub duration_us: Option<u64>,
    pub cookies: Vec<Cookie>,
    pub raw_tls_info: Option<TlsInfo>,
    pub stream_id: Option<u32>,
}

impl HttpMessage {
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    pub fn is_json(&self) -> bool {
        self.content_type
            .as_ref()
            .is_some_and(|ct| ct.contains("application/json"))
    }

    pub fn is_html(&self) -> bool {
        self.content_type
            .as_ref()
            .is_some_and(|ct| ct.contains("text/html"))
    }

    pub fn is_image(&self) -> bool {
        self.content_type
            .as_ref()
            .is_some_and(|ct| ct.starts_with("image/"))
    }

    pub fn host(&self) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("host"))
            .map(|(_, v)| v.as_str())
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn builder() -> HttpMessageBuilder {
        HttpMessageBuilder::default()
    }

    pub fn request(
        session_id: u64,
        protocol: HttpProtocol,
        scheme: Scheme,
        method: impl Into<String>,
        url: impl Into<String>,
        headers: Vec<(String, String)>,
        timestamp: u64,
    ) -> HttpMessageBuilder {
        HttpMessageBuilder::default()
            .id(session_id)
            .session_id(session_id)
            .direction(MessageDirection::Request)
            .protocol(protocol)
            .scheme(scheme)
            .method(method)
            .url(url)
            .headers(headers)
            .timestamp(timestamp)
    }

    pub fn response(
        session_id: u64,
        protocol: HttpProtocol,
        scheme: Scheme,
        status_code: u16,
        headers: Vec<(String, String)>,
        timestamp: u64,
        duration_us: u64,
    ) -> HttpMessageBuilder {
        HttpMessageBuilder::default()
            .id(session_id + 1)
            .session_id(session_id)
            .direction(MessageDirection::Response)
            .protocol(protocol)
            .scheme(scheme)
            .status_code(status_code)
            .headers(headers)
            .timestamp(timestamp)
            .duration_us(duration_us)
    }
}

impl Default for HttpMessage {
    fn default() -> Self {
        Self {
            id: 0,
            session_id: 0,
            direction: MessageDirection::Request,
            protocol: HttpProtocol::HTTP1_1,
            scheme: Scheme::Https,
            method: None,
            url: None,
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
            source_ip: None,
            dest_ip: None,
            source_port: None,
            dest_port: None,
            timestamp: 0,
            duration_us: None,
            cookies: vec![],
            raw_tls_info: None,
            stream_id: None,
        }
    }
}

#[derive(Default)]
pub struct HttpMessageBuilder {
    inner: HttpMessage,
}

impl HttpMessageBuilder {
    pub fn id(mut self, v: u64) -> Self { self.inner.id = v; self }
    pub fn session_id(mut self, v: u64) -> Self { self.inner.session_id = v; self }
    pub fn direction(mut self, v: MessageDirection) -> Self { self.inner.direction = v; self }
    pub fn protocol(mut self, v: HttpProtocol) -> Self { self.inner.protocol = v; self }
    pub fn scheme(mut self, v: Scheme) -> Self { self.inner.scheme = v; self }
    pub fn method(mut self, v: impl Into<String>) -> Self { self.inner.method = Some(v.into()); self }
    pub fn url(mut self, v: impl Into<String>) -> Self { self.inner.url = Some(v.into()); self }
    pub fn status_code(mut self, v: u16) -> Self { self.inner.status_code = Some(v); self }
    pub fn status_text(mut self, v: impl Into<String>) -> Self { self.inner.status_text = Some(v.into()); self }
    pub fn headers(mut self, v: Vec<(String, String)>) -> Self { self.inner.headers = v; self }
    pub fn body(mut self, v: Option<Vec<u8>>) -> Self { self.inner.body = v; self }
    pub fn body_size(mut self, v: usize) -> Self { self.inner.body_size = v; self }
    pub fn body_truncated(mut self, v: bool) -> Self { self.inner.body_truncated = v; self }
    pub fn content_type(mut self, v: impl Into<String>) -> Self { self.inner.content_type = Some(v.into()); self }
    pub fn process_name(mut self, v: impl Into<String>) -> Self { self.inner.process_name = Some(v.into()); self }
    pub fn process_id(mut self, v: u32) -> Self { self.inner.process_id = Some(v); self }
    pub fn process_path(mut self, v: impl Into<String>) -> Self { self.inner.process_path = Some(v.into()); self }
    pub fn source_ip(mut self, v: impl Into<String>) -> Self { self.inner.source_ip = Some(v.into()); self }
    pub fn dest_ip(mut self, v: impl Into<String>) -> Self { self.inner.dest_ip = Some(v.into()); self }
    pub fn source_port(mut self, v: u16) -> Self { self.inner.source_port = Some(v); self }
    pub fn dest_port(mut self, v: u16) -> Self { self.inner.dest_port = Some(v); self }
    pub fn timestamp(mut self, v: u64) -> Self { self.inner.timestamp = v; self }
    pub fn duration_us(mut self, v: u64) -> Self { self.inner.duration_us = Some(v); self }
    pub fn raw_tls_info(mut self, v: TlsInfo) -> Self { self.inner.raw_tls_info = Some(v); self }
    pub fn stream_id(mut self, v: u32) -> Self { self.inner.stream_id = Some(v); self }

    pub fn process_info(mut self, info: Option<&crate::process_info::ProcessInfo>) -> Self {
        if let Some(p) = info {
            self.inner.process_name = Some(p.name.clone());
            self.inner.process_id = Some(p.pid);
            self.inner.process_path = p.path.clone();
        }
        self
    }

    pub fn build(self) -> HttpMessage {
        self.inner
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpSession {
    pub id: u64,
    pub request: HttpMessage,
    pub response: Option<HttpMessage>,
    pub created_at: u64,
    pub completed_at: Option<u64>,
}

impl HttpSession {
    pub fn new(request: HttpMessage) -> Self {
        let created_at = request.timestamp;
        Self {
            id: request.session_id,
            request,
            response: None,
            created_at,
            completed_at: None,
        }
    }

    pub fn complete(&mut self, response: HttpMessage) {
        self.completed_at = Some(response.timestamp);
        self.response = Some(response);
    }

    pub fn duration_us(&self) -> Option<u64> {
        self.completed_at
            .map(|end| end.saturating_sub(self.created_at))
    }

    pub fn is_completed(&self) -> bool {
        self.response.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketFrame {
    pub id: u64,
    pub session_id: u64,
    pub direction: MessageDirection,
    pub opcode: WsOpcode,
    pub payload: Option<Vec<u8>>,
    pub payload_size: usize,
    pub payload_truncated: bool,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WsOpcode {
    Continuation,
    Text,
    Binary,
    Close,
    Ping,
    Pong,
}

impl std::fmt::Display for WsOpcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WsOpcode::Continuation => write!(f, "Continuation"),
            WsOpcode::Text => write!(f, "Text"),
            WsOpcode::Binary => write!(f, "Binary"),
            WsOpcode::Close => write!(f, "Close"),
            WsOpcode::Ping => write!(f, "Ping"),
            WsOpcode::Pong => write!(f, "Pong"),
        }
    }
}
