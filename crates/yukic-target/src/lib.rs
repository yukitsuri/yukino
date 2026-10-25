#![deny(unsafe_code)]

pub mod binary_format;
pub mod error;
pub mod spec;
pub mod triple;

pub use binary_format::{
    BinaryCompatibilityError, BinaryFormat, detect_binary_format, validate_binary_compatibility,
};
pub use error::UnknownTargetError;
pub use spec::{Architecture, DataModel, Environment, ObjectFormat, OperatingSystem, TargetSpec};
pub use triple::TargetTriple;






#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_default_is_linux_x86_64_glibc() {
        assert_eq!(TargetTriple::default(), TargetTriple::X86_64UnknownLinuxGnu);
        assert_eq!(TargetTriple::default().as_str(), "x86_64-unknown-linux-gnu");
    }

    #[test]
    fn test_parse_all_targets() {
        let expected = [
            (
                "x86_64-unknown-linux-gnu",
                TargetTriple::X86_64UnknownLinuxGnu,
            ),
            (
                "x86_64-unknown-linux-musl",
                TargetTriple::X86_64UnknownLinuxMusl,
            ),
            (
                "aarch64-unknown-linux-gnu",
                TargetTriple::Aarch64UnknownLinuxGnu,
            ),
            (
                "aarch64-unknown-linux-musl",
                TargetTriple::Aarch64UnknownLinuxMusl,
            ),
            ("x86_64-pc-windows-gnu", TargetTriple::X86_64PcWindowsGnu),
            ("x86_64-pc-windows-msvc", TargetTriple::X86_64PcWindowsMsvc),
            ("aarch64-linux-android", TargetTriple::Aarch64LinuxAndroid),
        ];

        for (triple_str, variant) in expected {
            let parsed = TargetTriple::parse(triple_str).expect("target must parse");
            assert_eq!(parsed, variant);
            assert_eq!(parsed.as_str(), triple_str);
        }
    }

    #[test]
    fn test_unknown_target_error_code_e0703() {
        let err = TargetTriple::parse("mips-unknown-linux-gnu").unwrap_err();
        assert_eq!(err.code(), "E0703");
        assert_eq!(err.target(), "mips-unknown-linux-gnu");
        let msg = err.to_string();
        assert!(msg.contains("E0703"));
        assert!(msg.contains("unsupported_target_triple"));
        assert!(msg.contains("mips-unknown-linux-gnu"));
    }

    #[test]
    fn test_target_specs_completeness() {
        let all = TargetTriple::all();
        assert_eq!(all.len(), 7);

        for target in all {
            let spec = target.spec();
            assert_eq!(spec.triple, target.as_str());
            assert!(!spec.data_layout.is_empty());
            let expected_linker = if spec.env == Environment::Msvc {
                "lld-link"
            } else {
                "ld.lld"
            };
            assert_eq!(spec.default_linker, expected_linker);
            assert!(!spec.system_libraries.is_empty());

            // All supported target triples are 64-bit.
            assert_eq!(spec.pointer_width, 64);

            match spec.os {
                OperatingSystem::Windows => {
                    assert_eq!(spec.object_format, ObjectFormat::Coff);
                    assert_eq!(spec.data_model, DataModel::Llp64);
                }
                OperatingSystem::Linux | OperatingSystem::Android => {
                    assert_eq!(spec.object_format, ObjectFormat::Elf);
                }
            }
        }
    }

    #[test]
    fn test_host_target_resolution() {
        let host = TargetTriple::host();
        let spec = host.spec();
        assert!(!spec.data_layout.is_empty());
    }

    #[test]
    fn test_from_str_trait() {
        use std::str::FromStr;
        let target = TargetTriple::from_str("x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(target, TargetTriple::X86_64UnknownLinuxGnu);
        let err = TargetTriple::from_str("unknown-target").unwrap_err();
        assert_eq!(err.code(), "E0703");
    }
}
