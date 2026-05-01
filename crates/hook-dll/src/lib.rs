#[cfg(windows)]
mod pipe_client;
#[cfg(windows)]
mod winhttp;

#[cfg(windows)]
use std::ffi::c_void;

#[cfg(windows)]
static mut G_HOOKS_INSTALLED: bool = false;

#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_hinst: *mut c_void, reason: u32, _reserved: *mut c_void) -> i32 {
    match reason {
        1 => {
            std::thread::spawn(|| {
                unsafe {
                    if G_HOOKS_INSTALLED {
                        return;
                    }
                    G_HOOKS_INSTALLED = true;
                }

                match winhttp::install() {
                    Ok(()) => {
                        tracing::info!("FlowReveal hook DLL: WinHTTP hooks installed");
                    }
                    Err(e) => {
                        tracing::error!("FlowReveal hook DLL: Failed to install hooks: {}", e);
                    }
                }
            });
        }
        0 => {
            unsafe {
                if G_HOOKS_INSTALLED {
                    winhttp::uninstall();
                    pipe_client::disconnect();
                    G_HOOKS_INSTALLED = false;
                }
            }
        }
        _ => {}
    }
    1
}
