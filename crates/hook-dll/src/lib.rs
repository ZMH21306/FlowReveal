#[cfg(windows)]
mod winhttp;

#[cfg(windows)]
use std::ffi::c_void;

#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_hinst: *mut c_void, reason: u32, _reserved: *mut c_void) -> i32 {
    match reason {
        1 => {
            std::thread::spawn(|| {
                tracing::info!("FlowReveal hook DLL attached");
            });
        }
        _ => {}
    }
    1
}
