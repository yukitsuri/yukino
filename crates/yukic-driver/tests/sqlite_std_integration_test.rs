use std::fs;
use std::path::PathBuf;

use yukic_driver::{DriverConfig, EmitKind, run_compiler_driver};
use yukic_target::TargetTriple;

fn setup_test_dir(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yukic-sqlite-std-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temporary test directory");
    let input = dir.join("main.ykn");
    let output = dir.join("main.ll");
    (dir, input, output)
}

#[test]
fn test_sqlite_std_module_ir_emission() {
    let (dir, input, output) = setup_test_dir("emission");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"sqlite_std_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    fs::write(
        &input,
        r#"
load std.sqlite: open_memory, sqlite_version, sqlite_int, sqlite_string, SqliteValue

fun main() {
    set ver = sqlite_version()
    set db = open_memory()
    set _ = db.execute("CREATE TABLE test (id INTEGER, val TEXT);")
    set params: SqliteValue[] = [sqlite_int(1), sqlite_string(ver)]
    set _ = db.execute_params("INSERT INTO test VALUES (?, ?);", params)
    set row = db.query_row("SELECT val FROM test WHERE id = 1;")
    set _ = row.get("val").as_string()
    db.close()
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::host().as_str().to_string()))
        .with_emit(EmitKind::LlvmIr);
    config.output_file = Some(output.to_string_lossy().into_owned());

    if let Err(e) = run_compiler_driver(&config) {
        panic!("driver compilation failed: {:?}", e);
    }
    assert!(output.exists(), "llvm ir file must be emitted");

    let ir = fs::read_to_string(&output).expect("read llvm ir");
    assert!(ir.contains("sqlite_version"), "ir must reference sqlite_version");
    assert!(ir.contains("open_memory"), "ir must reference open_memory");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_sqlite_std_module_binary_execution() {
    let (dir, input, _) = setup_test_dir("execution");
    fs::write(
        dir.join("yuki.toml"),
        "[bin]\nname = \"sqlite_exec_test\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");

    let exe_path = dir.join("sqlite_exec_test");
    fs::write(
        &input,
        r#"
load std.sqlite: open_memory, sqlite_string, SqliteValue

struct TestError {
    msg: string
}

fun main() {
    set db = open_memory()
    set _ = db.execute("CREATE TABLE kv (k TEXT PRIMARY KEY, v TEXT);")
    set params: SqliteValue[] = [sqlite_string("lang"), sqlite_string("Yukino")]
    set _ = db.execute_params("INSERT INTO kv VALUES (?, ?);", params)
    set row = db.query_row("SELECT v FROM kv WHERE k = 'lang';")
    if row.get("v").as_string() != "Yukino" {
        panic(TestError { msg: "value mismatch" })
    }
    db.close()
}
"#,
    )
    .expect("write source");

    let mut config = DriverConfig::new(input.to_string_lossy().into_owned())
        .with_target(Some(TargetTriple::host().as_str().to_string()))
        .with_emit(EmitKind::Bin);
    config.output_file = Some(exe_path.to_string_lossy().into_owned());

    if let Err(e) = run_compiler_driver(&config) {
        panic!("driver compilation failed: {:?}", e);
    }
    assert!(exe_path.exists(), "executable binary must exist");

    let status = std::process::Command::new(&exe_path)
        .status()
        .expect("run compiled sqlite binary");
    assert!(status.success(), "compiled sqlite binary must exit successfully");

    let _ = fs::remove_dir_all(&dir);
}
