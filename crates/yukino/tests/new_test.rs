#![deny(unsafe_code)]

use std::path::PathBuf;
use std::process::Command;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("yukino_cli_test_{}_{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn bin_path() -> PathBuf {
    let mut p = std::env::current_exe().expect("current exe");
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("yukino")
}

#[test]
fn test_cli_new_app() {
    let workdir = temp_dir("app");
    let bin = bin_path();

    let output = Command::new(&bin)
        .current_dir(&workdir)
        .args(["new", "test_app"])
        .output()
        .expect("run yukino new");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    assert!(stdout.contains("Creating project: test_app..."));
    assert!(stdout.contains("Finished: "));

    let app_dir = workdir.join("test_app");
    let toml_file = app_dir.join("yuki.toml");
    let main_file = app_dir.join("src/main.ykn");
    let gitignore = app_dir.join(".gitignore");

    assert!(toml_file.exists());
    assert!(main_file.exists());
    assert!(gitignore.exists());

    let toml_content = std::fs::read_to_string(&toml_file).expect("read yuki.toml");
    assert!(toml_content.contains("[bin]"));
    assert!(toml_content.contains("name = \"test_app\""));
    assert!(toml_content.contains("namedbg = \"test_app_debug\""));
    assert!(toml_content.contains("entry = \"src/main.ykn\""));
    assert!(toml_content.contains("yukino = [\"0.1.0\", \"+\"]"));
    assert!(toml_content.contains("includes = [\"stdlib\"]"));
    assert!(!toml_content.contains("std ="));
    assert!(toml_content.contains("# example = { link = \"https://.../lib.tar.zst\" }"));

    let main_content = std::fs::read_to_string(&main_file).expect("read main.ykn");
    assert!(main_content.contains("load std.io: print"));
    assert!(main_content.contains("fun main()"));

    let gitignore_content = std::fs::read_to_string(&gitignore).expect("read .gitignore");
    assert_eq!(gitignore_content, "yuki-work/local/\n");

    let _ = std::fs::remove_dir_all(&workdir);
}

#[test]
fn test_cli_new_static_lib() {
    let workdir = temp_dir("static_lib");
    let bin = bin_path();

    let output = Command::new(&bin)
        .current_dir(&workdir)
        .args(["new", "my_stat", "--static"])
        .output()
        .expect("run yukino new --static");

    assert!(output.status.success());
    let lib_dir = workdir.join("my_stat");
    let toml_file = lib_dir.join("lib.toml");
    let lib_file = lib_dir.join("my_stat/lib.ykn");

    assert!(toml_file.exists());
    assert!(lib_file.exists());
    assert!(!lib_dir.join("src").exists());

    let toml_content = std::fs::read_to_string(&toml_file).expect("read lib.toml");
    assert!(toml_content.contains("name = \"my_stat\""));
    assert!(toml_content.contains("output = \"static\""));
    assert!(toml_content.contains("entry = \"my_stat/lib.ykn\""));
    assert!(toml_content.contains("yukino = [\"0.1.0\", \"+\"]"));
    assert!(toml_content.contains("includes = [\"stdlib\"]"));

    let lib_content = std::fs::read_to_string(&lib_file).expect("read lib.ykn");
    assert!(lib_content.contains("// \"fun\" for Yukino function, \"cfun\" for c abi function"));
    assert!(lib_content.contains("public fun calc(a: int64, b: int64) -> int64"));

    let _ = std::fs::remove_dir_all(&workdir);
}

#[test]
fn test_cli_new_dynamic_lib() {
    let workdir = temp_dir("dynamic_lib");
    let bin = bin_path();

    let output = Command::new(&bin)
        .current_dir(&workdir)
        .args(["new", "my_dyn", "--dynamic"])
        .output()
        .expect("run yukino new --dynamic");

    assert!(output.status.success());
    let lib_dir = workdir.join("my_dyn");
    let toml_file = lib_dir.join("lib.toml");
    let lib_file = lib_dir.join("my_dyn/lib.ykn");

    assert!(toml_file.exists());
    assert!(lib_file.exists());
    assert!(!lib_dir.join("src").exists());
    let toml_content = std::fs::read_to_string(&toml_file).expect("read lib.toml");
    assert!(toml_content.contains("name = \"my_dyn\""));
    assert!(toml_content.contains("output = \"shared\""));
    assert!(toml_content.contains("includes = [\"stdlib\"]"));
    assert!(toml_content.contains("entry = \"my_dyn/lib.ykn\""));

    let lib_content = std::fs::read_to_string(&lib_file).expect("read lib.ykn");
    assert!(lib_content.contains("// c abi lib"));
    assert!(lib_content.contains("public cfun calc(a: c_longlong, b: c_longlong) -> c_longlong"));

    let _ = std::fs::remove_dir_all(&workdir);
}

#[test]
fn test_cli_new_colored_output() {
    let workdir = temp_dir("colored_cli");
    let bin = bin_path();

    let output = Command::new(&bin)
        .current_dir(&workdir)
        .env("CLICOLOR_FORCE", "1")
        .args(["new", "color_proj"])
        .output()
        .expect("run yukino new with CLICOLOR_FORCE=1");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    assert!(stdout.contains("\x1b[36m"));
    assert!(stdout.contains("color_proj"));

    let _ = std::fs::remove_dir_all(&workdir);
}

#[test]
fn test_cli_new_existing_folder_error_exact_message() {
    let workdir = temp_dir("existing");
    let bin = bin_path();

    let existing = workdir.join("already_exists");
    std::fs::create_dir_all(&existing).expect("create folder");

    let output = Command::new(&bin)
        .current_dir(&workdir)
        .args(["new", "already_exists"])
        .output()
        .expect("run yukino new");

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    let expected = "ERROR: A folder with that name already exists; if you want to add Yukino to it, use:\n\n    yukino new --init / yukino new -i\n";
    assert_eq!(stdout, expected);

    let _ = std::fs::remove_dir_all(&workdir);
}

#[test]
fn test_cli_new_init_mode() {
    let workdir = temp_dir("test_init_folder");
    let bin = bin_path();

    let output = Command::new(&bin)
        .current_dir(&workdir)
        .args(["new", "-i"])
        .output()
        .expect("run yukino new -i");

    assert!(output.status.success());
    assert!(workdir.join("yuki.toml").exists());
    assert!(workdir.join("src/main.ykn").exists());
    assert!(workdir.join(".gitignore").exists());

    let toml_content = std::fs::read_to_string(workdir.join("yuki.toml")).expect("read toml");
    let expected_name = workdir.file_name().unwrap().to_str().unwrap();
    assert!(toml_content.contains(&format!("name = \"{expected_name}\"")));
    assert!(toml_content.contains("includes = [\"stdlib\"]"));

    let _ = std::fs::remove_dir_all(&workdir);
}

#[test]
fn test_cli_new_init_with_explicit_name() {
    let workdir = temp_dir("init_explicit");
    let bin = bin_path();

    let output = Command::new(&bin)
        .current_dir(&workdir)
        .args(["new", "custom_named", "--init"])
        .output()
        .expect("run yukino new --init custom_named");

    assert!(output.status.success());
    let toml_content = std::fs::read_to_string(workdir.join("yuki.toml")).expect("read toml");
    assert!(toml_content.contains("name = \"custom_named\""));
    assert!(toml_content.contains("includes = [\"stdlib\"]"));

    let _ = std::fs::remove_dir_all(&workdir);
}

#[test]
fn test_cli_new_gitignore_existing_append_no_duplication() {
    let workdir = temp_dir("gitignore_append");
    let bin = bin_path();

    let gitignore = workdir.join(".gitignore");
    std::fs::write(&gitignore, "target/\n").expect("write gitignore");

    let output = Command::new(&bin)
        .current_dir(&workdir)
        .args(["new", "-i"])
        .output()
        .expect("run yukino new -i");

    assert!(output.status.success());
    let gitignore_content = std::fs::read_to_string(&gitignore).expect("read gitignore");
    assert_eq!(gitignore_content, "target/\nyuki-work/local/\n");

    // Second call should not duplicate
    let output2 = Command::new(&bin)
        .current_dir(&workdir)
        .args(["new", "-i"])
        .output()
        .expect("run yukino new -i second");

    assert!(output2.status.success());
    let gitignore_content2 = std::fs::read_to_string(&gitignore).expect("read gitignore second");
    assert_eq!(gitignore_content2, "target/\nyuki-work/local/\n");

    let _ = std::fs::remove_dir_all(&workdir);
}

#[test]
fn test_cli_new_app_no_network_fetch() {
    let workdir = temp_dir("app_network");
    let bin = bin_path();

    let output = Command::new(&bin)
        .current_dir(&workdir)
        .args(["new", "live_app"])
        .output()
        .expect("run yukino new live_app");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    assert!(stdout.contains("Creating project: live_app..."));
    assert!(!stdout.contains("Fetching"));
    assert!(!stdout.contains("Standart library: "));
    assert!(stdout.contains("Finished: "));

    let app_dir = workdir.join("live_app");
    assert!(app_dir.join("yuki.toml").exists());
    assert!(app_dir.join("src/main.ykn").exists());

    let toml_content = std::fs::read_to_string(app_dir.join("yuki.toml")).expect("read yuki.toml");
    assert!(!toml_content.contains("std ="));
    assert!(toml_content.contains("# example = { link = \"https://.../lib.tar.zst\" }"));

    let _ = std::fs::remove_dir_all(&workdir);
}

#[test]
fn test_cli_new_init_preserves_custom_code() {
    let workdir = temp_dir("init_preserve_code");
    let bin = bin_path();

    let src_dir = workdir.join("src");
    std::fs::create_dir_all(&src_dir).expect("create src");
    std::fs::write(src_dir.join("main.ykn"), "// my existing code\n").expect("write code");

    let output = Command::new(&bin)
        .current_dir(&workdir)
        .args(["new", "-i"])
        .output()
        .expect("run yukino new -i");

    assert!(output.status.success());
    let main_content = std::fs::read_to_string(src_dir.join("main.ykn")).expect("read code");
    assert_eq!(main_content, "// my existing code\n");

    let _ = std::fs::remove_dir_all(&workdir);
}

#[test]
fn test_cli_new_macro_lib() {
    let workdir = temp_dir("macro_cli");
    let bin = bin_path();

    let output = Command::new(&bin)
        .current_dir(&workdir)
        .args(["new", "test_macro", "--macro"])
        .output()
        .expect("run yukino new --macro");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    assert!(stdout.contains("Creating project: test_macro..."));

    let macro_dir = workdir.join("test_macro");
    let toml_file = macro_dir.join("lib.toml");
    let lib_file = macro_dir.join("test_macro/lib.ykn");
    let gitignore = macro_dir.join(".gitignore");

    assert!(toml_file.exists());
    assert!(lib_file.exists());
    assert!(gitignore.exists());

    let toml_content = std::fs::read_to_string(&toml_file).expect("read lib.toml");
    assert!(toml_content.contains("[lib]"));
    assert!(toml_content.contains("name = \"test_macro\""));
    assert!(toml_content.contains("output = \"macro\""));
    assert!(toml_content.contains("entry = \"test_macro/lib.ykn\""));

    let lib_content = std::fs::read_to_string(&lib_file).expect("read lib.ykn");
    assert!(lib_content.contains("procedural macro"));

    let _ = std::fs::remove_dir_all(&workdir);
}
