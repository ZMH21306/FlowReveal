use serde::{Deserialize, Serialize};

const IF_TYPE_IEEE80211: u32 = 71;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterInfo {
    pub name: String,
    pub friendly_name: String,
    pub is_wifi: bool,
    pub is_primary: bool,
    pub if_type: u32,
    pub mtu: u32,
}

pub fn is_wifi_adapter() -> bool {
    detect_primary_adapter().map(|a| a.is_wifi).unwrap_or(false)
}

pub fn detect_primary_adapter() -> Option<AdapterInfo> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::NetworkManagement::IpHelper::{GetIfTable2, FreeMibTable, MIB_IF_TABLE2};

        unsafe {
            let mut table_ptr: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
            let hr = GetIfTable2(&mut table_ptr);
            if hr.is_err() || table_ptr.is_null() {
                tracing::debug!("[WifiDetect] GetIfTable2 failed");
                return None;
            }

            let table = &*table_ptr;
            let num_entries = table.NumEntries;
            let rows_ptr = table.Table.as_ptr();

            for i in 0..num_entries {
                let row = &*rows_ptr.add(i as usize);
                let if_type = row.Type;

                let is_wifi = if_type == IF_TYPE_IEEE80211;

                let friendly_name = String::from_utf16_lossy(
                    &row.Alias[..row.Alias.iter().position(|&c| c == 0).unwrap_or(row.Alias.len())]
                );

                let adapter_name = String::from_utf16_lossy(
                    &row.Description[..row.Description.iter().position(|&c| c == 0).unwrap_or(row.Description.len())]
                );

                let oper_status = row.OperStatus;

                if oper_status.0 == 1 {
                    let info = AdapterInfo {
                        name: adapter_name,
                        friendly_name,
                        is_wifi,
                        is_primary: row.MediaConnectState.0 == 1,
                        if_type,
                        mtu: row.Mtu,
                    };

                    if is_wifi {
                        tracing::warn!(
                            name = %info.name,
                            friendly_name = %info.friendly_name,
                            "[WifiDetect] Wi-Fi adapter detected — fast-path may cause issues"
                        );
                    }

                    FreeMibTable(table_ptr as *mut _);
                    return Some(info);
                }
            }

            FreeMibTable(table_ptr as *mut _);
        }

        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

pub fn detect_all_adapters() -> Vec<AdapterInfo> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::NetworkManagement::IpHelper::{GetIfTable2, FreeMibTable, MIB_IF_TABLE2};

        let mut result = Vec::new();

        unsafe {
            let mut table_ptr: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
            let hr = GetIfTable2(&mut table_ptr);
            if hr.is_err() || table_ptr.is_null() {
                return result;
            }

            let table = &*table_ptr;
            let num_entries = table.NumEntries;
            let rows_ptr = table.Table.as_ptr();

            for i in 0..num_entries {
                let row = &*rows_ptr.add(i as usize);

                if row.OperStatus.0 == 1 {
                    let friendly_name = String::from_utf16_lossy(
                        &row.Alias[..row.Alias.iter().position(|&c| c == 0).unwrap_or(row.Alias.len())]
                    );
                    let adapter_name = String::from_utf16_lossy(
                        &row.Description[..row.Description.iter().position(|&c| c == 0).unwrap_or(row.Description.len())]
                    );

                    result.push(AdapterInfo {
                        name: adapter_name,
                        friendly_name,
                        is_wifi: row.Type == IF_TYPE_IEEE80211,
                        is_primary: row.MediaConnectState.0 == 1,
                        if_type: row.Type,
                        mtu: row.Mtu,
                    });
                }
            }

            FreeMibTable(table_ptr as *mut _);
        }

        result
    }

    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}
