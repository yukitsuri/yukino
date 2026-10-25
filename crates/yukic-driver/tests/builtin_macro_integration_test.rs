use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_macro_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-macros-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_builtin_macros_expansion_in_driver() {
    let (dir, input, output) = setup_macro_test_dir("basic");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"demo\"\nversion = \"0.1.0\"\n[libs.runtime]\nstd = { link = \"static\" }\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
static APP_NAME = yukienv!("bin.name");
static IS_STD = is_lib_connected!("std");
static IS_FOO = is_lib_connected!("foo");
static PROF = get_profileconfig!();
static VER = get_yukic_version!();
static TARGET = get_target!();
static FEATS: string[] = get_features!();
fun main() {
    set t = typeof!(42)
    set c = cast(100, typeof!(t))
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());
    config.profile_name = Some("release".to_string());
    config.features = vec!["ssl".to_string(), "crypto".to_string()];

    run_compiler_driver(&config).expect("compilation should succeed");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");

    assert!(ir.contains("demo"));
    assert!(ir.contains("@APP_NAME"));
    assert!(ir.contains("@IS_STD"));
    assert!(ir.contains("@IS_FOO"));
    assert!(ir.contains("@PROF"));
    assert!(ir.contains("release"));
    assert!(ir.contains("@VER"));
    assert!(ir.contains("@TARGET"));
    assert!(ir.contains("x86_64-unknown-linux-gnu"));
    assert!(ir.contains("@FEATS"));
    assert!(ir.contains("ssl"));
    assert!(ir.contains("crypto"));
    assert!(ir.contains("@VER"));
    assert!(ir.contains("@TARGET"));
    assert!(ir.contains("x86_64-unknown-linux-gnu"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_builtin_macros_cli_connected_libs_and_autodiscovery() {
    let (dir, _, output) = setup_macro_test_dir("cli_connected");
    let sub = dir.join("src");
    fs::create_dir_all(&sub).expect("create src directory");
    let src_input = sub.join("main.ykn");

    fs::write(
        dir.join("lib.toml"),
        "[lib]\nname = \"my_lib\"\nversion = \"0.2.0\"\n[targets.x86_64-unknown-linux-gnu.runtime]\nposix = { link = \"dynamic\" }\n",
    )
    .expect("write manifest");

    fs::write(
        &src_input,
        r#"
static LIB_NAME = yukienv!("lib.name");
static HAS_POSIX = is_lib_connected!("posix");
static HAS_EXTRA = is_lib_connected!("extra_lib");
fun main() { space }
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(src_input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());
    config.connected_libs.push("extra_lib".to_string());

    run_compiler_driver(&config).expect("compilation should succeed");
    let ir = fs::read_to_string(&output).expect("read generated LLVM IR");

    assert!(ir.contains("my_lib"));
    assert!(ir.contains("@LIB_NAME"));
    assert!(ir.contains("@HAS_POSIX"));
    assert!(ir.contains("@HAS_EXTRA"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_builtin_macros_scopes_and_relative_paths() {
    let (dir, _, output) = setup_macro_test_dir("scopes_and_paths");
    let src_dir = dir.join("src");
    let assets_dir = dir.join("assets");
    fs::create_dir_all(&src_dir).expect("create src dir");
    fs::create_dir_all(&assets_dir).expect("create assets dir");

    let src_input = src_dir.join("main.ykn");
    fs::write(assets_dir.join("msg.txt"), "hello from assets").expect("write asset");

    fs::write(
        dir.join("yuki.toml"),
        r#"
[bin]
name = "my_scoped_bin"
version = "3.2.1"
[dependencies]
my_dep = "1.0"
"#,
    )
    .expect("write manifest");

    fs::write(
        dir.join("yukispace.toml"),
        r#"
[workspace]
name = "my_scoped_workspace"
members = ["."]
"#,
    )
    .expect("write workspace");

    fs::write(
        &src_input,
        r#"
static BIN_NAME = yukienv!("bin.name", 1);
static WS_NAME = yukienv!("workspace.name", 2);
static HOST_WS = yukienv!("workspace.name", 3);
static HOST_BIN = yukienv!("bin.name", 4);
static ELSE_BIN = yukienv!("bin.name", 99);
static HAS_MY_DEP = is_lib_connected!("my_dep", 1);
static ASSET_TXT = include_str!("assets/msg.txt");

fun main() { space }
"#,
    )
    .expect("write src");

    let mut config = DriverConfig::new(src_input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::X86_64UnknownLinuxGnu.as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("compilation should succeed");
    let ir = fs::read_to_string(&output).expect("read LLVM IR");

    assert!(ir.contains("my_scoped_bin"));
    assert!(ir.contains("my_scoped_workspace"));
    assert!(ir.contains("hello from assets"));
    assert!(ir.contains("@BIN_NAME"));
    assert!(ir.contains("@WS_NAME"));
    assert!(ir.contains("@HOST_WS"));
    assert!(ir.contains("@HOST_BIN"));
    assert!(ir.contains("@ELSE_BIN"));
    assert!(ir.contains("@HAS_MY_DEP"));
    assert!(ir.contains("@ASSET_TXT"));

    let _ = fs::remove_dir_all(dir);
}
