use tauri::{command, State};
use crate::state::AppState;
use engine_core::rules::rule_types::{PresetRuleType, Rule, RuleId};

#[command]
pub async fn add_rule(state: State<'_, AppState>, rule: Rule) -> Result<RuleId, String> {
    state.rule_engine.add_rule(rule).await;
    Ok(state.rule_engine.get_rules().await.last().map(|r| r.id).unwrap_or(0))
}

#[command]
pub async fn remove_rule(state: State<'_, AppState>, id: RuleId) -> Result<bool, String> {
    Ok(state.rule_engine.remove_rule(id).await)
}

#[command]
pub async fn toggle_rule(state: State<'_, AppState>, id: RuleId, enabled: bool) -> Result<bool, String> {
    Ok(state.rule_engine.toggle_rule(id, enabled).await)
}

#[command]
pub async fn update_rule(state: State<'_, AppState>, id: RuleId, rule: Rule) -> Result<bool, String> {
    Ok(state.rule_engine.update_rule(id, rule).await)
}

#[command]
pub async fn get_rules(state: State<'_, AppState>) -> Result<Vec<Rule>, String> {
    Ok(state.rule_engine.get_rules().await)
}

#[command]
pub async fn enable_preset_rule(state: State<'_, AppState>, preset: PresetRuleType) -> Result<RuleId, String> {
    let rule = engine_core::rules::RuleEngine::get_preset(&preset);
    let id = state.rule_engine.add_rule(rule).await;
    Ok(id)
}

#[command]
pub async fn export_rules(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.rule_engine.export_rules().await)
}

#[command]
pub async fn import_rules(state: State<'_, AppState>, json: String) -> Result<usize, String> {
    state.rule_engine.import_rules(&json).await.map_err(|e| e)
}

#[command]
pub async fn clear_rules(state: State<'_, AppState>) -> Result<(), String> {
    state.rule_engine.clear_rules().await;
    Ok(())
}
