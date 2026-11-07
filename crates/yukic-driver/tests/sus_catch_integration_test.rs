use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-sus-catch-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_sus_catch_variables_ir_emission() {
    let (dir, input, output) = setup_test_dir("emission");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"sus_catch_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
struct JsonError {
    message: string
    line: int64
    column: int64
    offset: int64
}

fun test_typed_panic() {
    panic(JsonError {
        message: "unexpected token",
        line: 12,
        column: 34,
        offset: 56,
    })
}

fun main() {
    sus {
        test_typed_panic()
    } catch err: JsonError {
        set msg = err.message
        set ln = err.line
        set col = err.column
        set off = err.offset
        set k = errkind
    } catch {
        set k2 = errkind
    }
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("driver compilation for sus catch should succeed");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");

    assert!(ir.contains("yukino_raise_typed_panic"));
    assert!(ir.contains("yukino_exception_get_kind"));
    assert!(ir.contains("yukino_exception_get_payload"));
    assert!(ir.contains("errkind"));
    assert!(!ir.contains("errmsg"));
    assert!(!ir.contains("errcode"));

    let _ = fs::remove_dir_all(&dir);
}
