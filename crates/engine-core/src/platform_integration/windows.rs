use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxySettings {
    pub proxy_enabled: bool,
    pub proxy_server: String,
    pub proxy_override: String,
}

pub fn set_system_proxy(proxy_addr: &str) -> Result<ProxySettings, String> {
    let original = get_current_proxy_settings()?;

    set_registry_proxy(true, proxy_addr, "<local>")?;

    notify_proxy_change();

    tracing::info!("System proxy set to {}", proxy_addr);
    Ok(original)
}

pub fn restore_system_proxy(original: &ProxySettings) -> Result<(), String> {
    set_registry_proxy(
        original.proxy_enabled,
        &original.proxy_server,
        &original.proxy_override,
    )?;

    notify_proxy_change();

    tracing::info!("System proxy restored to enabled={}, server={}", original.proxy_enabled, original.proxy_server);
    Ok(())
}

pub fn clear_system_proxy() -> Result<(), String> {
    set_registry_proxy(false, "", "")?;
    notify_proxy_change();
    tracing::info!("System proxy cleared");
    Ok(())
}

fn get_current_proxy_settings() -> Result<ProxySettings, String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings")
        .map_err(|e| format!("Failed to open registry: {}", e))?;

    let proxy_enabled: u32 = key.get_value("ProxyEnable").unwrap_or(0);
    let proxy_server: String = key.get_value("ProxyServer").unwrap_or_default();
    let proxy_override: String = key.get_value("ProxyOverride").unwrap_or_default();

    Ok(ProxySettings {
        proxy_enabled: proxy_enabled != 0,
        proxy_server,
        proxy_override,
    })
}

fn set_registry_proxy(enabled: bool, server: &str, override_str: &str) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
            KEY_SET_VALUE,
        )
        .map_err(|e| format!("Failed to open registry for writing: {}", e))?;

    let enable_val: u32 = if enabled { 1 } else { 0 };
    key.set_value("ProxyEnable", &enable_val)
        .map_err(|e| format!("Failed to set ProxyEnable: {}", e))?;

    if !server.is_empty() {
        key.set_value("ProxyServer", &server)
            .map_err(|e| format!("Failed to set ProxyServer: {}", e))?;
    }

    if !override_str.is_empty() {
        key.set_value("ProxyOverride", &override_str)
            .map_err(|e| format!("Failed to set ProxyOverride: {}", e))?;
    }

    Ok(())
}

fn notify_proxy_change() {
    use windows::Win32::Networking::WinInet::*;

    unsafe {
        let _ = InternetSetOptionW(None, INTERNET_OPTION_SETTINGS_CHANGED, None, 0);
        let _ = InternetSetOptionW(None, INTERNET_OPTION_REFRESH, None, 0);
    }
}

pub fn install_ca_certificate(cert_pem: &str) -> Result<(), String> {
    let temp_dir = std::env::temp_dir();
    let cert_path = temp_dir.join("FlowReveal-CA.crt");
    std::fs::write(&cert_path, cert_pem)
        .map_err(|e| format!("Failed to write cert file: {}", e))?;

    let output = std::process::Command::new("certutil.exe")
        .args(["-addstore", "-user", "Root", &cert_path.to_string_lossy()])
        .output()
        .map_err(|e| format!("Failed to run certutil: {}", e))?;

    let _ = std::fs::remove_file(&cert_path);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        tracing::info!("CA certificate installed to current user's Trusted Root store");
        Ok(())
    } else if stdout.contains("already in the store") || stdout.contains("already exists") {
        tracing::info!("CA certificate already exists in the store");
        Ok(())
    } else {
        tracing::warn!("certutil output: {}", stdout);
        tracing::warn!("certutil stderr: {}", stderr);
        Err(format!("certutil failed: {}", stdout.trim()))
    }
}

pub fn uninstall_ca_certificate() -> Result<(), String> {
    let output = std::process::Command::new("certutil.exe")
        .args(["-delstore", "-user", "Root", "FlowReveal CA"])
        .output()
        .map_err(|e| format!("Failed to run certutil: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    tracing::debug!("certutil del output: {}", stdout);

    if output.status.success() {
        tracing::info!("CA certificate removed from current user's Trusted Root store");
    } else {
        tracing::debug!("certutil del may have failed (cert may not exist): {}", stdout.trim());
    }

    Ok(())
}

pub fn is_ca_certificate_installed() -> bool {
    let output = std::process::Command::new("certutil.exe")
        .args(["-store", "-user", "Root", "FlowReveal CA"])
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout.contains("FlowReveal CA") && stdout.contains("Cert")
        }
        Err(_) => false,
    }
}
