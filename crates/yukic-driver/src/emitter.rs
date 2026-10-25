#![deny(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use inkwell::context::Context;
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::module::Module;
use inkwell::targets::{FileType, RelocMode, TargetMachine};
use yukic_opt::{OptLevel, OptimizedCgu, create_target_machine};
use yukic_target::TargetTriple;

use crate::cgu_cache::CguCacheManifest;
use crate::cli::OutputType;
use crate::error::DriverError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmitProgressEvent {
    Idle,
    Emitting {
        cgu_name: String,
        current: usize,
        total: usize,
    },
    Cached {
        cgu_name: String,
        current: usize,
        total: usize,
    },
}

pub fn format_emit_progress(thread_id: usize, event: &EmitProgressEvent) -> String {
    match event {
        EmitProgressEvent::Idle => format!("thread :: {thread_id} [waiting]: idle"),
        EmitProgressEvent::Emitting {
            cgu_name,
            current,
            total,
        } => {
            let pct = (*current * 100).checked_div(*total).unwrap_or(0);
            format!("thread :: {thread_id} [{current}/{total} | {pct}%]: emitting {cgu_name}")
        }
        EmitProgressEvent::Cached {
            cgu_name,
            current,
            total,
        } => {
            let pct = (*current * 100).checked_div(*total).unwrap_or(0);
            format!("thread :: {thread_id} [{current}/{total} | {pct}%]: using cached {cgu_name}")
        }
    }
}

fn format_cgu_name(cgu_name: &str) -> String {
    let raw_name = Path::new(cgu_name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(cgu_name);
    if Path::new(raw_name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("o"))
    {
        raw_name.to_string()
    } else {
        format!("{raw_name}.o")
    }
}

pub fn format_cgu_summary(
    action_name: &str,
    source_files: &[PathBuf],
    cgu_name: &str,
    duration: Option<f32>,
    colored: bool,
) -> String {
    let cgu_filename = format_cgu_name(cgu_name);
    let time_str = duration.map_or(String::new(), |d| format!(" ({d:.2}s)"));
    let padded_action = format!("{action_name:>12}");

    if source_files.len() > 1 {
        let mut lines = Vec::new();
        if colored {
            use owo_colors::OwoColorize;
            let action = padded_action.green().bold().to_string();
            let dur = time_str.dimmed().to_string();
            lines.push(format!("{action} {cgu_filename}{dur}"));
        } else {
            lines.push(format!("{padded_action} {cgu_filename}{time_str}"));
        }

        for (i, src) in source_files.iter().enumerate() {
            let is_last = i == source_files.len() - 1;
            let branch = if is_last { "└── " } else { "├── " };
            let rel = crate::format_relative_path(src);
            if colored {
                use owo_colors::OwoColorize;
                lines.push(format!("      {}{}", branch.dimmed(), rel.display()));
            } else {
                lines.push(format!("      {branch}{}", rel.display()));
            }
        }
        lines.join("\n")
    } else {
        let src_path = source_files.first();
        let (last_dir, filename) = match src_path {
            Some(p) => extract_dir_and_filename(p),
            None => extract_default_dir_and_filename(),
        };
        let sep = std::path::MAIN_SEPARATOR;
        if colored {
            use owo_colors::OwoColorize;
            let action = padded_action.green().bold().to_string();
            let arrow = "->".dimmed();
            let dur = time_str.dimmed().to_string();
            format!("{action} {last_dir}{sep}{filename} {arrow} {cgu_filename}{dur}")
        } else {
            format!("{padded_action} {last_dir}{sep}{filename} -> {cgu_filename}{time_str}")
        }
    }
}

pub fn format_source_action(
    action_name: &str,
    source_file: Option<&str>,
    cgu_name: &str,
    colored: bool,
) -> String {
    let sources: Vec<PathBuf> = source_file.into_iter().map(PathBuf::from).collect();
    format_cgu_summary(action_name, &sources, cgu_name, None, colored)
}

pub fn format_source_cgu(source_file: Option<&str>, cgu_name: &str, colored: bool) -> String {
    format_source_action("Compiled", source_file, cgu_name, colored)
}

pub fn format_compiling_source(path: &Path, colored: bool) -> String {
    let rel = crate::format_relative_path(path);
    let padded_action = format!("{:>12}", "Compiling");
    if colored {
        use owo_colors::OwoColorize;
        format!("{} {}", padded_action.green().bold(), rel.display())
    } else {
        format!("{padded_action} {}", rel.display())
    }
}

fn extract_dir_and_filename(path: &Path) -> (String, String) {
    let fname = path
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("source.ykn")
        .to_string();
    let dir = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .or_else(current_dir_name)
        .unwrap_or_else(|| "src".to_string());
    (dir, fname)
}

fn extract_default_dir_and_filename() -> (String, String) {
    let dir = current_dir_name().unwrap_or_else(|| "src".to_string());
    (dir, "main.ykn".to_string())
}

fn current_dir_name() -> Option<String> {
    std::env::current_dir()
        .ok()
        .and_then(|cur| cur.file_name().map(|s| s.to_string_lossy().into_owned()))
}

pub fn reloc_mode_for(output_type: OutputType) -> RelocMode {
    match output_type {
        OutputType::Shared | OutputType::App | OutputType::Static => RelocMode::PIC,
    }
}

pub fn create_target_machine_for_driver(
    target: TargetTriple,
    opt_level: OptLevel,
    reloc: Option<RelocMode>,
) -> Result<TargetMachine, DriverError> {
    create_target_machine(target, opt_level, reloc, None).map_err(|e| {
        DriverError::TargetMachineCreation(target.spec().triple.to_string(), e.to_string())
    })
}

pub fn emit_module_to_file(
    module: &Module,
    machine: &TargetMachine,
    file_type: FileType,
    path: &Path,
) -> Result<(), DriverError> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    machine
        .write_to_file(module, file_type, path)
        .map_err(|err_str| DriverError::EmissionFailed(path.to_path_buf(), err_str.to_string()))
}

pub fn emit_object_file(
    module: &Module,
    machine: &TargetMachine,
    path: &Path,
) -> Result<(), DriverError> {
    emit_module_to_file(module, machine, FileType::Object, path)
}

pub fn emit_asm_file(
    module: &Module,
    machine: &TargetMachine,
    path: &Path,
) -> Result<(), DriverError> {
    emit_module_to_file(module, machine, FileType::Assembly, path)
}

fn emit_single_cgu_object(
    cgu: &OptimizedCgu,
    target: TargetTriple,
    opt_level: OptLevel,
    reloc: RelocMode,
    out_path: &Path,
) -> Result<(), DriverError> {
    let context = Context::create();
    let mem_buf = MemoryBuffer::create_from_memory_range_copy(cgu.ll_ir.as_bytes(), &cgu.name);
    let module = context
        .create_module_from_ir(mem_buf)
        .map_err(|e| DriverError::CodegenFailed(format!("{}: {}", cgu.name, e)))?;

    let machine = create_target_machine_for_driver(target, opt_level, Some(reloc))?;
    let tmp_path = out_path.with_extension(format!("o.tmp.{}", std::process::id()));
    emit_object_file(&module, &machine, &tmp_path)?;
    fs::rename(&tmp_path, out_path)
        .map_err(|e| DriverError::EmissionFailed(out_path.to_path_buf(), e.to_string()))
}

struct CachedCguPartition {
    results: Vec<Option<PathBuf>>,
    pending_indices: Vec<usize>,
    cgu_hashes: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
fn resolve_sources_for_cgu(
    idx: usize,
    cgu_sources: Option<&[Vec<PathBuf>]>,
    source_file: Option<&str>,
) -> Vec<PathBuf> {
    if let Some(cgu_srcs) = cgu_sources
        && let Some(srcs) = cgu_srcs.get(idx)
        && !srcs.is_empty()
    {
        return srcs.clone();
    }
    source_file.into_iter().map(PathBuf::from).collect()
}

fn resolve_cached_cgus<F>(
    cgus: &[OptimizedCgu],
    target: TargetTriple,
    opt_level: OptLevel,
    reloc: RelocMode,
    output_dir: &Path,
    manifest: Option<&CguCacheManifest>,
    source_file: Option<&str>,
    cgu_sources: Option<&[Vec<PathBuf>]>,
    cgu_extra_hashes: Option<&[Vec<String>]>,
    on_progress: &F,
) -> CachedCguPartition
where
    F: Fn(usize, EmitProgressEvent) + Sync + Send,
{
    let total_tasks = cgus.len();
    let mut results = vec![None; total_tasks];
    let mut pending_indices = Vec::new();
    let mut cgu_hashes = Vec::with_capacity(total_tasks);

    for (idx, cgu) in cgus.iter().enumerate() {
        let extra_refs: Vec<&str> = if let Some(extras) = cgu_extra_hashes
            && let Some(cgu_extras) = extras.get(idx)
        {
            cgu_extras.iter().map(|s| s.as_str()).collect()
        } else {
            Vec::new()
        };
        let hash = CguCacheManifest::compute_cgu_hash(cgu, target, opt_level, reloc, &extra_refs);
        cgu_hashes.push(hash.clone());

        if let Some(m) = manifest
            && let Some(cached_path) = m.is_cached(output_dir, &cgu.name, &hash)
        {
            let cgu_name = cached_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            let sources = resolve_sources_for_cgu(idx, cgu_sources, source_file);
            let action_line = format_cgu_summary(
                "Cached",
                &sources,
                &cgu_name,
                None,
                crate::should_color(),
            );
            println!("{action_line}");
            on_progress(
                1,
                EmitProgressEvent::Cached {
                    cgu_name: cgu.name.clone(),
                    current: idx + 1,
                    total: total_tasks,
                },
            );
            results[idx] = Some(cached_path);
        } else {
            pending_indices.push(idx);
        }
    }

    CachedCguPartition {
        results,
        pending_indices,
        cgu_hashes,
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_pending_cgus_worker_pool<F>(
    cgus: &[OptimizedCgu],
    target: TargetTriple,
    opt_level: OptLevel,
    reloc: RelocMode,
    output_dir: &Path,
    pending_indices: &[usize],
    num_threads: usize,
    source_file: Option<&str>,
    cgu_sources: Option<&[Vec<PathBuf>]>,
    on_progress: &F,
) -> Result<Vec<PathBuf>, DriverError>
where
    F: Fn(usize, EmitProgressEvent) + Sync + Send,
{
    let total_tasks = cgus.len();
    let thread_count = num_threads.max(1).min(pending_indices.len());
    let task_cursor = AtomicUsize::new(0);
    let pending_results: Vec<std::sync::Mutex<Option<PathBuf>>> = (0..pending_indices.len())
        .map(|_| std::sync::Mutex::new(None))
        .collect();

    std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(thread_count);
        for thread_idx in 0..thread_count {
            let thread_id = thread_idx + 1;
            let task_cursor_ref = &task_cursor;
            let pending_results_ref = &pending_results;

            handles.push(s.spawn(move || -> Result<(), DriverError> {
                loop {
                    let p_idx = task_cursor_ref.fetch_add(1, Ordering::SeqCst);
                    if p_idx >= pending_indices.len() {
                        on_progress(thread_id, EmitProgressEvent::Idle);
                        break;
                    }

                    let cgu_idx = pending_indices[p_idx];
                    let cgu = &cgus[cgu_idx];
                    on_progress(
                        thread_id,
                        EmitProgressEvent::Emitting {
                            cgu_name: cgu.name.clone(),
                            current: cgu_idx + 1,
                            total: total_tasks,
                        },
                    );

                    let start = std::time::Instant::now();
                    let out_path = output_dir.join(format!("{}.o", cgu.name));
                    emit_single_cgu_object(cgu, target, opt_level, reloc, &out_path)?;
                    let duration = start.elapsed().as_secs_f32();
                    let sources = resolve_sources_for_cgu(cgu_idx, cgu_sources, source_file);
                    let cgu_str = format_cgu_summary(
                        "Compiled",
                        &sources,
                        &cgu.name,
                        Some(duration),
                        crate::should_color(),
                    );
                    println!("{cgu_str}");

                    let mut slot = pending_results_ref[p_idx]
                        .lock()
                        .map_err(|_| DriverError::WorkerPanicked("mutex lock poisoned".into()))?;
                    *slot = Some(out_path);
                }
                Ok(())
            }));
        }

        for handle in handles {
            handle.join().map_err(|_| {
                DriverError::WorkerPanicked("thread panicked during emission".into())
            })??;
        }
        Ok::<(), DriverError>(())
    })?;

    let mut outputs = Vec::with_capacity(pending_indices.len());
    for slot in pending_results {
        let path = slot
            .into_inner()
            .map_err(|_| DriverError::WorkerPanicked("mutex poisoned".into()))?
            .ok_or_else(|| DriverError::WorkerPanicked("missing cgu emission output".into()))?;
        outputs.push(path);
    }
    Ok(outputs)
}

#[allow(clippy::too_many_arguments)]
pub fn emit_cgus_parallel<F>(
    cgus: &[OptimizedCgu],
    target: TargetTriple,
    opt_level: OptLevel,
    reloc: RelocMode,
    output_dir: &Path,
    num_threads: usize,
    cache_profile: Option<&str>,
    source_file: Option<&str>,
    cgu_sources: Option<&[Vec<PathBuf>]>,
    cgu_extra_hashes: Option<&[Vec<String>]>,
    on_progress: F,
) -> Result<Vec<PathBuf>, DriverError>
where
    F: Fn(usize, EmitProgressEvent) + Sync + Send,
{
    if cgus.is_empty() {
        return Ok(Vec::new());
    }

    fs::create_dir_all(output_dir)?;
    let mut manifest = cache_profile.map(|prof| {
        CguCacheManifest::load_or_create(output_dir, target.as_str(), prof, opt_level.as_str())
    });

    let mut partition = resolve_cached_cgus(
        cgus,
        target,
        opt_level,
        reloc,
        output_dir,
        manifest.as_ref(),
        source_file,
        cgu_sources,
        cgu_extra_hashes,
        &on_progress,
    );

    if !partition.pending_indices.is_empty() {
        let emitted = emit_pending_cgus_worker_pool(
            cgus,
            target,
            opt_level,
            reloc,
            output_dir,
            &partition.pending_indices,
            num_threads,
            source_file,
            cgu_sources,
            &on_progress,
        )?;

        for (p_idx, &cgu_idx) in partition.pending_indices.iter().enumerate() {
            let path = emitted[p_idx].clone();
            if let Some(m) = &mut manifest {
                let obj_filename = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                m.record(
                    cgus[cgu_idx].name.clone(),
                    partition.cgu_hashes[cgu_idx].clone(),
                    obj_filename,
                );
            }
            partition.results[cgu_idx] = Some(path);
        }

        if let Some(m) = &manifest {
            m.save_atomic(output_dir)?;
        }
    }

    partition
        .results
        .into_iter()
        .collect::<Option<Vec<PathBuf>>>()
        .ok_or_else(|| DriverError::WorkerPanicked("incomplete cgu outputs".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_cgu_summary_single_file() {
        let sep = std::path::MAIN_SEPARATOR;
        let single = vec![PathBuf::from("src/main.ykn")];
        let plain = format_cgu_summary("Compiled", &single, "cgu_0", Some(0.04), false);
        assert_eq!(plain, format!("    Compiled src{sep}main.ykn -> cgu_0.o (0.04s)"));

        let cached = format_cgu_summary("Cached", &single, "cgu_0", None, false);
        assert_eq!(cached, format!("      Cached src{sep}main.ykn -> cgu_0.o"));
    }

    #[test]
    fn test_format_cgu_summary_tree_multiple_files() {
        let multi = vec![PathBuf::from("src/main.ykn"), PathBuf::from("src/calc.ykn")];
        let plain = format_cgu_summary("Compiled", &multi, "cgu_0", Some(0.05), false);
        let expected = "    Compiled cgu_0.o (0.05s)\n      ├── src/main.ykn\n      └── src/calc.ykn";
        assert_eq!(plain, expected);
    }

    #[test]
    fn test_format_compiling_source() {
        let p = Path::new("src/main.ykn");
        let formatted = format_compiling_source(p, false);
        assert_eq!(formatted, "   Compiling src/main.ykn");
    }
}
