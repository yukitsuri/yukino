#![deny(unsafe_code)]

use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use clap::{Parser, Subcommand, ValueEnum};
use yukic_opt::OptLevel;

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum DriverSubcommand {
    #[command(name = "targets", about = "List supported target triples")]
    Targets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
#[value(rename_all = "kebab-case")]
pub enum EmitKind {
    #[default]
    #[value(name = "bin")]
    Bin,
    #[value(name = "llvm-ir")]
    LlvmIr,
    #[value(name = "asm")]
    Asm,
    #[value(name = "obj")]
    Obj,
}

impl EmitKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bin => "bin",
            Self::LlvmIr => "llvm-ir",
            Self::Asm => "asm",
            Self::Obj => "obj",
        }
    }
}

impl fmt::Display for EmitKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for EmitKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "bin" => Ok(Self::Bin),
            "llvm-ir" | "ir" | "ll" => Ok(Self::LlvmIr),
            "asm" | "s" => Ok(Self::Asm),
            "obj" | "o" => Ok(Self::Obj),
            other => Err(format!(
                "unknown emit format '{other}', expected bin, llvm-ir, asm, or obj"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
#[value(rename_all = "kebab-case")]
pub enum OutputType {
    #[default]
    #[value(name = "app")]
    App,
    #[value(name = "static")]
    Static,
    #[value(name = "shared")]
    Shared,
}

impl OutputType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Static => "static",
            Self::Shared => "shared",
        }
    }
}

impl fmt::Display for OutputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for OutputType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "app" => Ok(Self::App),
            "static" => Ok(Self::Static),
            "shared" => Ok(Self::Shared),
            other => Err(format!(
                "unknown output type '{other}', expected app, static, or shared"
            )),
        }
    }
}

#[derive(Parser, Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
#[command(
    name = crate::YUKIC_NAME,
    about = "Yukino compiler driver",
    disable_version_flag = true
)]
pub struct Args {
    #[arg(
        short = 'v',
        long = "version",
        action = clap::ArgAction::SetTrue,
        help = "Print version information"
    )]
    pub version: bool,

    #[command(subcommand)]
    pub command: Option<DriverSubcommand>,

    #[arg(value_name = "INPUT", help = "Input source file (.ykn)")]
    pub input: Option<PathBuf>,

    #[arg(long = "entry", value_name = "PATH", help = "Entry source file path")]
    pub entry: Option<PathBuf>,

    #[arg(
        short = 'o',
        long = "out",
        alias = "output",
        value_name = "PATH",
        help = "Output artifact path"
    )]
    pub out: Option<PathBuf>,

    #[arg(
        long = "target",
        value_name = "TRIPLE",
        help = "Target triple (e.g. x86_64-unknown-linux-gnu or host)"
    )]
    pub target: Option<String>,

    #[arg(
        short = 'F',
        long = "features",
        value_name = "FEATURES",
        value_delimiter = ',',
        action = clap::ArgAction::Append,
        help = "Enable conditional compilation features (comma-separated or repeatable -F)"
    )]
    pub features: Vec<String>,

    #[arg(
        long = "emit",
        value_name = "KIND",
        default_value = "bin",
        help = "Output artifact format (bin, llvm-ir, asm, obj)"
    )]
    pub emit: EmitKind,

    #[arg(
        short = 'O',
        long = "opt-level",
        default_value = "0",
        value_name = "LEVEL",
        help = "Optimization level (0, 1, 2, 3, s, z)"
    )]
    pub opt_level: OptLevel,

    #[arg(long = "verbose", help = "Enable verbose compiler output")]
    pub verbose: bool,

    #[arg(long = "no-warns", help = "Disable compiler warnings")]
    pub no_warns: bool,

    #[arg(long = "extra-info", help = "Display extended compilation information")]
    pub extra_info: bool,

    #[arg(long = "test", help = "Build test harness for #[test] functions")]
    pub test: bool,

    #[arg(
        long = "emit-header",
        value_name = "PATH",
        help = "Path to emit C header file (.h)"
    )]
    pub emit_header: Option<PathBuf>,

    #[arg(
        long = "cgunits",
        alias = "codegen-units",
        value_name = "N",
        help = "Number of parallel LLVM codegen units (CGU)"
    )]
    pub codegen_units: Option<usize>,

    #[arg(
        long = "cache-dir",
        value_name = "DIR",
        help = "Directory for caching codegen unit (CGU) object files and manifest"
    )]
    pub cache_dir: Option<PathBuf>,

    #[arg(
        short = 'j',
        long = "threads",
        value_name = "N",
        help = "Number of worker threads for parallel compilation"
    )]
    pub threads: Option<usize>,

    #[arg(long = "function-sections", help = "Place each function in its own section")]
    pub function_sections: bool,

    #[arg(long = "no-function-sections", help = "Do not place each function in its own section")]
    pub no_function_sections: bool,

    #[arg(
        long = "output-type",
        value_name = "TYPE",
        default_value = "app",
        help = "Type of output artifact (app, static, shared)"
    )]
    pub output_type: OutputType,

    #[arg(long = "strip-unused", help = "Enable dead code elimination (DCE)")]
    pub strip_unused: bool,

    #[arg(long = "lto", help = "Enable link-time optimization")]
    pub lto: bool,

    #[arg(long = "strip", help = "Strip symbols from output binary")]
    pub strip: bool,

    #[arg(
        long = "include",
        value_name = "DIR",
        help = "Directory for generated modules"
    )]
    pub include: Vec<PathBuf>,

    #[arg(
        long = "link-archive",
        value_name = "PATH",
        help = "Static archive dependency (.a)"
    )]
    pub link_archive: Vec<PathBuf>,

    #[arg(long = "runtime", value_name = "PATH", help = "Runtime archive (.a)")]
    pub runtime: Option<PathBuf>,

    #[arg(long = "no-runtime", help = "Do not compile or link runtime")]
    pub no_runtime: bool,

    #[arg(
        long = "link-static",
        value_name = "PATH",
        help = "User static C ABI library (.a/.o)"
    )]
    pub link_static: Vec<PathBuf>,

    #[arg(
        long = "link-system",
        short = 'l',
        value_name = "LIB",
        help = "System library name to link (-l<name>)"
    )]
    pub link_system: Vec<String>,

    #[arg(
        long = "linker",
        value_name = "LINKER",
        help = "Linker flavor or executable (lld, mold, system)"
    )]
    pub linker: Option<String>,

    #[arg(
        long = "inspect-attributes",
        value_name = "PATH",
        help = "Dump custom attribute metadata to JSON"
    )]
    pub inspect_attributes: Option<PathBuf>,

    #[arg(long = "debug", help = "Build with debug information")]
    pub debug: bool,

    #[arg(long = "release", help = "Build with release settings")]
    pub release: bool,

    #[arg(
        long = "profile-name",
        value_name = "NAME",
        help = "Name of the build profile (e.g. debug, release)"
    )]
    pub profile_name: Option<String>,

    #[arg(
        long = "manifest",
        value_name = "PATH",
        help = "Path to project manifest (yuki.toml or lib.toml)"
    )]
    pub manifest: Option<PathBuf>,

    #[arg(
        long = "workspace-manifest",
        value_name = "PATH",
        help = "Path to workspace manifest (yukispace.toml)"
    )]
    pub workspace_manifest: Option<PathBuf>,

    #[arg(
        long = "host-manifest",
        value_name = "PATH",
        help = "Path to host application manifest (yuki.toml or lib.toml)"
    )]
    pub host_manifest: Option<PathBuf>,

    #[arg(
        long = "host-workspace",
        value_name = "PATH",
        help = "Path to host application workspace (yukispace.toml)"
    )]
    pub host_workspace: Option<PathBuf>,

    #[arg(
        long = "connected-lib",
        value_name = "NAME",
        help = "Explicitly declare connected library name"
    )]
    pub connected_lib: Vec<String>,

    #[arg(
        long = "panic-strategy",
        value_name = "STRATEGY",
        value_parser = ["unwind", "abort"],
        default_value = "unwind",
        help = "Panic strategy: unwind or abort"
    )]
    pub panic_strategy: String,

    #[arg(
        long = "macro-runner",
        value_name = "NAME=PATH",
        action = clap::ArgAction::Append,
        help = "Register an external procedural macro runner process (<name>=<path>)"
    )]
    pub macro_runner: Vec<String>,

    #[arg(
        long = "macro-timeout",
        alias = "macrotimeout",
        value_name = "SECS",
        help = "Procedural macro execution timeout in seconds"
    )]
    pub macro_timeout: Option<u64>,
}

impl Args {
    pub fn parse_macro_runners(&self) -> std::collections::HashMap<String, PathBuf> {
        let mut map = std::collections::HashMap::new();
        for item in &self.macro_runner {
            if let Some((name, path)) = item.split_once('=') {
                map.insert(name.trim().to_string(), PathBuf::from(path.trim()));
            }
        }
        map
    }
    pub fn resolve_entry(&self) -> Option<PathBuf> {
        self.entry.clone().or_else(|| self.input.clone())
    }

    pub fn resolve_output(&self) -> Option<PathBuf> {
        self.out.clone()
    }

    pub fn resolve_threads(&self) -> usize {
        self.threads.unwrap_or(1).max(1)
    }

    pub fn resolve_codegen_units(&self) -> usize {
        self.codegen_units.unwrap_or(1).max(1)
    }

    pub fn resolve_function_sections(&self) -> Option<bool> {
        if self.no_function_sections {
            Some(false)
        } else if self.function_sections {
            Some(true)
        } else {
            None
        }
    }
}
