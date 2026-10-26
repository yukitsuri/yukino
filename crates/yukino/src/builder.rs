#![deny(unsafe_code)]

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use yukic_target::{Architecture, OperatingSystem, TargetTriple};

use crate::config::resolve_pool_threads;
use crate::dashboard::BuildDashboard;
use crate::features::unify_dependency_features;
use crate::lockfile::{
    LOCKFILE_REL_PATH, LockedPackage, Lockfile, collect_dir_sources, compute_source_dir_checksum,
};
use crate::manifest::{DependencySpec, ProfileConfig, ProjectManifest};
use crate::project::ProjectError;
use crate::tracker::ThreadStatus;
use crate::{VERSION, YUKIC_NAME, should_color};

pub fn target_os_str(target: TargetTriple) -> &'static str {
    match target.spec().os {
        OperatingSystem::Windows => "windows",
        OperatingSystem::Linux => "linux",
        OperatingSystem::Android => "android",
    }
}

pub fn target_arch_str(target: TargetTriple) -> &'static str {
    match target.spec().arch {
        Architecture::X86_64 => "x86_64",
        Architecture::Aarch64 => "aarch64",
        Architecture::Armv7 => "armv7",
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOptions {
    pub profile: String,
    pub target: Option<String>,
    pub runtime: Option<PathBuf>,
    pub no_runtime: bool,
    pub rebuild: bool,
    pub refresh_deps: bool,
    pub overrides: Vec<(String, String)>,
    pub features: Vec<String>,
    pub no_default_features: bool,
    pub all_features: bool,
    pub package: Option<String>,
    pub run: bool,
    pub run_args: Vec<String>,
    pub no_warns: bool,
    pub extra_info: bool,
    pub no_out: bool,
    pub no_lib_log: bool,
    pub lib_warns: bool,
    pub threads: Option<usize>,
    pub cgunits: Option<usize>,
    pub project_dir: PathBuf,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            profile: "debug".to_string(),
            target: None,
            runtime: None,
            no_runtime: false,
            rebuild: false,
            refresh_deps: false,
            overrides: Vec::new(),
            features: Vec::new(),
            no_default_features: false,
            all_features: false,
            package: None,
            run: false,
            run_args: Vec::new(),
            no_warns: false,
            extra_info: false,
            no_out: false,
            no_lib_log: false,
            lib_warns: false,
            threads: None,
            cgunits: None,
            project_dir: PathBuf::from("."),
        }
    }
}

#[allow(clippy::collapsible_if)]
pub fn resolve_compiler_exe() -> PathBuf {
    if let Some(cargo_bin) = std::env::var_os("CARGO_BIN_EXE_yukic") {
        let p = PathBuf::from(cargo_bin);
        if p.is_file() {
            return p;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for d in [Some(dir), dir.parent()].into_iter().flatten() {
                for name in [YUKIC_NAME, "yukic"] {
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
    PathBuf::from(YUKIC_NAME)
}

pub fn resolve_artifact_path(
    project_dir: &Path,
    name: &str,
    namedbg: Option<&str>,
    kind: &str,
    profile: &str,
    target: TargetTriple,
) -> PathBuf {
    let base = project_dir
        .join("yuki-work/local/build")
        .join(profile)
        .join(target.as_str())
        .join(name);

    match kind {
        "static" => base.join(format!("lib{name}.a")),
        "shared" => {
            if target.spec().os == OperatingSystem::Windows {
                base.join(format!("{name}.dll"))
            } else {
                base.join(format!("lib{name}.so"))
            }
        }
        "macro" => {
            let macro_dir = project_dir.join("yuki-work/local/build/macros");
            if target.spec().os == OperatingSystem::Windows {
                macro_dir.join(format!("{name}.exe"))
            } else {
                macro_dir.join(name)
            }
        }
        _ => {
            let bin_stem = if profile == "debug" {
                namedbg.unwrap_or(name)
            } else {
                name
            };
            if target.spec().os == OperatingSystem::Windows {
                base.join(format!("{bin_stem}.exe"))
            } else {
                base.join(bin_stem)
            }
        }
    }
}

struct DepBuildEnv<'a> {
    project_dir: &'a Path,
    host_manifest: Option<PathBuf>,
    host_workspace: Option<PathBuf>,
    target: TargetTriple,
    app_profile: &'a str,
    app_profile_cfg: &'a ProfileConfig,
    overrides: &'a [(String, String)],
    compiler: &'a Path,
    rebuild: bool,
    refresh_deps: bool,
    no_lib_log: bool,
    no_warns: bool,
    threads: usize,
    cgunits: usize,
    dashboard: &'a BuildDashboard,
}

#[allow(clippy::too_many_arguments)]
fn invoke_yukic_for_dep(
    compiler: &Path,
    entry: &Path,
    out_archive: &Path,
    target: TargetTriple,
    cfg: &ProfileConfig,
    profile_name: &str,
    manifest_path: Option<&Path>,
    host_manifest: Option<&Path>,
    host_workspace: Option<&Path>,
    cache_dir: Option<&Path>,
    features: &[String],
    dashboard: &BuildDashboard,
    no_lib_log: bool,
    no_warns: bool,
    threads: usize,
    cgunits: usize,
    is_macro: bool,
) -> Result<(), ProjectError> {
    let mut cmd = Command::new(compiler);
    cmd.arg("--entry").arg(entry);
    if is_macro {
        cmd.arg("--output-type").arg("app");
        cmd.arg("--linker").arg("lld");
    } else {
        cmd.arg("--output-type").arg("static");
    }
    cmd.arg("-o").arg(out_archive);
    cmd.arg("--target").arg(target.as_str());
    cmd.arg("-O").arg(cfg.opt_level_str());
    cmd.arg("--profile-name").arg(profile_name);
    cmd.arg("--threads").arg(threads.to_string());
    cmd.arg("--cgunits").arg(cgunits.to_string());
    if let Some(m) = manifest_path {
        cmd.arg("--manifest").arg(m);
    }
    if let Some(hm) = host_manifest {
        cmd.arg("--host-manifest").arg(hm);
    }
    if let Some(hw) = host_workspace {
        cmd.arg("--host-workspace").arg(hw);
    }
    if let Some(c) = cache_dir {
        cmd.arg("--cache-dir").arg(c);
    }
    if !features.is_empty() {
        cmd.arg("--features").arg(features.join(","));
    }
    if no_warns {
        cmd.arg("--no-warns");
    }
    if cfg.lto_enabled() {
        cmd.arg("--lto");
    }
    if cfg.strip_enabled() {
        cmd.arg("--strip");
    }
    if cfg.strip_unused_enabled() {
        cmd.arg("--strip-unused");
    }
    if let Some(strategy) = &cfg.panic_strategy {
        cmd.arg("--panic-strategy").arg(strategy);
    }
    let macro_timeout = crate::config::load_global_macrotimeout();
    cmd.arg("--macro-timeout").arg(macro_timeout.to_string());

    let status = if no_lib_log {
        let output = cmd
            .output()
            .map_err(|e| ProjectError::Io(format!("failed to run command: {e}")))?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            dashboard.log(&err);
        }
        output.status
    } else {
        dashboard.run_lib_compiler_command(&mut cmd)?
    };

    if !status.success() {
        return Err(ProjectError::Runtime(format!(
            "compilation of dependency failed: {}",
            out_archive.display()
        )));
    }
    Ok(())
}

fn record_locked_dep(
    name: &str,
    version: &str,
    source: String,
    checksum: String,
    commit: Option<String>,
    features: Vec<String>,
    kind: &str,
) -> LockedPackage {
    LockedPackage {
        name: name.to_string(),
        version: version.to_string(),
        kind: kind.to_string(),
        source,
        checksum,
        commit,
        features,
        dependencies: Vec::new(),
    }
}

#[derive(Debug, Clone)]
pub struct BuiltDependency {
    pub name: String,
    pub archive: PathBuf,
    pub source_dir: PathBuf,
    pub static_archives: Vec<PathBuf>,
    pub system_libs: Vec<String>,
    pub macro_binary: Option<PathBuf>,
}

fn resolve_dep_source_desc(dep_spec: &DependencySpec) -> String {
    if let Some(rel) = &dep_spec.path {
        format!("path+{}", rel.display())
    } else if let Some(url) = &dep_spec.link {
        format!("link+{url}")
    } else if let Some(git) = &dep_spec.git {
        format!("git+{git}")
    } else {
        "unknown".to_string()
    }
}

fn collect_dep_static_specs(
    dep_manifest: &ProjectManifest,
    dep_dir: &Path,
    target: TargetTriple,
) -> (Vec<PathBuf>, Vec<String>) {
    let mut archives = Vec::new();
    let specs = dep_manifest.static_libs(Some(target.as_str()));
    for arch in specs.archives {
        let resolved = if arch.is_relative() {
            dep_dir.join(&arch)
        } else {
            arch
        };
        archives.push(resolved);
    }
    for obj in specs.objects {
        let resolved = if obj.is_relative() {
            dep_dir.join(&obj)
        } else {
            obj
        };
        archives.push(resolved);
    }

    if let Ok(Some((ws, _))) = crate::manifest::WorkspaceManifest::load_from_dir(dep_dir) {
        let ws_manifest = ProjectManifest::Workspace(ws);
        let ws_specs = ws_manifest.static_libs(Some(target.as_str()));
        for arch in ws_specs.archives {
            let resolved = if arch.is_relative() { dep_dir.join(&arch) } else { arch };
            if !archives.contains(&resolved) {
                archives.push(resolved);
            }
        }
        for obj in ws_specs.objects {
            let resolved = if obj.is_relative() { dep_dir.join(&obj) } else { obj };
            if !archives.contains(&resolved) {
                archives.push(resolved);
            }
        }
        let mut system = specs.system;
        for s in ws_specs.system {
            if !system.contains(&s) {
                system.push(s);
            }
        }
        return (archives, system);
    }

    (archives, specs.system)
}

fn resolve_dep_artifact_path(
    project_dir: &Path,
    dep_name: &str,
    target: TargetTriple,
    is_macro: bool,
) -> (PathBuf, PathBuf) {
    if is_macro {
        let dir = project_dir.join("yuki-work/local/build/macros");
        let exe_name = if cfg!(target_os = "windows") {
            format!("{dep_name}.exe")
        } else {
            dep_name.to_string()
        };
        (dir.join(exe_name), dir)
    } else {
        let dir = project_dir
            .join("yuki-work/local/build/static")
            .join(target.as_str())
            .join(dep_name);
        (dir.join(format!("lib{dep_name}.a")), dir)
    }
}

fn resolve_dep_entry_file(dep_dir: &Path, dep_path: &Path, dep_manifest: &ProjectManifest) -> PathBuf {
    if dep_manifest.entry().is_absolute() {
        dep_manifest.entry().to_path_buf()
    } else if let Some(parent) = dep_path.parent() {
        let candidate = parent.join(dep_manifest.entry());
        if candidate.is_file() {
            candidate
        } else {
            dep_dir.join(dep_manifest.entry())
        }
    } else {
        dep_dir.join(dep_manifest.entry())
    }
}

fn build_single_dependency(
    dep_name: &str,
    dep_spec: &DependencySpec,
    env: &DepBuildEnv,
    thread_id: usize,
    lockfile: &Lockfile,
) -> Result<(BuiltDependency, LockedPackage), ProjectError> {
    let (dep_dir, fetch_checksum, commit) = crate::fetch::resolve_dependency_dir(
        dep_name,
        dep_spec,
        env.project_dir,
        thread_id,
        env.dashboard,
        env.refresh_deps,
        lockfile,
    )?;

    let (dep_manifest, dep_path) = ProjectManifest::load_with_package(
        &dep_dir,
        dep_spec.package.as_deref(),
    )?;
    let is_macro = dep_manifest.is_macro();
    let dep_target = if is_macro { TargetTriple::host() } else { env.target };

    let (chosen_profile, mut dep_cfg) =
        dep_manifest.resolve_inherited_profile(env.app_profile, env.app_profile_cfg);

    for (k, v) in env.overrides {
        let _ = dep_cfg.apply_override(k, v);
    }

    let (mut dep_static_archives, mut dep_system_libs) =
        collect_dep_static_specs(&dep_manifest, &dep_dir, dep_target);

    let dep_build_deps = dep_manifest.build_deps(Some(dep_target.as_str()));
    let mut dep_build_link_archives = Vec::new();
    let mut dep_build_include_dirs = Vec::new();
    let mut dep_build_connected_libs = Vec::new();
    if !dep_build_deps.is_empty() {
        let mut dep_lockfile = Lockfile::new(dep_target.as_str(), VERSION, env.app_profile);
        let dep_params = BuildDepParams {
            project_dir: &dep_dir,
            target: dep_target,
            profile: &chosen_profile,
            project_name: dep_name,
            compiler: env.compiler,
            dashboard: env.dashboard,
            refresh_deps: env.refresh_deps,
        };
        run_build_dependencies(
            &dep_build_deps,
            &dep_params,
            &mut dep_static_archives,
            &mut dep_system_libs,
            &mut dep_build_include_dirs,
            &mut dep_build_link_archives,
            &mut dep_build_connected_libs,
            &mut dep_lockfile,
        )?;
    }

    let dep_build_script = dep_dir.join("build.ykn");
    if dep_build_script.is_file() {
        run_build_script(
            &dep_build_script,
            &dep_dir,
            dep_target,
            &chosen_profile,
            dep_name,
            env.compiler,
            &dep_build_include_dirs,
            &dep_build_link_archives,
            &dep_build_connected_libs,
            &mut dep_static_archives,
            &mut dep_system_libs,
            env.dashboard,
        )?;
    }

    let (dep_artifact, out_dir) = resolve_dep_artifact_path(
        env.project_dir,
        dep_name,
        dep_target,
        is_macro,
    );
    fs::create_dir_all(&out_dir).map_err(|e| ProjectError::Io(e.to_string()))?;

    let feat_str = dep_spec.features.join(",");
    let manifest_file_name = dep_path.file_name().and_then(|n| n.to_str()).unwrap_or("lib.toml");
    let source_checksum = compute_source_dir_checksum(
        &dep_dir,
        manifest_file_name,
        &[dep_target.as_str(), &chosen_profile, dep_cfg.opt_level_str(), &feat_str],
    )
    .map_err(|e| ProjectError::Io(e.to_string()))?;

    let checksum = if !fetch_checksum.is_empty() {
        fetch_checksum
    } else {
        source_checksum
    };

    let locked_pkg = record_locked_dep(
        dep_name,
        dep_manifest.version(),
        resolve_dep_source_desc(dep_spec),
        checksum.clone(),
        commit,
        dep_spec.features.clone(),
        if is_macro { "macro" } else { "static" },
    );

    let should_rebuild = env.rebuild || env.refresh_deps;
    let kind_str = if is_macro { "macro" } else { "library" };
    if !should_rebuild && dep_artifact.is_file() && lockfile.verify_checksum(dep_name, &checksum) {
        if !env.no_lib_log {
            env.dashboard.log(&format!(
                "       Using cached: {dep_name} {kind_str}",
            ));
        }
        return Ok((
            BuiltDependency {
                name: dep_name.to_string(),
                archive: dep_artifact.clone(),
                source_dir: dep_dir,
                static_archives: dep_static_archives,
                system_libs: dep_system_libs,
                macro_binary: if is_macro { Some(dep_artifact) } else { None },
            },
            locked_pkg,
        ));
    }

    if should_rebuild && dep_artifact.exists() {
        let _ = fs::remove_file(&dep_artifact);
    }

    if !env.no_lib_log {
        if should_color() {
            use owo_colors::OwoColorize;
            env.dashboard.log(&format!(
                "{} {dep_name} {kind_str}",
                "   Compiling".green().bold()
            ));
        } else {
            env.dashboard.log(&format!("   Compiling {dep_name} {kind_str}"));
        }
    }
    let entry = resolve_dep_entry_file(&dep_dir, &dep_path, &dep_manifest);
    let dep_cache_dir = env
        .project_dir
        .join("yuki-work/local/cache/cgu")
        .join(&chosen_profile)
        .join(dep_target.as_str())
        .join(dep_name);
    if should_rebuild && dep_cache_dir.exists() {
        let _ = fs::remove_dir_all(&dep_cache_dir);
    }
    fs::create_dir_all(&dep_cache_dir).map_err(|e| ProjectError::Io(e.to_string()))?;
    if is_macro {
        let macro_opts = BuildOptions {
            project_dir: dep_dir.clone(),
            profile: chosen_profile.clone(),
            target: Some(dep_target.as_str().to_string()),
            rebuild: should_rebuild,
            refresh_deps: env.refresh_deps,
            features: dep_spec.features.clone(),
            no_lib_log: env.no_lib_log,
            no_warns: env.no_warns,
            threads: Some(env.threads),
            cgunits: Some(env.cgunits),
            ..BuildOptions::default()
        };
        let built_macro_path = build_project(&macro_opts)?;
        if built_macro_path != dep_artifact {
            fs::copy(&built_macro_path, &dep_artifact).map_err(|e| ProjectError::Io(e.to_string()))?;
        }
    } else {
        invoke_yukic_for_dep(
            env.compiler,
            &entry,
            &dep_artifact,
            dep_target,
            &dep_cfg,
            &chosen_profile,
            Some(&dep_path),
            env.host_manifest.as_deref(),
            env.host_workspace.as_deref(),
            Some(&dep_cache_dir),
            &dep_spec.features,
            env.dashboard,
            env.no_lib_log,
            env.no_warns,
            env.threads,
            env.cgunits,
            false,
        )?;
    }

    if !env.no_lib_log {
        if should_color() {
            use owo_colors::OwoColorize;
            env.dashboard.log(&format!(
                "{} {dep_name} {kind_str}",
                "    Compiled".green().bold()
            ));
        } else {
            env.dashboard.log(&format!("    Compiled {dep_name} {kind_str}"));
        }
    }

    Ok((
        BuiltDependency {
            name: dep_name.to_string(),
            archive: dep_artifact.clone(),
            source_dir: dep_dir,
            static_archives: dep_static_archives,
            system_libs: dep_system_libs,
            macro_binary: if is_macro { Some(dep_artifact) } else { None },
        },
        locked_pkg,
    ))
}

#[allow(clippy::too_many_arguments)]
fn run_build_script(
    script_path: &Path,
    project_dir: &Path,
    target: TargetTriple,
    profile: &str,
    name: &str,
    compiler: &Path,
    build_include_dirs: &[PathBuf],
    build_link_archives: &[PathBuf],
    build_connected_libs: &[String],
    link_archives: &mut Vec<PathBuf>,
    link_systems: &mut Vec<String>,
    dashboard: &BuildDashboard,
) -> Result<(), ProjectError> {
    if !script_path.is_file() {
        return Ok(());
    }

    let rel_script = format_relative_path(script_path);
    if should_color() {
        use owo_colors::OwoColorize;
        dashboard.log(&format!(
            "{} build script: {}",
            "   Compiling".green().bold(),
            rel_script.display()
        ));
    } else {
        dashboard.log(&format!("   Compiling build script: {}", rel_script.display()));
    }
    let exe_name = if cfg!(windows) { "build.exe" } else { "build" };

    // Compile the script in an isolated directory. The compiler walks the
    // entry's parent directory to collect sibling modules; for a dependency
    // that directory also holds the library sources, which would pull the
    // library code (and its native symbols) into the build script binary
    // before those symbols even exist. The script reads all project paths
    // from YUKINO_* environment variables, so the isolated copy behaves
    // identically to the original.
    let out_dir = project_dir
        .join("yuki-work/local/build")
        .join(profile)
        .join(target.as_str())
        .join(name);
    fs::create_dir_all(&out_dir).map_err(|e| ProjectError::Io(e.to_string()))?;
    let script_dir = out_dir.join("_script");
    fs::create_dir_all(&script_dir).map_err(|e| ProjectError::Io(e.to_string()))?;
    let isolated_script = script_dir.join(script_path.file_name().unwrap_or_default());
    fs::copy(script_path, &isolated_script).map_err(|e| ProjectError::Io(e.to_string()))?;

    let build_bin = script_dir.join(exe_name);

    let mut compile_cmd = Command::new(compiler);
    compile_cmd.current_dir(&script_dir);
    compile_cmd.arg("--target").arg("host");
    compile_cmd.arg("-o").arg(&build_bin);
    let manifest_file = project_dir.join("yuki.toml");
    if manifest_file.is_file() {
        compile_cmd.arg("--manifest").arg(&manifest_file);
    }
    compile_cmd.arg(&isolated_script);

    for inc in build_include_dirs {
        compile_cmd.arg("--include").arg(inc);
    }
    for arch in build_link_archives {
        compile_cmd.arg("--link-archive").arg(arch);
    }
    for lib in build_connected_libs {
        compile_cmd.arg("--connected-lib").arg(lib);
    }

    let status = dashboard.run_compiler_command(&mut compile_cmd)?;
    if !status.success() {
        let _ = fs::remove_file(&build_bin);
        return Err(ProjectError::Runtime("failed to compile build.ykn".to_string()));
    }

    dashboard.log(&format!("     Running pre-build script: {exe_name}"));


    let mut run_cmd = Command::new(&build_bin);
    run_cmd.current_dir(project_dir);
    run_cmd.env("YUKINO_OUT_DIR", &out_dir);
    run_cmd.env("YUKINO_PROJECT_DIR", project_dir);
    run_cmd.env("YUKINO_TARGET_TRIPLE", target.as_str());
    run_cmd.env("YUKINO_TARGET_OS", target_os_str(target));
    run_cmd.env("YUKINO_TARGET_ARCH", target_arch_str(target));
    run_cmd.env("YUKINO_PROFILE", profile);

    let output = run_cmd.output();
    let _ = fs::remove_file(&build_bin);

    let output = output.map_err(|e| ProjectError::Io(e.to_string()))?;
    if !output.status.success() {
        return Err(ProjectError::Runtime("build.ykn execution failed".to_string()));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    for line in stdout_str.lines() {
        if let Some(sys) = line.strip_prefix("yuki:link-system=") {
            link_systems.push(sys.trim().to_string());
        } else if let Some(arch) = line.strip_prefix("yuki:link-archive=") {
            let arch_path = PathBuf::from(arch.trim());
            let resolved = if arch_path.is_relative() {
                project_dir.join(arch_path)
            } else {
                arch_path
            };
            link_archives.push(resolved);
        } else if let Some(warn) = line.strip_prefix("yuki:warning=") {
            dashboard.log(&format!("warning: {}", warn.trim()));
        }
    }

    Ok(())
}

struct BuildDepParams<'a> {
    project_dir: &'a Path,
    target: TargetTriple,
    profile: &'a str,
    project_name: &'a str,
    compiler: &'a Path,
    dashboard: &'a BuildDashboard,
    refresh_deps: bool,
}

fn resolve_build_dep_entry(
    dep_dir: &Path,
    dep_manifest: Option<&ProjectManifest>,
) -> Result<PathBuf, ProjectError> {
    if dep_dir.join("build.ykn").is_file() {
        Ok(dep_dir.join("build.ykn"))
    } else if let Some(m) = dep_manifest {
        Ok(dep_dir.join(m.entry()))
    } else if dep_dir.join("src/main.ykn").is_file() {
        Ok(dep_dir.join("src/main.ykn"))
    } else if dep_dir.join("main.ykn").is_file() {
        Ok(dep_dir.join("main.ykn"))
    } else {
        Err(ProjectError::Runtime(format!(
            "build dependency has no entry file in '{}'",
            dep_dir.display()
        )))
    }
}

fn execute_build_dep_binary(
    build_bin: &Path,
    project_dir: &Path,
    out_dir: &Path,
    target: TargetTriple,
    profile: &str,
    dep_name: &str,
    link_archives: &mut Vec<PathBuf>,
    link_systems: &mut Vec<String>,
    dashboard: &BuildDashboard,
) -> Result<(), ProjectError> {
    let mut run_cmd = Command::new(build_bin);
    run_cmd.current_dir(project_dir);
    run_cmd.env("YUKINO_OUT_DIR", out_dir);
    run_cmd.env("YUKINO_PROJECT_DIR", project_dir);
    run_cmd.env("YUKINO_TARGET_TRIPLE", target.as_str());
    run_cmd.env("YUKINO_TARGET_OS", target_os_str(target));
    run_cmd.env("YUKINO_TARGET_ARCH", target_arch_str(target));
    run_cmd.env("YUKINO_PROFILE", profile);

    let output = run_cmd.output().map_err(|e| ProjectError::Io(e.to_string()))?;
    if !output.status.success() {
        return Err(ProjectError::Runtime(format!(
            "build dependency '{dep_name}' execution failed"
        )));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    for line in stdout_str.lines() {
        if let Some(sys) = line.strip_prefix("yuki:link-system=") {
            link_systems.push(sys.trim().to_string());
        } else if let Some(arch) = line.strip_prefix("yuki:link-archive=") {
            let arch_path = PathBuf::from(arch.trim());
            let resolved = if arch_path.is_relative() {
                project_dir.join(arch_path)
            } else {
                arch_path
            };
            link_archives.push(resolved);
        } else if let Some(warn) = line.strip_prefix("yuki:warning=") {
            dashboard.log(&format!("warning: [{dep_name}] {}", warn.trim()));
        }
    }
    Ok(())
}

fn run_single_build_dependency(
    dep_name: &str,
    dep_spec: &DependencySpec,
    params: &BuildDepParams,
    link_archives: &mut Vec<PathBuf>,
    link_systems: &mut Vec<String>,
    build_script_include_dirs: &mut Vec<PathBuf>,
    build_script_link_archives: &mut Vec<PathBuf>,
    build_script_connected_libs: &mut Vec<String>,
    lockfile: &mut Lockfile,
) -> Result<(), ProjectError> {
    let (dep_dir, fetch_checksum, commit) = crate::fetch::resolve_dependency_dir(
        dep_name,
        dep_spec,
        params.project_dir,
        0,
        params.dashboard,
        params.refresh_deps,
        lockfile,
    )?;

    let dep_manifest = ProjectManifest::load(&dep_dir).ok();
    let is_library = dep_manifest
        .as_ref()
        .map_or(false, |(m, _)| matches!(m, ProjectManifest::Lib(_)));

    let dep_version = dep_manifest
        .as_ref()
        .map(|(m, _)| m.version().to_string())
        .unwrap_or_else(|| "0.1.0".to_string());

    let host_deps_dir = params.project_dir.join("yuki-work/local/build/host/deps").join(dep_name);
    fs::create_dir_all(&host_deps_dir).map_err(|e| ProjectError::Io(e.to_string()))?;

    if is_library {
        let (manifest, manifest_path) = dep_manifest.unwrap();
        let entry_file = resolve_dep_entry_file(&dep_dir, &manifest_path, &manifest);
        let archive_path = host_deps_dir.join(format!("lib{dep_name}.a"));

        if should_color() {
            use owo_colors::OwoColorize;
            params.dashboard.log(&format!("{} build library: {dep_name}", "   Compiling".green().bold()));
        } else {
            params.dashboard.log(&format!("   Compiling build library: {dep_name}"));
        }

        let mut compile_cmd = Command::new(params.compiler);
        compile_cmd.current_dir(&dep_dir);
        compile_cmd.arg("--target").arg("host");
        compile_cmd.arg("--output-type").arg("static");
        compile_cmd.arg("--manifest").arg(&manifest_path);
        compile_cmd.arg("-o").arg(&archive_path);
        compile_cmd.arg(&entry_file);

        let status = params.dashboard.run_compiler_command(&mut compile_cmd)?;
        if !status.success() {
            let _ = fs::remove_file(&archive_path);
            return Err(ProjectError::Runtime(format!("failed to compile build library '{dep_name}'")));
        }

        build_script_link_archives.push(archive_path);
        build_script_include_dirs.push(dep_dir.clone());
        build_script_connected_libs.push(dep_name.to_string());

        let host_target = TargetTriple::host();
        let checksum = if !fetch_checksum.is_empty() {
            fetch_checksum
        } else {
            compute_source_dir_checksum(&dep_dir, "lib.toml", &[host_target.as_str(), "dev"])
                .or_else(|_| compute_source_dir_checksum(&dep_dir, "yuki.toml", &[host_target.as_str(), "dev"]))
                .unwrap_or_default()
        };

        let locked_pkg = record_locked_dep(
            dep_name,
            &dep_version,
            resolve_dep_source_desc(dep_spec),
            checksum,
            commit,
            dep_spec.features.clone(),
            "build",
        );
        lockfile.upsert_package(locked_pkg);
        return Ok(());
    }

    let entry_file = resolve_build_dep_entry(&dep_dir, dep_manifest.as_ref().map(|(m, _)| m))?;

    let exe_name = if cfg!(windows) { format!("{dep_name}.exe") } else { dep_name.to_string() };
    let build_bin = host_deps_dir.join(&exe_name);

    if should_color() {
        use owo_colors::OwoColorize;
        params.dashboard.log(&format!("{} build dependency: {dep_name}", "   Compiling".green().bold()));
    } else {
        params.dashboard.log(&format!("   Compiling build dependency: {dep_name}"));
    }

    let mut compile_cmd = Command::new(params.compiler);
    compile_cmd.current_dir(&dep_dir);
    compile_cmd.arg("--target").arg("host");
    compile_cmd.arg("-o").arg(&build_bin);
    compile_cmd.arg(&entry_file);

    let status = params.dashboard.run_compiler_command(&mut compile_cmd)?;
    if !status.success() {
        let _ = fs::remove_file(&build_bin);
        return Err(ProjectError::Runtime(format!("failed to compile build dependency '{dep_name}'")));
    }

    params.dashboard.log(&format!("     Running build dependency: {dep_name}"));
    let out_dir = params.project_dir
        .join("yuki-work/local/build")
        .join(params.profile)
        .join(params.target.as_str())
        .join(params.project_name);

    let exec_res = execute_build_dep_binary(
        &build_bin,
        params.project_dir,
        &out_dir,
        params.target,
        params.profile,
        dep_name,
        link_archives,
        link_systems,
        params.dashboard,
    );
    let _ = fs::remove_file(&build_bin);
    exec_res?;

    let checksum = if !fetch_checksum.is_empty() {
        fetch_checksum
    } else {
        compute_source_dir_checksum(&dep_dir, "lib.toml", &[TargetTriple::host().as_str(), "dev"])
            .or_else(|_| compute_source_dir_checksum(&dep_dir, "yuki.toml", &[TargetTriple::host().as_str(), "dev"]))
            .unwrap_or_default()
    };

    let locked_pkg = record_locked_dep(
        dep_name,
        &dep_version,
        resolve_dep_source_desc(dep_spec),
        checksum,
        commit,
        dep_spec.features.clone(),
        "build",
    );
    lockfile.upsert_package(locked_pkg);
    Ok(())
}

fn run_build_dependencies(
    build_deps: &HashMap<String, DependencySpec>,
    params: &BuildDepParams,
    link_archives: &mut Vec<PathBuf>,
    link_systems: &mut Vec<String>,
    build_script_include_dirs: &mut Vec<PathBuf>,
    build_script_link_archives: &mut Vec<PathBuf>,
    build_script_connected_libs: &mut Vec<String>,
    lockfile: &mut Lockfile,
) -> Result<(), ProjectError> {
    if build_deps.is_empty() {
        return Ok(());
    }
    let mut sorted_deps: Vec<_> = build_deps.iter().collect();
    sorted_deps.sort_by_key(|(k, _)| (*k).clone());

    for (dep_name, dep_spec) in sorted_deps {
        run_single_build_dependency(
            dep_name,
            dep_spec,
            params,
            link_archives,
            link_systems,
            build_script_include_dirs,
            build_script_link_archives,
            build_script_connected_libs,
            lockfile,
        )?;
    }
    Ok(())
}

fn setup_work_dirs(project_dir: &Path) -> Result<(), ProjectError> {
    let dirs = [
        project_dir.join("yuki-work/local/build"),
        project_dir.join("yuki-work/local/deps"),
        project_dir.join("yuki-work/local/cache"),
        project_dir.join("yuki-work/local/inspect"),
        project_dir.join("yuki-work/common"),
    ];
    for d in dirs {
        fs::create_dir_all(&d).map_err(|e| ProjectError::Io(e.to_string()))?;
    }
    Ok(())
}

pub fn clean_project(project_dir: &Path, all: bool) -> Result<(), ProjectError> {
    let local_dir = project_dir.join("yuki-work/local");
    if local_dir.exists() {
        fs::remove_dir_all(&local_dir)
            .map_err(|e| ProjectError::Io(format!("failed to clean {}: {e}", local_dir.display())))?;
        println!("Cleaned: {}", local_dir.display());
    }

    if all {
        let lockfile_path = project_dir.join(LOCKFILE_REL_PATH);
        if lockfile_path.is_file() {
            fs::remove_file(&lockfile_path).map_err(|e| {
                ProjectError::Io(format!("failed to remove {}: {e}", lockfile_path.display()))
            })?;
            println!("Cleaned: {}", lockfile_path.display());
        }
    }

    Ok(())
}

struct YukicCommandParams<'a> {
    compiler: &'a Path,
    entry_file: &'a Path,
    out_artifact: &'a Path,
    manifest_path: &'a Path,
    project_kind: &'a str,
    target: TargetTriple,
    profile_cfg: &'a ProfileConfig,
    profile_name: &'a str,
    threads: usize,
    cgunits: usize,
    features: &'a [String],
    link_archives: &'a [PathBuf],
    link_systems: &'a [String],
    connected_libs: &'a [String],
    macro_runners: &'a [(String, PathBuf)],
    runtime: Option<&'a Path>,
    no_runtime: bool,
    no_warns: bool,
    extra_info: bool,
    include_dirs: &'a [PathBuf],
    cache_dir: Option<&'a Path>,
}

fn build_yukic_command(p: &YukicCommandParams) -> Command {
    let mut cmd = Command::new(p.compiler);
    cmd.arg("--entry").arg(p.entry_file);
    cmd.arg("-o").arg(p.out_artifact);
    let yukic_output_type = if p.project_kind == "macro" {
        "app"
    } else {
        p.project_kind
    };
    cmd.arg("--output-type").arg(yukic_output_type);
    cmd.arg("--target").arg(p.target.as_str());
    cmd.arg("-O").arg(p.profile_cfg.opt_level_str());
    cmd.arg("--threads").arg(p.threads.to_string());
    cmd.arg("--cgunits").arg(p.cgunits.to_string());
    cmd.arg("--profile-name").arg(p.profile_name);
    cmd.arg("--manifest").arg(p.manifest_path);
    if let Some(cache) = p.cache_dir {
        cmd.arg("--cache-dir").arg(cache);
    }

    if p.profile_cfg.function_sections_enabled() {
        cmd.arg("--function-sections");
    }
    if p.profile_cfg.lto_enabled() {
        cmd.arg("--lto");
    }
    if p.profile_cfg.strip_enabled() {
        cmd.arg("--strip");
    }
    if p.profile_cfg.strip_unused_enabled() {
        cmd.arg("--strip-unused");
    }
    if let Some(strategy) = &p.profile_cfg.panic_strategy {
        cmd.arg("--panic-strategy").arg(strategy);
    }
    if !p.features.is_empty() {
        cmd.arg("--features").arg(p.features.join(","));
    }
    for inc in p.include_dirs {
        cmd.arg("--include").arg(inc);
    }
    for arch in p.link_archives {
        cmd.arg("--link-archive").arg(arch);
    }
    for sys in p.link_systems {
        cmd.arg("--link-system").arg(sys);
    }
    for lib in p.connected_libs {
        cmd.arg("--connected-lib").arg(lib);
    }
    for (name, path) in p.macro_runners {
        cmd.arg("--macro-runner").arg(format!("{}={}", name, path.display()));
    }
    if let Some(rt) = p.runtime {
        cmd.arg("--runtime").arg(rt);
    }
    if p.no_runtime {
        cmd.arg("--no-runtime");
    }
    if p.no_warns {
        cmd.arg("--no-warns");
    }
    if p.extra_info {
        cmd.arg("--extra-info");
    }
    let macro_timeout = crate::config::load_global_macrotimeout();
    cmd.arg("--macro-timeout").arg(macro_timeout.to_string());
    cmd.arg("--linker").arg("lld");
    cmd
}

fn strip_comments_and_strings(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' && chars.peek() == Some(&'/') {
            for next in chars.by_ref() {
                if next == '\n' {
                    out.push('\n');
                    break;
                }
            }
        } else if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(next) = chars.next() {
                if next == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
            }
        } else if c == '"' {
            while let Some(next) = chars.next() {
                if next == '\\' {
                    let _ = chars.next();
                } else if next == '"' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn source_imports_std(src: &str) -> bool {
    let clean = strip_comments_and_strings(src);
    let mut words = clean.split_whitespace();
    while let Some(word) = words.next() {
        if word == "load"
            && let Some(target) = words.next()
        {
            let clean_target = target.trim_end_matches(';').trim_end_matches(':');
            if clean_target == "std" || clean_target.starts_with("std.") {
                return true;
            }
        }
    }
    false
}

fn check_project_uses_std(project_dir: &Path, manifest: &ProjectManifest) -> bool {
    if manifest.runtime_deps(None).contains_key("std") {
        return true;
    }
    let mut rel_files = Vec::new();
    collect_dir_sources(project_dir, project_dir, &mut rel_files);
    for rel in rel_files {
        let full = project_dir.join(rel);
        if let Ok(content) = fs::read_to_string(&full)
            && source_imports_std(&content)
        {
            return true;
        }
    }
    false
}

fn build_runtime_dependencies(
    dep_entries: &[(String, DependencySpec)],
    env: &DepBuildEnv,
    lockfile: &Lockfile,
    threads: usize,
) -> Result<Vec<(BuiltDependency, LockedPackage)>, ProjectError> {
    let total_deps = dep_entries.len();
    if total_deps == 0 {
        return Ok(Vec::new());
    }

    if total_deps == 1 || threads <= 1 {
        let mut results = Vec::with_capacity(total_deps);
        for (idx, (name, spec)) in dep_entries.iter().enumerate() {
            env.dashboard.update_status(
                0,
                ThreadStatus::Compiling {
                    target: name.clone(),
                    current: idx + 1,
                    total: total_deps,
                    speed: None,
                },
            );
            let res = build_single_dependency(name, spec, env, 0, lockfile)?;
            env.dashboard.set_idle(0);
            results.push(res);
        }
        return Ok(results);
    }

    let worker_count = threads.min(total_deps);
    let task_idx = std::sync::atomic::AtomicUsize::new(0);
    let results_slots: Vec<std::sync::Mutex<Option<Result<(BuiltDependency, LockedPackage), ProjectError>>>> =
        (0..total_deps).map(|_| std::sync::Mutex::new(None)).collect();

    let task_idx_ref = &task_idx;
    let results_slots_ref = &results_slots;

    std::thread::scope(|s| {
        for worker_id in 0..worker_count {
            s.spawn(move || {
                loop {
                    let i = task_idx_ref.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if i >= total_deps {
                        break;
                    }
                    let (name, spec) = &dep_entries[i];
                    env.dashboard.update_status(
                        worker_id,
                        ThreadStatus::Compiling {
                            target: name.clone(),
                            current: i + 1,
                            total: total_deps,
                            speed: None,
                        },
                    );
                    let res = build_single_dependency(name, spec, env, worker_id, lockfile);
                    env.dashboard.set_idle(worker_id);
                    if let Ok(mut slot) = results_slots_ref[i].lock() {
                        *slot = Some(res);
                    }
                }
            });
        }
    });

    let mut final_results = Vec::with_capacity(total_deps);
    for slot in results_slots {
        let res = slot
            .into_inner()
            .map_err(|_| ProjectError::Runtime("mutex error".into()))?
            .unwrap_or_else(|| Err(ProjectError::Runtime("dependency build failed".into())))?;
        final_results.push(res);
    }
    Ok(final_results)
}

pub fn build_project(opts: &BuildOptions) -> Result<PathBuf, ProjectError> {
    let start_time = Instant::now();
    let (manifest, manifest_path) = ProjectManifest::load_with_package(
        &opts.project_dir,
        opts.package.as_deref(),
    )?;

    if let ProjectManifest::Workspace(ref ws) = manifest {
        let members = ws.find_all_members(&opts.project_dir)?;
        if members.is_empty() {
            println!("No members found in workspace {}", opts.project_dir.display());
            return Ok(opts.project_dir.join("yukispace.toml"));
        }
        let mut last_artifact = PathBuf::new();
        for (name, _, member_path) in members {
            let mut member_opts = opts.clone();
            let member_dir = member_path.parent().unwrap_or(&opts.project_dir).to_path_buf();
            member_opts.project_dir = member_dir;
            member_opts.package = Some(name);
            last_artifact = build_project(&member_opts)?;
        }
        return Ok(last_artifact);
    }

    let mut opts_local = opts.clone();
    if let Some(parent) = manifest_path.parent() {
        if parent != opts.project_dir && !manifest_path.ends_with("yukispace.toml") {
            opts_local.project_dir = parent.to_path_buf();
        }
    }
    let opts = &opts_local;

    manifest
        .validate_compiler_version(VERSION)
        .map_err(ProjectError::Runtime)?;

    let target = opts
        .target
        .as_deref()
        .map(TargetTriple::parse)
        .transpose()
        .map_err(|_| ProjectError::Runtime("unsupported target".to_string()))?
        .unwrap_or_else(TargetTriple::host);

    let mut profile_cfg = manifest.resolve_profile(&opts.profile);
    for (k, v) in &opts.overrides {
        profile_cfg.apply_override(k, v).map_err(ProjectError::Runtime)?;
    }

    let runtime_deps = manifest.runtime_deps(Some(target.as_str()));
    let unified = unify_dependency_features(
        manifest.features(),
        &opts.features,
        opts.all_features,
        opts.no_default_features,
        &runtime_deps,
    );
    let resolved_features: BTreeSet<String> = unified.root_features;

    let threads = resolve_pool_threads(opts.threads);
    let compiler = resolve_compiler_exe();

    setup_work_dirs(&opts.project_dir)?;
    let lock_path = opts.project_dir.join(LOCKFILE_REL_PATH);
    let mut lockfile = Lockfile::load(&lock_path)?
        .unwrap_or_else(|| Lockfile::new(target.as_str(), VERSION, &opts.profile));

    let project_name = manifest.name();
    let project_kind = manifest.output_kind();

    let dashboard = BuildDashboard::new(threads);
    dashboard.set_extra_info(opts.extra_info);
    dashboard.set_no_warns(opts.no_warns);
    dashboard.set_no_out(opts.no_out);
    dashboard.start();

    let rel_proj = format_relative_path(&opts.project_dir);
    let proj_desc = if rel_proj.as_os_str() == "." || rel_proj.as_os_str().is_empty() {
        format!("{project_name} v{}", manifest.version())
    } else {
        format!("{project_name} v{} ({})", manifest.version(), rel_proj.display())
    };
    if !opts.no_out {
        if should_color() {
            use owo_colors::OwoColorize;
            dashboard.log(&format!("{} {proj_desc}", "   Compiling".green().bold()));
        } else {
            dashboard.log(&format!("   Compiling {proj_desc}"));
        }
    }
    if opts.extra_info && !opts.no_out {
        dashboard.log(&format!(
            "     Profile: {} (opt-level: {}, lto: {}, strip: {})",
            opts.profile,
            profile_cfg.opt_level_str(),
            profile_cfg.lto_enabled(),
            profile_cfg.strip_enabled()
        ));
        dashboard.log(&format!("      Target: {}", target.as_str()));
        dashboard.log(&format!("     Threads: {threads}"));
        if !resolved_features.is_empty() {
            let feat_list: Vec<_> = resolved_features.iter().map(String::as_str).collect();
            dashboard.log(&format!("    Features: [{}]", feat_list.join(", ")));
        }
    }

    let mut link_archives = Vec::new();
    let mut link_systems = Vec::new();

    let host_workspace_path = {
        let mut curr = Some(manifest_path.as_path());
        let mut found = None;
        while let Some(p) = curr {
            let dir = if p.is_dir() { p } else { p.parent().unwrap_or(p) };
            let ws = dir.join("yukispace.toml");
            if ws.is_file() {
                found = Some(ws);
                break;
            }
            curr = dir.parent();
        }
        found
    };

    let dep_cgunits = opts.cgunits.unwrap_or(threads);
    let dep_env = DepBuildEnv {
        project_dir: &opts.project_dir,
        host_manifest: Some(manifest_path.clone()),
        host_workspace: host_workspace_path,
        target,
        app_profile: &opts.profile,
        app_profile_cfg: &profile_cfg,
        overrides: &opts.overrides,
        compiler: &compiler,
        rebuild: opts.rebuild,
        refresh_deps: opts.refresh_deps,
        no_lib_log: opts.no_lib_log,
        no_warns: !opts.lib_warns || opts.no_warns,
        threads,
        cgunits: dep_cgunits,
        dashboard: &dashboard,
    };

    let dep_entries: Vec<(String, DependencySpec)> = runtime_deps
        .iter()
        .filter(|(k, _)| *k != "std" && unified.active_deps.contains(*k))
        .map(|(k, v)| {
            let mut spec = v.clone();
            if let Some(feats) = unified.dep_features.get(k) {
                spec.features = feats.iter().cloned().collect();
            }
            (k.clone(), spec)
        })
        .collect();

    let built_deps = build_runtime_dependencies(&dep_entries, &dep_env, &lockfile, threads)?;

    let mut include_dirs = Vec::new();
    let mut macro_runners = Vec::new();
    let deps_dir = opts.project_dir.join("yuki-work/local/deps");
    if deps_dir.is_dir() {
        include_dirs.push(deps_dir);
    }

    for (built, locked_pkg) in built_deps {
        lockfile.upsert_package(locked_pkg);
        if let Some(macro_bin) = built.macro_binary {
            macro_runners.push((built.name.clone(), macro_bin));
        } else {
            if !link_archives.contains(&built.archive) {
                link_archives.push(built.archive);
            }
            for arch in built.static_archives {
                if !link_archives.contains(&arch) {
                    link_archives.push(arch);
                }
            }
            for sys in built.system_libs {
                if !link_systems.contains(&sys) {
                    link_systems.push(sys);
                }
            }
        }
        if built.source_dir.is_dir() && !include_dirs.contains(&built.source_dir) {
            include_dirs.push(built.source_dir);
        }
    }
    let _ = lockfile.save(&lock_path);

    let mut connected_libs: Vec<String> = dep_entries
        .iter()
        .map(|(k, _)| k.clone())
        .collect();

    let stdlib_included = manifest.includes_builtin("stdlib");
    let uses_std = stdlib_included && check_project_uses_std(&opts.project_dir, &manifest);
    if uses_std {
        let work_std_dir = opts.project_dir.join("yuki-work/local/stdlib");
        let needs_std_build = opts.refresh_deps || opts.rebuild || !yukino_builtin::is_stdlib_built(&work_std_dir, target);

        if opts.rebuild || opts.refresh_deps {
            let target_dir = work_std_dir.join(target.as_str());
            let stamp_file = target_dir.join(".built_hash");
            let std_archive = target_dir.join("libstd.a");
            let _ = fs::remove_file(&stamp_file);
            let _ = fs::remove_file(&std_archive);
        }

        if needs_std_build && !opts.no_lib_log && !opts.no_out {
            if should_color() {
                use owo_colors::OwoColorize;
                dashboard.log(&format!("{} std library", "   Compiling".green().bold()));
            } else {
                dashboard.log("   Compiling std library");
            }
        }

        let runner = if needs_std_build && !opts.no_lib_log && !opts.no_out {
            Some(|cmd: &mut Command| {
                dashboard.run_lib_compiler_command(cmd)
                    .map_err(|e| yukino_builtin::BuiltinError::CompilationFailed {
                        target: target.as_str().to_string(),
                        details: e.to_string(),
                    })
            })
        } else {
            None
        };

        let std_no_warns = !opts.lib_warns || opts.no_warns;
        let built_std = yukino_builtin::ensure_stdlib_with_runner(
            &work_std_dir,
            target,
            Some(&compiler),
            std_no_warns,
            Some(threads),
            Some(dep_cgunits),
            runner,
        )
        .map_err(|e| ProjectError::Runtime(format!("failed to prepare builtin stdlib: {e}")))?;
        if needs_std_build && !opts.no_lib_log && !opts.no_out {
            if should_color() {
                use owo_colors::OwoColorize;
                dashboard.log(&format!("{} std library", "    Compiled".green().bold()));
            } else {
                dashboard.log("    Compiled std library");
            }
        }
        dashboard.set_all_idle();
        if !link_archives.contains(&built_std.std_archive) {
            link_archives.push(built_std.std_archive);
        }
        if !link_archives.contains(&built_std.core_archive) {
            link_archives.push(built_std.core_archive);
        }
        if !include_dirs.contains(&built_std.stdlib_dir) {
            include_dirs.push(built_std.stdlib_dir);
        }
        if !connected_libs.contains(&"std".to_string()) {
            connected_libs.push("std".to_string());
        }
    }

    let macrosdk_included = manifest.includes_builtin("macrosdk");
    if macrosdk_included {
        let work_macrosdk_dir = opts.project_dir.join("yuki-work/local/macrosdk");
        let work_std_dir = opts.project_dir.join("yuki-work/local/stdlib");
        let needs_macrosdk_build = opts.refresh_deps || opts.rebuild || !yukino_builtin::is_macrosdk_built(&work_macrosdk_dir, target);

        if opts.rebuild || opts.refresh_deps {
            let target_dir = work_macrosdk_dir.join(target.as_str());
            let stamp_file = target_dir.join(".built_hash");
            let sdk_archive = target_dir.join("libmacrosdk.a");
            let _ = fs::remove_file(&stamp_file);
            let _ = fs::remove_file(&sdk_archive);
        }

        if needs_macrosdk_build && !opts.no_lib_log && !opts.no_out {
            if should_color() {
                use owo_colors::OwoColorize;
                dashboard.log(&format!("{} macrosdk", "   Compiling".green().bold()));
            } else {
                dashboard.log("   Compiling macrosdk");
            }
        }

        let runner = if needs_macrosdk_build && !opts.no_lib_log && !opts.no_out {
            Some(|cmd: &mut Command| {
                dashboard.run_lib_compiler_command(cmd)
                    .map_err(|e| yukino_builtin::BuiltinError::CompilationFailed {
                        target: target.as_str().to_string(),
                        details: e.to_string(),
                    })
            })
        } else {
            None
        };

        let sdk_no_warns = !opts.lib_warns || opts.no_warns;
        let built_sdk = yukino_builtin::ensure_macrosdk_with_runner(
            &work_macrosdk_dir,
            &work_std_dir,
            target,
            Some(&compiler),
            sdk_no_warns,
            Some(threads),
            Some(dep_cgunits),
            runner,
        )
        .map_err(|e| ProjectError::Runtime(format!("failed to prepare builtin macrosdk: {e}")))?;

        if needs_macrosdk_build && !opts.no_lib_log && !opts.no_out {
            if should_color() {
                use owo_colors::OwoColorize;
                dashboard.log(&format!("{} macrosdk", "    Compiled".green().bold()));
            } else {
                dashboard.log("    Compiled macrosdk");
            }
        }
        dashboard.set_all_idle();
        if !link_archives.contains(&built_sdk.sdk_archive) {
            link_archives.push(built_sdk.sdk_archive);
        }
        if !link_archives.contains(&built_sdk.core_archive) {
            link_archives.push(built_sdk.core_archive);
        }
        if !include_dirs.contains(&built_sdk.macrosdk_dir) {
            include_dirs.push(built_sdk.macrosdk_dir);
        }
        if !connected_libs.contains(&"macrosdk".to_string()) {
            connected_libs.push("macrosdk".to_string());
        }
    }

    let static_specs = manifest.static_libs(Some(target.as_str()));
    for arch in static_specs.archives {
        let resolved = if arch.is_relative() {
            opts.project_dir.join(arch)
        } else {
            arch
        };
        if resolved.is_file() {
            yukic_target::validate_binary_compatibility(&resolved, target)
                .map_err(|e| ProjectError::Runtime(e.to_string()))?;
        }
        if !link_archives.contains(&resolved) {
            link_archives.push(resolved);
        }
    }
    for obj in static_specs.objects {
        let resolved = if obj.is_relative() {
            opts.project_dir.join(obj)
        } else {
            obj
        };
        if resolved.is_file() {
            yukic_target::validate_binary_compatibility(&resolved, target)
                .map_err(|e| ProjectError::Runtime(e.to_string()))?;
        }
        if !link_archives.contains(&resolved) {
            link_archives.push(resolved);
        }
    }
    for sys in static_specs.system {
        if !link_systems.contains(&sys) {
            link_systems.push(sys);
        }
    }

    let build_deps = manifest.build_deps(Some(target.as_str()));
    let build_params = BuildDepParams {
        project_dir: &opts.project_dir,
        target,
        profile: &opts.profile,
        project_name,
        compiler: &compiler,
        dashboard: &dashboard,
        refresh_deps: opts.refresh_deps || opts.rebuild,
    };
    let mut build_script_link_archives = Vec::new();
    let mut build_script_include_dirs = Vec::new();
    let mut build_script_connected_libs = Vec::new();
    run_build_dependencies(
        &build_deps,
        &build_params,
        &mut link_archives,
        &mut link_systems,
        &mut build_script_include_dirs,
        &mut build_script_link_archives,
        &mut build_script_connected_libs,
        &mut lockfile,
    )?;

    for inc in &include_dirs {
        if !build_script_include_dirs.contains(inc) {
            build_script_include_dirs.push(inc.clone());
        }
    }

    let build_script = opts.project_dir.join("build.ykn");
    run_build_script(
        &build_script,
        &opts.project_dir,
        target,
        &opts.profile,
        project_name,
        &compiler,
        &build_script_include_dirs,
        &build_script_link_archives,
        &build_script_connected_libs,
        &mut link_archives,
        &mut link_systems,
        &dashboard,
    )?;

    let out_artifact = resolve_artifact_path(
        &opts.project_dir,
        project_name,
        manifest.namedbg(),
        project_kind,
        &opts.profile,
        target,
    );
    if let Some(parent) = out_artifact.parent() {
        fs::create_dir_all(parent).map_err(|e| ProjectError::Io(e.to_string()))?;
    }
    if opts.rebuild && out_artifact.exists() {
        let _ = fs::remove_file(&out_artifact);
    }

    let entry_file = opts.project_dir.join(manifest.entry());
    let feat_list: Vec<String> = resolved_features.iter().cloned().collect();
    let cgu_cache_dir = opts
        .project_dir
        .join("yuki-work/local/cache/cgu")
        .join(&opts.profile)
        .join(target.as_str())
        .join(project_name);
    if opts.rebuild && cgu_cache_dir.exists() {
        let _ = fs::remove_dir_all(&cgu_cache_dir);
    }
    fs::create_dir_all(&cgu_cache_dir).map_err(|e| ProjectError::Io(e.to_string()))?;

    let cgunits = opts
        .cgunits
        .or(profile_cfg.codegen_units)
        .unwrap_or(threads)
        .max(1);

    let mut yukic_cmd = build_yukic_command(&YukicCommandParams {
        compiler: &compiler,
        entry_file: &entry_file,
        out_artifact: &out_artifact,
        manifest_path: &manifest_path,
        project_kind,
        target,
        profile_cfg: &profile_cfg,
        profile_name: &opts.profile,
        threads,
        cgunits,
        features: &feat_list,
        link_archives: &link_archives,
        link_systems: &link_systems,
        connected_libs: &connected_libs,
        macro_runners: &macro_runners,
        runtime: opts.runtime.as_deref(),
        no_runtime: opts.no_runtime,
        no_warns: opts.no_warns,
        extra_info: opts.extra_info,
        include_dirs: &include_dirs,
        cache_dir: Some(&cgu_cache_dir),
    });

    dashboard.update_status(
        0,
        ThreadStatus::Compiling {
            target: project_name.to_string(),
            current: 1,
            total: 1,
            speed: None,
        },
    );

    let status = dashboard.run_compiler_command(&mut yukic_cmd)?;

    if !status.success() {
        dashboard.set_idle(0);
        dashboard.finish();
        return Err(ProjectError::Runtime("compilation failed".to_string()));
    }

    let root_checksum = compute_source_dir_checksum(
        &opts.project_dir,
        if project_kind == "app" { "yuki.toml" } else { "lib.toml" },
        &[target.as_str(), profile_cfg.opt_level_str()],
    )
    .unwrap_or_default();

    lockfile.upsert_package(LockedPackage {
        name: project_name.to_string(),
        version: manifest.version().to_string(),
        kind: project_kind.to_string(),
        source: "root".to_string(),
        checksum: root_checksum,
        commit: None,
        features: resolved_features.into_iter().collect(),
        dependencies: runtime_deps.keys().cloned().collect(),
    });
    lockfile.save(&lock_path)?;

    let elapsed = start_time.elapsed().as_secs_f32();
    let abs_artifact = std::fs::canonicalize(&out_artifact).unwrap_or_else(|_| {
        if out_artifact.is_absolute() {
            out_artifact.clone()
        } else if let Ok(cur) = std::env::current_dir() {
            cur.join(&out_artifact)
        } else {
            out_artifact.clone()
        }
    });
    let size_str = format_artifact_size(&abs_artifact);

    if opts.no_out {
        dashboard.finish();
        println!("0");
    } else {
        let (finished_line, artifact_line) = if should_color() {
            use owo_colors::OwoColorize;
            (
                format!(
                    "{} {} target(s) in {}",
                    "    Finished:".green().bold(),
                    opts.profile,
                    format!("{elapsed:.2}s").dimmed()
                ),
                format!(
                    "{} {} {}",
                    "    Artifact:".cyan().bold(),
                    abs_artifact.display(),
                    size_str.dimmed()
                ),
            )
        } else {
            (
                format!("    Finished: {} target(s) in {elapsed:.2}s", opts.profile),
                format!("    Artifact: {} {size_str}", abs_artifact.display()),
            )
        };
        dashboard.finish_with_summary(&[finished_line, artifact_line]);
    }

    if opts.run {
        if project_kind != "app" {
            return Err(ProjectError::Runtime(
                "ERROR: Cannot run a library project; use yukino test or compile an application"
                    .to_string(),
            ));
        }
        if !opts.no_out {
            if should_color() {
                use owo_colors::OwoColorize;
                println!("{} `{}`", "     Running".green().bold(), out_artifact.display());
            } else {
                println!("     Running `{}`", out_artifact.display());
            }
        }
        let mut run_cmd = Command::new(&out_artifact);
        run_cmd.args(&opts.run_args);
        let run_status = run_cmd
            .status()
            .map_err(|e| ProjectError::Io(format!("failed to execute {}: {e}", out_artifact.display())))?;
        if !run_status.success() {
            let code = run_status.code().unwrap_or(1);
            std::process::exit(code);
        }
    }

    Ok(out_artifact)
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TestOptions {
    pub filter: Option<String>,
    pub exact: bool,
    pub nocapture: bool,
    pub threads: Option<usize>,
    pub ignored: bool,
    pub include_ignored: bool,
    pub fail_fast: bool,
    pub list: bool,
    pub quiet: bool,
    pub timeout_ms: Option<u64>,
    pub rebuild: bool,
    pub features: Vec<String>,
    pub all_features: bool,
    pub no_default_features: bool,
    pub package: Option<String>,
}

pub fn test_project(
    project_dir: &Path,
    opts: &TestOptions,
) -> Result<(), ProjectError> {
    let (manifest, manifest_path) = ProjectManifest::load_with_package(project_dir, opts.package.as_deref())?;

    if let ProjectManifest::Workspace(ref ws) = manifest {
        let members = ws.find_all_members(project_dir)?;
        if members.is_empty() {
            println!("No members found in workspace {}", project_dir.display());
            return Ok(());
        }
        for (name, _, member_path) in members {
            let member_dir = member_path.parent().unwrap_or(project_dir);
            let mut member_opts = opts.clone();
            member_opts.package = Some(name);
            test_project(member_dir, &member_opts)?;
        }
        return Ok(());
    }

    let proj_dir_buf;
    let project_dir = if let Some(parent) = manifest_path.parent() {
        if parent != project_dir && !manifest_path.ends_with("yukispace.toml") {
            proj_dir_buf = parent.to_path_buf();
            &proj_dir_buf
        } else {
            project_dir
        }
    } else {
        project_dir
    };

    let compiler = resolve_compiler_exe();
    let target = TargetTriple::host();
    let name = manifest.name();

    let out_dir = project_dir
        .join("yuki-work/local/build/test")
        .join(target.as_str())
        .join(name);
    fs::create_dir_all(&out_dir).map_err(|e| ProjectError::Io(e.to_string()))?;
    let test_bin = out_dir.join(format!("test_{name}"));
    if opts.rebuild && test_bin.exists() {
        let _ = fs::remove_file(&test_bin);
    }

    let pool_threads = resolve_pool_threads(opts.threads);
    let dashboard = BuildDashboard::new(pool_threads);
    dashboard.start();

    let lock_path = project_dir.join(LOCKFILE_REL_PATH);
    let mut lockfile = Lockfile::load(&lock_path)?
        .unwrap_or_else(|| Lockfile::new(target.as_str(), VERSION, "debug"));

    let runtime_deps = manifest.runtime_deps(Some(target.as_str()));
    let unified = unify_dependency_features(
        manifest.features(),
        &opts.features,
        opts.all_features,
        opts.no_default_features,
        &runtime_deps,
    );

    let dep_entries: Vec<(String, DependencySpec)> = runtime_deps
        .iter()
        .filter(|(k, _)| *k != "std" && unified.active_deps.contains(*k))
        .map(|(k, v)| {
            let mut spec = v.clone();
            if let Some(feats) = unified.dep_features.get(k) {
                spec.features = feats.iter().cloned().collect();
            }
            (k.clone(), spec)
        })
        .collect();

    let host_workspace_path = {
        let mut curr = Some(manifest_path.as_path());
        let mut found = None;
        while let Some(p) = curr {
            let dir = if p.is_dir() { p } else { p.parent().unwrap_or(p) };
            let ws = dir.join("yukispace.toml");
            if ws.is_file() {
                found = Some(ws);
                break;
            }
            curr = dir.parent();
        }
        found
    };

    let profile_cfg = manifest.resolve_profile("debug");
    let overrides = Vec::new();
    let dep_env = DepBuildEnv {
        project_dir,
        host_manifest: Some(manifest_path.clone()),
        host_workspace: host_workspace_path,
        target,
        app_profile: "debug",
        app_profile_cfg: &profile_cfg,
        compiler: &compiler,
        overrides: &overrides,
        dashboard: &dashboard,
        rebuild: opts.rebuild,
        refresh_deps: false,
        no_lib_log: false,
        no_warns: true,
        threads: pool_threads,
        cgunits: pool_threads,
    };

    let built_deps = build_runtime_dependencies(&dep_entries, &dep_env, &lockfile, pool_threads)?;

    let mut link_archives = Vec::new();
    let mut link_systems = Vec::new();
    let mut include_dirs = Vec::new();
    let mut macro_runners = Vec::new();
    let deps_dir = project_dir.join("yuki-work/local/deps");
    if deps_dir.is_dir() {
        include_dirs.push(deps_dir);
    }

    for (built, locked_pkg) in built_deps {
        lockfile.upsert_package(locked_pkg);
        if let Some(macro_bin) = built.macro_binary {
            macro_runners.push((built.name.clone(), macro_bin));
        } else {
            if !link_archives.contains(&built.archive) {
                link_archives.push(built.archive);
            }
            for arch in built.static_archives {
                if !link_archives.contains(&arch) {
                    link_archives.push(arch);
                }
            }
            for sys in built.system_libs {
                if !link_systems.contains(&sys) {
                    link_systems.push(sys);
                }
            }
        }
        if built.source_dir.is_dir() && !include_dirs.contains(&built.source_dir) {
            include_dirs.push(built.source_dir);
        }
    }
    let _ = lockfile.save(&lock_path);

    let mut connected_libs: Vec<String> = dep_entries
        .iter()
        .map(|(k, _)| k.clone())
        .collect();

    let uses_std = manifest.includes_builtin("stdlib") && check_project_uses_std(project_dir, &manifest);
    if uses_std {
        let work_std_dir = project_dir.join("yuki-work/local/stdlib");
        let needs_std_build = opts.rebuild || !yukino_builtin::is_stdlib_built(&work_std_dir, target);

        if opts.rebuild {
            let target_dir = work_std_dir.join(target.as_str());
            let stamp_file = target_dir.join(".built_hash");
            let std_archive = target_dir.join("libstd.a");
            let _ = fs::remove_file(&stamp_file);
            let _ = fs::remove_file(&std_archive);
        }

        if needs_std_build {
            if should_color() {
                use owo_colors::OwoColorize;
                dashboard.log(&format!("{} std library", "   Compiling".green().bold()));
            } else {
                dashboard.log("   Compiling std library");
            }
        }

        let runner = if needs_std_build {
            Some(|cmd: &mut Command| {
                dashboard.run_lib_compiler_command(cmd)
                    .map_err(|e| yukino_builtin::BuiltinError::CompilationFailed {
                        target: target.as_str().to_string(),
                        details: e.to_string(),
                    })
            })
        } else {
            None
        };

        let built_std = yukino_builtin::ensure_stdlib_with_runner(
            &work_std_dir,
            target,
            Some(&compiler),
            true,
            Some(pool_threads),
            Some(pool_threads),
            runner,
        )
        .map_err(|e| ProjectError::Runtime(format!("failed to prepare builtin stdlib: {e}")))?;
        if needs_std_build {
            if should_color() {
                use owo_colors::OwoColorize;
                dashboard.log(&format!("{} std library", "    Compiled".green().bold()));
            } else {
                dashboard.log("    Compiled std library");
            }
        }
        dashboard.set_all_idle();
        if !link_archives.contains(&built_std.std_archive) {
            link_archives.push(built_std.std_archive);
        }
        if !link_archives.contains(&built_std.core_archive) {
            link_archives.push(built_std.core_archive);
        }
        if !include_dirs.contains(&built_std.stdlib_dir) {
            include_dirs.push(built_std.stdlib_dir);
        }
        if !connected_libs.contains(&"std".to_string()) {
            connected_libs.push("std".to_string());
        }
    }

    let macrosdk_included = manifest.includes_builtin("macrosdk");
    if macrosdk_included {
        let work_macrosdk_dir = project_dir.join("yuki-work/local/macrosdk");
        let work_std_dir = project_dir.join("yuki-work/local/stdlib");
        let needs_macrosdk_build = opts.rebuild || !yukino_builtin::is_macrosdk_built(&work_macrosdk_dir, target);

        if opts.rebuild {
            let target_dir = work_macrosdk_dir.join(target.as_str());
            let stamp_file = target_dir.join(".built_hash");
            let sdk_archive = target_dir.join("libmacrosdk.a");
            let _ = fs::remove_file(&stamp_file);
            let _ = fs::remove_file(&sdk_archive);
        }

        if needs_macrosdk_build {
            if should_color() {
                use owo_colors::OwoColorize;
                dashboard.log(&format!("{} macrosdk", "   Compiling".green().bold()));
            } else {
                dashboard.log("   Compiling macrosdk");
            }
        }

        let runner = if needs_macrosdk_build {
            Some(|cmd: &mut Command| {
                dashboard.run_lib_compiler_command(cmd)
                    .map_err(|e| yukino_builtin::BuiltinError::CompilationFailed {
                        target: target.as_str().to_string(),
                        details: e.to_string(),
                    })
            })
        } else {
            None
        };

        let built_sdk = yukino_builtin::ensure_macrosdk_with_runner(
            &work_macrosdk_dir,
            &work_std_dir,
            target,
            Some(&compiler),
            true,
            Some(pool_threads),
            Some(pool_threads),
            runner,
        )
        .map_err(|e| ProjectError::Runtime(format!("failed to prepare builtin macrosdk: {e}")))?;

        if needs_macrosdk_build {
            if should_color() {
                use owo_colors::OwoColorize;
                dashboard.log(&format!("{} macrosdk", "    Compiled".green().bold()));
            } else {
                dashboard.log("    Compiled macrosdk");
            }
        }
        dashboard.set_all_idle();
        if !link_archives.contains(&built_sdk.sdk_archive) {
            link_archives.push(built_sdk.sdk_archive);
        }
        if !link_archives.contains(&built_sdk.core_archive) {
            link_archives.push(built_sdk.core_archive);
        }
        if !include_dirs.contains(&built_sdk.macrosdk_dir) {
            include_dirs.push(built_sdk.macrosdk_dir);
        }
        if !connected_libs.contains(&"macrosdk".to_string()) {
            connected_libs.push("macrosdk".to_string());
        }
    }

    let static_specs = manifest.static_libs(Some(target.as_str()));
    for arch in static_specs.archives {
        let resolved = if arch.is_relative() {
            project_dir.join(arch)
        } else {
            arch
        };
        if !link_archives.contains(&resolved) {
            link_archives.push(resolved);
        }
    }
    for obj in static_specs.objects {
        let resolved = if obj.is_relative() {
            project_dir.join(obj)
        } else {
            obj
        };
        if !link_archives.contains(&resolved) {
            link_archives.push(resolved);
        }
    }
    for sys in static_specs.system {
        if !link_systems.contains(&sys) {
            link_systems.push(sys);
        }
    }

    let entry_file = project_dir.join(manifest.entry());
    let mut compile_cmd = Command::new(&compiler);
    compile_cmd.arg("--test");
    compile_cmd.arg("--entry").arg(&entry_file);
    compile_cmd.arg("-o").arg(&test_bin);
    compile_cmd.arg("--target").arg(target.as_str());
    compile_cmd.arg("--manifest").arg(&manifest_path);
    for inc in &include_dirs {
        compile_cmd.arg("--include").arg(inc);
    }
    for arch in &link_archives {
        compile_cmd.arg("--link-archive").arg(arch);
    }
    for sys in &link_systems {
        compile_cmd.arg("--link-system").arg(sys);
    }
    for lib in &connected_libs {
        compile_cmd.arg("--connected-lib").arg(lib);
    }
    if !unified.root_features.is_empty() {
        let feat_list: Vec<_> = unified.root_features.iter().map(String::as_str).collect();
        compile_cmd.arg("--features").arg(feat_list.join(","));
    }
    for (name, path) in &macro_runners {
        compile_cmd.arg("--macro-runner").arg(format!("{}={}", name, path.display()));
    }

    if should_color() {
        use owo_colors::OwoColorize;
        dashboard.log(&format!("{} test binary: {name}", "   Compiling".green().bold()));
    } else {
        dashboard.log(&format!("   Compiling test binary: {name}"));
    }
    dashboard.update_status(
        0,
        ThreadStatus::Compiling {
            target: format!("test_{name}"),
            current: 1,
            total: 1,
            speed: None,
        },
    );

    let status = dashboard.run_compiler_command(&mut compile_cmd)?;
    dashboard.set_idle(0);
    dashboard.finish();

    if !status.success() {
        return Err(ProjectError::Runtime("test compilation failed".to_string()));
    }

    let test_sec = manifest.test_section();
    let effective_threads = opts.threads.or_else(|| {
        test_sec.threads.as_ref().and_then(|v| match v {
            toml::Value::Integer(i) => usize::try_from(*i).ok(),
            _ => None,
        })
    });
    let effective_nocapture = opts.nocapture || test_sec.capture_output == Some(false);
    let effective_fail_fast = opts.fail_fast || test_sec.fail_fast == Some(true);
    let effective_timeout = opts.timeout_ms.or(test_sec.timeout_ms);

    if should_color() {
        use owo_colors::OwoColorize;
        println!("{} tests in `{}`", "     Running".green().bold(), test_bin.display());
    } else {
        println!("     Running tests in `{}`", test_bin.display());
    }
    let mut run_cmd = Command::new(&test_bin);
    if let Some(ref f) = opts.filter {
        run_cmd.arg(f);
    }
    if opts.exact {
        run_cmd.arg("--exact");
    }
    if let Some(th) = effective_threads {
        run_cmd.arg("--threads").arg(th.to_string());
    }
    if effective_nocapture {
        run_cmd.arg("--nocapture");
    }
    if opts.ignored {
        run_cmd.arg("--ignored");
    }
    if opts.include_ignored {
        run_cmd.arg("--include-ignored");
    }
    if effective_fail_fast {
        run_cmd.arg("--fail-fast");
    }
    if opts.list {
        run_cmd.arg("--list");
    }
    if opts.quiet {
        run_cmd.arg("--quiet");
    }
    if let Some(to) = effective_timeout {
        run_cmd.arg("--timeout").arg(to.to_string());
    }

    let run_status = run_cmd.status().map_err(|e| ProjectError::Io(e.to_string()))?;
    if !run_status.success() {
        return Err(ProjectError::Runtime("tests failed".to_string()));
    }

    Ok(())
}

pub fn format_relative_path(path: &Path) -> PathBuf {
    let Ok(cur) = std::env::current_dir() else {
        return path.to_path_buf();
    };
    if let Ok(stripped) = path.strip_prefix(&cur) {
        return if stripped.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            stripped.to_path_buf()
        };
    }
    let (Ok(can_path), Ok(can_cur)) = (path.canonicalize(), cur.canonicalize()) else {
        return path.to_path_buf();
    };
    if let Ok(stripped) = can_path.strip_prefix(&can_cur) {
        if stripped.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            stripped.to_path_buf()
        }
    } else {
        path.to_path_buf()
    }
}

#[allow(clippy::cast_precision_loss)]
pub fn format_artifact_size(path: &Path) -> String {
    let bytes = std::fs::metadata(path).map_or(0, |m| m.len());
    if bytes >= 1024 * 1024 {
        let mb = bytes as f64 / (1024.0 * 1024.0);
        format!("{mb:.2} MB")
    } else {
        let kb = bytes as f64 / 1024.0;
        format!("{kb:.2} KB")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_artifact_path_app() {
        let dir = Path::new("/workspace");
        let path = resolve_artifact_path(
            dir,
            "my_app",
            Some("my_app_debug"),
            "app",
            "debug",
            TargetTriple::X86_64UnknownLinuxGnu,
        );
        assert_eq!(
            path,
            PathBuf::from("/workspace/yuki-work/local/build/debug/x86_64-unknown-linux-gnu/my_app/my_app_debug")
        );

        let release_path = resolve_artifact_path(
            dir,
            "my_app",
            Some("my_app_debug"),
            "app",
            "release",
            TargetTriple::X86_64UnknownLinuxGnu,
        );
        assert_eq!(
            release_path,
            PathBuf::from("/workspace/yuki-work/local/build/release/x86_64-unknown-linux-gnu/my_app/my_app")
        );
    }

    #[test]
    fn test_resolve_artifact_path_static_and_shared_lib() {
        let dir = Path::new("/workspace");
        let static_path = resolve_artifact_path(
            dir,
            "my_lib",
            None,
            "static",
            "release",
            TargetTriple::X86_64UnknownLinuxGnu,
        );
        assert_eq!(
            static_path,
            PathBuf::from("/workspace/yuki-work/local/build/release/x86_64-unknown-linux-gnu/my_lib/libmy_lib.a")
        );

        let shared_path = resolve_artifact_path(
            dir,
            "my_lib",
            None,
            "shared",
            "release",
            TargetTriple::X86_64UnknownLinuxGnu,
        );
        assert_eq!(
            shared_path,
            PathBuf::from("/workspace/yuki-work/local/build/release/x86_64-unknown-linux-gnu/my_lib/libmy_lib.so")
        );
    }

    #[test]
    fn test_clean_project_removes_local_dir() {
        let tmp = std::env::temp_dir().join(format!("test_clean_{}", std::process::id()));
        let local_dir = tmp.join("yuki-work/local");
        let lock_file = tmp.join("yuki-work/common/yuki.lock");
        fs::create_dir_all(&local_dir).expect("create local");
        fs::create_dir_all(lock_file.parent().unwrap()).expect("create common");
        fs::write(&lock_file, "version = 1\n").expect("write lock");

        clean_project(&tmp, false).expect("clean local");
        assert!(!local_dir.exists());
        assert!(lock_file.is_file());

        clean_project(&tmp, true).expect("clean all");
        assert!(!lock_file.exists());

        let _ = fs::remove_dir_all(tmp);
    }

    #[test]
    fn test_source_imports_std() {
        assert!(source_imports_std("load std.io: print"));
        assert!(source_imports_std("load std.fs.file: File;"));
        assert!(source_imports_std("load std.io"));
        assert!(source_imports_std("load std.collections: map"));
        assert!(!source_imports_std("load .calc"));
        assert!(!source_imports_std("load local_mod: foo"));
        assert!(!source_imports_std("// load std.io: print\nfun main() {}"));
        assert!(!source_imports_std("/* load std.io */ fun main() {}"));
        assert!(!source_imports_std("fun main() { set s = \"load std.io: print\"; }"));
    }
}
