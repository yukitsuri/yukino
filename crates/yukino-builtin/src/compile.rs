use crate::error::BuiltinError;
use crate::toolchain::{cargo_path, check_target_installed, rustc_path};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use yukic_target::TargetTriple;
use yukic_target::spec::Environment;

use crate::IS_RELEASE_BUILD;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(1);

fn strip_archive(archive: &Path, target: TargetTriple) {
    if !IS_RELEASE_BUILD {
        return;
    }
    // llvm-strip cannot process MSVC archives that embed import members.
    if target.spec().env == Environment::Msvc {
        return;
    }
    let _ = Command::new("llvm-strip")
        .arg("--strip-debug")
        .arg(archive)
        .status()
        .or_else(|_| {
            if target == TargetTriple::host() {
                Command::new("strip")
                    .arg("--strip-debug")
                    .arg(archive)
                    .status()
            } else {
                Err(std::io::Error::other("skip host strip"))
            }
        });
}

pub fn compile_runtime_archive(
    src_dir: &Path,
    target: TargetTriple,
    out_archive: &Path,
) -> Result<PathBuf, BuiltinError> {
    check_target_installed(target)?;
    let rustc = rustc_path();
    let t_str = target.as_str();
    let runtime_lib = src_dir.join("src/lib.rs");
    if !runtime_lib.is_file() {
        return Err(BuiltinError::CompilationFailed {
            target: t_str.to_string(),
            details: format!("source file not found: {}", runtime_lib.display()),
        });
    }

    if let Some(parent) = out_archive.parent() {
        fs::create_dir_all(parent)?;
    }

    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_archive = out_archive.with_extension(format!("a.tmp.{}.{}", std::process::id(), counter));

    let mut cmd = Command::new(&rustc);
    cmd.arg("--crate-name")
        .arg("yukino_rt")
        .arg("--crate-type")
        .arg("staticlib")
        .arg("--edition")
        .arg("2024");

    if IS_RELEASE_BUILD {
        cmd.arg("-C")
            .arg("opt-level=z")
            .arg("-C")
            .arg("codegen-units=1")
            .arg("-C")
            .arg("panic=abort")
            .arg("-C")
            .arg("force-unwind-tables=yes")
            .arg("-C")
            .arg("strip=debuginfo")
            .arg("-C")
            .arg("link-dead-code=no");
    } else {
        cmd.arg("-C")
            .arg("opt-level=0")
            .arg("-C")
            .arg("debuginfo=2")
            .arg("-C")
            .arg("panic=abort")
            .arg("-C")
            .arg("force-unwind-tables=yes");
    }

    cmd.arg("--target")
        .arg(t_str)
        .arg(&runtime_lib)
        .arg("-o")
        .arg(&temp_archive);

    let output = cmd
        .output()
        .map_err(|e| BuiltinError::CompilationFailed {
            target: t_str.to_string(),
            details: format!("failed to spawn rustc: {e}"),
        })?;

    if !output.status.success() {
        let _ = fs::remove_file(&temp_archive);
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(BuiltinError::CompilationFailed {
            target: t_str.to_string(),
            details: stderr,
        });
    }

    strip_archive(&temp_archive, target);
    fs::rename(&temp_archive, out_archive)?;
    Ok(out_archive.to_path_buf())
}

fn try_compile_via_cargo(
    manifest_path: &Path,
    target_str: &str,
    target_dir: &Path,
    crate_name: &str,
) -> Result<PathBuf, String> {
    let cargo = cargo_path();
    let mut cmd = Command::new(&cargo);
    cmd.arg("build")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg("--target")
        .arg(target_str)
        .arg("--target-dir")
        .arg(target_dir);

    if IS_RELEASE_BUILD {
        cmd.arg("--release");
    }

    let output = cmd
        .output()
        .map_err(|e| format!("cargo spawn failed: {e}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let profile_subdir = if IS_RELEASE_BUILD { "release" } else { "debug" };
    let release_dir = target_dir.join(target_str).join(profile_subdir);
    let archive_name = if target_str.ends_with("-msvc") {
        format!("{crate_name}.lib")
    } else {
        format!("lib{crate_name}.a")
    };
    let candidate = release_dir.join(&archive_name);
    if candidate.is_file() {
        return Ok(candidate);
    }
    Err(format!("cargo did not produce {archive_name}"))
}

fn compile_rust_via_rustc(
    lib_rs: &Path,
    target_str: &str,
    out_archive: &Path,
) -> Result<(), BuiltinError> {
    let rustc = rustc_path();
    let mut cmd = Command::new(&rustc);
    cmd.arg("--crate-name")
        .arg("yukistd_core")
        .arg("--crate-type")
        .arg("staticlib")
        .arg("--edition")
        .arg("2024");

    if IS_RELEASE_BUILD {
        cmd.arg("-C")
            .arg("opt-level=3")
            .arg("-C")
            .arg("codegen-units=1")
            .arg("-C")
            .arg("panic=abort")
            .arg("-C")
            .arg("strip=debuginfo");
    } else {
        cmd.arg("-C")
            .arg("opt-level=0")
            .arg("-C")
            .arg("debuginfo=2")
            .arg("-C")
            .arg("panic=abort");
    }

    cmd.arg("--target")
        .arg(target_str)
        .arg(lib_rs)
        .arg("-o")
        .arg(out_archive);

    let output = cmd
        .output()
        .map_err(|e| BuiltinError::CompilationFailed {
            target: target_str.to_string(),
            details: format!("failed to spawn rustc: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(BuiltinError::CompilationFailed {
            target: target_str.to_string(),
            details: stderr,
        });
    }
    Ok(())
}

pub fn compile_stdlib_rust(
    src_dir: &Path,
    target: TargetTriple,
    out_archive: &Path,
) -> Result<PathBuf, BuiltinError> {
    check_target_installed(target)?;
    let t_str = target.as_str();
    let rust_lib = src_dir.join("core/lib.rs");
    if !rust_lib.is_file() {
        return Err(BuiltinError::CompilationFailed {
            target: t_str.to_string(),
            details: format!("source file not found: {}", rust_lib.display()),
        });
    }

    if let Some(parent) = out_archive.parent() {
        fs::create_dir_all(parent)?;
    }

    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_archive = out_archive.with_extension(format!("a.tmp.{}.{}", std::process::id(), counter));
    let cargo_toml = src_dir.join("Cargo.toml");
    let cargo_target_dir = src_dir.join("target");

    let cargo_built = if cargo_toml.is_file() {
        Some(try_compile_via_cargo(&cargo_toml, t_str, &cargo_target_dir, "yukistd_core").map_err(|err| {
            BuiltinError::CompilationFailed {
                target: t_str.to_string(),
                details: err,
            }
        })?)
    } else {
        None
    };

    if let Some(built) = cargo_built {
        fs::copy(&built, &temp_archive)?;
    } else {
        compile_rust_via_rustc(&rust_lib, t_str, &temp_archive)?;
    }

    strip_archive(&temp_archive, target);
    fs::rename(&temp_archive, out_archive)?;
    Ok(out_archive.to_path_buf())
}

pub fn compile_stdlib_yukino(
    src_dir: &Path,
    target: TargetTriple,
    yukic_bin: &Path,
    out_archive: &Path,
    core_archive: &Path,
) -> Result<PathBuf, BuiltinError> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    compile_stdlib_yukino_with_runner(
        src_dir,
        target,
        yukic_bin,
        out_archive,
        core_archive,
        true,
        threads,
        threads,
        None::<fn(&mut Command) -> Result<std::process::ExitStatus, BuiltinError>>,
    )
}

pub fn compile_stdlib_yukino_with_runner<F>(
    src_dir: &Path,
    target: TargetTriple,
    yukic_bin: &Path,
    out_archive: &Path,
    core_archive: &Path,
    no_warns: bool,
    threads: usize,
    cgunits: usize,
    mut runner: Option<F>,
) -> Result<PathBuf, BuiltinError>
where
    F: FnMut(&mut Command) -> Result<std::process::ExitStatus, BuiltinError>,
{
    let t_str = target.as_str();
    let entry = src_dir.join("std/lib.ykn");
    if !entry.is_file() {
        return Err(BuiltinError::CompilationFailed {
            target: t_str.to_string(),
            details: format!("entry file not found: {}", entry.display()),
        });
    }

    if let Some(parent) = out_archive.parent() {
        fs::create_dir_all(parent)?;
    }

    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_archive = out_archive.with_extension(format!("a.tmp.{}.{}", std::process::id(), counter));
    let abs_temp = if temp_archive.is_relative() {
        std::env::current_dir().unwrap_or_default().join(&temp_archive)
    } else {
        temp_archive.clone()
    };
    let abs_core = if core_archive.is_relative() {
        std::env::current_dir().unwrap_or_default().join(core_archive)
    } else {
        core_archive.to_path_buf()
    };

    let mut cmd = Command::new(yukic_bin);
    cmd.current_dir(src_dir)
        .arg("--entry")
        .arg("std/lib.ykn")
        .arg("--output-type")
        .arg("static")
        .arg("-o")
        .arg(&abs_temp)
        .arg("--target")
        .arg(t_str)
        .arg("-O")
        .arg(if IS_RELEASE_BUILD { "3" } else { "0" })
        .arg("--no-runtime")
        .arg("--manifest")
        .arg("lib.toml")
        .arg("--function-sections")
        .arg("--threads")
        .arg(threads.to_string())
        .arg("--cgunits")
        .arg(cgunits.to_string())
        .arg("--link-archive")
        .arg(&abs_core);

    if no_warns {
        cmd.arg("--no-warns");
    }

    let status = if let Some(ref mut run) = runner {
        run(&mut cmd)?
    } else {
        let output = cmd.output().map_err(|e| BuiltinError::CompilationFailed {
            target: t_str.to_string(),
            details: format!("failed to spawn yukic: {e}"),
        })?;
        if !output.status.success() {
            let _ = fs::remove_file(&temp_archive);
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(BuiltinError::CompilationFailed {
                target: t_str.to_string(),
                details: stderr,
            });
        }
        output.status
    };

    if !status.success() {
        let _ = fs::remove_file(&temp_archive);
        return Err(BuiltinError::CompilationFailed {
            target: t_str.to_string(),
            details: "compilation of stdlib failed".to_string(),
        });
    }

    strip_archive(&temp_archive, target);
    fs::rename(&temp_archive, out_archive)?;
    Ok(out_archive.to_path_buf())
}

pub fn compile_macrosdk_rust(
    src_dir: &Path,
    target: TargetTriple,
    out_archive: &Path,
) -> Result<PathBuf, BuiltinError> {
    check_target_installed(target)?;
    let t_str = target.as_str();
    let cargo_toml = src_dir.join("Cargo.toml");
    let cargo_target_dir = src_dir.join("target/macrosdk_core");

    if let Some(parent) = out_archive.parent() {
        fs::create_dir_all(parent)?;
    }

    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_archive = out_archive.with_extension(format!("a.tmp.{}.{}", std::process::id(), counter));

    let built = try_compile_via_cargo(&cargo_toml, t_str, &cargo_target_dir, "yukimacrosdk_core")
        .map_err(|details| BuiltinError::CompilationFailed {
            target: t_str.to_string(),
            details,
        })?;

    fs::copy(&built, &temp_archive)?;
    strip_archive(&temp_archive, target);
    fs::rename(&temp_archive, out_archive)?;
    Ok(out_archive.to_path_buf())
}

pub fn compile_macrosdk_yukino_with_runner<F>(
    src_dir: &Path,
    target: TargetTriple,
    yukic_bin: &Path,
    out_archive: &Path,
    core_archive: &Path,
    stdlib_dir: &Path,
    stdlib_archive: &Path,
    stdlib_core_archive: &Path,
    no_warns: bool,
    threads: usize,
    cgunits: usize,
    mut runner: Option<F>,
) -> Result<PathBuf, BuiltinError>
where
    F: FnMut(&mut Command) -> Result<std::process::ExitStatus, BuiltinError>,
{
    let t_str = target.as_str();
    let entry = src_dir.join("macrosdk/lib.ykn");
    if !entry.is_file() {
        return Err(BuiltinError::CompilationFailed {
            target: t_str.to_string(),
            details: format!("entry file not found: {}", entry.display()),
        });
    }

    if let Some(parent) = out_archive.parent() {
        fs::create_dir_all(parent)?;
    }

    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_archive = out_archive.with_extension(format!("a.tmp.{}.{}", std::process::id(), counter));
    let abs_temp = if temp_archive.is_relative() {
        std::env::current_dir().unwrap_or_default().join(&temp_archive)
    } else {
        temp_archive.clone()
    };
    let abs_core = if core_archive.is_relative() {
        std::env::current_dir().unwrap_or_default().join(core_archive)
    } else {
        core_archive.to_path_buf()
    };

    let mut cmd = Command::new(yukic_bin);
    cmd.current_dir(src_dir)
        .arg("--entry")
        .arg("macrosdk/lib.ykn")
        .arg("--output-type")
        .arg("static")
        .arg("-o")
        .arg(&abs_temp)
        .arg("--target")
        .arg(t_str)
        .arg("-O")
        .arg(if IS_RELEASE_BUILD { "3" } else { "0" })
        .arg("-j")
        .arg(threads.to_string())
        .arg("--codegen-units")
        .arg(cgunits.to_string())
        .arg("--link-archive")
        .arg(&abs_core)
        .arg("--include")
        .arg(stdlib_dir)
        .arg("--connected-lib")
        .arg("std");

    if stdlib_archive.is_file() {
        cmd.arg("--link-archive").arg(stdlib_archive);
    }
    if stdlib_core_archive.is_file() {
        cmd.arg("--link-archive").arg(stdlib_core_archive);
    }

    if no_warns {
        cmd.arg("--no-warns");
    }

    let status = if let Some(ref mut run) = runner {
        run(&mut cmd)?
    } else {
        let output = cmd.output().map_err(|e| BuiltinError::CompilationFailed {
            target: t_str.to_string(),
            details: format!("failed to spawn yukic: {e}"),
        })?;
        if !output.status.success() {
            let _ = fs::remove_file(&temp_archive);
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(BuiltinError::CompilationFailed {
                target: t_str.to_string(),
                details: stderr,
            });
        }
        output.status
    };

    if !status.success() {
        let _ = fs::remove_file(&temp_archive);
        return Err(BuiltinError::CompilationFailed {
            target: t_str.to_string(),
            details: "compilation of macrosdk failed".to_string(),
        });
    }

    strip_archive(&temp_archive, target);
    fs::rename(&temp_archive, out_archive)?;
    Ok(out_archive.to_path_buf())
}
