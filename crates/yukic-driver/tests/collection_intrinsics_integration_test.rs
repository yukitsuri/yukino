use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-collection-intrinsics-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_collection_intrinsics_ir_emission() {
    let (dir, input, output) = setup_test_dir("emission");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"collection_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
fun test_push_and_is_empty() {
    set arr = [1, 2, 3]
    set empty_before = arr.is_empty()
    arr.push(4)
    arr.push(5)
    set len1 = arr.len()
}

fun test_pop() {
    set arr = [10, 20]
    set last = arr.pop()
    set val = last ?? 0
    set second = arr.pop()
    set empty_pop = arr.pop()
    set is_null = empty_pop == null
}

fun test_del_and_clear() {
    set arr = [100, 200, 300]
    arr.del(1)
    set len_after_del = arr.len()
    arr.clear()
    set empty_after_clear = arr.is_empty()
}

fun test_reserve() {
    set arr = [1]
    arr.reserve(64)
    set capacity = arr.cap()
}

fun test_string_and_bytes_is_empty() {
    set s = ""
    set s_empty = s.is_empty()
    set b: bytes = cast(s, "bytes")
    set b_empty = b.is_empty()
}

fun main() {
    test_push_and_is_empty()
    test_pop()
    test_del_and_clear()
    test_reserve()
    test_string_and_bytes_is_empty()
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("driver compilation for collection intrinsics should succeed");
    assert!(output.exists(), "llvm ir file must be emitted");

    let ir = fs::read_to_string(&output).expect("read llvm ir");
    assert!(ir.contains("yukino_rt_slice_push"), "ir must contain yukino_rt_slice_push");
    assert!(ir.contains("yukino_rt_slice_del"), "ir must contain yukino_rt_slice_del");
    assert!(ir.contains("yukino_rt_slice_reserve"), "ir must contain yukino_rt_slice_reserve");
    assert!(ir.contains("pop.is_zero"), "ir must contain pop zero check");

    let _ = fs::remove_dir_all(&dir);
}
