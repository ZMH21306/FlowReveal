use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncReadExt};

static GLOBAL_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

pub(crate) fn next_session_id() -> u64 {
    GLOBAL_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
}

pub fn reset_session_counter() {
    GLOBAL_SESSION_COUNTER.store(1, Ordering::Relaxed);
}

pub(crate) fn extract_header(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

pub(crate) fn now_us() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}

pub(crate) fn parse_host_port(target: &str, default_port: u16) -> (String, u16) {
    if let Some(bracket_end) = target.find(']') {
        let host = target[..=bracket_end].to_string();
        let port = target[bracket_end + 1..]
            .strip_prefix(':')
            .and_then(|p| p.parse().ok())
            .unwrap_or(default_port);
        (host, port)
    } else if let Some(colon_pos) = target.rfind(':') {
        let host = target[..colon_pos].to_string();
        let port = target[colon_pos + 1..].parse().unwrap_or(default_port);
        (host, port)
    } else {
        (target.to_string(), default_port)
    }
}

pub(crate) fn truncate_body(body: &[u8], max_size: usize) -> (Option<Vec<u8>>, bool) {
    if body.is_empty() {
        (None, false)
    } else if body.len() > max_size {
        (Some(body[..max_size].to_vec()), true)
    } else {
        (Some(body.to_vec()), false)
    }
}

pub(crate) fn is_hop_by_hop_header(name: &str) -> bool {
    let lower = name.to_lowercase();
    matches!(
        lower.as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
            | "proxy-connection"
    )
}

pub(crate) async fn read_headers_from_buf<R: AsyncBufReadExt + Unpin>(
    reader: &mut R,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
    let mut headers = Vec::new();
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
            headers.push((name, value));
        }
    }
    Ok(headers)
}

pub(crate) async fn read_body_from_buf<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    content_length: usize,
    max_body_size: usize,
) -> Vec<u8> {
    if content_length == 0 {
        return Vec::new();
    }
    let to_read = content_length.min(max_body_size + 1);
    let mut buf = vec![0u8; to_read];
    reader.read_exact(&mut buf).await.ok();
    if content_length > max_body_size + 1 {
        let mut discard = [0u8; 4096];
        let mut remaining = content_length - to_read;
        while remaining > 0 {
            let chunk = remaining.min(4096);
            let n = reader.read(&mut discard[..chunk]).await.unwrap_or(0);
            if n == 0 { break; }
            remaining -= n;
        }
    }
    buf
}
