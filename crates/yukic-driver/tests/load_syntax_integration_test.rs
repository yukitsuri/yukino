use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, run_compiler_driver};

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
