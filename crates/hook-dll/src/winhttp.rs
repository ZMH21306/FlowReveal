use crate::pipe_client::{HookEvent, send_event};
use once_cell::sync::Lazy;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Mutex;
use windows::Win32::Networking::WinHttp::*;
use windows::core::PCWSTR;

type FnOpenRequest = unsafe fn(*mut c_void, PCWSTR, PCWSTR, PCWSTR, PCWSTR, *const PCWSTR, u32) -> *mut c_void;
type FnSendRequest = unsafe fn(*mut c_void, PCWSTR, u32, *const c_void, u32, u32, usize) -> i32;
type FnReceiveResponse = unsafe fn(*mut c_void, *mut c_void) -> i32;
type FnReadData = unsafe fn(*mut c_void, *mut c_void, u32, *mut u32) -> i32;
type FnCloseHandle = unsafe fn(*mut c_void) -> i32;
type FnQueryHeaders = unsafe fn(*mut c_void, u32, PCWSTR, *mut c_void, *mut u32, *mut u32) -> i32;

struct RequestInfo {
    #[allow(dead_code)]
    method: String,
    #[allow(dead_code)]
    url: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

static REQUEST_MAP: Lazy<Mutex<HashMap<u64, RequestInfo>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

struct OriginalFns {
    open_request: Option<FnOpenRequest>,
    send_request: Option<FnSendRequest>,
    receive_response: Option<FnReceiveResponse>,
    read_data: Option<FnReadData>,
    close_handle: Option<FnCloseHandle>,
    query_headers: Option<FnQueryHeaders>,
}

struct SyncUnsafeCell<T>(UnsafeCell<T>);

unsafe impl<T> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    fn get(&self) -> *mut T {
        self.0.get()
    }
}

static ORIGINAL_FNS: SyncUnsafeCell<OriginalFns> = SyncUnsafeCell::new(OriginalFns {
    open_request: None,
    send_request: None,
    receive_response: None,
    read_data: None,
    close_handle: None,
    query_headers: None,
});

unsafe fn get_fns() -> &'static mut OriginalFns {
    unsafe { &mut *ORIGINAL_FNS.get() }
}

fn timestamp_us() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}

fn read_wide_string(ptr: PCWSTR) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe {
        let mut len = 0u32;
        while *ptr.0.add(len as usize) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr.0, len as usize))
    }
}

fn read_wide_string_with_len(ptr: PCWSTR, len: u32) -> String {
    if ptr.is_null() || len == 0 {
        return String::new();
    }
    unsafe { String::from_utf16_lossy(std::slice::from_raw_parts(ptr.0, len as usize)) }
}

pub fn install() -> Result<(), String> {
    unsafe {
        let status = minhook_sys::MH_Initialize();
        if status != 0 && status != 5 {
            return Err(format!("MH_Initialize failed: {}", status));
        }

        let hmodule = windows::Win32::System::LibraryLoader::GetModuleHandleW(
            windows::core::w!("winhttp.dll"),
        )
        .map_err(|e| format!("GetModuleHandle failed: {}", e))?;

        let open_request_addr = windows::Win32::System::LibraryLoader::GetProcAddress(hmodule, windows::core::s!("WinHttpOpenRequest"))
            .ok_or("Failed to find WinHttpOpenRequest")?;
        let send_request_addr = windows::Win32::System::LibraryLoader::GetProcAddress(hmodule, windows::core::s!("WinHttpSendRequest"))
            .ok_or("Failed to find WinHttpSendRequest")?;
        let receive_response_addr = windows::Win32::System::LibraryLoader::GetProcAddress(hmodule, windows::core::s!("WinHttpReceiveResponse"))
            .ok_or("Failed to find WinHttpReceiveResponse")?;
        let read_data_addr = windows::Win32::System::LibraryLoader::GetProcAddress(hmodule, windows::core::s!("WinHttpReadData"))
            .ok_or("Failed to find WinHttpReadData")?;
        let close_handle_addr = windows::Win32::System::LibraryLoader::GetProcAddress(hmodule, windows::core::s!("WinHttpCloseHandle"))
            .ok_or("Failed to find WinHttpCloseHandle")?;
        let query_headers_addr = windows::Win32::System::LibraryLoader::GetProcAddress(hmodule, windows::core::s!("WinHttpQueryHeaders"))
            .ok_or("Failed to find WinHttpQueryHeaders")?;

        let mut orig_open: *mut c_void = std::ptr::null_mut();
        let mut orig_send: *mut c_void = std::ptr::null_mut();
        let mut orig_receive: *mut c_void = std::ptr::null_mut();
        let mut orig_read: *mut c_void = std::ptr::null_mut();
        let mut orig_close: *mut c_void = std::ptr::null_mut();
        let mut orig_query: *mut c_void = std::ptr::null_mut();

        let r = minhook_sys::MH_CreateHook(open_request_addr as *mut c_void, hooked_open_request as *mut c_void, &mut orig_open);
        if r != 0 { return Err(format!("MH_CreateHook WinHttpOpenRequest failed: {}", r)); }
        let r = minhook_sys::MH_CreateHook(send_request_addr as *mut c_void, hooked_send_request as *mut c_void, &mut orig_send);
        if r != 0 { return Err(format!("MH_CreateHook WinHttpSendRequest failed: {}", r)); }
        let r = minhook_sys::MH_CreateHook(receive_response_addr as *mut c_void, hooked_receive_response as *mut c_void, &mut orig_receive);
        if r != 0 { return Err(format!("MH_CreateHook WinHttpReceiveResponse failed: {}", r)); }
        let r = minhook_sys::MH_CreateHook(read_data_addr as *mut c_void, hooked_read_data as *mut c_void, &mut orig_read);
        if r != 0 { return Err(format!("MH_CreateHook WinHttpReadData failed: {}", r)); }
        let r = minhook_sys::MH_CreateHook(close_handle_addr as *mut c_void, hooked_close_handle as *mut c_void, &mut orig_close);
        if r != 0 { return Err(format!("MH_CreateHook WinHttpCloseHandle failed: {}", r)); }
        let r = minhook_sys::MH_CreateHook(query_headers_addr as *mut c_void, hooked_query_headers as *mut c_void, &mut orig_query);
        if r != 0 { return Err(format!("MH_CreateHook WinHttpQueryHeaders failed: {}", r)); }

        let fns = get_fns();
        fns.open_request = Some(std::mem::transmute(orig_open));
        fns.send_request = Some(std::mem::transmute(orig_send));
        fns.receive_response = Some(std::mem::transmute(orig_receive));
        fns.read_data = Some(std::mem::transmute(orig_read));
        fns.close_handle = Some(std::mem::transmute(orig_close));
        fns.query_headers = Some(std::mem::transmute(orig_query));

        minhook_sys::MH_EnableHook(std::ptr::null_mut());
    }

    Ok(())
}

pub fn uninstall() {
    unsafe {
        minhook_sys::MH_DisableHook(std::ptr::null_mut());
        minhook_sys::MH_Uninitialize();
    }
}

unsafe fn hooked_open_request(
    hconnect: *mut c_void,
    verb: PCWSTR,
    object_name: PCWSTR,
    version: PCWSTR,
    referrer: PCWSTR,
    accept_types: *const PCWSTR,
    flags: u32,
) -> *mut c_void {
    let original = match unsafe { get_fns() }.open_request {
        Some(f) => f,
        None => return std::ptr::null_mut(),
    };

    let result = unsafe { original(hconnect, verb, object_name, version, referrer, accept_types, flags) };

    if !result.is_null() {
        let method = read_wide_string(verb);
        let path = read_wide_string(object_name);
        let handle_val = result as u64;

        let event = HookEvent {
            event_type: "open_request".to_string(),
            process_id: std::process::id(),
            thread_id: unsafe { windows::Win32::System::Threading::GetCurrentThreadId() },
            handle_value: handle_val,
            method: Some(method.clone()),
            url: Some(path.clone()),
            headers: None,
            body: None,
            status_code: None,
            content_length: None,
            timestamp: timestamp_us(),
        };
        send_event(&event);

        if let Ok(mut map) = REQUEST_MAP.try_lock() {
            map.insert(handle_val, RequestInfo {
                method,
                url: path,
                headers: Vec::new(),
                body: None,
            });
        }
    }

    result
}

unsafe fn hooked_send_request(
    hrequest: *mut c_void,
    headers: PCWSTR,
    headers_len: u32,
    optional: *const c_void,
    optional_len: u32,
    total_len: u32,
    context: usize,
) -> i32 {
    let original = match unsafe { get_fns() }.send_request {
        Some(f) => f,
        None => return 0,
    };

    let handle_val = hrequest as u64;

    if !headers.is_null() && headers_len > 0 {
        let header_str = read_wide_string_with_len(headers, headers_len);
        if let Ok(mut map) = REQUEST_MAP.try_lock() {
            if let Some(info) = map.get_mut(&handle_val) {
                for line in header_str.split("\r\n") {
                    if let Some(colon_pos) = line.find(':') {
                        let name = line[..colon_pos].trim().to_string();
                        let value = line[colon_pos + 1..].trim().to_string();
                        info.headers.push((name, value));
                    }
                }
            }
        }
    }

    let mut body_data: Option<Vec<u8>> = None;
    if !optional.is_null() && optional_len > 0 {
        let body_slice = unsafe { std::slice::from_raw_parts(optional as *const u8, optional_len as usize) };
        body_data = Some(body_slice.to_vec());
        if let Ok(mut map) = REQUEST_MAP.try_lock() {
            if let Some(info) = map.get_mut(&handle_val) {
                info.body = body_data.clone();
            }
        }
    }

    let event = HookEvent {
        event_type: "send_request".to_string(),
        process_id: std::process::id(),
        thread_id: unsafe { windows::Win32::System::Threading::GetCurrentThreadId() },
        handle_value: handle_val,
        method: None,
        url: None,
        headers: None,
        body: body_data,
        status_code: None,
        content_length: Some(optional_len as usize),
        timestamp: timestamp_us(),
    };
    send_event(&event);

    unsafe { original(hrequest, headers, headers_len, optional, optional_len, total_len, context) }
}

unsafe fn hooked_receive_response(
    hrequest: *mut c_void,
    reserved: *mut c_void,
) -> i32 {
    let original = match unsafe { get_fns() }.receive_response {
        Some(f) => f,
        None => return 0,
    };

    let result = unsafe { original(hrequest, reserved) };

    if result != 0 {
        let event = HookEvent {
            event_type: "receive_response".to_string(),
            process_id: std::process::id(),
            thread_id: unsafe { windows::Win32::System::Threading::GetCurrentThreadId() },
            handle_value: hrequest as u64,
            method: None,
            url: None,
            headers: None,
            body: None,
            status_code: None,
            content_length: None,
            timestamp: timestamp_us(),
        };
        send_event(&event);
    }

    result
}

unsafe fn hooked_query_headers(
    hrequest: *mut c_void,
    info_level: u32,
    name: PCWSTR,
    buffer: *mut c_void,
    buffer_len: *mut u32,
    index: *mut u32,
) -> i32 {
    let original = match unsafe { get_fns() }.query_headers {
        Some(f) => f,
        None => return 0,
    };

    let result = unsafe { original(hrequest, info_level, name, buffer, buffer_len, index) };

    if result != 0 {
        if info_level == WINHTTP_QUERY_STATUS_CODE {
            if !buffer.is_null() && !buffer_len.is_null() {
                let len = unsafe { *buffer_len } / 2;
                let status_str = read_wide_string_with_len(PCWSTR(buffer as *const u16), len);
                if let Ok(status) = status_str.trim().parse::<u16>() {
                    let event = HookEvent {
                        event_type: "query_status".to_string(),
                        process_id: std::process::id(),
                        thread_id: unsafe { windows::Win32::System::Threading::GetCurrentThreadId() },
                        handle_value: hrequest as u64,
                        method: None,
                        url: None,
                        headers: None,
                        body: None,
                        status_code: Some(status),
                        content_length: None,
                        timestamp: timestamp_us(),
                    };
                    send_event(&event);
                }
            }
        }
    }

    result
}

unsafe fn hooked_read_data(
    hrequest: *mut c_void,
    buffer: *mut c_void,
    bytes_to_read: u32,
    bytes_read: *mut u32,
) -> i32 {
    let original = match unsafe { get_fns() }.read_data {
        Some(f) => f,
        None => return 0,
    };

    let result = unsafe { original(hrequest, buffer, bytes_to_read, bytes_read) };

    if result != 0 && !buffer.is_null() && !bytes_read.is_null() {
        let read_len = unsafe { *bytes_read };
        if read_len > 0 {
            let data = unsafe { std::slice::from_raw_parts(buffer as *const u8, read_len as usize) }.to_vec();

            let event = HookEvent {
                event_type: "read_data".to_string(),
                process_id: std::process::id(),
                thread_id: unsafe { windows::Win32::System::Threading::GetCurrentThreadId() },
                handle_value: hrequest as u64,
                method: None,
                url: None,
                headers: None,
                body: Some(data),
                status_code: None,
                content_length: Some(read_len as usize),
                timestamp: timestamp_us(),
            };
            send_event(&event);
        }
    }

    result
}

unsafe fn hooked_close_handle(handle: *mut c_void) -> i32 {
    let original = match unsafe { get_fns() }.close_handle {
        Some(f) => f,
        None => return 0,
    };

    let handle_val = handle as u64;

    if let Ok(mut map) = REQUEST_MAP.try_lock() {
        map.remove(&handle_val);
    }

    let event = HookEvent {
        event_type: "close_handle".to_string(),
        process_id: std::process::id(),
        thread_id: unsafe { windows::Win32::System::Threading::GetCurrentThreadId() },
        handle_value: handle_val,
        method: None,
        url: None,
        headers: None,
        body: None,
        status_code: None,
        content_length: None,
        timestamp: timestamp_us(),
    };
    send_event(&event);

    unsafe { original(handle) }
}
