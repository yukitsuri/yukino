#![deny(unsafe_code)]

use yukic_driver::{ensure_runtime_archive, find_default_runtime_archive, list_archive_members};
use yukic_target::TargetTriple;

/*
use yukic_driver::{
    EMBEDDED_TARGET_TRIPLE, EMBEDDED_YUKINO_RT, unpack_embedded_runtime,
};

#[test]
fn test_embedded_runtime_bytes_valid_ar_archive() {
    assert!(!EMBEDDED_YUKINO_RT.is_empty());
    // standard ar archive begins with !<arch>\n
    assert!(EMBEDDED_YUKINO_RT.starts_with(b"!<arch>\n"));
    assert!(!EMBEDDED_TARGET_TRIPLE.is_empty());
}

#[test]
fn test_unpack_embedded_runtime_and_idempotence() {
    let host_triple = TargetTriple::host();
    let unpacked_path = unpack_embedded_runtime(host_triple.as_str()).expect("unpack failed");

    assert!(unpacked_path.is_file());
    let metadata = fs::metadata(&unpacked_path).expect("read metadata");
    assert_eq!(metadata.len(), EMBEDDED_YUKINO_RT.len() as u64);

    // test idempotence
    let second_unpack = unpack_embedded_runtime(host_triple.as_str()).expect("second unpack failed");
    assert_eq!(unpacked_path, second_unpack);
}
*/

#[test]
fn test_ensure_runtime_archive_and_list_members() {
    let host = TargetTriple::host();
    let rt_path = ensure_runtime_archive(host).expect("ensure runtime archive");
    assert!(rt_path.is_file());

    let members = list_archive_members(&rt_path, None).expect("list members");
    assert!(members.iter().any(|m| {
        std::path::Path::new(m)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("o"))
    }));
}

#[test]
fn test_find_default_runtime_archive_integration() {
    let host = TargetTriple::host();
    let rt_path = find_default_runtime_archive(host).expect("find default runtime");
    assert!(rt_path.is_file());
    assert!(rt_path.to_string_lossy().contains("libyukino_rt.a"));
}
