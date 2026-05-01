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
