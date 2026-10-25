use std::fs;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

#[test]
fn conditional_functions_are_filtered_before_codegen() {
    let dir = std::env::temp_dir().join(format!("yukic-conditional-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("conditional.ykn");
    let output = dir.join("conditional.ll");
    fs::write(
        &input,
        r#"
fun main() { space }

#[feature = "ssl"]
fun connect() { space }

#[!feature = "ssl"]
fun connect() { space }

#[target = "x86_64-unknown-linux-gnu"]
fun linux_path() { space }

#[target = "x86_64-pc-windows-gnu"]
fun windows_path() { space }
"#,
    )
    .expect("write conditional source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(
            TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string(),
        ))
        .with_features(["ssl".to_string()])
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("conditional source should compile");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");
    assert!(ir.contains("@__yukifun_connect"));
    assert!(ir.contains("@__yukifun_linux_path"));
    assert!(!ir.contains("windows_path"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn active_conditional_collision_is_rejected() {
    let dir = std::env::temp_dir().join(format!(
        "yukic-conditional-collision-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("collision.ykn");
    fs::write(
        &input,
        r#"
fun main() { space }
#[feature = "ssl"]
fun connect() { space }
#[feature = "ssl"]
fun connect() { space }
"#,
    )
    .expect("write collision source");

    let config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_features(["ssl".to_string()])
        .with_emit(EmitKind::LlvmIr);
    let error = run_compiler_driver(&config).expect_err("active collision must fail");
    assert!(error.to_string().contains("E0304"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn conditional_class_methods_are_filtered_before_codegen() {
    let dir = std::env::temp_dir().join(format!("yukic-conditional-method-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("methods.ykn");
    let output = dir.join("methods.ll");
    fs::write(
        &input,
        r#"
class Platform {
    #[target = "x86_64-unknown-linux-gnu"]
    fun run() { space }

    #[target = "x86_64-pc-windows-gnu"]
    fun run() { space }
}

fun main() { space }
"#,
    )
    .expect("write conditional source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(
            TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string(),
        ))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("conditional methods should compile");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");
    assert_eq!(
        ir.matches("define fastcc void @_Y_Platform_run(").count(),
        1
    );

    let _ = fs::remove_dir_all(dir);
}


#[test]
fn module_level_conditional_attributes_exclude_entire_file() {
    let dir = std::env::temp_dir().join(format!("yukic-module-attr-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("module_attr.ykn");
    let output = dir.join("module_attr.ll");
    fs::write(
        &input,
        r#"
!#[target = "x86_64-pc-windows-gnu"]

fun windows_only() { space }
"#,
    )
    .expect("write module attr source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(
            TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string(),
        ))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("compiler driver should succeed");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");
    assert!(!ir.contains("windows_only"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn target_array_and_arch_attributes_are_supported() {
    let dir = std::env::temp_dir().join(format!("yukic-arch-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("arch.ykn");
    let output = dir.join("arch.ll");
    fs::write(
        &input,
        r#"
fun main() { space }

#[target = ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"]]
fun posix_entry() { space }

#[arch = "x86_64"]
fun x86_only() { space }

#[arch = "arm64"]
fun arm_only() { space }

#[!arch = "arm64"]
fun non_arm() { space }
"#,
    )
    .expect("write arch source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(
            TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string(),
        ))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("arch source should compile");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");
    assert!(ir.contains("@__yukifun_posix_entry"));
    assert!(ir.contains("@__yukifun_x86_only"));
    assert!(ir.contains("@__yukifun_non_arm"));
    assert!(!ir.contains("arm_only"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn marker_attribute_and_inspect_attributes_dump() {
    let dir = std::env::temp_dir().join(format!("yukic-inspect-attr-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("inspect.ykn");
    let output = dir.join("inspect.ll");
    let inspect_out = dir.join("attributes.json");
    fs::write(
        &input,
        r"
#[stable]
fun profiled_task() { space }

fun main() { space }
",
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(
            TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string(),
        ))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());
    config.inspect_attributes = Some(inspect_out.clone());

    run_compiler_driver(&config).expect("driver should compile with inspect_attributes");
    assert!(output.exists());
    assert!(inspect_out.exists());
    let json = fs::read_to_string(&inspect_out).expect("read dumped json");
    assert!(json.contains("profiled_task"));
    assert!(json.contains("stable"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_parenthesized_attribute_syntax_and_filtering() {
    let dir = std::env::temp_dir().join(format!("yukic-paren-attr-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("paren.ykn");
    let output = dir.join("paren.ll");
    fs::write(
        &input,
        r#"
fun main() { space }

#[target("x86_64-unknown-linux-gnu")]
fun linux_call() { space }

#[target("x86_64-pc-windows-gnu")]
fun windows_call() { space }

#[arch("x86_64")]
fun x86_call() { space }

#[!arch("arm64")]
fun non_arm_call() { space }

#[arch("arm64")]
fun arm_only() { space }
"#,
    )
    .expect("write paren source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(
            TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string(),
        ))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("parenthesized attribute source should compile");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");
    assert!(ir.contains("@__yukifun_linux_call"));
    assert!(ir.contains("@__yukifun_x86_call"));
    assert!(ir.contains("@__yukifun_non_arm_call"));
    assert!(!ir.contains("windows_call"));
    assert!(!ir.contains("arm_only"));

    let _ = fs::remove_dir_all(dir);
}
