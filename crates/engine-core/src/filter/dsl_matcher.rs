use crate::http_message::HttpSession;
use super::dsl_ast::*;

pub fn match_dsl(expr: &DslExpr, session: &HttpSession) -> bool {
    match expr {
        DslExpr::FieldMatch { field, op, value } => {
            match_field(session, field, op, value)
        }
        DslExpr::And(l, r) => match_dsl(l, session) && match_dsl(r, session),
        DslExpr::Or(l, r) => match_dsl(l, session) || match_dsl(r, session),
        DslExpr::Not(e) => !match_dsl(e, session),
    }
}

fn match_field(session: &HttpSession, field: &DslField, op: &DslOp, value: &DslValue) -> bool {
    let target = extract_field(session, field);
    match target {
        Some(val) => apply_op(&val, op, value),
        None => false,
    }
}

fn extract_field(session: &HttpSession, field: &DslField) -> Option<String> {
    let req = &session.request;
    let resp = session.response.as_ref();
    match field {
        DslField::Method => req.method.clone(),
        DslField::Url => req.url.clone(),
        DslField::Host => req.host().map(|h| h.to_string()),
        DslField::Path => req.url.as_ref().and_then(|u| {
            url::Url::parse(u).ok().map(|p| p.path().to_string())
        }),
        DslField::Status => resp.and_then(|r| r.status_code.map(|c| c.to_string())),
        DslField::ProcessName => req.process_name.clone(),
        DslField::ProcessId => req.process_id.map(|p| p.to_string()),
        DslField::Body => req.body.as_ref().map(|b| String::from_utf8_lossy(b).to_string()),
        DslField::ContentType => req.content_type.clone().or_else(|| resp.and_then(|r| r.content_type.clone())),
        DslField::Scheme => Some(req.scheme.to_string()),
        DslField::Duration => session.duration_us().map(|d| {
            let ms = d as f64 / 1000.0;
            if ms >= 1.0 { format!("{:.0}", ms) } else { format!("{}", d) }
        }),
        DslField::Size => {
            let req_size = req.body_size as u64;
            let resp_size = resp.map(|r| r.body_size as u64).unwrap_or(0);
            Some((req_size + resp_size).to_string())
        }
        DslField::Header(name) => {
            let val = req.header(name).map(|v| v.to_string());
            val.or_else(|| resp.and_then(|r| r.header(name).map(|v| v.to_string())))
        }
    }
}

fn apply_op(target: &str, op: &DslOp, value: &DslValue) -> bool {
    match (op, value) {
        (DslOp::Contains, DslValue::String(s)) => target.to_lowercase().contains(&s.to_lowercase()),
        (DslOp::Contains, DslValue::Number(n)) => target.contains(&n.to_string()),
        (DslOp::Equals, DslValue::String(s)) => target.eq_ignore_ascii_case(s),
        (DslOp::Equals, DslValue::Number(n)) => {
            let target_n: f64 = target.parse().unwrap_or(f64::NAN);
            (target_n - n).abs() < f64::EPSILON
        }
        (DslOp::NotEquals, v) => !apply_op(target, &DslOp::Equals, v),
        (DslOp::Regex, DslValue::String(pattern)) => {
            regex::Regex::new(pattern)
                .map(|re| re.is_match(target))
                .unwrap_or(false)
        }
        (DslOp::GreaterThan, DslValue::Number(n)) => {
            let target_n: f64 = target.parse().unwrap_or(f64::NAN);
            target_n > *n
        }
        (DslOp::GreaterThan, DslValue::DurationMs(ms)) => {
            let target_ms: f64 = target.parse().unwrap_or(f64::NAN);
            target_ms > *ms as f64
        }
        (DslOp::GreaterThan, DslValue::SizeBytes(bytes)) => {
            let target_bytes: f64 = target.parse().unwrap_or(f64::NAN);
            target_bytes > *bytes as f64
        }
        (DslOp::LessThan, DslValue::Number(n)) => {
            let target_n: f64 = target.parse().unwrap_or(f64::NAN);
            target_n < *n
        }
        (DslOp::LessThan, DslValue::DurationMs(ms)) => {
            let target_ms: f64 = target.parse().unwrap_or(f64::NAN);
            target_ms < *ms as f64
        }
        (DslOp::LessThan, DslValue::SizeBytes(bytes)) => {
            let target_bytes: f64 = target.parse().unwrap_or(f64::NAN);
            target_bytes < *bytes as f64
        }
        (DslOp::Range, DslValue::Range(lo, hi)) => {
            let target_n: f64 = target.parse().unwrap_or(f64::NAN);
            target_n >= *lo && target_n <= *hi
        }
        (DslOp::Wildcard, DslValue::String(pattern)) => {
            let regex_pattern = pattern
                .replace('.', r"\.")
                .replace('*', ".*")
                .replace('?', ".");
            regex::Regex::new(&format!("^{}$", regex_pattern))
                .map(|re| re.is_match(target))
                .unwrap_or(false)
        }
        (DslOp::StartsWith, DslValue::String(s)) => target.to_lowercase().starts_with(&s.to_lowercase()),
        (DslOp::EndsWith, DslValue::String(s)) => target.to_lowercase().ends_with(&s.to_lowercase()),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::DslParser;
    use crate::http_message::*;

    fn make_session(method: &str, url: &str, status: Option<u16>) -> HttpSession {
        let req = HttpMessage {
            id: 1,
            session_id: 1,
            direction: MessageDirection::Request,
            protocol: HttpProtocol::HTTP1_1,
            scheme: Scheme::Https,
            method: Some(method.to_string()),
            url: Some(url.to_string()),
            status_code: None,
            status_text: None,
            headers: vec![("Host".to_string(), "api.example.com".to_string())],
            body: Some(br#"{"token":"abc"}"#.to_vec()),
            body_size: 14,
            body_truncated: false,
            content_type: Some("application/json".to_string()),
            process_name: Some("chrome".to_string()),
            process_id: Some(1234),
            process_path: None,
            source_ip: Some("127.0.0.1".to_string()),
            dest_ip: None,
            source_port: None,
            dest_port: Some(443),
            timestamp: 1000,
            duration_us: None,
            cookies: vec![],
            raw_tls_info: None,
            stream_id: None,
        };
        let mut session = HttpSession::new(req);
        if let Some(code) = status {
            let resp = HttpMessage {
                id: 2,
                session_id: 1,
                direction: MessageDirection::Response,
                status_code: Some(code),
                status_text: None,
                ..Default::default()
            };
            session.complete(resp);
        }
        session
    }

    #[test]
    fn test_method_match() {
        let session = make_session("GET", "https://api.example.com", None);
        let expr = DslParser::parse("method:GET").unwrap();
        assert!(match_dsl(&expr, &session));
    }

    #[test]
    fn test_host_contains() {
        let session = make_session("GET", "https://api.example.com", None);
        let expr = DslParser::parse("host:example.com").unwrap();
        assert!(match_dsl(&expr, &session));
    }

    #[test]
    fn test_status_range() {
        let session = make_session("GET", "https://api.example.com", Some(200));
        let expr = DslParser::parse("status:200..299").unwrap();
        assert!(match_dsl(&expr, &session));
    }

    #[test]
    fn test_and_logic() {
        let session = make_session("POST", "https://api.example.com", Some(200));
        let expr = DslParser::parse("method:POST AND status:200..299").unwrap();
        assert!(match_dsl(&expr, &session));
    }

    #[test]
    fn test_not_logic() {
        let session = make_session("GET", "https://api.example.com", Some(404));
        let expr = DslParser::parse("NOT status:200..299").unwrap();
        assert!(match_dsl(&expr, &session));
    }

    #[test]
    fn test_body_contains() {
        let session = make_session("POST", "https://api.example.com", None);
        let expr = DslParser::parse("body:token").unwrap();
        assert!(match_dsl(&expr, &session));
    }

    #[test]
    fn test_complex_expression() {
        let session = make_session("POST", "https://api.example.com/v1/users", Some(201));
        let expr = DslParser::parse("method:POST AND host:example.com AND NOT status:500..599").unwrap();
        assert!(match_dsl(&expr, &session));
    }
}
