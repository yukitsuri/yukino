#![deny(unsafe_code)]

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use yukic_target::TargetTriple;
use yukic_target::spec::{Architecture, Environment, ObjectFormat, OperatingSystem};

use crate::DriverConfig;
use crate::error::DriverError;
use crate::linker_common::{
    collect_system_search_dirs, find_file_in_dirs, resolve_dynamic_linker, tool_command,
};

fn append_crt_start_files(
    args: &mut Vec<String>,
    dirs: &[PathBuf],
    target: TargetTriple,
) {
    let spec = target.spec();
    if spec.os == OperatingSystem::Linux {
        let scrt1 = find_file_in_dirs("Scrt1.o", dirs)
            .or_else(|| find_file_in_dirs("crt1.o", dirs))
            .map_or_else(|| "-l:Scrt1.o".to_string(), |p| p.to_string_lossy().to_string());
        args.push(scrt1);

        let crti = find_file_in_dirs("crti.o", dirs)
            .map_or_else(|| "-l:crti.o".to_string(), |p| p.to_string_lossy().to_string());
        args.push(crti);

        if let Some(crtbegin) = find_file_in_dirs("crtbeginS.o", dirs)
            .or_else(|| find_file_in_dirs("crtbegin.o", dirs))
        {
            args.push(crtbegin.to_string_lossy().to_string());
        }
    } else if spec.os == OperatingSystem::Windows {
        let crt2 = find_file_in_dirs("crt2.o", dirs)
            .map_or_else(|| "-l:crt2.o".to_string(), |p| p.to_string_lossy().to_string());
        args.push(crt2);
        if let Some(crtbegin) = find_file_in_dirs("crtbegin.o", dirs) {
            args.push(crtbegin.to_string_lossy().to_string());
        }
    }
}

fn append_crt_end_files(
    args: &mut Vec<String>,
    dirs: &[PathBuf],
    target: TargetTriple,
) {
    let spec = target.spec();
    if spec.os == OperatingSystem::Linux {
        if let Some(crtend) = find_file_in_dirs("crtendS.o", dirs)
            .or_else(|| find_file_in_dirs("crtend.o", dirs))
        {
            args.push(crtend.to_string_lossy().to_string());
        }
        let crtn = find_file_in_dirs("crtn.o", dirs)
            .map_or_else(|| "-l:crtn.o".to_string(), |p| p.to_string_lossy().to_string());
        args.push(crtn);
    } else if spec.os == OperatingSystem::Windows
        && let Some(crtend) = find_file_in_dirs("crtend.o", dirs)
    {
        args.push(crtend.to_string_lossy().to_string());
    }
}

fn append_target_flags(args: &mut Vec<String>, target: TargetTriple, config: &DriverConfig) {
    let spec = target.spec();
    if let Some(emu) = spec.linker_emulation {
        args.push("-m".to_string());
        args.push(emu.to_string());
    }
    if spec.env == Environment::Musl {
        args.push("-static".to_string());
    }
    if spec.os != OperatingSystem::Windows {
        args.push("-pie".to_string());
    }
    args.push("--gc-sections".to_string());
    args.push("--allow-multiple-definition".to_string());
    if spec.object_format == ObjectFormat::Elf {
        args.push("--eh-frame-hdr".to_string());
        args.push("-z".to_string());
        args.push("relro".to_string());
        args.push("-z".to_string());
        args.push("now".to_string());
    }
    if spec.os == OperatingSystem::Windows {
        args.push("--subsystem".to_string());
        args.push("console".to_string());
    }
    if spec.os == OperatingSystem::Android && spec.arch == Architecture::Aarch64 {
        args.push("--fix-cortex-a53-843419".to_string());
    }
    if let Some(dyn_linker) = resolve_dynamic_linker(target) {
        args.push("-dynamic-linker".to_string());
        args.push(dyn_linker.to_string());
    }
    if config.strip {
        args.push("-s".to_string());
    }
}

fn append_libraries(args: &mut Vec<String>, target: TargetTriple, config: &DriverConfig) {
    let spec = target.spec();
    args.push("--start-group".to_string());

    for arch in &config.link_archives {
        args.push(arch.to_string_lossy().to_string());
    }
    for stat in &config.link_statics {
        args.push(stat.to_string_lossy().to_string());
    }

    if spec.os == OperatingSystem::Windows {
        for mingw_lib in &["mingw32", "gcc", "gcc_eh", "mingwex"] {
            if !config.link_systems.iter().any(|s| s == mingw_lib) {
                args.push(format!("-l{mingw_lib}"));
            }
        }
    }

    for sys_lib in spec.system_libraries {
        args.push(format!("-l{sys_lib}"));
    }

    if spec.os == OperatingSystem::Windows {
        for win_lib in &["ntdll", "userenv"] {
            if !config.link_systems.iter().any(|s| s == win_lib) {
                args.push(format!("-l{win_lib}"));
            }
        }
    }

    let is_linux_gnu = spec.os == OperatingSystem::Linux && spec.env == Environment::Gnu;
    let has_gcc_s = config.link_systems.iter().any(|s| s == "gcc_s");
    if is_linux_gnu && !has_gcc_s {
        args.push("-lgcc_s".to_string());
    }
    for custom_lib in &config.link_systems {
        args.push(format!("-l{custom_lib}"));
    }

    args.push("--end-group".to_string());
}

fn build_msvc_executable_linker_args(
    output_path: &Path,
    objects: &[PathBuf],
    target: TargetTriple,
    config: &DriverConfig,
) -> Vec<String> {
    let spec = target.spec();
    let mut args = Vec::new();

    args.push(format!("/OUT:{}", output_path.to_string_lossy()));
    args.push("/SUBSYSTEM:CONSOLE".to_string());

    for obj in objects {
        args.push(obj.to_string_lossy().to_string());
    }

    let sys_dirs = collect_system_search_dirs(target);
    for dir in config.include_dirs.iter().chain(sys_dirs.iter()) {
        if dir.exists() {
            args.push(format!("/LIBPATH:{}", dir.display()));
        }
    }

    for archive in &config.link_archives {
        args.push(archive.to_string_lossy().to_string());
    }
    for stat in &config.link_statics {
        args.push(stat.to_string_lossy().to_string());
    }

    // libcmt provides the mainCRTStartup entry point.
    args.push("libcmt.lib".to_string());

    for sys_lib in spec.system_libraries {
        if !config.link_systems.iter().any(|s| s == sys_lib) {
            args.push(format!("{sys_lib}.lib"));
        }
    }
    for custom_lib in &config.link_systems {
        args.push(format!("{custom_lib}.lib"));
    }

    args
}

pub fn build_executable_linker_args(
    output_path: &Path,
    objects: &[PathBuf],
    target: TargetTriple,
    config: &DriverConfig,
) -> Vec<String> {
    if target.spec().env == Environment::Msvc {
        return build_msvc_executable_linker_args(output_path, objects, target, config);
    }
    let mut args = Vec::new();
    append_target_flags(&mut args, target, config);

    args.push("-o".to_string());
    args.push(output_path.to_string_lossy().to_string());

    let sys_dirs = collect_system_search_dirs(target);
    append_crt_start_files(&mut args, &sys_dirs, target);

    for obj in objects {
        args.push(obj.to_string_lossy().to_string());
    }

    for dir in &config.include_dirs {
        args.push(format!("-L{}", dir.display()));
    }
    for sys_dir in &sys_dirs {
        if sys_dir.exists() {
            args.push(format!("-L{}", sys_dir.display()));
        }
    }

    append_libraries(&mut args, target, config);
    append_crt_end_files(&mut args, &sys_dirs, target);
    args
}

pub fn validate_input_binaries(
    objects: &[PathBuf],
    target: TargetTriple,
    config: &DriverConfig,
) -> Result<(), DriverError> {
    for obj in objects {
        yukic_target::validate_binary_compatibility(obj, target)?;
    }
    for arch in &config.link_archives {
        if arch.is_file() {
            yukic_target::validate_binary_compatibility(arch, target)?;
        }
    }
    for stat in &config.link_statics {
        if stat.is_file() {
            yukic_target::validate_binary_compatibility(stat, target)?;
        }
    }
    Ok(())
}

pub fn link_executable(
    output_path: &Path,
    objects: &[PathBuf],
    target: TargetTriple,
    config: &DriverConfig,
) -> Result<(), DriverError> {
    if objects.is_empty() {
        return Err(DriverError::NoInputFile);
    }
    validate_input_binaries(objects, target, config)?;

    if let Some(parent) = output_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    let tool = match config.linker.as_deref() {
        Some("lld") | None => target.spec().default_linker,
        Some(custom) => custom,
    };
    let args = build_executable_linker_args(output_path, objects, target, config);

    if config.verbose {
        println!("Linker: {tool} {}", args.join(" "));
    }

    let mut cmd = tool_command(tool);
    cmd.args(&args);

    let output = match cmd.output() {
        Ok(out) => out,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Err(DriverError::ToolNotFound(tool.to_string()));
        }
        Err(err) => return Err(DriverError::Io(err)),
    };

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(DriverError::LinkerFailed(tool.to_string(), code, stderr));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_executable_args_linux_glibc() {
        let mut config = DriverConfig::new("src/main.ykn".into());
        config.strip = true;
        config.link_systems = vec!["gcc_s".into()];
        let objects = vec![PathBuf::from("cgu_0.o"), PathBuf::from("cgu_1.o")];
        let out = PathBuf::from("bin/app");
        let target = TargetTriple::X86_64UnknownLinuxGnu;

        let args = build_executable_linker_args(&out, &objects, target, &config);

        assert!(args.contains(&"-pie".to_string()));
        assert!(args.contains(&"--gc-sections".to_string()));
        assert!(args.contains(&"-s".to_string()));
        assert!(args.contains(&"-dynamic-linker".to_string()));
        assert!(args.contains(&"/lib64/ld-linux-x86-64.so.2".to_string()));
        assert!(args.contains(&"cgu_0.o".to_string()));
        assert!(args.contains(&"cgu_1.o".to_string()));
        assert!(args.contains(&"-lc".to_string()));
        assert!(args.contains(&"-lpthread".to_string()));
        assert!(args.contains(&"-lgcc_s".to_string()));
    }

    #[test]
    fn test_build_executable_args_windows_mingw() {
        let config = DriverConfig::new("src/main.ykn".into());
        let objects = vec![PathBuf::from("cgu_0.o")];
        let out = PathBuf::from("bin/app.exe");
        let target = TargetTriple::X86_64PcWindowsGnu;

        let args = build_executable_linker_args(&out, &objects, target, &config);

        assert!(args.contains(&"-m".to_string()));
        assert!(args.contains(&"i386pep".to_string()));
        assert!(args.contains(&"--subsystem".to_string()));
        assert!(args.contains(&"console".to_string()));
        assert!(args.contains(&"-lkernel32".to_string()));
        assert!(args.contains(&"-lmsvcrt".to_string()));
        assert!(!args.contains(&"-pie".to_string()));
    }

    #[test]
    fn test_build_executable_args_android_aarch64() {
        let config = DriverConfig::new("src/main.ykn".into());
        let objects = vec![PathBuf::from("cgu_0.o")];
        let out = PathBuf::from("bin/app");
        let target = TargetTriple::Aarch64LinuxAndroid;

        let args = build_executable_linker_args(&out, &objects, target, &config);

        assert!(args.contains(&"-m".to_string()));
        assert!(args.contains(&"aarch64linux".to_string()));
        assert!(args.contains(&"--fix-cortex-a53-843419".to_string()));
        assert!(args.contains(&"-llog".to_string()));
    }

    #[test]
    fn test_build_executable_args_linux_musl() {
        let config = DriverConfig::new("src/main.ykn".into());
        let objects = vec![PathBuf::from("cgu_0.o")];
        let out = PathBuf::from("bin/app");
        let target = TargetTriple::X86_64UnknownLinuxMusl;

        let args = build_executable_linker_args(&out, &objects, target, &config);

        assert!(args.contains(&"-static".to_string()));
        assert!(args.contains(&"-pie".to_string()));
        assert!(args.contains(&"-lc".to_string()));
        assert!(args.contains(&"-lm".to_string()));
        assert!(!args.contains(&"-lpthread".to_string()));
    }
}
