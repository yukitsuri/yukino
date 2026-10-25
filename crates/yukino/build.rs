use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let date = std::process::Command::new("date")
        .arg("+%d-%m-%Y")
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                String::from_utf8(out.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "20-09-2026".to_string());

    println!("cargo:rustc-env=YUKINO_BUILD_DATE={date}");
    configure_build_environment();
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
