#![deny(unsafe_code)]

use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DriverError {
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),

    #[error("target triple error: {0}")]
    Target(String),

    #[error("runtime error: {0}")]
    Builtin(#[from] yukino_builtin::BuiltinError),

    #[error("runtime archive is required for target '{0}'; pass --runtime <path>")]
    RuntimeRequired(String),

    #[error("--runtime is not allowed for the host target")]
    RuntimeNotAllowedForHost,

    #[error("failed to create target machine for target '{0}': {1}")]
    TargetMachineCreation(String, String),

    #[error("failed to emit machine code to '{0}': {1}")]
    EmissionFailed(PathBuf, String),

    #[error("tool '{0}' not found in system or path")]
    ToolNotFound(String),

    #[error("static archiver '{0}' failed with exit code {1}: {2}")]
    ArchiverFailed(String, i32, String),

    #[error("linker '{0}' failed with exit code {1}: {2}")]
    LinkerFailed(String, i32, String),

    #[error("cgu compilation failed: {0}")]
    CodegenFailed(String),

    #[error("conditional compilation failed: {0}")]
    ConditionalCompilation(String),

    #[error("cgu optimization failed: {0}")]
    OptFailed(String),

    #[error("parallel compilation worker thread panicked: {0}")]
    WorkerPanicked(String),

    #[error("missing required input file")]
    NoInputFile,

    #[error("macro expansion failed: {0}")]
    MacroExpansion(String),

    #[error("aborting due to {0} previous error(s)")]
    FrontendValidation(usize),

    #[error("{0}")]
    BinaryCompatibility(#[from] yukic_target::BinaryCompatibilityError),
}

impl DriverError {
    pub fn format_error(&self, colored: bool) -> String {
        match self {
            DriverError::CodegenFailed(msg) => {
                format!(
                    "cgu compilation failed: {msg}{}",
                    crate::format_bug_report_notice(colored)
                )
            }
            other => other.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codegen_failed_format_error() {
        let err = DriverError::CodegenFailed("cgu_0: llvm verification failed".to_string());
        let plain = err.format_error(false);
        assert_eq!(
            plain,
            "cgu compilation failed: cgu_0: llvm verification failed\n\nThis might be a compiler bug. Please report it here: https://github.com/yukitsuri/yukino/issues"
        );

        let colored = err.format_error(true);
        assert!(colored.contains("cgu compilation failed: cgu_0: llvm verification failed"));
        assert!(colored.contains("https://github.com/yukitsuri/yukino/issues"));
        assert!(colored.contains("\x1b[34m"));
        assert!(colored.contains("\x1b[4m"));
    }

    #[test]
    fn test_other_error_format_error() {
        let err = DriverError::NoInputFile;
        assert_eq!(err.format_error(false), "missing required input file");
        assert_eq!(err.format_error(true), "missing required input file");
    }
}

