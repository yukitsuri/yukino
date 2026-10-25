#![deny(unsafe_code)]

pub mod archive;
pub mod cgu_cache;
pub mod cli;
pub mod embedded_runtime;
pub mod emitter;
pub mod error;
pub mod executable;
pub mod header;
pub mod linker_common;
pub mod lint;
pub mod pipeline;
pub mod shared;
pub mod targets_view;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::io::IsTerminal;
use std::str::FromStr;

pub use archive::{create_static_archive, list_archive_members};
pub use cgu_cache::{CguCacheEntry, CguCacheManifest, MANIFEST_FILENAME};
pub use cli::{Args, DriverSubcommand, EmitKind, OutputType};
pub use targets_view::targets_output;
pub use embedded_runtime::{
    ensure_macrosdk_for, ensure_runtime_archive, ensure_runtime_archive_for, ensure_stdlib_for,
    ensure_stdlib_source_for, resolve_macrosdk_dir_for, resolve_runtime_dir, resolve_runtime_dir_for,
    resolve_stdlib_dir_for, unpack_embedded_runtime, unpack_embedded_runtime_at,
};
pub use emitter::{
    create_target_machine_for_driver, emit_asm_file, emit_cgus_parallel, emit_module_to_file,
    emit_object_file, format_cgu_summary, format_emit_progress, format_source_action, format_source_cgu,
    reloc_mode_for, EmitProgressEvent,
};
pub use error::DriverError;
pub use executable::{build_executable_linker_args, link_executable};
pub use header::{
    generate_c_header_from_ast, generate_c_header_from_llvm, resolve_header_path,
};
pub use linker_common::{
    collect_system_search_dirs, find_default_runtime_archive, resolve_dynamic_linker,
};
pub use pipeline::run_compiler_driver;
pub use shared::{build_shared_linker_args, link_shared_library};
use yukic_opt::OptLevel;
use yukic_target::TargetTriple;
use yukic_target::spec::OperatingSystem;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BUILD_DATE: &str = env!("YUKIC_BUILD_DATE");
pub const YUKINO_NAME: &str = env!("YUKINO_NAME");
pub const YUKIC_NAME: &str = env!("YUKIC_NAME");
pub const YUKINO_DIR: &str = env!("YUKINO_DIR");
pub const YUKINO_CFG_DIR: &str = env!("YUKINO_CFG_DIR");

pub fn should_color() -> bool {
    if std::env::var_os("CLICOLOR_FORCE").is_some_and(|value| value != "0") {
        return true;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if std::env::var("TERM").is_ok_and(|term| term == "dumb") {
        return false;
    }
    std::io::stdout().is_terminal()
}

pub fn should_color_stderr() -> bool {
    if std::env::var_os("CLICOLOR_FORCE").is_some_and(|value| value != "0") {
        return true;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if std::env::var("TERM").is_ok_and(|term| term == "dumb") {
        return false;
    }
    std::io::stderr().is_terminal()
}

pub fn format_bug_report_notice(colored: bool) -> String {
    if colored {
        use owo_colors::OwoColorize;
        format!(
            "\n\n{} {}",
            "This might be a compiler bug. Please report it here:".blue(),
            "https://github.com/yukitsuri/yukino/issues".blue().underline()
        )
    } else {
        "\n\nThis might be a compiler bug. Please report it here: https://github.com/yukitsuri/yukino/issues".to_string()
    }
}

pub fn version_output(colored: bool) -> String {
    if colored {
        use owo_colors::OwoColorize;
        format!(
            "Yukino compiler v{VERSION}\nUse {} or {} for more",
            YUKIC_NAME.cyan(),
            format!("{YUKIC_NAME} --help").cyan()
        )
    } else {
        format!("Yukino compiler v{VERSION}\nUse {YUKIC_NAME} or {YUKIC_NAME} --help for more")
    }
}

pub fn banner_output(colored: bool) -> String {
    if colored {
        use owo_colors::OwoColorize;
        format!(
            "{} v{VERSION}\n{}: {BUILD_DATE}\n{}: Apache 2.0 2026\n{}: {}",
            "Yukino language compiler".cyan().bold(),
            "Built".cyan(),
            "License".cyan(),
            "Source code".cyan(),
            "https://github.com/yukitsuri/yukino".underline()
        )
    } else {
        format!(
            "Yukino language compiler v{VERSION}\nBuilt: {BUILD_DATE}\nLicense: Apache 2.0 2026\nSource code: https://github.com/yukitsuri/yukino"
        )
    }
}

pub fn format_relative_path(path: &Path) -> PathBuf {
    let Ok(cur) = std::env::current_dir() else {
        return path.to_path_buf();
    };
    if let Ok(stripped) = path.strip_prefix(&cur) {
        return if stripped.as_os_str().is_empty() {
            PathBuf::from(".")
        } else { stripped.to_path_buf() };
    }
    let (Ok(can_path), Ok(can_cur)) = (path.canonicalize(), cur.canonicalize()) else {
        return path.to_path_buf();
    };
    if let Ok(stripped) = can_path.strip_prefix(&can_cur) {
        if stripped.as_os_str().is_empty() { PathBuf::from(".")
        } else { stripped.to_path_buf() }
    } else { path.to_path_buf() }
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
mod cli_output_tests {
    use super::*;

    #[test]
    fn test_version_output() {
        assert_eq!(
            version_output(false),
            format!("Yukino compiler v{VERSION}\nUse yukic or yukic --help for more")
        );
    }

    #[test]
    fn test_banner_output() {
        assert_eq!(
            banner_output(false),
            format!(
                "Yukino language compiler v{VERSION}\nBuilt: {BUILD_DATE}\nLicense: Apache 2.0 2026\nSource code: https://github.com/yukitsuri/yukino"
            )
        );
    }

    #[test]
    fn test_colored_output() {
        let version = version_output(true);
        assert!(version.contains("\x1b[36myukic\x1b[39m"));
        assert!(version.contains("\x1b[36myukic --help\x1b[39m"));

        let banner = banner_output(true);
        assert!(banner.contains("https://github.com/yukitsuri/yukino"));
        assert!(banner.contains("\x1b[4m"));
    }
}

pub fn default_threads() -> usize {
    1
}

#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct DriverConfig {
    pub input_file: String,
    pub output_file: Option<String>,
    pub target: Option<String>,
    pub features: Vec<String>,
    pub codegen_units: usize,
    pub threads: usize,
    pub emit: EmitKind,
    pub opt_level: OptLevel,
    pub verbose: bool,
    pub no_warns: bool,
    pub extra_info: bool,
    pub test_mode: bool,
    pub emit_header: Option<PathBuf>,
    pub output_type: OutputType,
    pub strip_unused: bool,
    pub lto: bool,
    pub strip: bool,
    pub include_dirs: Vec<PathBuf>,
    pub runtime: Option<PathBuf>,
    pub no_runtime: bool,
    pub link_archives: Vec<PathBuf>,
    pub link_statics: Vec<PathBuf>,
    pub link_systems: Vec<String>,
    pub linker: Option<String>,
    pub inspect_attributes: Option<PathBuf>,
    pub debug: bool,
    pub release: bool,
    pub profile_name: Option<String>,
    pub manifest_path: Option<PathBuf>,
    pub workspace_manifest_path: Option<PathBuf>,
    pub host_manifest_path: Option<PathBuf>,
    pub host_workspace_path: Option<PathBuf>,
    pub connected_libs: Vec<String>,
    pub cache_dir: Option<PathBuf>,
    pub panic_strategy: String,
    pub function_sections: bool,
    pub macro_runners: HashMap<String, PathBuf>,
}

impl DriverConfig {
    pub fn new(input_file: String) -> Self {
        Self {
            input_file,
            output_file: None,
            target: None,
            features: Vec::new(),
            codegen_units: 1,
            threads: 1,
            emit: EmitKind::Bin,
            opt_level: OptLevel::O0,
            verbose: false,
            no_warns: false,
            extra_info: false,
            test_mode: false,
            emit_header: None,
            output_type: OutputType::App,
            strip_unused: false,
            lto: false,
            strip: false,
            include_dirs: Vec::new(),
            runtime: None,
            no_runtime: false,
            link_archives: Vec::new(),
            link_statics: Vec::new(),
            link_systems: Vec::new(),
            linker: None,
            inspect_attributes: None,
            debug: false,
            release: false,
            profile_name: None,
            manifest_path: None,
            workspace_manifest_path: None,
            host_manifest_path: None,
            host_workspace_path: None,
            connected_libs: Vec::new(),
            cache_dir: None,
            panic_strategy: "unwind".to_string(),
            function_sections: false,
            macro_runners: HashMap::new(),
        }
    }

    pub fn from_args(args: Args) -> Option<Self> {
        let entry_path = args.resolve_entry()?;
        let threads = args.resolve_threads();
        let codegen_units = args.resolve_codegen_units();
        let function_sections = args.resolve_function_sections().unwrap_or(false);
        let macro_runners = args.parse_macro_runners();
        let runtime = args.runtime;
        let no_runtime = args.no_runtime;
        let mut link_archives = args.link_archive;
        if let Some(runtime_path) = &runtime {
            link_archives.insert(0, runtime_path.clone());
        }

        Some(Self {
            input_file: entry_path.to_string_lossy().to_string(),
            output_file: args.out.map(|p| p.to_string_lossy().to_string()),
            target: args.target,
            features: args.features,
            codegen_units,
            threads,
            emit: args.emit,
            opt_level: args.opt_level,
            verbose: args.verbose,
            no_warns: args.no_warns,
            extra_info: args.extra_info,
            test_mode: args.test,
            emit_header: args.emit_header,
            output_type: args.output_type,
            strip_unused: args.strip_unused,
            lto: args.lto,
            strip: args.strip,
            include_dirs: args.include,
            runtime,
            no_runtime,
            link_archives,
            link_statics: args.link_static,
            link_systems: args.link_system,
            linker: args.linker,
            inspect_attributes: args.inspect_attributes,
            debug: args.debug,
            release: args.release,
            profile_name: args.profile_name,
            manifest_path: args.manifest,
            workspace_manifest_path: args.workspace_manifest,
            host_manifest_path: args.host_manifest,
            host_workspace_path: args.host_workspace,
            connected_libs: args.connected_lib,
            cache_dir: args.cache_dir,
            panic_strategy: args.panic_strategy,
            function_sections,
            macro_runners,
        })
    }

    #[must_use]
    pub fn with_function_sections(mut self, enabled: bool) -> Self {
        self.function_sections = enabled;
        self
    }

    #[must_use]
    pub fn with_panic_strategy(mut self, strategy: impl Into<String>) -> Self {
        self.panic_strategy = strategy.into();
        self
    }

    #[must_use]
    pub fn with_cache_dir(mut self, dir: PathBuf) -> Self {
        self.cache_dir = Some(dir);
        self
    }

    #[must_use]
    pub fn with_codegen_units(mut self, cgu: usize) -> Self {
        self.codegen_units = cgu.max(1);
        self
    }

    #[must_use]
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.threads = threads.max(1);
        self
    }

    #[must_use]
    pub fn with_target(mut self, target: Option<String>) -> Self {
        self.target = target;
        self
    }

    #[must_use]
    pub fn with_features(mut self, features: impl IntoIterator<Item = String>) -> Self {
        self.features = features.into_iter().collect();
        self
    }

    #[must_use]
    pub fn with_emit(mut self, emit: EmitKind) -> Self {
        self.emit = emit;
        self
    }

    #[must_use]
    pub fn with_opt_level(mut self, opt_level: OptLevel) -> Self {
        self.opt_level = opt_level;
        self
    }

    #[must_use]
    pub fn with_output_type(mut self, output_type: OutputType) -> Self {
        self.output_type = output_type;
        self
    }

    #[must_use]
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    #[must_use]
    pub fn with_test_mode(mut self, test_mode: bool) -> Self {
        self.test_mode = test_mode;
        self
    }

    #[must_use]
    pub fn with_emit_header(mut self, emit_header: Option<PathBuf>) -> Self {
        self.emit_header = emit_header;
        self
    }

    pub fn resolve_target(&self) -> Result<TargetTriple, DriverError> {
        match &self.target {
            Some(t) if t == "host" => Ok(TargetTriple::host()),
            Some(t) => TargetTriple::from_str(t).map_err(|e| DriverError::Target(e.to_string())),
            None => Ok(TargetTriple::host()),
        }
    }

    pub fn resolve_output_path(&self, target: TargetTriple) -> PathBuf {
        if let Some(out) = &self.output_file {
            return PathBuf::from(out);
        }

        let input_path = Path::new(&self.input_file);
        let stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("app");

        match self.emit {
            EmitKind::LlvmIr => PathBuf::from(format!("{stem}.ll")),
            EmitKind::Asm => PathBuf::from(format!("{stem}.s")),
            EmitKind::Obj => PathBuf::from(format!("{stem}.o")),
            EmitKind::Bin => match self.output_type {
                OutputType::App => {
                    if target.spec().os == OperatingSystem::Windows {
                        PathBuf::from(format!("{stem}.exe"))
                    } else {
                        PathBuf::from(stem)
                    }
                }
                OutputType::Static => PathBuf::from(format!("lib{stem}.a")),
                OutputType::Shared => {
                    if target.spec().os == OperatingSystem::Windows {
                        PathBuf::from(format!("{stem}.dll"))
                    } else {
                        PathBuf::from(format!("lib{stem}.so"))
                    }
                }
            },
        }
    }

    pub fn assemble_static_archive(
        &self,
        objects: &[PathBuf],
        target: TargetTriple,
    ) -> Result<PathBuf, DriverError> {
        let out_path = self.resolve_output_path(target);
        create_static_archive(&out_path, objects, None)?;
        Ok(out_path)
    }

    pub fn assemble_shared_library(
        &self,
        objects: &[PathBuf],
        target: TargetTriple,
    ) -> Result<PathBuf, DriverError> {
        let out_path = self.resolve_output_path(target);
        link_shared_library(&out_path, objects, target, self)?;
        Ok(out_path)
    }

    pub fn assemble_executable(
        &self,
        objects: &[PathBuf],
        target: TargetTriple,
    ) -> Result<PathBuf, DriverError> {
        let out_path = self.resolve_output_path(target);
        link_executable(&out_path, objects, target, self)?;
        Ok(out_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_config_defaults() {
        let config = DriverConfig::new("src/main.ykn".to_string());
        assert_eq!(config.input_file, "src/main.ykn");
        assert_eq!(config.codegen_units, 1);
        assert_eq!(config.threads, 1);
        assert_eq!(config.emit, EmitKind::Bin);
        assert_eq!(config.opt_level, OptLevel::O0);
        assert!(!config.verbose);
        assert!(!config.test_mode);
        assert_eq!(config.emit_header, None);
    }

    #[test]
    fn test_driver_config_builder() {
        let config = DriverConfig::new("src/main.ykn".to_string())
            .with_codegen_units(8)
            .with_threads(4)
            .with_target(Some("x86_64-unknown-linux-gnu".to_string()))
            .with_features(["feat1".to_string(), "feat2".to_string()])
            .with_emit(EmitKind::LlvmIr)
            .with_opt_level(OptLevel::O3)
            .with_output_type(OutputType::Static)
            .with_verbose(true)
            .with_test_mode(true)
            .with_emit_header(Some(PathBuf::from("include/main.h")));

        assert_eq!(config.codegen_units, 8);
        assert_eq!(config.threads, 4);
        assert_eq!(config.target, Some("x86_64-unknown-linux-gnu".to_string()));
        assert_eq!(config.features, vec!["feat1", "feat2"]);
        assert_eq!(config.emit, EmitKind::LlvmIr);
        assert_eq!(config.opt_level, OptLevel::O3);
        assert_eq!(config.output_type, OutputType::Static);
        assert!(config.verbose);
        assert!(config.test_mode);
        assert_eq!(config.emit_header, Some(PathBuf::from("include/main.h")));
    }

    #[test]
    fn test_driver_config_resolve_output_path() {
        let config_app = DriverConfig::new("src/my_app.ykn".into());
        let target_linux = TargetTriple::X86_64UnknownLinuxGnu;
        let target_win = TargetTriple::X86_64PcWindowsGnu;

        assert_eq!(
            config_app.resolve_output_path(target_linux),
            PathBuf::from("my_app")
        );
        assert_eq!(
            config_app.resolve_output_path(target_win),
            PathBuf::from("my_app.exe")
        );

        let config_static =
            DriverConfig::new("src/math.ykn".into()).with_output_type(OutputType::Static);
        assert_eq!(
            config_static.resolve_output_path(target_linux),
            PathBuf::from("libmath.a")
        );

        let config_shared =
            DriverConfig::new("src/math.ykn".into()).with_output_type(OutputType::Shared);
        assert_eq!(
            config_shared.resolve_output_path(target_linux),
            PathBuf::from("libmath.so")
        );
        assert_eq!(
            config_shared.resolve_output_path(target_win),
            PathBuf::from("math.dll")
        );
    }

    #[test]
    fn test_driver_config_resolve_target() {
        let config = DriverConfig::new("src/main.ykn".into())
            .with_target(Some("x86_64-unknown-linux-musl".into()));
        let target = config.resolve_target().expect("valid target");
        assert_eq!(target, TargetTriple::X86_64UnknownLinuxMusl);

        let config_host = DriverConfig::new("src/main.ykn".into()).with_target(Some("host".into()));
        assert_eq!(
            config_host.resolve_target().expect("host target"),
            TargetTriple::host()
        );

        let config_invalid =
            DriverConfig::new("src/main.ykn".into()).with_target(Some("unknown-os-triple".into()));
        assert!(config_invalid.resolve_target().is_err());
    }

    #[test]
    fn test_default_threads() {
        assert_eq!(default_threads(), 1);
    }

    #[test]
    fn test_format_bug_report_notice_uncolored() {
        let plain = format_bug_report_notice(false);
        assert_eq!(
            plain,
            "\n\nThis might be a compiler bug. Please report it here: https://github.com/yukitsuri/yukino/issues"
        );
    }

    #[test]
    fn test_format_bug_report_notice_colored() {
        let colored = format_bug_report_notice(true);
        assert!(colored.contains("This might be a compiler bug. Please report it here:"));
        assert!(colored.contains("https://github.com/yukitsuri/yukino/issues"));
        assert!(colored.contains("\x1b[34m"));
        assert!(colored.contains("\x1b[4m"));
    }
}

