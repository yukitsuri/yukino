use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, run_compiler_driver};

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-env-std-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_env_std_execution() {
    let (dir, input, _) = setup_test_dir("exec");
    let app_out = dir.join("env_app");

    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"env_app\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    let dotenv_path = dir.join(".env");
    fs::write(
        &dotenv_path,
        r#"
# Sample test dotenv file
export TEST_HOST=localhost
TEST_PORT=8080 # service port
TEST_DEBUG=true
TEST_RATE=12.5
TEST_SINGLE='raw literal ${IGNORED}'
TEST_DOUBLE="interpolated ${TEST_HOST}:${TEST_PORT}"
TEST_FALLBACK="${NON_EXISTENT_VAR:-default_val}"
"#,
    )
    .expect("write .env file");

    let dotenv_str = dotenv_path.to_str().unwrap().replace('\\', "/");

    let yukino_code = format!(
        r#"
load std: *

struct TestError {{
    code: int32
    message: string
}}

fun fail(code: int32, msg: string) {{
    println("FAILED: " + msg)
    exit(code)
}}

fun main() {{
    println("STEP 0")
    // 1. Basic set and get
    set_env("YUKI_TEST_VAR_A", "hello_world")
    println("STEP 0.1")
    if !has_env("YUKI_TEST_VAR_A") {{ fail(1, "has should be true") }}
    println("STEP 0.2")
    if get_env("YUKI_TEST_VAR_A") != "hello_world" {{ fail(2, "get should match") }}
    println("STEP 0.3")
    println("STEP 0.4")
    if env("YUKI_TEST_VAR_A") != "hello_world" {{ fail(21, "env alias should match") }}
    println("STEP 0.5")
    if get_or("YUKI_TEST_NON_EXISTENT", "fallback") != "fallback" {{ fail(3, "get_or fallback") }}
    println("STEP 0.6")

    // 2. Typed getters
    println("STEP 2")
    set_env("YUKI_TEST_INT", "42")
    if get_int("YUKI_TEST_INT", 0) != 42 {{ fail(4, "get_int parsed") }}
    if get_int("YUKI_TEST_NON_EXISTENT", 99) != 99 {{ fail(5, "get_int default") }}

    set_env("YUKI_TEST_BOOL_1", "true")
    set_env("YUKI_TEST_BOOL_2", "no")
    if !get_bool("YUKI_TEST_BOOL_1", false) {{ fail(6, "get_bool true") }}
    if get_bool("YUKI_TEST_BOOL_2", true) {{ fail(7, "get_bool false") }}

    set_env("YUKI_TEST_FLOAT", "3.14")
    set f = get_float("YUKI_TEST_FLOAT", 0.0)
    if f < 3.13 || f > 3.15 {{ fail(8, "get_float parsed") }}

    // 3. Require
    println("STEP 3")
    set req = require("YUKI_TEST_VAR_A")
    if req != "hello_world" {{ fail(9, "require valid") }}

    // 4. Remove
    println("STEP 4")
    remove_env("YUKI_TEST_VAR_A")
    if has_env("YUKI_TEST_VAR_A") {{ fail(10, "has should be false after remove") }}
    if get_env("YUKI_TEST_VAR_A") != null {{ fail(11, "get should be null after remove") }}

    // 5. Load .env file
    println("STEP 5")
    set ok = load_env("{dotenv_str}")
    if !ok {{ fail(12, "load_env should succeed") }}

    if get_env("TEST_HOST") != "localhost" {{ fail(13, "TEST_HOST parsed") }}
    if get_int("TEST_PORT", 0) != 8080 {{ fail(14, "TEST_PORT parsed") }}
    if !get_bool("TEST_DEBUG", false) {{ fail(15, "TEST_DEBUG parsed") }}
    if get_env("TEST_SINGLE") != "raw literal $" + "{{IGNORED}}" {{ fail(16, "TEST_SINGLE literal") }}
    if get_env("TEST_DOUBLE") != "interpolated localhost:8080" {{ fail(17, "TEST_DOUBLE interpolated") }}
    if get_env("TEST_FALLBACK") != "default_val" {{ fail(18, "TEST_FALLBACK parsed") }}

    // 6. all() and all_entries()
    println("STEP 6")
    set all_vars = all()
    println("STEP 6.1")
    if all_vars.len() <= 0 {{ fail(19, "all_vars non-empty") }}
    println("STEP 6.2")
    if all_vars.get("TEST_HOST") != "localhost" {{ fail(20, "all map lookup") }}
    println("STEP 6.3")

    set entries = all_entries()
    println("STEP 6.4")
    if entries.len() <= 0 {{ fail(22, "entries non-empty") }}
    println("ALL PASSED")
}}
"#
    );

    fs::write(&input, yukino_code).expect("write main.ykn");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned());
    config.output_file = Some(app_out.to_string_lossy().into_owned());

    let compile_res = run_compiler_driver(&config);
    if let Err(e) = &compile_res {
        eprintln!("DRIVER ERROR: {e:?}");
    }
    compile_res.expect("driver compilation for env app should succeed");
    assert!(app_out.exists(), "binary must be emitted");

    let exec_res = std::process::Command::new(&app_out)
        .current_dir(&dir)
        .output()
        .expect("execute compiled env test app");

    if !exec_res.status.success() {
        let stdout = String::from_utf8_lossy(&exec_res.stdout);
        let stderr = String::from_utf8_lossy(&exec_res.stderr);
        panic!("Execution failed with status: {:?}\nstdout: {stdout}\nstderr: {stderr}", exec_res.status);
    }
}
