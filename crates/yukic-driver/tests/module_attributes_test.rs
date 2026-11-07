use std::fs;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn host_target() -> String {
    TargetTriple::host().as_str().to_string()
}

fn function_definition<'a>(ir: &'a str, symbol: &str) -> &'a str {
    ir.lines()
        .find(|line| line.contains("define") && line.contains(symbol))
        .unwrap_or_else(|| panic!("definition of {symbol} must be present in IR"))
}

/// Expands the attribute group ids (`#N`) of a function definition into their `attributes` lines.
fn function_attributes(ir: &str, symbol: &str) -> String {
    let def = function_definition(ir, symbol);
    let mut result = String::new();
    for word in def.split_whitespace() {
        if !word.starts_with('#') {
            continue;
        }
        let prefix = format!("attributes {word} = {{");
        for line in ir.lines().filter(|l| l.starts_with(&prefix)) {
            result.push_str(line);
        }
    }
    result
}

#[test]
fn module_level_codegen_attributes_apply_to_all_functions() {
    let dir = std::env::temp_dir().join(format!("yukic-module-cg-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("module_cg.ykn");
    let output = dir.join("module_cg.ll");
    fs::write(
        &input,
        r#"
!#[cold]

fun helper() { space }

cfun c_helper() { space }

fun main() { space }
"#,
    )
    .expect("write module attribute source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(
            host_target(),
        ))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("module attrs source should compile");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");

    // Both functions inherit `cold` from the file-level attribute.
    assert!(
        function_attributes(&ir, "@__yukifun_helper").contains("cold"),
        "file-level cold attribute must reach every function"
    );
    assert!(
        function_attributes(&ir, "@main(").contains("cold"),
        "file-level cold attribute must reach every function"
    );
    assert!(
        function_attributes(&ir, "@c_helper").contains("cold"),
        "file-level cold attribute must reach cfun definition"
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn item_level_attribute_overrides_module_level() {
    let dir = std::env::temp_dir().join(format!(
        "yukic-module-override-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("override.ykn");
    let output = dir.join("override.ll");
    fs::write(
        &input,
        r#"
!#[no_inline]

#[inline]
fun hinted() { space }

fun blocked() { space }

fun main() { space }
"#,
    )
    .expect("write override source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(
            host_target(),
        ))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("override source should compile");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");

    let hinted = function_attributes(&ir, "@__yukifun_hinted");
    assert!(
        hinted.contains("inlinehint") && !hinted.contains("noinline"),
        "item-level inline wins over module-level no_inline"
    );

    let blocked = function_attributes(&ir, "@__yukifun_blocked");
    assert!(
        blocked.contains("noinline"),
        "module-level no_inline applies when no item-level attribute is set"
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn module_level_deprecated_and_unknown_attributes_are_validated() {
    let dir = std::env::temp_dir().join(format!(
        "yukic-module-deprecated-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("deprecated.ykn");
    let output = dir.join("deprecated.ll");
    fs::write(
        &input,
        r#"
!#[deprecated = "whole module is retired"]

fun legacy_entry() { space }

fun main() { space }
"#,
    )
    .expect("write deprecated module source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(
            host_target(),
        ))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("deprecated module attr is valid");
    assert!(output.exists());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn module_level_mangle_is_rejected() {
    let dir = std::env::temp_dir().join(format!(
        "yukic-module-mangle-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("mangle.ykn");
    fs::write(
        &input,
        r#"
!#[mangle]

fun entry() { space }

fun main() { space }
"#,
    )
    .expect("write mangle module source");

    let config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(
            host_target(),
        ))
        .with_emit(EmitKind::LlvmIr);

    let error = run_compiler_driver(&config)
        .expect_err("module-level mangle must be rejected");
    assert!(error.to_string().contains("mangle"));

    let _ = fs::remove_dir_all(dir);
}
