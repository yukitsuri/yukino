use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-slicing-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_slicing_compilation_and_ir_emission() {
    let (dir, input, output) = setup_test_dir("emission");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"slicing_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
fun slice_string(greeting: string) -> string {
    set s1 = greeting[0..5]
    set s2 = greeting[1..=3]
    set s3 = greeting[2..]
    set s4 = greeting[..4]
    set s5 = greeting[..]
    return s1
}

fun slice_array(arr: int64[]) -> int64[] {
    set a1 = arr[0..2]
    set a2 = arr[1..=2]
    set a3 = arr[1..]
    set a4 = arr[..3]
    set a5 = arr[..]
    return a1
}

fun main() {
    set s = "hello world"
    set res_str = slice_string(s)
    set arr = [10, 20, 30, 40]
    set res_arr = slice_array(arr)
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("driver compilation for slicing should succeed");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");

    assert!(ir.contains("define fastcc { ptr, i64 } @__yukifun_slice_string"));
    assert!(ir.contains("define fastcc { ptr, i64, i64 } @__yukifun_slice_array"));

    let _ = fs::remove_dir_all(&dir);
}
