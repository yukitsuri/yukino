use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-nullable-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_nullable_operations_ir_emission() {
    let (dir, input, output) = setup_test_dir("emission");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"nullable_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
struct User {
    name: string
    age: int64
}

fun find_val(flag: bool) -> int64? {
    if flag {
        return 42
    }
    return null
}

fun check_is_null(x: int64?) -> bool {
    return x == null
}

fun check_is_not_null(x: int64?) -> bool {
    return x != null
}

fun with_default(x: int64?) -> int64 {
    return x ?? 999
}

fun get_user_name(u: User?) -> string? {
    return u?.name
}

fun main() {
    set val = find_val(true)
    set is_none = check_is_null(val)
    set is_some = check_is_not_null(val)
    set unwrapped = with_default(val)
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("driver compilation for nullable should succeed");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");

    // Verify nullable function return and wrapping
    assert!(ir.contains("find_val"));
    assert!(ir.contains("coal.rhs"));
    assert!(ir.contains("coal.merge"));
    assert!(ir.contains("is_null"));
    assert!(ir.contains("safe.call"));
    assert!(ir.contains("safe.null"));
    assert!(ir.contains("safe.merge"));

    let _ = fs::remove_dir_all(dir);
}
