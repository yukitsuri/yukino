use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-str-eq-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_string_equality_ir_emission() {
    let (dir, input, output) = setup_test_dir("emission");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"str_eq_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
fun main() {
    set a = "hello"
    set b = "world"
    set is_same = a == b
    set is_diff = a != b
    set literal_eq = "test" == "test"
    set literal_ne = "test" != "other"
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("driver compilation for string equality should succeed");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");

    // Must declare libc memcmp
    assert!(ir.contains("declare i32 @memcmp"));
    // Must contain string length comparison and branch
    assert!(ir.contains("str_len_eq"));
    assert!(ir.contains("str_call_memcmp"));
    assert!(ir.contains("str_eq_res = phi i1"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_string_equality_branches_and_nullable() {
    let (dir, input, output) = setup_test_dir("branches");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"str_eq_branches\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
fun compare_or_fallback(s: string) -> int32 {
    if s == "expected" {
        return 1
    }
    if s == "" {
        return 0
    }
    return -1
}

fun test_nullable(opt: string?) -> bool {
    if opt == "secret" {
        return true
    }
    return opt == null
}

fun main() {
    set res1 = compare_or_fallback("hello")
    set res2 = test_nullable(null)
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("driver compilation for string branches and nullable should succeed");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");

    assert!(ir.contains("declare i32 @memcmp"));
    assert!(ir.contains("str_len_eq"));

    let _ = fs::remove_dir_all(&dir);
}
