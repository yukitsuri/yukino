#![deny(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::project::ProjectError;
use crate::semver::{SemVer, VersionRange};

// yukino new --app appname
pub fn generate_app_manifest(name: &str, version: &str) -> String {
    let name = toml_escape(name);
    format!(
        r#"[bin]
name = "{name}"
namedbg = "{name}_debug"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["{version}", "+"] # [min, max]
includes = ["stdlib"]

[test]
threads = "auto"
timeout_ms = 10000 
capture_output = true
fail_fast = false
tests_dir = "tests"

[config.release]
inline_statics = true
strip_unused = true
opt_level = 3
lto = true
debug_assertions = false
panic_strategy = "unwind"
strip = true
function_sections = true

# LLVM config
cpu = "native"
features = ["+avx2"]
reloc_model = "pic"
code_model = "default"
fast_math = false
emit = ["bin"]

[features]
default = []

[libs.runtime]
# example = {{ link = "https://.../lib.tar.zst" }}

[libs.build]

[libs.static]
objects = []
archives = []
search_paths = []
system = []
"#
    )
}

// yukino new --static libname
// yukino new --dynamic libname

pub fn generate_lib_manifest(name: &str, version: &str, output: &str) -> String {
    let name = toml_escape(name);
    let includes_str = if output == "macro" {
        "includes = [\"stdlib\", \"macrosdk\"]"
    } else {
        "includes = [\"stdlib\"]"
    };
    format!(
        r#"[lib]
name = "{name}"
version = "0.1.0"
entry = "{name}/lib.ykn"
yukino = ["{version}", "+"] # [min, max]
output = "{output}"
{includes_str}

[test]
threads = "auto"
timeout_ms = 10000
capture_output = true
fail_fast = false
tests_dir = "tests"

[features]
default = []

[libs.runtime]
# example = {{ link = "https://.../lib.tar.zst" }}

[libs.build]

[libs.static]
archives = []
system = []
"#
    )
}

pub fn generate_workspace_manifest(name: &str) -> String {
    format!(
        r#"[workspace]
name = "{name}"
members = [
    "libs/*",
    "apps/*",
]

[workspace.build]
archives = []
system = []
"#
    )
}

pub fn generate_main_ykn() -> &'static str {
    "load std.io: print\n\nfun main() {\n    print(\"Hello, Yukino Yukinoshita!\\n\")\n}\n"
}

pub fn generate_static_lib_ykn() -> &'static str {
    "// \"fun\" for Yukino function, \"cfun\" for c abi function\n\npublic fun calc(a: int64, b: int64) -> int64 {\n      return a + b\n}\n"
}

pub fn generate_dynamic_lib_ykn() -> &'static str {
    "// c abi lib\n\npublic cfun calc(a: c_longlong, b: c_longlong) -> c_longlong {\n      return a + b\n}\n"
}

pub fn generate_macro_lib_ykn() -> &'static str {
    "// procedural macro\nload macrosdk: MacroRunner, MacroContext, TokenStream, token_stream\n\nfun expand_macro(ctx: MacroContext, input: TokenStream) -> TokenStream {\n    return token_stream(\"// generated code\")\n}\n\nfun main() {\n    set runner = init MacroRunner()\n    set _ = runner.register(\"my_macro\", expand_macro)\n    set _ = runner.run()\n}\n"
}

pub fn generate_lib_ykn() -> &'static str { generate_static_lib_ykn() }
fn default_static() -> String { "static".to_string() }
fn default_includes() -> Vec<String> { vec!["stdlib".to_string()] }

/// Escapes a value for a double-quoted TOML basic string.
fn toml_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinSection {
    pub name: String,
    pub namedbg: Option<String>,
    pub version: String,
    pub entry: PathBuf,
    pub yukino: Vec<String>,
    #[serde(default, alias = "type", alias = "kind")]
    pub output: Option<String>,
    #[serde(default = "default_includes")]
    pub includes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibSection {
    pub name: String,
    pub version: String,
    pub entry: PathBuf,
    pub yukino: Vec<String>,
    #[serde(default = "default_static", alias = "type", alias = "kind")]
    pub output: String,
    #[serde(default = "default_includes")]
    pub includes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TestSection {
    pub threads: Option<toml::Value>,
    pub timeout_ms: Option<u64>,
    pub capture_output: Option<bool>,
    pub fail_fast: Option<bool>,
    pub tests_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProfileConfig {
    pub inline_statics: Option<bool>,
    pub strip_unused: Option<bool>,
    pub opt_level: Option<toml::Value>,
    pub lto: Option<toml::Value>,
    pub debug_assertions: Option<bool>,
    pub panic_strategy: Option<String>,
    pub strip: Option<bool>,
    #[serde(default, alias = "function-sections")]
    pub function_sections: Option<bool>,
    #[serde(default, alias = "codegen-units", alias = "cgunits")]
    pub codegen_units: Option<usize>,
    pub cpu: Option<String>,
    pub features: Option<Vec<String>>,
    pub reloc_model: Option<String>,
    pub code_model: Option<String>,
    pub fast_math: Option<bool>,
    pub emit: Option<Vec<String>>,
}

impl ProfileConfig {
    pub fn default_debug() -> Self {
        Self {
            inline_statics: Some(false),
            strip_unused: Some(false),
            opt_level: Some(toml::Value::String("0".to_string())),
            lto: Some(toml::Value::Boolean(false)),
            debug_assertions: Some(true),
            panic_strategy: Some("unwind".to_string()),
            strip: Some(false),
            function_sections: Some(false),
            codegen_units: None,
            cpu: None,
            features: None,
            reloc_model: None,
            code_model: None,
            fast_math: Some(false),
            emit: Some(vec!["bin".to_string()]),
        }
    }

    pub fn default_release() -> Self {
        Self {
            inline_statics: Some(true),
            strip_unused: Some(true),
            opt_level: Some(toml::Value::String("3".to_string())),
            lto: Some(toml::Value::Boolean(true)),
            debug_assertions: Some(false),
            panic_strategy: Some("unwind".to_string()),
            strip: Some(true),
            function_sections: Some(true),
            codegen_units: None,
            cpu: Some("native".to_string()),
            features: None,
            reloc_model: None,
            code_model: None,
            fast_math: Some(false),
            emit: Some(vec!["bin".to_string()]),
        }
    }

    pub fn opt_level_str(&self) -> &str {
        match &self.opt_level {
            Some(toml::Value::Integer(i)) => match i {
                1 => "1",
                2 => "2",
                3 => "3",
                _ => "0",
            },
            Some(toml::Value::String(s)) => s.as_str(),
            _ => "0",
        }
    }

    pub fn lto_enabled(&self) -> bool {
        match &self.lto {
            Some(toml::Value::Boolean(b)) => *b,
            Some(toml::Value::String(s)) => s != "none" && s != "false",
            _ => false,
        }
    }

    pub fn strip_enabled(&self) -> bool {
        self.strip.unwrap_or(false)
    }

    pub fn strip_unused_enabled(&self) -> bool {
        self.strip_unused.unwrap_or(false)
    }

    pub fn function_sections_enabled(&self) -> bool {
        self.function_sections.unwrap_or(false)
    }

    pub fn apply_override(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "opt_level" => {
                if !["0", "1", "2", "3", "s", "z"].contains(&value) {
                    return Err(format!("invalid opt_level '{value}'"));
                }
                self.opt_level = Some(toml::Value::String(value.to_string()));
            }
            "lto" => {
                let parsed = match value {
                    "true" | "fat" => toml::Value::Boolean(true),
                    "false" | "none" => toml::Value::Boolean(false),
                    "thin" => toml::Value::String("thin".to_string()),
                    _ => return Err(format!("invalid lto '{value}'")),
                };
                self.lto = Some(parsed);
            }
            "strip" => {
                let b = value.parse::<bool>().map_err(|_| format!("invalid strip bool '{value}'"))?;
                self.strip = Some(b);
            }
            "function_sections" | "function-sections" => {
                let b = value.parse::<bool>().map_err(|_| format!("invalid function_sections bool '{value}'"))?;
                self.function_sections = Some(b);
            }
            "codegen_units" | "codegen-units" | "cgunits" => {
                let u = value.parse::<usize>().map_err(|_| format!("invalid codegen_units integer '{value}'"))?;
                self.codegen_units = Some(u);
            }
            "strip_unused" => {
                let b = value.parse::<bool>().map_err(|_| format!("invalid strip_unused bool '{value}'"))?;
                self.strip_unused = Some(b);
            }
            "debug_assertions" => {
                let b = value.parse::<bool>().map_err(|_| format!("invalid debug_assertions bool '{value}'"))?;
                self.debug_assertions = Some(b);
            }
            "fast_math" => {
                let b = value.parse::<bool>().map_err(|_| format!("invalid fast_math bool '{value}'"))?;
                self.fast_math = Some(b);
            }
            "inline_statics" => {
                let b = value.parse::<bool>().map_err(|_| format!("invalid inline_statics bool '{value}'"))?;
                self.inline_statics = Some(b);
            }
            "panic_strategy" => {
                if value != "unwind" && value != "abort" {
                    return Err(format!("invalid panic_strategy '{value}'"));
                }
                self.panic_strategy = Some(value.to_string());
            }
            "reloc_model" => self.reloc_model = Some(value.to_string()),
            "code_model" => self.code_model = Some(value.to_string()),
            "cpu" => self.cpu = Some(value.to_string()),
            "features" => {
                let list: Vec<String> = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                self.features = Some(list);
            }
            "emit" => {
                let list: Vec<String> = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                self.emit = Some(list);
            }
            other => return Err(format!("unknown configuration parameter '{other}'")),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DependencySpec {
    pub path: Option<PathBuf>,
    pub git: Option<String>,
    pub tag: Option<String>,
    pub commit: Option<String>,
    pub branch: Option<String>,
    pub link: Option<String>,
    pub checksum: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
    pub default_features: Option<bool>,
    pub optional: Option<bool>,
    #[serde(default, alias = "sublib", alias = "subpath", alias = "lib")]
    pub package: Option<String>,
}

impl DependencySpec {
    pub fn is_optional(&self) -> bool {
        self.optional.unwrap_or(false)
    }

    pub fn target_package<'a>(&'a self, fallback: &'a str) -> &'a str {
        self.package.as_deref().unwrap_or(fallback)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StaticLibsSection {
    #[serde(default)]
    pub objects: Vec<PathBuf>,
    #[serde(default)]
    pub archives: Vec<PathBuf>,
    #[serde(default)]
    pub search_paths: Vec<PathBuf>,
    #[serde(default)]
    pub system: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LibsSection {
    #[serde(default)]
    pub runtime: HashMap<String, DependencySpec>,
    #[serde(default)]
    pub build: HashMap<String, DependencySpec>,
    #[serde(rename = "static", default)]
    pub static_libs: Option<StaticLibsSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkspaceSection {
    pub name: Option<String>,
    #[serde(default)]
    pub members: Vec<String>,
    #[serde(default)]
    pub default_members: Vec<String>,
    #[serde(default)]
    pub build: Option<WorkspaceBuildSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkspaceBuildSection {
    #[serde(default)]
    pub archives: Vec<PathBuf>,
    #[serde(default)]
    pub objects: Vec<PathBuf>,
    #[serde(default)]
    pub search_paths: Vec<PathBuf>,
    #[serde(default)]
    pub system: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkspaceMemberPackage {
    #[serde(default = "default_static", alias = "type", alias = "output")]
    pub kind: String,
    pub entry: PathBuf,
    pub version: Option<String>,
    #[serde(default = "default_includes")]
    pub includes: Vec<String>,
    #[serde(default)]
    pub archives: Vec<PathBuf>,
    #[serde(default)]
    pub objects: Vec<PathBuf>,
    #[serde(default)]
    pub search_paths: Vec<PathBuf>,
    #[serde(default)]
    pub system: Vec<String>,
    #[serde(default)]
    pub features: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WorkspaceManifest {
    pub workspace: WorkspaceSection,
    #[serde(default)]
    pub build: Option<WorkspaceBuildSection>,
    #[serde(default)]
    pub members: HashMap<String, WorkspaceMemberPackage>,
}

impl WorkspaceManifest {
    pub fn load_from_dir(dir: &Path) -> Result<Option<(Self, PathBuf)>, ProjectError> {
        let ws_path = dir.join("yukispace.toml");
        if !ws_path.is_file() {
            return Ok(None);
        }
        let content = fs::read_to_string(&ws_path)
            .map_err(|e| ProjectError::Io(format!("cannot read {}: {e}", ws_path.display())))?;
        let manifest: WorkspaceManifest = toml::from_str(&content)
            .map_err(|e| ProjectError::Io(format!("invalid {}: {e}", ws_path.display())))?;
        Ok(Some((manifest, ws_path)))
    }

    pub fn find_package(
        &self,
        root_dir: &Path,
        package_name: &str,
    ) -> Result<Option<(ProjectManifest, PathBuf)>, ProjectError> {
        if let Some(pkg) = self.members.get(package_name) {
            let manifest = self.synthesize_member_manifest(package_name, pkg);
            return Ok(Some((manifest, root_dir.join("yukispace.toml"))));
        }

        let member_dirs = self.collect_member_dirs(root_dir);
        for member_dir in member_dirs {
            let (member_manifest, member_path) = match ProjectManifest::load(&member_dir) {
                Ok(res) => res,
                Err(_) => continue,
            };
            let dir_name = member_dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if member_manifest.name() == package_name || dir_name == package_name {
                return Ok(Some((member_manifest, member_path)));
            }
        }
        Ok(None)
    }

    pub fn find_all_members(
        &self,
        root_dir: &Path,
    ) -> Result<Vec<(String, ProjectManifest, PathBuf)>, ProjectError> {
        let mut result = Vec::new();
        let member_dirs = self.collect_member_dirs(root_dir);
        for member_dir in member_dirs {
            if let Ok((m, p)) = ProjectManifest::load(&member_dir) {
                let name = m.name().to_string();
                result.push((name, m, p));
            }
        }

        for (name, pkg) in &self.members {
            if !result.iter().any(|(n, _, _)| n == name) {
                let m = self.synthesize_member_manifest(name, pkg);
                result.push((name.clone(), m, root_dir.join("yukispace.toml")));
            }
        }
        Ok(result)
    }

    fn collect_member_dirs(&self, root_dir: &Path) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        for pattern in &self.workspace.members {
            if pattern.ends_with("/*") {
                let parent = pattern.trim_end_matches("/*");
                let parent_dir = root_dir.join(parent);
                if let Ok(entries) = fs::read_dir(&parent_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            dirs.push(path);
                        }
                    }
                }
            } else {
                let d = root_dir.join(pattern);
                if d.is_dir() {
                    dirs.push(d);
                }
            }
        }
        dirs
    }

    fn synthesize_member_manifest(&self, name: &str, pkg: &WorkspaceMemberPackage) -> ProjectManifest {
        let mut static_libs = StaticLibsSection::default();
        let ws_build = self.workspace.build.as_ref().or(self.build.as_ref());
        if let Some(b) = ws_build {
            static_libs.archives.clone_from(&b.archives);
            static_libs.objects.clone_from(&b.objects);
            static_libs.search_paths.clone_from(&b.search_paths);
            static_libs.system.clone_from(&b.system);
        }
        static_libs.archives.extend(pkg.archives.clone());
        static_libs.objects.extend(pkg.objects.clone());
        static_libs.search_paths.extend(pkg.search_paths.clone());
        static_libs.system.extend(pkg.system.clone());

        let libs_section = LibsSection {
            runtime: HashMap::new(),
            build: HashMap::new(),
            static_libs: Some(static_libs),
        };

        if pkg.kind == "app" {
            ProjectManifest::App(AppManifest {
                bin: BinSection {
                    name: name.to_string(),
                    namedbg: None,
                    version: pkg.version.clone().unwrap_or_else(|| "0.1.0".to_string()),
                    entry: pkg.entry.clone(),
                    yukino: vec!["0.1.0".to_string(), "+".to_string()],
                    output: None,
                    includes: pkg.includes.clone(),
                },
                package: None,
                test: TestSection::default(),
                config: HashMap::new(),
                customcgus: None,
                features: pkg.features.clone(),
                libs: libs_section,
                targets: HashMap::new(),
            })
        } else {
            ProjectManifest::Lib(LibManifest {
                lib: LibSection {
                    name: name.to_string(),
                    version: pkg.version.clone().unwrap_or_else(|| "0.1.0".to_string()),
                    entry: pkg.entry.clone(),
                    yukino: vec!["0.1.0".to_string(), "+".to_string()],
                    output: pkg.kind.clone(),
                    includes: pkg.includes.clone(),
                },
                test: TestSection::default(),
                config: HashMap::new(),
                customcgus: None,
                features: pkg.features.clone(),
                libs: libs_section,
                targets: HashMap::new(),
            })
        }
    }
}

static EMPTY_FEATURES_MAP: std::sync::LazyLock<HashMap<String, Vec<String>>> =
    std::sync::LazyLock::new(HashMap::new);
static EMPTY_CONFIG_MAP: std::sync::LazyLock<HashMap<String, ProfileConfig>> =
    std::sync::LazyLock::new(HashMap::new);
static DEFAULT_TEST_SECTION: std::sync::LazyLock<TestSection> =
    std::sync::LazyLock::new(TestSection::default);
static EMPTY_STRINGS: [String; 0] = [];
static DEFAULT_YUKINO_RANGE: [String; 2] = [String::new(), String::new()];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppManifest {
    pub bin: BinSection,
    #[serde(default)]
    pub package: Option<HashMap<String, toml::Value>>,
    #[serde(default)]
    pub test: TestSection,
    #[serde(default)]
    pub config: HashMap<String, ProfileConfig>,
    #[serde(default)]
    pub customcgus: Option<HashMap<String, Vec<String>>>,
    #[serde(default)]
    pub features: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub libs: LibsSection,
    #[serde(default)]
    pub targets: HashMap<String, LibsSection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibManifest {
    #[serde(alias = "package")]
    pub lib: LibSection,
    #[serde(default)]
    pub test: TestSection,
    #[serde(default)]
    pub config: HashMap<String, ProfileConfig>,
    #[serde(default)]
    pub customcgus: Option<HashMap<String, Vec<String>>>,
    #[serde(default)]
    pub features: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub libs: LibsSection,
    #[serde(default)]
    pub targets: HashMap<String, LibsSection>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectManifest {
    App(AppManifest),
    Lib(LibManifest),
    Workspace(WorkspaceManifest),
}

impl ProjectManifest {
    pub fn customcgus(&self) -> Option<&HashMap<String, Vec<String>>> {
        match self {
            ProjectManifest::App(a) => a.customcgus.as_ref(),
            ProjectManifest::Lib(l) => l.customcgus.as_ref(),
            ProjectManifest::Workspace(_) => None,
        }
    }

    pub fn load_with_package(
        dir: &Path,
        package: Option<&str>,
    ) -> Result<(Self, PathBuf), ProjectError> {
        if let Some(pkg_name) = package {
            if let Some((ws, _)) = WorkspaceManifest::load_from_dir(dir)? {
                if let Some(target) = ws.find_package(dir, pkg_name)? {
                    return Ok(target);
                }
                return Err(ProjectError::Runtime(format!(
                    "package '{pkg_name}' not found in workspace '{}'",
                    dir.display()
                )));
            }
        }
        Self::load(dir)
    }

    pub fn load(dir: &Path) -> Result<(Self, PathBuf), ProjectError> {
        let app_path = dir.join("yuki.toml");
        if app_path.is_file() {
            let content = fs::read_to_string(&app_path)
                .map_err(|e| ProjectError::Io(format!("cannot read {}: {e}", app_path.display())))?;
            if let Ok(manifest) = toml::from_str::<AppManifest>(&content) {
                return Ok((Self::App(manifest), app_path));
            }
            if let Ok(manifest) = toml::from_str::<LibManifest>(&content) {
                return Ok((Self::Lib(manifest), app_path));
            }
            let manifest: AppManifest = toml::from_str(&content)
                .map_err(|e| ProjectError::Io(format!("invalid {}: {e}", app_path.display())))?;
            return Ok((Self::App(manifest), app_path));
        }

        let lib_path = dir.join("lib.toml");
        if lib_path.is_file() {
            let content = fs::read_to_string(&lib_path)
                .map_err(|e| ProjectError::Io(format!("cannot read {}: {e}", lib_path.display())))?;
            let manifest: LibManifest = toml::from_str(&content)
                .map_err(|e| ProjectError::Io(format!("invalid {}: {e}", lib_path.display())))?;
            return Ok((Self::Lib(manifest), lib_path));
        }

        let ws_path = dir.join("yukispace.toml");
        if ws_path.is_file() {
            let content = fs::read_to_string(&ws_path)
                .map_err(|e| ProjectError::Io(format!("cannot read {}: {e}", ws_path.display())))?;
            let manifest: WorkspaceManifest = toml::from_str(&content)
                .map_err(|e| ProjectError::Io(format!("invalid {}: {e}", ws_path.display())))?;
            return Ok((Self::Workspace(manifest), ws_path));
        }

        Err(ProjectError::Io(format!(
            "neither yuki.toml, lib.toml nor yukispace.toml found in {}",
            dir.display()
        )))
    }

    pub fn name(&self) -> &str {
        match self {
            Self::App(a) => &a.bin.name,
            Self::Lib(l) => &l.lib.name,
            Self::Workspace(w) => w.workspace.name.as_deref().unwrap_or("workspace"),
        }
    }

    pub fn namedbg(&self) -> Option<&str> {
        match self {
            Self::App(a) => a.bin.namedbg.as_deref(),
            Self::Lib(_) | Self::Workspace(_) => None,
        }
    }

    pub fn version(&self) -> &str {
        match self {
            Self::App(a) => &a.bin.version,
            Self::Lib(l) => &l.lib.version,
            Self::Workspace(_) => "0.1.0",
        }
    }

    pub fn entry(&self) -> &Path {
        match self {
            Self::App(a) => &a.bin.entry,
            Self::Lib(l) => &l.lib.entry,
            Self::Workspace(_) => Path::new("yukispace.toml"),
        }
    }

    pub fn output_kind(&self) -> &str {
        match self {
            Self::App(a) => {
                if let Some(out) = &a.bin.output {
                    out.as_str()
                } else if let Some(pkg) = &a.package {
                    if let Some(t) = pkg.get("type").or_else(|| pkg.get("kind")).and_then(|v| v.as_str()) {
                        t
                    } else {
                        "app"
                    }
                } else {
                    "app"
                }
            }
            Self::Lib(l) => l.lib.output.as_str(),
            Self::Workspace(_) => "workspace",
        }
    }

    pub fn is_macro(&self) -> bool {
        self.output_kind() == "macro"
    }

    pub fn yukino_range(&self) -> &[String] {
        match self {
            Self::App(a) => &a.bin.yukino,
            Self::Lib(l) => &l.lib.yukino,
            Self::Workspace(_) => &DEFAULT_YUKINO_RANGE,
        }
    }

    pub fn includes(&self) -> &[String] {
        match self {
            Self::App(a) => &a.bin.includes,
            Self::Lib(l) => &l.lib.includes,
            Self::Workspace(_) => &EMPTY_STRINGS,
        }
    }

    pub fn includes_builtin(&self, name: &str) -> bool {
        self.includes().iter().any(|b| b == name)
    }

    pub fn features(&self) -> &HashMap<String, Vec<String>> {
        match self {
            Self::App(a) => &a.features,
            Self::Lib(l) => &l.features,
            Self::Workspace(_) => &EMPTY_FEATURES_MAP,
        }
    }

    pub fn test_section(&self) -> &TestSection {
        match self {
            Self::App(a) => &a.test,
            Self::Lib(l) => &l.test,
            Self::Workspace(_) => &DEFAULT_TEST_SECTION,
        }
    }

    #[allow(clippy::collapsible_if)]
    pub fn runtime_deps(&self, target_triple: Option<&str>) -> HashMap<String, DependencySpec> {
        let mut deps = match self {
            Self::App(a) => a.libs.runtime.clone(),
            Self::Lib(l) => l.libs.runtime.clone(),
            Self::Workspace(_) => HashMap::new(),
        };

        if let Some(triple) = target_triple {
            let target_libs = match self {
                Self::App(a) => a.targets.get(triple),
                Self::Lib(l) => l.targets.get(triple),
                Self::Workspace(_) => None,
            };
            if let Some(tl) = target_libs {
                deps.extend(tl.runtime.clone());
            }
        }

        deps
    }

    #[allow(clippy::collapsible_if)]
    pub fn build_deps(&self, target_triple: Option<&str>) -> HashMap<String, DependencySpec> {
        let mut deps = match self {
            Self::App(a) => a.libs.build.clone(),
            Self::Lib(l) => l.libs.build.clone(),
            Self::Workspace(_) => HashMap::new(),
        };

        if let Some(triple) = target_triple {
            let target_libs = match self {
                Self::App(a) => a.targets.get(triple),
                Self::Lib(l) => l.targets.get(triple),
                Self::Workspace(_) => None,
            };
            if let Some(tl) = target_libs {
                deps.extend(tl.build.clone());
            }
        }

        deps
    }

    #[allow(clippy::collapsible_if)]
    pub fn static_libs(&self, target_triple: Option<&str>) -> StaticLibsSection {
        let mut base = match self {
            Self::App(a) => a.libs.static_libs.clone().unwrap_or_default(),
            Self::Lib(l) => l.libs.static_libs.clone().unwrap_or_default(),
            Self::Workspace(w) => {
                let mut s = StaticLibsSection::default();
                let ws_build = w.workspace.build.as_ref().or(w.build.as_ref());
                if let Some(b) = ws_build {
                    s.archives.clone_from(&b.archives);
                    s.objects.clone_from(&b.objects);
                    s.search_paths.clone_from(&b.search_paths);
                    s.system.clone_from(&b.system);
                }
                s
            }
        };

        if let Some(triple) = target_triple {
            let target_libs = match self {
                Self::App(a) => a.targets.get(triple),
                Self::Lib(l) => l.targets.get(triple),
                Self::Workspace(_) => None,
            };
            if let Some(tl) = target_libs {
                if let Some(t_static) = &tl.static_libs {
                    base.objects.extend(t_static.objects.clone());
                    base.archives.extend(t_static.archives.clone());
                    base.search_paths.extend(t_static.search_paths.clone());
                    base.system.extend(t_static.system.clone());
                }
            }
        }

        base
    }

    pub fn resolve_profile(&self, name: &str) -> ProfileConfig {
        let config_map = match self {
            Self::App(a) => &a.config,
            Self::Lib(l) => &l.config,
            Self::Workspace(_) => &EMPTY_CONFIG_MAP,
        };

        let mut base = if name == "release" {
            ProfileConfig::default_release()
        } else {
            ProfileConfig::default_debug()
        };

        if let Some(cfg) = config_map.get(name) {
            if cfg.opt_level.is_some() {
                base.opt_level.clone_from(&cfg.opt_level);
            }
            if cfg.lto.is_some() {
                base.lto.clone_from(&cfg.lto);
            }
            if cfg.strip.is_some() {
                base.strip = cfg.strip;
            }
            if cfg.strip_unused.is_some() {
                base.strip_unused = cfg.strip_unused;
            }
            if cfg.debug_assertions.is_some() {
                base.debug_assertions = cfg.debug_assertions;
            }
            if cfg.panic_strategy.is_some() {
                base.panic_strategy.clone_from(&cfg.panic_strategy);
            }
            if cfg.cpu.is_some() {
                base.cpu.clone_from(&cfg.cpu);
            }
            if cfg.features.is_some() {
                base.features.clone_from(&cfg.features);
            }
            if cfg.reloc_model.is_some() {
                base.reloc_model.clone_from(&cfg.reloc_model);
            }
            if cfg.code_model.is_some() {
                base.code_model.clone_from(&cfg.code_model);
            }
            if cfg.fast_math.is_some() {
                base.fast_math = cfg.fast_math;
            }
            if cfg.emit.is_some() {
                base.emit.clone_from(&cfg.emit);
            }
            if cfg.inline_statics.is_some() {
                base.inline_statics = cfg.inline_statics;
            }
            if cfg.function_sections.is_some() {
                base.function_sections = cfg.function_sections;
            }
            if cfg.codegen_units.is_some() {
                base.codegen_units = cfg.codegen_units;
            }
        }

        base
    }

    pub fn config_map(&self) -> &HashMap<String, ProfileConfig> {
        match self {
            Self::App(a) => &a.config,
            Self::Lib(l) => &l.config,
            Self::Workspace(_) => &EMPTY_CONFIG_MAP,
        }
    }

    pub fn resolve_inherited_profile(
        &self,
        app_profile: &str,
        app_profile_cfg: &ProfileConfig,
    ) -> (String, ProfileConfig) {
        let map = self.config_map();
        if map.is_empty() {
            return (app_profile.to_string(), app_profile_cfg.clone());
        }

        let chosen_name = if map.contains_key(app_profile) {
            app_profile.to_string()
        } else if map.contains_key("release") {
            "release".to_string()
        } else if let Some((first_key, _)) = map.iter().next() {
            first_key.clone()
        } 

        else { app_profile.to_string() };

        let resolved = self.resolve_profile(&chosen_name);
        (chosen_name, resolved)
    }

    pub fn validate_compiler_version(&self, current_version: &str) -> Result<(), String> {
        if matches!(self, Self::Workspace(_)) {
            return Ok(());
        }
        let range_vec = self.yukino_range();
        if range_vec.len() != 2 {
            return Err("yukino version range must have exactly 2 elements [min, max]".to_string());
        }
        let range = VersionRange::parse_slice(range_vec)
            .map_err(|e| format!("invalid yukino version range: {e}"))?;
        let ver = SemVer::parse(current_version)
            .map_err(|e| format!("invalid current compiler version '{current_version}': {e}"))?;
        if !range.contains(&ver) {
            return Err(format!(
                "current yukino version {current_version} does not satisfy required range [{}, {}]",
                range_vec[0], range_vec[1]
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_manifest_is_valid_toml() {
        let manifest = generate_app_manifest("my_app", "0.1.0");
        assert!(manifest.contains("name = \"my_app\""));
        assert!(manifest.contains("namedbg = \"my_app_debug\""));
        assert!(manifest.contains("yukino = [\"0.1.0\", \"+\"]"));
        assert!(manifest.contains("includes = [\"stdlib\"]"));
        let parsed: Result<AppManifest, _> = toml::from_str(&manifest);
        assert!(parsed.is_ok(), "generated app manifest must parse into AppManifest");
        let app = parsed.unwrap();
        assert_eq!(app.bin.includes, vec!["stdlib".to_string()]);
    }

    #[test]
    fn test_lib_manifest_content() {
        let manifest = generate_lib_manifest(
            "my_lib",
            "0.1.0",
            "static",
        );
        let parsed: Result<LibManifest, _> = toml::from_str(&manifest);
        assert!(parsed.is_ok(), "generated lib manifest must parse into LibManifest");
        let lib = parsed.unwrap();
        assert_eq!(
            lib.lib.entry,
            std::path::PathBuf::from("my_lib/lib.ykn")
        );
        assert_eq!(lib.lib.includes, vec!["stdlib".to_string()]);
    }

    #[test]
    fn test_main_and_lib_ykn_sources() {
        assert!(generate_main_ykn().contains("load std.io: print"));
        assert!(generate_main_ykn().contains("fun main()"));
        assert!(generate_static_lib_ykn().contains("public fun calc(a: int64, b: int64) -> int64"));
        assert!(generate_dynamic_lib_ykn().contains("public cfun calc"));
        assert_eq!(generate_lib_ykn(), generate_static_lib_ykn());
    }

    #[test]
    fn test_profile_override_valid_and_invalid() {
        let mut cfg = ProfileConfig::default_debug();
        assert_eq!(cfg.opt_level_str(), "0");

        cfg.apply_override("opt_level", "2").expect("valid override");
        assert_eq!(cfg.opt_level_str(), "2");

        cfg.apply_override("lto", "true").expect("valid lto");
        assert!(cfg.lto_enabled());

        let err = cfg.apply_override("opt_level", "99");
        assert!(err.is_err());
    }

    #[test]
    fn test_compiler_version_validation() {
        let manifest_str = generate_app_manifest("demo", "0.1.0");
        let app: AppManifest = toml::from_str(&manifest_str).expect("parse app");
        let project = ProjectManifest::App(app);

        assert!(project.validate_compiler_version("0.1.0").is_ok());
        assert!(project.validate_compiler_version("0.2.0").is_ok());
        assert!(project.validate_compiler_version("0.0.9").is_err());
    }

    #[test]
    fn test_resolve_inherited_profile_fallbacks() {
        let app_cfg = ProfileConfig::default_debug();

        // 1. empty lib config -> inherits app config directly
        let empty_lib: LibManifest = toml::from_str(
            r#"
[lib]
name = "empty_lib"
version = "0.1.0"
entry = "src/lib.ykn"
yukino = ["0.1.0", "+"]
"#,
        )
        .expect("parse empty lib");
        let proj = ProjectManifest::Lib(empty_lib);
        let (name, resolved) = proj.resolve_inherited_profile("bench", &app_cfg);
        assert_eq!(name, "bench");
        assert_eq!(resolved.opt_level_str(), app_cfg.opt_level_str());

        // 2. exact matching profile
        let match_lib: LibManifest = toml::from_str(
            r#"
[lib]
name = "match_lib"
version = "0.1.0"
entry = "src/lib.ykn"
yukino = ["0.1.0", "+"]

[config.bench]
opt_level = 2
"#,
        )
        .expect("parse match lib");
        let proj = ProjectManifest::Lib(match_lib);
        let (name, resolved) = proj.resolve_inherited_profile("bench", &app_cfg);
        assert_eq!(name, "bench");
        assert_eq!(resolved.opt_level_str(), "2");

        // 3. no matching profile -> falls back to release
        let release_lib: LibManifest = toml::from_str(
            r#"
[lib]
name = "release_lib"
version = "0.1.0"
entry = "src/lib.ykn"
yukino = ["0.1.0", "+"]

[config.debug]
opt_level = 0

[config.release]
opt_level = 3
"#,
        )
        .expect("parse release lib");
        let proj = ProjectManifest::Lib(release_lib);
        let (name, resolved) = proj.resolve_inherited_profile("bench", &app_cfg);
        assert_eq!(name, "release");
        assert_eq!(resolved.opt_level_str(), "3");

        // 4. no matching profile and no release -> picks first available
        let first_lib: LibManifest = toml::from_str(
            r#"
[lib]
name = "first_lib"
version = "0.1.0"
entry = "src/lib.ykn"
yukino = ["0.1.0", "+"]

[config.custom_opt]
opt_level = 1
"#,
        )
        .expect("parse first lib");
        let proj = ProjectManifest::Lib(first_lib);
        let (name, resolved) = proj.resolve_inherited_profile("bench", &app_cfg);
        assert_eq!(name, "custom_opt");
        assert_eq!(resolved.opt_level_str(), "1");
    }

    #[test]
    fn test_custom_cgus_and_profile_options() {
        let manifest_str = r#"
[bin]
name = "my_app"
version = "0.1.0"
entry = "src/main.ykn"
yukino = ["0.1.0", "+"]

[config.release]
opt_level = 3
function_sections = true
codegen_units = 8

[customcgus]
cgu_io = ["src/io"]
cgu_math = ["src/math"]
"#;
        let app: AppManifest = toml::from_str(manifest_str).expect("parse app");
        let proj = ProjectManifest::App(app);
        let profile = proj.resolve_profile("release");
        assert_eq!(profile.function_sections, Some(true));
        assert_eq!(profile.codegen_units, Some(8));

        let cgus = proj.customcgus().expect("customcgus present");
        assert_eq!(cgus.get("cgu_io").unwrap(), &vec!["src/io".to_string()]);
        assert_eq!(cgus.get("cgu_math").unwrap(), &vec!["src/math".to_string()]);
    }

    #[test]
    fn test_generate_workspace_manifest() {
        let ws_str = generate_workspace_manifest("my_ws");
        assert!(ws_str.contains("[workspace]"));
        assert!(ws_str.contains("name = \"my_ws\""));
        assert!(ws_str.contains("members = ["));
        let parsed: Result<WorkspaceManifest, _> = toml::from_str(&ws_str);
        assert!(parsed.is_ok(), "generated workspace manifest must parse");
    }

    #[test]
    fn test_dependency_spec_optional_and_package() {
        let toml_str = r#"
path = "../lib"
optional = true
package = "my_pkg"
"#;
        let dep: DependencySpec = toml::from_str(toml_str).expect("parse dep");
        assert!(dep.is_optional());
        assert_eq!(dep.target_package("fallback"), "my_pkg");

        let alias_str = r#"
path = "../lib"
lib = "aliased_pkg"
"#;
        let dep2: DependencySpec = toml::from_str(alias_str).expect("parse dep alias");
        assert!(!dep2.is_optional());
        assert_eq!(dep2.target_package("fallback"), "aliased_pkg");
    }

    #[test]
    fn test_workspace_manifest_inline_member() {
        let ws_toml = r#"
[workspace]
name = "bundle"
members = []

[workspace.build]
system = ["pthread", "m"]

[members.core]
kind = "static"
entry = "src/core/lib.ykn"
version = "0.2.0"
archives = ["native/libcore.a"]
"#;
        let ws: WorkspaceManifest = toml::from_str(ws_toml).expect("parse workspace");
        assert_eq!(ws.workspace.name.as_deref(), Some("bundle"));
        assert!(ws.members.contains_key("core"));

        let dummy_root = Path::new("/dummy");
        let pkg = ws.find_package(dummy_root, "core").expect("lookup core");
        assert!(pkg.is_some());
        let (manifest, _) = pkg.unwrap();
        assert_eq!(manifest.name(), "core");
        assert_eq!(manifest.version(), "0.2.0");
        assert_eq!(manifest.output_kind(), "static");
        let static_specs = manifest.static_libs(None);
        assert!(static_specs.system.contains(&"pthread".to_string()));
        assert!(static_specs.archives.contains(&PathBuf::from("native/libcore.a")));
    }
}
