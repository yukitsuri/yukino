#![deny(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::project::ProjectError;

pub const LOCKFILE_REL_PATH: &str = "yuki-work/common/yuki.lock";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lockfile {
    pub version: u32,
    pub target: String,
    pub compiler: String,
    pub profile: String,
    #[serde(default)]
    pub package: Vec<LockedPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub kind: String,
    pub source: String,
    pub checksum: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
}

pub(crate) fn should_skip_dir(name: &str) -> bool {
    name.starts_with('.')
        || name == "yuki-work"
        || name == "target"
        || name == "build"
        || name == "node_modules"
}

#[allow(clippy::collapsible_if)]
pub(crate) fn collect_dir_sources(dir: &Path, base: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut sub_entries: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    sub_entries.sort();

    for path in sub_entries {
        if path.is_dir() {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !should_skip_dir(name) {
                collect_dir_sources(&path, base, out);
            }
        } else if path.extension().is_some_and(|ext| ext == "ykn") {
            if let Ok(rel) = path.strip_prefix(base) {
                out.push(rel.to_path_buf());
            }
        }
    }
}

pub fn compute_source_dir_checksum(
    dir: &Path,
    manifest_name: &str,
    extra_tags: &[&str],
) -> Result<String, std::io::Error> {
    let mut hasher = blake3::Hasher::new();

    for tag in extra_tags {
        hasher.update(tag.as_bytes());
        hasher.update(b"\0");
    }

    let manifest_path = dir.join(manifest_name);
    if manifest_path.is_file() {
        let content = fs::read(&manifest_path)?;
        hasher.update(b"manifest:");
        hasher.update(&content);
    }

    let mut rel_files = Vec::new();
    collect_dir_sources(dir, dir, &mut rel_files);
    rel_files.sort();

    for rel in rel_files {
        hasher.update(rel.to_string_lossy().as_bytes());
        let full = dir.join(&rel);
        let content = fs::read(full)?;
        hasher.update(&content);
    }

    let hex = hasher.finalize().to_hex();
    Ok(format!("blake3:{hex}"))
}

pub fn compute_archive_checksum(archive_path: &Path) -> Result<String, std::io::Error> {
    let bytes = fs::read(archive_path)?;
    let hex = blake3::hash(&bytes).to_hex();
    Ok(format!("blake3:{hex}"))
}

impl Lockfile {
    pub fn new(target: &str, compiler: &str, profile: &str) -> Self {
        Self {
            version: 1,
            target: target.to_string(),
            compiler: compiler.to_string(),
            profile: profile.to_string(),
            package: Vec::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Option<Self>, ProjectError> {
        if !path.is_file() {
            return Ok(None);
        }
        let content = fs::read_to_string(path)
            .map_err(|e| ProjectError::Io(format!("cannot read {}: {e}", path.display())))?;
        let lock: Self = toml::from_str(&content)
            .map_err(|e| ProjectError::Io(format!("invalid lockfile {}: {e}", path.display())))?;
        Ok(Some(lock))
    }

    pub fn save(&self, path: &Path) -> Result<(), ProjectError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ProjectError::Io(format!("cannot create dir {}: {e}", parent.display())))?;
        }
        let serialized = toml::to_string_pretty(self)
            .map_err(|e| ProjectError::Io(format!("cannot serialize lockfile: {e}")))?;
        fs::write(path, serialized)
            .map_err(|e| ProjectError::Io(format!("cannot write {}: {e}", path.display())))?;
        Ok(())
    }

    pub fn find_package(&self, name: &str) -> Option<&LockedPackage> {
        self.package.iter().find(|p| p.name == name)
    }

    pub fn verify_checksum(&self, name: &str, expected_checksum: &str) -> bool {
        self.find_package(name)
            .is_some_and(|p| p.checksum == expected_checksum)
    }

    pub fn upsert_package(&mut self, package: LockedPackage) {
        if let Some(idx) = self.package.iter().position(|p| p.name == package.name) {
            self.package[idx] = package;
        } else {
            self.package.push(package);
        }
        self.package.sort_by(|a, b| a.name.cmp(&b.name));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lockfile_roundtrip_serialization() {
        let mut lock = Lockfile::new("x86_64-unknown-linux-gnu", "0.1.0", "release");
        lock.upsert_package(LockedPackage {
            name: "app".to_string(),
            version: "0.1.0".to_string(),
            kind: "app".to_string(),
            source: "root".to_string(),
            checksum: "blake3:1234567890abcdef".to_string(),
            commit: None,
            features: vec!["default".to_string()],
            dependencies: vec!["std".to_string()],
        });

        let toml_str = toml::to_string_pretty(&lock).expect("serialize lockfile");
        let parsed: Lockfile = toml::from_str(&toml_str).expect("deserialize lockfile");
        assert_eq!(lock, parsed);
        assert!(parsed.verify_checksum("app", "blake3:1234567890abcdef"));
        assert!(!parsed.verify_checksum("app", "blake3:mismatch"));
    }

    #[test]
    fn test_compute_source_dir_checksum_deterministic() {
        let tmp = std::env::temp_dir().join(format!("test_blake3_{}", std::process::id()));
        let src = tmp.join("src");
        fs::create_dir_all(&src).expect("create dir");
        fs::write(tmp.join("yuki.toml"), "[bin]\nname = \"demo\"\n").expect("write manifest");
        fs::write(src.join("main.ykn"), "fun main() {}\n").expect("write main");

        let hash1 = compute_source_dir_checksum(&tmp, "yuki.toml", &["0.1.0", "release"]).expect("hash1");
        let hash2 = compute_source_dir_checksum(&tmp, "yuki.toml", &["0.1.0", "release"]).expect("hash2");
        assert_eq!(hash1, hash2);
        assert!(hash1.starts_with("blake3:"));

        fs::write(src.join("main.ykn"), "fun main() { // change\n}\n").expect("write change");
        let hash3 = compute_source_dir_checksum(&tmp, "yuki.toml", &["0.1.0", "release"]).expect("hash3");
        assert_ne!(hash1, hash3);

        let _ = fs::remove_dir_all(tmp);
    }
}
