use crate::http_message::HttpMessage;
use super::rule_types::{MatchCondition, MatchField, MatchFilter, MatchLogic, MatchOperator, Rule};

pub struct RuleMatcher;

impl RuleMatcher {
    pub fn matches(rule: &Rule, request: &HttpMessage, response: Option<&HttpMessage>) -> bool {
        if !rule.enabled {
            return false;
        }
        Self::match_condition(&rule.match_condition, request, response)
    }

    fn match_condition(
        cond: &MatchCondition,
        request: &HttpMessage,
        response: Option<&HttpMessage>,
    ) -> bool {
        if cond.filters.is_empty() {
            return true;
        }
        match cond.logic {
            MatchLogic::And => cond
                .filters
                .iter()
                .all(|f| Self::match_filter(f, request, response)),
            MatchLogic::Or => cond
                .filters
                .iter()
                .any(|f| Self::match_filter(f, request, response)),
        }
    }

    fn match_filter(
        filter: &MatchFilter,
        request: &HttpMessage,
        response: Option<&HttpMessage>,
    ) -> bool {
        let target_value = Self::extract_field_value(filter, request, response);
        match target_value {
            Some(val) => Self::apply_operator(&val, &filter.value, &filter.operator, filter.case_sensitive),
            None => false,
        }
    }

    fn extract_field_value(
        filter: &MatchFilter,
        request: &HttpMessage,
        response: Option<&HttpMessage>,
    ) -> Option<String> {
        match &filter.field {
            MatchField::Method => request.method.clone(),
            MatchField::Url => request.url.clone(),
            MatchField::Host => request.host().map(|h| h.to_string()),
            MatchField::Path => request.url.as_ref().and_then(|u| url_path(u)),
            MatchField::StatusCode => response
                .and_then(|r| r.status_code.map(|c| c.to_string())),
            MatchField::ContentType => request
                .content_type
                .clone()
                .or_else(|| response.and_then(|r| r.content_type.clone())),
            MatchField::HeaderName => {
                let found = request
                    .headers
                    .iter()
                    .any(|(k, _)| Self::apply_operator(k, &filter.value, &MatchOperator::Contains, filter.case_sensitive));
                Some(found.to_string())
            }
            MatchField::HeaderValue => {
                let found = request
                    .headers
                    .iter()
                    .any(|(_, v)| Self::apply_operator(v, &filter.value, &MatchOperator::Contains, filter.case_sensitive));
                Some(found.to_string())
            }
            MatchField::Body => request
                .body
                .as_ref()
                .map(|b| String::from_utf8_lossy(b).to_string()),
            MatchField::ProcessName => request.process_name.clone(),
            MatchField::Scheme => Some(request.scheme.to_string()),
            MatchField::QueryParam => request.url.as_ref().and_then(|u| url_query(u)),
        }
    }

    fn apply_operator(lhs: &str, rhs: &str, op: &MatchOperator, case_sensitive: bool) -> bool {
        let (l, r) = if case_sensitive {
            (lhs.to_string(), rhs.to_string())
        } else {
            (lhs.to_lowercase(), rhs.to_lowercase())
        };

        match op {
            MatchOperator::Equals => l == r,
            MatchOperator::NotEquals => l != r,
            MatchOperator::Contains => l.contains(&r),
            MatchOperator::NotContains => !l.contains(&r),
            MatchOperator::StartsWith => l.starts_with(&r),
            MatchOperator::EndsWith => l.ends_with(&r),
            MatchOperator::MatchesRegex => regex::Regex::new(rhs)
                .map(|re| re.is_match(lhs))
                .unwrap_or(false),
            MatchOperator::GreaterThan => {
                let ln: f64 = l.parse().unwrap_or(f64::NAN);
                let rn: f64 = r.parse().unwrap_or(f64::NAN);
                ln > rn
            }
            MatchOperator::LessThan => {
                let ln: f64 = l.parse().unwrap_or(f64::NAN);
                let rn: f64 = r.parse().unwrap_or(f64::NAN);
                ln < rn
            }
            MatchOperator::InRange => {
                let parts: Vec<&str> = rhs.split(',').collect();
                if parts.len() == 2 {
                    let lo = parts[0].trim();
                    let hi = parts[1].trim();
                    l.as_str() >= lo && l.as_str() <= hi
                } else {
                    false
                }
            }
            MatchOperator::Wildcard => glob_match(&l, &r),
        }
    }
}

fn url_path(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    Some(parsed.path().to_string())
}

fn url_query(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    parsed.query().map(|q| q.to_string())
}

fn glob_match(text: &str, pattern: &str) -> bool {
    let regex_pattern = pattern
        .replace('.', r"\.")
        .replace('*', ".*")
        .replace('?', ".");
    regex::Regex::new(&format!("^{}$", regex_pattern))
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_message::{HttpMessage, HttpProtocol, MessageDirection, Scheme};

    fn make_request() -> HttpMessage {
        HttpMessage {
            id: 1,
            session_id: 1,
            direction: MessageDirection::Request,
            protocol: HttpProtocol::HTTP1_1,
            scheme: Scheme::Https,
            method: Some("GET".to_string()),
            url: Some("https://api.example.com/v1/users?page=1".to_string()),
            status_code: None,
            status_text: None,
            headers: vec![
                ("Host".to_string(), "api.example.com".to_string()),
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            body: Some(br#"{"token":"abc123"}"#.to_vec()),
            body_size: 17,
            body_truncated: false,
            content_type: Some("application/json".to_string()),
            process_name: Some("chrome".to_string()),
            process_id: Some(1234),
            process_path: None,
            source_ip: Some("127.0.0.1".to_string()),
            dest_ip: Some("api.example.com".to_string()),
            source_port: None,
            dest_port: Some(443),
            timestamp: 0,
            duration_us: None,
            cookies: vec![],
            raw_tls_info: None,
            stream_id: None,
        }
    }

    #[test]
    fn test_method_equals() {
        let req = make_request();
        let filter = MatchFilter {
            field: MatchField::Method,
            operator: MatchOperator::Equals,
            value: "GET".to_string(),
            case_sensitive: false,
        };
        assert!(RuleMatcher::match_filter(&filter, &req, None));
    }

    #[test]
    fn test_host_contains() {
        let req = make_request();
        let filter = MatchFilter {
            field: MatchField::Host,
            operator: MatchOperator::Contains,
            value: "example.com".to_string(),
            case_sensitive: false,
        };
        assert!(RuleMatcher::match_filter(&filter, &req, None));
    }

    #[test]
    fn test_url_regex() {
        let req = make_request();
        let filter = MatchFilter {
            field: MatchField::Url,
            operator: MatchOperator::MatchesRegex,
            value: r"/v[0-9]+/users".to_string(),
            case_sensitive: false,
        };
        assert!(RuleMatcher::match_filter(&filter, &req, None));
    }

    #[test]
    fn test_body_contains() {
        let req = make_request();
        let filter = MatchFilter {
            field: MatchField::Body,
            operator: MatchOperator::Contains,
            value: "token".to_string(),
            case_sensitive: false,
        };
        assert!(RuleMatcher::match_filter(&filter, &req, None));
    }

    #[test]
    fn test_rule_matches_and_logic() {
        let req = make_request();
        let rule = Rule {
            id: 1,
            name: "Test".to_string(),
            category: crate::rules::RuleCategory::AutoReply,
            enabled: true,
            priority: 10,
            match_condition: MatchCondition {
                logic: MatchLogic::And,
                filters: vec![
                    MatchFilter {
                        field: MatchField::Method,
                        operator: MatchOperator::Equals,
                        value: "GET".to_string(),
                        case_sensitive: false,
                    },
                    MatchFilter {
                        field: MatchField::Host,
                        operator: MatchOperator::Contains,
                        value: "example.com".to_string(),
                        case_sensitive: false,
                    },
                ],
            },
            action: crate::rules::RuleAction::AutoReply(crate::rules::AutoReplyAction {
                status_code: 200,
                status_text: "OK".to_string(),
                headers: vec![],
                body_source: crate::rules::BodySource::Empty,
                delay_ms: 0,
            }),
            created_at: 0,
            updated_at: 0,
        };
        assert!(RuleMatcher::matches(&rule, &req, None));
    }

    #[test]
    fn test_rule_disabled() {
        let req = make_request();
        let rule = Rule {
            id: 1,
            name: "Test".to_string(),
            category: crate::rules::RuleCategory::AutoReply,
            enabled: false,
            priority: 10,
            match_condition: MatchCondition {
                logic: MatchLogic::And,
                filters: vec![],
            },
            action: crate::rules::RuleAction::AutoReply(crate::rules::AutoReplyAction {
                status_code: 200,
                status_text: "OK".to_string(),
                headers: vec![],
                body_source: crate::rules::BodySource::Empty,
                delay_ms: 0,
            }),
            created_at: 0,
            updated_at: 0,
        };
        assert!(!RuleMatcher::matches(&rule, &req, None));
    }

    #[test]
    fn test_empty_condition_matches() {
        let req = make_request();
        let rule = Rule {
            id: 1,
            name: "Test".to_string(),
            category: crate::rules::RuleCategory::AutoReply,
            enabled: true,
            priority: 10,
            match_condition: MatchCondition {
                logic: MatchLogic::And,
                filters: vec![],
            },
            action: crate::rules::RuleAction::AutoReply(crate::rules::AutoReplyAction {
                status_code: 200,
                status_text: "OK".to_string(),
                headers: vec![],
                body_source: crate::rules::BodySource::Empty,
                delay_ms: 0,
            }),
            created_at: 0,
            updated_at: 0,
        };
        assert!(RuleMatcher::matches(&rule, &req, None));
    }
}
