use serde::{Deserialize, Serialize};
use crate::process_info::ProcessInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxySettings {
    pub enabled: bool,
    pub server: String,
    pub port: u16,
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
            tracing::trace!(addr = local_addr, "[ProcessLookup] IPv4 地址解析失败");
            return None;
        }
    };

    let ip_native = u32::from_le_bytes(local_ip.octets());
    let port_native = (local_port as u16).to_be() as u32;

    tracing::trace!(
        addr = local_addr,
        port = local_port,
        "[ProcessLookup] IPv4 查询"
    );

    unsafe {
        let mut size: u32 = 0;
        let _ = GetExtendedTcpTable(None, &mut size, false, AF_INET.0 as u32, TCP_TABLE_OWNER_PID_ALL, 0);
        if size == 0 {
            tracing::debug!("[ProcessLookup] GetExtendedTcpTable(IPv4) 返回 size=0");
            return None;
        }

        let mut buffer = vec![0u8; size as usize];
        let result = GetExtendedTcpTable(
            Some(buffer.as_mut_ptr() as *mut _),
            &mut size,
            false,
            AF_INET.0 as u32,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        );
        if result != 0 {
            tracing::warn!(result, "[ProcessLookup] GetExtendedTcpTable(IPv4) 调用失败");
            return None;
        }

        let table = buffer.as_ptr() as *const MIB_TCPTABLE_OWNER_PID;
        let num_entries = (*table).dwNumEntries;

        let rows_ptr = std::ptr::addr_of!((*table).table) as *const MIB_TCPROW_OWNER_PID;
        for i in 0..num_entries {
            let row = &*rows_ptr.add(i as usize);
            if row.dwLocalAddr == ip_native && row.dwLocalPort == port_native {
                let pid = row.dwOwningPid;
                tracing::debug!(pid, addr = local_addr, port = local_port, "[ProcessLookup] IPv4 命中");
                return get_process_info(pid);
            }
        }

        tracing::trace!(num_entries, "[ProcessLookup] IPv4 未命中");
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
            tracing::trace!(addr = local_addr, "[ProcessLookup] IPv6 地址解析失败");
            return None;
        }
    };

    let ip_bytes = local_ip.octets();
    let port_native = (local_port as u16).to_be() as u32;

    tracing::trace!(
        addr = local_addr,
        port = local_port,
        "[ProcessLookup] IPv6 查询"
    );

    unsafe {
        let mut size: u32 = 0;
        let _ = GetExtendedTcpTable(None, &mut size, false, AF_INET6.0 as u32, TCP_TABLE_OWNER_PID_ALL, 0);
        if size == 0 {
            tracing::debug!("[ProcessLookup] GetExtendedTcpTable(IPv6) 返回 size=0");
            return None;
        }

        let mut buffer = vec![0u8; size as usize];
        let result = GetExtendedTcpTable(
            Some(buffer.as_mut_ptr() as *mut _),
            &mut size,
            false,
            AF_INET6.0 as u32,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        );
        if result != 0 {
            tracing::warn!(result, "[ProcessLookup] GetExtendedTcpTable(IPv6) 调用失败");
            return None;
        }

        let table = buffer.as_ptr() as *const MIB_TCP6TABLE_OWNER_PID;
        let num_entries = (*table).dwNumEntries;

        let rows_ptr = std::ptr::addr_of!((*table).table) as *const MIB_TCP6ROW_OWNER_PID;
        for i in 0..num_entries {
            let row = &*rows_ptr.add(i as usize);
            let row_ip_bytes = std::slice::from_raw_parts(
                row.ucLocalAddr.as_ptr() as *const u8,
                16,
            );
            if row_ip_bytes == ip_bytes && row.dwLocalPort == port_native {
                let pid = row.dwOwningPid;
                tracing::debug!(pid, addr = local_addr, port = local_port, "[ProcessLookup] IPv6 命中");
                return get_process_info(pid);
            }
        }

        tracing::trace!(num_entries, "[ProcessLookup] IPv6 未命中");
    }
    None
}

fn get_process_info(pid: u32) -> Option<ProcessInfo> {
    use windows::Win32::System::Threading::*;

    unsafe {
        let access = PROCESS_QUERY_LIMITED_INFORMATION;
        let handle = match OpenProcess(access, false, pid) {
            Ok(h) => h,
            Err(_) => {
                tracing::trace!(pid, "[ProcessLookup] OpenProcess 失败");
                return None;
            }
        };

        let name = get_process_name(handle);
        let path = get_process_path(handle);
        let username = get_process_username(handle);
        let is_64_bit = get_process_is_64_bit(handle);
        let _ = windows::Win32::Foundation::CloseHandle(handle);

        match name {
            Some(n) if !n.is_empty() => {
                let mut info = ProcessInfo::new(pid, n).with_path(path.unwrap_or_default());
                if let Some(u) = username {
                    info = info.with_username(u);
                }
                if let Some(b) = is_64_bit {
                    info = info.with_is_64_bit(b);
                }
                Some(info)
            }
            _ => None
        }
    }
}

fn get_process_username(handle: windows::Win32::Foundation::HANDLE) -> Option<String> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::{GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
    use windows::Win32::System::Threading::OpenProcessToken;

    unsafe {
        let mut token_handle: HANDLE = HANDLE::default();
        if OpenProcessToken(handle, TOKEN_QUERY, &mut token_handle).is_err() {
            return None;
        }

        let mut return_length: u32 = 0;
        let _ = GetTokenInformation(
            token_handle,
            TokenUser,
            None,
            0,
            &mut return_length,
        );

        if return_length == 0 {
            let _ = windows::Win32::Foundation::CloseHandle(token_handle);
            return None;
        }

        let mut buffer = vec![0u8; return_length as usize];
        if GetTokenInformation(
            token_handle,
            TokenUser,
            Some(buffer.as_mut_ptr() as *mut _),
            return_length,
            &mut return_length,
        ).is_err() {
            let _ = windows::Win32::Foundation::CloseHandle(token_handle);
            return None;
        }

        let _ = windows::Win32::Foundation::CloseHandle(token_handle);

        let token_user = buffer.as_ptr() as *const TOKEN_USER;
        let sid = (*token_user).User.Sid;

        let mut name_len: u32 = 0;
        let mut domain_len: u32 = 0;

        unsafe fn lookup_account_sid(
            sid: windows::Win32::Security::PSID,
            name_buf: Option<*mut u16>,
            name_len: *mut u32,
            domain_buf: Option<*mut u16>,
            domain_len: *mut u32,
        ) -> bool {
            #[link(name = "advapi32")]
            unsafe extern "system" {
                fn LookupAccountSidW(
                    lpsystemname: usize,
                    sid: windows::Win32::Security::PSID,
                    name: *mut u16,
                    cchname: *mut u32,
                    referenceddomainname: *mut u16,
                    cchreferenceddomainname: *mut u32,
                    peuse: *mut u32,
                ) -> i32;
            }
            let mut pe_use: u32 = 0;
            unsafe {
                LookupAccountSidW(
                    0,
                    sid,
                    name_buf.unwrap_or(std::ptr::null_mut()),
                    name_len,
                    domain_buf.unwrap_or(std::ptr::null_mut()),
                    domain_len,
                    &mut pe_use,
                ) != 0
            }
        }

        if !lookup_account_sid(sid, None, &mut name_len, None, &mut domain_len) {
            return None;
        }

        if name_len == 0 {
            return None;
        }

        let mut name_buf = vec![0u16; name_len as usize];
        let mut domain_buf = vec![0u16; domain_len as usize];

        if !lookup_account_sid(sid, Some(name_buf.as_mut_ptr()), &mut name_len, Some(domain_buf.as_mut_ptr()), &mut domain_len) {
            return None;
        }

        let domain = String::from_utf16_lossy(&domain_buf[..domain_len as usize]);
        let name = String::from_utf16_lossy(&name_buf[..name_len as usize]);

        if domain.is_empty() {
            Some(name)
        } else {
            Some(format!("{}\\{}", domain, name))
        }
    }
}

fn get_process_is_64_bit(handle: windows::Win32::Foundation::HANDLE) -> Option<bool> {
    use windows::Win32::System::Threading::IsWow64Process;

    unsafe {
        let mut is_wow64: i32 = 0;
        if IsWow64Process(handle, &mut is_wow64 as *mut _ as *mut _).is_ok() {
            Some(is_wow64 == 0)
        } else {
            None
        }
    }
}

fn get_process_name(handle: windows::Win32::Foundation::HANDLE) -> Option<String> {
    use windows::Win32::System::Threading::*;
    unsafe {
        let mut buffer = [0u16; 512];
        let mut size = buffer.len() as u32;
        if QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut size,
        ).is_err() {
            return None;
        }
        let path = String::from_utf16_lossy(&buffer[..size as usize]);
        path.rsplit('\\').next().map(|s| s.to_string())
    }
}

fn get_process_path(handle: windows::Win32::Foundation::HANDLE) -> Option<String> {
    use windows::Win32::System::Threading::*;
    unsafe {
        let mut buffer = [0u16; 512];
        let mut size = buffer.len() as u32;
        if QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut size,
        ).is_err() {
            return None;
        }
        let path = String::from_utf16_lossy(&buffer[..size as usize]);
        if path.is_empty() { None } else { Some(path) }
    }
}

pub fn install_ca_certificate(cert_pem: &str) -> Result<(), String> {
    use windows::Win32::Security::Cryptography::*;
    use windows::core::PCWSTR;

    let der = pem_to_der(cert_pem).map_err(|e| e.to_string())?;

    let fg_guard = std::thread::spawn(|| {
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::core::PCWSTR;

        let class_name = windows::core::HSTRING::from("#32770");
        for _ in 0..80 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            unsafe {
                let hwnd = match FindWindowW(PCWSTR(class_name.as_ptr()), PCWSTR::null()) {
                    Ok(h) if !h.is_invalid() => h,
                    _ => continue,
                };
                if !IsWindowVisible(hwnd).as_bool() {
                    continue;
                }
                let mut title = [0u16; 256];
                let title_len = GetWindowTextW(hwnd, &mut title);
                let title_str = String::from_utf16_lossy(&title[..title_len as usize]);
                let lower = title_str.to_lowercase();
                if lower.contains("certificate") || lower.contains("cert")
                    || lower.contains("安全") || lower.contains("安装")
                    || lower.contains("根证书") || lower.contains("root")
                    || lower.contains("trust") || lower.contains("warning")
                    || lower.contains("confirm")
                {
                    let _ = SetForegroundWindow(hwnd);
                    let _ = BringWindowToTop(hwnd);
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                    break;
                }
            }
        }
    });

    unsafe {
        let store_name = windows::core::HSTRING::from("Root");
        let h_store = CertOpenStore(
            CERT_STORE_PROV_SYSTEM_W,
            CERT_QUERY_ENCODING_TYPE(0),
            None,
            CERT_OPEN_STORE_FLAGS(CERT_SYSTEM_STORE_CURRENT_USER as u32),
            Some(PCWSTR(store_name.as_ptr()).as_ptr() as *const _),
        ).map_err(|e| format!("Failed to open certificate store: {}", e))?;

        let result = CertAddEncodedCertificateToStore(
            Some(h_store),
            X509_ASN_ENCODING | PKCS_7_ASN_ENCODING,
            &der,
            CERT_STORE_ADD_REPLACE_EXISTING,
            None,
        );

        let _ = CertCloseStore(Some(h_store), 0);

        if result.is_err() {
            return Err("Failed to add certificate to store".to_string());
        }

        tracing::info!("[CertInstall] CA 证书已安装到受信任根证书存储");
    }

    let _ = fg_guard.join();
    Ok(())
}

pub fn remove_ca_certificate(cert_pem: &str) -> Result<(), String> {
    use windows::Win32::Security::Cryptography::*;
    use windows::core::PCWSTR;

    let der = pem_to_der(cert_pem).map_err(|e| e.to_string())?;

    unsafe {
        let store_name = windows::core::HSTRING::from("Root");
        let h_store = CertOpenStore(
            CERT_STORE_PROV_SYSTEM_W,
            CERT_QUERY_ENCODING_TYPE(0),
            None,
            CERT_OPEN_STORE_FLAGS(CERT_SYSTEM_STORE_CURRENT_USER as u32),
            Some(PCWSTR(store_name.as_ptr()).as_ptr() as *const _),
        ).map_err(|e| format!("Failed to open certificate store: {}", e))?;

        let mut found = false;
        let mut cert_context = CertEnumCertificatesInStore(h_store, None);

        while !cert_context.is_null() {
            let cert_der = std::slice::from_raw_parts(
                (*cert_context).pbCertEncoded,
                (*cert_context).cbCertEncoded as usize,
            );

            if cert_der == der.as_slice() {
                let _ = CertDeleteCertificateFromStore(cert_context);
                found = true;
                break;
            }

            cert_context = CertEnumCertificatesInStore(h_store, Some(cert_context));
        }

        let _ = CertCloseStore(Some(h_store), 0);

        if found {
            tracing::info!("[CertInstall] CA 证书已从受信任根证书存储移除");
        } else {
            tracing::debug!("[CertInstall] 未找到匹配的 CA 证书");
        }
        Ok(())
    }
}

fn pem_to_der(pem: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let lines: Vec<&str> = pem.lines().collect();
    let b64: String = lines[1..lines.len().saturating_sub(1)]
        .iter()
        .map(|l| l.trim())
        .collect();
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| e.into())
}

pub fn set_system_proxy(proxy_addr: &str) -> Result<ProxySettings, String> {
    let original = get_system_proxy()?;

    let parts: Vec<&str> = proxy_addr.split(':').collect();
    let server = parts.first().unwrap_or(&"127.0.0.1").to_string();
    let port: u16 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(8080);

    set_system_proxy_internal(true, &server, port)?;

    tracing::info!(
        enabled = original.0,
        server = %original.1,
        port = original.2,
        "[SystemProxy] 系统代理已设置，原始配置已保存"
    );

    Ok(ProxySettings {
        enabled: original.0,
        server: original.1,
        port: original.2,
    })
}

pub fn restore_system_proxy(settings: &ProxySettings) -> Result<(), String> {
    if settings.enabled {
        set_system_proxy_internal(true, &settings.server, settings.port)?;
        tracing::info!(server = %settings.server, port = settings.port, "[SystemProxy] 系统代理已恢复");
    } else {
        set_system_proxy_internal(false, "", 0)?;
        tracing::info!("[SystemProxy] 系统代理已禁用");
    }
    Ok(())
}

pub fn clear_system_proxy() -> Result<(), String> {
    set_system_proxy_internal(false, "", 0)?;
    tracing::info!("[SystemProxy] 系统代理已清除");
    Ok(())
}

fn set_system_proxy_internal(enable: bool, server: &str, port: u16) -> Result<(), String> {
    use windows::Win32::System::Registry::*;
    use windows::core::{HSTRING, PCWSTR};

    let key_path = HSTRING::from(r"Software\Microsoft\Windows\CurrentVersion\Internet Settings");
    let mut h_key = Default::default();

    unsafe {
        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(key_path.as_ptr()),
            Some(0),
            KEY_SET_VALUE,
            &mut h_key,
        );

        if result.is_err() {
            return Err("Failed to open registry key".to_string());
        }

        if enable {
            let enable_val: u32 = 1;
            let server_str = format!("{}:{}", server, port);
            let server_wide = HSTRING::from(server_str.as_str());

            let _ = RegSetValueExW(
                h_key,
                PCWSTR(HSTRING::from("ProxyEnable").as_ptr()),
                None,
                REG_DWORD,
                Some(&enable_val.to_le_bytes()),
            );

            let server_bytes: Vec<u8> = std::slice::from_raw_parts(
                    server_wide.as_ptr() as *const u8,
                    server_wide.len() * 2 + 2,
                ).to_vec();
            let _ = RegSetValueExW(
                h_key,
                PCWSTR(HSTRING::from("ProxyServer").as_ptr()),
                None,
                REG_SZ,
                Some(&server_bytes),
            );
        } else {
            let enable_val: u32 = 0;
            let _ = RegSetValueExW(
                h_key,
                PCWSTR(HSTRING::from("ProxyEnable").as_ptr()),
                None,
                REG_DWORD,
                Some(&enable_val.to_le_bytes()),
            );
        }

        let _ = RegCloseKey(h_key);
        let _ = notify_proxy_change();
    }

    Ok(())
}

pub fn get_system_proxy() -> Result<(bool, String, u16), String> {
    use windows::Win32::System::Registry::*;
    use windows::core::{HSTRING, PCWSTR};

    let key_path = HSTRING::from(r"Software\Microsoft\Windows\CurrentVersion\Internet Settings");
    let mut h_key = Default::default();

    unsafe {
        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(key_path.as_ptr()),
            Some(0),
            KEY_QUERY_VALUE,
            &mut h_key,
        );

        if result.is_err() {
            return Err("Failed to open registry key".to_string());
        }

        let mut enable_val: u32 = 0;
        let mut size: u32 = 4;
        let _ = RegQueryValueExW(
            h_key,
            PCWSTR(HSTRING::from("ProxyEnable").as_ptr()),
            None,
            None,
            Some(&mut enable_val as *mut u32 as *mut u8),
            Some(&mut size),
        );

        let mut server_buf = [0u16; 256];
        let mut server_size: u32 = 512;
        let _ = RegQueryValueExW(
            h_key,
            PCWSTR(HSTRING::from("ProxyServer").as_ptr()),
            None,
            None,
            Some(server_buf.as_mut_ptr() as *mut u8),
            Some(&mut server_size),
        );

        let _ = RegCloseKey(h_key);

        let server_str = String::from_utf16_lossy(
            &server_buf[..server_size as usize / 2]
        );
        let server_str = server_str.trim_end_matches('\0');

        let (server, port) = if let Some(colon) = server_str.rfind(':') {
            (server_str[..colon].to_string(), server_str[colon+1..].parse().unwrap_or(8080))
        } else {
            (server_str.to_string(), 8080)
        };

        Ok((enable_val != 0, server, port))
    }
}

fn notify_proxy_change() -> Result<(), String> {
    use windows::Win32::Networking::WinInet::*;
    unsafe {
        let options = INTERNET_OPTION_SETTINGS_CHANGED;
        if InternetSetOptionW(None, options, None, 0).is_err() {
            return Err("InternetSetOptionW failed".to_string());
        }
        let refresh = INTERNET_OPTION_REFRESH;
        if InternetSetOptionW(None, refresh, None, 0).is_err() {
            return Err("InternetSetOptionW refresh failed".to_string());
        }
    }
    Ok(())
}

pub fn is_ca_certificate_installed() -> bool {
    use windows::Win32::Security::Cryptography::*;
    use windows::core::PCWSTR;

    unsafe {
        let store_name = windows::core::HSTRING::from("Root");
        let h_store = match CertOpenStore(
            CERT_STORE_PROV_SYSTEM_W,
            CERT_QUERY_ENCODING_TYPE(0),
            None,
            CERT_OPEN_STORE_FLAGS(CERT_SYSTEM_STORE_CURRENT_USER as u32),
            Some(PCWSTR(store_name.as_ptr()).as_ptr() as *const _),
        ) {
            Ok(s) => s,
            Err(_) => {
                tracing::debug!("[CertCheck] 无法打开证书存储");
                return false;
            }
        };

        let mut cert_context = CertEnumCertificatesInStore(h_store, None);

        while !cert_context.is_null() {
            let cert_der = std::slice::from_raw_parts(
                (*cert_context).pbCertEncoded,
                (*cert_context).cbCertEncoded as usize,
            );

            if cert_der.len() > 20 {
                let cn_check = String::from_utf8_lossy(cert_der);
                if cn_check.contains("FlowReveal") || cn_check.contains("flowreveal") {
                    let _ = CertCloseStore(Some(h_store), 0);
                    tracing::debug!("[CertCheck] 找到 FlowReveal CA 证书");
                    return true;
                }
            }

            cert_context = CertEnumCertificatesInStore(h_store, Some(cert_context));
        }

        let _ = CertCloseStore(Some(h_store), 0);
        tracing::trace!("[CertCheck] 未找到 FlowReveal CA 证书");
        false
    }
}

pub fn uninstall_ca_certificate() -> Result<(), String> {
    use windows::Win32::Security::Cryptography::*;
    use windows::core::PCWSTR;

    unsafe {
        let store_name = windows::core::HSTRING::from("Root");
        let h_store = CertOpenStore(
            CERT_STORE_PROV_SYSTEM_W,
            CERT_QUERY_ENCODING_TYPE(0),
            None,
            CERT_OPEN_STORE_FLAGS(CERT_SYSTEM_STORE_CURRENT_USER as u32),
            Some(PCWSTR(store_name.as_ptr()).as_ptr() as *const _),
        ).map_err(|e| format!("Failed to open certificate store: {}", e))?;

        let mut found = false;
        let mut cert_context = CertEnumCertificatesInStore(h_store, None);

        while !cert_context.is_null() {
            let cert_der = std::slice::from_raw_parts(
                (*cert_context).pbCertEncoded,
                (*cert_context).cbCertEncoded as usize,
            );

            if cert_der.len() > 20 {
                let cn_check = String::from_utf8_lossy(cert_der);
                if cn_check.contains("FlowReveal") || cn_check.contains("flowreveal") {
                    let _ = CertDeleteCertificateFromStore(cert_context);
                    found = true;
                    break;
                }
            }

            cert_context = CertEnumCertificatesInStore(h_store, Some(cert_context));
        }

        let _ = CertCloseStore(Some(h_store), 0);

        if found {
            tracing::info!("[CertInstall] CA 证书已从受信任根证书存储移除");
        } else {
            tracing::debug!("[CertInstall] 未找到 FlowReveal CA 证书");
        }
        Ok(())
    }
}
