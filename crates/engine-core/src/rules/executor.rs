use super::rule_types::{
    AutoReplyAction, BodySource, HeaderAction, HeaderModifierAction, RedirectAction, RedirectType,
    RuleAction,
};

#[derive(Debug, Clone)]
pub enum RuleExecutionResult {
    AutoReply {
        status_code: u16,
        status_text: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
        delay_ms: u64,
    },
    HeaderModified {
        request_actions: Vec<HeaderAction>,
        response_actions: Vec<HeaderAction>,
    },
    Redirected {
        new_url: String,
        redirect_type: RedirectType,
        preserve_query: bool,
        preserve_path: bool,
    },
}

pub struct RuleExecutor;

impl RuleExecutor {
    pub fn execute(action: &RuleAction) -> RuleExecutionResult {
        match action {
            RuleAction::AutoReply(a) => Self::execute_auto_reply(a),
            RuleAction::HeaderModifier(a) => Self::execute_header_modifier(a),
            RuleAction::Redirect(a) => Self::execute_redirect(a),
        }
    }

    fn execute_auto_reply(a: &AutoReplyAction) -> RuleExecutionResult {
        let body = match &a.body_source {
            BodySource::Inline(s) => s.as_bytes().to_vec(),
            BodySource::File(path) => std::fs::read(path).unwrap_or_default(),
            BodySource::Empty => vec![],
        };
        RuleExecutionResult::AutoReply {
            status_code: a.status_code,
            status_text: a.status_text.clone(),
            headers: a.headers.clone(),
            body,
            delay_ms: a.delay_ms,
        }
    }

    fn execute_header_modifier(a: &HeaderModifierAction) -> RuleExecutionResult {
        RuleExecutionResult::HeaderModified {
            request_actions: a.request_actions.clone(),
            response_actions: a.response_actions.clone(),
        }
    }

    fn execute_redirect(a: &RedirectAction) -> RuleExecutionResult {
        RuleExecutionResult::Redirected {
            new_url: a.target_url.clone(),
            redirect_type: a.redirect_type,
            preserve_query: a.preserve_query,
            preserve_path: a.preserve_path,
        }
    }

    pub fn apply_header_actions(
        headers: &mut Vec<(String, String)>,
        actions: &[HeaderAction],
    ) {
        for action in actions {
            match action {
                HeaderAction::Add {
                    name,
                    value,
                    only_if_missing,
                } => {
                    let exists = headers
                        .iter()
                        .any(|(k, _)| k.eq_ignore_ascii_case(name));
                    if *only_if_missing && exists {
                        continue;
                    }
                    if !*only_if_missing || !exists {
                        headers.push((name.clone(), value.clone()));
                    }
                }
                HeaderAction::Remove { name } => {
                    headers.retain(|(k, _)| !k.eq_ignore_ascii_case(name));
                }
                HeaderAction::Replace { name, value } => {
                    for (k, v) in headers.iter_mut() {
                        if k.eq_ignore_ascii_case(name) {
                            *v = value.clone();
                        }
                    }
                }
                HeaderAction::ReplaceRegex {
                    name,
                    pattern,
                    replacement,
                } => {
                    if let Ok(re) = regex::Regex::new(pattern) {
                        for (k, v) in headers.iter_mut() {
                            if k.eq_ignore_ascii_case(name) {
                                *v = re.replace_all(v.as_str(), replacement.as_str()).to_string();
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn build_redirect_url(
        original_url: &str,
        target_url: &str,
        preserve_query: bool,
        preserve_path: bool,
    ) -> String {
        let mut result = target_url.to_string();

        if preserve_path || preserve_query {
            if let Ok(parsed_original) = url::Url::parse(original_url) {
                if let Ok(parsed_target) = url::Url::parse(&result) {
                    let mut built = parsed_target.clone();

                    if preserve_path && parsed_target.path() == "/" {
                        built.set_path(parsed_original.path());
                    }

                    if preserve_query {
                        for (key, value) in parsed_original.query_pairs() {
                            built.query_pairs_mut().append_pair(&key, &value);
                        }
                    }

                    result = built.to_string();
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_header_add() {
        let mut headers = vec![
            ("Content-Type".to_string(), "text/html".to_string()),
        ];
        RuleExecutor::apply_header_actions(
            &mut headers,
            &[HeaderAction::Add {
                name: "X-Custom".to_string(),
                value: "test".to_string(),
                only_if_missing: false,
            }],
        );
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[1].0, "X-Custom");
    }

    #[test]
    fn test_apply_header_add_only_if_missing() {
        let mut headers = vec![
            ("Content-Type".to_string(), "text/html".to_string()),
        ];
        RuleExecutor::apply_header_actions(
            &mut headers,
            &[HeaderAction::Add {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
                only_if_missing: true,
            }],
        );
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].1, "text/html");
    }

    #[test]
    fn test_apply_header_remove() {
        let mut headers = vec![
            ("Content-Type".to_string(), "text/html".to_string()),
            ("Cookie".to_string(), "session=abc".to_string()),
        ];
        RuleExecutor::apply_header_actions(
            &mut headers,
            &[HeaderAction::Remove {
                name: "Cookie".to_string(),
            }],
        );
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, "Content-Type");
    }

    #[test]
    fn test_apply_header_replace() {
        let mut headers = vec![
            ("Content-Type".to_string(), "text/html".to_string()),
        ];
        RuleExecutor::apply_header_actions(
            &mut headers,
            &[HeaderAction::Replace {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
            }],
        );
        assert_eq!(headers[0].1, "application/json");
    }

    #[test]
    fn test_build_redirect_url_preserve_query() {
        let result = RuleExecutor::build_redirect_url(
            "https://old.com/api?key=val",
            "https://new.com/api",
            true,
            false,
        );
        assert!(result.contains("key=val"));
        assert!(result.starts_with("https://new.com"));
    }
}
