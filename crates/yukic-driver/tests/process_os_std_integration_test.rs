use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, run_compiler_driver};

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-process-os-std-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_process_and_os_std_execution() {
    if cfg!(windows) {
        // uses std.os.unix / std.os.linux APIs, unix hosts only
        return;
    }
    let (dir, input, _) = setup_test_dir("exec");
    let app_out = dir.join("process_os_app");

    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"process_os_app\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
load std.process: *
load std.os.unix: *
load std.os.linux: *

fun fail(code: int32, msg: string) {
    panic(ProcessError { message: msg, code: code })
}

fun main() {
    // 1. Current process info & env
    set pid = current_pid()
    if pid <= 0 { fail(1, "current_pid > 0") }

    set exe = current_exe()
    if exe.len() == 0 { fail(2, "current_exe not empty") }

    set cwd = current_dir()
    if cwd.len() == 0 { fail(3, "current_dir not empty") }

    set_env_var("YUKINO_PROC_TEST", "val_123")
    set v = env_var("YUKINO_PROC_TEST") ?? ""
    if v != "val_123" { fail(4, "env_var match") }
    remove_env_var("YUKINO_PROC_TEST")
    set v_after = env_var("YUKINO_PROC_TEST")
    if v_after != null { fail(5, "env_var removed") }

    // 2. High-level run and read_output
    set out = run("echo", ["proc_ok"])
    if !out.success() || out.code() != 0 || out.stdout_string().len() == 0 {
        fail(6, "run echo")
    }

    set text = read_output("echo", ["trimmed_proc"])
    if text != "trimmed_proc" { fail(7, "read_output trimmed") }

    // 3. Command builder
    set cmd = command_new("echo")
    cmd.arg("builder_test")
    set b_out = cmd.output()
    if !b_out.success() { fail(8, "cmd.output") }

    // 4. std.os.unix
    set u_pid = getpid()
    if u_pid != pid { fail(9, "unix getpid == current_pid") }

    set u_uid = getuid()
    if u_uid < 0 || SIGKILL != 9 || SIGTERM != 15 || SIGINT != 2 {
        fail(10, "unix signal constants")
    }

    // 5. std.os.linux
    set tid = gettid()
    if tid <= 0 { fail(11, "linux gettid > 0") }
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned());
    config.output_file = Some(app_out.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("build executable");
    assert!(app_out.exists());

    let status = std::process::Command::new(&app_out)
        .status()
        .expect("run executable");
    assert!(status.success(), "binary execution must succeed");

    let _ = fs::remove_dir_all(dir);
}
