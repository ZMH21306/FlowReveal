use serde::{Deserialize, Serialize};
use crate::http_message::{HttpMessage, HttpSession};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarExport {
    pub log: HarLog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarLog {
    pub version: String,
    pub creator: HarCreator,
    pub entries: Vec<HarEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarCreator {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarEntry {
    pub started_date_time: String,
    pub time: f64,
    pub request: HarRequest,
    pub response: Option<HarResponse>,
    pub timings: HarTimings,
    pub server_ip_address: Option<String>,
    pub connection: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarRequest {
    pub method: String,
    pub url: String,
    pub http_version: String,
    pub headers: Vec<HarNameValuePair>,
    pub cookies: Vec<HarCookie>,
    pub query_string: Vec<HarNameValuePair>,
    pub post_data: Option<HarPostData>,
    pub headers_size: i64,
    pub body_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarResponse {
    pub status: u16,
    pub status_text: String,
    pub http_version: String,
    pub headers: Vec<HarNameValuePair>,
    pub cookies: Vec<HarCookie>,
    pub content: HarContent,
    pub redirect_url: String,
    pub headers_size: i64,
    pub body_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarContent {
    pub size: i64,
    pub mime_type: String,
    pub text: Option<String>,
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarCookie {
    pub name: String,
    pub value: String,
    pub path: Option<String>,
    pub domain: Option<String>,
    pub expires: Option<String>,
    pub http_only: Option<bool>,
    pub secure: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarNameValuePair {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarPostData {
    pub mime_type: String,
    pub text: Option<String>,
    pub params: Vec<HarPostParam>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarPostParam {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarTimings {
    pub send: f64,
    pub wait: f64,
    pub receive: f64,
}

impl HarExport {
    pub fn from_sessions(sessions: &[HttpSession]) -> Self {
        let entries: Vec<HarEntry> = sessions.iter().map(|s| s.into()).collect();
        Self {
            log: HarLog {
                version: "1.2".to_string(),
                creator: HarCreator {
                    name: "FlowReveal".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
                entries,
            },
        }
    }
}

impl From<&HttpSession> for HarEntry {
    fn from(session: &HttpSession) -> Self {
        let started_date_time = unix_us_to_iso8601(session.request.timestamp);
        let time = session.duration_us().map(|d| d as f64 / 1000.0).unwrap_or(0.0);

        HarEntry {
            started_date_time,
            time,
            request: HarRequest::from(&session.request),
            response: session.response.as_ref().map(HarResponse::from),
            timings: HarTimings {
                send: 0.0,
                wait: time,
                receive: 0.0,
            },
            server_ip_address: session.request.dest_ip.clone(),
            connection: None,
        }
    }
}

impl From<&HttpMessage> for HarRequest {
    fn from(msg: &HttpMessage) -> Self {
        Self {
            method: msg.method.clone().unwrap_or_default(),
            url: msg.url.clone().unwrap_or_default(),
            http_version: msg.protocol.to_string(),
            headers: msg.headers.iter().map(|(k, v)| HarNameValuePair { name: k.clone(), value: v.clone() }).collect(),
            cookies: msg.cookies.iter().map(|c| HarCookie {
                name: c.name.clone(),
                value: c.value.clone(),
                path: Some(c.path.clone()),
                domain: Some(c.domain.clone()),
                expires: c.expires.map(|e| e.to_string()),
                http_only: Some(c.http_only),
                secure: Some(c.secure),
            }).collect(),
            query_string: vec![],
            post_data: None,
            headers_size: -1,
            body_size: msg.body_size as i64,
        }
    }
}

impl From<&HttpMessage> for HarResponse {
    fn from(msg: &HttpMessage) -> Self {
        Self {
            status: msg.status_code.unwrap_or(0),
            status_text: msg.status_text.clone().unwrap_or_default(),
            http_version: msg.protocol.to_string(),
            headers: msg.headers.iter().map(|(k, v)| HarNameValuePair { name: k.clone(), value: v.clone() }).collect(),
            cookies: msg.cookies.iter().map(|c| HarCookie {
                name: c.name.clone(),
                value: c.value.clone(),
                path: Some(c.path.clone()),
                domain: Some(c.domain.clone()),
                expires: c.expires.map(|e| e.to_string()),
                http_only: Some(c.http_only),
                secure: Some(c.secure),
            }).collect(),
            content: HarContent {
                size: msg.body_size as i64,
                mime_type: msg.content_type.clone().unwrap_or_default(),
                text: msg.body.as_ref().map(|b| String::from_utf8_lossy(b).to_string()),
                encoding: msg.body.as_ref().and_then(|b| {
                    if String::from_utf8(b.clone()).is_err() {
                        Some("base64".to_string())
                    } else {
                        None
                    }
                }),
            },
            redirect_url: String::new(),
            headers_size: -1,
            body_size: msg.body_size as i64,
        }
    }
}

fn unix_us_to_iso8601(us: u64) -> String {
    let secs = us / 1_000_000;
    let millis = (us % 1_000_000) / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs as i64, millis as u32)
        .unwrap_or_default();
    datetime.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
