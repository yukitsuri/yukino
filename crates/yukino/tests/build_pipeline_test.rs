#![deny(unsafe_code)]

use std::fs;
use std::path::PathBuf;

use yukino::ProjectError;
use yukino::builder::{BuildOptions, build_project, clean_project};
use yukino::config::{get_config_value_at, list_config_at, set_config_value_at};
use yukino::lockfile::{LOCKFILE_REL_PATH, Lockfile};

fn setup_temp_workspace(name: &str) -> PathBuf {
    let tmp = std::env::temp_dir().join(format!("yukino_test_{name}_{}", std::process::id()));
    if tmp.exists() {
        let _ = fs::remove_dir_all(&tmp);
    }
    fs::create_dir_all(&tmp).expect("create workspace");
    tmp
}

#[test]
fn test_build_app_with_multi_file_and_lockfile() {
    let ws = setup_temp_workspace("app_multifile");
    let src = ws.join("src");
    let sub = src.join("math");
    fs::create_dir_all(&sub).expect("create src/math");

    let manifest = r#"[bin]
name = "math_app"
namedbg = "math_app_debug"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[config.debug]
opt_level = 0
strip = false
debug_assertions = true

[features]
default = ["algebra"]
algebra = ["arithmetic"]
arithmetic = []
extra = []
"#;
    fs::write(ws.join("yuki.toml"), manifest).expect("write yuki.toml");

    let helper_src = r"public fun add_nums(a: int64, b: int64) -> int64 {
    return a + b
}
";
    fs::write(sub.join("calc.ykn"), helper_src).expect("write calc.ykn");

    let main_src = r"fun main() {
    set sum = add_nums(2, 3)
}
";
    fs::write(src.join("main.ykn"), main_src).expect("write main.ykn");

    let opts = BuildOptions {
        project_dir: ws.clone(),
        profile: "debug".to_string(),
        features: vec!["extra".to_string()],
        ..Default::default()
    };

    let artifact = build_project(&opts).expect("build should succeed");
    assert!(artifact.is_file(), "binary artifact should exist");

    let lock_path = ws.join(LOCKFILE_REL_PATH);
    assert!(lock_path.is_file(), "yuki.lock must exist");

    let lockfile = Lockfile::load(&lock_path).expect("load lock").expect("lockfile present");
    let pkg = lockfile.find_package("math_app").expect("math_app in lockfile");
    assert_eq!(pkg.name, "math_app");
    assert_eq!(pkg.kind, "app");
    assert!(pkg.checksum.starts_with("blake3:"));

    // features should include: default, algebra, arithmetic, extra
    assert!(pkg.features.contains(&"default".to_string()));
    assert!(pkg.features.contains(&"algebra".to_string()));
    assert!(pkg.features.contains(&"arithmetic".to_string()));
    assert!(pkg.features.contains(&"extra".to_string()));

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_build_static_library_and_path_dependency() {
    let ws = setup_temp_workspace("lib_and_dep");
    let lib_dir = ws.join("core_lib");
    let app_dir = ws.join("app");

    fs::create_dir_all(lib_dir.join("src")).expect("create lib/src");
    fs::create_dir_all(app_dir.join("src")).expect("create app/src");

    let lib_toml = r#"[lib]
name = "core_lib"
version = "0.1.0"
entry = "src/lib.ykn"
yukino = ["0.1.0", "+"]
output = "static"

[features]
default = []
"#;
    fs::write(lib_dir.join("lib.toml"), lib_toml).expect("write lib.toml");

    let lib_ykn = r"public cfun core_add(x: int64, y: int64) -> int64 {
    return x + y
}
";
    fs::write(lib_dir.join("src/lib.ykn"), lib_ykn).expect("write lib.ykn");

    let app_toml = r#"[bin]
name = "consumer"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[features]
default = []

[libs.runtime]
core_lib = { path = "../core_lib" }
"#;
    fs::write(app_dir.join("yuki.toml"), app_toml).expect("write app.toml");

    let main_ykn = r"externc {
    cfun core_add(x: int64, y: int64) -> int64
}

fun main() {
    set res = core_add(10, 20)
}
";
    fs::write(app_dir.join("src/main.ykn"), main_ykn).expect("write main.ykn");

    let opts = BuildOptions {
        project_dir: app_dir.clone(),
        profile: "debug".to_string(),
        ..Default::default()
    };

    let artifact = build_project(&opts).expect("build consumer app");
    assert!(artifact.is_file());

    let lock_path = app_dir.join(LOCKFILE_REL_PATH);
    assert!(lock_path.is_file());

    let lock = Lockfile::load(&lock_path).expect("load lock").expect("lock present");
    assert!(lock.find_package("consumer").is_some());
    assert!(lock.find_package("core_lib").is_some());

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_clean_pipeline() {
    let ws = setup_temp_workspace("clean_test");
    let src = ws.join("src");
    fs::create_dir_all(&src).expect("create src");

    let manifest = r#"[bin]
name = "clean_app"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[features]
default = []
"#;
    fs::write(ws.join("yuki.toml"), manifest).expect("write manifest");
    fs::write(src.join("main.ykn"), "fun main() { space }\n").expect("write main");

    let opts = BuildOptions {
        project_dir: ws.clone(),
        ..Default::default()
    };
    build_project(&opts).expect("build app");

    let local_build = ws.join("yuki-work/local");
    let lock_file = ws.join(LOCKFILE_REL_PATH);
    assert!(local_build.is_dir());
    assert!(lock_file.is_file());

    clean_project(&ws, false).expect("clean local only");
    assert!(!local_build.exists());
    assert!(lock_file.is_file());

    clean_project(&ws, true).expect("clean all");
    assert!(!lock_file.exists());

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_config_crud() {
    let ws = setup_temp_workspace("config_test");
    let cfg_path = ws.join("config.json");

    set_config_value_at(&cfg_path, "cthreads", "6").expect("set cthreads");
    set_config_value_at(&cfg_path, "verbose", "true").expect("set verbose");

    assert_eq!(get_config_value_at(&cfg_path, "cthreads"), Some("6".to_string()));
    assert_eq!(get_config_value_at(&cfg_path, "verbose"), Some("true".to_string()));

    let list = list_config_at(&cfg_path);
    assert_eq!(list.len(), 2);
    assert!(list.iter().any(|(k, v)| k == "cthreads" && v == "6"));

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_build_script_pre_build() {
    let ws = setup_temp_workspace("build_script_test");
    let src = ws.join("src");
    fs::create_dir_all(&src).expect("create src");

    let manifest = r#"[bin]
name = "script_app"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[features]
default = []
"#;
    fs::write(ws.join("yuki.toml"), manifest).expect("write manifest");
    fs::write(src.join("main.ykn"), "fun main() { space }\n").expect("write main");

    let build_ykn = r"externc {
    cfun exit(code: int32)
}

fun main() {
    exit(0)
}
";
    fs::write(ws.join("build.ykn"), build_ykn).expect("write build.ykn");

    let opts = BuildOptions {
        project_dir: ws.clone(),
        ..Default::default()
    };
    let artifact = build_project(&opts).expect("build project with build.ykn");
    assert!(artifact.is_file());

    // build executable should have been removed
    let build_bin = ws.join(if cfg!(windows) { "build.exe" } else { "build" });
    assert!(!build_bin.exists(), "build binary must be deleted after execution");

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_lib_inherits_profile_from_app_with_fallbacks() {
    let ws = setup_temp_workspace("inherit_profile_test");
    let app_dir = ws.join("app");
    let lib1_dir = ws.join("lib_matching");
    let lib2_dir = ws.join("lib_release");

    fs::create_dir_all(app_dir.join("src")).expect("create app src");
    fs::create_dir_all(lib1_dir.join("src")).expect("create lib1 src");
    fs::create_dir_all(lib2_dir.join("src")).expect("create lib2 src");

    // Lib 1 has config.custom_prof
    let lib1_toml = r#"[lib]
name = "lib_matching"
version = "0.1.0"
entry = "src/lib.ykn"
output = "static"
yukino = ["0.1.0", "+"]

[config.custom_prof]
opt_level = 2
"#;
    fs::write(lib1_dir.join("lib.toml"), lib1_toml).expect("write lib1.toml");
    fs::write(
        lib1_dir.join("src/lib.ykn"),
        "public fun f1() -> int64 { return 10 }\n",
    )
    .expect("write lib1 code");

    // Lib 2 has only config.release
    let lib2_toml = r#"[lib]
name = "lib_release"
version = "0.1.0"
entry = "src/lib.ykn"
output = "static"
yukino = ["0.1.0", "+"]

[config.release]
opt_level = 3
"#;
    fs::write(lib2_dir.join("lib.toml"), lib2_toml).expect("write lib2.toml");
    fs::write(
        lib2_dir.join("src/lib.ykn"),
        "public fun f2() -> int64 { return 20 }\n",
    )
    .expect("write lib2 code");

    // App depends on both and defines [config.custom_prof]
    let app_toml = r#"[bin]
name = "app_profile_demo"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[config.custom_prof]
opt_level = 1

[libs.runtime]
lib_matching = { path = "../lib_matching" }
lib_release = { path = "../lib_release" }
"#;
    fs::write(app_dir.join("yuki.toml"), app_toml).expect("write app.toml");
    fs::write(
        app_dir.join("src/main.ykn"),
        "fun main() { space }\n",
    )
    .expect("write app code");

    let opts = BuildOptions {
        project_dir: app_dir.clone(),
        profile: "custom_prof".to_string(),
        ..Default::default()
    };
    let artifact = build_project(&opts).expect("build with custom_prof");
    assert!(artifact.is_file());

    let lock = Lockfile::load(&app_dir.join(LOCKFILE_REL_PATH))
        .expect("load lockfile")
        .expect("lockfile exists");
    assert!(lock.package.iter().any(|p| p.name == "lib_matching"));
    assert!(lock.package.iter().any(|p| p.name == "lib_release"));

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_build_app_with_builtin_macros() {
    let ws = setup_temp_workspace("app_macros");
    let lib_dir = ws.join("dep_lib");
    let app_dir = ws.join("app");
    fs::create_dir_all(lib_dir.join("src")).expect("create lib/src");
    fs::create_dir_all(app_dir.join("src")).expect("create app/src");

    let lib_toml = r#"[lib]
name = "dep_lib"
version = "0.1.0"
entry = "src/lib.ykn"
yukino = ["0.1.0", "+"]
output = "static"
"#;
    fs::write(lib_dir.join("lib.toml"), lib_toml).expect("write lib.toml");
    fs::write(lib_dir.join("src/lib.ykn"), "public fun ping() {}\n").expect("write lib");

    let app_toml = r#"[bin]
name = "macro_app"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[config.debug]
opt_level = 0

[libs.runtime]
dep_lib = { path = "../dep_lib" }
"#;
    fs::write(app_dir.join("yuki.toml"), app_toml).expect("write yuki.toml");

    let main_src = r#"
static APP_NAME = yukienv!("bin.name");
static PROF = get_profileconfig!();
static VER = get_yukic_version!();
static TARGET = get_target!();
static HAS_DEP = is_lib_connected!("dep_lib");
static HAS_NONE = is_lib_connected!("non_existent");

fun main() {
    space
}
"#;
    fs::write(app_dir.join("src/main.ykn"), main_src).expect("write main.ykn");

    let opts = BuildOptions {
        project_dir: app_dir.clone(),
        profile: "debug".to_string(),
        ..Default::default()
    };
    let artifact = build_project(&opts).expect("build macro_app");
    assert!(artifact.is_file());
    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_dependency_static_and_system_libraries_propagation() {
    let ws = setup_temp_workspace("dep_static_propagation");
    let lib_dir = ws.join("native_dep");
    let app_dir = ws.join("app");
    fs::create_dir_all(lib_dir.join("src")).expect("create lib/src");
    fs::create_dir_all(app_dir.join("src")).expect("create app/src");

    // Dummy ar archive
    let native_a = lib_dir.join("libdummy_native.a");
    fs::write(&native_a, "!<arch>\n").expect("write dummy native.a");

    let lib_toml = r#"[lib]
name = "native_dep"
version = "0.1.0"
entry = "src/lib.ykn"
yukino = ["0.1.0", "+"]
output = "static"

[libs.static]
archives = ["./libdummy_native.a"]
system = ["m"]
"#;
    fs::write(lib_dir.join("lib.toml"), lib_toml).expect("write lib.toml");
    fs::write(lib_dir.join("src/lib.ykn"), "public fun ping() {}\n").expect("write lib");

    let app_toml = r#"[bin]
name = "app_with_native_dep"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[config.debug]
opt_level = 0

[libs.runtime]
native_dep = { path = "../native_dep" }
"#;
    fs::write(app_dir.join("yuki.toml"), app_toml).expect("write yuki.toml");
    fs::write(app_dir.join("src/main.ykn"), "fun main() { space }\n").expect("write main");

    let opts = BuildOptions {
        project_dir: app_dir.clone(),
        profile: "debug".to_string(),
        ..Default::default()
    };
    let artifact = build_project(&opts).expect("build app with native dep");
    assert!(artifact.is_file());

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_dependency_build_script_pre_build() {
    let ws = setup_temp_workspace("dep_build_script_test");
    let lib_dir = ws.join("script_lib");
    let app_dir = ws.join("app");
    fs::create_dir_all(lib_dir.join("src")).expect("create lib/src");
    fs::create_dir_all(app_dir.join("src")).expect("create app/src");

    let lib_toml = r#"[lib]
name = "script_lib"
version = "0.1.0"
entry = "src/lib.ykn"
yukino = ["0.1.0", "+"]
output = "static"
"#;
    fs::write(lib_dir.join("lib.toml"), lib_toml).expect("write lib.toml");
    fs::write(lib_dir.join("src/lib.ykn"), "public fun ping() {}\n").expect("write lib");

    let build_ykn = r"externc {
    cfun exit(code: int32)
}

fun main() {
    exit(0)
}
";
    fs::write(lib_dir.join("build.ykn"), build_ykn).expect("write lib build.ykn");

    let app_toml = r#"[bin]
name = "app_consumer"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[libs.runtime]
script_lib = { path = "../script_lib" }
"#;
    fs::write(app_dir.join("yuki.toml"), app_toml).expect("write app.toml");
    fs::write(app_dir.join("src/main.ykn"), "fun main() { space }\n").expect("write main");

    let opts = BuildOptions {
        project_dir: app_dir.clone(),
        ..Default::default()
    };
    let artifact = build_project(&opts).expect("build app with lib build.ykn");
    assert!(artifact.is_file());

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_cgu_cache_in_yukino_build() {
    let ws = setup_temp_workspace("cgu_cache_build");
    let src = ws.join("src");
    fs::create_dir_all(&src).expect("create src");

    let manifest = r#"[bin]
name = "cgu_app"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]
"#;
    fs::write(ws.join("yuki.toml"), manifest).expect("write yuki.toml");
    fs::write(src.join("main.ykn"), "fun main() { space }\n").expect("write main.ykn");

    let opts = BuildOptions {
        project_dir: ws.clone(),
        profile: "debug".to_string(),
        ..Default::default()
    };

    let target = yukic_target::TargetTriple::host();
    let cache_dir = ws
        .join("yuki-work/local/cache/cgu/debug")
        .join(target.as_str())
        .join("cgu_app");

    // 1st build: populates CGU cache
    let artifact1 = build_project(&opts).expect("1st build");
    assert!(artifact1.is_file());

    let manifest_path = cache_dir.join("cgu_manifest.json");
    assert!(manifest_path.is_file(), "cgu_manifest.json must exist in cache_dir");
    let cgu_obj_path = cache_dir.join("cgu_0.o");
    assert!(cgu_obj_path.is_file(), "cgu_0.o must exist in cache_dir");

    let manifest_content = fs::read_to_string(&manifest_path).expect("read manifest");
    assert!(manifest_content.contains("\"compiler_version\""));
    assert!(manifest_content.contains("\"cgu_0\""));
    assert!(manifest_content.contains("blake3:"));

    // 2nd build: reuses CGU cache
    let artifact2 = build_project(&opts).expect("2nd build (cached)");
    assert!(artifact2.is_file());
    assert!(manifest_path.is_file());
    assert!(cgu_obj_path.is_file());

    // 3rd build with rebuild: forces cache refresh
    let rebuild_opts = BuildOptions {
        project_dir: ws.clone(),
        profile: "debug".to_string(),
        rebuild: true,
        ..Default::default()
    };
    let artifact3 = build_project(&rebuild_opts).expect("3rd build (rebuild)");
    assert!(artifact3.is_file());
    assert!(manifest_path.is_file());
    assert!(cgu_obj_path.is_file());

    // clean_project: removes yuki-work/local including cache
    clean_project(&ws, false).expect("clean project");
    assert!(!ws.join("yuki-work/local").exists());

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_build_app_auto_unpacks_and_compiles_runtime() {
    let ws = setup_temp_workspace("auto_rt_build");
    let src = ws.join("src");
    fs::create_dir_all(&src).expect("create src");

    let manifest = r#"[bin]
name = "auto_rt_app"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]
"#;
    fs::write(ws.join("yuki.toml"), manifest).expect("write yuki.toml");
    fs::write(src.join("main.ykn"), "fun main() { space }\n").expect("write main.ykn");

    let opts = BuildOptions {
        project_dir: ws.clone(),
        profile: "debug".to_string(),
        ..Default::default()
    };
    let artifact = build_project(&opts).expect("build should succeed and compile runtime");
    assert!(artifact.is_file());

    let target = yukic_target::TargetTriple::host();
    let runtime_dir = ws.join("yuki-work/local/runtime");
    assert!(runtime_dir.join("src/lib.rs").is_file());
    assert!(runtime_dir.join("Cargo.toml").is_file());
    assert!(runtime_dir.join(target.as_str()).join("libyukino_rt.a").is_file());

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_build_app_auto_unpacks_and_compiles_stdlib() {
    let ws = setup_temp_workspace("auto_stdlib_build");
    let src = ws.join("src");
    fs::create_dir_all(&src).expect("create src");

    let manifest = r#"[bin]
name = "auto_stdlib_app"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]
"#;
    fs::write(ws.join("yuki.toml"), manifest).expect("write yuki.toml");
    let main_code = "load std.io: print\nfun main() { print(\"hi\") }\n";
    fs::write(src.join("main.ykn"), main_code).expect("write main.ykn");

    let opts = BuildOptions {
        project_dir: ws.clone(),
        profile: "debug".to_string(),
        ..Default::default()
    };
    let artifact = match build_project(&opts) {
        Ok(a) => a,
        Err(e) => panic!("build should compile stdlib failed: {e:?}"),
    };
    assert!(artifact.is_file());

    let target = yukic_target::TargetTriple::host();
    let stdlib_dir = ws.join("yuki-work/local/stdlib");
    assert!(stdlib_dir.join("lib.toml").is_file());
    assert!(stdlib_dir.join("Cargo.toml").is_file());
    assert!(stdlib_dir.join(target.as_str()).join("libstd.a").is_file());
    assert!(stdlib_dir.join(target.as_str()).join("libyukistd_core.a").is_file());

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_build_app_empty_includes() {
    let ws = setup_temp_workspace("nostd_build");
    let src = ws.join("src");
    fs::create_dir_all(&src).expect("create src");

    let manifest = r#"[bin]
name = "nostd_app"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]
includes = []
"#;
    fs::write(ws.join("yuki.toml"), manifest).expect("write yuki.toml");
    fs::write(src.join("main.ykn"), "fun main() { space }\n").expect("write main.ykn");

    let opts = BuildOptions {
        project_dir: ws.clone(),
        profile: "debug".to_string(),
        ..Default::default()
    };
    let artifact = build_project(&opts).expect("build with empty includes should succeed");
    assert!(artifact.is_file());
    assert!(!ws.join("yuki-work/local/stdlib").exists());

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_libs_build_execution_and_root_working_dir() {
    let ws = setup_temp_workspace("libs_build_test");
    let codegen_dir = ws.join("codegen_tool");
    let app_dir = ws.join("app");
    fs::create_dir_all(codegen_dir.join("src")).expect("create codegen/src");
    fs::create_dir_all(app_dir.join("src")).expect("create app/src");

    let codegen_manifest = r#"[bin]
name = "codegen_tool"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]
"#;
    fs::write(codegen_dir.join("yuki.toml"), codegen_manifest).expect("write codegen yuki.toml");

    let codegen_ykn = r#"cfun fopen(name: ptr<uint8>, mode: ptr<uint8>) -> ptr<void>;
cfun fputs(str: ptr<uint8>, f: ptr<void>) -> int32;
cfun fclose(f: ptr<void>) -> int32;

load std.io: print

fun main() {
    set f = fopen("marker.txt\0".as_ptr(), "w\0".as_ptr())
    if f != null {
        fputs("generated_from_libs_build\n\0".as_ptr(), f)
        fclose(f)
    }
    print("yuki:warning=codegen_tool executed\n")
}
"#;
    fs::write(codegen_dir.join("src/main.ykn"), codegen_ykn).expect("write codegen main.ykn");

    let app_manifest = r#"[bin]
name = "consumer_app"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[libs.build]
codegen_tool = { path = "../codegen_tool" }
"#;
    fs::write(app_dir.join("yuki.toml"), app_manifest).expect("write app yuki.toml");
    fs::write(app_dir.join("src/main.ykn"), "fun main() { space }\n").expect("write app main.ykn");

    let opts = BuildOptions {
        project_dir: app_dir.clone(),
        ..Default::default()
    };
    let artifact = build_project(&opts).expect("build app with libs.build dependency");
    assert!(artifact.is_file(), "artifact must be built");

    // marker.txt must be generated in app_dir (root directory of app)
    let marker_file = app_dir.join("marker.txt");
    assert!(marker_file.is_file(), "marker.txt must be created in app root directory");
    let content = fs::read_to_string(&marker_file).expect("read marker.txt");
    assert_eq!(content.trim(), "generated_from_libs_build");

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_libs_build_failure_propagation() {
    let ws = setup_temp_workspace("libs_build_fail_test");
    let fail_dir = ws.join("fail_tool");
    let app_dir = ws.join("app");
    fs::create_dir_all(fail_dir.join("src")).expect("create fail_tool/src");
    fs::create_dir_all(app_dir.join("src")).expect("create app/src");

    let fail_manifest = r#"[bin]
name = "fail_tool"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]
"#;
    fs::write(fail_dir.join("yuki.toml"), fail_manifest).expect("write fail yuki.toml");

    let fail_ykn = r"externc {
    cfun exit(code: int32);
}

fun main() {
    exit(1)
}
";
    fs::write(fail_dir.join("src/main.ykn"), fail_ykn).expect("write fail main.ykn");

    let app_manifest = r#"[bin]
name = "consumer_app"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[libs.build]
fail_tool = { path = "../fail_tool" }
"#;
    fs::write(app_dir.join("yuki.toml"), app_manifest).expect("write app yuki.toml");
    fs::write(app_dir.join("src/main.ykn"), "fun main() { space }\n").expect("write app main.ykn");

    let opts = BuildOptions {
        project_dir: app_dir.clone(),
        ..Default::default()
    };
    let result = build_project(&opts);
    assert!(result.is_err(), "build_project must fail when build dependency fails");
    if let Err(ProjectError::Runtime(err)) = result {
        assert!(err.contains("build dependency 'fail_tool' execution failed"));
    } else {
        panic!("unexpected error: {:?}", result);
    }

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_rebuild_recompiles_dependencies() {
    let ws = setup_temp_workspace("rebuild_deps_test");
    let app_dir = ws.join("app");
    let lib_dir = ws.join("dep_lib");

    fs::create_dir_all(app_dir.join("src")).expect("create app src");
    fs::create_dir_all(lib_dir.join("src")).expect("create lib src");

    let lib_toml = r#"[lib]
name = "dep_lib"
version = "0.1.0"
entry = "src/lib.ykn"
output = "static"
yukino = ["0.1.0", "+"]
"#;
    fs::write(lib_dir.join("lib.toml"), lib_toml).expect("write lib.toml");
    fs::write(
        lib_dir.join("src/lib.ykn"),
        "public cfun dep_val() -> int64 { return 42 }\n",
    )
    .expect("write lib.ykn");

    let app_toml = r#"[bin]
name = "consumer_app"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[libs.runtime]
dep_lib = { path = "../dep_lib" }
"#;
    fs::write(app_dir.join("yuki.toml"), app_toml).expect("write app.toml");
    let main_ykn = r"externc {
    cfun dep_val() -> int64
}
fun main() {
    set res = dep_val()
}
";
    fs::write(app_dir.join("src/main.ykn"), main_ykn).expect("write main.ykn");

    let host_triple = yukic_target::TargetTriple::host();
    let dep_archive = app_dir
        .join("yuki-work/local/build/static")
        .join(host_triple.as_str())
        .join("dep_lib")
        .join("libdep_lib.a");

    let opts1 = BuildOptions {
        project_dir: app_dir.clone(),
        ..Default::default()
    };
    let art1 = build_project(&opts1).expect("initial build");
    assert!(art1.is_file());
    assert!(dep_archive.is_file());
    let mtime1 = fs::metadata(&dep_archive).expect("mtime1").modified().expect("mtime1 valid");

    let opts2 = BuildOptions {
        project_dir: app_dir.clone(),
        rebuild: false,
        ..Default::default()
    };
    build_project(&opts2).expect("cached build");
    let mtime2 = fs::metadata(&dep_archive).expect("mtime2").modified().expect("mtime2 valid");
    assert_eq!(mtime1, mtime2, "cached build must not recompile dep archive");

    std::thread::sleep(std::time::Duration::from_millis(50));

    let opts3 = BuildOptions {
        project_dir: app_dir.clone(),
        rebuild: true,
        ..Default::default()
    };
    build_project(&opts3).expect("rebuild with rebuild=true");
    assert!(dep_archive.is_file());
    let mtime3 = fs::metadata(&dep_archive).expect("mtime3").modified().expect("mtime3 valid");
    assert_ne!(mtime2, mtime3, "rebuild=true must recompile dependency library");

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_workspace_build_and_dependency_resolution() {
    let ws = setup_temp_workspace("workspace_test");

    // 1. setup workspace with yukispace.toml
    let ws_toml = r#"[workspace]
name = "my_workspace"
members = ["libs/*", "apps/*"]
"#;
    fs::write(ws.join("yukispace.toml"), ws_toml).expect("write yukispace.toml");

    // 2. member lib: libs/core
    let core_dir = ws.join("libs/core");
    fs::create_dir_all(core_dir.join("src")).expect("create libs/core/src");
    let core_manifest = r#"[lib]
name = "core"
version = "0.1.0"
entry = "src/lib.ykn"
yukino = ["0.1.0", "+"]
output = "static"
"#;
    fs::write(core_dir.join("lib.toml"), core_manifest).expect("write core lib.toml");
    let core_ykn = r"public fun get_magic() -> int64 {
    return 42
}
";
    fs::write(core_dir.join("src/lib.ykn"), core_ykn).expect("write core lib.ykn");

    // 3. member app: apps/demo
    let demo_dir = ws.join("apps/demo");
    fs::create_dir_all(demo_dir.join("src")).expect("create apps/demo/src");
    let demo_manifest = r#"[bin]
name = "demo"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]
"#;
    fs::write(demo_dir.join("yuki.toml"), demo_manifest).expect("write demo yuki.toml");
    let demo_ykn = r"fun main() {
    space
}
";
    fs::write(demo_dir.join("src/main.ykn"), demo_ykn).expect("write demo main.ykn");

    // 4. build workspace root
    let ws_opts = BuildOptions {
        project_dir: ws.clone(),
        ..Default::default()
    };
    build_project(&ws_opts).expect("build workspace");

    // 5. external consumer depending on workspace package
    let consumer = setup_temp_workspace("workspace_consumer");
    fs::create_dir_all(consumer.join("src")).expect("create consumer src");
    let consumer_manifest = format!(
        r#"[bin]
name = "consumer"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[libs.runtime]
core = {{ path = "{}", package = "core" }}
"#,
        ws.display()
    );
    fs::write(consumer.join("yuki.toml"), consumer_manifest).expect("write consumer yuki.toml");
    let consumer_ykn = r"externc {
    cfun get_magic() -> int64
}
fun main() {
    space
}
";
    fs::write(consumer.join("src/main.ykn"), consumer_ykn).expect("write consumer main.ykn");

    let consumer_opts = BuildOptions {
        project_dir: consumer.clone(),
        ..Default::default()
    };
    let artifact = build_project(&consumer_opts).expect("build consumer");
    assert!(artifact.is_file());

    let _ = fs::remove_dir_all(ws);
    let _ = fs::remove_dir_all(consumer);
}

#[test]
fn test_build_project_with_macro_dependency() {
    let base = setup_temp_workspace("macro_dep_test");
    let macro_dir = base.join("macro_dep");
    fs::create_dir_all(macro_dir.join("src")).expect("create macro src");

    let macro_manifest = r#"[package]
name = "my_macro"
version = "0.1.0"
type = "macro"

[bin]
name = "my_macro"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]
"#;
    fs::write(macro_dir.join("yuki.toml"), macro_manifest).expect("write macro yuki.toml");
    fs::write(macro_dir.join("src/main.ykn"), "fun main() { space }\n").expect("write macro main.ykn");

    let app_dir = base.join("app");
    fs::create_dir_all(app_dir.join("src")).expect("create app src");
    let app_manifest = r#"[bin]
name = "app"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[libs.runtime]
my_macro = { path = "../macro_dep" }
"#;
    fs::write(app_dir.join("yuki.toml"), app_manifest).expect("write app yuki.toml");
    fs::write(app_dir.join("src/main.ykn"), "fun main() { space }\n").expect("write app main.ykn");

    let opts = BuildOptions {
        project_dir: app_dir.clone(),
        profile: "debug".to_string(),
        ..Default::default()
    };
    let artifact = build_project(&opts).expect("build project with macro dependency");
    assert!(artifact.is_file());

    let macro_bin_name = if cfg!(target_os = "windows") { "my_macro.exe" } else { "my_macro" };
    let macro_bin = app_dir.join("yuki-work/local/build/macros").join(macro_bin_name);
    assert!(macro_bin.is_file(), "macro binary should exist at {}", macro_bin.display());

    let lockfile_content = fs::read_to_string(app_dir.join("yuki-work/common/yuki.lock")).expect("read yuki.lock");
    assert!(lockfile_content.contains("kind = \"macro\""), "yuki.lock should record macro kind: {}", lockfile_content);

    let cached_artifact = build_project(&opts).expect("cached rebuild with macro dependency");
    assert_eq!(cached_artifact, artifact);

    let _ = fs::remove_dir_all(base);
}

#[test]
fn test_dependency_recursive_reexport() {
    let base = setup_temp_workspace("recursive_reexport");
    let lib_c = base.join("lib_c");
    let lib_b = base.join("lib_b");
    let lib_a = base.join("lib_a");
    let app_dir = base.join("app");

    fs::create_dir_all(lib_c.join("src")).expect("create lib_c src");
    fs::create_dir_all(lib_b.join("src")).expect("create lib_b src");
    fs::create_dir_all(lib_a.join("src")).expect("create lib_a src");
    fs::create_dir_all(app_dir.join("src")).expect("create app src");

    let toml_c = r#"[lib]
name = "lib_c"
version = "0.1.0"
entry = "src/lib.ykn"
output = "static"
yukino = ["0.1.0", "+"]
"#;
    fs::write(lib_c.join("yuki.toml"), toml_c).expect("write lib_c toml");
    fs::write(lib_c.join("src/lib.ykn"), "public fun fn_c() -> int32 { return 30 }\n").expect("write lib_c src");

    let toml_b = r#"[lib]
name = "lib_b"
version = "0.1.0"
entry = "src/lib.ykn"
output = "static"
yukino = ["0.1.0", "+"]

[libs.runtime]
lib_c = { path = "../lib_c", reexport = true }
"#;
    fs::write(lib_b.join("yuki.toml"), toml_b).expect("write lib_b toml");
    fs::write(lib_b.join("src/lib.ykn"), "public fun fn_b() -> int32 { return 20 }\n").expect("write lib_b src");

    let toml_a = r#"[lib]
name = "lib_a"
version = "0.1.0"
entry = "src/lib.ykn"
output = "static"
yukino = ["0.1.0", "+"]

[libs.runtime]
lib_b = { path = "../lib_b", reexport = true }
"#;
    fs::write(lib_a.join("yuki.toml"), toml_a).expect("write lib_a toml");
    fs::write(lib_a.join("src/lib.ykn"), "public fun fn_a() -> int32 { return 10 }\n").expect("write lib_a src");

    let app_toml = r#"[bin]
name = "app"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[libs.runtime]
lib_a = { path = "../lib_a" }
"#;
    fs::write(app_dir.join("yuki.toml"), app_toml).expect("write app toml");

    let app_code = r#"load lib_a: fn_a
load lib_a_lib_b: fn_b
load lib_a_lib_b_lib_c: fn_c

fun main() {
    set sum = fn_a() + fn_b() + fn_c()
}
"#;
    fs::write(app_dir.join("src/main.ykn"), app_code).expect("write app src");

    let opts = BuildOptions {
        project_dir: app_dir.clone(),
        profile: "debug".to_string(),
        ..Default::default()
    };
    let artifact = build_project(&opts).expect("build project with recursive reexport");
    assert!(artifact.is_file(), "binary must be built");

    let target = yukic_target::TargetTriple::host();
    let static_dir = app_dir.join(format!("yuki-work/local/build/static/{}", target.as_str()));
    assert!(static_dir.join("lib_a/liblib_a.a").is_file(), "lib_a must be built");
    assert!(static_dir.join("lib_a_lib_b/liblib_a_lib_b.a").is_file(), "reexported lib_a_lib_b must be built");
    assert!(static_dir.join("lib_a_lib_b_lib_c/liblib_a_lib_b_lib_c.a").is_file(), "reexported lib_a_lib_b_lib_c must be built");

    let _ = fs::remove_dir_all(base);
}

#[test]
fn test_dependency_splitnames_unified_namespace() {
    let base = setup_temp_workspace("splitnames_unified");
    let lib_c = base.join("lib_c");
    let lib_b = base.join("lib_b");
    let lib_a = base.join("lib_a");
    let app_dir = base.join("app");

    fs::create_dir_all(lib_c.join("src")).expect("create lib_c src");
    fs::create_dir_all(lib_b.join("src")).expect("create lib_b src");
    fs::create_dir_all(lib_a.join("src")).expect("create lib_a src");
    fs::create_dir_all(app_dir.join("src")).expect("create app src");

    let toml_c = r#"[lib]
name = "lib_c"
version = "0.1.0"
entry = "src/lib.ykn"
output = "static"
yukino = ["0.1.0", "+"]
"#;
    fs::write(lib_c.join("yuki.toml"), toml_c).expect("write lib_c toml");
    fs::write(lib_c.join("src/lib.ykn"), "public fun fn_c() -> int32 { return 300 }\n").expect("write lib_c src");

    let toml_b = r#"[lib]
name = "lib_b"
version = "0.1.0"
entry = "src/lib.ykn"
output = "static"
yukino = ["0.1.0", "+"]

[libs.runtime]
lib_c = { path = "../lib_c", reexport = true, splitnames = true }
"#;
    fs::write(lib_b.join("yuki.toml"), toml_b).expect("write lib_b toml");
    fs::write(lib_b.join("src/lib.ykn"), "public fun fn_b() -> int32 { return 200 }\n").expect("write lib_b src");

    let toml_a = r#"[lib]
name = "lib_a"
version = "0.1.0"
entry = "src/lib.ykn"
output = "static"
yukino = ["0.1.0", "+"]

[libs.runtime]
lib_b = { path = "../lib_b", reexport = true, splitnames = true }
"#;
    fs::write(lib_a.join("yuki.toml"), toml_a).expect("write lib_a toml");
    fs::write(lib_a.join("src/lib.ykn"), "public fun fn_a() -> int32 { return 100 }\n").expect("write lib_a src");

    let app_toml = r#"[bin]
name = "app"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[libs.runtime]
lib_a = { path = "../lib_a" }
"#;
    fs::write(app_dir.join("yuki.toml"), app_toml).expect("write app toml");

    // All symbols resolved from lib_a due to splitnames
    let app_code = r#"load lib_a: fn_a, fn_b, fn_c

fun main() {
    set total = fn_a() + fn_b() + fn_c()
}
"#;
    fs::write(app_dir.join("src/main.ykn"), app_code).expect("write app src");

    let opts = BuildOptions {
        project_dir: app_dir.clone(),
        profile: "debug".to_string(),
        ..Default::default()
    };
    let artifact = build_project(&opts).expect("build project with splitnames");
    assert!(artifact.is_file(), "binary must be built");

    let _ = fs::remove_dir_all(base);
}

#[test]
fn test_dependency_splitnames_collision_rejected() {
    let base = setup_temp_workspace("splitnames_collision");
    let lib_b = base.join("lib_b");
    let lib_a = base.join("lib_a");
    let app_dir = base.join("app");

    fs::create_dir_all(lib_b.join("src")).expect("create lib_b src");
    fs::create_dir_all(lib_a.join("src")).expect("create lib_a src");
    fs::create_dir_all(app_dir.join("src")).expect("create app src");

    let toml_b = r#"[lib]
name = "lib_b"
version = "0.1.0"
entry = "src/lib.ykn"
output = "static"
yukino = ["0.1.0", "+"]
"#;
    fs::write(lib_b.join("yuki.toml"), toml_b).expect("write lib_b toml");
    fs::write(lib_b.join("src/lib.ykn"), "public fun duplicate_symbol() -> int32 { return 1 }\n").expect("write lib_b src");

    let toml_a = r#"[lib]
name = "lib_a"
version = "0.1.0"
entry = "src/lib.ykn"
output = "static"
yukino = ["0.1.0", "+"]

[libs.runtime]
lib_b = { path = "../lib_b", reexport = true, splitnames = true }
"#;
    fs::write(lib_a.join("yuki.toml"), toml_a).expect("write lib_a toml");
    fs::write(lib_a.join("src/lib.ykn"), "public fun duplicate_symbol() -> int32 { return 2 }\n").expect("write lib_a src");

    let app_toml = r#"[bin]
name = "app"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[libs.runtime]
lib_a = { path = "../lib_a" }
"#;
    fs::write(app_dir.join("yuki.toml"), app_toml).expect("write app toml");

    let app_code = r#"load lib_a: duplicate_symbol

fun main() {
    set x = duplicate_symbol()
}
"#;
    fs::write(app_dir.join("src/main.ykn"), app_code).expect("write app src");

    let opts = BuildOptions {
        project_dir: app_dir.clone(),
        profile: "debug".to_string(),
        ..Default::default()
    };
    let result = build_project(&opts);
    assert!(result.is_err(), "splitnames collision must fail build");
    if let Err(ProjectError::Runtime(err)) = result {
        assert!(err.contains("compilation failed"), "expected compilation error: {err}");
    }

    let _ = fs::remove_dir_all(base);
}

#[test]
fn test_dependency_macro_reexport() {
    let base = setup_temp_workspace("macro_reexport");
    let macro_dir = base.join("macro_lib");
    let lib_a = base.join("lib_a");
    let app_dir = base.join("app");

    fs::create_dir_all(macro_dir.join("src")).expect("create macro src");
    fs::create_dir_all(lib_a.join("src")).expect("create lib_a src");
    fs::create_dir_all(app_dir.join("src")).expect("create app src");

    let macro_manifest = r#"[package]
name = "macro_lib"
version = "0.1.0"
type = "macro"

[bin]
name = "macro_lib"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]
"#;
    fs::write(macro_dir.join("yuki.toml"), macro_manifest).expect("write macro toml");
    fs::write(macro_dir.join("src/main.ykn"), "fun main() { space }\n").expect("write macro src");

    let lib_a_toml = r#"[lib]
name = "lib_a"
version = "0.1.0"
entry = "src/lib.ykn"
output = "static"
yukino = ["0.1.0", "+"]

[libs.runtime]
macro_lib = { path = "../macro_lib", reexport = true, splitnames = true }
"#;
    fs::write(lib_a.join("yuki.toml"), lib_a_toml).expect("write lib_a toml");
    fs::write(lib_a.join("src/lib.ykn"), "public fun hello_from_a() -> int32 { return 42 }\n").expect("write lib_a src");

    let app_toml = r#"[bin]
name = "app"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[libs.runtime]
lib_a = { path = "../lib_a" }
"#;
    fs::write(app_dir.join("yuki.toml"), app_toml).expect("write app toml");
    fs::write(app_dir.join("src/main.ykn"), "load lib_a: hello_from_a\nfun main() { set x = hello_from_a() }\n").expect("write app src");

    let opts = BuildOptions {
        project_dir: app_dir.clone(),
        profile: "debug".to_string(),
        ..Default::default()
    };
    let artifact = build_project(&opts).expect("build project with macro reexport");
    assert!(artifact.is_file(), "binary must be built");

    let macro_bin_name = if cfg!(target_os = "windows") { "lib_a_macro_lib.exe" } else { "lib_a_macro_lib" };
    let macro_bin = app_dir.join("yuki-work/local/build/macros").join(macro_bin_name);
    assert!(macro_bin.is_file(), "reexported macro binary should exist at {}", macro_bin.display());

    let _ = fs::remove_dir_all(base);
}

#[test]
fn test_all_config_profile_fields_work_end_to_end() {
    let ws = setup_temp_workspace("all_profile_fields");
    fs::create_dir_all(ws.join("src")).expect("create src");
    fs::create_dir_all(ws.join("native_lib")).expect("create native_lib");

    let manifest = r#"[bin]
name = "cfg_app"
namedbg = "cfg_app_debug"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.1", "+"]
includes = ["core"]

[config.custom]
inline_statics = true
strip_unused = true
opt_level = 2
lto = "thin"
debug_assertions = false
panic_strategy = "abort"
strip = true
function_sections = true
cpu = "generic"
features = []
reloc_model = "pic"
code_model = "small"
fast_math = true
emit = ["bin", "llvm-ir", "asm"]

[libs.static]
search_paths = ["native_lib"]
"#;
    fs::write(ws.join("yuki.toml"), manifest).expect("write yuki.toml");

    let src = r#"
load core.macro: cfg

static MAGIC: int64 = 777

#[cfg(debug_assertions)]
fun only_debug() -> int64 { return 111 }

#[cfg(not(debug_assertions))]
fun only_release_mode() -> int64 { return MAGIC }

fun calc_fp(a: float64, b: float64) -> float64 {
    return a * b + 1.0
}

fun main() {
    set v = only_release_mode()
    set _ = calc_fp(2.0, 3.0)
    if cfg!(debug_assertions) {
        set _ = 999
    }
    set _ = v
}
"#;
    fs::write(ws.join("src/main.ykn"), src).expect("write main.ykn");

    let opts = BuildOptions {
        project_dir: ws.clone(),
        profile: "custom".to_string(),
        ..Default::default()
    };
    let artifact = build_project(&opts).expect("build with custom profile");
    assert!(artifact.is_file(), "binary artifact should exist");

    let ll_path = artifact.with_extension("ll");
    let asm_path = artifact.with_extension("s");
    assert!(ll_path.is_file(), "llvm-ir output should be emitted at {}", ll_path.display());
    assert!(asm_path.is_file(), "asm output should be emitted at {}", asm_path.display());

    let ll_content = fs::read_to_string(&ll_path).expect("read emitted ll");
    assert!(!ll_content.contains("only_debug"), "debug_assertions=false must strip #[cfg(debug_assertions)]");
    assert!(ll_content.contains("i64 777"), "inline_statics=true should inline constant 777");
    assert!(!ll_content.contains("i64 999"), "cfg!(debug_assertions) should evaluate to false");

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_workspace_default_members_and_build_inheritance() {
    let ws = setup_temp_workspace("ws_default_members");
    let lib_a = ws.join("libs/lib_a");
    let lib_b = ws.join("libs/lib_b");
    let search_dir = ws.join("vendor/libs");
    fs::create_dir_all(lib_a.join("lib_a")).expect("create lib_a");
    fs::create_dir_all(lib_b.join("lib_b")).expect("create lib_b");
    fs::create_dir_all(&search_dir).expect("create search_dir");

    let ws_toml = r#"[workspace]
name = "my_ws"
members = ["libs/*"]
default_members = ["libs/lib_a"]

[workspace.build]
search_paths = ["vendor/libs"]
"#;
    fs::write(ws.join("yukispace.toml"), ws_toml).expect("write yukispace.toml");

    fs::write(
        lib_a.join("lib.toml"),
        "[lib]\nname = \"lib_a\"\nversion = \"0.1.0\"\nentry = \"lib_a/lib.ykn\"\nyukino = [\"0.1.1\", \"+\"]\nincludes = []\n",
    )
    .expect("write lib_a toml");
    fs::write(lib_a.join("lib_a/lib.ykn"), "public fun a_fn() -> int64 { return 1 }\n").expect("write lib_a src");

    fs::write(
        lib_b.join("lib.toml"),
        "[lib]\nname = \"lib_b\"\nversion = \"0.1.0\"\nentry = \"lib_b/lib.ykn\"\nyukino = [\"0.1.1\", \"+\"]\nincludes = []\n",
    )
    .expect("write lib_b toml");
    fs::write(lib_b.join("lib_b/lib.ykn"), "public fun b_fn() -> int64 { return 2 }\n").expect("write lib_b src");

    let opts = BuildOptions {
        project_dir: ws.clone(),
        profile: "debug".to_string(),
        ..Default::default()
    };
    let artifact = build_project(&opts).expect("build workspace");
    assert!(artifact.is_file(), "default member lib_a artifact must be built");
    assert!(lib_a.join("yuki-work/local/build").exists(), "lib_a should be built");
    assert!(!lib_b.join("yuki-work/local/build").exists(), "lib_b should NOT be built when excluded by default_members");

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_build_with_mfpath_and_package_priority() {
    let ws = setup_temp_workspace("mfpath_pkg_priority");
    let app_dir = ws.join("apps/cli");
    let lib_dir = ws.join("libs/utils");
    fs::create_dir_all(app_dir.join("src")).expect("create app src");
    fs::create_dir_all(lib_dir.join("src")).expect("create lib src");

    fs::write(
        ws.join("yukispace.toml"),
        "[workspace]\nname = \"mf_ws\"\nmembers = [\"libs/*\", \"apps/*\"]\n",
    )
    .expect("write ws");
    fs::write(
        lib_dir.join("lib.toml"),
        "[lib]\nname = \"helper_lib\"\nversion = \"0.1.0\"\nentry = \"src/lib.ykn\"\nyukino = [\"0.1.1\", \"+\"]\nincludes = []\n",
    )
    .expect("write lib.toml");
    fs::write(lib_dir.join("src/lib.ykn"), "public fun id(x: int64) -> int64 { return x }\n")
        .expect("write lib.ykn");

    fs::write(
        app_dir.join("yuki.toml"),
        "[bin]\nname = \"cli_app\"\nversion = \"0.1.0\"\nentry = \"src/main.ykn\"\nyukino = [\"0.1.1\", \"+\"]\nincludes = []\n",
    )
    .expect("write yuki.toml");
    fs::write(app_dir.join("src/main.ykn"), "fun main() { space }\n").expect("write main.ykn");

    // 1. Build standalone manifest from outside directory using manifest_path (--mfpath)
    let mf_opts = BuildOptions {
        project_dir: std::env::temp_dir(),
        manifest_path: Some(app_dir.join("yuki.toml")),
        profile: "debug".to_string(),
        ..Default::default()
    };
    let cli_artifact = build_project(&mf_opts).expect("build via --mfpath");
    assert!(cli_artifact.is_file(), "artifact built via --mfpath must exist");

    // 2. Build workspace via --mfpath yukispace.toml targeting package by name (differs from folder name)
    let ws_pkg_opts = BuildOptions {
        project_dir: std::env::temp_dir(),
        manifest_path: Some(ws.join("yukispace.toml")),
        package: Some("helper_lib".to_string()),
        profile: "debug".to_string(),
        ..Default::default()
    };
    let lib_artifact = build_project(&ws_pkg_opts).expect("build workspace package by name via --mfpath");
    assert!(lib_artifact.is_file(), "lib artifact built by name must exist");

    // 3. Fallback to relative path "libs/utils" inside workspace
    let ws_rel_opts = BuildOptions {
        project_dir: ws.clone(),
        package: Some("libs/utils".to_string()),
        profile: "debug".to_string(),
        ..Default::default()
    };
    let rel_artifact = build_project(&ws_rel_opts).expect("build workspace package by relative path");
    assert!(rel_artifact.is_file(), "lib artifact built by relative path must exist");

    // 4. Unknown package errors explicitly
    let err_opts = BuildOptions {
        project_dir: ws.clone(),
        package: Some("nonexistent_pkg".to_string()),
        profile: "debug".to_string(),
        ..Default::default()
    };
    assert!(build_project(&err_opts).is_err(), "unknown package must fail");

    let _ = fs::remove_dir_all(ws);
}

#[test]
fn test_cload_c_header_end_to_end() {
    let cc = if std::process::Command::new("cc").arg("--version").output().is_ok() {
        "cc"
    } else if std::process::Command::new("clang").arg("--version").output().is_ok() {
        "clang"
    } else {
        return;
    };

    let base = setup_temp_workspace("cload_e2e");
    fs::create_dir_all(base.join("src")).expect("create src");
    fs::create_dir_all(base.join("native/include")).expect("create native/include");

    let sub_h = r#"
#pragma once
#include <stdint.h>

#define SUB_MAGIC 7
enum Kind {
    KIND_NONE = 0,
    KIND_OK = 3
};

struct Vec3 {
    int32_t elems[3];
};
"#;
    let main_h = r#"
#pragma once
#include "sub_types.h"

union Packet {
    int32_t as_int;
    uint8_t bytes[4];
};

extern int c_global_seed;
int c_sum_vec(struct Vec3 v);
int c_sum_var(int count, ...);
int c_apply(int (*cb)(int), int val);
int c_read_packet(union Packet p);
"#;
    let native_c = r#"
#include "include/main_api.h"
#include <stdarg.h>

int c_global_seed = 100;

int c_sum_vec(struct Vec3 v) {
    return v.elems[0] + v.elems[1] + v.elems[2];
}

int c_sum_var(int count, ...) {
    va_list ap;
    va_start(ap, count);
    int total = 0;
    for (int i = 0; i < count; i++) {
        total += va_arg(ap, int);
    }
    va_end(ap);
    return total;
}

int c_apply(int (*cb)(int), int val) {
    return cb(val);
}

int c_read_packet(union Packet p) {
    return p.as_int;
}
"#;
    fs::write(base.join("native/include/sub_types.h"), sub_h).expect("write sub_types.h");
    fs::write(base.join("native/include/main_api.h"), main_h).expect("write main_api.h");
    fs::write(base.join("native/native.c"), native_c).expect("write native.c");

    let obj_out = base.join("native/native.o");
    let cc_status = std::process::Command::new(cc)
        .args([
            "-c",
            base.join("native/native.c").to_str().unwrap(),
            "-o",
            obj_out.to_str().unwrap(),
        ])
        .status()
        .expect("run C compiler");
    assert!(cc_status.success(), "C compilation must succeed");

    let manifest = r#"[bin]
name = "cload_app"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.1", "+"]
includes = []

[libs.static]
objects = ["native/native.o"]
"#;
    fs::write(base.join("yuki.toml"), manifest).expect("write yuki.toml");

    // cload path resolves strictly relative to manifest root (`native/include/main_api.h`), NOT src/.
    let main_ykn = r#"
cload "native/include/main_api.h"

cfun exit(code: c_int) -> void;

cfun double_it(x: c_int) -> c_int {
    return x * 2
}

fun main() {
    set v = Vec3 { elems: [10, 20, 30] }
    set p = main_api::Packet { as_int: 42 }
    set s1 = c_sum_vec(v)
    set s2 = main_api::c_sum_var(3, 1, 2, 3)
    set s3 = c_apply(double_it, 5)
    set s4 = c_read_packet(p)
    if s1 != 60 { exit(11) }
    if s2 != 6 { exit(12) }
    if s3 != 10 { exit(13) }
    if s4 != 42 { exit(14) }
    if c_global_seed != 100 { exit(15) }
    if SUB_MAGIC != 7 { exit(16) }
    if KIND_OK != 3 { exit(17) }
    set total = s1 + s2 + s3 + s4 + c_global_seed + SUB_MAGIC + KIND_OK
    if total != 228 {
        exit(1)
    }
}
"#;
    fs::write(base.join("src/main.ykn"), main_ykn).expect("write main.ykn");

    let opts = BuildOptions {
        project_dir: base.clone(),
        profile: "debug".to_string(),
        ..Default::default()
    };
    let exe = build_project(&opts).expect("build project using cload header");
    assert!(exe.is_file(), "output binary should exist");

    let run_out = std::process::Command::new(&exe)
        .output()
        .expect("run built executable");
    assert!(
        run_out.status.success(),
        "cload executable exited with {:?}: stdout={} stderr={}",
        run_out.status,
        String::from_utf8_lossy(&run_out.stdout),
        String::from_utf8_lossy(&run_out.stderr)
    );

    let _ = fs::remove_dir_all(base);
}

#[test]
fn test_build_script_manifest_directives_restore_on_success_and_failure() {
    let ws = setup_temp_workspace("build_script_manifest_overrides");
    let src = ws.join("src");
    fs::create_dir_all(&src).expect("create src");

    let orig_manifest = concat!(
        "# user manifest comment preserved\n",
        "[bin]\n",
        "name = \"orig_app\"\n",
        "namedbg = \"orig_app_dbg\"\n",
        "version = \"0.1.0\"\n",
        "entry = \"src/main.ykn\"\n",
        "yukino = [\"0.1.0\", \"+\"]\n",
        "includes = [\"core\", \"stdlib\"]\n",
        "\n",
        "[features]\n",
        "default = []\n",
    );
    let mf_path = ws.join("yuki.toml");
    fs::write(&mf_path, orig_manifest).expect("write manifest");

    let main_ok = concat!(
        "load std.io: println\n",
        "\n",
        "static OVERRIDDEN_NAME = yukienv!(\"bin.name\")\n",
        "\n",
        "fun main() {\n",
        "    println(OVERRIDDEN_NAME)\n",
        "}\n",
    );
    fs::write(src.join("main.ykn"), main_ok).expect("write main.ykn");

    let build_ykn = concat!(
        "load std.io: print\n",
        "\n",
        "fun main() {\n",
        "    print(\"yuki:manifest=bin.name=temp_renamed_app\\n\")\n",
        "    print(\"yuki:manifest=config.debug.opt_level=2\\n\")\n",
        "    print(\"yuki:manifest-append=libs.static.system=m\\n\")\n",
        "    print(\"yuki:manifest-remove=bin.namedbg\\n\")\n",
        "}\n",
    );
    fs::write(ws.join("build.ykn"), build_ykn).expect("write build.ykn");

    // 1. Successful build applies temporary overrides and restores original yuki.toml
    let opts = BuildOptions {
        project_dir: ws.clone(),
        profile: "debug".to_string(),
        ..Default::default()
    };
    let artifact = build_project(&opts).expect("build succeeds with temporary manifest overrides");
    assert!(artifact.is_file(), "output binary should exist");
    assert_eq!(
        artifact.file_name().and_then(|n| n.to_str()),
        Some(if cfg!(windows) { "temp_renamed_app.exe" } else { "temp_renamed_app" })
    );
    let run_out = std::process::Command::new(&artifact)
        .output()
        .expect("run compiled binary");
    assert!(run_out.status.success());
    assert_eq!(String::from_utf8_lossy(&run_out.stdout).trim(), "temp_renamed_app");
    assert_eq!(
        fs::read_to_string(&mf_path).expect("read manifest after success"),
        orig_manifest
    );

    // 2. Failing build still restores original yuki.toml
    fs::write(src.join("main.ykn"), "load std.io: println\nfun main() { undefined_symbol_err() }\n")
        .expect("write broken main.ykn");
    let err = build_project(&opts).expect_err("broken source must fail compilation");
    assert!(!err.to_string().is_empty());
    assert_eq!(
        fs::read_to_string(&mf_path).expect("read manifest after failure"),
        orig_manifest
    );

    let _ = fs::remove_dir_all(ws);
}




