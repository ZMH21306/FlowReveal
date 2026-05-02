use serde::{Deserialize, Serialize};

pub type RuleId = u64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RuleCategory {
    AutoReply,
    HeaderModifier,
    Redirect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: RuleId,
    pub name: String,
    pub category: RuleCategory,
    pub enabled: bool,
    pub priority: u32,
    pub match_condition: MatchCondition,
    pub action: RuleAction,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchCondition {
    pub logic: MatchLogic,
    pub filters: Vec<MatchFilter>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchLogic {
    And,
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchFilter {
    pub field: MatchField,
    pub operator: MatchOperator,
    pub value: String,
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchField {
    Method,
    Url,
    Host,
    Path,
    StatusCode,
    ContentType,
    HeaderName,
    HeaderValue,
    Body,
    ProcessName,
    Scheme,
    QueryParam,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    StartsWith,
    EndsWith,
    MatchesRegex,
    GreaterThan,
    LessThan,
    InRange,
    Wildcard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    AutoReply(AutoReplyAction),
    HeaderModifier(HeaderModifierAction),
    Redirect(RedirectAction),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoReplyAction {
    pub status_code: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body_source: BodySource,
    pub delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BodySource {
    Inline(String),
    File(String),
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderModifierAction {
    pub request_actions: Vec<HeaderAction>,
    pub response_actions: Vec<HeaderAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HeaderAction {
    Add {
        name: String,
        value: String,
        only_if_missing: bool,
    },
    Remove {
        name: String,
    },
    Replace {
        name: String,
        value: String,
    },
    ReplaceRegex {
        name: String,
        pattern: String,
        replacement: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedirectAction {
    pub target_url: String,
    pub redirect_type: RedirectType,
    pub preserve_query: bool,
    pub preserve_path: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RedirectType {
    Permanent301,
    Temporary302,
    Temporary307,
    Permanent308,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PresetRuleType {
    CorsEnable,
    CacheDisable,
    CookiesRemove,
    ServiceUnavailable503,
    Redirect302,
    Ok200,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_roundtrip() {
        let rule = Rule {
            id: 1,
            name: "CORS Enable".to_string(),
            category: RuleCategory::HeaderModifier,
            enabled: true,
            priority: 10,
            match_condition: MatchCondition {
                logic: MatchLogic::And,
                filters: vec![MatchFilter {
                    field: MatchField::Host,
                    operator: MatchOperator::Contains,
                    value: "api.example.com".to_string(),
                    case_sensitive: false,
                }],
            },
            action: RuleAction::HeaderModifier(HeaderModifierAction {
                request_actions: vec![],
                response_actions: vec![HeaderAction::Add {
                    name: "Access-Control-Allow-Origin".to_string(),
                    value: "*".to_string(),
                    only_if_missing: true,
                }],
            }),
            created_at: 0,
            updated_at: 0,
        };
        let json = serde_json::to_string(&rule).unwrap();
        let de: Rule = serde_json::from_str(&json).unwrap();
        assert_eq!(de.id, rule.id);
        assert_eq!(de.name, rule.name);
        assert_eq!(de.category, rule.category);
        assert_eq!(de.enabled, rule.enabled);
    }

    #[test]
    fn test_auto_reply_action_serialization() {
        let action = AutoReplyAction {
            status_code: 503,
            status_text: "Service Unavailable".to_string(),
            headers: vec![("X-Custom".to_string(), "value".to_string())],
            body_source: BodySource::Inline("Maintenance".to_string()),
            delay_ms: 100,
        };
        let json = serde_json::to_string(&action).unwrap();
        let de: AutoReplyAction = serde_json::from_str(&json).unwrap();
        assert_eq!(de.status_code, 503);
        assert_eq!(de.delay_ms, 100);
    }

    #[test]
    fn test_redirect_action_serialization() {
        let action = RedirectAction {
            target_url: "https://new.example.com".to_string(),
            redirect_type: RedirectType::Temporary302,
            preserve_query: true,
            preserve_path: true,
        };
        let json = serde_json::to_string(&action).unwrap();
        let de: RedirectAction = serde_json::from_str(&json).unwrap();
        assert_eq!(de.target_url, "https://new.example.com");
        assert!(de.preserve_query);
    }
}
