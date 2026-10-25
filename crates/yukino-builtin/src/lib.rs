#![deny(unsafe_code)]

pub mod compile;
pub mod error;
pub mod toolchain;
pub mod unpack;

pub use compile::{
    compile_macrosdk_rust, compile_macrosdk_yukino_with_runner, compile_runtime_archive,
    compile_stdlib_rust, compile_stdlib_yukino, compile_stdlib_yukino_with_runner,
};
pub use error::BuiltinError;
pub use toolchain::{cargo_path, check_rustc_available, check_target_installed, rustc_path, yukic_path};
pub use unpack::{unpack_macrosdk_source, unpack_runtime_source, unpack_stdlib_source};

include!(concat!(env!("OUT_DIR"), "/embedded_runtime_data.rs"));

use std::fs;
use std::path::{Path, PathBuf};
use yukic_target::TargetTriple;

#[derive(Debug, Clone)]
pub struct BuiltinStdlib {
    pub std_archive: PathBuf,
    pub core_archive: PathBuf,
    pub stdlib_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct BuiltinMacrosdk {
    pub sdk_archive: PathBuf,
    pub core_archive: PathBuf,
    pub macrosdk_dir: PathBuf,
}

pub fn ensure_runtime(
    work_runtime_dir: &Path,
    target: TargetTriple,
) -> Result<PathBuf, BuiltinError> {
    unpack_runtime_source(work_runtime_dir)?;

    let t_str = target.as_str();
    let target_dir = work_runtime_dir.join(t_str);
    let out_archive = target_dir.join("libyukino_rt.a");
    let stamp_file = target_dir.join(".built_hash");
    let profile_str = if IS_RELEASE_BUILD { "release" } else { "debug" };
    let expected_stamp = format!("{RUNTIME_SOURCE_HASH}:{profile_str}");

    if out_archive.is_file()
        && stamp_file.is_file()
        && let Ok(existing_hash) = fs::read_to_string(&stamp_file)
        && existing_hash.trim() == expected_stamp
    {
        return Ok(out_archive);
    }

    if target.as_str() == HOST_PREBUILT_TARGET && let Some(prebuilt) = HOST_PREBUILT_RT {
        fs::create_dir_all(&target_dir)?;
        fs::write(&out_archive, prebuilt)?;
        let _ = fs::write(&stamp_file, &expected_stamp);
        return Ok(out_archive);
    }

    compile_runtime_archive(work_runtime_dir, target, &out_archive)?;
    let _ = fs::write(&stamp_file, &expected_stamp);
    Ok(out_archive)
}

pub fn is_stdlib_built(work_stdlib_dir: &Path, target: TargetTriple) -> bool {
    let t_str = target.as_str();
    let target_dir = work_stdlib_dir.join(t_str);
    let std_archive = target_dir.join("libstd.a");
    let core_archive = target_dir.join("libyukistd_core.a");
    let stamp_file = target_dir.join(".built_hash");
    let profile_str = if IS_RELEASE_BUILD { "release" } else { "debug" };
    let expected_stamp = format!("{STDLIB_SOURCE_HASH}:{profile_str}");

    std_archive.is_file()
        && core_archive.is_file()
        && stamp_file.is_file()
        && fs::read_to_string(&stamp_file).is_ok_and(|h| h.trim() == expected_stamp)
}

pub fn ensure_stdlib(
    work_stdlib_dir: &Path,
    target: TargetTriple,
    custom_yukic: Option<&Path>,
) -> Result<BuiltinStdlib, BuiltinError> {
    ensure_stdlib_with_runner(
        work_stdlib_dir,
        target,
        custom_yukic,
        true,
        None,
        None,
        None::<fn(&mut std::process::Command) -> Result<std::process::ExitStatus, BuiltinError>>,
    )
}

pub fn ensure_stdlib_with_runner<F>(
    work_stdlib_dir: &Path,
    target: TargetTriple,
    custom_yukic: Option<&Path>,
    no_warns: bool,
    threads: Option<usize>,
    cgunits: Option<usize>,
    runner: Option<F>,
) -> Result<BuiltinStdlib, BuiltinError>
where
    F: FnMut(&mut std::process::Command) -> Result<std::process::ExitStatus, BuiltinError>,
{
    unpack_stdlib_source(work_stdlib_dir)?;

    let t_str = target.as_str();
    let target_dir = work_stdlib_dir.join(t_str);
    let std_archive = target_dir.join("libstd.a");
    let core_archive = target_dir.join("libyukistd_core.a");
    let stamp_file = target_dir.join(".built_hash");
    let profile_str = if IS_RELEASE_BUILD { "release" } else { "debug" };
    let expected_stamp = format!("{STDLIB_SOURCE_HASH}:{profile_str}");

    if std_archive.is_file()
        && core_archive.is_file()
        && stamp_file.is_file()
        && let Ok(existing_hash) = fs::read_to_string(&stamp_file)
        && existing_hash.trim() == expected_stamp
    {
        return Ok(BuiltinStdlib {
            std_archive,
            core_archive,
            stdlib_dir: work_stdlib_dir.to_path_buf(),
        });
    }

    fs::create_dir_all(&target_dir)?;

    let core_ready = if target.as_str() == HOST_PREBUILT_TARGET
        && let Some(prebuilt_core) = HOST_PREBUILT_STD_CORE
    {
        fs::write(&core_archive, prebuilt_core)?;
        true
    } else {
        false
    };

    if !core_ready {
        compile::compile_stdlib_rust(work_stdlib_dir, target, &core_archive)?;
    }

    let std_ready = if target.as_str() == HOST_PREBUILT_TARGET
        && let Some(prebuilt_std) = HOST_PREBUILT_STD
    {
        fs::write(&std_archive, prebuilt_std)?;
        true
    } else {
        false
    };

    if !std_ready {
        let yukic = custom_yukic
            .map(Path::to_path_buf)
            .unwrap_or_else(toolchain::yukic_path);

        let resolved_threads = threads.unwrap_or_else(|| {
            std::thread::available_parallelism().map_or(4, |n| n.get())
        });
        let resolved_cgunits = cgunits.unwrap_or(resolved_threads);

        compile::compile_stdlib_yukino_with_runner(
            work_stdlib_dir,
            target,
            &yukic,
            &std_archive,
            &core_archive,
            no_warns,
            resolved_threads,
            resolved_cgunits,
            runner,
        )?;
    }

    let _ = fs::write(&stamp_file, &expected_stamp);

    Ok(BuiltinStdlib {
        std_archive,
        core_archive,
        stdlib_dir: work_stdlib_dir.to_path_buf(),
    })
}

pub fn is_macrosdk_built(work_macrosdk_dir: &Path, target: TargetTriple) -> bool {
    let t_str = target.as_str();
    let target_dir = work_macrosdk_dir.join(t_str);
    let sdk_archive = target_dir.join("libmacrosdk.a");
    let core_archive = target_dir.join("libyukimacrosdk_core.a");
    let stamp_file = target_dir.join(".built_hash");
    let profile_str = if IS_RELEASE_BUILD { "release" } else { "debug" };
    let expected_stamp = format!("{MACROSDK_SOURCE_HASH}:{profile_str}");

    sdk_archive.is_file()
        && core_archive.is_file()
        && stamp_file.is_file()
        && fs::read_to_string(&stamp_file).is_ok_and(|h| h.trim() == expected_stamp)
}

pub fn ensure_macrosdk(
    work_macrosdk_dir: &Path,
    work_stdlib_dir: &Path,
    target: TargetTriple,
    custom_yukic: Option<&Path>,
) -> Result<BuiltinMacrosdk, BuiltinError> {
    ensure_macrosdk_with_runner(
        work_macrosdk_dir,
        work_stdlib_dir,
        target,
        custom_yukic,
        true,
        None,
        None,
        None::<fn(&mut std::process::Command) -> Result<std::process::ExitStatus, BuiltinError>>,
    )
}

pub fn ensure_macrosdk_with_runner<F>(
    work_macrosdk_dir: &Path,
    work_stdlib_dir: &Path,
    target: TargetTriple,
    custom_yukic: Option<&Path>,
    no_warns: bool,
    threads: Option<usize>,
    cgunits: Option<usize>,
    runner: Option<F>,
) -> Result<BuiltinMacrosdk, BuiltinError>
where
    F: FnMut(&mut std::process::Command) -> Result<std::process::ExitStatus, BuiltinError>,
{
    unpack_macrosdk_source(work_macrosdk_dir)?;

    let t_str = target.as_str();
    let target_dir = work_macrosdk_dir.join(t_str);
    let sdk_archive = target_dir.join("libmacrosdk.a");
    let core_archive = target_dir.join("libyukimacrosdk_core.a");
    let stamp_file = target_dir.join(".built_hash");
    let profile_str = if IS_RELEASE_BUILD { "release" } else { "debug" };
    let expected_stamp = format!("{MACROSDK_SOURCE_HASH}:{profile_str}");

    if sdk_archive.is_file()
        && core_archive.is_file()
        && stamp_file.is_file()
        && let Ok(existing_hash) = fs::read_to_string(&stamp_file)
        && existing_hash.trim() == expected_stamp
    {
        return Ok(BuiltinMacrosdk {
            sdk_archive,
            core_archive,
            macrosdk_dir: work_macrosdk_dir.to_path_buf(),
        });
    }

    fs::create_dir_all(&target_dir)?;

    let core_ready = if target.as_str() == HOST_PREBUILT_TARGET
        && let Some(prebuilt_core) = HOST_PREBUILT_MACROSDK_CORE
    {
        fs::write(&core_archive, prebuilt_core)?;
        true
    } else {
        false
    };

    if !core_ready {
        compile::compile_macrosdk_rust(work_macrosdk_dir, target, &core_archive)?;
    }

    let stdlib = ensure_stdlib_with_runner(
        work_stdlib_dir,
        target,
        custom_yukic,
        no_warns,
        threads,
        cgunits,
        None::<fn(&mut std::process::Command) -> Result<std::process::ExitStatus, BuiltinError>>,
    )?;

    let yukic = custom_yukic.map_or_else(yukic_path, Path::to_path_buf);
    compile::compile_macrosdk_yukino_with_runner(
        work_macrosdk_dir,
        target,
        &yukic,
        &sdk_archive,
        &core_archive,
        &stdlib.stdlib_dir,
        &stdlib.std_archive,
        &stdlib.core_archive,
        no_warns,
        threads.unwrap_or(4),
        cgunits.unwrap_or(4),
        runner,
    )?;

    fs::write(&stamp_file, &expected_stamp)?;
    Ok(BuiltinMacrosdk {
        sdk_archive,
        core_archive,
        macrosdk_dir: work_macrosdk_dir.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_runtime_files_not_empty() {
        assert!(!RUNTIME_FILES.is_empty());
        assert!(!RUNTIME_SOURCE_HASH.is_empty());

        let has_cargo = RUNTIME_FILES.iter().any(|(p, _)| *p == "Cargo.toml");
        let has_lib = RUNTIME_FILES.iter().any(|(p, _)| *p == "src/lib.rs");
        assert!(has_cargo);
        assert!(has_lib);
    }

    #[test]
    fn test_embedded_stdlib_files_not_empty() {
        assert!(!STDLIB_FILES.is_empty());
        assert!(!STDLIB_SOURCE_HASH.is_empty());

        let has_cargo = STDLIB_FILES.iter().any(|(p, _)| *p == "Cargo.toml");
        let has_lib = STDLIB_FILES.iter().any(|(p, _)| *p == "core/lib.rs");
        let has_io = STDLIB_FILES.iter().any(|(p, _)| *p == "std/io.ykn");
        assert!(has_cargo);
        assert!(has_lib);
        assert!(has_io);
    }

    #[test]
    fn test_unpack_runtime_source() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_yukino_builtin_unpack_rt_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);

        let unpacked = unpack_runtime_source(&temp_dir).expect("unpack should succeed");
        assert!(unpacked.join("Cargo.toml").is_file());
        assert!(unpacked.join("src/lib.rs").is_file());
        assert!(unpacked.join(".source_hash").is_file());

        let hash_content = fs::read_to_string(unpacked.join(".source_hash")).expect("read hash");
        assert_eq!(hash_content.trim(), RUNTIME_SOURCE_HASH);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_unpack_stdlib_source() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_yukino_builtin_unpack_std_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);

        let unpacked = unpack_stdlib_source(&temp_dir).expect("unpack should succeed");
        assert!(unpacked.join("Cargo.toml").is_file());
        assert!(unpacked.join("core/lib.rs").is_file());
        assert!(unpacked.join("lib.toml").is_file());
        assert!(unpacked.join("std/io.ykn").is_file());
        assert!(unpacked.join(".source_hash").is_file());

        let hash_content = fs::read_to_string(unpacked.join(".source_hash")).expect("read hash");
        assert_eq!(hash_content.trim(), STDLIB_SOURCE_HASH);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_check_rustc_available() {
        let ver = check_rustc_available().expect("rustc should be available");
        assert!(ver.contains("rustc"));
    }

    #[test]
    fn test_check_target_installed_host() {
        let host = TargetTriple::host();
        let libdir = check_target_installed(host).expect("host target should be installed");
        assert!(libdir.is_dir());
    }

    #[test]
    fn test_ensure_runtime_host() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_yukino_builtin_ensure_rt_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);

        let host = TargetTriple::host();
        let archive =
            ensure_runtime(&temp_dir, host).expect("ensure runtime should compile and succeed");
        assert!(archive.is_file());

        let archive2 = ensure_runtime(&temp_dir, host).expect("ensure runtime should use cache");
        assert_eq!(archive, archive2);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_compile_stdlib_rust_host() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_yukino_builtin_std_rust_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);

        let host = TargetTriple::host();
        unpack_stdlib_source(&temp_dir).expect("unpack stdlib");
        let core_archive = temp_dir.join("libyukistd_core.a");
        compile_stdlib_rust(&temp_dir, host, &core_archive).expect("compile rust core");
        assert!(core_archive.is_file());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_host_prebuilt_archives_embedded() {
        assert_eq!(HOST_PREBUILT_TARGET, TargetTriple::host().as_str());

        let rt_bytes = HOST_PREBUILT_RT.expect("host prebuilt runtime should be embedded");
        assert!(rt_bytes.len() > 1000, "rt archive should not be empty");
        assert_eq!(&rt_bytes[..8], b"!<arch>\n", "rt archive should have ar header");

        let core_bytes = HOST_PREBUILT_STD_CORE.expect("host prebuilt std core should be embedded");
        assert!(core_bytes.len() > 1000, "core archive should not be empty");
        assert_eq!(&core_bytes[..8], b"!<arch>\n", "core archive should have ar header");
    }

    #[test]
    fn test_ensure_runtime_fast_path() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_yukino_builtin_fast_rt_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);

        let host = TargetTriple::host();
        let archive = ensure_runtime(&temp_dir, host).expect("ensure runtime fast path");
        assert!(archive.is_file());
        assert!(temp_dir.join("src/lib.rs").is_file());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ensure_stdlib_uses_prebuilt_core() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_yukino_builtin_fast_core_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);

        let host = TargetTriple::host();
        let t_str = host.as_str();
        let target_dir = temp_dir.join(t_str);
        let core_archive = target_dir.join("libyukistd_core.a");

        let _ = ensure_stdlib(&temp_dir, host, Some(Path::new("non_existent_yukic")));
        assert!(core_archive.is_file(), "core archive must be written from prebuilt bytes");
        let core_bytes = fs::read(&core_archive).expect("read core archive");
        assert_eq!(&core_bytes[..8], b"!<arch>\n");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_embedded_macrosdk_files_not_empty() {
        assert!(!MACROSDK_FILES.is_empty());
        assert!(!MACROSDK_SOURCE_HASH.is_empty());

        let has_cargo = MACROSDK_FILES.iter().any(|(p, _)| *p == "Cargo.toml");
        let has_lib_rs = MACROSDK_FILES.iter().any(|(p, _)| *p == "core/lib.rs");
        let has_lib_ykn = MACROSDK_FILES.iter().any(|(p, _)| *p == "macrosdk/lib.ykn");
        assert!(has_cargo);
        assert!(has_lib_rs);
        assert!(has_lib_ykn);
    }

    #[test]
    fn test_unpack_macrosdk_source() {
        let temp_dir =
            std::env::temp_dir().join(format!("test_yukino_builtin_unpack_macrosdk_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);

        let unpacked = unpack_macrosdk_source(&temp_dir).expect("unpack macrosdk should succeed");
        assert!(unpacked.join("Cargo.toml").is_file());
        assert!(unpacked.join("core/lib.rs").is_file());
        assert!(unpacked.join("macrosdk/lib.ykn").is_file());
        assert!(unpacked.join(".source_hash").is_file());

        let hash_content = fs::read_to_string(unpacked.join(".source_hash")).expect("read hash");
        assert_eq!(hash_content.trim(), MACROSDK_SOURCE_HASH);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
