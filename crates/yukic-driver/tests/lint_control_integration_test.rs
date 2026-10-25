#![deny(unsafe_code)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use yukic_driver::{DriverConfig, DriverError, EmitKind, run_compiler_driver};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

fn unique_test_dir(name: &str) -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("yukic_lint_test_{name}_{}_{id}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create test dir");
    dir
}

#[test]
fn test_lint_deny_aborts_compilation_with_frontend_validation_error() {
    let test_dir = unique_test_dir("deny_file");
    let main_file = test_dir.join("main.ykn");
    let out_file = test_dir.join("out.o");

    let src = r#"#![lint::deny("unused_variable")]

fun main() -> int32 {
    set unused_val = 123
    return 0
}
"#;
    fs::write(&main_file, src).expect("write main.ykn");

    let mut config = DriverConfig::new(main_file.to_string_lossy().into_owned())
        .with_emit(EmitKind::Obj)
        .with_threads(1);
    config.output_file = Some(out_file.to_string_lossy().into_owned());

    let res = run_compiler_driver(&config);
    assert!(res.is_err(), "compilation with denied warning must fail");
    match res.unwrap_err() {
        DriverError::FrontendValidation(err_count) => {
            assert!(err_count >= 1, "expected at least 1 frontend validation error");
        }
        other => panic!("expected FrontendValidation error, got: {other:?}"),
    }

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_lint_ignore_suppresses_warning_and_succeeds() {
    let test_dir = unique_test_dir("ignore_file");
    let main_file = test_dir.join("main.ykn");
    let out_file = test_dir.join("out.o");

    let src = r#"#![lint::ignore("unused_variable")]

fun main() -> int32 {
    set unused_val = 123
    return 0
}
"#;
    fs::write(&main_file, src).expect("write main.ykn");

    let mut config = DriverConfig::new(main_file.to_string_lossy().into_owned())
        .with_emit(EmitKind::Obj)
        .with_threads(1);
    config.output_file = Some(out_file.to_string_lossy().into_owned());

    let res = run_compiler_driver(&config);
    assert!(res.is_ok(), "compilation with ignored warning should succeed: {res:?}");
    assert!(out_file.is_file(), "output object file should exist");

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_lint_deny_on_function_attribute() {
    let test_dir = unique_test_dir("deny_fn");
    let main_file = test_dir.join("main.ykn");
    let out_file = test_dir.join("out.o");

    let src = r#"
#[lint::deny("unused_variable")]
fun main() -> int32 {
    set local_var = 42
    return 0
}
"#;
    fs::write(&main_file, src).expect("write main.ykn");

    let mut config = DriverConfig::new(main_file.to_string_lossy().into_owned())
        .with_emit(EmitKind::Obj)
        .with_threads(1);
    config.output_file = Some(out_file.to_string_lossy().into_owned());

    let res = run_compiler_driver(&config);
    assert!(res.is_err(), "function-level deny must abort compilation");
    match res.unwrap_err() {
        DriverError::FrontendValidation(err_count) => {
            assert!(err_count >= 1, "expected at least 1 frontend validation error");
        }
        other => panic!("expected FrontendValidation error, got: {other:?}"),
    }

    let _ = fs::remove_dir_all(&test_dir);
}
