#![deny(unsafe_code)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use yukic_driver::{DriverConfig, DriverError, EmitKind, run_compiler_driver};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

fn unique_test_dir(name: &str) -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("yukic_deprecated_{name}_{}_{id}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create test dir");
    dir
}

#[test]
fn test_deprecated_function_call_compiles() {
    let test_dir = unique_test_dir("fn_call");
    let main_file = test_dir.join("main.ykn");
    let out_file = test_dir.join("out.o");

    let src = r#"#[deprecated]
fun old_calc() -> int32 {
    return 42
}

fun main() -> int32 {
    set res = old_calc()
    return res
}
"#;
    fs::write(&main_file, src).expect("write main.ykn");

    let mut config = DriverConfig::new(main_file.to_string_lossy().into_owned())
        .with_emit(EmitKind::Obj)
        .with_threads(1);
    config.output_file = Some(out_file.to_string_lossy().into_owned());

    let res = run_compiler_driver(&config);
    assert!(res.is_ok(), "compilation with deprecated warning must succeed: {:?}", res.err());
    assert!(out_file.exists(), "output object file should exist");

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_deprecated_deny_aborts_compilation() {
    let test_dir = unique_test_dir("deny");
    let main_file = test_dir.join("main.ykn");
    let out_file = test_dir.join("out.o");

    let src = r#"#![lint::deny(deprecated)]

#[deprecated = "use new_calc() instead"]
fun old_calc() -> int32 {
    return 42
}

fun main() -> int32 {
    set res = old_calc()
    return res
}
"#;
    fs::write(&main_file, src).expect("write main.ykn");

    let mut config = DriverConfig::new(main_file.to_string_lossy().into_owned())
        .with_emit(EmitKind::Obj)
        .with_threads(1);
    config.output_file = Some(out_file.to_string_lossy().into_owned());

    let res = run_compiler_driver(&config);
    assert!(res.is_err(), "compilation with denied deprecated must fail");
    match res.unwrap_err() {
        DriverError::FrontendValidation(err_count) => {
            assert!(err_count >= 1, "expected at least 1 frontend validation error");
        }
        other => panic!("expected FrontendValidation error, got: {other:?}"),
    }

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_deprecated_inside_deprecated_caller_does_not_fail_deny() {
    let test_dir = unique_test_dir("nested_dep");
    let main_file = test_dir.join("main.ykn");
    let out_file = test_dir.join("out.o");

    let src = r#"#![lint::deny(deprecated)]

#[deprecated]
fun old_internal() -> int32 {
    return 10
}

#[deprecated]
fun old_wrapper() -> int32 {
    return old_internal()
}

fun main() -> int32 {
    return 0
}
"#;
    fs::write(&main_file, src).expect("write main.ykn");

    let mut config = DriverConfig::new(main_file.to_string_lossy().into_owned())
        .with_emit(EmitKind::Obj)
        .with_threads(1);
    config.output_file = Some(out_file.to_string_lossy().into_owned());

    let res = run_compiler_driver(&config);
    assert!(res.is_ok(), "deprecated caller calling deprecated item must not trigger warning: {:?}", res.err());
    assert!(out_file.exists());

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_deprecated_ignore_suppresses_warning() {
    let test_dir = unique_test_dir("ignore");
    let main_file = test_dir.join("main.ykn");
    let out_file = test_dir.join("out.o");

    let src = r#"#![lint::ignore("deprecated_item")]

#[deprecated = "use modern_calc instead"]
fun legacy_calc() -> int32 {
    return 100
}

fun main() -> int32 {
    return legacy_calc()
}
"#;
    fs::write(&main_file, src).expect("write main.ykn");

    let mut config = DriverConfig::new(main_file.to_string_lossy().into_owned())
        .with_emit(EmitKind::Obj)
        .with_threads(1);
    config.output_file = Some(out_file.to_string_lossy().into_owned());

    let res = run_compiler_driver(&config);
    assert!(res.is_ok(), "ignored deprecation must succeed: {:?}", res.err());
    assert!(out_file.exists());

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_deprecated_struct_and_method_deny() {
    let test_dir = unique_test_dir("struct_method");
    let main_file = test_dir.join("main.ykn");
    let out_file = test_dir.join("out.o");

    let src = r#"#![lint::deny(deprecated_item)]

#[deprecated = "use ModernConfig instead"]
struct LegacyConfig {
    port: int32
}

fun main() -> int32 {
    set cfg = LegacyConfig { port: 8080 }
    return cfg.port
}
"#;
    fs::write(&main_file, src).expect("write main.ykn");

    let mut config = DriverConfig::new(main_file.to_string_lossy().into_owned())
        .with_emit(EmitKind::Obj)
        .with_threads(1);
    config.output_file = Some(out_file.to_string_lossy().into_owned());

    let res = run_compiler_driver(&config);
    assert!(res.is_err(), "deprecated struct instantiation must fail under deny");

    let _ = fs::remove_dir_all(&test_dir);
}
