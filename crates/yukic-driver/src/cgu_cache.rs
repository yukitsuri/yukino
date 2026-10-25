#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use inkwell::targets::RelocMode;
use serde::{Deserialize, Serialize};
use yukic_opt::{OptLevel, OptimizedCgu};
use yukic_target::TargetTriple;

pub const MANIFEST_FILENAME: &str = "cgu_manifest.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CguCacheEntry {
    pub hash: String,
    pub object_file: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CguCacheManifest {
    pub version: u32,
    pub compiler_version: String,
    pub target: String,
    pub profile: String,
    pub opt_level: String,
    pub entries: BTreeMap<String, CguCacheEntry>,
}

impl CguCacheManifest {
    pub fn new(target: &str, profile: &str, opt_level: &str) -> Self {
        Self {
            version: 1,
            compiler_version: env!("CARGO_PKG_VERSION").to_string(),
            target: target.to_string(),
            profile: profile.to_string(),
            opt_level: opt_level.to_string(),
            entries: BTreeMap::new(),
        }
    }

    pub fn load_or_create(dir: &Path, target: &str, profile: &str, opt_level: &str) -> Self {
        let manifest_path = dir.join(MANIFEST_FILENAME);
        if let Ok(content) = fs::read_to_string(&manifest_path)
            && let Ok(parsed) = serde_json::from_str::<Self>(&content)
            && parsed.is_compatible(target, profile, opt_level)
        {
            return parsed;
        }
        Self::new(target, profile, opt_level)
    }

    fn is_compatible(&self, target: &str, profile: &str, opt_level: &str) -> bool {
        self.version == 1
            && self.compiler_version == env!("CARGO_PKG_VERSION")
            && self.target == target
            && self.profile == profile
            && self.opt_level == opt_level
    }

    pub fn compute_cgu_hash(
        cgu: &OptimizedCgu,
        target: TargetTriple,
        opt_level: OptLevel,
        reloc: RelocMode,
        extra_dependency_hashes: &[&str],
    ) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(cgu.name.as_bytes());
        hasher.update(b"\0");
        hasher.update(cgu.ll_ir.as_bytes());
        hasher.update(b"\0");
        hasher.update(target.as_str().as_bytes());
        hasher.update(b"\0");
        hasher.update(opt_level.as_str().as_bytes());
        hasher.update(b"\0");
        let reloc_str = match reloc {
            RelocMode::Default => "default",
            RelocMode::Static => "static",
            RelocMode::PIC => "pic",
            RelocMode::DynamicNoPic => "dynamic-no-pic",
        };
        hasher.update(reloc_str.as_bytes());
        hasher.update(b"\0");
        hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
        for dep in extra_dependency_hashes {
            hasher.update(b"\0dep:");
            hasher.update(dep.as_bytes());
        }
        format!("blake3:{}", hasher.finalize().to_hex())
    }

    pub fn is_cached(&self, dir: &Path, cgu_name: &str, expected_hash: &str) -> Option<PathBuf> {
        let entry = self.entries.get(cgu_name)?;
        if entry.hash != expected_hash {
            return None;
        }
        let obj_path = dir.join(&entry.object_file);
        if obj_path.is_file() {
            Some(obj_path)
        } else {
            None
        }
    }

    pub fn record(&mut self, cgu_name: String, hash: String, object_file: String) {
        self.entries.insert(
            cgu_name,
            CguCacheEntry {
                hash,
                object_file,
            },
        );
    }

    pub fn save_atomic(&self, dir: &Path) -> Result<(), std::io::Error> {
        fs::create_dir_all(dir)?;
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        let tmp_path = dir.join(format!("{MANIFEST_FILENAME}.tmp.{}", std::process::id()));
        fs::write(&tmp_path, json)?;
        fs::rename(&tmp_path, dir.join(MANIFEST_FILENAME))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_cgu_hash_deterministic() {
        let cgu1 = OptimizedCgu {
            name: "cgu_0".to_string(),
            ll_ir: "define void @main() { ret void }".to_string(),
            bitcode: None,
            item_count: 1,
            estimated_weight: 10,
        };
        let cgu2 = cgu1.clone();
        let target = TargetTriple::host();
        let h1 = CguCacheManifest::compute_cgu_hash(&cgu1, target, OptLevel::O0, RelocMode::PIC, &[]);
        let h2 = CguCacheManifest::compute_cgu_hash(&cgu2, target, OptLevel::O0, RelocMode::PIC, &[]);
        assert_eq!(h1, h2);
        assert!(h1.starts_with("blake3:"));

        let h_dep = CguCacheManifest::compute_cgu_hash(
            &cgu1,
            target,
            OptLevel::O0,
            RelocMode::PIC,
            &["blake3:macro_runner_bin_123"],
        );
        assert_ne!(h1, h_dep);

        let mut cgu_diff = cgu1.clone();
        cgu_diff.ll_ir = "define void @main() { ret void }\n; comment".to_string();
        let h3 = CguCacheManifest::compute_cgu_hash(&cgu_diff, target, OptLevel::O0, RelocMode::PIC, &[]);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_manifest_load_save_atomic() {
        let tmp = std::env::temp_dir().join(format!("cgu_test_save_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).expect("tmp dir");

        let mut manifest = CguCacheManifest::new("x86_64-unknown-linux-gnu", "debug", "0");
        manifest.record(
            "cgu_0".to_string(),
            "blake3:abc12345".to_string(),
            "cgu_0.o".to_string(),
        );
        manifest.save_atomic(&tmp).expect("save atomic");

        let loaded = CguCacheManifest::load_or_create(&tmp, "x86_64-unknown-linux-gnu", "debug", "0");
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries.get("cgu_0").unwrap().hash, "blake3:abc12345");

        let mismatch = CguCacheManifest::load_or_create(&tmp, "x86_64-unknown-linux-gnu", "release", "2");
        assert!(mismatch.entries.is_empty());

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_is_cached_requires_actual_file() {
        let tmp = std::env::temp_dir().join(format!("cgu_test_file_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).expect("tmp dir");

        let mut manifest = CguCacheManifest::new("host", "debug", "0");
        manifest.record(
            "cgu_0".to_string(),
            "blake3:xyz999".to_string(),
            "cgu_0.o".to_string(),
        );

        // object file does not exist yet -> not cached
        assert!(manifest.is_cached(&tmp, "cgu_0", "blake3:xyz999").is_none());

        // create object file
        fs::write(tmp.join("cgu_0.o"), b"elf").expect("write obj");
        let cached = manifest.is_cached(&tmp, "cgu_0", "blake3:xyz999");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), tmp.join("cgu_0.o"));

        // wrong hash -> not cached
        assert!(manifest.is_cached(&tmp, "cgu_0", "blake3:wrong").is_none());

        let _ = fs::remove_dir_all(&tmp);
    }
}
