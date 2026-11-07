#![deny(unsafe_code)]

//!!! depr
//!
//!
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//! 
//!!! depr 

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::dashboard::BuildDashboard;
use crate::lockfile::{Lockfile, compute_archive_checksum};
use crate::manifest::DependencySpec;
use crate::project::ProjectError;
use crate::tracker::ThreadStatus;

pub fn resolve_dependency_dir(
    dep_name: &str,
    dep_spec: &DependencySpec,
    project_dir: &Path,
    thread_id: usize,
    dashboard: &BuildDashboard,
    refresh_deps: bool,
    lockfile: &Lockfile,
) -> Result<(PathBuf, String, Option<String>), ProjectError> {
    if let Some(rel_path) = &dep_spec.path {
        return resolve_path_dependency(dep_name, rel_path, project_dir);
    }
    if let Some(url) = &dep_spec.link {
        return resolve_link_dependency(
            dep_name,
            url,
            dep_spec,
            project_dir,
            thread_id,
            dashboard,
            refresh_deps,
            lockfile,
        );
    }
    if let Some(git_url) = &dep_spec.git {
        return resolve_git_dependency(
            dep_name,
            git_url,
            dep_spec,
            project_dir,
            thread_id,
            dashboard,
            refresh_deps,
            lockfile,
        );
    }
    Err(ProjectError::Runtime(format!(
        "dependency '{dep_name}' must specify either 'path', 'link', or 'git'"
    )))
}

fn has_manifest_file(dir: &Path) -> bool {
    dir.join("lib.toml").is_file()
        || dir.join("yuki.toml").is_file()
        || dir.join("yukispace.toml").is_file()
        || dir.join("build.ykn").is_file()
}

fn resolve_path_dependency(
    dep_name: &str,
    rel_path: &Path,
    project_dir: &Path,
) -> Result<(PathBuf, String, Option<String>), ProjectError> {
    let dep_dir = project_dir.join(rel_path);
    if !has_manifest_file(&dep_dir) {
        return Err(ProjectError::Runtime(format!(
            "dependency '{dep_name}' path '{}' does not contain 'lib.toml', 'yuki.toml', or 'yukispace.toml'",
            dep_dir.display()
        )));
    }
    Ok((dep_dir, String::new(), None))
}

fn resolve_link_dependency(
    dep_name: &str,
    url: &str,
    dep_spec: &DependencySpec,
    project_dir: &Path,
    thread_id: usize,
    dashboard: &BuildDashboard,
    refresh_deps: bool,
    lockfile: &Lockfile,
) -> Result<(PathBuf, String, Option<String>), ProjectError> {
    let dest_dir = project_dir.join("yuki-work/local/deps").join(dep_name);
    let cache_dir = project_dir.join("yuki-work/local/deps/cache");
    let ext = detect_archive_extension(url);
    let archive_path = cache_dir.join(format!("{dep_name}.{ext}"));

    if !refresh_deps && has_manifest_file(&dest_dir) && archive_path.is_file() {
        let current_hash = compute_archive_checksum(&archive_path).unwrap_or_default();
        let lock_matches = match lockfile.find_package(dep_name) {
            Some(pkg) => !pkg.checksum.is_empty() && pkg.checksum == current_hash,
            None => true,
        };
        let spec_matches = dep_spec.checksum.as_deref().map_or(true, |c| c == current_hash);
        if lock_matches && spec_matches {
            return Ok((dest_dir, current_hash, None));
        }
    }

    dashboard.update_status(
        thread_id,
        ThreadStatus::Downloading {
            dep: dep_name.to_string(),
            current: 1,
            total: 1,
            speed: None,
        },
    );

    let (downloaded, checksum) = download_archive_file(dep_name, url, &archive_path)?;
    if let Some(expected) = &dep_spec.checksum {
        if &checksum != expected {
            let _ = fs::remove_file(&downloaded);
            return Err(ProjectError::Runtime(format!(
                "checksum mismatch for '{dep_name}': expected {expected}, found {checksum}"
            )));
        }
    }

    dashboard.update_status(
        thread_id,
        ThreadStatus::Unpacking {
            dep: dep_name.to_string(),
            current: 1,
            total: 1,
        },
    );

    let _ = fs::remove_dir_all(&dest_dir);
    unpack_archive(&downloaded, &dest_dir, dep_name)?;
    normalize_lib_root(&dest_dir, dep_name)?;

    Ok((dest_dir, checksum, None))
}

fn resolve_git_dependency(
    dep_name: &str,
    git_url: &str,
    dep_spec: &DependencySpec,
    project_dir: &Path,
    thread_id: usize,
    dashboard: &BuildDashboard,
    refresh_deps: bool,
    lockfile: &Lockfile,
) -> Result<(PathBuf, String, Option<String>), ProjectError> {
    let dest_dir = project_dir.join("yuki-work/local/deps").join(dep_name);
    let cache_dir = project_dir.join("yuki-work/local/deps/cache");

    if !refresh_deps && has_manifest_file(&dest_dir) {
        if let Some(locked_pkg) = lockfile.find_package(dep_name) {
            if dep_spec.commit.is_none() || dep_spec.commit == locked_pkg.commit {
                return Ok((dest_dir, String::new(), locked_pkg.commit.clone()));
            }
        }
    }

    dashboard.update_status(
        thread_id,
        ThreadStatus::Downloading {
            dep: dep_name.to_string(),
            current: 1,
            total: 1,
            speed: None,
        },
    );

    let commit = fetch_git_repo(
        dep_name,
        git_url,
        dep_spec.tag.as_deref(),
        dep_spec.branch.as_deref(),
        &dest_dir,
        &cache_dir,
    )?;

    normalize_lib_root(&dest_dir, dep_name)?;
    Ok((dest_dir, String::new(), commit))
}

fn download_archive_file(
    dep_name: &str,
    url: &str,
    target_file: &Path,
) -> Result<(PathBuf, String), ProjectError> {
    if let Some(parent) = target_file.parent() {
        fs::create_dir_all(parent).map_err(|e| ProjectError::Io(e.to_string()))?;
    }

    let output = Command::new("curl")
        .args(["-sSL", "--max-time", "180", "-w", "\n%{http_code}", "-o"])
        .arg(target_file)
        .arg(url)
        .output()
        .map_err(|e| ProjectError::Runtime(format!("failed to run curl for '{dep_name}': {e}")))?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let http_code = stdout_str
        .lines()
        .last()
        .unwrap_or("")
        .trim()
        .parse::<u16>()
        .unwrap_or(0);

    if !output.status.success() || http_code < 200 || http_code >= 300 {
        let _ = fs::remove_file(target_file);
        return Err(ProjectError::Runtime(format!(
            "failed to download dependency '{dep_name}' from '{url}': HTTP {http_code}"
        )));
    }

    let checksum = compute_archive_checksum(target_file)
        .map_err(|e| ProjectError::Io(e.to_string()))?;

    Ok((target_file.to_path_buf(), checksum))
}

fn detect_archive_extension(url: &str) -> &'static str {
    let clean = url.split('?').next().unwrap_or(url);
    if clean.ends_with(".tar.zst") {
        "tar.zst"
    } else if clean.ends_with(".tar.gz") || clean.ends_with(".tgz") {
        "tar.gz"
    } else if clean.ends_with(".tar.xz") || clean.ends_with(".txz") {
        "tar.xz"
    } else if clean.ends_with(".zip") {
        "zip"
    } else if clean.ends_with(".tar") {
        "tar"
    } else {
        "tar.gz"
    }
}

fn unpack_archive(archive_path: &Path, dest_dir: &Path, dep_name: &str) -> Result<(), ProjectError> {
    fs::create_dir_all(dest_dir).map_err(|e| ProjectError::Io(e.to_string()))?;
    let filename = archive_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    let status = if filename.ends_with(".tar.zst") {
        Command::new("tar")
            .args(["--zstd", "-xf"])
            .arg(archive_path)
            .args(["-C"])
            .arg(dest_dir)
            .status()
    } else if filename.ends_with(".tar.gz") || filename.ends_with(".tgz") {
        Command::new("tar")
            .args(["-xzf"])
            .arg(archive_path)
            .args(["-C"])
            .arg(dest_dir)
            .status()
    } else if filename.ends_with(".tar.xz") || filename.ends_with(".txz") {
        Command::new("tar")
            .args(["-xJf"])
            .arg(archive_path)
            .args(["-C"])
            .arg(dest_dir)
            .status()
    } else if filename.ends_with(".zip") {
        Command::new("unzip")
            .args(["-q", "-o"])
            .arg(archive_path)
            .args(["-d"])
            .arg(dest_dir)
            .status()
    } else {
        Command::new("tar")
            .args(["-xf"])
            .arg(archive_path)
            .args(["-C"])
            .arg(dest_dir)
            .status()
    };

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(ProjectError::Runtime(format!(
            "failed to unpack dependency '{dep_name}': exit code {:?}",
            s.code()
        ))),
        Err(e) => Err(ProjectError::Runtime(format!(
            "failed to spawn unpacker for '{dep_name}': {e}"
        ))),
    }
}

fn normalize_lib_root(dest_dir: &Path, dep_name: &str) -> Result<(), ProjectError> {
    if has_manifest_file(dest_dir) {
        return Ok(());
    }

    let entries: Vec<PathBuf> = fs::read_dir(dest_dir)
        .map_err(|e| ProjectError::Io(e.to_string()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();

    let visible_dirs: Vec<&PathBuf> = entries
        .iter()
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map_or(false, |s| !s.starts_with('.'))
        })
        .collect();

    if visible_dirs.len() == 1 && has_manifest_file(visible_dirs[0]) {
        let wrapper = visible_dirs[0];
        let sub_entries = fs::read_dir(wrapper).map_err(|e| ProjectError::Io(e.to_string()))?;
        for sub in sub_entries.flatten() {
            let target = dest_dir.join(sub.file_name());
            let _ = fs::rename(sub.path(), target);
        }
        let _ = fs::remove_dir_all(wrapper);
    }

    if !has_manifest_file(dest_dir) {
        return Err(ProjectError::Runtime(format!(
            "dependency '{dep_name}' does not contain 'lib.toml', 'yuki.toml', or 'yukispace.toml' in root"
        )));
    }

    Ok(())
}

fn fetch_git_repo(
    dep_name: &str,
    git_url: &str,
    tag: Option<&str>,
    branch: Option<&str>,
    dest_dir: &Path,
    cache_dir: &Path,
) -> Result<Option<String>, ProjectError> {
    let _ = fs::remove_dir_all(dest_dir);

    let mut cmd = Command::new("git");
    cmd.arg("clone").arg("--depth").arg("1");
    if let Some(t) = tag {
        cmd.arg("--branch").arg(t);
    } else if let Some(b) = branch {
        cmd.arg("--branch").arg(b);
    }
    cmd.arg(git_url).arg(dest_dir);

    if let Ok(output) = cmd.output() {
        if output.status.success() {
            let sha = Command::new("git")
                .args(["-C"])
                .arg(dest_dir)
                .args(["rev-parse", "HEAD"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string());
            return Ok(sha);
        }
    }

    if let Some(archive_url) = parse_git_archive_url(git_url, tag, branch) {
        let ext = detect_archive_extension(&archive_url);
        let archive_file = cache_dir.join(format!("{dep_name}.{ext}"));
        let (arch_path, _) = download_archive_file(dep_name, &archive_url, &archive_file)?;
        unpack_archive(&arch_path, dest_dir, dep_name)?;
        return Ok(None);
    }

    Err(ProjectError::Runtime(format!(
        "failed to clone git repository '{git_url}' for dependency '{dep_name}'"
    )))
}

fn parse_git_archive_url(git_url: &str, tag: Option<&str>, branch: Option<&str>) -> Option<String> {
    let clean = git_url.trim_end_matches(".git").trim_end_matches('/');
    let github_rest = clean
        .strip_prefix("https://github.com/")
        .or_else(|| clean.strip_prefix("git@github.com:"));
    if let Some(rest) = github_rest {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() == 2 {
            let (owner, repo) = (parts[0], parts[1]);
            if let Some(t) = tag {
                return Some(format!("https://github.com/{owner}/{repo}/archive/refs/tags/{t}.tar.gz"));
            }
            if let Some(b) = branch {
                return Some(format!("https://github.com/{owner}/{repo}/archive/refs/heads/{b}.tar.gz"));
            }
            return Some(format!("https://github.com/{owner}/{repo}/archive/HEAD.tar.gz"));
        }
    }
    let gitlab_rest = clean
        .strip_prefix("https://gitlab.com/")
        .or_else(|| clean.strip_prefix("git@gitlab.com:"));
    if let Some(rest) = gitlab_rest {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() == 2 {
            let (owner, repo) = (parts[0], parts[1]);
            let ref_name = tag.or(branch).unwrap_or("HEAD");
            return Some(format!("https://gitlab.com/{owner}/{repo}/-/archive/{ref_name}/{repo}-{ref_name}.tar.gz"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_archive_extension() {
        assert_eq!(detect_archive_extension("https://site.com/lib.tar.zst"), "tar.zst");
        assert_eq!(detect_archive_extension("https://site.com/lib.tar.gz?token=123"), "tar.gz");
        assert_eq!(detect_archive_extension("https://site.com/lib.tgz"), "tar.gz");
        assert_eq!(detect_archive_extension("https://site.com/lib.tar.xz"), "tar.xz");
        assert_eq!(detect_archive_extension("https://site.com/lib.zip"), "zip");
        assert_eq!(detect_archive_extension("https://site.com/lib.tar"), "tar");
        assert_eq!(detect_archive_extension("https://site.com/unknown"), "tar.gz");
    }

    #[test]
    fn test_parse_git_archive_url_github() {
        let https_url = "https://github.com/foo/bar.git";
        assert_eq!(
            parse_git_archive_url(https_url, None, None),
            Some("https://github.com/foo/bar/archive/HEAD.tar.gz".to_string())
        );
        assert_eq!(
            parse_git_archive_url(https_url, Some("v1.0.0"), None),
            Some("https://github.com/foo/bar/archive/refs/tags/v1.0.0.tar.gz".to_string())
        );
        assert_eq!(
            parse_git_archive_url(https_url, None, Some("develop")),
            Some("https://github.com/foo/bar/archive/refs/heads/develop.tar.gz".to_string())
        );

        let ssh_url = "git@github.com:foo/bar.git";
        assert_eq!(
            parse_git_archive_url(ssh_url, Some("v2.0"), None),
            Some("https://github.com/foo/bar/archive/refs/tags/v2.0.tar.gz".to_string())
        );
    }

    #[test]
    fn test_parse_git_archive_url_gitlab() {
        let https_url = "https://gitlab.com/group/repo.git";
        assert_eq!(
            parse_git_archive_url(https_url, None, None),
            Some("https://gitlab.com/group/repo/-/archive/HEAD/repo-HEAD.tar.gz".to_string())
        );
        assert_eq!(
            parse_git_archive_url(https_url, Some("v1.2.3"), None),
            Some("https://gitlab.com/group/repo/-/archive/v1.2.3/repo-v1.2.3.tar.gz".to_string())
        );

        let ssh_url = "git@gitlab.com:group/repo";
        assert_eq!(
            parse_git_archive_url(ssh_url, None, Some("main")),
            Some("https://gitlab.com/group/repo/-/archive/main/repo-main.tar.gz".to_string())
        );
    }

    #[test]
    fn test_normalize_lib_root_unwraps_single_dir() {
        let tmp = std::env::temp_dir().join(format!("yukino_norm_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("nested_folder")).unwrap();
        fs::write(tmp.join("nested_folder/lib.toml"), "[lib]\nname = \"nested\"\n").unwrap();
        fs::write(tmp.join("nested_folder/lib.ykn"), "public fun f() {}\n").unwrap();

        let norm_res = normalize_lib_root(&tmp, "nested");
        assert!(norm_res.is_ok());
        assert!(tmp.join("lib.toml").is_file());
        assert!(tmp.join("lib.ykn").is_file());
        assert!(!tmp.join("nested_folder").exists());

        let _ = fs::remove_dir_all(&tmp);
    }
}
