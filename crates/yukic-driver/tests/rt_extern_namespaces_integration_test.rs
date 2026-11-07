use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-rt-extern-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_rt_and_extern_namespaces_compilation() {
    let (dir, input, output) = setup_test_dir("compilation");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"rt_extern_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
externc {
    cfun abs(x: int32) -> int32
}

fun main() {
    set str_num = rt::cast(123, "string")
    set t = rt::typeof!(str_num)
    set c_res = extern::abs(-42)
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("compilation for rt:: and extern:: should succeed");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");
    assert!(ir.contains("abs"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_extern_error_on_non_c_function() {
    let (dir, input, output) = setup_test_dir("error-non-c");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"rt_extern_err\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
fun my_pure_fun() -> int32 {
    return 10
}

fun main() {
    set x = extern::my_pure_fun()
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    let res = run_compiler_driver(&config);
    assert!(res.is_err(), "using extern:: on a pure Yukino function must fail");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_extern_error_on_missing_function() {
    let (dir, input, output) = setup_test_dir("error-missing");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"rt_extern_missing\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
fun main() {
    set x = extern::non_existent_func()
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    let res = run_compiler_driver(&config);
    assert!(res.is_err(), "using extern:: on non-existent function must fail");
    let _ = fs::remove_dir_all(dir);
}

