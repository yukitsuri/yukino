use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-toml-yaml-std-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_toml_yaml_std_modules_ir_emission() {
    let (dir, input, output) = setup_test_dir("emission");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"toml_yaml_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
load std.toml: toml_parse, toml_stringify, TomlValue
load std.yaml: yaml_parse, yaml_stringify, YamlValue

fun main() {
    set toml_doc = toml_parse("title = \"Yukino\"\n")
    set toml_out = toml_stringify(toml_doc)

    set yaml_doc = yaml_parse("name: Yukino\n")
    set yaml_out = yaml_stringify(yaml_doc)
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    if let Err(e) = run_compiler_driver(&config) {
        panic!("driver compilation failed: {:?}", e);
    }
    assert!(output.exists(), "llvm ir file must be emitted");

    let ir = fs::read_to_string(&output).expect("read llvm ir");
    assert!(ir.contains("toml_parse"), "ir must reference toml_parse");
    assert!(ir.contains("yaml_parse"), "ir must reference yaml_parse");

    let _ = fs::remove_dir_all(&dir);
}
