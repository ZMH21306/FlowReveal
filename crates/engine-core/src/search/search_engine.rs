use std::sync::Arc;
use tokio::sync::RwLock;
use crate::http_message::HttpSession;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SearchScope {
    All,
    Url,
    Headers,
    Body,
}

pub struct SearchEngine {
    sessions: Arc<RwLock<Vec<HttpSession>>>,
}

impl SearchEngine {
    pub fn new(sessions: Arc<RwLock<Vec<HttpSession>>>) -> Self {
        Self { sessions }
    }

    pub async fn search(
        &self,
        query: &str,
        scope: &SearchScope,
        case_sensitive: bool,
        use_regex: bool,
    ) -> Vec<u64> {
        let sessions = self.sessions.read().await;
        let pattern = if use_regex {
            regex::Regex::new(query).ok()
        } else {
            None
        };

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for session in sessions.iter() {
            if self.matches_session(session, &query_lower, scope, case_sensitive, pattern.as_ref()) {
                results.push(session.id);
            }
        }

        results
    }

    fn matches_session(
        &self,
        session: &HttpSession,
        query_lower: &str,
        scope: &SearchScope,
        case_sensitive: bool,
        regex_pattern: Option<&regex::Regex>,
    ) -> bool {
        let texts = self.extract_searchable_text(session, scope);
        for text in texts {
            if let Some(re) = regex_pattern {
                if re.is_match(&text) { return true; }
            } else if case_sensitive {
                if text.contains(query_lower) { return true; }
            } else {
                if text.to_lowercase().contains(query_lower) { return true; }
            }
        }
        false
    }

    fn extract_searchable_text(&self, session: &HttpSession, scope: &SearchScope) -> Vec<String> {
        let mut texts = Vec::new();
        let req = &session.request;
        let resp = session.response.as_ref();

        match scope {
            SearchScope::All => {
                if let Some(url) = &req.url { texts.push(url.clone()); }
                texts.extend(req.headers.iter().map(|(k, v)| format!("{}: {}", k, v)));
                if let Some(body) = &req.body {
                    texts.push(String::from_utf8_lossy(body).to_string());
                }
                if let Some(resp) = resp {
                    texts.extend(resp.headers.iter().map(|(k, v)| format!("{}: {}", k, v)));
                    if let Some(body) = &resp.body {
                        texts.push(String::from_utf8_lossy(body).to_string());
                    }
                }
            }
            SearchScope::Url => {
                if let Some(url) = &req.url { texts.push(url.clone()); }
            }
            SearchScope::Headers => {
                texts.extend(req.headers.iter().map(|(k, v)| format!("{}: {}", k, v)));
                if let Some(resp) = resp {
                    texts.extend(resp.headers.iter().map(|(k, v)| format!("{}: {}", k, v)));
                }
            }
            SearchScope::Body => {
                if let Some(body) = &req.body {
                    texts.push(String::from_utf8_lossy(body).to_string());
                }
                if let Some(resp) = resp {
                    if let Some(body) = &resp.body {
                        texts.push(String::from_utf8_lossy(body).to_string());
                    }
                }
            }
        }
        texts
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub session_ids: Vec<u64>,
    pub total: usize,
    pub query: String,
}
