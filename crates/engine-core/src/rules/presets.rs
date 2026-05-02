use super::rule_engine::RuleEngine;
use super::rule_types::*;

impl RuleEngine {
    pub fn preset_cors_enable() -> Rule {
        Rule {
            id: 0,
            name: "添加 CORS 标头".to_string(),
            category: RuleCategory::HeaderModifier,
            enabled: true,
            priority: 10,
            match_condition: MatchCondition {
                logic: MatchLogic::And,
                filters: vec![],
            },
            action: RuleAction::HeaderModifier(HeaderModifierAction {
                request_actions: vec![],
                response_actions: vec![
                    HeaderAction::Add {
                        name: "Access-Control-Allow-Origin".to_string(),
                        value: "*".to_string(),
                        only_if_missing: true,
                    },
                    HeaderAction::Add {
                        name: "Access-Control-Allow-Methods".to_string(),
                        value: "GET, POST, PUT, DELETE, PATCH, OPTIONS".to_string(),
                        only_if_missing: true,
                    },
                    HeaderAction::Add {
                        name: "Access-Control-Allow-Headers".to_string(),
                        value: "*".to_string(),
                        only_if_missing: true,
                    },
                    HeaderAction::Add {
                        name: "Access-Control-Max-Age".to_string(),
                        value: "86400".to_string(),
                        only_if_missing: true,
                    },
                ],
            }),
            created_at: 0,
            updated_at: 0,
        }
    }

    pub fn preset_cache_disable() -> Rule {
        Rule {
            id: 0,
            name: "禁用缓存".to_string(),
            category: RuleCategory::HeaderModifier,
            enabled: true,
            priority: 10,
            match_condition: MatchCondition {
                logic: MatchLogic::And,
                filters: vec![],
            },
            action: RuleAction::HeaderModifier(HeaderModifierAction {
                request_actions: vec![
                    HeaderAction::Add {
                        name: "Cache-Control".to_string(),
                        value: "no-cache, no-store, must-revalidate".to_string(),
                        only_if_missing: false,
                    },
                    HeaderAction::Add {
                        name: "Pragma".to_string(),
                        value: "no-cache".to_string(),
                        only_if_missing: false,
                    },
                ],
                response_actions: vec![
                    HeaderAction::Add {
                        name: "Cache-Control".to_string(),
                        value: "no-cache, no-store, must-revalidate".to_string(),
                        only_if_missing: false,
                    },
                    HeaderAction::Add {
                        name: "Pragma".to_string(),
                        value: "no-cache".to_string(),
                        only_if_missing: false,
                    },
                    HeaderAction::Add {
                        name: "Expires".to_string(),
                        value: "0".to_string(),
                        only_if_missing: false,
                    },
                ],
            }),
            created_at: 0,
            updated_at: 0,
        }
    }

    pub fn preset_cookies_remove() -> Rule {
        Rule {
            id: 0,
            name: "删除 Cookies".to_string(),
            category: RuleCategory::HeaderModifier,
            enabled: true,
            priority: 10,
            match_condition: MatchCondition {
                logic: MatchLogic::And,
                filters: vec![],
            },
            action: RuleAction::HeaderModifier(HeaderModifierAction {
                request_actions: vec![
                    HeaderAction::Remove {
                        name: "Cookie".to_string(),
                    },
                ],
                response_actions: vec![
                    HeaderAction::Remove {
                        name: "Set-Cookie".to_string(),
                    },
                ],
            }),
            created_at: 0,
            updated_at: 0,
        }
    }

    pub fn preset_503_service_unavailable() -> Rule {
        Rule {
            id: 0,
            name: "503 服务不可用".to_string(),
            category: RuleCategory::AutoReply,
            enabled: true,
            priority: 50,
            match_condition: MatchCondition {
                logic: MatchLogic::And,
                filters: vec![],
            },
            action: RuleAction::AutoReply(AutoReplyAction {
                status_code: 503,
                status_text: "Service Unavailable".to_string(),
                headers: vec![
                    ("Content-Type".to_string(), "text/plain".to_string()),
                ],
                body_source: BodySource::Inline("Service Unavailable (FlowReveal Mock)".to_string()),
                delay_ms: 0,
            }),
            created_at: 0,
            updated_at: 0,
        }
    }

    pub fn preset_302_redirect() -> Rule {
        Rule {
            id: 0,
            name: "302 重定向".to_string(),
            category: RuleCategory::AutoReply,
            enabled: true,
            priority: 50,
            match_condition: MatchCondition {
                logic: MatchLogic::And,
                filters: vec![],
            },
            action: RuleAction::AutoReply(AutoReplyAction {
                status_code: 302,
                status_text: "Found".to_string(),
                headers: vec![
                    ("Location".to_string(), "https://example.com".to_string()),
                ],
                body_source: BodySource::Empty,
                delay_ms: 0,
            }),
            created_at: 0,
            updated_at: 0,
        }
    }

    pub fn preset_200_ok() -> Rule {
        Rule {
            id: 0,
            name: "200 OK".to_string(),
            category: RuleCategory::AutoReply,
            enabled: true,
            priority: 50,
            match_condition: MatchCondition {
                logic: MatchLogic::And,
                filters: vec![],
            },
            action: RuleAction::AutoReply(AutoReplyAction {
                status_code: 200,
                status_text: "OK".to_string(),
                headers: vec![
                    ("Content-Type".to_string(), "text/plain".to_string()),
                ],
                body_source: BodySource::Inline("OK (FlowReveal Mock)".to_string()),
                delay_ms: 0,
            }),
            created_at: 0,
            updated_at: 0,
        }
    }

    pub fn get_preset(preset: &PresetRuleType) -> Rule {
        match preset {
            PresetRuleType::CorsEnable => Self::preset_cors_enable(),
            PresetRuleType::CacheDisable => Self::preset_cache_disable(),
            PresetRuleType::CookiesRemove => Self::preset_cookies_remove(),
            PresetRuleType::ServiceUnavailable503 => Self::preset_503_service_unavailable(),
            PresetRuleType::Redirect302 => Self::preset_302_redirect(),
            PresetRuleType::Ok200 => Self::preset_200_ok(),
        }
    }
}
