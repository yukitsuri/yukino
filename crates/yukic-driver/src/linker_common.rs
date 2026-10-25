#![deny(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use yukic_target::TargetTriple;
use yukic_target::spec::{Architecture, Environment, OperatingSystem};

/// On Windows, bare tool names containing a dot (like `lld-link`) need an
/// explicit `.exe` suffix to be found in `PATH`.
pub fn tool_command(tool: &str) -> Command {
    if cfg!(target_os = "windows")
        && !tool.contains(['/', '\\'])
        && !tool.to_ascii_lowercase().ends_with(".exe")
    {
        Command::new(format!("{tool}.exe"))
    } else {
        Command::new(tool)
    }
}

pub fn resolve_dynamic_linker(target: TargetTriple) -> Option<&'static str> {
    let spec = target.spec();
    match (spec.os, spec.env, spec.arch) {
        (OperatingSystem::Linux, Environment::Gnu, Architecture::X86_64) => {
            Some("/lib64/ld-linux-x86-64.so.2")
        }
        (OperatingSystem::Linux, Environment::Gnu, Architecture::Aarch64) => {
            Some("/lib/ld-linux-aarch64.so.1")
        }
        (OperatingSystem::Android, _, Architecture::Aarch64) => Some("/system/bin/linker64"),
        _ => None,
    }
}

pub fn collect_system_search_dirs(target: TargetTriple) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let spec = target.spec();

    match spec.os {
        OperatingSystem::Linux => {
            let arch_keyword = match spec.arch {
                Architecture::X86_64 => "x86_64",
                Architecture::Aarch64 => "aarch64",
                Architecture::Armv7 => "arm",
            };
            collect_gcc_lib_dirs(&mut dirs, arch_keyword);
            dirs.push(PathBuf::from("/usr/lib64"));
            dirs.push(PathBuf::from("/lib64"));
            dirs.push(PathBuf::from("/usr/lib"));
            dirs.push(PathBuf::from("/lib"));
            dirs.push(PathBuf::from("/usr/lib/x86_64-linux-gnu"));
            dirs.push(PathBuf::from("/usr/lib/aarch64-linux-gnu"));
            dirs.push(PathBuf::from("/usr/lib/musl"));
            dirs.push(PathBuf::from("/usr/lib/x86_64-linux-musl"));
        }
        OperatingSystem::Windows => {
            if spec.env == Environment::Msvc {
                collect_windows_msvc_dirs(&mut dirs);
            } else {
                collect_gcc_lib_dirs(&mut dirs, "x86_64-w64-mingw32");
                dirs.push(PathBuf::from("/usr/x86_64-w64-mingw32/lib"));
                dirs.push(PathBuf::from("/usr/lib/x86_64-w64-mingw32"));
                collect_mingw_path_dirs(&mut dirs, "x86_64-w64-mingw32");
            }
        }
        OperatingSystem::Android => {
            collect_android_search_dirs(&mut dirs, target);
        }
    }
    dirs
}

fn collect_android_search_dirs(dirs: &mut Vec<PathBuf>, target: TargetTriple) {
    if let Ok(ndk) = std::env::var("ANDROID_NDK_HOME").or_else(|_| std::env::var("ANDROID_NDK_ROOT")) {
        let p = PathBuf::from(ndk);
        let triple_str = target.as_str();
        dirs.push(p.join(format!("toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/lib/{triple_str}/26")));
        dirs.push(p.join("toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/lib"));
    }
    dirs.push(PathBuf::from("/system/lib64"));
    dirs.push(PathBuf::from("/system/lib"));
}

fn collect_windows_msvc_dirs(dirs: &mut Vec<PathBuf>) {
    if let Ok(lib) = std::env::var("LIB") {
        for dir in lib.split(';').map(PathBuf::from) {
            if dir.is_dir() && !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
    }
    collect_vs_vctools_dirs(dirs);
    collect_windows_kits_dirs(dirs);
}

fn collect_vs_vctools_dirs(dirs: &mut Vec<PathBuf>) {
    let Some(program_files_x86) = program_files_x86() else {
        return;
    };
    let vswhere = program_files_x86
        .join("Microsoft Visual Studio")
        .join("Installer")
        .join("vswhere.exe");
    let output = match Command::new(&vswhere)
        .args(["-latest", "-products", "*", "-property", "installationPath"])
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return,
    };
    let vs_root = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    let msvc_root = vs_root.join("VC").join("Tools").join("MSVC");
    if let Some(latest) = latest_version_dir(&msvc_root) {
        let lib = latest.join("lib").join("x64");
        if lib.is_dir() && !dirs.contains(&lib) {
            dirs.push(lib);
        }
    }
}

fn collect_windows_kits_dirs(dirs: &mut Vec<PathBuf>) {
    let Some(program_files_x86) = program_files_x86() else {
        return;
    };
    let kits_root = program_files_x86
        .join("Windows Kits")
        .join("10")
        .join("Lib");
    let Some(latest) = latest_version_dir(&kits_root) else {
        return;
    };
    for part in ["um", "ucrt"] {
        let lib = latest.join(part).join("x64");
        if lib.is_dir() && !dirs.contains(&lib) {
            dirs.push(lib);
        }
    }
}

fn program_files_x86() -> Option<PathBuf> {
    std::env::var_os("ProgramFiles(x86)").map(PathBuf::from)
}

fn latest_version_dir(root: &Path) -> Option<PathBuf> {
    fs::read_dir(root)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .and_then(|name| name.chars().next())
                    .is_some_and(|c| c.is_ascii_digit())
        })
        .max()
}

pub fn collect_gcc_lib_dirs(dirs: &mut Vec<PathBuf>, keyword: &str) {
    let gcc_root = Path::new("/usr/lib/gcc");
    let Ok(entries) = fs::read_dir(gcc_root) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() || !p.to_string_lossy().contains(keyword) {
            continue;
        }
        if let Ok(sub_entries) = fs::read_dir(&p) {
            for sub in sub_entries.flatten() {
                let sub_p = sub.path();
                if sub_p.is_dir() {
                    dirs.push(sub_p);
                }
            }
        }
    }
}

/// On native Windows the `/usr/...` search paths used by the Linux driver
/// do not exist. Derive mingw library locations from a toolchain tool
/// found in PATH instead: for a tool at `<prefix>/bin/tool.exe`, import
/// libraries live in `<prefix>/<triple>/lib` and GCC runtime libraries in
/// `<prefix>/lib/gcc/<triple>/<version>`. No absolute paths are hardcoded;
/// the layout is probed from whatever mingw toolchain is installed.
fn collect_mingw_path_dirs(dirs: &mut Vec<PathBuf>, triple: &str) {
    for tool in ["gcc", "clang", "cc", "ld", "ld.lld"] {
        let Some(tool_path) = find_tool_in_path(tool) else {
            continue;
        };
        let Some(root) = tool_path.parent().and_then(Path::parent) else {
            continue;
        };

        // Unified lib dir (`<prefix>/lib`) holds crt2.o, import libraries
        // and, on some layouts, the GCC runtime. The per-triple dir
        // (`<prefix>/<triple>/lib`) is the layout used by llvm-mingw and
        // cross toolchains. Probe both.
        let lib_dir = root.join("lib");
        if lib_dir.is_dir() && !dirs.contains(&lib_dir) {
            dirs.push(lib_dir);
        }

        let triple_lib = root.join(triple).join("lib");
        if triple_lib.is_dir() && !dirs.contains(&triple_lib) {
            dirs.push(triple_lib);
        }

        let gcc_root = root.join("lib").join("gcc");
        if let Ok(entries) = fs::read_dir(&gcc_root) {
            for entry in entries.flatten() {
                let p = entry.path();
                if !p.is_dir() || !p.to_string_lossy().contains(triple) {
                    continue;
                }
                if let Ok(sub_entries) = fs::read_dir(&p) {
                    for sub in sub_entries.flatten() {
                        let sub_p = sub.path();
                        if sub_p.is_dir() && !dirs.contains(&sub_p) {
                            dirs.push(sub_p);
                        }
                    }
                }
            }
        }
    }
}

fn find_tool_in_path(tool: &str) -> Option<PathBuf> {
    let name = if cfg!(target_os = "windows") {
        format!("{tool}.exe")
    } else {
        tool.to_string()
    };
    let path_env = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_env) {
        let candidate = dir.join(&name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn find_file_in_dirs(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter()
        .map(|d| d.join(name))
        .find(|candidate| candidate.is_file())
}

pub fn find_default_runtime_archive(target: TargetTriple) -> Option<PathBuf> {
    let t_str = target.as_str();
    let local_candidates = [
        format!("yuki-work/local/runtime/{t_str}/libyukino_rt.a"),
        format!("yukino/local/runtime/{t_str}/libyukino_rt.a"),
    ];
    for c in &local_candidates {
        let p = PathBuf::from(c);
        if p.is_file() {
            return Some(p);
        }
    }

    if let Ok(unpacked) = crate::embedded_runtime::ensure_runtime_archive(target) {
        return Some(unpacked);
    }

    let fallback_candidates = [
        format!("target/{t_str}/release/libyukino_rt.a"),
        format!("target/{t_str}/debug/libyukino_rt.a"),
    ];
    for c in &fallback_candidates {
        let p = PathBuf::from(c);
        if p.is_file() {
            return Some(p);
        }
    }

    find_runtime_archive_near_exe(target, t_str)
}

fn find_runtime_archive_near_exe(target: TargetTriple, t_str: &str) -> Option<PathBuf> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf));
    if let Some(dir) = exe_dir {
        let target_candidates = [
            dir.join(format!("runtime/{t_str}/libyukino_rt.a")),
            dir.join(format!("../{t_str}/release/libyukino_rt.a")),
            dir.join(format!("../{t_str}/debug/libyukino_rt.a")),
        ];
        for p in &target_candidates {
            if p.is_file() { return Some(p.clone()); }
        }
        if target == TargetTriple::host() {
            for candidate in ["libyukino_rt.a", "../release/libyukino_rt.a", "../debug/libyukino_rt.a"] {
                let p = dir.join(candidate);
                if p.is_file() { return Some(p); }
            }
        }
    }
    None
}
