use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-cast-bool-chars-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_cast_bool_and_chars_compilation() {
    let (dir, input, output) = setup_test_dir("compilation");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"cast_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
fun main() {
    // 1. bool -> string
    set str_t = cast(true, "string")
    set str_f = cast(false, "string")

    // 2. string -> bool
    set b1 = cast("true", "bool")
    set b2 = cast("false", "bool")
    set b3 = cast("1", "bool")
    set b4 = cast("0", "bool")

    // 3. string <-> char[]
    set chars = cast("hello", "char[]")
    set back_str = cast(chars, "string")

    // 4. rt::cast namespace
    set rt_str = rt::cast(true, "string")
    set rt_chars = rt::cast("world", "char[]")
    set rt_back = rt::cast(rt_chars, "string")
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("driver compilation for bool and char[] casts should succeed");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");
    assert!(ir.contains("yukino_rt_bool_to_string"));
    assert!(ir.contains("yukino_rt_string_to_bool"));
    assert!(ir.contains("yukino_rt_string_to_chars"));
    assert!(ir.contains("yukino_rt_chars_to_string"));
    let _ = fs::remove_dir_all(dir);
}
