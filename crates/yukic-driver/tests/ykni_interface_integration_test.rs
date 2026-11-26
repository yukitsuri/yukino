use std::fs;
use std::path::PathBuf;
use std::process::Command;

use yukic_driver::{DriverConfig, EmitKind, OutputType, run_compiler_driver};

fn temp_test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("yukic-ykni-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temporary test directory");
    dir
}

#[test]
fn test_static_lib_ykni_prebuilt_consumption() {
    let root = temp_test_dir("static_prebuilt");
    let lib_dir = root.join("lib_src");
    let out_dir = root.join("prebuilt");
    let app_dir = root.join("consumer");
    fs::create_dir_all(&lib_dir).unwrap();
    fs::create_dir_all(&out_dir).unwrap();
    fs::create_dir_all(&app_dir).unwrap();

    fs::write(
        lib_dir.join("lib.toml"),
        "[lib]\nname = \"mathpkg\"\nversion = \"0.1.0\"\nincludes = [\"core\", \"stdlib\"]\n",
    )
    .unwrap();

    fs::write(
        lib_dir.join("lib.ykn"),
        r#"
public struct Point {
    x: int32
    y: int32
}

public enum Mode {
    Fast,
    Safe
}

public class Counter {
    public value: int32

    public fun init(v: int32) {
        rewrite this.value = v
    }

    public fun add(delta: int32) -> int32 {
        rewrite this.value = this.value + delta
        return this.value
    }
}

public fun add_def(a: int32, b: int32 = 10) -> int32 {
    return a + b
}

public fun greet(name: string) -> string {
    return "hi " + name
}

public cfun c_double(x: c_int) -> c_int {
    return x * 2
}
"#,
    )
    .unwrap();

    let archive_path = out_dir.join("libmathpkg.a");
    let ykni_path = out_dir.join("mathpkg.ykni");

    let mut lib_cfg = DriverConfig::new(lib_dir.join("lib.ykn").to_string_lossy().into_owned())
        .with_output_type(OutputType::Static);
    lib_cfg.output_file = Some(archive_path.to_string_lossy().into_owned());
    lib_cfg.manifest_path = Some(lib_dir.join("lib.toml"));

    run_compiler_driver(&lib_cfg).expect("compile mathpkg static archive + ykni");
    assert!(archive_path.is_file(), "libmathpkg.a should be emitted");
    assert!(ykni_path.is_file(), "mathpkg.ykni should be emitted alongside libmathpkg.a");

    let ykni_content = fs::read_to_string(&ykni_path).expect("read mathpkg.ykni");
    assert!(ykni_content.contains("#![ykni::version(1)]"));
    assert!(ykni_content.contains("public fun add_def("));
    assert!(ykni_content.contains("public class Counter"));
    assert!(ykni_content.contains("public struct Point"));
    assert!(ykni_content.contains("public enum Mode"));

    // Consumer builds purely from prebuilt .ykni + .a without access to lib_src
    fs::write(
        app_dir.join("yuki.toml"),
        "[bin]\nname = \"consumer\"\nversion = \"0.1.0\"\nincludes = [\"core\", \"stdlib\"]\n",
    )
    .unwrap();
    fs::write(
        app_dir.join("main.ykn"),
        r#"
load std.io
load mathpkg
load core.utils: cast

fun main() {
    set d1 = add_def(5)
    set d2 = add_def(5, 2)
    set p = Point { x: 3, y: 4 }
    set c = init Counter(10)
    set c2 = c.add(5)
    set msg = greet("yuki")
    set cd = cast(c_double(21), "int32")
    set m = Mode::Fast
    set m_ok = match_mode(m)
    println("${d1}|${d2}|${p.x + p.y}|${c2}|${msg}|${cd}|${m_ok}")
}

fun match_mode(m: Mode) -> int32 {
    switch m {
        Mode::Fast -> { return 1 }
        Mode::Safe -> { return 2 }
    }
    return 0
}
"#,
    )
    .unwrap();

    let app_bin = app_dir.join("consumer");
    let mut app_cfg = DriverConfig::new(app_dir.join("main.ykn").to_string_lossy().into_owned());
    app_cfg.output_file = Some(app_bin.to_string_lossy().into_owned());
    app_cfg.manifest_path = Some(app_dir.join("yuki.toml"));
    app_cfg.prebuilt_interfaces = vec![(None, ykni_path)];
    app_cfg.link_archives = vec![archive_path];

    run_compiler_driver(&app_cfg).expect("compile consumer using prebuilt ykni + archive");
    assert!(app_bin.is_file(), "consumer binary should exist");

    let out = Command::new(&app_bin)
        .current_dir(&app_dir)
        .output()
        .expect("run consumer binary");
    assert!(
        out.status.success(),
        "consumer exited with {:?}: stderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.trim(), "15|7|7|15|hi yuki|42|1");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_shared_lib_emits_ykni_and_metadata_symbol() {
    let root = temp_test_dir("shared_meta");
    let lib_dir = root.join("lib_src");
    let out_dir = root.join("out");
    fs::create_dir_all(&lib_dir).unwrap();
    fs::create_dir_all(&out_dir).unwrap();

    fs::write(
        lib_dir.join("lib.toml"),
        "[lib]\nname = \"plugmod\"\nversion = \"0.3.0\"\nincludes = [\"core\"]\n",
    )
    .unwrap();
    fs::write(
        lib_dir.join("lib.ykn"),
        r#"
public fun mul(a: int64, b: int64) -> int64 {
    return a * b
}

public cfun c_ping(v: c_int) -> c_int {
    return v + 1
}
"#,
    )
    .unwrap();

    let so_path = out_dir.join("libplugmod.so");
    let ll_path = out_dir.join("libplugmod.ll");
    let ykni_path = out_dir.join("plugmod.ykni");

    let mut cfg = DriverConfig::new(lib_dir.join("lib.ykn").to_string_lossy().into_owned())
        .with_output_type(OutputType::Shared);
    cfg.output_file = Some(so_path.to_string_lossy().into_owned());
    cfg.extra_emits = vec![EmitKind::LlvmIr];
    cfg.manifest_path = Some(lib_dir.join("lib.toml"));

    run_compiler_driver(&cfg).expect("compile shared library + ykni + metadata");
    assert!(so_path.is_file(), "libplugmod.so should be emitted");
    assert!(ykni_path.is_file(), "plugmod.ykni should be emitted");
    assert!(ll_path.is_file(), "libplugmod.ll should be emitted");

    let ll_text = fs::read_to_string(&ll_path).unwrap();
    assert!(
        ll_text.contains("@__yukino_metadata_v1"),
        "shared LLVM IR must define __yukino_metadata_v1"
    );
    assert!(
        ll_text.contains("plugmod"),
        "shared LLVM IR metadata must mention package name"
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_shared_lib_runtime_sharedc_yukino_calls() {
    let root = temp_test_dir("sharedc_runtime");
    let lib_dir = root.join("lib_src");
    let out_dir = root.join("out");
    let app_dir = root.join("host");
    fs::create_dir_all(&lib_dir).unwrap();
    fs::create_dir_all(&out_dir).unwrap();
    fs::create_dir_all(&app_dir).unwrap();

    fs::write(
        lib_dir.join("lib.toml"),
        "[lib]\nname = \"dynplug\"\nversion = \"0.4.0\"\nincludes = [\"core\", \"stdlib\"]\n",
    )
    .unwrap();
    fs::write(
        lib_dir.join("lib.ykn"),
        r#"
public class Worker {
    public factor: int64

    public fun init(f: int64) {
        rewrite this.factor = f
    }

    public fun scale(x: int64) -> int64 {
        return x * this.factor
    }
}

public fun make_worker_buf(f: int64) -> int64 {
    return f * 100
}

public fun mul_i64(a: int64, b: int64) -> int64 {
    return a * b
}

public fun make_greeting(who: string) -> string {
    return "hello, " + who + "!"
}

public cfun c_add32(a: c_int, b: c_int) -> c_int {
    return a + b
}
"#,
    )
    .unwrap();

    let so_path = out_dir.join("libdynplug.so");
    let ykni_path = out_dir.join("dynplug.ykni");

    let mut lib_cfg = DriverConfig::new(lib_dir.join("lib.ykn").to_string_lossy().into_owned())
        .with_output_type(OutputType::Shared);
    lib_cfg.output_file = Some(so_path.to_string_lossy().into_owned());
    lib_cfg.manifest_path = Some(lib_dir.join("lib.toml"));
    run_compiler_driver(&lib_cfg).expect("compile libdynplug.so + ykni");
    assert!(so_path.is_file());
    assert!(ykni_path.is_file());

    let so_str = so_path.to_string_lossy().into_owned();
    let ykni_str = ykni_path.to_string_lossy().into_owned();

    fs::write(
        app_dir.join("yuki.toml"),
        "[bin]\nname = \"host\"\nversion = \"0.1.0\"\nincludes = [\"core\", \"stdlib\"]\n",
    )
    .unwrap();
    fs::write(
        app_dir.join("main.ykn"),
        format!(
            r#"
load std.io
load std.sharedc
load core.mem: ptr_of
load core.utils: cast

struct FakeWorker {{
    vptr: ptr<void>
    factor: int64
}}

fun main() {{
    set lib = open("{so_path_lit}")
    set has_meta = lib.is_yukino()
    set info = lib.yukino_info()
    set pkg = info?.package_name ?? "none"
    set exps = lib.exports()
    set mul_ptr = lib.yukino_symbol_or_panic("mul_i64")
    set mul_fn = cast(mul_ptr, "ptr<fun(int64, int64) -> int64>")
    set mul_res = mul_fn(6, 7)

    set greet_ptr = lib.yukino_symbol_sig_or_panic("make_greeting", "fun(string) -> string")
    set greet_fn = cast(greet_ptr, "ptr<fun(string) -> string>")
    set greet_res = greet_fn("yukino")

    set c_ptr = lib.yukino_symbol_or_panic("c_add32")
    set c_fn = cast(c_ptr, "cfun(c_int, c_int) -> c_int")
    set c_res = cast(c_fn(cast(19, "c_int"), cast(23, "c_int")), "int32")

    set fw = FakeWorker {{ vptr: null, factor: 10 }}
    set stat_ptr = lib.method_symbol_or_panic("Worker", "scale", 1)
    set stat_fn = cast(stat_ptr, "ptr<fun(ptr<void>, int64) -> int64>")
    set stat_res = stat_fn(cast(ptr_of(fw), "ptr<void>"), 9)

    set loaded_ykni = lib.load_interface("{ykni_path_lit}")
    set mul2_ptr = lib.yukino_symbol("mul_i64")
    set mul2_fn = cast(mul2_ptr, "ptr<fun(int64, int64) -> int64>")
    set mul2_res = mul2_fn(3, 5)

    println("${{has_meta}}|${{pkg}}|${{exps.len() > 0}}|${{mul_res}}|${{greet_res}}|${{c_res}}|${{stat_res}}|${{loaded_ykni}}|${{mul2_res}}")
    lib.close()
}}
"#,
            so_path_lit = so_str.replace('\\', "\\\\").replace('"', "\\\""),
            ykni_path_lit = ykni_str.replace('\\', "\\\\").replace('"', "\\\""),
        ),
    )
    .unwrap();

    let host_bin = app_dir.join("host");
    let mut host_cfg = DriverConfig::new(app_dir.join("main.ykn").to_string_lossy().into_owned());
    host_cfg.output_file = Some(host_bin.to_string_lossy().into_owned());
    host_cfg.manifest_path = Some(app_dir.join("yuki.toml"));

    run_compiler_driver(&host_cfg).expect("compile host binary calling .so via std.sharedc");
    assert!(host_bin.is_file());

    let out = Command::new(&host_bin)
        .current_dir(&app_dir)
        .output()
        .expect("run host binary");
    assert!(
        out.status.success(),
        "host exited with {:?}: stderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.trim(), "true|dynplug|true|42|hello, yukino!|42|90|true|15");

    let _ = fs::remove_dir_all(&root);
}
