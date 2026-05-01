use crate::engine_error::EngineError;
use crate::http_message::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

const PIPE_NAME: &str = r"\\.\pipe\FlowReveal-HookIPC";

#[derive(Debug, Clone, Deserialize)]
pub struct HookEvent {
    pub event_type: String,
    pub process_id: u32,
    pub thread_id: u32,
    pub handle_value: u64,
    pub method: Option<String>,
    pub url: Option<String>,
    pub headers: Option<Vec<(String, String)>>,
    pub body: Option<Vec<u8>>,
    pub status_code: Option<u16>,
    pub content_length: Option<usize>,
    pub timestamp: u64,
}

struct TrackedSession {
    session_id: u64,
    process_id: u32,
    method: String,
    url: String,
    request_headers: Vec<(String, String)>,
    request_body: Option<Vec<u8>>,
    status_code: Option<u16>,
    response_body: Vec<u8>,
    request_sent: bool,
    open_timestamp: u64,
}

pub struct ApiHookEngine {
    event_tx: mpsc::Sender<HttpMessage>,
    dll_path: PathBuf,
    injected_pids: Arc<Mutex<Vec<u32>>>,
    running: Arc<AtomicBool>,
    next_session_id: Arc<Mutex<u64>>,
}

pub struct ApiHookShutdownHandle {
    pub shutdown_tx: oneshot::Sender<()>,
}

impl ApiHookEngine {
    pub fn new(event_tx: mpsc::Sender<HttpMessage>) -> Self {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        let dll_path = exe_dir.join("hook_dll_v2.dll");

        Self {
            event_tx,
            dll_path,
            injected_pids: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(AtomicBool::new(false)),
            next_session_id: Arc::new(Mutex::new(1)),
        }
    }

    pub fn with_dll_path(mut self, path: PathBuf) -> Self {
        self.dll_path = path;
        self
    }

    pub async fn start(self) -> Result<ApiHookShutdownHandle, EngineError> {
        if !self.dll_path.exists() {
            tracing::warn!(
                "hook_dll.dll not found at: {} - DLL injection will not work until the file is available",
                self.dll_path.display()
            );
        }

        self.running.store(true, Ordering::Relaxed);

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let event_tx = self.event_tx.clone();
        let running = self.running.clone();
        let next_session_id = self.next_session_id.clone();

        tokio::spawn(async move {
            run_pipe_server(event_tx, running, next_session_id, shutdown_rx).await;
        });

        tracing::info!("API Hook engine started, pipe server listening on {}", PIPE_NAME);

        Ok(ApiHookShutdownHandle { shutdown_tx })
    }

    pub async fn inject(&self, pid: u32) -> Result<(), EngineError> {
        {
            let pids = self.injected_pids.lock().await;
            if pids.contains(&pid) {
                tracing::info!("Process {} already injected, skipping", pid);
                return Ok(());
            }
        }

        if !self.dll_path.exists() {
            return Err(EngineError::HookError(format!(
                "hook_dll.dll not found at: {}",
                self.dll_path.display()
            )));
        }

        inject_dll(pid, &self.dll_path)?;

        let mut pids = self.injected_pids.lock().await;
        pids.push(pid);

        tracing::info!("Injected hook DLL into process {}", pid);
        Ok(())
    }

    pub async fn eject(&self, pid: u32) -> Result<(), EngineError> {
        let mut pids = self.injected_pids.lock().await;
        pids.retain(|&p| p != pid);
        tracing::info!("Marked process {} for hook cleanup", pid);
        Ok(())
    }

    pub async fn eject_all(&self) -> Result<(), EngineError> {
        let mut pids = self.injected_pids.lock().await;
        pids.clear();
        Ok(())
    }

    pub async fn injected_pids(&self) -> Vec<u32> {
        self.injected_pids.lock().await.clone()
    }
}

async fn run_pipe_server(
    event_tx: mpsc::Sender<HttpMessage>,
    running: Arc<AtomicBool>,
    next_session_id: Arc<Mutex<u64>>,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    let (pipe_tx, mut pipe_rx) = mpsc::channel::<HookEvent>(4096);

    let pipe_running = running.clone();
    let pipe_handle = tokio::spawn(async move {
        pipe_accept_loop(pipe_tx, pipe_running).await;
    });

    let mut sessions: HashMap<(u32, u64), TrackedSession> = HashMap::new();

    loop {
        tokio::select! {
            event = pipe_rx.recv() => {
                match event {
                    Some(hook_event) => {
                        process_hook_event(&hook_event, &mut sessions, &event_tx, &next_session_id).await;
                    }
                    None => break,
                }
            }
            _ = &mut shutdown_rx => {
                break;
            }
        }
    }

    running.store(false, Ordering::Relaxed);
    pipe_handle.abort();
    tracing::info!("API Hook pipe server stopped");
}

#[cfg(target_os = "windows")]
async fn pipe_accept_loop(tx: mpsc::Sender<HookEvent>, running: Arc<AtomicBool>) {
    use tokio::net::windows::named_pipe::ServerOptions;

    let mut is_first = true;

    loop {
        if !running.load(Ordering::Relaxed) {
            break;
        }

        let server = if is_first {
            let result = ServerOptions::new()
                .first_pipe_instance(true)
                .max_instances(4)
                .create(PIPE_NAME);
            match result {
                Ok(s) => {
                    is_first = false;
                    s
                }
                Err(_) => {
                    let result = ServerOptions::new()
                        .first_pipe_instance(false)
                        .max_instances(4)
                        .create(PIPE_NAME);
                    match result {
                        Ok(s) => {
                            is_first = false;
                            s
                        }
                        Err(e) => {
                            if running.load(Ordering::Relaxed) {
                                tracing::warn!("Failed to create named pipe: {}, retrying...", e);
                                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            }
                            continue;
                        }
                    }
                }
            }
        } else {
            match ServerOptions::new()
                .first_pipe_instance(false)
                .max_instances(4)
                .create(PIPE_NAME)
            {
                Ok(s) => s,
                Err(e) => {
                    if running.load(Ordering::Relaxed) {
                        tracing::warn!("Failed to create named pipe: {}, retrying...", e);
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                    continue;
                }
            }
        };

        match server.connect().await {
            Ok(()) => {
                tracing::info!("Hook DLL client connected to pipe");
            }
            Err(e) => {
                if running.load(Ordering::Relaxed) {
                    tracing::debug!("Pipe connect error: {}", e);
                }
                continue;
            }
        }

        let tx_clone = tx.clone();
        let running_clone = running.clone();
        tokio::spawn(async move {
            read_pipe_events_async(server, tx_clone, running_clone).await;
        });
    }
}

#[cfg(not(target_os = "windows"))]
async fn pipe_accept_loop(_tx: mpsc::Sender<HookEvent>, _running: Arc<AtomicBool>) {
    tracing::error!("API Hook pipe server is only supported on Windows");
}

#[cfg(target_os = "windows")]
async fn read_pipe_events_async(
    mut server: tokio::net::windows::named_pipe::NamedPipeServer,
    tx: mpsc::Sender<HookEvent>,
    running: Arc<AtomicBool>,
) {
    use tokio::io::AsyncReadExt;

    while running.load(Ordering::Relaxed) {
        let mut len_buf = [0u8; 4];
        match server.read_exact(&mut len_buf).await {
            Ok(4) => {}
            _ => break,
        }

        let msg_len = u32::from_le_bytes(len_buf) as usize;
        if msg_len == 0 || msg_len > 16 * 1024 * 1024 {
            tracing::warn!("Invalid pipe message length: {}", msg_len);
            break;
        }

        let mut buf = vec![0u8; msg_len];
        match server.read_exact(&mut buf).await {
            Ok(_) => {}
            _ => break,
        }

        match serde_json::from_slice::<HookEvent>(&buf) {
            Ok(event) => {
                if tx.send(event).await.is_err() {
                    break;
                }
            }
            Err(e) => {
                tracing::warn!("Failed to deserialize hook event: {}", e);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
async fn read_pipe_events_async(
    _server: (),
    _tx: mpsc::Sender<HookEvent>,
    _running: Arc<AtomicBool>,
) {
}

async fn process_hook_event(
    event: &HookEvent,
    sessions: &mut HashMap<(u32, u64), TrackedSession>,
    event_tx: &mpsc::Sender<HttpMessage>,
    next_session_id: &Mutex<u64>,
) {
    let key = (event.process_id, event.handle_value);

    match event.event_type.as_str() {
        "open_request" => {
            let session_id = {
                let mut id = next_session_id.lock().await;
                *id += 1;
                *id
            };

            let method = event.method.clone().unwrap_or_default();
            let url = event.url.clone().unwrap_or_default();

            sessions.insert(
                key,
                TrackedSession {
                    session_id,
                    process_id: event.process_id,
                    method,
                    url,
                    request_headers: Vec::new(),
                    request_body: None,
                    status_code: None,
                    response_body: Vec::new(),
                    request_sent: false,
                    open_timestamp: event.timestamp,
                },
            );
        }
        "send_request" => {
            if let Some(session) = sessions.get_mut(&key) {
                if let Some(ref headers) = event.headers {
                    session.request_headers.extend(headers.iter().cloned());
                }
                if event.body.is_some() {
                    session.request_body = event.body.clone();
                }

                let msg = build_request_message(session);
                if event_tx.send(msg).await.is_err() {
                    return;
                }
                session.request_sent = true;
            }
        }
        "query_status" => {
            if let Some(session) = sessions.get_mut(&key) {
                session.status_code = event.status_code;
            }
        }
        "read_data" => {
            if let Some(session) = sessions.get_mut(&key) {
                if let Some(ref data) = event.body {
                    session.response_body.extend_from_slice(data);
                }
            }
        }
        "close_handle" => {
            if let Some(session) = sessions.remove(&key) {
                if session.request_sent {
                    let msg = build_response_message(&session);
                    let _ = event_tx.send(msg).await;
                }
            }
        }
        _ => {}
    }
}

fn build_request_message(session: &TrackedSession) -> HttpMessage {
    let scheme = if session.url.starts_with("https://") {
        Scheme::Https
    } else {
        Scheme::Http
    };

    let content_type = session
        .request_headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.clone());

    HttpMessage {
        id: session.session_id * 2,
        session_id: session.session_id,
        direction: MessageDirection::Request,
        protocol: HttpProtocol::HTTP1_1,
        scheme,
        method: Some(session.method.clone()),
        url: Some(session.url.clone()),
        status_code: None,
        status_text: None,
        headers: session.request_headers.clone(),
        body: session.request_body.clone(),
        body_size: session.request_body.as_ref().map(|b| b.len()).unwrap_or(0),
        body_truncated: false,
        content_type,
        process_id: Some(session.process_id),
        process_name: None,
        process_path: None,
        source_ip: None,
        dest_ip: None,
        source_port: None,
        dest_port: None,
        timestamp: session.open_timestamp,
        duration_us: None,
        cookies: Vec::new(),
        raw_tls_info: None,
    }
}

fn build_response_message(session: &TrackedSession) -> HttpMessage {
    let content_type = session
        .request_headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.clone());

    HttpMessage {
        id: session.session_id * 2 + 1,
        session_id: session.session_id,
        direction: MessageDirection::Response,
        protocol: HttpProtocol::HTTP1_1,
        scheme: Scheme::Http,
        method: None,
        url: None,
        status_code: session.status_code,
        status_text: session.status_code.map(|c| {
            http_status_text(c).map(|t| format!("{} {}", c, t)).unwrap_or_else(|| format!("{}", c))
        }),
        headers: Vec::new(),
        body: if session.response_body.is_empty() {
            None
        } else {
            Some(session.response_body.clone())
        },
        body_size: session.response_body.len(),
        body_truncated: false,
        content_type,
        process_id: Some(session.process_id),
        process_name: None,
        process_path: None,
        source_ip: None,
        dest_ip: None,
        source_port: None,
        dest_port: None,
        timestamp: session.open_timestamp,
        duration_us: None,
        cookies: Vec::new(),
        raw_tls_info: None,
    }
}

fn http_status_text(code: u16) -> Option<&'static str> {
    match code {
        200 => Some("OK"),
        201 => Some("Created"),
        204 => Some("No Content"),
        301 => Some("Moved Permanently"),
        302 => Some("Found"),
        304 => Some("Not Modified"),
        400 => Some("Bad Request"),
        401 => Some("Unauthorized"),
        403 => Some("Forbidden"),
        404 => Some("Not Found"),
        405 => Some("Method Not Allowed"),
        500 => Some("Internal Server Error"),
        502 => Some("Bad Gateway"),
        503 => Some("Service Unavailable"),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn inject_dll(pid: u32, dll_path: &PathBuf) -> Result<(), EngineError> {
    use windows::Win32::Foundation::*;
    use windows::Win32::System::Threading::*;
    use windows::Win32::System::Memory::*;
    use windows::Win32::System::Diagnostics::Debug::*;
    use windows::Win32::System::LibraryLoader::*;

    let dll_path_str = dll_path.to_string_lossy().to_string();
    let wide_dll_path: Vec<u16> = dll_path_str.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let process = OpenProcess(
            PROCESS_CREATE_THREAD | PROCESS_VM_OPERATION | PROCESS_VM_WRITE | PROCESS_QUERY_INFORMATION,
            false,
            pid,
        )
        .map_err(|e| EngineError::HookError(format!("OpenProcess({}) failed: {}", pid, e)))?;

        let path_size = wide_dll_path.len() * std::mem::size_of::<u16>();

        let remote_buf = VirtualAllocEx(
            process,
            None,
            path_size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );

        if remote_buf.is_null() {
            let _ = CloseHandle(process);
            return Err(EngineError::HookError(
                "VirtualAllocEx failed: returned null".to_string(),
            ));
        }

        let mut bytes_written: usize = 0;
        let write_result = WriteProcessMemory(
            process,
            remote_buf,
            wide_dll_path.as_ptr() as *const _,
            path_size,
            Some(&mut bytes_written),
        );

        if write_result.is_err() {
            let _ = VirtualFreeEx(process, remote_buf, 0, MEM_RELEASE);
            let _ = CloseHandle(process);
            return Err(EngineError::HookError(
                "WriteProcessMemory failed".to_string(),
            ));
        }

        let kernel32 = GetModuleHandleW(windows::core::w!("kernel32.dll"))
            .map_err(|e| EngineError::HookError(format!("GetModuleHandle failed: {}", e)))?;

        let load_library_addr = GetProcAddress(kernel32, windows::core::s!("LoadLibraryW"))
            .ok_or_else(|| EngineError::HookError("GetProcAddress LoadLibraryW failed".to_string()))?;

        let start_routine: LPTHREAD_START_ROUTINE = Some(std::mem::transmute(load_library_addr));

        let thread = CreateRemoteThread(
            process,
            None,
            0,
            start_routine,
            Some(remote_buf),
            0,
            None,
        )
        .map_err(|e| {
            let _ = VirtualFreeEx(process, remote_buf, 0, MEM_RELEASE);
            let _ = CloseHandle(process);
            EngineError::HookError(format!("CreateRemoteThread failed: {}", e))
        })?;

        let _ = WaitForSingleObject(thread, 5000);
        let _ = CloseHandle(thread);

        let _ = VirtualFreeEx(process, remote_buf, 0, MEM_RELEASE);
        let _ = CloseHandle(process);
    }

    tracing::info!("Successfully injected hook DLL into process {}", pid);
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn inject_dll(_pid: u32, _dll_path: &PathBuf) -> Result<(), EngineError> {
    Err(EngineError::HookError(
        "DLL injection is only supported on Windows".to_string(),
    ))
}

pub fn list_processes() -> Result<Vec<crate::process_info::ProcessInfo>, EngineError> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::*;
        use windows::Win32::System::Diagnostics::ToolHelp::*;

        let mut result = Vec::new();

        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            let snapshot = match snapshot {
                Ok(s) => s,
                Err(e) => return Err(EngineError::HookError(format!("CreateToolhelp32Snapshot failed: {}", e))),
            };

            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };

            if Process32FirstW(snapshot, &mut entry).is_ok() {
                loop {
                    let name = String::from_utf16_lossy(
                        &entry.szExeFile[..entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len())]
                    );

                    if !name.is_empty() {
                        result.push(crate::process_info::ProcessInfo {
                            pid: entry.th32ProcessID,
                            name,
                            path: None,
                            command_line: None,
                            icon_data: None,
                        });
                    }

                    if Process32NextW(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }

            let _ = CloseHandle(snapshot);
        }

        result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(result)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(Vec::new())
    }
}
