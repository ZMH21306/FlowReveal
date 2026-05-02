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
