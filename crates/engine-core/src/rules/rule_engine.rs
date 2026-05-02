use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::http_message::HttpMessage;
use super::executor::RuleExecutionResult;
use super::matcher::RuleMatcher;
use super::rule_types::{Rule, RuleId};

pub struct RuleEngine {
    rules: Arc<RwLock<Vec<Rule>>>,
    rule_counter: AtomicU64,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            rule_counter: AtomicU64::new(1),
        }
    }

    pub async fn add_rule(&self, mut rule: Rule) -> RuleId {
        let id = self.rule_counter.fetch_add(1, Ordering::Relaxed);
        rule.id = id;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        rule.created_at = now;
        rule.updated_at = now;
        self.rules.write().await.push(rule);
        tracing::info!("[RuleEngine] 规则已添加 id={} name={}", id, self.rules.read().await.last().map(|r| r.name.as_str()).unwrap_or(""));
        id
    }

    pub async fn remove_rule(&self, id: RuleId) -> bool {
        let mut rules = self.rules.write().await;
        let before = rules.len();
        rules.retain(|r| r.id != id);
        let removed = rules.len() < before;
        if removed {
            tracing::info!("[RuleEngine] 规则已删除 id={}", id);
        }
        removed
    }

    pub async fn toggle_rule(&self, id: RuleId, enabled: bool) -> bool {
        let mut rules = self.rules.write().await;
        if let Some(rule) = rules.iter_mut().find(|r| r.id == id) {
            rule.enabled = enabled;
            tracing::info!("[RuleEngine] 规则已{} id={} name={}", if enabled { "启用" } else { "禁用" }, id, rule.name);
            true
        } else {
            false
        }
    }

    pub async fn update_rule(&self, id: RuleId, updated: Rule) -> bool {
        let mut rules = self.rules.write().await;
        if let Some(rule) = rules.iter_mut().find(|r| r.id == id) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            *rule = updated;
            rule.id = id;
            rule.updated_at = now;
            tracing::info!("[RuleEngine] 规则已更新 id={}", id);
            true
        } else {
            false
        }
    }

    pub async fn get_rules(&self) -> Vec<Rule> {
        let mut rules = self.rules.read().await.clone();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        rules
    }

    pub async fn apply(
        &self,
        request: &HttpMessage,
        response: Option<&HttpMessage>,
    ) -> Option<RuleExecutionResult> {
        let rules = self.rules.read().await;
        let mut sorted: Vec<&Rule> = rules.iter().filter(|r| r.enabled).collect();
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));

        for rule in sorted {
            if RuleMatcher::matches(rule, request, response) {
                tracing::info!(
                    "[RuleEngine] 规则匹配: name={} id={} category={:?}",
                    rule.name,
                    rule.id,
                    rule.category
                );
                return Some(RuleExecutor::execute(&rule.action));
            }
        }
        None
    }

    pub async fn export_rules(&self) -> String {
        let rules = self.get_rules().await;
        serde_json::to_string_pretty(&rules).unwrap_or_default()
    }

    pub async fn import_rules(&self, json: &str) -> Result<usize, String> {
        let imported: Vec<Rule> = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let count = imported.len();
        let mut rules = self.rules.write().await;
        for mut rule in imported {
            rule.id = self.rule_counter.fetch_add(1, Ordering::Relaxed);
            rules.push(rule);
        }
        tracing::info!("[RuleEngine] 导入了 {} 条规则", count);
        Ok(count)
    }

    pub async fn clear_rules(&self) {
        self.rules.write().await.clear();
        tracing::info!("[RuleEngine] 所有规则已清除");
    }
}

use super::executor::RuleExecutor;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::rule_types::*;

    fn make_test_rule(name: &str, priority: u32, field: MatchField, value: &str) -> Rule {
        Rule {
            id: 0,
            name: name.to_string(),
            category: RuleCategory::AutoReply,
            enabled: true,
            priority,
            match_condition: MatchCondition {
                logic: MatchLogic::And,
                filters: vec![MatchFilter {
                    field,
                    operator: MatchOperator::Contains,
                    value: value.to_string(),
                    case_sensitive: false,
                }],
            },
            action: RuleAction::AutoReply(AutoReplyAction {
                status_code: 200,
                status_text: "OK".to_string(),
                headers: vec![],
                body_source: BodySource::Inline("mocked".to_string()),
                delay_ms: 0,
            }),
            created_at: 0,
            updated_at: 0,
        }
    }

    #[tokio::test]
    async fn test_add_and_get_rules() {
        let engine = RuleEngine::new();
        let _id1 = engine.add_rule(make_test_rule("Rule1", 10, MatchField::Host, "example.com")).await;
        let _id2 = engine.add_rule(make_test_rule("Rule2", 20, MatchField::Host, "test.com")).await;
        let rules = engine.get_rules().await;
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].priority, 20);
        assert_eq!(rules[1].priority, 10);
    }

    #[tokio::test]
    async fn test_remove_rule() {
        let engine = RuleEngine::new();
        let id = engine.add_rule(make_test_rule("Rule1", 10, MatchField::Host, "example.com")).await;
        assert!(engine.remove_rule(id).await);
        assert_eq!(engine.get_rules().await.len(), 0);
    }

    #[tokio::test]
    async fn test_toggle_rule() {
        let engine = RuleEngine::new();
        let id = engine.add_rule(make_test_rule("Rule1", 10, MatchField::Host, "example.com")).await;
        assert!(engine.toggle_rule(id, false).await);
        let rules = engine.get_rules().await;
        assert!(!rules[0].enabled);
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let engine = RuleEngine::new();
        engine.add_rule(make_test_rule("Low", 1, MatchField::Host, "example.com")).await;
        engine.add_rule(make_test_rule("High", 100, MatchField::Host, "example.com")).await;

        let req = HttpMessage {
            method: Some("GET".to_string()),
            url: Some("https://example.com".to_string()),
            headers: vec![("Host".to_string(), "example.com".to_string())],
            ..Default::default()
        };

        let result = engine.apply(&req, None).await;
        assert!(result.is_some());
        if let Some(RuleExecutionResult::AutoReply { body, .. }) = result {
            assert_eq!(body, b"mocked");
        }
    }

    #[tokio::test]
    async fn test_export_import_roundtrip() {
        let engine = RuleEngine::new();
        engine.add_rule(make_test_rule("Rule1", 10, MatchField::Host, "example.com")).await;

        let json = engine.export_rules().await;

        let engine2 = RuleEngine::new();
        let count = engine2.import_rules(&json).await.unwrap();
        assert_eq!(count, 1);
        let rules = engine2.get_rules().await;
        assert_eq!(rules[0].name, "Rule1");
    }
}
