#![deny(unsafe_code)]

pub mod attributes;
pub mod error;
pub mod level;
pub mod lto;
pub mod parallel;
pub mod pipeline;

pub use attributes::*;
pub use error::OptError;
pub use level::OptLevel;
pub use lto::{LtoKind, write_bitcode_to_memory, write_bitcode_to_path};
pub use parallel::{OptProgressEvent, OptimizedCgu, format_opt_progress, optimize_cgus_parallel};
pub use pipeline::{PipelineConfig, create_target_machine, run_pipeline};

use inkwell::module::Module;
use inkwell::targets::TargetMachine;
use std::path::Path;
use yukic_codegen::CompiledCgu;
use yukic_target::TargetTriple;

pub struct Optimizer { config: PipelineConfig, }

impl Optimizer {
    pub fn new(level: OptLevel, lto: LtoKind) -> Self {
        Self { config: PipelineConfig::new(level, lto), }
    }

    pub fn with_config(config: PipelineConfig) -> Self { Self { config } }

    pub fn config(&self) -> &PipelineConfig { &self.config }

    pub fn optimize(&self, module: &Module, target: TargetTriple) -> Result<(), OptError> {
        let machine = create_target_machine(target, self.config.level, None, None)?;
        self.optimize_with_machine(module, &machine)
    }

    pub fn optimize_with_machine(
        &self,
        module: &Module,
        machine: &TargetMachine,
    ) -> Result<(), OptError> {
        run_pipeline(module, machine, &self.config)
    }

    pub fn emit_bitcode(&self, module: &Module, path: &Path) -> Result<(), OptError> {
        write_bitcode_to_path(module, path)
    }

    pub fn optimize_cgus_parallel<F>(
        &self,
        cgus: Vec<CompiledCgu>,
        target: TargetTriple,
        num_threads: usize,
        on_progress: F,
    ) -> Result<Vec<OptimizedCgu>, OptError>
    where
        F: Fn(usize, OptProgressEvent) + Sync + Send,
    {
        optimize_cgus_parallel(cgus, target, &self.config, num_threads, on_progress)
    }
}

impl Default for Optimizer {
    fn default() -> Self { Self::new(OptLevel::O2, LtoKind::None) }
}
