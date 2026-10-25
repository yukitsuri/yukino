use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let date = Command::new("date")
        .arg("+%d-%m-%Y")
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                String::from_utf8(out.stdout)
                    .ok()
                    .map(|value| value.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "20-09-2026".to_string());

    println!("cargo:rustc-env=YUKIC_BUILD_DATE={date}");
    configure_build_environment();

    /*
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let target = env::var("TARGET").expect("TARGET not set");
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());

    let runtime_dir = manifest_dir.join("../../yukino-static/runtime");
    let runtime_lib = runtime_dir.join("src/lib.rs");

    println!("cargo:rerun-if-changed={}", runtime_dir.join("src/lib.rs").display());
    println!("cargo:rerun-if-changed={}", runtime_dir.join("src/panic.rs").display());
    println!("cargo:rerun-if-changed={}", runtime_dir.join("src/personality.rs").display());
    println!("cargo:rerun-if-changed={}", runtime_dir.join("src/region.rs").display());
    println!("cargo:rerun-if-changed={}", runtime_dir.join("src/rt_cast.rs").display());
    println!("cargo:rerun-if-changed={}", runtime_dir.join("Cargo.toml").display());
    println!("cargo:rerun-if-env-changed=RUSTC");
    println!("cargo:rerun-if-env-changed=TARGET");

    let out_archive = out_dir.join("libyukino_rt.a");

    let status = Command::new(&rustc)
        .arg("--crate-name")
        .arg("yukino_rt")
        .arg("--crate-type")
        .arg("staticlib")
        .arg("--edition")
        .arg("2024")
        .arg("-C")
        .arg("opt-level=z")
        .arg("-C")
        .arg("codegen-units=1")
        .arg("-C")
        .arg("panic=abort")
        .arg("-C")
        .arg("force-unwind-tables=yes")
        .arg("-C")
        .arg("strip=debuginfo")
        .arg("-C")
        .arg("link-dead-code=no")
        .arg("--target")
        .arg(&target)
        .arg(&runtime_lib)
        .arg("-o")
        .arg(&out_archive)
        .status()
        .expect("failed to execute rustc to compile embedded runtime");

    assert!(status.success(), "rustc failed to compile embedded runtime libyukino_rt.a for target {target}");

    // strip debug sections to minimize embedded archive footprint
    let _ = Command::new("llvm-strip")
        .arg("--strip-debug")
        .arg(&out_archive)
        .status()
        .or_else(|_| {
            let host = env::var("HOST").unwrap_or_default();
            if target == host {
                Command::new("strip").arg("--strip-debug").arg(&out_archive).status()
            } else {
                Err(std::io::Error::other("skip host strip"))
            }
        });

    println!("cargo:rustc-env=YUKINO_RT_TARGET={target}");
    */
}

fn configure_build_environment() {
    println!("cargo:rerun-if-env-changed=YUKINO_RELEASEBUILD");

    let release = env::var("YUKINO_RELEASEBUILD").map_or(true, |value| value == "1");
    let (yukic_name, yukino_name, yukino_dir, yukino_cfg_dir) = if release {
        ("yukic", "yukino", ".yuki-lang", ".yukino-lang")
    } else {
        (
            "yukic-dbg",
            "yukino-dbg",
            ".yuki-lang-dbg",
            ".yukino-lang-dbg",
        )
    };
    println!("cargo:rustc-env=YUKINO_NAME={yukino_name}");
    println!("cargo:rustc-env=YUKIC_NAME={yukic_name}");
    println!("cargo:rustc-env=YUKINO_DIR={yukino_dir}");
    println!("cargo:rustc-env=YUKINO_CFG_DIR={yukino_cfg_dir}");

    if env::var_os("YUKINO_RELEASEBUILD").is_none() {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let project_dir = manifest_dir.join("../..");
    let temp_dir = project_dir.join("temp");
    fs::create_dir_all(&temp_dir).expect("failed to create temp directory");
    let config = format!(
        "{{\n  \"yukic_name\": \"{yukic_name}\",\n  \"yukino_name\": \"{yukino_name}\",\n  \"yukino_dir\": \"{yukino_dir}\",\n  \"yukino_cfg_dir\": \"{yukino_cfg_dir}\"\n}}\n"
    );
    fs::write(temp_dir.join("build-config.json"), config)
        .expect("failed to write build config");
}
