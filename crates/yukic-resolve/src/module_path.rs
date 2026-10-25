#![deny(unsafe_code)]

use std::path::{Path, PathBuf};

use yukic_ast::{ImportPath, ImportPathSegment, ImportPattern, PathPrefix};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedPath {
    File(PathBuf),
    Directory(PathBuf),
}

impl ResolvedPath {
    pub fn as_path(&self) -> &Path {
        match self {
            Self::File(p) | Self::Directory(p) => p,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SearchPaths {
    lib_roots: Vec<LibRoot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibRoot {
    pub name: String,
    pub path: PathBuf,
}

impl SearchPaths {
    pub fn new() -> Self { Self::default() }

    pub fn add_lib(&mut self, name: impl Into<String>, path: impl Into<PathBuf>) {
        self.lib_roots.push(LibRoot {
            name: name.into(),
            path: path.into(),
        });
    }

    pub fn add_search_dir(&mut self, dir: impl AsRef<Path>) {
        let dir = dir.as_ref();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir()
                    && let Some(name) = entry_path.file_name().and_then(|n| n.to_str())
                {
                    if name == "src"
                        || name == "docs"
                        || name == "tests"
                        || name == "target"
                        || name == "node_modules"
                        || name.starts_with('.')
                        || name.starts_with("yuki")
                    {
                        continue;
                    }
                    self.lib_roots.push(LibRoot {
                        name: name.to_string(),
                        path: entry_path,
                    });
                }
            }
        }
    }

    pub fn find_lib(&self, name: &str) -> Option<&Path> {
        self.lib_roots
            .iter()
            .find(|r| r.name == name)
            .map(|r| r.path.as_path())
    }

    pub fn libs(&self) -> &[LibRoot] {
        &self.lib_roots
    }
}

pub fn compute_base_and_segments(
    import_path: &ImportPath,
    project_root: &Path,
    current_file: &Path,
    search_paths: &SearchPaths,
) -> (PathBuf, Vec<ImportPathSegment>) {
    let current_dir = current_file.parent().unwrap_or(project_root);
    match &import_path.prefix {
        PathPrefix::Root => (project_root.to_path_buf(), import_path.segments.clone()),
        PathPrefix::Relative(levels) => {
            let mut dir = current_dir;
            for _ in 0..*levels {
                dir = dir.parent().unwrap_or(project_root);
            }
            (dir.to_path_buf(), import_path.segments.clone())
        }
        PathPrefix::Package(id) => {
            if id.as_str() != "root" && let Some(lib_dir) = search_paths.find_lib(id.as_str()) {
                let resolved = resolve_lib_dir_for_package(lib_dir, id.as_str());
                (resolved, import_path.segments.clone())
            } else {
                let mut segs = Vec::with_capacity(import_path.segments.len() + 1);
                segs.push(ImportPathSegment {
                    pattern: ImportPattern::Exact(id.clone()),
                    force_folder: false,
                    span: id.span,
                });
                segs.extend(import_path.segments.clone());
                (current_dir.to_path_buf(), segs)
            }
        }
    }
}

pub fn resolve_lib_dir(path: &Path) -> PathBuf {
    if path.join("lib.ykn").is_file() {
        return path.to_path_buf();
    }
    if let Some(name) = path.file_name().and_then(|n| n.to_str())
        && path.join(name).is_dir()
    {
        return path.join(name);
    }
    if path.join("src").is_dir() {
        return path.join("src");
    }
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() && (p.join("lib.ykn").is_file() || p.join("mod.ykn").is_file()) {
                return p;
            }
        }
    }
    path.to_path_buf()
}

pub fn resolve_lib_dir_for_package(path: &Path, package_name: &str) -> PathBuf {
    // Prefer the directory named after the package itself: this makes
    // path dependencies resolve regardless of the checkout directory name.
    if path.join(package_name).is_dir() {
        return path.join(package_name);
    }
    resolve_lib_dir(path)
}

pub fn find_module_file_for_path(
    import_path: &ImportPath,
    project_root: &Path,
    current_file: &Path,
    search_paths: &SearchPaths,
) -> Option<PathBuf> {
    let (base_dir, segments) = compute_base_and_segments(
        import_path,
        project_root,
        current_file,
        search_paths,
    );

    if segments.is_empty() {
        return resolve_root_fallback(&base_dir);
    }

    let (last, lead) = segments.split_last()?;
    let mut parent = base_dir;
    for seg in lead {
        let ImportPattern::Exact(id) = &seg.pattern else {
            return None;
        };
        parent.push(id.as_str());
    }

    let ImportPattern::Exact(last_id) = &last.pattern else {
        return None;
    };
    resolve_segment_file(&parent, last_id.as_str(), last.force_folder)
}

fn resolve_root_fallback(base: &Path) -> Option<PathBuf> {
    let lib_cand = base.join("lib.ykn");
    if lib_cand.is_file() {
        return Some(lib_cand);
    }
    let src_lib_cand = base.join("src/lib.ykn");
    if src_lib_cand.is_file() {
        return Some(src_lib_cand);
    }
    let mod_cand = base.join("mod.ykn");
    if mod_cand.is_file() {
        return Some(mod_cand);
    }
    let src_mod_cand = base.join("src/mod.ykn");
    if src_mod_cand.is_file() {
        return Some(src_mod_cand);
    }
    let main_cand = base.join("main.ykn");
    if main_cand.is_file() {
        return Some(main_cand);
    }
    let src_main_cand = base.join("src/main.ykn");
    if src_main_cand.is_file() {
        return Some(src_main_cand);
    }
    let direct_ykn = base.with_extension("ykn");
    if direct_ykn.is_file() {
        return Some(direct_ykn);
    }
    if let Some(parent) = base.parent() {
        let parent_lib = parent.join("lib.ykn");
        if parent_lib.is_file() {
            return Some(parent_lib);
        }
    }
    None
}

fn resolve_segment_file(parent: &Path, name: &str, force_folder: bool) -> Option<PathBuf> {
    if force_folder {
        let folder = parent.join(name);
        return resolve_folder_entry(&folder);
    }

    let file_cand = parent.join(format!("{name}.ykn"));
    if file_cand.is_file() {
        return Some(file_cand);
    }

    let folder = parent.join(name);
    resolve_folder_entry(&folder)
}

fn resolve_folder_entry(folder: &Path) -> Option<PathBuf> {
    if !folder.is_dir() { return None; }
    let mod_cand = folder.join("mod.ykn");
    if mod_cand.is_file() {
        return Some(mod_cand);
    }
    let lib_cand = folder.join("lib.ykn");
    if lib_cand.is_file() {
        return Some(lib_cand);
    }
    let main_cand = folder.join("main.ykn");
    if main_cand.is_file() {
        return Some(main_cand);
    }
    None
}

pub fn find_matching_modules(
    import_path: &ImportPath,
    project_root: &Path,
    current_file: &Path,
    search_paths: &SearchPaths,
) -> Vec<(String, PathBuf)> {
    let (base_dir, segments) = compute_base_and_segments(
        import_path,
        project_root,
        current_file,
        search_paths,
    );
    if segments.is_empty() {
        return Vec::new();
    }

    let (last, lead) = match segments.split_last() {
        Some(pair) => pair,
        None => return Vec::new(),
    };

    let mut parent = base_dir;
    for seg in lead {
        let ImportPattern::Exact(id) = &seg.pattern else {
            return Vec::new();
        };
        parent.push(id.as_str());
    }

    let entries = match std::fs::read_dir(&parent) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let base_name = build_target_name_from_segments(&import_path.prefix, lead);
    let mut results = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "ykn") {
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if stem != "mod" && stem != "lib" && last.pattern.matches(stem) {
                let mod_name = if base_name.is_empty() {
                    stem.to_string()
                } else {
                    format!("{base_name}.{stem}")
                };
                results.push((mod_name, path));
            }
        } else if path.is_dir()
            && let Some(dir_name) = path.file_name().and_then(|s| s.to_str())
            && last.pattern.matches(dir_name)
            && let Some(f) = resolve_folder_entry(&path)
        {
            let mod_name = if base_name.is_empty() {
                dir_name.to_string()
            } else {
                format!("{base_name}.{dir_name}")
            };
            results.push((mod_name, f));
        }
    }

    results.sort_by(|a, b| a.0.cmp(&b.0));
    results
}

pub fn resolve_import_path(
    import_path: &ImportPath,
    project_root: &Path,
    current_file: &Path,
    search_paths: &SearchPaths,
) -> Vec<ResolvedPath> {
    if let Some(path) = find_module_file_for_path(
        import_path,
        project_root,
        current_file,
        search_paths,
    ) {
        vec![ResolvedPath::File(path)]
    } 

    else { Vec::new() }
}

pub fn find_module_file(
    import_path: &ImportPath,
    project_root: &Path,
    current_file: &Path,
    search_paths: &SearchPaths,
) -> Option<ResolvedPath> {
    find_module_file_for_path(import_path, project_root, current_file, search_paths)
        .map(ResolvedPath::File)
}

pub fn find_module_file_for_mod(
    import_path: &ImportPath,
    project_root: &Path,
    current_file: &Path,
    search_paths: &SearchPaths,
) -> Option<PathBuf> {
    find_module_file_for_path(import_path, project_root, current_file, search_paths)
}

pub fn find_package_dir(
    import_path: &ImportPath,
    project_root: &Path,
    current_file: &Path,
    search_paths: &SearchPaths,
) -> Option<PathBuf> {
    let (base_dir, segments) = compute_base_and_segments(
        import_path,
        project_root,
        current_file,
        search_paths,
    );
    let mut dir = base_dir;
    for seg in segments {
        let ImportPattern::Exact(id) = seg.pattern else {
            return None;
        };
        dir.push(id.as_str());
    }
    if dir.is_dir() { Some(dir) } else { None }
}

pub fn find_module_in_package(package_dir: &Path, mod_name: &str) -> Option<PathBuf> {
    let file_cand = package_dir.join(format!("{mod_name}.ykn"));
    if file_cand.is_file() { return Some(file_cand); }
    let mod_cand = package_dir.join(mod_name).join("mod.ykn");
    if mod_cand.is_file() { return Some(mod_cand); }

    
    None
}

pub fn build_target_name(path: &ImportPath) -> String {
    build_target_name_from_segments(&path.prefix, &path.segments)
}

pub fn build_target_name_from_segments(
    prefix: &PathPrefix,
    segments: &[ImportPathSegment],
) -> String {
    let mut parts = Vec::new();
    if let PathPrefix::Package(id) = prefix {
        parts.push(id.as_str());
    }
    for seg in segments {
        match &seg.pattern {
            ImportPattern::Exact(id) | ImportPattern::Prefix(id) | ImportPattern::Suffix(id) => {
                parts.push(id.as_str());
            }
            ImportPattern::All => {
                parts.push("*");
            }
        }
    }
    parts.join(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_ast::Ident;
    use yukic_span::{BytePos, FileId, Span};

    fn dummy_span() -> Span {
        Span::new(FileId(0), BytePos(0), BytePos(0))
    }

    fn make_ident(name: &str) -> Ident {
        Ident::new(name, dummy_span())
    }

    fn make_segment(name: &str, force_folder: bool) -> ImportPathSegment {
        ImportPathSegment {
            pattern: ImportPattern::Exact(make_ident(name)),
            force_folder,
            span: dummy_span(),
        }
    }

    fn empty_search() -> SearchPaths {
        SearchPaths::new()
    }

    #[test]
    fn test_resolve_current_relative_prefix() {
        let path = ImportPath {
            prefix: PathPrefix::Relative(0),
            segments: vec![make_segment("calc", false)],
            span: dummy_span(),
        };
        let project = Path::new("/project/src");
        let current = Path::new("/project/src/utils/math.ykn");
        let (base, segs) = compute_base_and_segments(&path, project, current, &empty_search());
        assert_eq!(base, Path::new("/project/src/utils"));
        assert_eq!(segs.len(), 1);
    }

    #[test]
    fn test_resolve_parent_relative_prefix() {
        let path = ImportPath {
            prefix: PathPrefix::Relative(1),
            segments: vec![make_segment("config", false)],
            span: dummy_span(),
        };
        let project = Path::new("/project/src");
        let current = Path::new("/project/src/utils/math.ykn");
        let (base, segs) = compute_base_and_segments(&path, project, current, &empty_search());
        assert_eq!(base, Path::new("/project/src"));
        assert_eq!(segs.len(), 1);
    }

    #[test]
    fn test_resolve_root_prefix() {
        let path = ImportPath {
            prefix: PathPrefix::Root,
            segments: vec![make_segment("net", false), make_segment("http", false)],
            span: dummy_span(),
        };
        let project = Path::new("/project/src");
        let current = Path::new("/project/src/deep/nested/file.ykn");
        let (base, segs) = compute_base_and_segments(&path, project, current, &empty_search());
        assert_eq!(base, Path::new("/project/src"));
        assert_eq!(segs.len(), 2);
    }

    #[test]
    fn test_search_paths_find_lib() {
        let mut search = SearchPaths::new();
        search.add_lib("json", "/libs/json/src");
        search.add_lib("http", "/libs/http/src");
        assert_eq!(search.find_lib("json"), Some(Path::new("/libs/json/src")));
        assert_eq!(search.find_lib("http"), Some(Path::new("/libs/http/src")));
        assert_eq!(search.find_lib("missing"), None);
    }

    #[test]
    fn test_folder_vs_file_priority_and_bang_override() {
        let temp = std::env::temp_dir().join(format!("yuki_test_bang_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp);

        let file_ykn = temp.join("test.ykn");
        let _ = std::fs::write(&file_ykn, "public fun f() {}");

        let folder = temp.join("test");
        let _ = std::fs::create_dir_all(&folder);
        let mod_ykn = folder.join("mod.ykn");
        let _ = std::fs::write(&mod_ykn, "public fun g() {}");

        // without '!': file wins
        let normal_path = ImportPath {
            prefix: PathPrefix::Relative(0),
            segments: vec![make_segment("test", false)],
            span: dummy_span(),
        };
        let resolved_normal = find_module_file_for_path(
            &normal_path,
            &temp,
            &temp.join("main.ykn"),
            &empty_search(),
        );
        assert_eq!(resolved_normal, Some(file_ykn));

        // with '!': folder wins
        let bang_path = ImportPath {
            prefix: PathPrefix::Relative(0),
            segments: vec![make_segment("test", true)],
            span: dummy_span(),
        };
        let resolved_bang = find_module_file_for_path(
            &bang_path,
            &temp,
            &temp.join("main.ykn"),
            &empty_search(),
        );
        assert_eq!(resolved_bang, Some(mod_ykn));

        let _ = std::fs::remove_dir_all(&temp);
    }
}
