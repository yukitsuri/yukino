#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use yukic_target::TargetTriple;

// deprecated
/* ... */

#[derive(Debug, Serialize, Deserialize, Default)]
struct RuntimeIndex {
    compiler_version: String,
    runtimes: BTreeMap<String, RuntimeRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RuntimeRecord {
    compiler_version: String,
    target: String,
    archive: String,
    sha256: String,
    #[serde(default = "default_remote_source")]
    source: RuntimeSource,
    #[serde(default)]
    last_checked: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuntimeSource {
    Local,
    Embedded,
    Remote,
}

fn default_remote_source() -> RuntimeSource {
    RuntimeSource::Remote
}

#[derive(Debug)]
pub enum RuntimeError {
    Io(io::Error),
    EmptyList,
    NoRemovable,
    InvalidTarget(String),
    InvalidArchive(String),
    NotInstalled(String),
    Remote(String),
    UserDeclined,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::EmptyList => write!(f, "no runtimes installed"),
            Self::NoRemovable => write!(f, "no removable runtimes"),
            Self::InvalidTarget(target) => write!(f, "unsupported target '{target}'"),
            Self::InvalidArchive(path) => write!(f, "invalid runtime archive '{path}'"),
            Self::NotInstalled(target) => write!(f, "runtime is not installed for '{target}'"),
            Self::Remote(message) => write!(f, "{message}"),
            Self::UserDeclined => write!(f, "installation cancelled"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<io::Error> for RuntimeError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn install_local(target: &str, source: &Path) -> Result<PathBuf, RuntimeError> {
    let target = parse_target(target)?;
    validate_archive(source)?;
    let paths = RuntimePaths::new()?;
    let destination = paths.archive_path(target);
    fs::create_dir_all(&paths.runtime_dir)?;
    fs::copy(source, &destination)?;
    let hash = sha256_file(&destination)?;
    let mut index = paths.load_index()?;
    index.compiler_version = crate::VERSION.to_string();
    index.runtimes.insert(
        target.as_str().to_string(),
        RuntimeRecord {
            compiler_version: crate::VERSION.to_string(),
            target: target.as_str().to_string(),
            archive: destination.display().to_string(),
            sha256: hash,
            source: RuntimeSource::Local,
            last_checked: unix_timestamp(),
        },
    );
    paths.save_index(&index)?;
    Ok(destination)
}

pub fn remove(target: Option<&str>) -> Result<Vec<String>, RuntimeError> {
    let paths = RuntimePaths::new()?;
    let mut index = paths.load_index()?;
    let explicit_target = target.is_some();
    let targets: Vec<TargetTriple> = match target {
        Some(value) => vec![parse_target(value)?],
        None => TargetTriple::all()
            .iter()
            .copied()
            .filter(|item| *item != TargetTriple::host())
            .collect(),
    };
    let mut removed = Vec::new();
    for item in targets {
        if item == TargetTriple::host() && !explicit_target {
            continue;
        }
        let path = paths.archive_path(item);
        if path.is_file() {
            fs::remove_file(path)?;
            index.runtimes.remove(item.as_str());
            removed.push(item.as_str().to_string());
        }
    }
    paths.save_index(&index)?;
    if removed.is_empty() {
        return Err(match target {
            Some(value) => RuntimeError::NotInstalled(value.to_string()),
            None => RuntimeError::NoRemovable,
        });
    }
    Ok(removed)
}

pub fn installed_archive(target: &str) -> Result<PathBuf, RuntimeError> {
    let target = parse_target(target)?;
    let paths = RuntimePaths::new()?;
    let archive = paths.archive_path(target);
    if !archive.is_file() {
        return Err(RuntimeError::NotInstalled(target.as_str().to_string()));
    }
    let index = paths.load_index()?;
    let record = index
        .runtimes
        .get(target.as_str())
        .ok_or_else(|| RuntimeError::NotInstalled(target.as_str().to_string()))?;
    if matches!(record.source, RuntimeSource::Local | RuntimeSource::Embedded) {
        return Ok(archive);
    }
    let actual_hash = sha256_file(&archive)?;
    if actual_hash != record.sha256 {
        return Err(RuntimeError::Remote(format!(
            "runtime archive for '{}' has changed, reinstall it",
            target.as_str()
        )));
    }
    /*
    if unix_timestamp().saturating_sub(record.last_checked) >= REMOTE_CHECK_INTERVAL {
        let (_, expected_hash) = load_remote_config()?
            .get(target.as_str())
            .cloned()
            .ok_or_else(|| RuntimeError::Remote(format!("no remote runtime for '{}'", target.as_str())))?;
        if actual_hash != expected_hash {
            return Err(RuntimeError::Remote(format!(
                "runtime archive for '{}' is outdated, reinstall it",
                target.as_str()
            )));
        }
        if let Some(record) = index.runtimes.get_mut(target.as_str()) {
            record.last_checked = unix_timestamp();
        }
        paths.save_index(&index)?;
    }
    */
    Ok(archive)
}

pub fn archive_path(target: &str) -> Result<PathBuf, RuntimeError> {
    let target = parse_target(target)?;
    Ok(RuntimePaths::new()?.archive_path(target))
}

pub fn register_embedded(target: &str, archive: &Path) -> Result<PathBuf, RuntimeError> {
    let target = parse_target(target)?;
    validate_archive(archive)?;
    let paths = RuntimePaths::new()?;
    let destination = paths.archive_path(target);
    if archive != destination {
        fs::create_dir_all(&paths.runtime_dir)?;
        fs::copy(archive, &destination)?;
    }
    let mut index = paths.load_index()?;
    index.compiler_version = crate::VERSION.to_string();
    index.runtimes.insert(
        target.as_str().to_string(),
        RuntimeRecord {
            compiler_version: crate::VERSION.to_string(),
            target: target.as_str().to_string(),
            archive: destination.display().to_string(),
            sha256: sha256_file(&destination)?,
            source: RuntimeSource::Embedded,
            last_checked: unix_timestamp(),
        },
    );
    paths.save_index(&index)?;
    Ok(destination)
}

pub fn install_remote(target: Option<&str>) -> Result<Vec<PathBuf>, RuntimeError> {
    let targets = selected_targets(target)?;
    let paths = RuntimePaths::new()?;
    fs::create_dir_all(&paths.runtime_dir)?;
    yukino_builtin::unpack_runtime_source(&paths.runtime_dir)
        .map_err(|e| RuntimeError::Remote(e.to_string()))?;
    let mut installed = Vec::new();
    for item in targets {
        let destination = paths.archive_path(item);
        yukino_builtin::compile_runtime_archive(&paths.runtime_dir, item, &destination)
            .map_err(|e| RuntimeError::Remote(e.to_string()))?;
        let hash = sha256_file(&destination)?;
        let mut index = paths.load_index()?;
        index.compiler_version = crate::VERSION.to_string();
        index.runtimes.insert(
            item.as_str().to_string(),
            RuntimeRecord {
                compiler_version: crate::VERSION.to_string(),
                target: item.as_str().to_string(),
                archive: destination.display().to_string(),
                sha256: hash,
                source: RuntimeSource::Embedded,
                last_checked: unix_timestamp(),
            },
        );
        paths.save_index(&index)?;
        installed.push(destination);
    }
    Ok(installed)
}

/*
pub fn install_remote_old(target: Option<&str>) -> Result<Vec<PathBuf>, RuntimeError> {
    let targets = selected_targets(target)?;
    let config = load_remote_config()?;
    let paths = RuntimePaths::new()?;
    fs::create_dir_all(&paths.runtime_dir)?;
    let mut installed = Vec::new();
    for item in targets {
        let target_name = item.as_str();
        let (template, expected_hash) = config
            .get(target_name)
            .ok_or_else(|| RuntimeError::Remote(format!("no remote runtime for '{target_name}'")))?;
        let url = template.replace("{target}", target_name);
        let destination = paths.archive_path(item);
        let temporary = destination.with_extension("a.download");
        download_archive(&url, &temporary)?;
        let actual_hash = sha256_file(&temporary)?;
        if actual_hash != *expected_hash {
            let _ = fs::remove_file(&temporary);
            return Err(RuntimeError::Remote(format!(
                "SHA-256 mismatch for '{target_name}': expected {expected_hash}, got {actual_hash}"
            )));
        }
        fs::rename(&temporary, &destination)?;
        let mut index = paths.load_index()?;
        index.compiler_version = crate::VERSION.to_string();
        index.runtimes.insert(
            target_name.to_string(),
            RuntimeRecord {
                compiler_version: crate::VERSION.to_string(),
                target: target_name.to_string(),
                archive: destination.display().to_string(),
                sha256: actual_hash,
                source: RuntimeSource::Remote,
                last_checked: unix_timestamp(),
            },
        );
        paths.save_index(&index)?;
        installed.push(destination);
    }
    Ok(installed)
}
*/

pub fn reinstall() -> Result<Vec<PathBuf>, RuntimeError> {
    let targets = list()?
        .into_iter()
        .map(|record| record.target)
        .collect::<Vec<_>>();
    if targets.is_empty() {
        return Err(RuntimeError::EmptyList);
    }
    let names = targets.join(",");
    install_remote(Some(&names))
}

pub fn list() -> Result<Vec<RuntimeRecordView>, RuntimeError> {
    let paths = RuntimePaths::new()?;
    let index = paths.load_index()?;
    let records = index
        .runtimes
        .values()
        .map(|record| RuntimeRecordView {
            target: record.target.clone(),
            compiler_version: record.compiler_version.clone(),
            archive: record.archive.clone(),
            sha256: record.sha256.clone(),
            source: record.source.clone(),
        })
        .collect::<Vec<_>>();
    if records.is_empty() {
        return Err(RuntimeError::EmptyList);
    }
    Ok(records)
}

pub fn info(target: Option<&str>) -> Result<Vec<RuntimeRecordView>, RuntimeError> {
    let records = list()?;
    match target {
        Some(value) => {
            parse_target(value)?;
            let records = records
                .into_iter()
                .filter(|item| item.target == value)
                .collect::<Vec<_>>();
            if records.is_empty() {
                return Err(RuntimeError::NotInstalled(value.to_string()));
            }
            Ok(records)
        }
        None if records.is_empty() => Err(RuntimeError::EmptyList),
        None => Ok(records),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRecordView {
    pub target: String,
    pub compiler_version: String,
    pub archive: String,
    pub sha256: String,
    pub source: RuntimeSource,
}

impl RuntimeSource {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Embedded => "embedded",
            Self::Remote => "remote",
        }
    }
}

fn parse_target(value: &str) -> Result<TargetTriple, RuntimeError> {
    TargetTriple::parse(value).map_err(|_| RuntimeError::InvalidTarget(value.to_string()))
}

fn validate_archive(path: &Path) -> Result<(), RuntimeError> {
    if !path.is_file() {
        return Err(RuntimeError::InvalidArchive(path.display().to_string()));
    }
    let data = fs::read(path)?;
    if !data.starts_with(b"!<arch>\n") {
        return Err(RuntimeError::InvalidArchive(path.display().to_string()));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, RuntimeError> {
    let data = fs::read(path)?;
    let digest = Sha256::digest(data);
    Ok(format!("{digest:x}"))
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn selected_targets(target: Option<&str>) -> Result<Vec<TargetTriple>, RuntimeError> {
    match target {
        Some(value) if value.contains(',') => value
            .split(',')
            .map(str::trim)
            .map(parse_target)
            .collect(),
        Some(value) => Ok(vec![parse_target(value)?]),
        None => Ok(TargetTriple::all().to_vec()),
    }
}

/*
fn load_remote_config() -> Result<BTreeMap<String, (String, String)>, RuntimeError> {
    let output = Command::new("curl")
        .args(["-sSfL", "--max-time", "20", REMOTE_CONFIG_URL])
        .output()
        .map_err(|_| RuntimeError::Remote("cannot reach runtime server".to_string()))?;
    if !output.status.success() {
        return Err(RuntimeError::Remote("cannot reach runtime server".to_string()));
    }
    let content = String::from_utf8(output.stdout)
        .map_err(|_| RuntimeError::Remote("invalid runtime server response".to_string()))?;
    let parsed: toml::Value = toml::from_str(&content)
        .map_err(|_| RuntimeError::Remote("invalid runtime server response".to_string()))?;
    let table = parsed
        .get("rt")
        .and_then(|value| value.get(crate::VERSION))
        .and_then(toml::Value::as_table)
        .ok_or_else(|| RuntimeError::Remote("runtime is unavailable for this compiler".to_string()))?;
    let mut runtimes = BTreeMap::new();
    for (target, value) in table {
        let pair = value.as_array().ok_or_else(|| {
            RuntimeError::Remote(format!("invalid runtime entry for '{target}'"))
        })?;
        let url = pair.first().and_then(toml::Value::as_str).ok_or_else(|| {
            RuntimeError::Remote(format!("missing runtime URL for '{target}'"))
        })?;
        let hash = pair.get(1).and_then(toml::Value::as_str).ok_or_else(|| {
            RuntimeError::Remote(format!("missing runtime SHA-256 for '{target}'"))
        })?;
        runtimes.insert(target.clone(), (url.to_string(), hash.to_string()));
    }
    Ok(runtimes)
}

fn download_archive(url: &str, destination: &Path) -> Result<(), RuntimeError> {
    let mut response = Command::new("curl")
        .args(["-fL", "--progress-bar", "--max-time", "300", "-o"])
        .arg(destination)
        .arg(url)
        .spawn()
        .map_err(|_| RuntimeError::Remote("cannot download runtime: check internet connection".to_string()))?;
    let status = response
        .wait()
        .map_err(|_| RuntimeError::Remote("cannot download runtime: check internet connection".to_string()))?;
    if !status.success() {
        return Err(RuntimeError::Remote("cannot download runtime: check internet connection".to_string()));
    }
    Ok(())
}
*/

struct RuntimePaths {
    root: PathBuf,
    runtime_dir: PathBuf,
    index_path: PathBuf,
}

impl RuntimePaths {
    fn new() -> Result<Self, RuntimeError> {
        let root = data_root()?.join(crate::YUKINO_CFG_DIR);
        let libs_dir = root.join("libs");
        Ok(Self {
            runtime_dir: libs_dir.join("runtime"),
            index_path: libs_dir.join("runtime.json"),
            root,
        })
    }

    fn archive_path(&self, target: TargetTriple) -> PathBuf {
        let name = if target == TargetTriple::host() {
            "host.a".to_string()
        } else {
            format!("{}.a", target.as_str())
        };
        self.runtime_dir.join(name)
    }

    fn load_index(&self) -> Result<RuntimeIndex, RuntimeError> {
        if !self.index_path.is_file() {
            return Ok(RuntimeIndex {
                compiler_version: crate::VERSION.to_string(),
                runtimes: BTreeMap::new(),
            });
        }
        let content = fs::read_to_string(&self.index_path)?;
        serde_json::from_str(&content).map_err(|error| RuntimeError::Remote(error.to_string()))
    }

    fn save_index(&self, index: &RuntimeIndex) -> Result<(), RuntimeError> {
        let libs_dir = self.root.join("libs");
        fs::create_dir_all(&libs_dir)?;
        let content = serde_json::to_string_pretty(index)
            .map_err(|error| RuntimeError::Remote(error.to_string()))?;
        fs::write(&self.index_path, format!("{content}\n"))?;
        Ok(())
    }
}

fn data_root() -> Result<PathBuf, RuntimeError> {
    #[cfg(target_os = "windows")]
    let base = env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base = env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
    base.ok_or_else(|| RuntimeError::Remote("cannot determine application data directory".to_string()))
}
