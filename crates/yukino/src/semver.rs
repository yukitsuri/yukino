#![deny(unsafe_code)]

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl SemVer {
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn parse(input: &str) -> Result<Self, SemVerError> {
        let s = input.trim();
        let s = s
            .strip_prefix('v')
            .or_else(|| s.strip_prefix('V'))
            .unwrap_or(s);
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(SemVerError::InvalidFormat(input.to_string()));
        }
        let major = parts[0]
            .parse::<u64>()
            .map_err(|_| SemVerError::InvalidFormat(input.to_string()))?;
        let minor = parts[1]
            .parse::<u64>()
            .map_err(|_| SemVerError::InvalidFormat(input.to_string()))?;
        let patch = parts[2]
            .parse::<u64>()
            .map_err(|_| SemVerError::InvalidFormat(input.to_string()))?;
        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LowerBound {
    Unbounded,
    Exact(SemVer),
}

impl fmt::Display for LowerBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unbounded => write!(f, "-"),
            Self::Exact(v) => write!(f, "{v}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpperBound {
    Unbounded,
    Exact(SemVer),
}

impl fmt::Display for UpperBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unbounded => write!(f, "+"),
            Self::Exact(v) => write!(f, "{v}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRange {
    pub lower: LowerBound,
    pub upper: UpperBound,
}

impl fmt::Display for VersionRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[\"{}\", \"{}\"]", self.lower, self.upper)
    }
}

impl VersionRange {
    pub fn parse_slice(slice: &[String]) -> Result<Self, SemVerError> {
        if slice.is_empty() {
            return Err(SemVerError::EmptyRange);
        }
        if slice.len() != 2 {
            return Err(SemVerError::InvalidRangeLength(slice.len()));
        }
        let lower = Self::parse_lower(&slice[0])?;
        let upper = Self::parse_upper(&slice[1])?;
        match (&lower, &upper) {
            (LowerBound::Exact(min), UpperBound::Exact(max)) if min > max => {
                return Err(SemVerError::InvertedRange(min.to_string(), max.to_string()));
            }
            _ => {}
        }
        Ok(Self { lower, upper })
    }

    fn parse_lower(s: &str) -> Result<LowerBound, SemVerError> {
        let trimmed = s.trim();
        if trimmed == "-" {
            Ok(LowerBound::Unbounded)
        } else if trimmed == "+" {
            Err(SemVerError::InvalidBound(
                "'+' is not a valid lower bound".to_string(),
            ))
        } else {
            let ver = SemVer::parse(trimmed)?;
            Ok(LowerBound::Exact(ver))
        }
    }

    fn parse_upper(s: &str) -> Result<UpperBound, SemVerError> {
        let trimmed = s.trim();
        if trimmed == "+" {
            Ok(UpperBound::Unbounded)
        } else if trimmed == "-" {
            Err(SemVerError::InvalidBound(
                "'-' is not a valid upper bound".to_string(),
            ))
        } else {
            let ver = SemVer::parse(trimmed)?;
            Ok(UpperBound::Exact(ver))
        }
    }

    pub fn contains(&self, version: &SemVer) -> bool {
        match &self.lower {
            LowerBound::Exact(min) if version < min => return false,
            _ => {}
        }
        match &self.upper {
            UpperBound::Exact(max) if version > max => return false,
            _ => {}
        }
        true
    }

    pub fn check_compatibility(&self, version: &SemVer) -> Result<(), SemVerError> {
        if self.contains(version) {
            Ok(())
        } else {
            Err(SemVerError::IncompatibleVersion {
                current: version.to_string(),
                range: self.to_string(),
            })
        }
    }

    pub fn default_range(version: &SemVer) -> [String; 2] {
        [version.to_string(), "+".to_string()]
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SemVerError {
    InvalidFormat(String),
    InvalidRangeLength(usize),
    EmptyRange,
    InvalidBound(String),
    InvertedRange(String, String),
    IncompatibleVersion { current: String, range: String },
}

impl fmt::Display for SemVerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat(s) => write!(f, "invalid semver format: '{s}'"),
            Self::InvalidRangeLength(n) => write!(f, "range requires 2 elements, got {n}"),
            Self::EmptyRange => write!(f, "range array cannot be empty"),
            Self::InvalidBound(msg) => write!(f, "{msg}"),
            Self::InvertedRange(min, max) => {
                write!(f, "range min '{min}' cannot be greater than max '{max}'")
            }
            Self::IncompatibleVersion { current, range } => {
                write!(
                    f,
                    "version '{current}' does not satisfy required range {range}"
                )
            }
        }
    }
}

impl std::error::Error for SemVerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_parse_valid() {
        let ver = SemVer::parse("1.2.3").expect("valid");
        assert_eq!(ver, SemVer::new(1, 2, 3));
        assert_eq!(ver.to_string(), "1.2.3");
    }

    #[test]
    fn test_semver_parse_invalid() {
        assert!(SemVer::parse("1.2").is_err());
        assert!(SemVer::parse("1.2.3.4").is_err());
        assert!(SemVer::parse("abc").is_err());
    }

    #[test]
    fn test_semver_ordering() {
        assert!(SemVer::new(0, 1, 0) < SemVer::new(0, 2, 0));
        assert!(SemVer::new(0, 1, 1) > SemVer::new(0, 1, 0));
        assert!(SemVer::new(1, 0, 0) > SemVer::new(0, 9, 9));
        assert_eq!(SemVer::new(1, 2, 3), SemVer::new(1, 2, 3));
    }

    #[test]
    fn test_range_inclusive() {
        let range = VersionRange::parse_slice(&["0.1.0".to_string(), "0.2.0".to_string()])
            .expect("valid range");
        assert!(range.contains(&SemVer::new(0, 1, 0)));
        assert!(range.contains(&SemVer::new(0, 1, 5)));
        assert!(range.contains(&SemVer::new(0, 2, 0)));
        assert!(!range.contains(&SemVer::new(0, 0, 9)));
        assert!(!range.contains(&SemVer::new(0, 2, 1)));
    }

    #[test]
    fn test_range_exact() {
        let range = VersionRange::parse_slice(&["0.1.0".to_string(), "0.1.0".to_string()])
            .expect("valid range");
        assert!(range.contains(&SemVer::new(0, 1, 0)));
        assert!(!range.contains(&SemVer::new(0, 1, 1)));
    }

    #[test]
    fn test_range_lower_unbounded() {
        let range = VersionRange::parse_slice(&["-".to_string(), "0.2.0".to_string()])
            .expect("valid range");
        assert!(range.contains(&SemVer::new(0, 0, 1)));
        assert!(range.contains(&SemVer::new(0, 2, 0)));
        assert!(!range.contains(&SemVer::new(0, 2, 1)));
    }

    #[test]
    fn test_range_upper_unbounded() {
        let range = VersionRange::parse_slice(&["0.1.0".to_string(), "+".to_string()])
            .expect("valid range");
        assert!(range.contains(&SemVer::new(0, 1, 0)));
        assert!(range.contains(&SemVer::new(1, 0, 0)));
        assert!(!range.contains(&SemVer::new(0, 0, 9)));
    }

    #[test]
    fn test_range_invalid_syntax() {
        assert!(VersionRange::parse_slice(&[]).is_err());
        assert!(VersionRange::parse_slice(&["0.1.0".to_string()]).is_err());
        assert!(VersionRange::parse_slice(&["0.1.0".to_string(), "-".to_string()]).is_err());
        assert!(VersionRange::parse_slice(&["+".to_string(), "0.2.0".to_string()]).is_err());
        assert!(VersionRange::parse_slice(&["0.2.0".to_string(), "0.1.0".to_string()]).is_err());
    }

    #[test]
    fn test_default_range() {
        let range = VersionRange::default_range(&SemVer::new(0, 1, 0));
        assert_eq!(range, ["0.1.0".to_string(), "+".to_string()]);
    }

    #[test]
    fn test_semver_parse_v_prefix() {
        let ver1 = SemVer::parse("v1.2.3").expect("parse v1.2.3");
        assert_eq!(ver1, SemVer::new(1, 2, 3));
        let ver2 = SemVer::parse("V2.0.0").expect("parse V2.0.0");
        assert_eq!(ver2, SemVer::new(2, 0, 0));
    }

    #[test]
    fn test_range_check_compatibility() {
        let range = VersionRange::parse_slice(&["0.1.0".to_string(), "+".to_string()])
            .expect("valid range");
        assert!(range.check_compatibility(&SemVer::new(0, 1, 0)).is_ok());
        assert!(range.check_compatibility(&SemVer::new(1, 0, 0)).is_ok());
        assert!(range.check_compatibility(&SemVer::new(0, 0, 9)).is_err());
    }

    #[test]
    fn test_range_display() {
        let range = VersionRange::parse_slice(&["0.1.0".to_string(), "+".to_string()])
            .expect("valid range");
        assert_eq!(format!("{range}"), "[\"0.1.0\", \"+\"]");
    }
}
