#![deny(unsafe_code)]

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::spec::{Architecture, ObjectFormat};
use crate::triple::TargetTriple;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryFormat {
    Elf {
        bits: u8,
        arch: Option<Architecture>,
    },
    Coff {
        bits: u8,
        arch: Option<Architecture>,
    },
    Macho {
        bits: u8,
        arch: Option<Architecture>,
    },
    Archive {
        member_format: Option<Box<BinaryFormat>>,
    },
    Unknown,
}

impl fmt::Display for BinaryFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Elf { bits, arch } => {
                let a = arch.map_or("unknown arch".to_string(), |x| format!("{x:?}").to_lowercase());
                write!(f, "Linux/Unix ELF {bits}-bit ({a})")
            }
            Self::Coff { bits, arch } => {
                let a = arch.map_or("unknown arch".to_string(), |x| format!("{x:?}").to_lowercase());
                write!(f, "Windows COFF {bits}-bit ({a})")
            }
            Self::Macho { bits, arch } => {
                let a = arch.map_or("unknown arch".to_string(), |x| format!("{x:?}").to_lowercase());
                write!(f, "macOS Mach-O {bits}-bit ({a})")
            }
            Self::Archive { member_format } => match member_format {
                Some(inner) => write!(f, "Static Archive containing {inner}"),
                None => write!(f, "Static Archive (.a / .lib)"),
            },
            Self::Unknown => write!(f, "Unknown format"),
        }
    }
}

fn parse_u16_endian(bytes: &[u8], is_be: bool) -> u16 {
    if is_be {
        u16::from_be_bytes([bytes[0], bytes[1]])
    } else {
        u16::from_le_bytes([bytes[0], bytes[1]])
    }
}

fn detect_elf(data: &[u8]) -> BinaryFormat {
    let bits = match data.get(4) {
        Some(1) => 32,
        Some(2) => 64,
        _ => return BinaryFormat::Unknown,
    };
    let is_be = matches!(data.get(5), Some(2));
    let arch = if data.len() >= 20 {
        let machine = parse_u16_endian(&data[18..20], is_be);
        match machine {
            62 => Some(Architecture::X86_64),
            183 => Some(Architecture::Aarch64),
            40 => Some(Architecture::Armv7),
            _ => None,
        }
    } else {
        None
    };
    BinaryFormat::Elf { bits, arch }
}

fn detect_coff(data: &[u8]) -> BinaryFormat {
    let machine = u16::from_le_bytes([data[0], data[1]]);
    match machine {
        0x8664 => BinaryFormat::Coff {
            bits: 64,
            arch: Some(Architecture::X86_64),
        },
        0xaa64 => BinaryFormat::Coff {
            bits: 64,
            arch: Some(Architecture::Aarch64),
        },
        0x014c => BinaryFormat::Coff {
            bits: 32,
            arch: Some(Architecture::X86_64),
        },
        0x01c0 | 0x01c2 | 0x01c4 => BinaryFormat::Coff {
            bits: 32,
            arch: Some(Architecture::Armv7),
        },
        _ => BinaryFormat::Coff { bits: 32, arch: None },
    }
}

fn detect_macho(data: &[u8]) -> BinaryFormat {
    match (data[0], data[1], data[2], data[3]) {
        (0xfe, 0xed, 0xfa, 0xce) | (0xce, 0xfa, 0xed, 0xfe) => BinaryFormat::Macho {
            bits: 32,
            arch: None,
        },
        (0xfe, 0xed, 0xfa, 0xcf) | (0xcf, 0xfa, 0xed, 0xfe) => BinaryFormat::Macho {
            bits: 64,
            arch: None,
        },
        _ => BinaryFormat::Unknown,
    }
}

fn scan_archive_member(data: &[u8]) -> Option<BinaryFormat> {
    let mut offset = 8;
    while offset + 60 <= data.len() {
        let header = &data[offset..offset + 60];
        let name = std::str::from_utf8(&header[0..16]).unwrap_or("").trim();
        let size_str = std::str::from_utf8(&header[48..58]).unwrap_or("").trim();
        let size: usize = size_str.parse().unwrap_or(0);
        offset += 60;
        let content_end = (offset + size).min(data.len());
        let is_symdef = name == "/" || name == "//" || name.starts_with("__.SYMDEF");
        if !is_symdef && offset < content_end {
            let member_fmt = detect_binary_format(&data[offset..content_end]);
            if member_fmt != BinaryFormat::Unknown {
                return Some(member_fmt);
            }
        }
        offset = (content_end + 1) & !1;
    }
    None
}

pub fn detect_binary_format(data: &[u8]) -> BinaryFormat {
    if data.len() >= 4 && data[0..4] == [0x7f, b'E', b'L', b'F'] {
        return detect_elf(data);
    }
    if data.len() >= 8 && data.starts_with(b"!<arch>\n") {
        let inner = scan_archive_member(data);
        return BinaryFormat::Archive {
            member_format: inner.map(Box::new),
        };
    }
    if data.len() >= 2 {
        let m = u16::from_le_bytes([data[0], data[1]]);
        if matches!(m, 0x8664 | 0xaa64 | 0x014c | 0x01c0 | 0x01c2 | 0x01c4) {
            return detect_coff(data);
        }
    }
    if data.len() >= 4 {
        let macho = detect_macho(data);
        if macho != BinaryFormat::Unknown {
            return macho;
        }
    }
    BinaryFormat::Unknown
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryCompatibilityError {
    NotFound(PathBuf),
    Empty(PathBuf),
    Io(PathBuf, String),
    Incompatible {
        path: PathBuf,
        target: TargetTriple,
        expected: String,
        found: String,
        help: String,
    },
}

impl fmt::Display for BinaryCompatibilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(p) => write!(f, "file not found: '{}'", p.display()),
            Self::Empty(p) => write!(f, "file is empty: '{}'", p.display()),
            Self::Io(p, e) => write!(f, "failed to read '{}': {e}", p.display()),
            Self::Incompatible {
                path,
                target,
                expected,
                found,
                help,
            } => {
                write!(
                    f,
                    "incompatible binary file '{}':\n  expected: {expected} for target '{}'\n     found: {found}\n  help: {help}",
                    path.display(),
                    target.as_str()
                )
            }
        }
    }
}

impl std::error::Error for BinaryCompatibilityError {}

fn target_format_matches(fmt: &BinaryFormat, target: TargetTriple) -> bool {
    let spec = target.spec();
    match fmt {
        BinaryFormat::Elf { arch, .. } => {
            spec.object_format == ObjectFormat::Elf && arch.is_none_or(|a| a == spec.arch)
        }
        BinaryFormat::Coff { arch, .. } => {
            spec.object_format == ObjectFormat::Coff && arch.is_none_or(|a| a == spec.arch)
        }
        BinaryFormat::Archive { member_format } => match member_format {
            Some(inner) => target_format_matches(inner, target),
            None => true,
        },
        BinaryFormat::Macho { .. } | BinaryFormat::Unknown => false,
    }
}

fn build_help_message(file_name: &str, target: TargetTriple) -> String {
    let base_name = file_name
        .strip_suffix(".o")
        .or_else(|| file_name.strip_suffix(".obj"))
        .unwrap_or(file_name);
    let spec = target.spec();
    let t_str = target.as_str();
    if spec.os == crate::spec::OperatingSystem::Windows {
        format!(
            "recompile '{file_name}' for target '{t_str}' (e.g. using 'x86_64-w64-mingw32-gcc -c {base_name}.c' or 'clang --target={t_str} -c {base_name}.c')"
        )
    } else {
        format!(
            "recompile '{file_name}' for target '{t_str}' (e.g. using 'gcc -c {base_name}.c' or 'clang --target={t_str} -c {base_name}.c')"
        )
    }
}

pub fn validate_binary_compatibility(
    path: &Path,
    target: TargetTriple,
) -> Result<(), BinaryCompatibilityError> {
    if !path.is_file() {
        return Err(BinaryCompatibilityError::NotFound(path.to_path_buf()));
    }
    let mut file = File::open(path)
        .map_err(|e| BinaryCompatibilityError::Io(path.to_path_buf(), e.to_string()))?;
    let mut buf = [0u8; 4096];
    let n = file
        .read(&mut buf)
        .map_err(|e| BinaryCompatibilityError::Io(path.to_path_buf(), e.to_string()))?;
    if n == 0 {
        return Err(BinaryCompatibilityError::Empty(path.to_path_buf()));
    }

    let detected = detect_binary_format(&buf[0..n]);
    if detected == BinaryFormat::Unknown {
        return Ok(());
    }

    if !target_format_matches(&detected, target) {
        let spec = target.spec();
        let expected_desc = match spec.object_format {
            ObjectFormat::Elf => format!("Linux/Unix ELF ({:?})", spec.arch).to_lowercase(),
            ObjectFormat::Coff => format!("Windows COFF ({:?})", spec.arch).to_lowercase(),
        };
        let file_name = path.file_name().map_or("source", |n| n.to_str().unwrap_or("source"));
        let help = build_help_message(file_name, target);
        return Err(BinaryCompatibilityError::Incompatible {
            path: path.to_path_buf(),
            target,
            expected: expected_desc,
            found: detected.to_string(),
            help,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_elf_x86_64() {
        let mut buf = [0u8; 64];
        buf[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
        buf[4] = 2; // 64-bit
        buf[5] = 1; // LE
        buf[18..20].copy_from_slice(&62u16.to_le_bytes()); // x86_64
        let fmt = detect_binary_format(&buf);
        assert_eq!(
            fmt,
            BinaryFormat::Elf {
                bits: 64,
                arch: Some(Architecture::X86_64),
            }
        );
    }

    #[test]
    fn test_detect_coff_x86_64() {
        let mut buf = [0u8; 64];
        buf[0..2].copy_from_slice(&0x8664u16.to_le_bytes());
        let fmt = detect_binary_format(&buf);
        assert_eq!(
            fmt,
            BinaryFormat::Coff {
                bits: 64,
                arch: Some(Architecture::X86_64),
            }
        );
    }

    #[test]
    fn test_target_format_matches() {
        let elf_x86_64 = BinaryFormat::Elf {
            bits: 64,
            arch: Some(Architecture::X86_64),
        };
        let coff_x86_64 = BinaryFormat::Coff {
            bits: 64,
            arch: Some(Architecture::X86_64),
        };

        assert!(target_format_matches(&elf_x86_64, TargetTriple::X86_64UnknownLinuxGnu));
        assert!(!target_format_matches(&elf_x86_64, TargetTriple::X86_64PcWindowsGnu));
        assert!(target_format_matches(&coff_x86_64, TargetTriple::X86_64PcWindowsGnu));
        assert!(!target_format_matches(&coff_x86_64, TargetTriple::X86_64UnknownLinuxGnu));
    }
}
