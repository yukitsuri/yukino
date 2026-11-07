use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-load-syntax-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_load_module_without_colon_executes() {
    let (dir, input, _) = setup_test_dir("no_colon");
    let app_out = dir.join("load_app");

    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"load_app\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    let yukino_code = r#"
load std.io

fun main() {
    println("HELLO FROM LOAD WITHOUT COLON")
}
"#;

    fs::write(&input, yukino_code).expect("write main.ykn");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned());
    config.output_file = Some(app_out.to_string_lossy().into_owned());

    let compile_res = run_compiler_driver(&config);
    if let Err(e) = &compile_res {
        eprintln!("DRIVER ERROR: {e:?}");
    }
    compile_res.expect("driver compilation for load app should succeed");
    assert!(app_out.exists(), "binary must be emitted");

    let exec_res = std::process::Command::new(&app_out)
        .current_dir(&dir)
        .output()
        .expect("execute compiled load app");

    if !exec_res.status.success() {
        let stdout = String::from_utf8_lossy(&exec_res.stdout);
        let stderr = String::from_utf8_lossy(&exec_res.stderr);
        panic!("Execution failed with status: {:?}\nstdout: {stdout}\nstderr: {stderr}", exec_res.status);
    }

    let stdout = String::from_utf8_lossy(&exec_res.stdout);
    assert!(stdout.contains("HELLO FROM LOAD WITHOUT COLON"));
}

#[test]
fn test_load_std_whole_package_executes() {
    let (dir, input, _) = setup_test_dir("std_all");
    let app_out = dir.join("load_std_app");

    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"load_std_app\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    let yukino_code = r#"
load std

fun main() {
    println("HELLO FROM LOAD STD")
}
"#;

    fs::write(&input, yukino_code).expect("write main.ykn");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned());
    config.output_file = Some(app_out.to_string_lossy().into_owned());

    let compile_res = run_compiler_driver(&config);
    if let Err(e) = &compile_res {
        eprintln!("DRIVER ERROR: {e:?}");
    }
    compile_res.expect("driver compilation for load std app should succeed");
    assert!(app_out.exists(), "binary must be emitted");

    let exec_res = std::process::Command::new(&app_out)
        .current_dir(&dir)
        .output()
        .expect("execute compiled load std app");

    if !exec_res.status.success() {
        let stdout = String::from_utf8_lossy(&exec_res.stdout);
        let stderr = String::from_utf8_lossy(&exec_res.stderr);
        panic!("Execution failed with status: {:?}\nstdout: {stdout}\nstderr: {stderr}", exec_res.status);
    }

    let stdout = String::from_utf8_lossy(&exec_res.stdout);
    assert!(stdout.contains("HELLO FROM LOAD STD"));
}

#[test]
fn test_reexport_and_splitnames_driver() {
    let (dir, _, _) = setup_test_dir("reexport_driver");
    let lib_b = dir.join("lib_b");
    let lib_a = dir.join("lib_a");
    let app_dir = dir.join("app");
    let input = app_dir.join("src/main.ykn");
    let app_out = dir.join("app_bin");

    fs::create_dir_all(lib_b.join("src")).expect("create lib_b/src");
    fs::create_dir_all(lib_a.join("src")).expect("create lib_a/src");
    fs::create_dir_all(app_dir.join("src")).expect("create app/src");

    fs::write(
        lib_b.join("yuki.toml"),
        "[lib]\nname = \"lib_b\"\nversion = \"0.1.0\"\n",
    ).expect("write lib_b yuki.toml");
    fs::write(
        lib_b.join("src/lib.ykn"),
        "public fun hello_b() -> int32 { return 20 }\n",
    ).expect("write lib_b lib.ykn");

    fs::write(
        lib_a.join("yuki.toml"),
        "[lib]\nname = \"lib_a\"\nversion = \"0.1.0\"\n[libs.runtime]\nlib_b = { path = \"../lib_b\", reexport = true, splitnames = true }\n",
    ).expect("write lib_a yuki.toml");
    fs::write(
        lib_a.join("src/lib.ykn"),
        "public fun hello_a() -> int32 { return 10 }\n",
    ).expect("write lib_a lib.ykn");

    fs::write(
        app_dir.join("yuki.toml"),
        "[bin]\nname = \"app\"\nversion = \"0.1.0\"\n[libs.runtime]\nlib_a = { path = \"../lib_a\" }\n",
    ).expect("write app yuki.toml");

    let yukino_code = r#"
load lib_a: hello_a, hello_b

fun main() {
    set sum = hello_a() + hello_b()
}
"#;
    fs::write(&input, yukino_code).expect("write main.ykn");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned());
    let ll_out = app_out.with_extension("ll");
    config.output_file = Some(ll_out.to_string_lossy().into_owned());
    config.emit = EmitKind::LlvmIr;
    config.manifest_path = Some(app_dir.join("yuki.toml"));

    let compile_res = run_compiler_driver(&config);
    if let Err(e) = &compile_res {
        eprintln!("DRIVER ERROR: {e:?}");
    }
    compile_res.expect("driver compilation should succeed");
    assert!(ll_out.exists(), "llvm ir must be emitted");
}
