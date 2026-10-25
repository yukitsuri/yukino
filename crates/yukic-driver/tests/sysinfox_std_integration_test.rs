use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, run_compiler_driver};

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-sysinfox-std-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_sysinfox_std_direct_load() {
    let (dir, input, _) = setup_test_dir("direct");
    let app_out = dir.join("sysinfox_app");

    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"sysinfox_app\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
load std.sysinfox: *
load std.io: println

struct TestError {
    code: int32
    message: string
}

fun fail(code: int32, msg: string) {
    panic(TestError { code: code, message: msg })
}

fun main() {
    set ifaces = network_interfaces()
    if ifaces.len() == 0 {
        fail(1, "network interfaces list should not be empty")
    }

    set lo = network_interface("lo")
    if lo != null {
        set is_loop = lo?.is_loopback ?? false
        if !is_loop {
            fail(2, "lo should be loopback")
        }
    }

    set sensors = thermal_sensors()

    set procs = process_list()
    if procs.len() == 0 {
        fail(3, "process list should not be empty")
    }

    set first_p = procs[0]
    set found = process_by_pid(first_p.pid)
    if found == null {
        fail(4, "process_by_pid should find process")
    }

    println("Sysinfox direct load test passed: ifaces=${ifaces.len()}, procs=${procs.len()}")
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned());
    config.output_file = Some(app_out.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("driver compilation for sysinfox app should succeed");
    assert!(app_out.exists(), "binary must be emitted");

    let run_res = std::process::Command::new(&app_out)
        .output()
        .expect("run sysinfox app");
    assert!(
        run_res.status.success(),
        "app execution failed: status={:?}, stdout={}, stderr={}",
        run_res.status,
        String::from_utf8_lossy(&run_res.stdout),
        String::from_utf8_lossy(&run_res.stderr)
    );

    let stdout = String::from_utf8_lossy(&run_res.stdout);
    assert!(
        stdout.contains("Sysinfox direct load test passed"),
        "unexpected output: {stdout}"
    );

    let _ = fs::remove_dir_all(&dir);
}
