use serde::Serialize;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::mpsc::{self, Sender};
use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::*;
use windows::core::PCWSTR;

const PIPE_NAME: &str = r"\\.\pipe\FlowReveal-HookIPC";

#[derive(Debug, Clone, Serialize)]
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

static PIPE_HANDLE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static SENDER_PTR: AtomicPtr<Sender<HookEvent>> = AtomicPtr::new(std::ptr::null_mut());
static PIPE_THREAD_STARTED: AtomicBool = AtomicBool::new(false);

fn ensure_sender() -> Option<&'static Sender<HookEvent>> {
    let ptr = SENDER_PTR.load(Ordering::Acquire);
    if !ptr.is_null() {
        unsafe { Some(&*ptr) }
    } else {
        None
    }
}

fn start_pipe_thread() {
    if PIPE_THREAD_STARTED.swap(true, Ordering::AcqRel) {
        return;
    }

    let (tx, rx) = mpsc::channel::<HookEvent>();

    let boxed = Box::new(tx);
    let raw = Box::into_raw(boxed);
    SENDER_PTR.store(raw, Ordering::Release);

    std::thread::spawn(move || {
        pipe_writer_loop(rx);
    });
}

fn write_event_to_pipe(event: &HookEvent) -> bool {
    let json = match serde_json::to_string(event) {
        Ok(j) => j,
        Err(_) => return false,
    };

    let bytes = json.as_bytes();
    let len = bytes.len() as u32;
    let mut written: u32 = 0;

    let ptr = PIPE_HANDLE.load(Ordering::Relaxed);
    if ptr.is_null() {
        return false;
    }
    let handle = HANDLE(ptr);

    let len_result = unsafe {
        WriteFile(handle, Some(&len.to_le_bytes()), Some(&mut written), None)
    };
    if len_result.is_err() {
        return false;
    }

    let data_result = unsafe {
        WriteFile(handle, Some(bytes), Some(&mut written), None)
    };
    if data_result.is_err() {
        return false;
    }

    true
}

fn pipe_writer_loop(rx: mpsc::Receiver<HookEvent>) {
    loop {
        let event = match rx.recv() {
            Ok(e) => e,
            Err(_) => break,
        };

        if !ensure_pipe_connected() {
            continue;
        }

        if !write_event_to_pipe(&event) {
            disconnect_pipe();
            continue;
        }

        while let Ok(event) = rx.try_recv() {
            if !ensure_pipe_connected() {
                break;
            }
            if !write_event_to_pipe(&event) {
                disconnect_pipe();
                break;
            }
        }
    }

    disconnect_pipe();
    PIPE_THREAD_STARTED.store(false, Ordering::Relaxed);
}

fn ensure_pipe_connected() -> bool {
    let ptr = PIPE_HANDLE.load(Ordering::Relaxed);
    if !ptr.is_null() {
        return true;
    }

    let wide_name: Vec<u16> = PIPE_NAME.encode_utf16().chain(std::iter::once(0)).collect();

    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide_name.as_ptr()),
            GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    };

    match handle {
        Ok(h) => {
            PIPE_HANDLE.store(h.0 as *mut c_void, Ordering::Release);
            true
        }
        Err(_) => false,
    }
}

fn disconnect_pipe() {
    let ptr = PIPE_HANDLE.swap(std::ptr::null_mut(), Ordering::AcqRel);
    if !ptr.is_null() {
        let _ = unsafe { CloseHandle(HANDLE(ptr)) };
    }
}

pub fn send_event(event: &HookEvent) {
    start_pipe_thread();

    if let Some(sender) = ensure_sender() {
        let _ = sender.send(event.clone());
    }
}

pub fn disconnect() {
    let ptr = SENDER_PTR.swap(std::ptr::null_mut(), Ordering::AcqRel);
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
    disconnect_pipe();
    PIPE_THREAD_STARTED.store(false, Ordering::Relaxed);
}
