#![deny(unsafe_code)]

use std::fmt;
use std::path::Path;
use std::str::FromStr;

use crate::error::OptError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LtoKind {
    #[default]
    None,
    Thin,
    Full,
}

impl LtoKind {
    pub fn parse_str(s: &str) -> Result<Self, OptError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "none" | "false" | "off" | "no" | "" => Ok(Self::None),
            "thin" | "true" | "on" | "yes" | "lto" => Ok(Self::Thin),
            "full" | "fat" => Ok(Self::Full),
            _ => Err(OptError::InvalidLto(s.to_string())),
        }
    }

    pub fn is_enabled(self) -> bool { self != Self::None }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Thin => "thin",
            Self::Full => "full",
        }
    }
}

impl FromStr for LtoKind {
    type Err = OptError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s)
    }
}

impl fmt::Display for LtoKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub fn write_bitcode_to_path(
    module: &inkwell::module::Module,
    path: &Path,
) -> Result<(), OptError> {
    if module.write_bitcode_to_path(path) { Ok(()) } 
    else { Err(OptError::BitcodeWriteFailed(path.display().to_string())) }
}

pub fn write_bitcode_to_memory(module: &inkwell::module::Module) -> Vec<u8> {
    let mem_buf = module.write_bitcode_to_memory();
    mem_buf.as_slice().to_vec()
}
