#![deny(unsafe_code)]

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use yukino::{
    BuildOptions, FOLDER_EXISTS_ERROR, NewOptions, ProjectError, ProjectKind, TestOptions, banner_output,
    build_project, clean_project, config, create_project, runtime, should_color, test_project,
    version_output, YUKINO_NAME,
};

#[derive(Parser, Debug)]
#[command(
    name = YUKINO_NAME,
    about = "Yukino toolchain and project manager",
    disable_version_flag = true
)]
pub struct Cli {
    #[arg(
        short = 'v',
        long = "version",
        action = clap::ArgAction::SetTrue,
        help = "Print version information"
    )]
    pub version: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Commands {
    New(NewArgs),
    Build(BuildArgs),
    Run(RunArgs),
    Test(TestArgs),
    Clean(CleanArgs),
    Config(ConfigArgs),
    Runtime(RuntimeArgs),
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Args, Debug, PartialEq, Eq)]
pub struct BuildArgs {
    #[arg(value_name = "PROFILE", help = "Build profile (debug, release, or custom)")]
    pub profile: Option<String>,

    #[arg(long = "target", value_name = "TRIPLE", help = "Target triple")]
    pub target: Option<String>,

    #[arg(long = "runtime", value_name = "PATH", help = "Runtime archive (.a)")]
    pub runtime: Option<PathBuf>,

    #[arg(long = "no-runtime", help = "Do not compile or link runtime")]
    pub no_runtime: bool,

    #[arg(short = 'b', long = "rebuild", alias = "rb", help = "Force rebuild")]
    pub rebuild: bool,

    #[arg(long = "refresh-deps", alias = "rd", help = "Force refresh dependencies")]
    pub refresh_deps: bool,

    #[arg(short = 'o', long = "override", action = clap::ArgAction::Append, help = "Override config")]
    pub overrides: Vec<String>,

    #[arg(
        short = 'F',
        long = "features",
        value_delimiter = ',',
        action = clap::ArgAction::Append,
        help = "Enable features (comma-separated or repeatable -F <feat>)"
    )]
    pub features: Vec<String>,

    #[arg(long = "no-default-features", help = "Disable default features")]
    pub no_default_features: bool,

    #[arg(long = "all-features", help = "Enable all features")]
    pub all_features: bool,

    #[arg(
        short = 'p',
        long = "package",
        value_name = "NAME",
        help = "Build specific package in workspace"
    )]
    pub package: Option<String>,

    #[arg(short = 'r', long = "run", help = "Run project after building")]
    pub run: bool,

    #[arg(long = "no-warns", help = "Disable compiler warnings")]
    pub no_warns: bool,

    #[arg(long = "extra-info", help = "Display extended compilation information")]
    pub extra_info: bool,

    #[arg(long = "no-out", help = "Disable compiler stdout, output 0 on success or 1 on error")]
    pub no_out: bool,

    #[arg(long = "no-lib-log", help = "Disable library compilation logs")]
    pub no_lib_log: bool,

    #[arg(long = "lib-warns", help = "Display warnings from third-party libraries and stdlib")]
    pub lib_warns: bool,

    #[arg(short = 'j', long = "threads", help = "Thread count")]
    pub threads: Option<usize>,

    #[arg(long = "cgunits", alias = "codegen-units", help = "Number of codegen units")]
    pub cgunits: Option<usize>,

    #[arg(trailing_var_arg = true)]
    pub trailing_args: Vec<String>,
}

#[derive(Args, Debug, PartialEq, Eq, Default)]
pub struct RunArgs {
    #[arg(value_name = "PROFILE", help = "Build profile (debug, release)")]
    pub profile: Option<String>,

    #[arg(long = "target", value_name = "TRIPLE", help = "Target triple")]
    pub target: Option<String>,

    #[arg(short = 'o', long = "override", action = clap::ArgAction::Append, help = "Override config")]
    pub overrides: Vec<String>,

    #[arg(
        short = 'F',
        long = "features",
        value_delimiter = ',',
        action = clap::ArgAction::Append,
        help = "Enable features (comma-separated or repeatable -F <feat>)"
    )]
    pub features: Vec<String>,

    #[arg(long = "no-default-features", help = "Disable default features")]
    pub no_default_features: bool,

    #[arg(long = "all-features", help = "Enable all features")]
    pub all_features: bool,

    #[arg(
        short = 'p',
        long = "package",
        value_name = "NAME",
        help = "Build specific package in workspace"
    )]
    pub package: Option<String>,

    #[arg(long = "no-warns", help = "Disable compiler warnings")]
    pub no_warns: bool,

    #[arg(long = "extra-info", help = "Display extended compilation information")]
    pub extra_info: bool,

    #[arg(long = "no-out", help = "Disable compiler stdout, output 0 on success or 1 on error")]
    pub no_out: bool,

    #[arg(long = "no-lib-log", help = "Disable library compilation logs")]
    pub no_lib_log: bool,

    #[arg(long = "lib-warns", help = "Display warnings from third-party libraries and stdlib")]
    pub lib_warns: bool,

    #[arg(short = 'j', long = "threads", help = "Thread count")]
    pub threads: Option<usize>,

    #[arg(short = 'b', long = "rebuild", alias = "rb", help = "Force rebuild")]
    pub rebuild: bool,

    #[arg(long = "refresh-deps", alias = "rd", help = "Force refresh dependencies")]
    pub refresh_deps: bool,

    #[arg(long = "cgunits", alias = "codegen-units", help = "Number of codegen units")]
    pub cgunits: Option<usize>,

    #[arg(trailing_var_arg = true)]
    pub trailing_args: Vec<String>,
}

#[derive(Args, Debug, PartialEq, Eq, Default)]
pub struct TestArgs {
    #[arg(value_name = "FILTER", help = "Test filter pattern")]
    pub filter: Option<String>,

    #[arg(long = "exact", help = "Filter matches name exactly")]
    pub exact: bool,

    #[arg(long = "nocapture", help = "Show stdout/stderr from tests")]
    pub nocapture: bool,

    #[arg(short = 'j', long = "threads", help = "Test thread count")]
    pub threads: Option<usize>,

    #[arg(long = "ignored", help = "Run only ignored tests")]
    pub ignored: bool,

    #[arg(long = "include-ignored", help = "Run both ignored and unignored tests")]
    pub include_ignored: bool,

    #[arg(long = "fail-fast", help = "Stop after first failed test")]
    pub fail_fast: bool,

    #[arg(long = "list", help = "List all matching tests without running")]
    pub list: bool,

    #[arg(short = 'q', long = "quiet", help = "Quiet test output")]
    pub quiet: bool,

    #[arg(long = "timeout", help = "Default test timeout in milliseconds")]
    pub timeout_ms: Option<u64>,

    #[arg(short = 'b', long = "rebuild", alias = "rb", help = "Force rebuild")]
    pub rebuild: bool,

    #[arg(
        short = 'F',
        long = "features",
        value_delimiter = ',',
        action = clap::ArgAction::Append,
        help = "Enable features (comma-separated or repeatable -F <feat>)"
    )]
    pub features: Vec<String>,

    #[arg(long = "no-default-features", help = "Disable default features")]
    pub no_default_features: bool,

    #[arg(long = "all-features", help = "Enable all features")]
    pub all_features: bool,

    #[arg(
        short = 'p',
        long = "package",
        value_name = "NAME",
        help = "Test specific package in workspace"
    )]
    pub package: Option<String>,
}

#[derive(Args, Debug, PartialEq, Eq, Default)]
pub struct CleanArgs {
    #[arg(short = 'a', long = "all", help = "Also remove yuki-work/common/yuki.lock")]
    pub all: bool,
}

#[derive(Args, Debug, PartialEq, Eq)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: Option<ConfigCommands>,
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum ConfigCommands {
    Cthreads(CthreadsArgs),
    Macrotimeout(MacrotimeoutArgs),
    Get(KeyArgs),
    Set(KeyValueArgs),
    List,
    Path,
}

#[derive(Args, Debug, PartialEq, Eq)]
pub struct CthreadsArgs {
    pub count: usize,
}

#[derive(Args, Debug, PartialEq, Eq)]
pub struct MacrotimeoutArgs {
    pub timeout: u64,
}

#[derive(Args, Debug, PartialEq, Eq)]
pub struct KeyArgs {
    pub key: String,
}

#[derive(Args, Debug, PartialEq, Eq)]
pub struct KeyValueArgs {
    pub key: String,
    pub value: String,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Args, Debug, PartialEq, Eq)]
pub struct RuntimeArgs {
    #[arg(
        short = 'i',
        long = "install",
        conflicts_with_all = ["remove", "list", "info", "reinstall"]
    )]
    pub install: bool,

    #[arg(
        short = 'r',
        long = "remove",
        conflicts_with_all = ["install", "list", "info", "reinstall"]
    )]
    pub remove: bool,

    #[arg(
        short = 'l',
        long = "list",
        conflicts_with_all = ["install", "remove", "info", "reinstall"]
    )]
    pub list: bool,

    #[arg(
        long = "info",
        conflicts_with_all = ["install", "remove", "list", "reinstall"]
    )]
    pub info: bool,

    #[arg(
        long = "reinstall",
        conflicts_with_all = ["install", "remove", "list", "info"]
    )]
    pub reinstall: bool,

    #[arg(value_name = "TARGET")]
    pub target: Option<String>,

    #[arg(value_name = "ARCHIVE")]
    pub archive: Option<std::path::PathBuf>,
}

#[derive(Args, Debug, PartialEq, Eq)]
pub struct ProjectKindArgs {
    #[arg(
        long,
        conflicts_with_all = ["static_lib", "dynamic_lib", "workspace", "macro_lib"],
        help = "Create an executable application"
    )]
    pub app: bool,

    #[arg(
        long = "static",
        conflicts_with_all = ["app", "dynamic_lib", "workspace", "macro_lib"],
        help = "Create a static library"
    )]
    pub static_lib: bool,

    #[arg(
        long = "dynamic",
        conflicts_with_all = ["app", "static_lib", "workspace", "macro_lib"],
        help = "Create a shared/dynamic library"
    )]
    pub dynamic_lib: bool,

    #[arg(
        long = "workspace",
        conflicts_with_all = ["app", "static_lib", "dynamic_lib", "macro_lib"],
        help = "Create a workspace"
    )]
    pub workspace: bool,

    #[arg(
        long = "macro",
        conflicts_with_all = ["app", "static_lib", "dynamic_lib", "workspace"],
        help = "Create a procedural macro package"
    )]
    pub macro_lib: bool,
}

impl ProjectKindArgs {
    pub fn to_kind(&self) -> ProjectKind {
        if self.workspace {
            ProjectKind::Workspace
        } else if self.macro_lib {
            ProjectKind::Macro
        } else if self.static_lib {
            ProjectKind::StaticLib
        } else if self.dynamic_lib {
            ProjectKind::DynamicLib
        } else {
            ProjectKind::App
        }
    }
}

#[derive(Args, Debug, PartialEq, Eq)]
pub struct NewArgs {
    #[arg(value_name = "NAME")]
    pub name: Option<String>,

    #[command(flatten)]
    pub kind: ProjectKindArgs,

    #[arg(
        short = 'i',
        long = "init",
        help = "Initialize project in current directory"
    )]
    pub init: bool,
}

fn parse_overrides(raw: &[String]) -> Result<Vec<(String, String)>, ProjectError> {
    let mut pairs = Vec::new();
    for item in raw {
        let Some((k, v)) = item.split_once('=') else {
            return Err(ProjectError::Runtime(format!(
                "invalid override format '{item}', expected key=value"
            )));
        };
        pairs.push((k.trim().to_string(), v.trim().to_string()));
    }
    Ok(pairs)
}

fn execute_build(args: BuildArgs) -> Result<(), ProjectError> {
    let current_dir = std::env::current_dir().map_err(|e| ProjectError::Io(e.to_string()))?;
    let overrides = parse_overrides(&args.overrides)?;

    let opts = BuildOptions {
        profile: args.profile.unwrap_or_else(|| "debug".to_string()),
        target: args.target,
        runtime: args.runtime,
        no_runtime: args.no_runtime,
        rebuild: args.rebuild,
        refresh_deps: args.refresh_deps,
        overrides,
        features: args.features,
        no_default_features: args.no_default_features,
        all_features: args.all_features,
        package: args.package,
        run: args.run,
        run_args: args.trailing_args,
        no_warns: args.no_warns,
        extra_info: args.extra_info,
        no_out: args.no_out,
        no_lib_log: args.no_lib_log,
        lib_warns: args.lib_warns,
        threads: args.threads,
        cgunits: args.cgunits,
        project_dir: current_dir,
    };
    build_project(&opts)?;
    Ok(())
}

fn execute_run(args: RunArgs) -> Result<(), ProjectError> {
    let current_dir = std::env::current_dir().map_err(|e| ProjectError::Io(e.to_string()))?;
    let overrides = parse_overrides(&args.overrides)?;

    let opts = BuildOptions {
        profile: args.profile.unwrap_or_else(|| "debug".to_string()),
        target: args.target,
        runtime: None,
        no_runtime: false,
        rebuild: args.rebuild,
        refresh_deps: args.refresh_deps,
        overrides,
        features: args.features,
        no_default_features: args.no_default_features,
        all_features: args.all_features,
        package: args.package,
        run: true,
        run_args: args.trailing_args,
        no_warns: args.no_warns,
        extra_info: args.extra_info,
        no_out: args.no_out,
        no_lib_log: args.no_lib_log,
        lib_warns: args.lib_warns,
        threads: args.threads,
        cgunits: args.cgunits,
        project_dir: current_dir,
    };
    build_project(&opts)?;
    Ok(())
}

fn execute_test(args: &TestArgs) -> Result<(), ProjectError> {
    let current_dir = std::env::current_dir().map_err(|e| ProjectError::Io(e.to_string()))?;
    let opts = TestOptions {
        filter: args.filter.clone(),
        exact: args.exact,
        nocapture: args.nocapture,
        threads: args.threads,
        ignored: args.ignored,
        include_ignored: args.include_ignored,
        fail_fast: args.fail_fast,
        list: args.list,
        quiet: args.quiet,
        timeout_ms: args.timeout_ms,
        rebuild: args.rebuild,
        features: args.features.clone(),
        all_features: args.all_features,
        no_default_features: args.no_default_features,
        package: args.package.clone(),
    };
    test_project(&current_dir, &opts)?;
    Ok(())
}

fn execute_clean(args: &CleanArgs) -> Result<(), ProjectError> {
    let current_dir = std::env::current_dir().map_err(|e| ProjectError::Io(e.to_string()))?;
    clean_project(&current_dir, args.all)?;
    Ok(())
}

fn execute_config(args: ConfigArgs) -> Result<(), ProjectError> {
    match args.command {
        Some(ConfigCommands::Cthreads(sub)) => {
            config::set_cthreads(sub.count).map_err(|e| ProjectError::Io(e.to_string()))?;
            println!("cthreads set to {}", sub.count);
        }
        Some(ConfigCommands::Macrotimeout(sub)) => {
            config::set_macrotimeout(sub.timeout).map_err(|e| ProjectError::Io(e.to_string()))?;
            println!("macrotimeout set to {}", sub.timeout);
        }
        Some(ConfigCommands::Get(sub)) => {
            if let Some(val) = config::get_global_config_value(&sub.key) {
                println!("{val}");
            } else if sub.key == "noanim" {
                println!("false");
            } else if sub.key == "minibar" {
                println!("false");
            } else if sub.key == "cthreads" {
                println!("8");
            } else if sub.key == "full-lib-log" {
                println!("false");
            } else if sub.key == "macrotimeout" {
                println!("{}", config::load_global_macrotimeout());
            } else {
                eprintln!("key '{}' not found", sub.key);
                std::process::exit(1);
            }
        }
        Some(ConfigCommands::Set(sub)) => {
            config::set_global_config_value(&sub.key, &sub.value)
                .map_err(|e| ProjectError::Io(e.to_string()))?;
            println!("{} = {}", sub.key, sub.value);
        }
        Some(ConfigCommands::List) => {
            for (k, v) in config::list_global_config() {
                println!("{k} = {v}");
            }
        }
        Some(ConfigCommands::Path) | None => {
            println!("{}", config::global_config_path().display());
        }
    }
    Ok(())
}

fn execute_runtime(args: RuntimeArgs) -> Result<(), ProjectError> {
    if args.install {
        if let Some(archive) = args.archive {
            let target = args.target.ok_or_else(|| {
                ProjectError::Runtime("target is required for local installation".to_string())
            })?;
            let path = runtime::install_local(&target, &archive)
                .map_err(|error| ProjectError::Runtime(error.to_string()))?;
            println!("Installed runtime: {}", path.display());
        } else {
            let installed = runtime::install_remote(args.target.as_deref())
                .map_err(|error| ProjectError::Runtime(error.to_string()))?;
            for path in installed {
                println!("Installed runtime: {}", path.display());
            }
        }
        return Ok(());
    }

    if args.remove {
        let removed = runtime::remove(args.target.as_deref())
            .map_err(|error| ProjectError::Runtime(error.to_string()))?;
        for target in removed {
            println!("Removed runtime: {target}");
        }
        return Ok(());
    }

    if args.list {
        for record in runtime::list().map_err(|error| ProjectError::Runtime(error.to_string()))? {
            println!(
                "{} ({}, {})",
                record.target,
                record.compiler_version,
                record.source.name()
            );
        }
        return Ok(());
    }

    if args.info {
        for record in runtime::info(args.target.as_deref())
            .map_err(|error| ProjectError::Runtime(error.to_string()))?
        {
            println!("Target: {}", record.target);
            println!("Compiler: {}", record.compiler_version);
            println!("Source: {}", record.source.name());
            println!("Archive: {}", record.archive);
            println!("SHA-256: {}", record.sha256);
        }
        return Ok(());
    }

    if args.reinstall {
        let installed = runtime::reinstall()
            .map_err(|error| ProjectError::Runtime(error.to_string()))?;
        for path in installed {
            println!("Reinstalled runtime: {}", path.display());
        }
        return Ok(());
    }

    Err(ProjectError::Runtime(
        "choose one of --install, --remove, --list, --info, or --reinstall".to_string(),
    ))
}

fn execute_command(cmd: Commands) -> Result<(), ProjectError> {
    match cmd {
        Commands::New(args) => {
            let opts = NewOptions {
                name: args.name,
                kind: args.kind.to_kind(),
                init: args.init,
            };
            let current_dir =
                std::env::current_dir().map_err(|e| ProjectError::Io(e.to_string()))?;
            create_project(&opts, &current_dir).map(|_| ())
        }
        Commands::Build(args) => execute_build(args),
        Commands::Run(args) => execute_run(args),
        Commands::Test(args) => execute_test(&args),
        Commands::Clean(args) => execute_clean(&args),
        Commands::Config(args) => execute_config(args),
        Commands::Runtime(args) => execute_runtime(args),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && args[1].is_empty() {
        println!("{}", banner_output(should_color()));
        return;
    }

    let cli = Cli::parse();
    if cli.version {
        println!("{}", version_output(should_color()));
        return;
    }

    if let Some(cmd) = cli.command {
        let is_no_out = match &cmd {
            Commands::Build(b) => b.no_out,
            Commands::Run(r) => r.no_out,
            _ => false,
        };
        if let Err(e) = execute_command(cmd) {
            if is_no_out {
                println!("1");
            } else {
                match e {
                    ProjectError::FolderAlreadyExists => {
                        println!("{FOLDER_EXISTS_ERROR}");
                    }
                    _ => {
                        eprintln!("Error: {e}");
                    }
                }
            }
            std::process::exit(1);
        }
    } else {
        println!("{}", banner_output(should_color()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_flag_parsing() {
        let cli = Cli::try_parse_from(["yukino", "-v"]).expect("parse -v");
        assert!(cli.version);
        let cli_long = Cli::try_parse_from(["yukino", "--version"]).expect("parse --version");
        assert!(cli_long.version);
    }

    #[test]
    fn test_subcommand_new_default_app() {
        let cli = Cli::try_parse_from(["yukino", "new", "my_project"]).expect("parse new");
        assert_eq!(
            cli.command,
            Some(Commands::New(NewArgs {
                name: Some("my_project".to_string()),
                kind: ProjectKindArgs {
                    app: false,
                    static_lib: false,
                    dynamic_lib: false,
                    workspace: false,
                    macro_lib: false,
                },
                init: false,
            }))
        );

        let cli_macro = Cli::try_parse_from(["yukino", "new", "my_macro", "--macro"]).expect("parse new macro");
        match cli_macro.command {
            Some(Commands::New(args)) => {
                assert!(args.kind.macro_lib);
                assert_eq!(args.kind.to_kind(), ProjectKind::Macro);
            }
            _ => panic!("expected New command with macro"),
        }

        let cli_ws = Cli::try_parse_from(["yukino", "new", "my_workspace", "--workspace"]).expect("parse new workspace");
        match cli_ws.command {
            Some(Commands::New(args)) => {
                assert!(args.kind.workspace);
                assert_eq!(args.kind.to_kind(), ProjectKind::Workspace);
            }
            _ => panic!("expected New command with workspace"),
        }
    }

    #[test]
    fn test_subcommand_build_parsing() {
        let cli = Cli::try_parse_from([
            "yukino",
            "build",
            "release",
            "--target",
            "x86_64-pc-windows-gnu",
            "-o",
            "opt_level=2",
            "-F",
            "net",
            "--features",
            "ssl,crypto",
            "--rebuild",
            "--refresh-deps",
            "-r",
            "--",
            "arg1",
        ])
        .expect("parse build");

        match cli.command {
            Some(Commands::Build(args)) => {
                assert_eq!(args.profile, Some("release".to_string()));
                assert_eq!(args.target, Some("x86_64-pc-windows-gnu".to_string()));
                assert_eq!(args.overrides, vec!["opt_level=2".to_string()]);
                assert_eq!(
                    args.features,
                    vec!["net".to_string(), "ssl".to_string(), "crypto".to_string()]
                );
                assert!(args.rebuild);
                assert!(args.refresh_deps);
                assert!(args.run);
                assert_eq!(args.trailing_args, vec!["arg1".to_string()]);
                assert!(!args.no_out);
            }
            _ => panic!("expected Build command"),
        }

        let cli_no_out = Cli::try_parse_from(["yukino", "build", "--no-out"]).expect("parse build --no-out");
        match cli_no_out.command {
            Some(Commands::Build(args)) => assert!(args.no_out),
            _ => panic!("expected Build command"),
        }

        let cli_no_lib_log = Cli::try_parse_from(["yukino", "build", "--no-lib-log"]).expect("parse build --no-lib-log");
        match cli_no_lib_log.command {
            Some(Commands::Build(args)) => assert!(args.no_lib_log),
            _ => panic!("expected Build command"),
        }

        let cli_lib_warns = Cli::try_parse_from(["yukino", "build", "--lib-warns"]).expect("parse build --lib-warns");
        match cli_lib_warns.command {
            Some(Commands::Build(args)) => assert!(args.lib_warns),
            _ => panic!("expected Build command"),
        }

        let cli_all_feat = Cli::try_parse_from(["yukino", "build", "--all-features", "-p", "core"]).expect("parse build with flags");
        match cli_all_feat.command {
            Some(Commands::Build(args)) => {
                assert!(args.all_features);
                assert_eq!(args.package.as_deref(), Some("core"));
            }
            _ => panic!("expected Build command with all-features and package"),
        }
    }

    #[test]
    fn test_subcommand_config_parsing() {
        let cli_set = Cli::try_parse_from(["yukino", "config", "set", "cthreads", "8"])
            .expect("parse config set");
        match cli_set.command {
            Some(Commands::Config(args)) => match args.command {
                Some(ConfigCommands::Set(sub)) => {
                    assert_eq!(sub.key, "cthreads");
                    assert_eq!(sub.value, "8");
                }
                _ => panic!("expected Set"),
            },
            _ => panic!("expected Config"),
        }

        let cli_cthreads = Cli::try_parse_from(["yukino", "config", "cthreads", "6"])
            .expect("parse config cthreads");
        match cli_cthreads.command {
            Some(Commands::Config(args)) => match args.command {
                Some(ConfigCommands::Cthreads(sub)) => assert_eq!(sub.count, 6),
                _ => panic!("expected Cthreads"),
            },
            _ => panic!("expected Config"),
        }

        let cli_macrotimeout = Cli::try_parse_from(["yukino", "config", "macrotimeout", "15"])
            .expect("parse config macrotimeout");
        match cli_macrotimeout.command {
            Some(Commands::Config(args)) => match args.command {
                Some(ConfigCommands::Macrotimeout(sub)) => assert_eq!(sub.timeout, 15),
                _ => panic!("expected Macrotimeout"),
            },
            _ => panic!("expected Config"),
        }
    }

    #[test]
    fn test_subcommand_clean_parsing() {
        let cli = Cli::try_parse_from(["yukino", "clean", "-a"]).expect("parse clean");
        match cli.command {
            Some(Commands::Clean(args)) => assert!(args.all),
            _ => panic!("expected Clean"),
        }
    }
}
