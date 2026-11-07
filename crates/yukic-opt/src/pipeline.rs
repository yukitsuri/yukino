#![deny(unsafe_code)]

use inkwell::module::Module;
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::{
    CodeModel, RelocMode, Target, TargetMachine, TargetTriple as InkwellTriple,
};
use yukic_codegen::init_llvm_targets;
use yukic_target::TargetTriple;

use crate::error::OptError;
use crate::level::OptLevel;
use crate::lto::LtoKind;

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineConfig {
    pub level: OptLevel,
    pub lto: LtoKind,
    pub loop_vectorization: bool,
    pub slp_vectorization: bool,
    pub loop_unrolling: bool,
    pub merge_functions: bool,
    pub verify_each: bool,
    pub debug_logging: bool,
}

impl PipelineConfig {
    pub fn new(level: OptLevel, lto: LtoKind) -> Self {
        let is_vectorizable = matches!(level, OptLevel::O2 | OptLevel::O3);
        let is_mergeable = matches!(level, OptLevel::Os | OptLevel::Oz | OptLevel::O3);
        Self {
            level,
            lto,
            loop_vectorization: is_vectorizable,
            slp_vectorization: is_vectorizable,
            loop_unrolling: is_vectorizable,
            merge_functions: is_mergeable,
            verify_each: false,
            debug_logging: false,
        }
    }
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self::new(OptLevel::O2, LtoKind::None)
    }
}

pub fn create_target_machine(
    target: TargetTriple,
    opt_level: OptLevel,
    reloc: Option<RelocMode>,
    code_model: Option<CodeModel>,
) -> Result<TargetMachine, OptError> {
    init_llvm_targets();
    let triple_str = target.spec().triple;
    let inkwell_triple = InkwellTriple::create(triple_str);
    let llvm_target = Target::from_triple(&inkwell_triple)
        .map_err(|e| OptError::TargetNotFound(format!("{triple_str}: {e}")))?;

    let opt = opt_level.to_inkwell_opt_level();
    let reloc_mode = reloc.unwrap_or(RelocMode::Default);
    let model = code_model.unwrap_or(CodeModel::Default);

    llvm_target
        .create_target_machine(&inkwell_triple, "generic", "", opt, reloc_mode, model)
        .ok_or_else(|| OptError::TargetMachineCreationFailed(triple_str.to_string()))
}

pub fn run_pipeline(
    module: &Module,
    machine: &TargetMachine,
    config: &PipelineConfig,
) -> Result<(), OptError> {
    module
        .verify()
        .map_err(|e| OptError::PreVerificationFailed(e.to_string()))?;

    let passes = config.level.default_pass_pipeline(config.lto);
    let options = build_pass_options(config);

    module
        .run_passes(passes, machine, options)
        .map_err(|e| OptError::PassExecutionFailed(passes.to_string(), e.to_string()))?;

    module
        .verify()
        .map_err(|e| OptError::PostVerificationFailed(e.to_string()))
}

fn build_pass_options(config: &PipelineConfig) -> PassBuilderOptions {
    let options = PassBuilderOptions::create();
    options.set_loop_vectorization(config.loop_vectorization);
    options.set_loop_slp_vectorization(config.slp_vectorization);
    options.set_loop_unrolling(config.loop_unrolling);
    options.set_merge_functions(config.merge_functions);
    options.set_verify_each(config.verify_each);
    options.set_debug_logging(config.debug_logging);
    options
}
