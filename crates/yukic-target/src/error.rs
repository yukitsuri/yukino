use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub struct UnknownTargetError {
    target: String,
}

impl UnknownTargetError {
    pub fn new(target: impl Into<String>) -> Self {
        Self { target: target.into(), }
    }

    pub fn code(&self) -> &'static str { "E0703" }
    pub fn target(&self) -> &str { &self.target }
}

impl fmt::Display for UnknownTargetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] unsupported_target_triple: unknown target triple '{}'",
            self.code(),
            self.target
        )
    }
}
