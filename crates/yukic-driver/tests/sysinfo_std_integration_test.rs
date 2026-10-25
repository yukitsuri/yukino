use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, run_compiler_driver};

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-sysinfo-std-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_sysinfo_std_execution() {
    let (dir, input, _) = setup_test_dir("exec");
    let app_out = dir.join("sysinfo_app");

    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"sysinfo_app\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
load std: *

struct TestError {
    code: int32
    message: string
}

fun fail(code: int32, msg: string) {
    panic(TestError { code: code, message: msg })
}

fun main() {
    // 1. Host and Operating System
    set os = os_name()
    if os.len() == 0 { fail(1, "os_name not empty") }

    set arch = cpu_arch()
    if arch.len() == 0 { fail(2, "cpu_arch not empty") }

    set dist = distribution_id()
    if dist.len() == 0 { fail(3, "distribution_id not empty") }

    set h = host_name()
    set kern = kernel_version()
    set up = uptime()
    set boot = boot_time()

    // 2. Users
    set user = current_user_name()
    set uid = current_user_id()
    set elevated = is_elevated()

    // 3. CPU and Load
    set logical = logical_core_count()
    if logical == 0 { fail(4, "logical cores > 0") }

    set physical = physical_core_count()
    set brand = cpu_brand()
    set freq = cpu_frequency()
    set load_avg = load_average()

    // 4. Memory and Swap
    set total_mem = total_memory()
    if total_mem == 0 { fail(5, "total_memory > 0") }

    set avail_mem = available_memory()
    set used_mem = used_memory()
    set free_mem = free_memory()
    set total_sw = total_swap()
    set free_sw = free_swap()
    set used_sw = used_swap()

    // 5. Storage and Disks
    set root_space = disk_space("/")
    if root_space != null {
        set total_sp = root_space?.total_space ?? 0
        if total_sp == 0 {
            fail(6, "root total space > 0")
        }
    }

    set disk_list = disks()
    if disk_list.len() > 0 {
        set first_disk = disk_list[0]
        if first_disk.total_space == 0 {
            fail(7, "first disk total space > 0")
        }
    }

    // 6. Battery
    set bat = battery_percent()
    set charging = is_battery_charging()

    println("Sysinfo test passed successfully: OS=" + os + ", Arch=" + arch + ", LogicalCores=${logical}")
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned());
    config.output_file = Some(app_out.to_string_lossy().into_owned());

    run_compiler_driver(&config).expect("driver compilation for sysinfo app should succeed");
    assert!(app_out.exists(), "binary must be emitted");

    let run_res = std::process::Command::new(&app_out)
        .output()
        .expect("run sysinfo app");
    assert!(
        run_res.status.success(),
        "app execution failed: status={:?}, stdout={}, stderr={}",
        run_res.status,
        String::from_utf8_lossy(&run_res.stdout),
        String::from_utf8_lossy(&run_res.stderr)
    );

    let stdout = String::from_utf8_lossy(&run_res.stdout);
    assert!(stdout.contains("Sysinfo test passed successfully"), "unexpected output: {stdout}");

    let _ = fs::remove_dir_all(&dir);
}
