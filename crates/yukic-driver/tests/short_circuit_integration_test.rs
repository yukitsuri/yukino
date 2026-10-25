use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-shortcircuit-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_short_circuit_ir_emission() {
    let (dir, input, output) = setup_test_dir("emission");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"short_circuit_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
struct ShortCircuitError {
    message: string
}

fun check_and_short_circuit(flag: bool) -> bool {
    return flag && panic(ShortCircuitError { message: "unreachable and" })
}

fun check_or_short_circuit(flag: bool) -> bool {
    return flag || panic(ShortCircuitError { message: "unreachable or" })
}

fun check_complex(a: bool, b: bool, c: bool) -> bool {
    return (a && b) || c
}

fun main() {
    set a = check_and_short_circuit(false)
    set b = check_or_short_circuit(true)
    set c = check_complex(true, false, true)
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("driver compilation for short circuit should succeed");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");

    assert!(ir.contains("and.rhs"));
    assert!(ir.contains("and.merge"));
    assert!(ir.contains("or.rhs"));
    assert!(ir.contains("or.merge"));
    assert!(ir.contains("phi i1"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_short_circuit_runtime_execution() {
    let dir = std::env::temp_dir().join(format!("yukic-shortcircuit-exec-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create test dir");
    let input = dir.join("main.ykn");
    let app_out = dir.join("short_circuit_app");

    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"short_circuit_app\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
struct ShortCircuitError {
    message: string
}

fun main() {
    set a = false && panic(ShortCircuitError { message: "must not panic on false &&" })
    set b = true || panic(ShortCircuitError { message: "must not panic on true ||" })
    if a {
        panic(ShortCircuitError { message: "a must be false" })
    }
    if !b {
        panic(ShortCircuitError { message: "b must be true" })
    }
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned());
    config.output_file = Some(app_out.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("build executable");
    assert!(app_out.exists());

    let status = std::process::Command::new(&app_out)
        .status()
        .expect("run executable");
    assert!(status.success());

    let _ = fs::remove_dir_all(dir);
}
