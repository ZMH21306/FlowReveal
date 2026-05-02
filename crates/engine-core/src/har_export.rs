use serde::Serialize;
use crate::http_message::HttpSession;

#[derive(Debug, Serialize)]
pub struct HarLog {
    pub version: String,
    pub creator: HarCreator,
    pub entries: Vec<HarEntry>,
}

#[derive(Debug, Serialize)]
pub struct HarCreator {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct HarEntry {
    pub started_date_time: String,
    pub time: f64,
    pub request: HarRequest,
    pub response: HarResponse,
    pub timings: HarTimings,
}

#[derive(Debug, Serialize)]
pub struct HarRequest {
    pub method: String,
    pub url: String,
    pub http_version: String,
    pub headers: Vec<HarHeader>,
    pub cookies: Vec<HarCookie>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_data: Option<HarPostData>,
    pub body_size: i64,
}

#[derive(Debug, Serialize)]
pub struct HarResponse {
    pub status: u16,
    pub status_text: String,
    pub http_version: String,
    pub headers: Vec<HarHeader>,
    pub cookies: Vec<HarCookie>,
    pub content: HarContent,
    pub redirect_url: String,
    pub headers_size: i64,
    pub body_size: i64,
}

#[derive(Debug, Serialize)]
pub struct HarHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct HarCookie {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct HarPostData {
    pub mime_type: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct HarContent {
    pub size: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HarTimings {
    pub send: i64,
    pub wait: i64,
    pub receive: i64,
}

#[derive(Debug, Serialize)]
pub struct HarExport {
    pub log: HarLog,
}

pub fn sessions_to_har(sessions: &[&HttpSession]) -> HarExport {
    let entries: Vec<HarEntry> = sessions
        .iter()
        .filter_map(|s| {
            let req = &s.request;
            let resp = s.response.as_ref()?;

            let started_date_time = format_timestamp_iso8601(req.timestamp);
            let time_ms = resp.duration_us.map(|d| d as f64 / 1000.0).unwrap_or(0.0);

            let req_headers: Vec<HarHeader> = req
                .headers
                .iter()
                .map(|(k, v)| HarHeader {
                    name: k.clone(),
                    value: v.clone(),
                })
                .collect();

            let req_cookies = extract_cookies(&req.headers);

            let post_data = if req.body_size > 0 {
                Some(HarPostData {
                    mime_type: req.content_type.clone().unwrap_or_default(),
                    text: req
                        .body
                        .as_ref()
                        .map(|b| String::from_utf8_lossy(b).to_string())
                        .unwrap_or_default(),
                })
            } else {
                None
            };

            let resp_headers: Vec<HarHeader> = resp
                .headers
                .iter()
                .map(|(k, v)| HarHeader {
                    name: k.clone(),
                    value: v.clone(),
                })
                .collect();

            let resp_cookies = extract_cookies(&resp.headers);

            let resp_body_text = resp
                .body
                .as_ref()
                .map(|b| {
                    let is_binary = resp
                        .content_type
                        .as_ref()
                        .map(|ct| {
                            let ct = ct.to_lowercase();
                            ct.starts_with("image/")
                                || ct.starts_with("video/")
                                || ct.starts_with("audio/")
                                || ct.contains("octet-stream")
                        })
                        .unwrap_or(false);
                    if is_binary {
                        (Some(base64_encode(b)), Some("base64".to_string()))
                    } else {
                        (Some(String::from_utf8_lossy(b).to_string()), None)
                    }
                })
                .unwrap_or((None, None));

            Some(HarEntry {
                started_date_time,
                time: time_ms,
                request: HarRequest {
                    method: req.method.clone().unwrap_or_default(),
                    url: req.url.clone().unwrap_or_default(),
                    http_version: req.protocol.to_string(),
                    headers: req_headers,
                    cookies: req_cookies,
                    post_data,
                    body_size: req.body_size as i64,
                },
                response: HarResponse {
                    status: resp.status_code.unwrap_or(0),
                    status_text: resp.status_text.clone().unwrap_or_default(),
                    http_version: resp.protocol.to_string(),
                    headers: resp_headers,
                    cookies: resp_cookies,
                    content: HarContent {
                        size: resp.body_size as i64,
                        mime_type: resp.content_type.clone(),
                        text: resp_body_text.0,
                        encoding: resp_body_text.1,
                    },
                    redirect_url: String::new(),
                    headers_size: -1,
                    body_size: resp.body_size as i64,
                },
                timings: HarTimings {
                    send: 0,
                    wait: time_ms as i64,
                    receive: 0,
                },
            })
        })
        .collect();

    HarExport {
        log: HarLog {
            version: "1.2".to_string(),
            creator: HarCreator {
                name: "FlowReveal".to_string(),
                version: "0.1.0".to_string(),
            },
            entries,
        },
    }
}

fn format_timestamp_iso8601(us: u64) -> String {
    let secs = us / 1_000_000;
    let millis = (us % 1_000_000) / 1000;
    let datetime = chrono::DateTime::from_timestamp(secs as i64, (millis as u32) * 1_000_000)
        .unwrap_or_else(|| chrono::DateTime::UNIX_EPOCH);
    datetime.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn extract_cookies(headers: &[(String, String)]) -> Vec<HarCookie> {
    headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("cookie"))
        .flat_map(|(_, v)| {
            v.split(';')
                .filter_map(|pair| {
                    let mut parts = pair.trim().splitn(2, '=');
                    let name = parts.next()?.trim().to_string();
                    let value = parts.next()?.trim().to_string();
                    Some(HarCookie { name, value })
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        result.push(if chunk.len() > 1 { CHARS[((triple >> 6) & 0x3F) as usize] as char } else { '=' });
        result.push(if chunk.len() > 2 { CHARS[(triple & 0x3F) as usize] as char } else { '=' });
    }
    result
}
