use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-array-alloc-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_array_alloc_syntax_and_ir_emission() {
    let (dir, input, output) = setup_test_dir("emission");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"array_alloc_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
fun test_alloc() {
    set ints: int32[100]
    set count: int64 = ints.len()
    set first: int32 = ints[0]
    rewrite ints[5] = 42

    set flags: bool[16]
    set first_flag: bool = flags[0]
}

fun main() {
    test_alloc()
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("driver compilation for array alloc should succeed");
    assert!(output.exists(), "llvm ir file must be emitted");

    let ir = fs::read_to_string(&output).expect("read llvm ir");
    assert!(ir.contains("llvm.memset"), "ir must contain memset for zero-initialization");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_array_alloc_rejects_scalar_without_eq() {
    let (dir, input, output) = setup_test_dir("scalar-no-eq");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"scalar_no_eq_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
fun main() {
    set x: int32
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    let result = run_compiler_driver(&config);
    assert!(result.is_err(), "scalar variable without '=' must fail to compile");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("aborting due to"),
        "error must abort compilation: {err_msg}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_array_alloc_rejects_unsized_slice_without_eq() {
    let (dir, input, output) = setup_test_dir("unsized-no-eq");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"unsized_no_eq_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
fun main() {
    set x: int32[]
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    let result = run_compiler_driver(&config);
    assert!(result.is_err(), "dynamic slice without length or '=' must fail to compile");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("aborting due to"),
        "error must abort compilation: {err_msg}"
    );

    let _ = fs::remove_dir_all(&dir);
}
