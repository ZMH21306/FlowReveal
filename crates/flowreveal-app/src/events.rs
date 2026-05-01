use tauri::{AppHandle, Emitter};
use engine_core::http_message::HttpMessage;

pub fn emit_traffic_event(app: &AppHandle, msg: &HttpMessage) {
    if let Err(e) = app.emit("traffic:request", msg) {
        tracing::warn!("Failed to emit traffic event: {}", e);
    }
}
