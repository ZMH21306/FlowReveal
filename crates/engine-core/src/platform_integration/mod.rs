#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "windows")]
pub mod api_hook;

#[cfg(not(target_os = "windows"))]
pub mod stub;

#[cfg(not(target_os = "windows"))]
pub mod api_hook;
