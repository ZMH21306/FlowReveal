use tauri::command;

#[command]
pub async fn install_cert() -> Result<(), String> {
    tracing::info!("Certificate installation requested (not yet implemented)");
    Err("Certificate installation not yet implemented".to_string())
}

#[command]
pub async fn uninstall_cert() -> Result<(), String> {
    tracing::info!("Certificate uninstallation requested (not yet implemented)");
    Err("Certificate uninstallation not yet implemented".to_string())
}
