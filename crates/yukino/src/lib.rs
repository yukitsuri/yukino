#![deny(unsafe_code)]

use std::io::IsTerminal;

pub mod builder;
pub mod config;
pub mod dashboard;
pub mod features;
pub mod fetch;
pub mod lockfile;
pub mod manifest;
pub mod pool;
pub mod project;
pub mod remote;
pub mod runtime;
pub mod semver;
pub mod tracker;

pub use builder::{BuildOptions, TestOptions, build_project, clean_project, test_project};
pub use dashboard::BuildDashboard;

pub use config::{
    default_pool_threads, global_config_path, load_global_cthreads, load_global_full_lib_log,
    load_global_macrotimeout, num_cpus, parse_cthreads_from_json, resolve_pool_threads,
    set_macrotimeout,
};
pub use pool::CompilationThreadPool;
pub use project::{
    FOLDER_EXISTS_ERROR, NewOptions, ProjectError, ProjectKind, create_project,
    resolve_project_target, update_gitignore,
};
pub use semver::{LowerBound, SemVer, SemVerError, UpperBound, VersionRange};
pub use tracker::{ThreadStatus, ThreadTracker, format_thread_linear, format_thread_status};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BUILD_DATE: &str = env!("YUKINO_BUILD_DATE");
pub const YUKINO_NAME: &str = env!("YUKINO_NAME");
pub const YUKIC_NAME: &str = env!("YUKIC_NAME");
pub const YUKINO_DIR: &str = env!("YUKINO_DIR");
pub const YUKINO_CFG_DIR: &str = env!("YUKINO_CFG_DIR");

pub fn should_color() -> bool {
    if std::env::var_os("CLICOLOR_FORCE").is_some_and(|v| v != "0") {
        return true;
    }
    if std::env::var_os("NO_COLOR").is_some() { return false; }
    if std::env::var("TERM").is_ok_and(|t| t == "dumb") {
        return false;
    }
    std::io::stdout().is_terminal()
}

pub fn version_output(colored: bool) -> String {
    if colored {
        use owo_colors::OwoColorize;
        format!(
            "Yukino tool v{VERSION}\nUse {} or {} for more",
            YUKINO_NAME.cyan(),
            format!("{YUKINO_NAME} --help").cyan()
        )
    } else {
        format!("Yukino tool v{VERSION}\nUse {YUKINO_NAME} or {YUKINO_NAME} --help for more")
    }
}

pub fn banner_output(colored: bool) -> String {
    if colored {
        use owo_colors::OwoColorize;
        format!(
            "{} v{VERSION}\n{}: {BUILD_DATE}\n{}: Apache 2.0 2026\n{}: {}",
            "Yukino language tool".cyan().bold(),
            "Built".cyan(),
            "License".cyan(),
            "Source code".cyan(),
            "https://github.com/yukitsuri/yukino".underline()
        )
    } else {
        format!(
            "Yukino language tool v{VERSION}\nBuilt: {BUILD_DATE}\nLicense: Apache 2.0 2026\nSource code: https://github.com/yukitsuri/yukino"
        )
    }
}

pub fn format_url(url: &str, colored: bool) -> String {
    if colored {
        use owo_colors::OwoColorize;
        url.green().underline().to_string()
    } 

    else { url.to_string() }
}

pub fn format_path(path: &str, colored: bool) -> String {
    if colored {
        use owo_colors::OwoColorize;
        path.cyan().to_string()
    } 

    else { path.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_output_plain() {
        let out = version_output(false);
        assert!(out.starts_with("Yukino tool v"));
        assert!(out.contains("Use yukino or yukino --help for more"));
    }

    #[test]
    fn test_banner_output_plain() {
        let out = banner_output(false);
        assert!(out.starts_with("Yukino language tool v"));
        assert!(out.contains("Built: "));
        assert!(out.contains("License: Apache 2.0 2026"));
        assert!(out.contains("Source code: https://github.com/yukitsuri/yukino"));
    }

    #[test]
    fn test_version_output_colored() {
        let out = version_output(true);
        assert!(out.contains("\x1b[36myukino\x1b[39m"));
        assert!(out.contains("\x1b[36myukino --help\x1b[39m"));
    }

    #[test]
    fn test_format_url_and_path() {
        let plain_url = format_url("https://example.com", false);
        assert_eq!(plain_url, "https://example.com");

        let colored_url = format_url("https://example.com", true);
        assert!(colored_url.contains("https://example.com"));
        assert!(colored_url.contains("\x1b[32m") || colored_url.contains("\x1b[4m"));

        let plain_path = format_path("/home/user/project", false);
        assert_eq!(plain_path, "/home/user/project");

        let colored_path = format_path("/home/user/project", true);
        assert!(colored_path.contains("/home/user/project"));
        assert!(colored_path.contains("\x1b[36m"));
    }
}
