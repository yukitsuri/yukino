#![deny(unsafe_code)]

use std::fmt;
use std::path::{Path, PathBuf};

use crate::VERSION;
use crate::manifest::{
    generate_app_manifest, generate_dynamic_lib_ykn, generate_lib_manifest, generate_macro_lib_ykn,
    generate_main_ykn, generate_static_lib_ykn, generate_workspace_manifest,
};

pub const 
FOLDER_EXISTS_ERROR: 
&str = 
"ERROR: A folder with that name already exists; if you want to add Yukino to it, use:\n\n    yukino new --init / yukino new -i";

#[derive(Debug, PartialEq, Eq)]
pub enum ProjectError {
    FolderAlreadyExists,
    MissingProjectName,
    Io(String),
    Runtime(String),
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FolderAlreadyExists => write!(f, "{FOLDER_EXISTS_ERROR}"),
            Self::MissingProjectName => write!(f, "project name is required when not using --init"),
            Self::Io(msg) | Self::Runtime(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ProjectError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectKind {
    App,
    StaticLib,
    DynamicLib,
    Macro,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewOptions {
    pub name: Option<String>,
    pub kind: ProjectKind,
    pub init: bool,
}

pub fn update_gitignore(project_dir: &Path) -> std::io::Result<()> {
    let gitignore_path = project_dir.join(".gitignore");
    if gitignore_path.exists() {
        let content = std::fs::read_to_string(&gitignore_path)?;
        let already_present = content.lines().any(|line| {
            let trimmed = line.trim();
            trimmed == "yuki-work/local/" || trimmed == "yuki-work/local"
        });
        if !already_present {
            let mut updated = content;
            if !updated.is_empty() && !updated.ends_with('\n') {
                updated.push('\n');
            }
            updated.push_str("yuki-work/local/\n");
            std::fs::write(&gitignore_path, updated)?;
        }
    } else {
        std::fs::write(&gitignore_path, "yuki-work/local/\n")?;
    }
    Ok(())
}

pub fn resolve_project_target(
    opts: &NewOptions,
    base_dir: &Path,
) -> Result<(String, PathBuf), ProjectError> {
    if opts.init {
        let base = base_dir
            .canonicalize()
            .unwrap_or_else(|_| base_dir.to_path_buf());
        let name = match &opts.name {
            Some(n) if !n.trim().is_empty() => n.trim().to_string(),
            _ => base
                .file_name()
                .and_then(|s| s.to_str())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("unnamed")
                .to_string(),
        };
        Ok((name, base_dir.to_path_buf()))
    } else {
        let given = match &opts.name {
            Some(n) if !n.trim().is_empty() => n.trim().to_string(),
            _ => return Err(ProjectError::MissingProjectName),
        };
        let mut target_dir = base_dir.to_path_buf();
        for part in given.split(|c| c == '/' || c == '\\') {
            if !part.is_empty() {
                target_dir.push(part);
            }
        }
        if target_dir.exists() {
            return Err(ProjectError::FolderAlreadyExists);
        }
        // the manifest name is the folder itself: a path would break TOML on Windows
        let name = target_dir
            .file_name()
            .and_then(|s| s.to_str())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(&given)
            .to_string();
        Ok((name, target_dir))
    }
}

fn write_project_files(
    kind: ProjectKind,
    target_dir: &Path,
    name: &str,
) -> std::io::Result<()> {
    match kind {
        ProjectKind::App => {
            let src_dir = target_dir.join("src");
            std::fs::create_dir_all(&src_dir)?;
            let main_path = src_dir.join("main.ykn");
            if !main_path.exists() {
                std::fs::write(&main_path, generate_main_ykn())?;
            }
            let toml_path = target_dir.join("yuki.toml");
            if !toml_path.exists() {
                std::fs::write(&toml_path, generate_app_manifest(name, VERSION))?;
            }
        }
        ProjectKind::StaticLib => {
            let lib_src_dir = target_dir.join(name);
            std::fs::create_dir_all(&lib_src_dir)?;
            let lib_path = lib_src_dir.join("lib.ykn");
            if !lib_path.exists() {
                std::fs::write(&lib_path, generate_static_lib_ykn())?;
            }
            let toml_path = target_dir.join("lib.toml");
            if !toml_path.exists() {
                std::fs::write(
                    &toml_path,
                    generate_lib_manifest(name, VERSION, "static"),
                )?;
            }
        }
        ProjectKind::DynamicLib => {
            let lib_src_dir = target_dir.join(name);
            std::fs::create_dir_all(&lib_src_dir)?;
            let lib_path = lib_src_dir.join("lib.ykn");
            if !lib_path.exists() {
                std::fs::write(&lib_path, generate_dynamic_lib_ykn())?;
            }
            let toml_path = target_dir.join("lib.toml");
            if !toml_path.exists() {
                std::fs::write(
                    &toml_path,
                    generate_lib_manifest(name, VERSION, "shared"),
                )?;
            }
        }
        ProjectKind::Macro => {
            let lib_src_dir = target_dir.join(name);
            std::fs::create_dir_all(&lib_src_dir)?;
            let lib_path = lib_src_dir.join("lib.ykn");
            if !lib_path.exists() {
                std::fs::write(&lib_path, generate_macro_lib_ykn())?;
            }
            let toml_path = target_dir.join("lib.toml");
            if !toml_path.exists() {
                std::fs::write(
                    &toml_path,
                    generate_lib_manifest(name, VERSION, "macro"),
                )?;
            }
        }
        ProjectKind::Workspace => {
            std::fs::create_dir_all(target_dir)?;
            let toml_path = target_dir.join("yukispace.toml");
            if !toml_path.exists() {
                std::fs::write(&toml_path, generate_workspace_manifest(name))?;
            }
            let libs_dir = target_dir.join("libs");
            std::fs::create_dir_all(&libs_dir)?;
            let apps_dir = target_dir.join("apps");
            std::fs::create_dir_all(&apps_dir)?;
        }
    }

    update_gitignore(target_dir)?;
    Ok(())
}

pub fn create_project(opts: &NewOptions, base_dir: &Path) -> Result<PathBuf, ProjectError> {
    let (name, target_dir) = resolve_project_target(opts, base_dir)?;

    println!("Creating project: {name}...");

    write_project_files(opts.kind, &target_dir, &name)
        .map_err(|e| ProjectError::Io(e.to_string()))?;

    let colored = crate::should_color();
    let abs_path = target_dir.canonicalize().unwrap_or(target_dir);
    let display_path = crate::format_path(&abs_path.display().to_string(), colored);
    println!("Finished: {display_path}");

    Ok(abs_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("yukino_test_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp test dir");
        dir
    }

    #[test]
    fn test_gitignore_creation_and_no_duplicates() {
        let dir = temp_test_dir("gitignore");
        update_gitignore(&dir).expect("update gitignore");

        let gitignore = dir.join(".gitignore");
        assert!(gitignore.exists());
        let content = std::fs::read_to_string(&gitignore).expect("read");
        assert_eq!(content, "yuki-work/local/\n");

        // run again to ensure no duplicates
        update_gitignore(&dir).expect("second update");
        let content2 = std::fs::read_to_string(&gitignore).expect("read second");
        assert_eq!(content2, "yuki-work/local/\n");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_gitignore_append_existing() {
        let dir = temp_test_dir("gitignore_append");
        let gitignore = dir.join(".gitignore");
        std::fs::write(&gitignore, "target/\n").expect("write existing");

        update_gitignore(&dir).expect("update gitignore");
        let content = std::fs::read_to_string(&gitignore).expect("read");
        assert_eq!(content, "target/\nyuki-work/local/\n");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_existing_folder_error() {
        let dir = temp_test_dir("existing_folder");
        let existing_sub = dir.join("already_here");
        std::fs::create_dir_all(&existing_sub).expect("create sub");

        let opts = NewOptions {
            name: Some("already_here".to_string()),
            kind: ProjectKind::App,
            init: false,
        };

        let res = resolve_project_target(&opts, &dir);
        assert_eq!(res, Err(ProjectError::FolderAlreadyExists));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_init_infers_name() {
        let dir = temp_test_dir("inferred_proj");
        let opts = NewOptions {
            name: None,
            kind: ProjectKind::App,
            init: true,
        };

        let (name, target) = resolve_project_target(&opts, &dir).expect("resolve init");
        assert_eq!(target, dir);
        assert!(name.contains("inferred_proj"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_create_app_project() {
        let dir = temp_test_dir("create_app");
        let opts = NewOptions {
            name: Some("sample_app".to_string()),
            kind: ProjectKind::App,
            init: false,
        };

        let res = create_project(&opts, &dir);
        assert!(res.is_ok());
        let proj_dir = dir.join("sample_app");
        assert!(proj_dir.join("yuki.toml").exists());
        assert!(proj_dir.join("src/main.ykn").exists());
        assert!(proj_dir.join(".gitignore").exists());

        let toml_str = std::fs::read_to_string(proj_dir.join("yuki.toml")).expect("read toml");
        assert!(toml_str.contains("name = \"sample_app\""));
        assert!(toml_str.contains("yukino = [\"0.1.0\", \"+\"]"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_create_app_project_with_nested_path_name() {
        // path-like names join into the base dir; the manifest keeps only the folder name
        let dir = temp_test_dir("create_app_nested");
        let opts = NewOptions {
            name: Some("sub\\dir\\nested_app".to_string()),
            kind: ProjectKind::App,
            init: false,
        };

        create_project(&opts, &dir).expect("create nested app");
        let proj_dir = dir.join("sub").join("dir").join("nested_app");
        assert!(proj_dir.join("yuki.toml").exists());

        let toml_str = std::fs::read_to_string(proj_dir.join("yuki.toml")).expect("read toml");
        assert!(toml_str.contains("name = \"nested_app\""));
        assert!(!toml_str.contains("sub\\dir"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_create_static_and_dynamic_lib() {
        let dir = temp_test_dir("create_libs");

        // Static lib
        let opts_static = NewOptions {
            name: Some("my_static".to_string()),
            kind: ProjectKind::StaticLib,
            init: false,
        };
        create_project(&opts_static, &dir).expect("create static");
        let static_toml =
            std::fs::read_to_string(dir.join("my_static/lib.toml")).expect("read toml");
        assert!(static_toml.contains("output = \"static\""));
        assert!(static_toml.contains("entry = \"my_static/lib.ykn\""));
        assert!(!dir.join("my_static/src").exists());
        let static_lib =
            std::fs::read_to_string(dir.join("my_static/my_static/lib.ykn")).expect("read static lib");
        assert!(static_lib.contains("// \"fun\" for Yukino function, \"cfun\" for c abi function"));
        assert!(static_lib.contains("public fun calc(a: int64, b: int64) -> int64"));

        // Dynamic lib
        let opts_dynamic = NewOptions {
            name: Some("my_dynamic".to_string()),
            kind: ProjectKind::DynamicLib,
            init: false,
        };
        create_project(&opts_dynamic, &dir).expect("create dynamic");
        let dynamic_toml =
            std::fs::read_to_string(dir.join("my_dynamic/lib.toml")).expect("read toml");
        assert!(dynamic_toml.contains("output = \"shared\""));
        assert!(dynamic_toml.contains("entry = \"my_dynamic/lib.ykn\""));
        assert!(!dir.join("my_dynamic/src").exists());
        let dynamic_lib =
            std::fs::read_to_string(dir.join("my_dynamic/my_dynamic/lib.ykn")).expect("read dynamic lib");
        assert!(dynamic_lib.contains("// c abi lib"));
        assert!(
            dynamic_lib.contains("public cfun calc(a: c_longlong, b: c_longlong) -> c_longlong")
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_init_preserves_existing_files() {
        let dir = temp_test_dir("init_preserve");
        let src_dir = dir.join("src");
        std::fs::create_dir_all(&src_dir).expect("create src");
        std::fs::write(src_dir.join("main.ykn"), "// custom user code\n").expect("write custom");
        std::fs::write(dir.join("yuki.toml"), "# custom toml\n").expect("write toml");

        let opts = NewOptions {
            name: None,
            kind: ProjectKind::App,
            init: true,
        };
        create_project(&opts, &dir).expect("create project init");

        let main_content = std::fs::read_to_string(src_dir.join("main.ykn")).expect("read main");
        assert_eq!(main_content, "// custom user code\n");

        let toml_content = std::fs::read_to_string(dir.join("yuki.toml")).expect("read toml");
        assert_eq!(toml_content, "# custom toml\n");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_create_macro_lib() {
        let dir = temp_test_dir("create_macro");
        let opts = NewOptions {
            name: Some("my_macro".to_string()),
            kind: ProjectKind::Macro,
            init: false,
        };
        create_project(&opts, &dir).expect("create macro");
        let toml = std::fs::read_to_string(dir.join("my_macro/lib.toml")).expect("read toml");
        assert!(toml.contains("output = \"macro\""));
        assert!(toml.contains("entry = \"my_macro/lib.ykn\""));
        let src = std::fs::read_to_string(dir.join("my_macro/my_macro/lib.ykn")).expect("read lib");
        assert!(src.contains("procedural macro"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_create_workspace() {
        let dir = temp_test_dir("create_ws");
        let opts = NewOptions {
            name: Some("my_ws".to_string()),
            kind: ProjectKind::Workspace,
            init: false,
        };
        create_project(&opts, &dir).expect("create workspace");
        let toml = std::fs::read_to_string(dir.join("my_ws/yukispace.toml")).expect("read toml");
        assert!(toml.contains("[workspace]"));
        assert!(toml.contains("name = \"my_ws\""));
        assert!(dir.join("my_ws/libs").is_dir());
        assert!(dir.join("my_ws/apps").is_dir());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
