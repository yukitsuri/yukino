use crate::error::BuiltinError;
use std::path::PathBuf;
use std::process::Command;
use yukic_target::TargetTriple;

pub fn rustc_path() -> String {
    std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string())
}

pub fn cargo_path() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

pub fn check_rustc_available() -> Result<String, BuiltinError> {
    let rustc = rustc_path();
    let output = Command::new(&rustc)
        .arg("--version")
        .output()
        .map_err(|_| BuiltinError::RustcNotFound)?;
    if !output.status.success() {
        return Err(BuiltinError::RustcNotFound);
    }
    let ver_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(ver_str)
}

pub fn check_target_installed(target: TargetTriple) -> Result<PathBuf, BuiltinError> {
    check_rustc_available()?;
    let rustc = rustc_path();
    let t_str = target.as_str();

    let output = Command::new(&rustc)
        .arg("--target")
        .arg(t_str)
        .arg("--print")
        .arg("target-libdir")
        .output()
        .map_err(|_| BuiltinError::RustcNotFound)?;

    if !output.status.success() {
        return Err(BuiltinError::TargetNotInstalled {
            target: t_str.to_string(),
            install_cmd: format!("rustup target add {t_str}"),
        });
    }

    let libdir_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let libdir = PathBuf::from(&libdir_str);
    if !libdir.is_dir() {
        return Err(BuiltinError::TargetNotInstalled {
            target: t_str.to_string(),
            install_cmd: format!("rustup target add {t_str}"),
        });
    }
    Ok(libdir)
}

pub fn yukic_path() -> PathBuf {
    if let Ok(p) = std::env::var("YUKIC") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return pb;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(stem) = exe.file_stem().and_then(|s| s.to_str()) {
            if stem.starts_with("yukic") {
                return exe;
            }
        }
        if let Some(dir) = exe.parent() {
            for d in [Some(dir), dir.parent()].into_iter().flatten() {
                for name in ["yukic-dbg", "yukic"] {
                    let candidate = d.join(name);
                    if candidate.is_file() {
                        return candidate;
                    }
                    #[cfg(target_os = "windows")]
                    {
                        let win_candidate = d.join(format!("{name}.exe"));
                        if win_candidate.is_file() {
                            return win_candidate;
                        }
                    }
                }
            }
        }
    }
    PathBuf::from("yukic")
}
