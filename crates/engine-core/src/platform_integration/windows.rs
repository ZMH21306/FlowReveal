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

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub path: Option<String>,
}

pub fn find_process_by_connection(local_addr: &str, local_port: u16) -> Option<ProcessInfo> {
    if let Some(info) = find_process_by_ipv4(local_addr, local_port) {
        return Some(info);
    }
    find_process_by_ipv6(local_addr, local_port)
}

fn find_process_by_ipv4(local_addr: &str, local_port: u16) -> Option<ProcessInfo> {
    use windows::Win32::NetworkManagement::IpHelper::*;
    use windows::Win32::Networking::WinSock::*;
    use std::net::Ipv4Addr;

    let local_ip: Ipv4Addr = match local_addr.parse() {
        Ok(ip) => ip,
        Err(_) => {
            tracing::debug!("[ProcessLookup] IPv4解析失败: {}", local_addr);
            return None;
        }
    };

    let ip_native = u32::from_le_bytes(local_ip.octets());
    let port_native = (local_port as u16).to_be() as u32;

    tracing::info!("[ProcessLookup] IPv4查询目标 {}:{} | ip_native=0x{:08X} port_native=0x{:08X}", local_addr, local_port, ip_native, port_native);

    unsafe {
        let mut size: u32 = 0;
        let _ = GetExtendedTcpTable(None, &mut size, false, AF_INET.0 as u32, TCP_TABLE_OWNER_PID_ALL, 0);
        if size == 0 {
            tracing::warn!("[ProcessLookup] IPv4 TCP表大小为0");
            return None;
        }

        let mut buffer = vec![0u8; size as usize];
        let result = GetExtendedTcpTable(Some(buffer.as_mut_ptr() as *mut _), &mut size, false, AF_INET.0 as u32, TCP_TABLE_OWNER_PID_ALL, 0);
        if result != 0 {
            tracing::warn!("[ProcessLookup] GetExtendedTcpTable(IPv4)失败: {}", result);
            return None;
        }

        let table = buffer.as_ptr() as *const MIB_TCPTABLE_OWNER_PID;
        let num_entries = (*table).dwNumEntries;
        let rows_ptr = std::ptr::addr_of!((*table).table) as *const MIB_TCPROW_OWNER_PID;

        tracing::info!("[ProcessLookup] IPv4 TCP表共{}条", num_entries);

        let mut match_count = 0u32;
        for i in 0..num_entries {
            let row = &*rows_ptr.add(i as usize);

            let row_ip = row.dwLocalAddr;
            let row_port = row.dwLocalPort;
            let row_pid = row.dwOwningPid;

            if i < 10 {
                let row_ip_le = u32::from_le_bytes(row_ip.to_ne_bytes());
                let row_ip_str_le = format!("{}.{}.{}.{}",
                    (row_ip_le >> 24) as u8,
                    ((row_ip_le >> 16) & 0xFF) as u8,
                    ((row_ip_le >> 8) & 0xFF) as u8,
                    (row_ip_le & 0xFF) as u8,
                );
                let row_ip_str_raw = format!("{}.{}.{}.{}",
                    (row_ip >> 24) as u8,
                    ((row_ip >> 16) & 0xFF) as u8,
                    ((row_ip >> 8) & 0xFF) as u8,
                    (row_ip & 0xFF) as u8,
                );
                let row_port_val = (row_port >> 16) as u16;
                let row_port_le = u32::from_le(row_port) >> 16;
                tracing::info!("[ProcessLookup]   行[{}] ip_raw=0x{:08X}({}) ip_le=0x{:08X}({}) port_raw=0x{:08X}(>>16={}) port_le_shifted={} pid={}",
                    i, row_ip, row_ip_str_raw, row_ip_le, row_ip_str_le, row_port, row_port_val, row_port_le, row_pid);
            }

            let ip_match = row_ip == ip_native;
            let port_match = row_port == port_native;

            if ip_match && port_match {
                tracing::info!("[ProcessLookup] ✓ IPv4命中 PID={}", row_pid);
                return get_process_info(row_pid);
            }
            if ip_match { match_count += 1; }
        }

        tracing::info!("[ProcessLookup] IPv4未命中 (ip匹配{}条但port无匹配)", match_count);
    }
    None
}

fn find_process_by_ipv6(local_addr: &str, local_port: u16) -> Option<ProcessInfo> {
    use windows::Win32::NetworkManagement::IpHelper::*;
    use windows::Win32::Networking::WinSock::*;
    use std::net::Ipv6Addr;

    let local_ip: Ipv6Addr = match local_addr.parse() {
        Ok(ip) => ip,
        Err(_) => {
            tracing::debug!("[ProcessLookup] IPv6解析失败: {}", local_addr);
            return None;
        }
    };

    let port_native = (local_port as u16).to_be() as u32;
    let ip_bytes = local_ip.octets();

    tracing::info!("[ProcessLookup] IPv6查询目标 [{}]:{} | port_native=0x{:08X}", local_addr, local_port, port_native);

    unsafe {
        let mut size: u32 = 0;
        let _ = GetExtendedTcpTable(None, &mut size, false, AF_INET6.0 as u32, TCP_TABLE_OWNER_PID_ALL, 0);
        if size == 0 {
            tracing::info!("[ProcessLookup] IPv6 TCP表大小为0");
            return None;
        }

        let mut buffer = vec![0u8; size as usize];
        let result = GetExtendedTcpTable(Some(buffer.as_mut_ptr() as *mut _), &mut size, false, AF_INET6.0 as u32, TCP_TABLE_OWNER_PID_ALL, 0);
        if result != 0 {
            tracing::warn!("[ProcessLookup] GetExtendedTcpTable(IPv6)失败: {}", result);
            return None;
        }

        let table = buffer.as_ptr() as *const MIB_TCP6TABLE_OWNER_PID;
        let num_entries = (*table).dwNumEntries;
        let rows_ptr = std::ptr::addr_of!((*table).table) as *const MIB_TCP6ROW_OWNER_PID;

        tracing::info!("[ProcessLookup] IPv6 TCP表共{}条", num_entries);

        for i in 0..num_entries {
            let row = &*rows_ptr.add(i as usize);
            let row_addr = &row.ucLocalAddr;

            let ip_match = row_addr[..] == ip_bytes;
            let port_match = row.dwLocalPort == port_native;

            if ip_match {
                let row_port_val = (row.dwLocalPort >> 16) as u16;
                tracing::info!("[ProcessLookup]   IPv6行[{}] ip匹配! port=0x{:08X}(解码={}) pid={} port_match={}",
                    i, row.dwLocalPort, row_port_val, row.dwOwningPid, port_match);
            }

            if ip_match && port_match {
                tracing::info!("[ProcessLookup] ✓ IPv6命中 PID={}", row.dwOwningPid);
                return get_process_info(row.dwOwningPid);
            }
        }

        tracing::info!("[ProcessLookup] IPv6未命中");
    }
    None
}

fn get_process_info(pid: u32) -> Option<ProcessInfo> {
    use windows::Win32::System::Diagnostics::ToolHelp::*;

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        let mut entry = PROCESSENTRY32::default();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

        if Process32First(snapshot, &mut entry).is_ok() {
            loop {
                if entry.th32ProcessID == pid {
                    let name_bytes: Vec<u8> = entry.szExeFile
                        .iter()
                        .take_while(|&&c| c != 0)
                        .map(|&c| c as u8)
                        .collect();
                    let name = String::from_utf8_lossy(&name_bytes).to_string();
                    let path = get_process_path(pid);
                    return Some(ProcessInfo { pid, name, path });
                }
                if Process32Next(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
    }

    None
}

fn get_process_path(pid: u32) -> Option<String> {
    use windows::Win32::System::Threading::*;

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 512];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut size,
        );
        if ok.is_ok() && size > 0 {
            Some(String::from_utf16_lossy(&buf[..size as usize]))
        } else {
            None
        }
    }
}
