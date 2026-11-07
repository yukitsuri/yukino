#![deny(unsafe_code)]

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OptError {
    #[error("unknown optimization level '{0}', expected 0, 1, 2, 3, s, or z")]
    InvalidOptLevel(String),

    #[error("unknown LTO configuration '{0}', expected 'none', 'thin', or 'full'")]
    InvalidLto(String),

    #[error("target '{0}' not found in LLVM target registry")]
    TargetNotFound(String),

    #[error("failed to create LLVM TargetMachine for target '{0}'")]
    TargetMachineCreationFailed(String),

    #[error("pre-optimization LLVM verification failed: {0}")]
    PreVerificationFailed(String),

    #[error("post-optimization LLVM verification failed: {0}")]
    PostVerificationFailed(String),

    #[error("LLVM New Pass Manager execution failed for pipeline '{0}': {1}")]
    PassExecutionFailed(String, String),

    #[error("failed to write LLVM bitcode to '{0}'")]
    BitcodeWriteFailed(String),

    #[error("failed to emit object file to '{0}': {1}")]
    ObjectEmitFailed(String, String),

    #[error("worker thread panicked during parallel optimization: {0}")]
    WorkerPanicked(String),
}
