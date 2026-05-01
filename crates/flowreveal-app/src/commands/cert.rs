use tauri::{command, State};
use crate::state::AppState;
use engine_core::mitm::{CaManager, CertificateAuthority};

#[command]
pub async fn get_ca_cert_pem(state: State<'_, AppState>) -> Result<String, String> {
    let ca_manager = state.ca_manager.read().await;
    match ca_manager.as_ref() {
        Some(m) => Ok(m.ca_cert_pem().to_string()),
        None => {
            drop(ca_manager);
            let mut guard = state.ca_manager.write().await;
            match guard.as_ref() {
                Some(m) => Ok(m.ca_cert_pem().to_string()),
                None => {
                    let app_data_dir = dirs::data_local_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                        .join("FlowReveal");
                    let manager = CaManager::load_or_create(&app_data_dir)
                        .map_err(|e| format!("Failed to create CA: {}", e))?;
                    let pem = manager.ca_cert_pem().to_string();
                    *guard = Some(manager);
                    Ok(pem)
                }
            }
        }
    }
}

#[command]
pub async fn get_ca_info(state: State<'_, AppState>) -> Result<CertificateAuthority, String> {
    let ca_manager = state.ca_manager.read().await;
    match ca_manager.as_ref() {
        Some(m) => Ok(m.ca_certificate_authority()),
        None => {
            drop(ca_manager);
            let mut guard = state.ca_manager.write().await;
            match guard.as_ref() {
                Some(m) => Ok(m.ca_certificate_authority()),
                None => {
                    let app_data_dir = dirs::data_local_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                        .join("FlowReveal");
                    let manager = CaManager::load_or_create(&app_data_dir)
                        .map_err(|e| format!("Failed to create CA: {}", e))?;
                    let info = manager.ca_certificate_authority();
                    *guard = Some(manager);
                    Ok(info)
                }
            }
        }
    }
}

#[command]
pub async fn install_cert() -> Result<(), String> {
    tracing::info!("Certificate installation requested - please install the CA cert manually");
    Err("Please export the CA certificate and install it to your system's Trusted Root store manually".to_string())
}

#[command]
pub async fn uninstall_cert() -> Result<(), String> {
    tracing::info!("Certificate uninstallation requested - please remove the CA cert manually");
    Err("Please remove the CA certificate from your system's Trusted Root store manually".to_string())
}
