#![deny(unsafe_code)]

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::DriverError;

pub fn create_static_archive(
    archive_path: &Path,
    objects: &[PathBuf],
    ar_tool: Option<&str>,
) -> Result<(), DriverError> {
    if objects.is_empty() {
        return Err(DriverError::NoInputFile);
    }

    if let Some(parent) = archive_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    let tool = ar_tool.unwrap_or("llvm-ar");
    let mut cmd = Command::new(tool);
    cmd.arg("crs").arg(archive_path);
    for obj in objects {
        cmd.arg(obj);
    }

    let output = match cmd.output() {
        Ok(out) => out,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Err(DriverError::ToolNotFound(tool.to_string()));
        }
        Err(err) => return Err(DriverError::Io(err)),
    };

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(DriverError::ArchiverFailed(tool.to_string(), code, stderr));
    }

    Ok(())
}

pub fn list_archive_members(
    archive_path: &Path,
    ar_tool: Option<&str>,
) -> Result<Vec<String>, DriverError> {
    let tool = ar_tool.unwrap_or("llvm-ar");
    let mut cmd = Command::new(tool);
    cmd.arg("t").arg(archive_path);

    let output = match cmd.output() {
        Ok(out) => out,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Err(DriverError::ToolNotFound(tool.to_string()));
        }
        Err(err) => return Err(DriverError::Io(err)),
    };

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(DriverError::ArchiverFailed(tool.to_string(), code, stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let members = stdout
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect();
    Ok(members)
}
