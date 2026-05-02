use tauri::{command, State};
use crate::state::AppState;
use engine_core::filter::DslParser;
use engine_core::search::search_engine::SearchScope;
use engine_core::stats::StatsCollector;

#[command]
pub async fn search_traffic(
    state: State<'_, AppState>,
    query: String,
    scope: String,
    case_sensitive: bool,
    use_regex: bool,
) -> Result<Vec<u64>, String> {
    let scope = match scope.as_str() {
        "Url" => SearchScope::Url,
        "Headers" => SearchScope::Headers,
        "Body" => SearchScope::Body,
        _ => SearchScope::All,
    };
    let engine = engine_core::search::SearchEngine::new(state.sessions.clone());
    let results = engine.search(&query, &scope, case_sensitive, use_regex).await;
    Ok(results)
}

#[command]
pub async fn filter_traffic_dsl(
    state: State<'_, AppState>,
    dsl_expression: String,
) -> Result<Vec<u64>, String> {
    let expr = DslParser::parse(&dsl_expression).map_err(|e| e.to_string())?;
    let sessions = state.sessions.read().await;
    let results: Vec<u64> = sessions
        .iter()
        .filter(|s| engine_core::filter::match_dsl(&expr, s))
        .map(|s| s.id)
        .collect();
    Ok(results)
}

#[command]
pub async fn get_traffic_stats(state: State<'_, AppState>) -> Result<engine_core::stats::TrafficStats, String> {
    let sessions = state.sessions.read().await;
    Ok(StatsCollector::collect(&sessions))
}
