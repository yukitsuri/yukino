use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-static-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_static_constants_ir_emission() {
    let (dir, input, output) = setup_test_dir("emission");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"static_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
static GLOBAL_STR = "top_level_const"
static GLOBAL_INT: int32 = -42
static GLOBAL_FLAG: bool = true

fun main() {
    static LOCAL_STR = "local_const"
    static LOCAL_INT = 100
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("compilation should succeed");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");

    assert!(ir.contains("@GLOBAL_STR"));
    assert!(ir.contains("@GLOBAL_INT"));
    assert!(ir.contains("@GLOBAL_FLAG"));
    assert!(ir.contains("constant i32 -42"));

    let _ = fs::remove_dir_all(dir);
}
