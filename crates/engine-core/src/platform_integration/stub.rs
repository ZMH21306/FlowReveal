use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxySettings {
    pub proxy_enabled: bool,
    pub proxy_server: String,
    pub proxy_override: String,
}

pub fn set_system_proxy(_proxy_addr: &str) -> Result<ProxySettings, String> {
    Err("System proxy configuration is only supported on Windows".to_string())
}

pub fn restore_system_proxy(_original: &ProxySettings) -> Result<(), String> {
    Err("System proxy configuration is only supported on Windows".to_string())
}

pub fn clear_system_proxy() -> Result<(), String> {
    Err("System proxy configuration is only supported on Windows".to_string())
}

pub fn install_ca_certificate(_cert_pem: &str) -> Result<(), String> {
    Err("Certificate installation is only supported on Windows".to_string())
}

pub fn uninstall_ca_certificate() -> Result<(), String> {
    Err("Certificate removal is only supported on Windows".to_string())
}

pub fn is_ca_certificate_installed() -> bool {
    false
}
