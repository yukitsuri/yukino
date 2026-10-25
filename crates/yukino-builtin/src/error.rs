use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BuiltinError {
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),

    #[error(
        "rustc is required to compile the Yukino runtime, but was not found in PATH. Please install Rust: https://rustup.rs"
    )]
    RustcNotFound,

    #[error(
        "Rust target '{target}' is not installed for rustc.\nPlease install it by running:\n  {install_cmd}"
    )]
    TargetNotInstalled { target: String, install_cmd: String },

    #[error("failed to compile runtime for target '{target}': {details}")]
    CompilationFailed { target: String, details: String },
}
