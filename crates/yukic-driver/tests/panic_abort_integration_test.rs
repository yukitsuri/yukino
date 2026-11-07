use std::fs;
use std::path::PathBuf;
use std::process::Command;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-abort-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_panic_abort_ir_emission() {
    let (dir, input, output) = setup_test_dir("emission");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"panic_abort_ir_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
struct AbortError {
    message: string
    code: int32
}

fun trigger_abort() {
    panic(AbortError {
        message: "immediate abort triggered",
        code: 99,
    })
}

fun main() {
    sus {
        trigger_abort()
    } catch err: AbortError {
        // unreachable in abort mode
    }
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_panic_strategy("abort")
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("compilation for abort strategy should succeed");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");

    assert!(ir.contains("yukino_panic_abort"), "IR must call yukino_panic_abort");
    assert!(!ir.contains("yukino_personality"), "IR must not use personality function in abort mode");
    assert!(!ir.contains("yukino_raise_typed_panic"), "IR must not use typed unwind panic in abort mode");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_panic_abort_runtime_execution() {
    let dir = std::env::temp_dir().join(format!("yukic-abort-exec-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create test dir");
    let input = dir.join("main.ykn");
    let app_out = dir.join("abort_app");

    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"abort_app\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
struct AbortError {
    message: string
    code: int32
}

fun trigger_abort() {
    panic(AbortError {
        message: "immediate abort triggered",
        code: 99,
    })
}

fun main() {
    sus {
        trigger_abort()
    } catch err: AbortError {
        // unreachable in abort mode
    }
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_panic_strategy("abort");
    config.output_file = Some(app_out.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("build executable in abort mode");
    assert!(app_out.exists());

    let output = Command::new(&app_out).output().expect("execute abort application");
    assert!(!output.status.success(), "abort app must fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("fatal panic (abort): [AbortError #99] immediate abort triggered"),
        "stderr must contain abort diagnostic message, got: {stderr}"
    );

    let _ = fs::remove_dir_all(&dir);
}
