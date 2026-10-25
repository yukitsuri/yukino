use crate::error::BuiltinError;
use crate::{
    MACROSDK_FILES, MACROSDK_SOURCE_HASH, RUNTIME_FILES, RUNTIME_SOURCE_HASH, STDLIB_FILES,
    STDLIB_SOURCE_HASH,
};
use std::fs;
use std::path::{Path, PathBuf};

fn unpack_files(
    destination_dir: &Path,
    files: &[(&str, &[u8])],
    hash: &str,
    probe_file: &str,
) -> Result<PathBuf, BuiltinError> {
    let stamp_file = destination_dir.join(".source_hash");
    if stamp_file.is_file()
        && let Ok(existing_hash) = fs::read_to_string(&stamp_file)
        && existing_hash.trim() == hash
        && destination_dir.join(probe_file).is_file()
    {
        return Ok(destination_dir.to_path_buf());
    }

    if destination_dir.exists() {
        let _ = fs::remove_dir_all(destination_dir.join("core"));
        let _ = fs::remove_dir_all(destination_dir.join("std"));
        let _ = fs::remove_dir_all(destination_dir.join("src"));
    }
    fs::create_dir_all(destination_dir)?;
    for (rel_path, bytes) in files {
        let target_path = destination_dir.join(rel_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if *rel_path == "Cargo.toml" {
            let s = String::from_utf8_lossy(bytes);
            let with_ws = format!("[workspace]\n\n{s}");
            fs::write(&target_path, with_ws.as_bytes())?;
        } else {
            fs::write(&target_path, bytes)?;
        }
    }
    fs::write(&stamp_file, hash)?;
    Ok(destination_dir.to_path_buf())
}

pub fn unpack_runtime_source(destination_dir: &Path) -> Result<PathBuf, BuiltinError> {
    unpack_files(destination_dir, RUNTIME_FILES, RUNTIME_SOURCE_HASH, "src/lib.rs")
}

pub fn unpack_stdlib_source(destination_dir: &Path) -> Result<PathBuf, BuiltinError> {
    unpack_files(destination_dir, STDLIB_FILES, STDLIB_SOURCE_HASH, "std/io.ykn")
}

pub fn unpack_macrosdk_source(destination_dir: &Path) -> Result<PathBuf, BuiltinError> {
    unpack_files(destination_dir, MACROSDK_FILES, MACROSDK_SOURCE_HASH, "macrosdk/lib.ykn")
}
