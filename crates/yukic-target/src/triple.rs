use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::UnknownTargetError;
use crate::spec::{
    Architecture, DataModel, Environment, ObjectFormat, OperatingSystem, TargetSpec,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TargetTriple {
    #[default]
    X86_64UnknownLinuxGnu,
    X86_64UnknownLinuxMusl,
    Aarch64UnknownLinuxGnu,
    Aarch64UnknownLinuxMusl,
    X86_64PcWindowsGnu,
    X86_64PcWindowsMsvc,
    Aarch64LinuxAndroid,
}

static X86_64_UNKNOWN_LINUX_GNU_SPEC: TargetSpec = TargetSpec {
    triple: "x86_64-unknown-linux-gnu",
    os: OperatingSystem::Linux,
    arch: Architecture::X86_64,
    env: Environment::Gnu,
    object_format: ObjectFormat::Elf,
    data_model: DataModel::Lp64,
    data_layout: "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128",
    pointer_width: 64,
    default_linker: "ld.lld",
    linker_emulation: None,
    system_libraries: &["pthread", "m", "dl", "c"],
};

static X86_64_UNKNOWN_LINUX_MUSL_SPEC: TargetSpec = TargetSpec {
    triple: "x86_64-unknown-linux-musl",
    os: OperatingSystem::Linux,
    arch: Architecture::X86_64,
    env: Environment::Musl,
    object_format: ObjectFormat::Elf,
    data_model: DataModel::Lp64,
    data_layout: "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128",
    pointer_width: 64,
    default_linker: "ld.lld",
    linker_emulation: None,
    system_libraries: &["c", "m"],
};

static AARCH64_UNKNOWN_LINUX_GNU_SPEC: TargetSpec = TargetSpec {
    triple: "aarch64-unknown-linux-gnu",
    os: OperatingSystem::Linux,
    arch: Architecture::Aarch64,
    env: Environment::Gnu,
    object_format: ObjectFormat::Elf,
    data_model: DataModel::Lp64,
    data_layout: "e-m:e-p270:32:32-p271:32:32-p272:64:64-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128-Fn32",
    pointer_width: 64,
    default_linker: "ld.lld",
    linker_emulation: Some("aarch64linux"),
    system_libraries: &["pthread", "m", "dl", "c"],
};

static AARCH64_UNKNOWN_LINUX_MUSL_SPEC: TargetSpec = TargetSpec {
    triple: "aarch64-unknown-linux-musl",
    os: OperatingSystem::Linux,
    arch: Architecture::Aarch64,
    env: Environment::Musl,
    object_format: ObjectFormat::Elf,
    data_model: DataModel::Lp64,
    data_layout: "e-m:e-p270:32:32-p271:32:32-p272:64:64-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128-Fn32",
    pointer_width: 64,
    default_linker: "ld.lld",
    linker_emulation: Some("aarch64linux"),
    system_libraries: &["c", "m"],
};

static X86_64_PC_WINDOWS_GNU_SPEC: TargetSpec = TargetSpec {
    triple: "x86_64-pc-windows-gnu",
    os: OperatingSystem::Windows,
    arch: Architecture::X86_64,
    env: Environment::Gnu,
    object_format: ObjectFormat::Coff,
    data_model: DataModel::Llp64,
    data_layout: "e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128",
    pointer_width: 64,
    default_linker: "ld.lld",
    linker_emulation: Some("i386pep"),
    system_libraries: &[
        "ws2_32", "user32", "shell32", "advapi32", "kernel32", "msvcrt", "bcrypt",
    ],
};

static X86_64_PC_WINDOWS_MSVC_SPEC: TargetSpec = TargetSpec {
    triple: "x86_64-pc-windows-msvc",
    os: OperatingSystem::Windows,
    arch: Architecture::X86_64,
    env: Environment::Msvc,
    object_format: ObjectFormat::Coff,
    data_model: DataModel::Llp64,
    data_layout: "e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128",
    pointer_width: 64,
    default_linker: "lld-link",
    linker_emulation: None,
    system_libraries: &[
        "ws2_32", "user32", "shell32", "advapi32", "kernel32", "ntdll", "userenv",
    ],
};

static AARCH64_LINUX_ANDROID_SPEC: TargetSpec = TargetSpec {
    triple: "aarch64-linux-android",
    os: OperatingSystem::Android,
    arch: Architecture::Aarch64,
    env: Environment::Bionic,
    object_format: ObjectFormat::Elf,
    data_model: DataModel::Lp64,
    data_layout: "e-m:e-p270:32:32-p271:32:32-p272:64:64-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128-Fn32",
    pointer_width: 64,
    default_linker: "ld.lld",
    linker_emulation: Some("aarch64linux"),
    system_libraries: &["log", "c", "m", "dl"],
};

const ALL_TARGETS: [TargetTriple; 7] = [
    TargetTriple::X86_64UnknownLinuxGnu,
    TargetTriple::X86_64UnknownLinuxMusl,
    TargetTriple::Aarch64UnknownLinuxGnu,
    TargetTriple::Aarch64UnknownLinuxMusl,
    TargetTriple::X86_64PcWindowsGnu,
    TargetTriple::X86_64PcWindowsMsvc,
    TargetTriple::Aarch64LinuxAndroid,
];

impl TargetTriple {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::X86_64UnknownLinuxGnu => "x86_64-unknown-linux-gnu",
            Self::X86_64UnknownLinuxMusl => "x86_64-unknown-linux-musl",
            Self::Aarch64UnknownLinuxGnu => "aarch64-unknown-linux-gnu",
            Self::Aarch64UnknownLinuxMusl => "aarch64-unknown-linux-musl",
            Self::X86_64PcWindowsGnu => "x86_64-pc-windows-gnu",
            Self::X86_64PcWindowsMsvc => "x86_64-pc-windows-msvc",
            Self::Aarch64LinuxAndroid => "aarch64-linux-android",
        }
    }

    pub fn parse(s: &str) -> Result<Self, UnknownTargetError> {
        match s {
            "x86_64-unknown-linux-gnu" => Ok(Self::X86_64UnknownLinuxGnu),
            "x86_64-unknown-linux-musl" => Ok(Self::X86_64UnknownLinuxMusl),
            "aarch64-unknown-linux-gnu" => Ok(Self::Aarch64UnknownLinuxGnu),
            "aarch64-unknown-linux-musl" => Ok(Self::Aarch64UnknownLinuxMusl),
            "x86_64-pc-windows-gnu" => Ok(Self::X86_64PcWindowsGnu),
            "x86_64-pc-windows-msvc" => Ok(Self::X86_64PcWindowsMsvc),
            "aarch64-linux-android" => Ok(Self::Aarch64LinuxAndroid),
            other => Err(UnknownTargetError::new(other)),
        }
    }

    pub fn host() -> Self {
        #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
        {
            Self::X86_64UnknownLinuxGnu
        }
        #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "musl"))]
        {
            Self::X86_64UnknownLinuxMusl
        }
        #[cfg(all(target_arch = "aarch64", target_os = "linux", target_env = "gnu"))]
        {
            Self::Aarch64UnknownLinuxGnu
        }
        #[cfg(all(target_arch = "aarch64", target_os = "linux", target_env = "musl"))]
        {
            Self::Aarch64UnknownLinuxMusl
        }
        #[cfg(all(target_arch = "x86_64", target_os = "windows", target_env = "msvc"))]
        {
            Self::X86_64PcWindowsMsvc
        }
        #[cfg(all(target_arch = "x86_64", target_os = "windows", target_env = "gnu"))]
        {
            Self::X86_64PcWindowsGnu
        }
        #[cfg(not(any(
            all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"),
            all(target_arch = "x86_64", target_os = "linux", target_env = "musl"),
            all(target_arch = "aarch64", target_os = "linux", target_env = "gnu"),
            all(target_arch = "aarch64", target_os = "linux", target_env = "musl"),
            all(target_arch = "x86_64", target_os = "windows", target_env = "msvc"),
            all(target_arch = "x86_64", target_os = "windows", target_env = "gnu")
        )))]
        {
            Self::X86_64UnknownLinuxGnu
        }
    }

    pub fn all() -> &'static [Self] {
        &ALL_TARGETS
    }

    pub fn spec(&self) -> &'static TargetSpec {
        match self {
            Self::X86_64UnknownLinuxGnu => &X86_64_UNKNOWN_LINUX_GNU_SPEC,
            Self::X86_64UnknownLinuxMusl => &X86_64_UNKNOWN_LINUX_MUSL_SPEC,
            Self::Aarch64UnknownLinuxGnu => &AARCH64_UNKNOWN_LINUX_GNU_SPEC,
            Self::Aarch64UnknownLinuxMusl => &AARCH64_UNKNOWN_LINUX_MUSL_SPEC,
            Self::X86_64PcWindowsGnu => &X86_64_PC_WINDOWS_GNU_SPEC,
            Self::X86_64PcWindowsMsvc => &X86_64_PC_WINDOWS_MSVC_SPEC,
            Self::Aarch64LinuxAndroid => &AARCH64_LINUX_ANDROID_SPEC,
        }
    }
}

impl fmt::Display for TargetTriple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for TargetTriple {
    type Err = UnknownTargetError;
    fn from_str(s: &str) -> Result<Self, Self::Err> { Self::parse(s) }
}
