#![deny(unsafe_code)]

use clap::Parser;
use yukic_driver::{
    Args, DriverConfig, DriverSubcommand, banner_output, run_compiler_driver, should_color,
    targets_output, version_output,
};

fn build_driver_config(args: Args) -> Option<DriverConfig> {
    DriverConfig::from_args(args)
}

fn main() {
    let args = Args::parse();
    if args.version {
        println!("{}", version_output(should_color()));
        return;
    }
    if let Some(DriverSubcommand::Targets) = args.command {
        println!("{}", targets_output(should_color()));
        return;
    }
    if let Some(config) = build_driver_config(args) {
        if config.extra_info {
            println!("Compiler initialized");
        }
        if config.verbose {
            println!("Target: {}", config.target.as_deref().unwrap_or("host"));
            if !config.features.is_empty() {
                println!("Features: {}", config.features.join(","));
            }
            println!("Emit: {}", config.emit);
            println!("Opt-level: {}", config.opt_level);
            println!("Codegen units: {}", config.codegen_units);
            println!("Worker threads: {}", config.threads);
            if config.test_mode {
                println!("Test mode enabled");
            }
            if let Some(header) = &config.emit_header {
                println!("Emit C header: {}", header.display());
            }
        }

        if !std::path::Path::new(&config.input_file).exists() {
            eprintln!("error: input file '{}' not found", config.input_file);
            std::process::exit(1);
        }

        match run_compiler_driver(&config) {
            Ok(output_path) => {
                let abs_output = std::fs::canonicalize(&output_path).unwrap_or_else(|_| {
                    if output_path.is_absolute() {
                        output_path.clone()
                    } else if let Ok(cur) = std::env::current_dir() {
                        cur.join(&output_path)
                    } else {
                        output_path.clone()
                    }
                });
                let size_str = yukic_driver::format_artifact_size(&abs_output);
                if yukic_driver::should_color() {
                    use owo_colors::OwoColorize;
                    println!(
                        "{} {} {}",
                        "    Artifact:".cyan().bold(),
                        abs_output.display(),
                        size_str.dimmed()
                    );
                } else {
                    println!("    Artifact: {} {size_str}", abs_output.display());
                }
            }
            Err(yukic_driver::DriverError::CodegenFailed(msg)) => {
                let colored = yukic_driver::should_color_stderr();
                eprintln!(
                    "error: cgu compilation failed: {msg}{}",
                    yukic_driver::format_bug_report_notice(colored)
                );
                std::process::exit(1);
            }
            Err(err) => {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
        }
    } else {
        println!("{}", banner_output(should_color()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use yukic_driver::EmitKind;
    use yukic_opt::OptLevel;

    #[test]
    fn test_args_parsing_defaults() {
        let args = Args::try_parse_from(["yukic", "src/main.ykn"]).expect("parse args");
        assert_eq!(args.input, Some(PathBuf::from("src/main.ykn")));
        assert_eq!(args.codegen_units, None);
        assert_eq!(args.threads, None);
        assert_eq!(args.emit, EmitKind::Bin);
        assert_eq!(args.opt_level, OptLevel::O0);
        assert!(!args.verbose);
        assert!(!args.version);
        assert!(!args.test);
        assert_eq!(args.emit_header, None);
    }

    #[test]
    fn test_version_flag_parsing() {
        let short = Args::try_parse_from(["yukic", "-v"]).expect("parse -v");
        assert!(short.version);
        let long = Args::try_parse_from(["yukic", "--version"]).expect("parse --version");
        assert!(long.version);
    }

    #[test]
    fn test_targets_subcommand_parsing() {
        let args = Args::try_parse_from(["yukic", "targets"]).expect("parse targets");
        assert_eq!(args.command, Some(DriverSubcommand::Targets));
    }

    #[test]
    fn test_args_parsing_codegen_units_and_threads() {
        let args =
            Args::try_parse_from(["yukic", "src/main.ykn", "--codegen-units", "8", "-j", "4"])
                .expect("parse args");
        assert_eq!(args.codegen_units, Some(8));
        assert_eq!(args.threads, Some(4));
    }

    #[test]
    fn test_args_parsing_all_cli_flags() {
        let args = Args::try_parse_from([
            "yukic",
            "src/main.ykn",
            "--target",
            "x86_64-unknown-linux-gnu",
            "--features",
            "feat1,feat2",
            "--emit",
            "llvm-ir",
            "-O",
            "3",
            "-o",
            "bin/output.ll",
            "--verbose",
            "--test",
            "--emit-header",
            "include/lib.h",
            "--codegen-units",
            "16",
            "--threads",
            "8",
        ])
        .expect("parse all args");

        assert_eq!(args.input, Some(PathBuf::from("src/main.ykn")));
        assert_eq!(args.target, Some("x86_64-unknown-linux-gnu".to_string()));
        assert_eq!(args.features, vec!["feat1", "feat2"]);
        assert_eq!(args.emit, EmitKind::LlvmIr);
        assert_eq!(args.opt_level, OptLevel::O3);
        assert_eq!(args.out, Some(PathBuf::from("bin/output.ll")));
        assert!(args.verbose);
        assert!(args.test);
        assert_eq!(args.emit_header, Some(PathBuf::from("include/lib.h")));
        assert_eq!(args.codegen_units, Some(16));
        assert_eq!(args.threads, Some(8));
    }

    #[test]
    fn test_args_parsing_opt_levels() {
        let cases = [
            ("0", OptLevel::O0),
            ("1", OptLevel::O1),
            ("2", OptLevel::O2),
            ("3", OptLevel::O3),
            ("s", OptLevel::Os),
            ("z", OptLevel::Oz),
        ];

        for (flag_val, expected) in cases {
            let args = Args::try_parse_from(["yukic", "src/main.ykn", "-O", flag_val])
                .expect("parse opt level");
            assert_eq!(args.opt_level, expected);
        }
    }

    #[test]
    fn test_args_parsing_emit_kinds() {
        let cases = [
            ("bin", EmitKind::Bin),
            ("llvm-ir", EmitKind::LlvmIr),
            ("asm", EmitKind::Asm),
            ("obj", EmitKind::Obj),
        ];

        for (flag_val, expected) in cases {
            let args = Args::try_parse_from(["yukic", "src/main.ykn", "--emit", flag_val])
                .expect("parse emit kind");
            assert_eq!(args.emit, expected);
        }
    }

    #[test]
    fn test_build_driver_config_integration() {
        let args = Args::try_parse_from([
            "yukic",
            "src/main.ykn",
            "-o",
            "bin/app",
            "--codegen-units",
            "4",
            "-j",
            "2",
        ])
        .expect("parse args");

        let config = build_driver_config(args).expect("config should be present");
        assert_eq!(config.input_file, "src/main.ykn");
        assert_eq!(config.output_file, Some("bin/app".to_string()));
        assert_eq!(config.codegen_units, 4);
        assert_eq!(config.threads, 2);
    }

    #[test]
    fn test_build_driver_config_with_entry_flag() {
        let args = Args::try_parse_from([
            "yukic",
            "--entry",
            "src/lib.ykn",
            "--output-type",
            "static",
            "--verbose",
        ])
        .expect("parse args");

        let config = build_driver_config(args).expect("config should be present");
        assert_eq!(config.input_file, "src/lib.ykn");
        assert!(config.verbose);
    }
}
