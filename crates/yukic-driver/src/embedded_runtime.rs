#![deny(unsafe_code)]

use std::path::{Path, PathBuf};
use yukic_target::TargetTriple;
use yukino_builtin::{ensure_runtime, BuiltinError};

pub fn resolve_runtime_dir() -> PathBuf {
    if Path::new("yukino-work/local").is_dir() {
        PathBuf::from("yukino-work/local/runtime")
    } else if Path::new("yuki-work/local").is_dir() {
        PathBuf::from("yuki-work/local/runtime")
    } else if Path::new("yukino-work").is_dir() {
        PathBuf::from("yukino-work/local/runtime")
    } else {
        PathBuf::from("yuki-work/local/runtime")
    }
}

pub fn resolve_runtime_dir_for(project_dir: Option<&Path>) -> PathBuf {
    if let Some(dir) = project_dir {
        if dir.join("yukino-work/local").is_dir() {
            dir.join("yukino-work/local/runtime")
        } else {
            dir.join("yuki-work/local/runtime")
        }
    } else {
        resolve_runtime_dir()
    }
}

pub fn ensure_runtime_archive_for(project_dir: Option<&Path>, target: TargetTriple) -> Result<PathBuf, BuiltinError> {
    let base_dir = resolve_runtime_dir_for(project_dir);
    ensure_runtime(&base_dir, target)
}

pub fn ensure_runtime_archive(target: TargetTriple) -> Result<PathBuf, BuiltinError> {
    ensure_runtime_archive_for(None, target)
}

pub fn unpack_embedded_runtime_at(path: &Path) -> Result<PathBuf, BuiltinError> {
    let host = TargetTriple::host();
    if let Some(parent) = path.parent() {
        ensure_runtime(parent, host)
    } else {
        let base_dir = resolve_runtime_dir();
        ensure_runtime(&base_dir, host)
    }
}

pub fn unpack_embedded_runtime(target_str: &str) -> Result<PathBuf, BuiltinError> {
    let target = TargetTriple::parse(target_str).unwrap_or_else(|_| TargetTriple::host());
    ensure_runtime_archive(target)
}

pub fn resolve_stdlib_dir_for(project_dir: Option<&Path>) -> PathBuf {
    if let Some(dir) = project_dir {
        if dir.join("yukino-work/local").is_dir() {
            dir.join("yukino-work/local/stdlib")
        } else {
            dir.join("yuki-work/local/stdlib")
        }
    } else if Path::new("yukino-work/local").is_dir() {
        PathBuf::from("yukino-work/local/stdlib")
    } else {
        PathBuf::from("yuki-work/local/stdlib")
    }
}

pub fn ensure_stdlib_source_for(
    project_dir: Option<&Path>,
) -> Result<PathBuf, BuiltinError> {
    let base_dir = resolve_stdlib_dir_for(project_dir);
    yukino_builtin::unpack_stdlib_source(&base_dir)
}

pub fn ensure_stdlib_for(
    project_dir: Option<&Path>,
    target: TargetTriple,
) -> Result<yukino_builtin::BuiltinStdlib, BuiltinError> {
    let base_dir = resolve_stdlib_dir_for(project_dir);
    yukino_builtin::ensure_stdlib(&base_dir, target, None)
}

pub fn resolve_macrosdk_dir_for(project_dir: Option<&Path>) -> PathBuf {
    if let Some(dir) = project_dir {
        if dir.join("yukino-work/local").is_dir() {
            dir.join("yukino-work/local/macrosdk")
        } else {
            dir.join("yuki-work/local/macrosdk")
        }
    } else if Path::new("yukino-work/local").is_dir() {
        PathBuf::from("yukino-work/local/macrosdk")
    } else {
        PathBuf::from("yuki-work/local/macrosdk")
    }
}

pub fn ensure_macrosdk_for(
    project_dir: Option<&Path>,
    target: TargetTriple,
) -> Result<yukino_builtin::BuiltinMacrosdk, BuiltinError> {
    let base_dir = resolve_macrosdk_dir_for(project_dir);
    let std_dir = resolve_stdlib_dir_for(project_dir);
    yukino_builtin::ensure_macrosdk(&base_dir, &std_dir, target, None)
}

/*
use std::fs;
use std::io;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

pub const EMBEDDED_TARGET_TRIPLE: &str = env!("YUKINO_RT_TARGET");
pub const EMBEDDED_YUKINO_RT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/libyukino_rt.a"));

pub fn unpack_embedded_runtime_at(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_path = path.with_extension(format!("a.tmp.{}.{}", std::process::id(), counter));
    fs::write(&tmp_path, EMBEDDED_YUKINO_RT)?;
    fs::rename(tmp_path, path)?;
    Ok(())
}

pub fn ensure_runtime_archive(target: TargetTriple) -> Option<PathBuf> {
    let t_str = target.as_str();

    let local_paths = [
        format!("yuki-work/local/runtime/{t_str}/libyukino_rt.a"),
        format!("yukino/local/runtime/{t_str}/libyukino_rt.a"),
        format!("target/{t_str}/release/libyukino_rt.a"),
        format!("target/{t_str}/debug/libyukino_rt.a"),
    ];

    for path_str in &local_paths {
        let p = PathBuf::from(path_str);
        if p.is_file() {
            return Some(p);
        }
    }

    if (t_str == EMBEDDED_TARGET_TRIPLE || target == TargetTriple::host())
        && let Ok(unpacked) = unpack_embedded_runtime(t_str)
    {
        return Some(unpacked);
    }

    try_compile_cross_runtime(target)
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn unpack_embedded_runtime(target_str: &str) -> io::Result<PathBuf> {
    let base_dir = resolve_runtime_dir(target_str);
    let archive_path = base_dir.join("libyukino_rt.a");
    unpack_embedded_runtime_at(&archive_path)?;
    Ok(archive_path)
}


fn resolve_runtime_dir(target_str: &str) -> PathBuf {
    if Path::new("yukino/local").is_dir() {
        PathBuf::from("yukino/local/runtime").join(target_str)
    } else {
        PathBuf::from("yuki-work/local/runtime").join(target_str)
    }
}

fn try_compile_cross_runtime(target: TargetTriple) -> Option<PathBuf> {
    let t_str = target.as_str();
    let base_dir = resolve_runtime_dir(t_str);
    let archive_path = base_dir.join("libyukino_rt.a");
    if archive_path.is_file() {
        return Some(archive_path);
    }

    let dev_rt = PathBuf::from("builtin/runtime/src/lib.rs");
    if !dev_rt.is_file() {
        return None;
    }

    let _ = fs::create_dir_all(&base_dir);
    let status = Command::new("rustc")
        .arg("--crate-name")
        .arg("yukino_rt")
        .arg("--crate-type")
        .arg("staticlib")
        .arg("--edition")
        .arg("2024")
        .arg("-C")
        .arg("opt-level=z")
        .arg("-C")
        .arg("codegen-units=1")
        .arg("-C")
        .arg("panic=abort")
        .arg("-C")
        .arg("strip=debuginfo")
        .arg("-C")
        .arg("link-dead-code=no")
        .arg("--target")
        .arg(t_str)
        .arg(&dev_rt)
        .arg("-o")
        .arg(&archive_path)
        .status()
        .ok()?;

    if status.success() && archive_path.is_file() {
        let _ = Command::new("llvm-strip")
            .arg("--strip-debug")
            .arg(&archive_path)
            .status()
            .or_else(|_| {
                if target == TargetTriple::host() {
                    Command::new("strip").arg("--strip-debug").arg(&archive_path).status()
                } else {
                    Err(std::io::Error::other("skip host strip on cross target"))
                }
            });
        Some(archive_path)
    } else {
        None
    }
}
*/
