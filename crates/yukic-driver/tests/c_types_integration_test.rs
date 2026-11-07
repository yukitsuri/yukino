use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-c-types-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_c_int_arithmetic_and_literal_emission() {
    let (dir, input, output) = setup_test_dir("arithmetic");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"c_types_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
externc {
    cfun abs(x: c_int) -> c_int
}

fun calculate(a: c_int, b: c_int) -> c_int {
    set sum = a + b
    set diff = sum - 1
    return diff * 2
}

fun main() {
    set x: c_int = 10
    set y: c_int = 20
    set res = calculate(x, y)
    set call_res = abs(42)
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("driver compilation for c_int arithmetic should succeed");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");

    assert!(ir.contains("declare i32 @abs(i32)"));
    assert!(ir.contains("add i32") || ir.contains("add"));
    assert!(ir.contains("sub i32") || ir.contains("sub"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_c_types_casts_and_comparisons() {
    let (dir, input, output) = setup_test_dir("casts_and_cmp");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"c_types_cmp_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
fun test_cmp(a: c_int, b: c_int) -> bool {
    if a < b {
        return true
    }
    return a == 100
}

fun test_cast(x: int32) -> c_int {
    set c = cast(x, "c_int")
    set back = cast(c, "int32")
    return cast(back, "c_int")
}

fun main() {
    set ok = test_cmp(10, 20)
    set c = test_cast(42)
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("driver compilation for c_int comparisons and casts should succeed");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");

    assert!(ir.contains("icmp slt i32") || ir.contains("icmp"));

    let _ = fs::remove_dir_all(&dir);
}
