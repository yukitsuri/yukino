use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-iface-dispatch-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_interface_itable_dispatch_emission() {
    let (dir, input, output) = setup_test_dir("emission");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"iface_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
interface Describable {
    fun describe() -> string
}

class Dog with Describable {
    fun describe() -> string {
        return "dog"
    }
}

fun test_dispatch(d: Describable) -> string {
    return d.describe()
}

fun main() {
    set dog = init Dog()
    set d: Describable = dog
    set _s1 = d.describe()
    set _s2 = test_dispatch(init Dog())
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("driver compilation for interface dispatch should succeed");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");

    assert!(ir.contains("@_Y_ITABLE_Dog_Describable"));
    assert!(ir.contains("islot_gep"));
    assert!(ir.contains("imethod_ptr"));
    assert!(ir.contains("call fastcc { ptr, i64 }"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_interface_multi_method_and_params_dispatch() {
    let (dir, input, output) = setup_test_dir("multi_method");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"multi_iface_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
interface Calculator {
    fun add(a: int32, b: int32) -> int32
    fun sub(a: int32, b: int32) -> int32
}

class FastCalc with Calculator {
    fun add(a: int32, b: int32) -> int32 {
        return a + b
    }
    fun sub(a: int32, b: int32) -> int32 {
        return a - b
    }
}

fun compute_diff(calc: Calculator, x: int32, y: int32) -> int32 {
    return calc.sub(x, y)
}

fun main() {
    set c = init FastCalc()
    set iface_calc: Calculator = c
    set _sum = iface_calc.add(10, 20)
    set _diff = compute_diff(init FastCalc(), 50, 15)
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("driver compilation for multi-method interface dispatch should succeed");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");

    assert!(ir.contains("@_Y_ITABLE_FastCalc_Calculator"));
    assert!(ir.contains("call fastcc i32"));

    let _ = fs::remove_dir_all(&dir);
}

