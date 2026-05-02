use tauri::{command, State};
use crate::state::AppState;
use engine_core::http_message::HttpSession;
use engine_core::har_export;

#[command]
pub async fn get_requests(
    state: State<'_, AppState>,
    offset: usize,
    limit: usize,
) -> Result<Vec<HttpSession>, String> {
    let sessions = state.sessions.read().await;
    let result: Vec<HttpSession> = sessions
        .iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();
    Ok(result)
}

#[command]
pub async fn get_sessions(state: State<'_, AppState>) -> Result<Vec<HttpSession>, String> {
    let sessions = state.sessions.read().await;
    Ok(sessions.iter().cloned().collect())
}

#[command]
pub async fn export_har(
    state: State<'_, AppState>,
    session_ids: Vec<u64>,
) -> Result<String, String> {
    let sessions = state.sessions.read().await;
    let selected: Vec<&HttpSession> = if session_ids.is_empty() {
        sessions.iter().collect()
    } else {
        session_ids
            .iter()
            .filter_map(|id| sessions.iter().find(|s| s.id == *id))
            .collect()
    };

    let har = har_export::sessions_to_har(&selected);
    serde_json::to_string_pretty(&har).map_err(|e| format!("Failed to serialize HAR: {}", e))
}

#[command]
pub async fn replay_request(
    state: State<'_, AppState>,
    session_id: u64,
) -> Result<String, String> {
    let sessions = state.sessions.read().await;
    let session = sessions
        .iter()
        .find(|s| s.id == session_id)
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    match engine_core::replay::replay_session(session).await {
        Ok((status, body)) => Ok(format!(
            "Replay completed: status={}, body_size={}",
            status,
            body.len()
        )),
        Err(e) => Err(format!("Replay failed: {}", e)),
    }
}
