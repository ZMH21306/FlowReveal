fn main() {
    tauri_build::build();

    #[cfg(feature = "windivert")]
    {
        copy_windivert_files();
    }
}

#[cfg(feature = "windivert")]
fn copy_windivert_files() {
    use std::fs;
    use std::path::PathBuf;

    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let (sys_file, dll_file) = match arch.as_str() {
        "x86_64" => ("WinDivert64.sys", "WinDivert.dll"),
        "x86" => ("WinDivert32.sys", "WinDivert.dll"),
        _ => return,
    };

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let profile = std::env::var("PROFILE").unwrap();
    let out_dir = PathBuf::from(&manifest_dir).join("../../target").join(&profile);

    for search_root in &[out_dir.clone(), out_dir.join("build")] {
        if let Ok(entries) = fs::read_dir(search_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let sys_path = path.join(sys_file);
                    let dll_path = path.join(dll_file);

                    if sys_path.exists() && !out_dir.join(sys_file).exists() {
                        let _ = fs::copy(&sys_path, out_dir.join(sys_file));
                        println!("cargo:warning=Copied {} to output directory", sys_file);
                    }
                    if dll_path.exists() && !out_dir.join(dll_file).exists() {
                        let _ = fs::copy(&dll_path, out_dir.join(dll_file));
                        println!("cargo:warning=Copied {} to output directory", dll_file);
                    }

                    if out_dir.join(sys_file).exists() && out_dir.join(dll_file).exists() {
                        return;
                    }
                }
            }
        }
    }

    let driver_dir = PathBuf::from(&manifest_dir).join("windivert-driver");
    if driver_dir.exists() {
        let src = driver_dir.join(sys_file);
        if src.exists() {
            let _ = fs::copy(&src, out_dir.join(sys_file));
            println!("cargo:warning=Copied {} from local driver directory", sys_file);
        }
    }
}
