use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-incdec-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_increment_decrement_ir_emission() {
    let (dir, input, output) = setup_test_dir("emission");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"incdec_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
struct User {
    score: int32
}

fun test_counter() -> int32 {
    set count: int32 = 0
    rewrite count++
    rewrite ++count
    rewrite count--
    rewrite --count
    return count
}

fun test_struct() -> int32 {
    set u = User { score: 10 }
    rewrite u.score++
    rewrite u.score--
    return u.score
}

fun test_array() -> int32 {
    set arr: int32[4]
    rewrite arr[0]++
    rewrite arr[0]--
    return arr[0]
}

fun main() {
    set c = test_counter()
    set s = test_struct()
    set a = test_array()
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    let result = run_compiler_driver(&config);
    assert!(result.is_ok(), "compilation failed: {:?}", result.err());

    let ir = fs::read_to_string(&output).expect("read emitted LLVM IR");
    assert!(ir.contains("add i64") || ir.contains("add i32"), "IR must contain add for increment");
    assert!(ir.contains("sub i64") || ir.contains("sub i32"), "IR must contain sub for decrement");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_increment_without_rewrite_fails_compilation() {
    let (dir, input, output) = setup_test_dir("no-rewrite");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"incdec_bad_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
fun main() {
    set i = 0
    i++
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    let result = run_compiler_driver(&config);
    assert!(result.is_err(), "i++ without rewrite must fail");

    let _ = fs::remove_dir_all(&dir);
}
