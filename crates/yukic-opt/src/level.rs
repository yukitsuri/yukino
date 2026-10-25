#![deny(unsafe_code)]

use std::fmt;
use std::str::FromStr;

use crate::error::OptError;
use crate::lto::LtoKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OptLevel {
    O0,
    O1,
    #[default]
    O2,
    O3,
    Os,
    Oz,
}

impl OptLevel {
    pub fn parse_str(s: &str) -> Result<Self, OptError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "0" | "o0" | "none" => Ok(Self::O0),
            "1" | "o1" | "less" => Ok(Self::O1),
            "2" | "o2" | "default" => Ok(Self::O2),
            "3" | "o3" | "aggressive" => Ok(Self::O3),
            "s" | "os" | "size" => Ok(Self::Os),
            "z" | "oz" | "minsize" => Ok(Self::Oz),
            _ => Err(OptError::InvalidOptLevel(s.to_string())),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::O0 => "O0",
            Self::O1 => "O1",
            Self::O2 => "O2",
            Self::O3 => "O3",
            Self::Os => "Os",
            Self::Oz => "Oz",
        }
    }

    pub fn to_inkwell_opt_level(self) -> inkwell::OptimizationLevel {
        match self {
            Self::O0 => inkwell::OptimizationLevel::None,
            Self::O1 => inkwell::OptimizationLevel::Less,
            Self::O2 | Self::Os | Self::Oz => inkwell::OptimizationLevel::Default,
            Self::O3 => inkwell::OptimizationLevel::Aggressive,
        }
    }

    pub fn default_pass_pipeline(self, lto: LtoKind) -> &'static str {
        match (self, lto) {
            (Self::O0, LtoKind::Thin) => "thinlto-pre-link<O0>",
            (Self::O1, LtoKind::Thin) => "thinlto-pre-link<O1>",
            (Self::O2, LtoKind::Thin) => "thinlto-pre-link<O2>",
            (Self::O3, LtoKind::Thin) => "thinlto-pre-link<O3>",
            (Self::Os, LtoKind::Thin) => "thinlto-pre-link<Os>",
            (Self::Oz, LtoKind::Thin) => "thinlto-pre-link<Oz>",
            (Self::O0, LtoKind::Full) => "lto-pre-link<O0>",
            (Self::O1, LtoKind::Full) => "lto-pre-link<O1>",
            (Self::O2, LtoKind::Full) => "lto-pre-link<O2>",
            (Self::O3, LtoKind::Full) => "lto-pre-link<O3>",
            (Self::Os, LtoKind::Full) => "lto-pre-link<Os>",
            (Self::Oz, LtoKind::Full) => "lto-pre-link<Oz>",
            (Self::O0, LtoKind::None) => "default<O0>",
            (Self::O1, LtoKind::None) => "default<O1>",
            (Self::O2, LtoKind::None) => "default<O2>",
            (Self::O3, LtoKind::None) => "default<O3>",
            (Self::Os, LtoKind::None) => "default<Os>",
            (Self::Oz, LtoKind::None) => "default<Oz>",
        }
    }
}

impl FromStr for OptLevel {
    type Err = OptError;

    fn from_str(s: &str) -> Result<Self, Self::Err> { Self::parse_str(s) }
}

impl fmt::Display for OptLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
