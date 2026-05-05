pub fn is_elevated() -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
        use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

        unsafe {
            let mut token_handle: HANDLE = HANDLE::default();
            let result = OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_QUERY,
                &mut token_handle,
            );

            if result.is_err() {
                tracing::debug!("[Elevation] OpenProcessToken failed");
                return false;
            }

            let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut returned_len = 0u32;

            let result = GetTokenInformation(
                token_handle,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut _),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut returned_len,
            );

            let _ = windows::Win32::Foundation::CloseHandle(token_handle);

            if result.is_err() {
                tracing::debug!("[Elevation] GetTokenInformation failed");
                return false;
            }

            elevation.TokenIsElevated != 0
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

pub fn request_elevation() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::core::HSTRING;
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Cannot get exe path: {}", e))?;

        let exe_str = exe_path.to_string_lossy().to_string();
        let verb = HSTRING::from("runas");
        let file = HSTRING::from(&exe_str);

        let result = unsafe {
            ShellExecuteW(
                None,
                &verb,
                &file,
                None,
                None,
                SW_SHOW,
            )
        };

        tracing::info!("[Elevation] UAC elevation requested");

        if result.0 as isize <= 32 {
            tracing::warn!("[Elevation] UAC was cancelled or failed");
            return Err("UAC 提权被取消或失败".to_string());
        }

        std::process::exit(0);
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Elevation not supported on this platform".to_string())
    }
}

pub fn ensure_elevated() -> Result<(), String> {
    if is_elevated() {
        Ok(())
    } else {
        Err("Administrator privileges required for global capture".to_string())
    }
}
