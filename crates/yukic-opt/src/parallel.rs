#![deny(unsafe_code)]

use std::sync::atomic::{AtomicUsize, Ordering};

use inkwell::context::Context;
use inkwell::memory_buffer::MemoryBuffer;
use yukic_codegen::CompiledCgu;
use yukic_target::TargetTriple;

use crate::error::OptError;
use crate::lto::write_bitcode_to_memory;
use crate::pipeline::{PipelineConfig, create_target_machine, run_pipeline};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptProgressEvent {
    Idle,
    Optimizing {
        cgu_name: String,
        current: usize,
        total: usize,
    },
}

pub fn format_opt_progress(thread_id: usize, event: &OptProgressEvent) -> String {
    match event {
        OptProgressEvent::Idle => format!("thread :: {thread_id} [waiting]: idle"),
        OptProgressEvent::Optimizing {
            cgu_name,
            current,
            total,
        } => {
            let pct = (*current * 100).checked_div(*total).unwrap_or(0);
            format!("thread :: {thread_id} [{current}/{total} | {pct}%]: optimizing {cgu_name}")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizedCgu {
    pub name: String,
    pub ll_ir: String,
    pub bitcode: Option<Vec<u8>>,
    pub item_count: usize,
    pub estimated_weight: usize,
}

pub fn optimize_cgus_parallel<F>(
    mut cgus: Vec<CompiledCgu>,
    target: TargetTriple,
    config: &PipelineConfig,
    num_threads: usize,
    on_progress: F,
) -> Result<Vec<OptimizedCgu>, OptError>
where
    F: Fn(usize, OptProgressEvent) + Sync + Send,
{
    if cgus.is_empty() { return Ok(Vec::new()); }

    cgus.sort_by(|a, b| {
        b.estimated_weight
            .cmp(&a.estimated_weight)
            .then_with(|| a.name.cmp(&b.name))
    });

    let total_tasks = cgus.len();
    let thread_count = num_threads.max(1).min(total_tasks);
    let task_index = AtomicUsize::new(0);
    let results: Vec<std::sync::Mutex<Option<OptimizedCgu>>> = (0..total_tasks)
        .map(|_| std::sync::Mutex::new(None))
        .collect();

    std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(thread_count);
        for thread_idx in 0..thread_count {
            let thread_id = thread_idx + 1;
            let cgus_ref = &cgus;
            let task_index_ref = &task_index;
            let results_ref = &results;
            let on_progress_ref = &on_progress;
            let config_clone = config.clone();

            handles.push(s.spawn(move || -> Result<(), OptError> {
                loop {
                    let idx = task_index_ref.fetch_add(1, Ordering::SeqCst);
                    if idx >= total_tasks {
                        on_progress_ref(thread_id, OptProgressEvent::Idle);
                        break;
                    }

                    let cgu = &cgus_ref[idx];
                    on_progress_ref(
                        thread_id,
                        OptProgressEvent::Optimizing {
                            cgu_name: cgu.name.clone(),
                            current: idx + 1,
                            total: total_tasks,
                        },
                    );

                    let optimized = optimize_single_cgu(cgu, target, &config_clone)?;
                    let mut slot = results_ref[idx]
                        .lock()
                        .map_err(|_| OptError::WorkerPanicked("mutex lock poisoned".to_string()))?;
                    *slot = Some(optimized);
                }
                Ok(())
            }));
        }

        for handle in handles {
            handle.join().map_err(|_| {
                OptError::WorkerPanicked("thread panicked during optimization".to_string())
            })??;
        }
        Ok(())
    })?;

    let mut final_results = Vec::with_capacity(total_tasks);
    for slot in results {
        let opt = slot
            .into_inner()
            .map_err(|_| OptError::WorkerPanicked("mutex poisoned".to_string()))?
            .ok_or_else(|| OptError::WorkerPanicked("missing cgu result".to_string()))?;
        final_results.push(opt);
    }

    Ok(final_results)
}

fn optimize_single_cgu(
    cgu: &CompiledCgu,
    target: TargetTriple,
    config: &PipelineConfig,
) -> Result<OptimizedCgu, OptError> {
    // isolated LLVM context
    let context = Context::create();
    let mem_buf = MemoryBuffer::create_from_memory_range_copy(cgu.ll_ir.as_bytes(), &cgu.name);
    let module = context
        .create_module_from_ir(mem_buf)
        .map_err(|e| OptError::PreVerificationFailed(format!("{}: {}", cgu.name, e)))?;

    let machine = create_target_machine(target, config.level, None, None)?;
    run_pipeline(&module, &machine, config)?;

    let bitcode = if config.lto.is_enabled() {
        Some(write_bitcode_to_memory(&module))
    } 

    else { None }; // ;

    Ok(OptimizedCgu {
        name: cgu.name.clone(),
        ll_ir: module.print_to_string().to_string(),
        bitcode,
        item_count: cgu.item_count,
        estimated_weight: cgu.estimated_weight,
    })
}
