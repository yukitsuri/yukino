#![deny(unsafe_code)]

use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

use inkwell::context::Context;
use inkwell::memory_buffer::MemoryBuffer;
use yukic_borrow::BorrowChecker;
use yukic_codegen::{compile_units_parallel_with_overrides, partition_module_with_custom};
use yukic_dce::DeadCodeEliminator;
use yukic_diagnostics::{ColorChoice, DiagnosticEmitter};
use yukic_lexer::Lexer;
use yukic_macro::{BuiltinMacroConfig, MacroExpander};
use yukic_opt::{LtoKind, OptLevel, PipelineConfig, optimize_cgus_parallel, run_pipeline};
use yukic_parser::Parser;
use yukic_resolve::{
    ConditionalConfig, ModuleId, ModuleScope, Resolver, SearchPaths, build_target_name,
    filter_module, find_matching_modules, find_module_file_for_path,
};
use yukic_span::{FileId, SourceMap};
use yukic_target::TargetTriple;
use yukic_types::{FnSig, Type, TypeChecker, TypeParam, ast_type_to_type};

use crate::DriverConfig;
use crate::cli::{EmitKind, OutputType};
use crate::emitter::{
    create_target_machine_for_driver, emit_asm_file, emit_cgus_parallel, emit_object_file,
    format_emit_progress, format_source_cgu, reloc_mode_for,
};
use crate::error::DriverError;
use crate::header::{
    generate_c_header_from_ast, generate_c_header_from_llvm, resolve_header_path,
};

fn ensure_parent_dir(path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn emit_header_if_needed(
    config: &DriverConfig,
    out_path: &Path,
    ast: Option<&yukic_ast::Module>,
    module: Option<&inkwell::module::Module>,
) -> Result<(), DriverError> {
    if let Some(header_path) = resolve_header_path(config, out_path) {
        ensure_parent_dir(&header_path)?;
        let lib_stem = header_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("lib");
        let content = if let Some(ast) = ast {
            generate_c_header_from_ast(ast, lib_stem)
        } else if let Some(m) = module {
            generate_c_header_from_llvm(m, lib_stem)
        } else {
            String::new()
        };
        fs::write(&header_path, content)?;
        if config.verbose {
            println!("Emit C header: {}", header_path.display());
        }
    }
    Ok(())
}

fn emit_final_artifact(
    config: &DriverConfig,
    target: TargetTriple,
    obj_paths: &[PathBuf],
) -> Result<PathBuf, DriverError> {
    if obj_paths.is_empty() {
        return Err(DriverError::NoInputFile);
    }

    match config.output_type {
        OutputType::Static => config.assemble_static_archive(obj_paths, target),
        OutputType::Shared => config.assemble_shared_library(obj_paths, target),
        OutputType::App => config.assemble_executable(obj_paths, target),
    }
}

fn compile_ll_file(
    config: &DriverConfig,
    target: TargetTriple,
    input_path: &Path,
) -> Result<PathBuf, DriverError> {
    println!(
        "{}",
        crate::emitter::format_compiling_source(input_path, crate::should_color())
    );
    let ir_content = fs::read_to_string(input_path)?;
    let context = Context::create();
    let mem_buf = MemoryBuffer::create_from_memory_range_copy(ir_content.as_bytes(), "ll_input");
    let module = context
        .create_module_from_ir(mem_buf)
        .map_err(|e| DriverError::CodegenFailed(e.to_string()))?;

    yukic_codegen::embed_artifact_version_metadata(&module, &context);

    let reloc = reloc_mode_for(config.output_type);
    let machine = create_target_machine_for_driver(target, config.opt_level, Some(reloc))?;

    if config.opt_level != OptLevel::O0 {
        let opt_cfg = PipelineConfig::new(config.opt_level, LtoKind::None);
        run_pipeline(&module, &machine, &opt_cfg)
            .map_err(|e| DriverError::OptFailed(e.to_string()))?;
    }

    let out_path = config.resolve_output_path(target);
    ensure_parent_dir(&out_path)?;
    emit_header_if_needed(config, &out_path, None, Some(&module))?;
    match config.emit {
        EmitKind::LlvmIr => {
            fs::write(&out_path, module.print_to_string().to_string())?;
            let cgu_str = format_source_cgu(
                Some(&config.input_file),
                &out_path.to_string_lossy(),
                crate::should_color(),
            );
            println!("{cgu_str}");
            Ok(out_path)
        }
        EmitKind::Asm => {
            emit_asm_file(&module, &machine, &out_path)?;
            let cgu_str = format_source_cgu(
                Some(&config.input_file),
                &out_path.to_string_lossy(),
                crate::should_color(),
            );
            println!("{cgu_str}");
            Ok(out_path)
        }
        EmitKind::Obj => {
            emit_object_file(&module, &machine, &out_path)?;
            let cgu_str = format_source_cgu(
                Some(&config.input_file),
                &out_path.to_string_lossy(),
                crate::should_color(),
            );
            println!("{cgu_str}");
            Ok(out_path)
        }
        EmitKind::Bin => {
            let tmp_obj = out_path.with_extension("tmp.o");
            ensure_parent_dir(&tmp_obj)?;
            emit_object_file(&module, &machine, &tmp_obj)?;
            let cgu_str = format_source_cgu(
                Some(&config.input_file),
                &tmp_obj.to_string_lossy(),
                crate::should_color(),
            );
            println!("{cgu_str}");
            let final_path = emit_final_artifact(config, target, std::slice::from_ref(&tmp_obj))?;
            if tmp_obj.exists() && tmp_obj != final_path {
                let _ = fs::remove_file(tmp_obj);
            }
            Ok(final_path)
        }
    }
}

fn emit_obj_artifacts(
    obj_paths: &[PathBuf],
    out_path: &Path,
    source_file: Option<&str>,
) -> Result<(), DriverError> {
    if obj_paths.len() <= 1 {
        if let Some(first) = obj_paths.first() {
            fs::copy(first, out_path)?;
            let cgu_str = format_source_cgu(
                source_file,
                &out_path.to_string_lossy(),
                crate::should_color(),
            );
            println!("{cgu_str}");
        }
        return Ok(());
    }
    for (i, obj) in obj_paths.iter().enumerate() {
        let target_obj = if i == 0 {
            out_path.to_path_buf()
        } else {
            let stem = out_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("obj");
            let ext = out_path.extension().and_then(|s| s.to_str()).unwrap_or("o");
            out_path.with_file_name(format!("{stem}_{i}.{ext}"))
        };
        fs::copy(obj, &target_obj)?;
        let cgu_str = format_source_cgu(
            source_file,
            &target_obj.to_string_lossy(),
            crate::should_color(),
        );
        println!("{cgu_str}");
    }
    Ok(())
}

fn should_skip_dir(name: &str) -> bool {
    name.starts_with('.')
        || name == "yuki-work"
        || name == "target"
        || name == "build"
        || name == "node_modules"
}

fn is_build_script(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()) == Some("build.ykn")
}

fn walk_ykn_files(dir: &Path, out: &mut Vec<PathBuf>) {
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
                walk_ykn_files(&path, out);
            }
        } else if path.extension().is_some_and(|ext| ext == "ykn") {
            if is_build_script(&path) {
                continue;
            }
            out.push(path);
        }
    }
}

fn collect_ykn_files(entry_path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let norm_entry = entry_path.to_path_buf();
    files.push(norm_entry.clone());

    if is_build_script(entry_path) {
        return files;
    }

    let base_dir = entry_path.parent().unwrap_or_else(|| Path::new("."));
    let mut discovered = Vec::new();
    walk_ykn_files(base_dir, &mut discovered);

    discovered.sort();
    for file in discovered {
        let is_same = file == norm_entry
            || file.canonicalize().ok() == norm_entry.canonicalize().ok();
        if !is_same && !files.contains(&file) {
            files.push(file);
        }
    }

    files
}

pub struct LoadedModule {
    pub name: String,
    pub path: PathBuf,
    pub ast: yukic_ast::Module,
    pub is_external: bool,
    pub tracked_files: Vec<PathBuf>,
}

fn extract_manifest_name(dir: &Path) -> Option<String> {
    let manifest_path = if dir.join("lib.toml").is_file() {
        Some(dir.join("lib.toml"))
    } else if dir.join("yuki.toml").is_file() {
        Some(dir.join("yuki.toml"))
    } else {
        None
    };
    let val = load_manifest_value(manifest_path.as_deref())?;
    let table = val.as_table()?;
    let section = table
        .get("lib")
        .or_else(|| table.get("package"))
        .or_else(|| table.get("project"))
        .and_then(toml::Value::as_table)?;
    section.get("name").and_then(toml::Value::as_str).map(|s| s.to_string())
}

fn build_search_paths(
    config: &DriverConfig,
    manifest_value: Option<&toml::Value>,
    manifest_dir: &Path,
) -> SearchPaths {
    let mut search_paths = SearchPaths::new();
    for inc in &config.include_dirs {
        if inc.join("lib.toml").is_file() || inc.join("yuki.toml").is_file() {
            let name = extract_manifest_name(inc)
                .or_else(|| inc.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()));
            if let Some(name) = name {
                search_paths.add_lib(name, inc.clone());
            }
        } else {
            search_paths.add_search_dir(inc);
            if let Ok(entries) = std::fs::read_dir(inc) {
                for entry in entries.flatten() {
                    let child_path = entry.path();
                    if child_path.is_dir() {
                        if let Some(name) = extract_manifest_name(&child_path) {
                            search_paths.add_lib(name, child_path);
                        }
                    }
                }
            }
        }
    }

    if let Some(val) = manifest_value
        && let Some(tbl) = val.as_table()
    {
        for section in ["libs.runtime", "libs.build"] {
            let parts: Vec<&str> = section.split('.').collect();
            if let Some(sec) = tbl.get(parts[0]).and_then(toml::Value::as_table)
                && let Some(sub) = sec.get(parts[1]).and_then(toml::Value::as_table)
            {
                for (lib_name, spec) in sub {
                    if let Some(spec_tbl) = spec.as_table() {
                        if let Some(p) = spec_tbl.get("path").and_then(toml::Value::as_str) {
                            let resolved = manifest_dir.join(p);
                            search_paths.add_lib(lib_name.clone(), resolved);
                        } else {
                            let local_dep_dir = manifest_dir.join("yuki-work/local/deps").join(lib_name);
                            if local_dep_dir.is_dir() {
                                search_paths.add_lib(lib_name.clone(), local_dep_dir);
                            }
                        }
                    }
                }
            }
        }
    }

    search_paths
}

fn parse_and_expand_file(
    path: &Path,
    conditions: &ConditionalConfig,
    macro_cfg: &BuiltinMacroConfig,
    source_map: &mut SourceMap,
    emitter: &mut DiagnosticEmitter,
) -> Result<(yukic_ast::Module, Vec<PathBuf>), DriverError> {
    let source_text = fs::read_to_string(path)?;
    let file_id = source_map.add_file(path.display().to_string(), source_text.clone());

    let lexer = Lexer::new(file_id, &source_text);
    let (tokens, lex_diags) = lexer.tokenize();
    for diag in lex_diags {
        let _ = emitter.emit(diag, source_map, &mut std::io::stderr());
    }

    let mut parser = Parser::with_file_id(tokens, file_id);
    let (mut ast, parse_diags) = parser.parse_module();
    for diag in parse_diags {
        let _ = emitter.emit(diag, source_map, &mut std::io::stderr());
    }

    filter_module(&mut ast, conditions).map_err(DriverError::ConditionalCompilation)?;
    yukic_ast::propagate_module_attributes(&mut ast);

    let expander = MacroExpander::with_config(macro_cfg.clone());
    let (res, raw_tracked) = expander.expand_module_tracked(&mut ast, Some(source_map));
    if let Err(diags) = res {
        for d in diags {
            let _ = emitter.emit(d, source_map, &mut std::io::stderr());
        }
    }

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let tracked_files: Vec<PathBuf> = raw_tracked
        .into_iter()
        .map(|t| {
            let p = PathBuf::from(t);
            if p.is_absolute() { p } else { parent.join(p) }
        })
        .collect();

    Ok((ast, tracked_files))
}

fn module_name_for_path(path: &Path, input_path: &Path) -> String {
    if path == input_path {
        return "main".to_string();
    }
    let base = input_path.parent().unwrap_or_else(|| Path::new("."));
    if let Ok(rel) = path.strip_prefix(base) {
        let without_ext = rel.with_extension("");
        let parts: Vec<_> = without_ext
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();
        if !parts.is_empty() {
            return parts.join(".");
        }
    }
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("mod")
        .to_string()
}

fn load_project_modules(
    source_files: &[PathBuf],
    input_path: &Path,
    conditions: &ConditionalConfig,
    macro_cfg: &BuiltinMacroConfig,
    source_map: &mut SourceMap,
    emitter: &mut DiagnosticEmitter,
) -> Result<Vec<LoadedModule>, DriverError> {
    let mut modules = Vec::with_capacity(source_files.len());
    for file in source_files {
        let name = module_name_for_path(file, input_path);
        let (ast, tracked_files) = parse_and_expand_file(file, conditions, macro_cfg, source_map, emitter)?;
        modules.push(LoadedModule {
            name,
            path: file.clone(),
            ast,
            is_external: false,
            tracked_files,
        });
    }
    Ok(modules)
}

fn discover_external_modules(
    loaded_modules: &mut Vec<LoadedModule>,
    src_dir: &Path,
    search_paths: &SearchPaths,
    conditions: &ConditionalConfig,
    macro_cfg: &BuiltinMacroConfig,
    source_map: &mut SourceMap,
    emitter: &mut DiagnosticEmitter,
) -> Result<(), DriverError> {
    let mut i = 0;
    while i < loaded_modules.len() {
        let current_file = loaded_modules[i].path.clone();
        let imports: Vec<yukic_ast::ImportDecl> = loaded_modules[i]
            .ast
            .items
            .iter()
            .filter_map(|item| match &item.kind {
                yukic_ast::ItemKind::Import(decl) => Some(decl.clone()),
                _ => None,
            })
            .collect();

        for import in imports {
            load_import_targets(
                &import,
                &current_file,
                src_dir,
                search_paths,
                conditions,
                macro_cfg,
                source_map,
                emitter,
                loaded_modules,
            )?;
        }
        i += 1;
    }
    Ok(())
}

fn compute_canonical_module_name(
    file_path: &Path,
    src_dir: &Path,
    search_paths: &SearchPaths,
) -> String {
    let norm_path = file_path.canonicalize().unwrap_or_else(|_| file_path.to_path_buf());
    for lib in search_paths.libs() {
        let actual_lib = yukic_resolve::module_path::resolve_lib_dir_for_package(&lib.path, &lib.name);
        let norm_lib = actual_lib.canonicalize().unwrap_or_else(|_| actual_lib);
        if let Ok(rel) = norm_path.strip_prefix(&norm_lib) {
            let without_ext = rel.with_extension("");
            let parts: Vec<&str> = without_ext
                .components()
                .filter_map(|c| c.as_os_str().to_str())
                .filter(|s| *s != "mod" && *s != "lib")
                .collect();
            if parts.is_empty() {
                return lib.name.clone();
            } else {
                return format!("{}.{}", lib.name, parts.join("."));
            }
        }
    }
    let norm_src = src_dir.canonicalize().unwrap_or_else(|_| src_dir.to_path_buf());
    if let Ok(rel) = norm_path.strip_prefix(&norm_src) {
        let without_ext = rel.with_extension("");
        let parts: Vec<&str> = without_ext
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .filter(|s| *s != "mod" && *s != "lib")
            .collect();
        if !parts.is_empty() {
            return parts.join(".");
        }
    }
    file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("mod")
        .to_string()
}

fn load_import_targets(
    import: &yukic_ast::ImportDecl,
    current_file: &Path,
    src_dir: &Path,
    search_paths: &SearchPaths,
    conditions: &ConditionalConfig,
    macro_cfg: &BuiltinMacroConfig,
    source_map: &mut SourceMap,
    emitter: &mut DiagnosticEmitter,
    loaded_modules: &mut Vec<LoadedModule>,
) -> Result<(), DriverError> {
    let is_wildcard = import
        .path
        .segments
        .last()
        .map(|s| s.pattern.is_wildcard())
        .unwrap_or(false);

    if is_wildcard {
        let matches = find_matching_modules(&import.path, src_dir, current_file, search_paths);
        for (_seg_name, file_path) in matches {
            let mod_name = compute_canonical_module_name(&file_path, src_dir, search_paths);
            load_single_module_if_absent(
                mod_name,
                file_path,
                conditions,
                macro_cfg,
                source_map,
                emitter,
                loaded_modules,
            )?;
        }
    } else if let Some(file_path) =
        find_module_file_for_path(&import.path, src_dir, current_file, search_paths)
    {
        if file_path.is_file() {
            let target_name = compute_canonical_module_name(&file_path, src_dir, search_paths);
            load_single_module_if_absent(
                target_name,
                file_path,
                conditions,
                macro_cfg,
                source_map,
                emitter,
                loaded_modules,
            )?;
        }
    }
    Ok(())
}

fn load_single_module_if_absent(
    name: String,
    path: PathBuf,
    conditions: &ConditionalConfig,
    macro_cfg: &BuiltinMacroConfig,
    source_map: &mut SourceMap,
    emitter: &mut DiagnosticEmitter,
    loaded_modules: &mut Vec<LoadedModule>,
) -> Result<(), DriverError> {
    if loaded_modules.iter().any(|m| m.path == path || m.name == name) {
        return Ok(());
    }
    let (ast, tracked_files) = parse_and_expand_file(&path, conditions, macro_cfg, source_map, emitter)?;
    loaded_modules.push(LoadedModule {
        name,
        path,
        ast,
        is_external: true,
        tracked_files,
    });
    Ok(())
}

fn find_manifest_path(input_path: &Path, explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit
        && path.is_file()
    {
        return Some(path.to_path_buf());
    }
    let mut current = input_path.parent();
    while let Some(dir) = current {
        let yuki = dir.join("yuki.toml");
        if yuki.is_file() {
            return Some(yuki);
        }
        let lib = dir.join("lib.toml");
        if lib.is_file() {
            return Some(lib);
        }
        current = dir.parent();
    }
    None
}

fn find_workspace_manifest_path(input_path: &Path, explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit
        && path.is_file()
    {
        return Some(path.to_path_buf());
    }
    let mut current = if input_path.is_dir() {
        Some(input_path)
    } else {
        input_path.parent()
    };
    while let Some(dir) = current {
        let ws = dir.join("yukispace.toml");
        if ws.is_file() {
            return Some(ws);
        }
        current = dir.parent();
    }
    None
}

fn resolve_project_dirs(input_path: &Path, explicit: Option<&Path>) -> (PathBuf, PathBuf) {
    let manifest = find_manifest_path(input_path, explicit);
    let manifest_dir = manifest
        .as_deref()
        .and_then(Path::parent)
        .map_or_else(
            || {
                input_path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .to_path_buf()
            },
            Path::to_path_buf,
        );

    let src_dir = if manifest_dir.join("src").is_dir() {
        manifest_dir.join("src")
    } else {
        manifest_dir.clone()
    };

    (manifest_dir, src_dir)
}

fn load_manifest_value(manifest_path: Option<&Path>) -> Option<toml::Value> {
    let path = manifest_path?;
    let content = fs::read_to_string(path).ok()?;
    content.parse::<toml::Value>().ok()
}

fn load_includes_from_manifest(manifest_value: Option<&toml::Value>) -> Vec<String> {
    if let Some(val) = manifest_value
        && let Some(table) = val.as_table()
    {
        let section = table.get("bin").or_else(|| table.get("lib"));
        if let Some(sec_val) = section
            && let Some(sec_table) = sec_val.as_table()
            && let Some(inc_val) = sec_table.get("includes")
        {
            if let Some(arr) = inc_val.as_array() {
                return arr
                    .iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect();
            }
        }
    }
    vec!["stdlib".to_string()]
}

fn load_custom_cgus(
    manifest: Option<&toml::Value>,
) -> Option<std::collections::HashMap<String, Vec<String>>> {
    let table = manifest?.as_table()?;
    let custom_tbl = table.get("customcgus")?.as_table()?;
    let mut map = std::collections::HashMap::new();
    for (k, v) in custom_tbl {
        if let Some(arr) = v.as_array() {
            let patterns: Vec<String> = arr
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect();
            if !patterns.is_empty() {
                map.insert(k.clone(), patterns);
            }
        } else if let Some(s) = v.as_str() {
            map.insert(k.clone(), vec![s.to_string()]);
        }
    }
    if map.is_empty() {
        None
    } else {
        Some(map)
    }
}

fn collect_manifest_libs(val: &toml::Value, target_str: &str, libs: &mut Vec<String>) {
    let Some(table) = val.as_table() else {
        return;
    };
    if let Some(libs_tbl) = table.get("libs").and_then(toml::Value::as_table)
        && let Some(runtime_tbl) = libs_tbl.get("runtime").and_then(toml::Value::as_table)
    {
        for k in runtime_tbl.keys() {
            if !libs.iter().any(|l| l == k) {
                libs.push(k.clone());
            }
        }
    }
    if let Some(targets_tbl) = table.get("targets").and_then(toml::Value::as_table)
        && let Some(target_sec) = targets_tbl.get(target_str).and_then(toml::Value::as_table)
        && let Some(runtime_tbl) = target_sec.get("runtime").and_then(toml::Value::as_table)
    {
        for k in runtime_tbl.keys() {
            if !libs.iter().any(|l| l == k) {
                libs.push(k.clone());
            }
        }
    }
}

fn collect_connected_libs(
    config: &DriverConfig,
    manifest: Option<&toml::Value>,
    target: TargetTriple,
) -> Vec<String> {
    let mut libs = config.connected_libs.clone();
    for arch in &config.link_archives {
        if let Some(stem) = arch.file_stem().and_then(|s| s.to_str()) {
            let clean = stem.strip_prefix("lib").unwrap_or(stem);
            if !libs.iter().any(|l| l == clean) {
                libs.push(clean.to_string());
            }
        }
    }
    for sys in &config.link_systems {
        if !libs.iter().any(|l| l == sys) {
            libs.push(sys.clone());
        }
    }
    if let Some(val) = manifest {
        collect_manifest_libs(val, target.as_str(), &mut libs);
    }
    libs
}

fn build_macro_config(
    config: &DriverConfig,
    target: TargetTriple,
    input_path: &Path,
) -> BuiltinMacroConfig {
    let manifest_path = find_manifest_path(input_path, config.manifest_path.as_deref());
    let manifest_value = load_manifest_value(manifest_path.as_deref());

    let workspace_manifest_path = find_workspace_manifest_path(
        manifest_path.as_deref().unwrap_or(input_path),
        config.workspace_manifest_path.as_deref(),
    );
    let workspace_manifest_value = load_manifest_value(workspace_manifest_path.as_deref());

    let host_manifest_path = config.host_manifest_path.clone();
    let host_manifest_value = load_manifest_value(host_manifest_path.as_deref());

    let host_workspace_path = config.host_workspace_path.clone();
    let host_workspace_value = load_manifest_value(host_workspace_path.as_deref());

    let is_host = host_manifest_path.is_none();

    let connected_libs = collect_connected_libs(config, manifest_value.as_ref(), target);
    let profile_name = config
        .profile_name
        .clone()
        .unwrap_or_else(|| "debug".to_string());

    BuiltinMacroConfig {
        base_dir: input_path.parent().map(Path::to_path_buf),
        manifest_path,
        manifest_value,
        workspace_manifest_path,
        workspace_manifest_value,
        host_manifest_path,
        host_manifest_value,
        host_workspace_path,
        host_workspace_value,
        is_host,
        project_env: indexmap::IndexMap::new(),
        connected_libs,
        features: config.features.clone(),
        profile_name,
        yukic_version: env!("CARGO_PKG_VERSION").to_string(),
        target_triple: target.as_str().to_string(),
        macro_runners: config.macro_runners.clone(),
        macro_timeout: config.macro_timeout,
        is_test: config.test_mode,
    }
}

fn resolve_import_dependency_target_pos(
    decl: &yukic_ast::ImportDecl,
    from_idx: usize,
    loaded_modules: &[LoadedModule],
) -> Option<usize> {
    let raw_target_name = build_target_name(&decl.path);
    let from_name = &loaded_modules[from_idx].name;

    if let yukic_ast::PathPrefix::Relative(levels) = &decl.path.prefix {
        let parts: Vec<&str> = from_name.split('.').collect();
        // Candidate 1: folder module
        if parts.len() >= *levels {
            let keep = parts.len() - *levels;
            let base = parts[..keep].join(".");
            let cand = if base.is_empty() {
                raw_target_name.clone()
            } else if raw_target_name.is_empty() {
                base
            } else {
                format!("{base}.{raw_target_name}")
            };
            let lib_cand = format!("{cand}.lib");
            let mod_cand = format!("{cand}.mod");
            if let Some(pos) = loaded_modules
                .iter()
                .position(|m| m.name == cand || m.name == lib_cand || m.name == mod_cand)
            {
                return Some(pos);
            }
        }
        // Candidate 2: leaf file
        if parts.len() > *levels {
            let keep = parts.len() - 1 - *levels;
            let base = parts[..keep].join(".");
            let cand = if base.is_empty() {
                raw_target_name.clone()
            } else if raw_target_name.is_empty() {
                base
            } else {
                format!("{base}.{raw_target_name}")
            };
            let lib_cand = format!("{cand}.lib");
            let mod_cand = format!("{cand}.mod");
            if let Some(pos) = loaded_modules
                .iter()
                .position(|m| m.name == cand || m.name == lib_cand || m.name == mod_cand)
            {
                return Some(pos);
            }
        }
    }

    let lib_target = format!("{raw_target_name}.lib");
    let mod_target = format!("{raw_target_name}.mod");
    loaded_modules
        .iter()
        .position(|m| m.name == raw_target_name || m.name == lib_target || m.name == mod_target)
}

fn register_import_dependencies(
    decl: &yukic_ast::ImportDecl,
    from_id: ModuleId,
    from_idx: usize,
    loaded_modules: &[LoadedModule],
    mod_ids: &[ModuleId],
    resolver: &mut Resolver,
) {
    let is_wildcard = decl
        .path
        .segments
        .last()
        .map(|s| s.pattern.is_wildcard())
        .unwrap_or(false);

    if is_wildcard {
        let (last, lead) = match decl.path.segments.split_last() {
            Some(pair) => pair,
            None => return,
        };
        let base_name =
            yukic_resolve::module_path::build_target_name_from_segments(&decl.path.prefix, lead);
        for (pos, m) in loaded_modules.iter().enumerate() {
            let matched = if base_name.is_empty() {
                last.pattern.matches(&m.name)
            } else if let Some(rest) = m.name.strip_prefix(&format!("{base_name}.")) {
                last.pattern.matches(rest)
            } else {
                false
            };
            if matched {
                resolver.add_dependency(from_id, mod_ids[pos], decl.span);
            }
        }
    } else if let Some(pos) = resolve_import_dependency_target_pos(decl, from_idx, loaded_modules) {
        resolver.add_dependency(from_id, mod_ids[pos], decl.span);
    }
}

fn run_resolution_pass(
    loaded_modules: &[LoadedModule],
    source_map: &SourceMap,
    emitter: &mut DiagnosticEmitter,
) -> Resolver {
    let mut resolver = Resolver::new();
    let mut mod_ids = Vec::with_capacity(loaded_modules.len());

    for module in loaded_modules {
        let (id, diags) = resolver.add_module(module.name.clone(), &module.ast);
        for d in diags {
            let _ = emitter.emit(d, source_map, &mut std::io::stderr());
        }
        mod_ids.push(id);
    }

    for (idx, module) in loaded_modules.iter().enumerate() {
        let from_id = mod_ids[idx];
        for item in &module.ast.items {
            if let yukic_ast::ItemKind::Import(decl) = &item.kind {
                register_import_dependencies(
                    decl,
                    from_id,
                    idx,
                    loaded_modules,
                    &mod_ids,
                    &mut resolver,
                );
            }
        }
    }

    for d in resolver.check_cycles() {
        let _ = emitter.emit(d, source_map, &mut std::io::stderr());
    }

    let pairs: Vec<(ModuleId, yukic_ast::Module)> = mod_ids
        .iter()
        .zip(loaded_modules)
        .map(|(&id, m)| (id, m.ast.clone()))
        .collect();

    for d in resolver.resolve_all(&pairs) {
        let _ = emitter.emit(d, source_map, &mut std::io::stderr());
    }

    resolver
}

fn merge_project_ast(
    loaded_modules: &[LoadedModule],
) -> (yukic_ast::Module, std::collections::HashMap<yukic_ast::ItemId, PathBuf>) {
    let root_file_id = loaded_modules
        .first()
        .map_or(FileId(0), |m| m.ast.file_id);
    let mut combined_ast = yukic_ast::Module::new(root_file_id);
    let mut next_item_id = 0u32;
    let mut item_sources = std::collections::HashMap::new();

    for module in loaded_modules.iter().filter(|m| !m.is_external) {
        if combined_ast.items.is_empty() {
            combined_ast.module_attrs.clone_from(&module.ast.module_attrs);
        } else {
            combined_ast.module_attrs.extend(module.ast.module_attrs.clone());
        }
        for mut item in module.ast.items.clone() {
            item.id = yukic_ast::ItemId(next_item_id);
            item_sources.insert(item.id, module.path.clone());
            next_item_id = next_item_id.saturating_add(1);
            combined_ast.items.push(item);
        }
    }

    (combined_ast, item_sources)
}

fn fun_decl_to_sig(decl: &yukic_ast::FunDecl) -> FnSig {
    let params = decl
        .params
        .iter()
        .map(|p| {
            let ty = p.ty.as_ref().map_or(Type::Infer(0), ast_type_to_type);
            TypeParam::new(ty, p.is_ref, p.is_mut)
        })
        .collect();
    let ret = decl
        .return_type
        .as_ref()
        .map_or(Type::Void, ast_type_to_type);
    FnSig {
        params,
        ret,
        is_cfun: false,
    }
}

fn cfun_decl_to_sig(decl: &yukic_ast::CfunDecl) -> FnSig {
    let params = decl
        .params
        .iter()
        .map(|p| TypeParam::value(ast_type_to_type(&p.ty)))
        .collect();
    let ret = decl
        .return_type
        .as_ref()
        .map_or(Type::Void, ast_type_to_type);
    FnSig {
        params,
        ret,
        is_cfun: true,
    }
}

fn struct_decl_to_def(decl: &yukic_ast::StructDecl) -> yukic_types::StructDef {
    let mut fields = std::collections::HashMap::new();
    let mut field_names = Vec::new();
    for f in &decl.fields {
        fields.insert(f.name.name.clone(), ast_type_to_type(&f.ty));
        field_names.push(f.name.name.clone());
    }
    let mut methods = std::collections::HashMap::new();
    for method in &decl.methods {
        let params = method
            .params
            .iter()
            .map(|p| {
                let mut ty = p.ty.as_ref().map_or(yukic_types::Type::Infer(0), ast_type_to_type);
                if p.is_variadic && !matches!(ty, yukic_types::Type::Array(_)) {
                    ty = yukic_types::Type::Array(Box::new(ty));
                }
                yukic_types::TypeParam::new(ty, p.is_ref, p.is_mut)
            })
            .collect();
        let ret = method
            .return_type
            .as_ref()
            .map_or(yukic_types::Type::Void, ast_type_to_type);
        methods.insert(
            method.name.name.clone(),
            FnSig {
                params,
                ret,
                is_cfun: false,
            },
        );
    }
    yukic_types::StructDef {
        fields,
        field_names,
        methods,
    }
}

fn class_decl_to_def(decl: &yukic_ast::ClassDecl) -> yukic_types::ClassDef {
    let mut fields = std::collections::HashMap::new();
    for f in &decl.fields {
        fields.insert(f.name.name.clone(), ast_type_to_type(&f.ty));
    }
    let mut interfaces = std::collections::BTreeSet::new();
    for iface in &decl.interfaces {
        interfaces.insert(iface.name.clone());
        let base = iface.name.split('<').next().unwrap_or(&iface.name);
        if base != iface.name {
            interfaces.insert(base.to_string());
        }
    }
    let mut methods = std::collections::HashMap::new();
    for method in &decl.methods {
        let params = method
            .params
            .iter()
            .map(|p| {
                let mut ty = p.ty.as_ref().map_or(yukic_types::Type::Infer(0), ast_type_to_type);
                if p.is_variadic && !matches!(ty, yukic_types::Type::Array(_)) {
                    ty = yukic_types::Type::Array(Box::new(ty));
                }
                yukic_types::TypeParam::new(ty, p.is_ref, p.is_mut)
            })
            .collect();
        let ret = method
            .return_type
            .as_ref()
            .map_or(yukic_types::Type::Void, ast_type_to_type);
        methods.insert(
            method.name.name.clone(),
            FnSig {
                params,
                ret,
                is_cfun: false,
            },
        );
    }
    let mut getters = std::collections::HashMap::new();
    let mut setters = std::collections::HashMap::new();
    for acc in &decl.accessors {
        let ty = ast_type_to_type(&acc.ty);
        match &acc.kind {
            yukic_ast::item::AccessorKind::Get => {
                getters.insert(acc.name.name.clone(), ty);
            }
            yukic_ast::item::AccessorKind::Set(_) => {
                setters.insert(acc.name.name.clone(), ty);
            }
        }
    }
    yukic_types::ClassDef {
        fields,
        methods,
        getters,
        setters,
        superclass: decl.base_class.as_ref().map(|b| b.name.clone()),
        interfaces,
        is_locked: decl.is_locked,
    }
}

struct ExternalSignatures {
    fns: std::collections::HashMap<String, FnSig>,
    classes: std::collections::HashMap<String, yukic_types::ClassDef>,
    structs: std::collections::HashMap<String, yukic_types::StructDef>,
    interfaces: std::collections::HashMap<String, yukic_types::InterfaceDef>,
    statics: std::collections::HashMap<String, yukic_types::Type>,
    type_aliases: std::collections::HashMap<String, yukic_types::Type>,
}

fn collect_externc_signatures(
    block: &yukic_ast::ExterncBlock,
    mod_name: &str,
    sigs: &mut ExternalSignatures,
) {
    for ext_item in &block.items {
        match ext_item {
            yukic_ast::ExterncItem::Cfun(decl) => {
                let sig = cfun_decl_to_sig(decl);
                sigs.fns.insert(decl.name.name.clone(), sig.clone());
                sigs.fns.insert(format!("{mod_name}::{}", decl.name.name), sig);
            }
            yukic_ast::ExterncItem::Cstruct(decl) => {
                let mut fields = std::collections::HashMap::new();
                let mut field_names = Vec::new();
                for f in &decl.fields {
                    fields.insert(f.name.name.clone(), ast_type_to_type(&f.ty));
                    field_names.push(f.name.name.clone());
                }
                let def = yukic_types::StructDef {
                    fields,
                    field_names,
                    methods: std::collections::HashMap::new(),
                };
                sigs.structs.insert(decl.name.name.clone(), def.clone());
                sigs.structs.insert(format!("{mod_name}::{}", decl.name.name), def);
            }
        }
    }
}

fn collect_external_signatures(loaded_modules: &[LoadedModule]) -> ExternalSignatures {
    let mut sigs = ExternalSignatures {
        fns: std::collections::HashMap::new(),
        classes: std::collections::HashMap::new(),
        structs: std::collections::HashMap::new(),
        interfaces: std::collections::HashMap::new(),
        statics: std::collections::HashMap::new(),
        type_aliases: std::collections::HashMap::new(),
    };

    for ext_mod in loaded_modules.iter().filter(|m| m.is_external) {
        for item in &ext_mod.ast.items {
            match &item.kind {
                yukic_ast::ItemKind::TypeAlias(decl) => {
                    let ty = ast_type_to_type(&decl.target);
                    sigs.type_aliases.insert(decl.name.name.clone(), ty.clone());
                    sigs.type_aliases.insert(format!("{}::{}", ext_mod.name, decl.name.name), ty);
                }
                yukic_ast::ItemKind::Interface(decl) => {
                    let mut def = yukic_types::InterfaceDef::default();
                    def.generics = decl.generics.iter().map(|g| g.name.name.clone()).collect();
                    for method in &decl.methods {
                        let params = method
                            .params
                            .iter()
                            .map(|p| {
                                let mut ty = p.ty.as_ref().map_or(yukic_types::Type::Infer(0), ast_type_to_type);
                                if p.is_variadic && !matches!(ty, yukic_types::Type::Array(_)) {
                                    ty = yukic_types::Type::Array(Box::new(ty));
                                }
                                yukic_types::TypeParam::new(ty, p.is_ref, p.is_mut)
                            })
                            .collect();
                        let ret = method
                            .return_type
                            .as_ref()
                            .map_or(yukic_types::Type::Void, ast_type_to_type);
                        def.methods.insert(
                            method.name.name.clone(),
                            yukic_types::FnSig {
                                params,
                                ret,
                                is_cfun: false,
                            },
                        );
                    }
                    sigs.interfaces.insert(decl.name.name.clone(), def.clone());
                    sigs.interfaces.insert(format!("{}::{}", ext_mod.name, decl.name.name), def);
                }
                yukic_ast::ItemKind::Fun(decl) => {
                    let sig = fun_decl_to_sig(decl);
                    sigs.fns.insert(decl.name.name.clone(), sig.clone());
                    sigs.fns.insert(format!("{}::{}", ext_mod.name, decl.name.name), sig);
                }
                yukic_ast::ItemKind::Cfun(decl) => {
                    let sig = cfun_decl_to_sig(decl);
                    sigs.fns.insert(decl.name.name.clone(), sig.clone());
                    sigs.fns.insert(format!("{}::{}", ext_mod.name, decl.name.name), sig);
                }
                yukic_ast::ItemKind::Struct(decl) => {
                    let def = struct_decl_to_def(decl);
                    sigs.structs.insert(decl.name.name.clone(), def.clone());
                    sigs.structs.insert(format!("{}::{}", ext_mod.name, decl.name.name), def);
                }
                yukic_ast::ItemKind::Class(decl) => {
                    let def = class_decl_to_def(decl);
                    sigs.classes.insert(decl.name.name.clone(), def.clone());
                    sigs.classes.insert(format!("{}::{}", ext_mod.name, decl.name.name), def);
                }
                yukic_ast::ItemKind::Static(decl) => {
                    let ty = decl.ty.as_ref().map_or_else(
                        || match &decl.init.kind {
                            yukic_ast::ExprKind::Literal(lit) => match lit {
                                yukic_ast::LiteralValue::Int(_) => yukic_types::Type::Int64,
                                yukic_ast::LiteralValue::UInt(_) => yukic_types::Type::Uint64,
                                yukic_ast::LiteralValue::Float(_) => yukic_types::Type::Float64,
                                yukic_ast::LiteralValue::Bool(_) => yukic_types::Type::Bool,
                                yukic_ast::LiteralValue::Char(_) => yukic_types::Type::Char,
                                yukic_ast::LiteralValue::String(_) => yukic_types::Type::String,
                                yukic_ast::LiteralValue::Bytes(_) => yukic_types::Type::Bytes,
                                yukic_ast::LiteralValue::Null => yukic_types::Type::Null,
                            },
                            _ => yukic_types::Type::Error,
                        },
                        ast_type_to_type,
                    );
                    sigs.statics.insert(decl.name.name.clone(), ty.clone());
                    sigs.statics.insert(format!("{}::{}", ext_mod.name, decl.name.name), ty);
                }
                yukic_ast::ItemKind::Externc(block) => {
                    collect_externc_signatures(block, &ext_mod.name, &mut sigs);
                }
                _ => {}
            }
        }
    }
    sigs
}

fn populate_scope_imported(
    scope: &ModuleScope,
    resolver: &Resolver,
    sigs: &ExternalSignatures,
    checker: &mut TypeChecker,
) {
    for (name, binding) in scope.bindings() {
        if binding.is_imported
            && let Some(entry) = resolver.registry.get(binding.symbol_id)
        {
            let mod_name = resolver
                .module_ids()
                .iter()
                .find(|(id, _)| *id == entry.module_id)
                .map(|(_, n)| n.as_str());
            let qualified = mod_name.map(|m| format!("{m}::{}", entry.name));

            let ty = qualified
                .as_ref()
                .and_then(|q| sigs.type_aliases.get(q))
                .or_else(|| sigs.type_aliases.get(&entry.name));
            if let Some(ty) = ty {
                checker.env.register_type_alias(name.clone(), ty.clone());
            }

            let sig = qualified
                .as_ref()
                .and_then(|q| sigs.fns.get(q))
                .or_else(|| sigs.fns.get(&entry.name));
            if let Some(sig) = sig {
                if sig.is_cfun {
                    checker.env.register_cfun(name.clone(), sig.clone());
                } else {
                    checker.env.register_fn(name.clone(), sig.clone());
                }
            }

            let def = qualified
                .as_ref()
                .and_then(|q| sigs.structs.get(q))
                .or_else(|| sigs.structs.get(&entry.name));
            if let Some(def) = def {
                checker.env.register_struct(name.clone(), def.clone());
            }

            let def = qualified
                .as_ref()
                .and_then(|q| sigs.classes.get(q))
                .or_else(|| sigs.classes.get(&entry.name));
            if let Some(def) = def {
                checker.env.register_class(name.clone(), def.clone());
            }

            let def = qualified
                .as_ref()
                .and_then(|q| sigs.interfaces.get(q))
                .or_else(|| sigs.interfaces.get(&entry.name));
            if let Some(def) = def {
                checker.env.register_interface(name.clone(), def.clone());
            }

            let ty = qualified
                .as_ref()
                .and_then(|q| sigs.statics.get(q))
                .or_else(|| sigs.statics.get(&entry.name));
            if let Some(ty) = ty {
                checker.env.insert_static_var(name.clone(), ty.clone(), entry.span);
            }
        }
    }

    for (alias_name, alias_entry) in scope.aliases() {
        if let Some(entry) = resolver.registry.get(alias_entry.target_symbol) {
            let mod_name = resolver
                .module_ids()
                .iter()
                .find(|(id, _)| *id == entry.module_id)
                .map(|(_, n)| n.as_str());
            let qualified = mod_name.map(|m| format!("{m}::{}", entry.name));

            let ty = qualified
                .as_ref()
                .and_then(|q| sigs.type_aliases.get(q))
                .or_else(|| sigs.type_aliases.get(&entry.name));
            if let Some(ty) = ty {
                checker.env.register_type_alias(alias_name.clone(), ty.clone());
            }

            let sig = qualified
                .as_ref()
                .and_then(|q| sigs.fns.get(q))
                .or_else(|| sigs.fns.get(&entry.name));
            if let Some(sig) = sig {
                if sig.is_cfun {
                    checker.env.register_cfun(alias_name.clone(), sig.clone());
                } else {
                    checker.env.register_fn(alias_name.clone(), sig.clone());
                }
            }

            let def = qualified
                .as_ref()
                .and_then(|q| sigs.structs.get(q))
                .or_else(|| sigs.structs.get(&entry.name));
            if let Some(def) = def {
                checker.env.register_struct(alias_name.clone(), def.clone());
            }

            let def = qualified
                .as_ref()
                .and_then(|q| sigs.classes.get(q))
                .or_else(|| sigs.classes.get(&entry.name));
            if let Some(def) = def {
                checker.env.register_class(alias_name.clone(), def.clone());
            }

            let def = qualified
                .as_ref()
                .and_then(|q| sigs.interfaces.get(q))
                .or_else(|| sigs.interfaces.get(&entry.name));
            if let Some(def) = def {
                checker.env.register_interface(alias_name.clone(), def.clone());
            }

            let ty = qualified
                .as_ref()
                .and_then(|q| sigs.statics.get(q))
                .or_else(|| sigs.statics.get(&entry.name));
            if let Some(ty) = ty {
                checker.env.insert_static_var(alias_name.clone(), ty.clone(), entry.span);
            }
        }
    }

    for (ns_name, ns_entry) in scope.namespaces() {
        if let Some(target_scope) = resolver.symbol_table.get_scope(ns_entry.target_module) {
            for (item_name, binding) in target_scope.bindings() {
                if let Some(entry) = resolver.registry.get(binding.symbol_id)
                    && entry.is_public()
                {
                    let qualified = format!("{ns_name}::{item_name}");
                    if let Some(ty) = sigs.type_aliases.get(&entry.name) {
                        checker.env.register_type_alias(qualified.clone(), ty.clone());
                    }
                    if let Some(sig) = sigs.fns.get(&entry.name) {
                        if sig.is_cfun {
                            checker.env.register_cfun(qualified.clone(), sig.clone());
                        } else {
                            checker.env.register_fn(qualified.clone(), sig.clone());
                        }
                    }
                    if let Some(def) = sigs.structs.get(&entry.name) {
                        checker.env.register_struct(qualified.clone(), def.clone());
                    }
                    if let Some(def) = sigs.classes.get(&entry.name) {
                        checker.env.register_class(qualified.clone(), def.clone());
                    }
                    if let Some(def) = sigs.interfaces.get(&entry.name) {
                        checker.env.register_interface(qualified.clone(), def.clone());
                    }
                    if let Some(ty) = sigs.statics.get(&entry.name) {
                        checker.env.insert_static_var(qualified, ty.clone(), entry.span);
                    }
                }
            }
        }
    }

    for (sym_name, namespaces) in scope.ambiguous_symbols() {
        checker.env.register_ambiguous(sym_name.clone(), namespaces.clone());
    }
}

fn populate_imported_signatures(
    resolver: &Resolver,
    loaded_modules: &[LoadedModule],
    checker: &mut TypeChecker,
) {
    let sigs = collect_external_signatures(loaded_modules);

    for (name, class_def) in &sigs.classes {
        checker.env.register_class(name.clone(), class_def.clone());
    }
    for (name, struct_def) in &sigs.structs {
        checker.env.register_struct(name.clone(), struct_def.clone());
    }
    for (name, ty) in &sigs.type_aliases {
        checker.env.register_type_alias(name.clone(), ty.clone());
    }

    for (mod_idx, _) in loaded_modules.iter().enumerate().filter(|(_, m)| !m.is_external) {
        let mod_id = ModuleId(u32::try_from(mod_idx).unwrap_or(0));
        if let Some(scope) = resolver.symbol_table.get_scope(mod_id) {
            populate_scope_imported(scope, resolver, &sigs, checker);
        }
    }
}

fn run_type_check_pass(
    ast: &yukic_ast::Module,
    resolver: &Resolver,
    loaded_modules: &[LoadedModule],
    source_map: &SourceMap,
    emitter: &mut DiagnosticEmitter,
    target: TargetTriple,
) {
    let mut checker = TypeChecker::with_target(target);
    populate_imported_signatures(resolver, loaded_modules, &mut checker);

    if let Err(diags) = checker.check_module(ast) {
        for d in diags {
            let _ = emitter.emit(d, source_map, &mut std::io::stderr());
        }
    }
    for d in checker.diagnostics {
        if !d.is_error() {
            let _ = emitter.emit(d, source_map, &mut std::io::stderr());
        }
    }
}

fn populate_borrow_signatures(
    resolver: &Resolver,
    loaded_modules: &[LoadedModule],
    checker: &mut BorrowChecker,
) {
    let sigs = collect_external_signatures(loaded_modules);
    for (name, sig) in &sigs.fns {
        checker.fn_returns.insert(name.clone(), sig.ret.clone());
        checker.fn_types.insert(
            name.clone(),
            yukic_types::Type::Fn {
                params: sig.params.clone(),
                ret: Box::new(sig.ret.clone()),
                is_cfun: sig.is_cfun,
            },
        );
    }
    for (name, class_def) in &sigs.classes {
        checker.type_fields.insert(name.clone(), class_def.fields.clone());
        checker
            .fn_returns
            .insert(name.clone(), yukic_types::Type::Class(name.clone()));
        for (mname, msig) in &class_def.methods {
            checker.fn_returns.insert(format!("{name}::{mname}"), msig.ret.clone());
            checker.fn_returns.insert(mname.clone(), msig.ret.clone());
        }
    }
    for (name, struct_def) in &sigs.structs {
        checker.type_fields.insert(name.clone(), struct_def.fields.clone());
    }
    for (mod_idx, _) in loaded_modules.iter().enumerate().filter(|(_, m)| !m.is_external) {
        let mod_id = ModuleId(u32::try_from(mod_idx).unwrap_or(0));
        if let Some(scope) = resolver.symbol_table.get_scope(mod_id) {
            populate_borrow_scope_imported(scope, resolver, &sigs, checker);
        }
    }
}

fn populate_borrow_scope_imported(
    scope: &ModuleScope,
    resolver: &Resolver,
    sigs: &ExternalSignatures,
    checker: &mut BorrowChecker,
) {
    for (name, binding) in scope.bindings() {
        if binding.is_imported
            && let Some(entry) = resolver.registry.get(binding.symbol_id)
            && let Some(sig) = sigs.fns.get(&entry.name)
        {
            checker.fn_returns.insert(name.clone(), sig.ret.clone());
            checker.fn_types.insert(
                name.clone(),
                yukic_types::Type::Fn {
                    params: sig.params.clone(),
                    ret: Box::new(sig.ret.clone()),
                    is_cfun: sig.is_cfun,
                },
            );
        }
    }
    for (alias_name, alias_entry) in scope.aliases() {
        if let Some(entry) = resolver.registry.get(alias_entry.target_symbol)
            && let Some(sig) = sigs.fns.get(&entry.name)
        {
            checker.fn_returns.insert(alias_name.clone(), sig.ret.clone());
            checker.fn_types.insert(
                alias_name.clone(),
                yukic_types::Type::Fn {
                    params: sig.params.clone(),
                    ret: Box::new(sig.ret.clone()),
                    is_cfun: sig.is_cfun,
                },
            );
        }
    }
    for (ns_name, ns_entry) in scope.namespaces() {
        if let Some(target_scope) = resolver.symbol_table.get_scope(ns_entry.target_module) {
            for (item_name, binding) in target_scope.bindings() {
                if let Some(entry) = resolver.registry.get(binding.symbol_id)
                    && entry.is_public()
                    && let Some(sig) = sigs.fns.get(&entry.name)
                {
                    checker.fn_returns.insert(format!("{ns_name}::{item_name}"), sig.ret.clone());
                    checker.fn_returns.insert(item_name.clone(), sig.ret.clone());
                    let fn_ty = yukic_types::Type::Fn {
                        params: sig.params.clone(),
                        ret: Box::new(sig.ret.clone()),
                        is_cfun: sig.is_cfun,
                    };
                    checker.fn_types.insert(format!("{ns_name}::{item_name}"), fn_ty.clone());
                    checker.fn_types.insert(item_name.clone(), fn_ty);
                }
            }
        }
    }
}

fn run_borrow_check_pass(
    ast: &yukic_ast::Module,
    resolver: &Resolver,
    loaded_modules: &[LoadedModule],
    source_map: &SourceMap,
    emitter: &mut DiagnosticEmitter,
) {
    let mut checker = BorrowChecker::new();
    populate_borrow_signatures(resolver, loaded_modules, &mut checker);

    if let Err(diags) = checker.check_module(ast) {
        for d in diags {
            let _ = emitter.emit(d, source_map, &mut std::io::stderr());
        }
    }
    for d in checker.diagnostics {
        if !d.is_error() {
            let _ = emitter.emit(d, source_map, &mut std::io::stderr());
        }
    }
}

fn to_dce_output_type(output_type: OutputType) -> yukic_dce::OutputType {
    match output_type {
        OutputType::App => yukic_dce::OutputType::App,
        OutputType::Static => yukic_dce::OutputType::StaticLib,
        OutputType::Shared => yukic_dce::OutputType::SharedLib,
    }
}

fn run_dce_pass(
    ast: &mut yukic_ast::Module,
    config: &DriverConfig,
    input_path: &Path,
) -> Result<(), DriverError> {
    if let Some(inspect_path) = &config.inspect_attributes {
        ensure_parent_dir(inspect_path)?;
        let project_name = Path::new(&config.input_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("app");
        let json = yukic_dce::generate_attributes_json(ast, project_name);
        fs::write(inspect_path, json)?;
        if config.verbose {
            println!("Dump attributes: {}", inspect_path.display());
        }
    }

    let mut dce_cfg = yukic_dce::DceConfig::new()
        .with_output_type(to_dce_output_type(config.output_type))
        .with_strip_unused(config.strip_unused)
        .with_test(config.test_mode);
    if let Some(stem) = input_path.file_stem().and_then(|s| s.to_str()) {
        dce_cfg = dce_cfg.with_project_name(stem);
    }
    let eliminator = DeadCodeEliminator::new(dce_cfg);
    let _metrics = eliminator.eliminate(ast);
    Ok(())
}

fn collect_imported_function_overrides(
    resolver: &Resolver,
    loaded_modules: &[LoadedModule],
) -> std::collections::HashMap<String, String> {
    let mut overrides = std::collections::HashMap::new();
    let mut reg = yukic_codegen::TypeRegistry::new();
    for m in loaded_modules {
        reg.register_module(&m.ast);
    }
    reg.canonicalize_registered_types();

    for (mod_idx, _) in loaded_modules.iter().enumerate().filter(|(_, m)| !m.is_external) {
        let mod_id = ModuleId(u32::try_from(mod_idx).unwrap_or(0));
        if let Some(scope) = resolver.symbol_table.get_scope(mod_id) {
            for (name, binding) in scope.bindings() {
                if binding.is_imported {
                    if let Some(entry) = resolver.registry.get(binding.symbol_id) {
                        let target_mod_idx = entry.module_id.0 as usize;
                        if let Some(target_mod) = loaded_modules.get(target_mod_idx) {
                            for item in &target_mod.ast.items {
                                match &item.kind {
                                    yukic_ast::ItemKind::Fun(fun) if fun.name.name == entry.name => {
                                        let sym = if yukic_codegen::should_mangle_fun(&fun.name.name, &fun.attrs) {
                                            yukic_codegen::mangle_fun_decl(fun, &reg)
                                        } else {
                                            fun.name.name.clone()
                                        };
                                        overrides.insert(name.clone(), sym);
                                    }
                                    yukic_ast::ItemKind::Cfun(cfun) if cfun.name.name == entry.name => {
                                        let sym = if let Some(ref s) = cfun.symbol_override {
                                            s.name.clone()
                                        } else if yukic_codegen::should_mangle_cfun(&cfun.name.name, &cfun.yukino_attrs) {
                                            yukic_codegen::mangle_cfun_decl(cfun, &reg)
                                        } else {
                                            cfun.name.name.clone()
                                        };
                                        overrides.insert(name.clone(), sym);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    overrides
}

fn compute_cgu_extra_hashes(
    config: &DriverConfig,
    cgu_sources: &[Vec<PathBuf>],
    loaded_modules: &[LoadedModule],
) -> Vec<Vec<String>> {
    let mut macro_hashes = Vec::new();
    for (name, bin) in &config.macro_runners {
        if let Ok(bytes) = fs::read(bin) {
            let h = blake3::hash(&bytes).to_hex().to_string();
            macro_hashes.push(format!("macro:{name}:{h}"));
        } else {
            macro_hashes.push(format!("macro:{name}:missing"));
        }
    }

    cgu_sources
        .iter()
        .map(|srcs| {
            let mut hashes = macro_hashes.clone();
            let mut seen_tracked = std::collections::HashSet::new();
            for m in loaded_modules {
                if srcs.contains(&m.path) {
                    for tf in &m.tracked_files {
                        if seen_tracked.insert(tf.clone()) {
                            if let Ok(bytes) = fs::read(tf) {
                                let h = blake3::hash(&bytes).to_hex().to_string();
                                hashes.push(format!("tracked:{}:{}", tf.display(), h));
                            } else {
                                hashes.push(format!("tracked:{}:missing", tf.display()));
                            }
                        }
                    }
                }
            }
            hashes
        })
        .collect()
}

fn run_backend_pipeline(
    config: &DriverConfig,
    target: TargetTriple,
    ast: &yukic_ast::Module,
    foreign_modules: &[yukic_ast::Module],
    item_sources: &std::collections::HashMap<yukic_ast::ItemId, PathBuf>,
    imported_overrides: &std::collections::HashMap<String, String>,
    loaded_modules: &[LoadedModule],
) -> Result<PathBuf, DriverError> {
    let input_path = Path::new(&config.input_file);
    let manifest_path = find_manifest_path(input_path, config.manifest_path.as_deref());
    let manifest_val = load_manifest_value(manifest_path.as_deref());
    let custom_cgus = load_custom_cgus(manifest_val.as_ref());

    let units = partition_module_with_custom(
        ast,
        config.codegen_units,
        custom_cgus.as_ref(),
        Some(item_sources),
    );
    let mut cgu_sources: Vec<Vec<PathBuf>> = units
        .iter()
        .map(|u| {
            let mut srcs: Vec<PathBuf> = Vec::new();
            for id in &u.item_ids {
                if let Some(src) = item_sources.get(id)
                    && !srcs.contains(src)
                {
                    srcs.push(src.clone());
                }
            }
            if srcs.is_empty() {
                srcs.push(PathBuf::from(&config.input_file));
            }
            srcs
        })
        .collect();

    let mut compiled = compile_units_parallel_with_overrides(
        ast,
        foreign_modules,
        target,
        units,
        &config.panic_strategy,
        config.function_sections,
        imported_overrides,
    )
    .map_err(DriverError::CodegenFailed)?;

    if config.test_mode {
        let mut reg = yukic_codegen::TypeRegistry::new();
        for m in loaded_modules {
            reg.register_module(&m.ast);
        }
        reg.register_module(ast);
        reg.canonicalize_registered_types();

        let default_timeout_ms = if let Some(val) = manifest_val.as_ref()
            && let Some(test_tbl) = val.get("test").and_then(toml::Value::as_table)
            && let Some(to) = test_tbl.get("timeout_ms").and_then(toml::Value::as_integer)
        {
            to as u64
        } else {
            60_000
        };

        let tests = yukic_codegen::collect_test_functions(
            ast,
            item_sources,
            &reg,
            manifest_path.as_deref().and_then(Path::parent),
        );
        let harness = yukic_codegen::generate_test_harness_cgu(&tests, default_timeout_ms, target)
            .map_err(DriverError::CodegenFailed)?;
        compiled.push(harness);
        cgu_sources.push(vec![PathBuf::from(&config.input_file)]);
    }

    let opt_config = PipelineConfig::new(config.opt_level, LtoKind::None);
    let optimized = optimize_cgus_parallel(
        compiled,
        target,
        &opt_config,
        config.threads,
        |thread_id, event| {
            println!("{}", yukic_opt::format_opt_progress(thread_id, &event));
        },
    )
    .map_err(|e| DriverError::OptFailed(e.to_string()))?;

    let out_path = config.resolve_output_path(target);
    ensure_parent_dir(&out_path)?;
    emit_header_if_needed(config, &out_path, Some(ast), None)?;

    if config.emit == EmitKind::LlvmIr {
        let combined_ir: String = optimized.iter().map(|c| c.ll_ir.as_str()).collect();
        fs::write(&out_path, combined_ir)?;
        let cgu_str = format_source_cgu(
            Some(&config.input_file),
            &out_path.to_string_lossy(),
            crate::should_color(),
        );
        println!("{cgu_str}");
        return Ok(out_path);
    }

    if config.emit == EmitKind::Asm {
        let first = optimized.first().ok_or(DriverError::NoInputFile)?;
        let context = Context::create();
        let mem_buf =
            MemoryBuffer::create_from_memory_range_copy(first.ll_ir.as_bytes(), &first.name);
        let module = context
            .create_module_from_ir(mem_buf)
            .map_err(|e| DriverError::CodegenFailed(e.to_string()))?;
        let reloc = reloc_mode_for(config.output_type);
        let machine = create_target_machine_for_driver(target, config.opt_level, Some(reloc))?;
        emit_asm_file(&module, &machine, &out_path)?;
        let cgu_str = format_source_cgu(
            Some(&config.input_file),
            &out_path.to_string_lossy(),
            crate::should_color(),
        );
        println!("{cgu_str}");
        return Ok(out_path);
    }

    let is_temp = config.cache_dir.is_none();
    let cgu_dir = config.cache_dir.clone().unwrap_or_else(|| {
        std::env::temp_dir().join(format!(
            "yukic_cgus_{}_{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ))
    });
    let reloc = reloc_mode_for(config.output_type);
    let cache_profile = if is_temp {
        None
    } else {
        Some(config.profile_name.as_deref().unwrap_or("default"))
    };
    let cgu_extra_hashes = compute_cgu_extra_hashes(config, &cgu_sources, loaded_modules);
    let obj_paths = emit_cgus_parallel(
        &optimized,
        target,
        config.opt_level,
        reloc,
        &cgu_dir,
        config.threads,
        cache_profile,
        Some(&config.input_file),
        Some(&cgu_sources),
        Some(&cgu_extra_hashes),
        |thread_id, event| {
            println!("{}", format_emit_progress(thread_id, &event));
        },
    )?;

    if config.emit == EmitKind::Obj {
        emit_obj_artifacts(&obj_paths, &out_path, Some(&config.input_file))?;
        if is_temp {
            let _ = fs::remove_dir_all(&cgu_dir);
        }
        return Ok(out_path);
    }

    let final_path = emit_final_artifact(config, target, &obj_paths)?;
    if is_temp {
        let _ = fs::remove_dir_all(&cgu_dir);
    }
    Ok(final_path)
}

fn collect_alias_map(resolver: &Resolver) -> yukic_ast::AliasMap {
    let mut map = yukic_ast::AliasMap::new();
    for scope in resolver.symbol_table.scopes().values() {
        for (alias_name, alias_entry) in scope.aliases() {
            if let Some(entry) = resolver.registry.get(alias_entry.target_symbol) {
                map.insert(alias_name.clone(), entry.name.clone());
            }
        }
        for (name, binding) in scope.bindings() {
            if binding.is_imported
                && binding.is_point_import
                && let Some(entry) = resolver.registry.get(binding.symbol_id)
                && entry.name != *name
            {
                map.insert(name.clone(), entry.name.clone());
            }
        }
    }
    let keys: Vec<String> = map.keys().cloned().collect();
    for key in keys {
        let mut target = map[&key].clone();
        while let Some(next) = map.get(&target) {
            if next == &target {
                break;
            }
            target = next.clone();
        }
        map.insert(key, target);
    }
    map
}

fn compile_ykn_file(
    config: &DriverConfig,
    target: TargetTriple,
    input_path: &Path,
) -> Result<PathBuf, DriverError> {
    let mut source_map = SourceMap::new();
    let color_choice = if crate::should_color() || (std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none()) {
        ColorChoice::Always
    } else {
        ColorChoice::Never
    };
    let mut emitter = DiagnosticEmitter::new(color_choice);
    if config.no_warns {
        emitter.filter_mut().suppress_all();
    }

    let (manifest_dir, src_dir) =
        resolve_project_dirs(input_path, config.manifest_path.as_deref());
    let mut macro_cfg = build_macro_config(config, target, input_path);
    macro_cfg.is_test = config.test_mode;
    let mut conditions = ConditionalConfig::new(target, config.features.clone());
    if config.test_mode {
        conditions = conditions.with_test(true);
    }
    let mut search_paths = build_search_paths(config, macro_cfg.manifest_value.as_ref(), &manifest_dir);
    search_paths.add_search_dir(&manifest_dir);
    search_paths.add_search_dir(&src_dir);
    if let Some(val) = macro_cfg.manifest_value.as_ref() {
        if let Some(pkg_tbl) = val
            .get("package")
            .or_else(|| val.get("project"))
            .or_else(|| val.get("lib"))
            .and_then(toml::Value::as_table)
        {
            if let Some(pkg_name) = pkg_tbl.get("name").and_then(toml::Value::as_str) {
                search_paths.add_lib(pkg_name.to_string(), manifest_dir.clone());
            }
        }
    }

    let mut source_files = collect_ykn_files(input_path);
    if config.test_mode {
        let tests_dir = if let Some(val) = macro_cfg.manifest_value.as_ref()
            && let Some(test_tbl) = val.get("test").and_then(toml::Value::as_table)
            && let Some(dir_str) = test_tbl.get("tests_dir").and_then(toml::Value::as_str)
        {
            manifest_dir.join(dir_str)
        } else {
            manifest_dir.join("tests")
        };
        if tests_dir.is_dir() {
            let mut test_files = Vec::new();
            walk_ykn_files(&tests_dir, &mut test_files);
            test_files.sort();
            for tf in test_files {
                if !source_files.contains(&tf) {
                    source_files.push(tf);
                }
            }
        }
    }
    let total_sources = source_files.len();
    for (idx, file) in source_files.iter().enumerate() {
        println!(
            "{}",
            crate::emitter::format_compiling_source(file, crate::should_color())
        );
        let pct = ((idx + 1) * 100).checked_div(total_sources).unwrap_or(0);
        let rel_file = file.strip_prefix(&src_dir).unwrap_or(file).to_string_lossy();
        println!("thread :: 1 [{}/{} | {}%]: compiling {}", idx + 1, total_sources, pct, rel_file);
    }
    if total_sources > 0 {
        println!("thread :: 1 [waiting]: idle");
    }
    let mut loaded_modules = load_project_modules(
        &source_files,
        input_path,
        &conditions,
        &macro_cfg,
        &mut source_map,
        &mut emitter,
    )?;

    discover_external_modules(
        &mut loaded_modules,
        &src_dir,
        &search_paths,
        &conditions,
        &macro_cfg,
        &mut source_map,
        &mut emitter,
    )?;

    if emitter.has_errors() {
        return Err(DriverError::FrontendValidation(emitter.error_count()));
    }

    let resolver = run_resolution_pass(&loaded_modules, &source_map, &mut emitter);
    if emitter.has_errors() {
        return Err(DriverError::FrontendValidation(emitter.error_count()));
    }

    let (mut combined_ast, item_sources) = merge_project_ast(&loaded_modules);

    let external_modules: Vec<yukic_ast::Module> = loaded_modules
        .iter()
        .filter(|m| m.is_external)
        .map(|m| m.ast.clone())
        .collect();
    yukic_ast::expand_default_args(&mut combined_ast, &external_modules);

    let alias_map = collect_alias_map(&resolver);
    yukic_ast::substitute_module_aliases(&mut combined_ast, &alias_map);

    run_type_check_pass(
        &combined_ast,
        &resolver,
        &loaded_modules,
        &source_map,
        &mut emitter,
        target,
    );
    if emitter.has_errors() {
        return Err(DriverError::FrontendValidation(emitter.error_count()));
    }

    run_borrow_check_pass(
        &combined_ast,
        &resolver,
        &loaded_modules,
        &source_map,
        &mut emitter,
    );
    if emitter.has_errors() {
        return Err(DriverError::FrontendValidation(emitter.error_count()));
    }

    crate::lint::run_lint_pass(&loaded_modules, &source_map, &mut emitter);
    if emitter.has_errors() {
        return Err(DriverError::FrontendValidation(emitter.error_count()));
    }

    if config.test_mode {
        for item in &mut combined_ast.items {
            if let yukic_ast::ItemKind::Fun(ref mut fun) = item.kind {
                if fun.name.name == "main" {
                    fun.name.name = "__yukino_user_main".to_string();
                }
            }
        }
    }

    run_dce_pass(&mut combined_ast, config, input_path)?;

    let uses_std = modules_use_std(&loaded_modules);
    let mut final_config = config.clone();
    if final_config.test_mode {
        final_config.output_type = OutputType::App;
    }
    let manifest_path = find_manifest_path(input_path, final_config.manifest_path.as_deref());
    let manifest_val = load_manifest_value(manifest_path.as_deref());
    let includes = load_includes_from_manifest(manifest_val.as_ref());
    let stdlib_included = includes.iter().any(|b| b == "stdlib");

    if uses_std && stdlib_included {
        if final_config.emit == EmitKind::Bin && final_config.output_type != OutputType::Static {
            let (project_dir, _) = resolve_project_dirs(input_path, final_config.manifest_path.as_deref());
            if let Ok(stdlib) = crate::ensure_stdlib_for(Some(&project_dir), target) {
                if !final_config.link_archives.contains(&stdlib.std_archive) {
                    final_config.link_archives.push(stdlib.std_archive);
                }
                if !final_config.link_archives.contains(&stdlib.core_archive) {
                    final_config.link_archives.push(stdlib.core_archive);
                }
            }
        }
        if !final_config.connected_libs.iter().any(|l| l == "std") {
            final_config.connected_libs.push("std".to_string());
        }
    } else {
        final_config.link_archives.retain(|a| {
            let name = a.file_name().and_then(|s| s.to_str()).unwrap_or("");
            name != "libstd.a" && name != "libyukistd_core.a"
        });
        final_config.connected_libs.retain(|l| l != "std");
    }

    let uses_macrosdk = modules_use_macrosdk(&loaded_modules);
    let macrosdk_included = includes.iter().any(|b| b == "macrosdk");

    if uses_macrosdk && macrosdk_included {
        let (project_dir, _) = resolve_project_dirs(input_path, final_config.manifest_path.as_deref());
        if let Ok(macrosdk) = crate::ensure_macrosdk_for(Some(&project_dir), target) {
            if final_config.emit == EmitKind::Bin && final_config.output_type != OutputType::Static {
                if !final_config.link_archives.contains(&macrosdk.sdk_archive) {
                    final_config.link_archives.push(macrosdk.sdk_archive);
                }
                if !final_config.link_archives.contains(&macrosdk.core_archive) {
                    final_config.link_archives.push(macrosdk.core_archive);
                }
            }
            if !final_config.connected_libs.iter().any(|l| l == "macrosdk") {
                final_config.connected_libs.push("macrosdk".to_string());
            }
        }
    } else {
        final_config.link_archives.retain(|a| {
            let name = a.file_name().and_then(|s| s.to_str()).unwrap_or("");
            name != "libmacrosdk.a" && name != "libyukimacrosdk_core.a"
        });
        final_config.connected_libs.retain(|l| l != "macrosdk");
    }

    let imported_overrides = collect_imported_function_overrides(&resolver, &loaded_modules);
    run_backend_pipeline(
        &final_config,
        target,
        &combined_ast,
        &external_modules,
        &item_sources,
        &imported_overrides,
        &loaded_modules,
    )
}

fn modules_use_std(loaded_modules: &[LoadedModule]) -> bool {
    for m in loaded_modules {
        if m.name == "std" || m.name.starts_with("std.") {
            return true;
        }
        for item in &m.ast.items {
            if let yukic_ast::ItemKind::Import(decl) = &item.kind
                && let yukic_ast::PathPrefix::Package(id) = &decl.path.prefix
                && id.as_str() == "std"
            {
                return true;
            }
        }
    }
    false
}

fn modules_use_macrosdk(loaded_modules: &[LoadedModule]) -> bool {
    for m in loaded_modules {
        if m.name == "macrosdk" || m.name.starts_with("macrosdk.") {
            return true;
        }
        for item in &m.ast.items {
            if let yukic_ast::ItemKind::Import(decl) = &item.kind
                && let yukic_ast::PathPrefix::Package(id) = &decl.path.prefix
                && id.as_str() == "macrosdk"
            {
                return true;
            }
        }
    }
    false
}

fn needs_runtime(config: &DriverConfig) -> bool {
    !config.no_runtime && config.emit == EmitKind::Bin && config.output_type != OutputType::Static
}

fn prepare_runtime(
    config: &DriverConfig,
    target: TargetTriple,
) -> Result<(DriverConfig, Option<PathBuf>), DriverError> {
    if !needs_runtime(config) {
        return Ok((config.clone(), None));
    }
    if let Some(explicit_rt) = &config.runtime {
        let mut prepared = config.clone();
        if !prepared.link_archives.contains(explicit_rt) {
            prepared.link_archives.insert(0, explicit_rt.clone());
        }
        return Ok((prepared, None));
    }

    let input_path = Path::new(&config.input_file);
    let (project_dir, _) = resolve_project_dirs(input_path, config.manifest_path.as_deref());
    let archive = crate::ensure_runtime_archive_for(Some(&project_dir), target)?;
    let mut prepared = config.clone();
    if !prepared.link_archives.contains(&archive) {
        prepared.link_archives.insert(0, archive);
    }
    Ok((prepared, None))
}

fn prepare_stdlib(
    config: &DriverConfig,
    _target: TargetTriple,
) -> Result<DriverConfig, DriverError> {
    let input_path = Path::new(&config.input_file);
    let manifest_path = find_manifest_path(input_path, config.manifest_path.as_deref());
    let manifest_val = load_manifest_value(manifest_path.as_deref());
    let includes = load_includes_from_manifest(manifest_val.as_ref());
    if !includes.iter().any(|b| b == "stdlib") {
        return Ok(config.clone());
    }

    let (project_dir, _) = resolve_project_dirs(input_path, config.manifest_path.as_deref());
    let stdlib_dir = crate::ensure_stdlib_source_for(Some(&project_dir))?;

    let mut prepared = config.clone();
    if !prepared.include_dirs.contains(&stdlib_dir) {
        prepared.include_dirs.push(stdlib_dir);
    }

    Ok(prepared)
}

fn prepare_macrosdk(
    config: &DriverConfig,
    target: TargetTriple,
) -> Result<DriverConfig, DriverError> {
    let input_path = Path::new(&config.input_file);
    let manifest_path = find_manifest_path(input_path, config.manifest_path.as_deref());
    let manifest_val = load_manifest_value(manifest_path.as_deref());
    let includes = load_includes_from_manifest(manifest_val.as_ref());
    if !includes.iter().any(|b| b == "macrosdk") {
        return Ok(config.clone());
    }

    let (project_dir, _) = resolve_project_dirs(input_path, config.manifest_path.as_deref());
    let sdk = crate::ensure_macrosdk_for(Some(&project_dir), target)?;

    let mut prepared = config.clone();
    if !prepared.include_dirs.contains(&sdk.macrosdk_dir) {
        prepared.include_dirs.push(sdk.macrosdk_dir);
    }

    Ok(prepared)
}

pub fn run_compiler_driver(config: &DriverConfig) -> Result<PathBuf, DriverError> {
    let target = config.resolve_target()?;
    let input_path = Path::new(&config.input_file);
    if !input_path.exists() {
        return Err(DriverError::NoInputFile);
    }

    let is_llvm_ir = input_path
        .extension()
        .is_some_and(|ext| ext == "ll" || ext == "bc");

    let mut effective_config = config.clone();
    if effective_config.test_mode {
        effective_config.output_type = OutputType::App;
    }

    let (rt_config, temp_runtime) = prepare_runtime(&effective_config, target)?;
    let std_config = prepare_stdlib(&rt_config, target)?;
    let prepared_config = prepare_macrosdk(&std_config, target)?;
    let result = if is_llvm_ir {
        compile_ll_file(&prepared_config, target, input_path)
    } else {
        compile_ykn_file(&prepared_config, target, input_path)
    };
    if let Some(temp_dir) = temp_runtime {
        let _ = fs::remove_dir_all(temp_dir);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_prepared_when_needed() {
        let config = DriverConfig::new("main.ykn".to_string());
        let (prepared, _) = prepare_runtime(&config, TargetTriple::host()).expect("prepare host runtime");
        let archive = prepared.link_archives.first().expect("runtime archive");
        assert!(archive.is_file());
    }

    #[test]
    fn test_no_runtime_skips_preparation() {
        let mut config = DriverConfig::new("main.ykn".to_string());
        config.no_runtime = true;
        let (prepared, _) = prepare_runtime(&config, TargetTriple::host()).expect("skip runtime");
        assert!(prepared.link_archives.is_empty());
    }

    /*
    #[test]
    fn test_host_runtime_is_prepared_in_temp_dir() {
        let config = DriverConfig::new("main.ykn".to_string());
        let (prepared, temp_dir) = prepare_runtime(&config, TargetTriple::host()).expect("prepare host runtime");
        let archive = prepared.link_archives.first().expect("runtime archive");
        assert!(archive.is_file());
        assert!(temp_dir.is_some());
        let _ = fs::remove_dir_all(temp_dir.expect("runtime temp directory"));
    }

    #[test]
    fn test_custom_target_requires_runtime() {
        let config = DriverConfig::new("main.ykn".to_string());
        let error = prepare_runtime(
            &config,
            TargetTriple::X86_64UnknownLinuxMusl,
        )
        .expect_err("custom target should require runtime");
        assert!(matches!(error, DriverError::RuntimeRequired(_)));
    }

    #[test]
    fn test_host_rejects_explicit_runtime() {
        let mut config = DriverConfig::new("main.ykn".to_string());
        config.runtime = Some(PathBuf::from("runtime.a"));
        let error = prepare_runtime(&config, TargetTriple::host())
            .expect_err("host should reject explicit runtime");
        assert!(matches!(error, DriverError::RuntimeNotAllowedForHost));
    }
    */

    #[test]
    fn test_collect_ykn_files_discovers_nested_sources() {
        let tmp = std::env::temp_dir().join(format!("yukic_test_collect_{}", std::process::id()));
        let src = tmp.join("src");
        let nested = src.join("nested");
        let hidden = src.join(".git");
        let work = src.join("yuki-work");
        fs::create_dir_all(&nested).expect("create nested");
        fs::create_dir_all(&hidden).expect("create hidden");
        fs::create_dir_all(&work).expect("create work");

        let entry = src.join("main.ykn");
        let b = src.join("b.ykn");
        let a = nested.join("a.ykn");
        let ignore1 = hidden.join("ignored.ykn");
        let ignore2 = work.join("ignored.ykn");

        fs::write(&entry, "fun main() {}\n").expect("write entry");
        fs::write(&b, "fun b() {}\n").expect("write b");
        fs::write(&a, "fun a() {}\n").expect("write a");
        fs::write(&ignore1, "fun ignored() {}\n").expect("write ignored");
        fs::write(&ignore2, "fun ignored() {}\n").expect("write ignored");

        let collected = collect_ykn_files(&entry);
        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0], entry);
        assert!(collected.contains(&a));
        assert!(collected.contains(&b));

        let _ = fs::remove_dir_all(tmp);
    }
}
