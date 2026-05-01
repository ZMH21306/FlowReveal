use tauri::{command, State};
use crate::state::AppState;
use engine_core::http_message::HttpMessage;

#[command]
pub async fn get_requests(
    state: State<'_, AppState>,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Result<Vec<HttpMessage>, String> {
    let sessions = state.sessions.read().await;
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(50);

    let result: Vec<HttpMessage> = sessions
        .iter()
        .skip(offset)
        .take(limit)
        .map(|s| s.request.clone())
        .collect();

    Ok(result)
}

#[command]
pub async fn get_sessions(
    state: State<'_, AppState>,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Result<Vec<engine_core::http_message::HttpSession>, String> {
    let sessions = state.sessions.read().await;
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(50);

    let result: Vec<engine_core::http_message::HttpSession> = sessions
        .iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();

    Ok(result)
}
