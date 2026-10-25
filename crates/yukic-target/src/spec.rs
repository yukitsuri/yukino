use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperatingSystem { Linux, Windows, Android, }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Architecture { X86_64, Aarch64, Armv7, }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Environment { Gnu, Musl, Bionic, Msvc, }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectFormat { Elf, Coff, }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataModel { Lp64, Llp64, Ilp32, }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSpec {
    pub triple: &'static str,
    pub os: OperatingSystem,
    pub arch: Architecture,
    pub env: Environment,
    pub object_format: ObjectFormat,
    pub data_model: DataModel,
    pub data_layout: &'static str,
    pub pointer_width: u8,
    pub default_linker: &'static str,
    pub linker_emulation: Option<&'static str>,
    pub system_libraries: &'static [&'static str],
}
