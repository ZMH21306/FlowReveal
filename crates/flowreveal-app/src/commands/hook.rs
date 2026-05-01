use tauri::{command, State};
use crate::state::AppState;
use engine_core::platform_integration::api_hook::{ApiHookEngine, list_processes};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub path: Option<String>,
    pub is_injected: bool,
}

#[command]
pub async fn list_running_processes() -> Result<Vec<ProcessEntry>, String> {
    let processes = list_processes().map_err(|e| format!("Failed to list processes: {}", e))?;

    let entries: Vec<ProcessEntry> = processes
        .into_iter()
        .filter(|p| p.pid != 0 && p.pid != std::process::id())
        .map(|p| ProcessEntry {
            pid: p.pid,
            name: p.name,
            path: p.path,
            is_injected: false,
        })
        .collect();

    Ok(entries)
}

#[command]
pub async fn inject_hook(
    state: State<'_, AppState>,
    pid: u32,
) -> Result<(), String> {
    {
        let status = state.capture_status.read().await;
        if *status != engine_core::engine_stats::CaptureStatus::Running {
            return Err("Capture is not running - start API Hook mode first".to_string());
        }
    }

    let config = state.config.read().await;
    if let Some(cfg) = config.as_ref() {
        if cfg.mode != engine_core::capture_config::CaptureMode::ApiHook {
            return Err("Current mode is not API Hook - cannot inject".to_string());
        }
    } else {
        return Err("No capture configuration found".to_string());
    }
    drop(config);

    let event_tx = state.event_tx.lock().await;
    let tx = event_tx.as_ref().ok_or("No event channel available")?.clone();
    drop(event_tx);

    let hook_engine = ApiHookEngine::new(tx);
    hook_engine.inject(pid).await.map_err(|e| format!("Injection failed: {}", e))?;

    {
        let mut pids = state.hook_injected_pids.lock().await;
        if !pids.contains(&pid) {
            pids.push(pid);
        }
    }

    tracing::info!("Hook DLL injected into process {}", pid);
    Ok(())
}

#[command]
pub async fn eject_hook(
    state: State<'_, AppState>,
    pid: u32,
) -> Result<(), String> {
    {
        let mut pids = state.hook_injected_pids.lock().await;
        pids.retain(|&p| p != pid);
    }

    tracing::info!("Hook DLL ejected from process {}", pid);
    Ok(())
}

#[command]
pub async fn get_injected_pids(state: State<'_, AppState>) -> Result<Vec<u32>, String> {
    let pids = state.hook_injected_pids.lock().await;
    Ok(pids.clone())
}
