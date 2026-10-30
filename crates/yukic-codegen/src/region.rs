#![deny(unsafe_code)]

use inkwell::AddressSpace;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module as LlvmModule;
use inkwell::types::{BasicType, BasicTypeEnum, FunctionType, PointerType, StructType};
use inkwell::values::{FunctionValue, IntValue, PointerValue};
use yukic_borrow::EscapeKind;
use yukic_types::Type;

pub const DEFAULT_APP_REGION_CAPACITY: u64 = 65536;
pub const DEFAULT_LOCAL_REGION_CAPACITY: u64 = 4096;
pub const DEFAULT_REGION_ALIGN: u64 = 16;

#[derive(Clone, Copy)]
pub struct RegionManager<'ctx> {
    context: &'ctx Context,
}

impl<'ctx> RegionManager<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self { context }
    }

    pub fn ptr_type(&self) -> PointerType<'ctx> {
        self.context.ptr_type(AddressSpace::default())
    }

    pub fn region_type(&self) -> StructType<'ctx> {
        let ptr = self.ptr_type();
        self.context
            .struct_type(&[ptr.into(), ptr.into(), ptr.into()], false)
    }

    pub fn is_regional_type(ty: &Type) -> bool {
        match ty {
            Type::String
            | Type::Bytes
            | Type::Array(_)
            | Type::Map(_, _)
            | Type::Class(_)
            | Type::Interface(_)
            | Type::Struct(_)
            | Type::Enum(_) => true,
            Type::Ref { inner, .. } => Self::is_regional_type(inner),
            Type::Union(variants) => variants.iter().any(Self::is_regional_type),
            _ => false,
        }
    }

    pub fn get_or_declare_init(&self, module: &LlvmModule<'ctx>) -> FunctionValue<'ctx> {
        if let Some(f) = module.get_function("yukino_region_init") {
            return f;
        }
        let fn_ty = self.context.void_type().fn_type(
            &[self.ptr_type().into(), self.context.i64_type().into()],
            false,
        );
        module.add_function("yukino_region_init", fn_ty, None)
    }

    pub fn get_or_declare_alloc(&self, module: &LlvmModule<'ctx>) -> FunctionValue<'ctx> {
        if let Some(f) = module.get_function("yukino_region_alloc") {
            return f;
        }
        let fn_ty = self.ptr_type().fn_type(
            &[
                self.ptr_type().into(),
                self.context.i64_type().into(),
                self.context.i64_type().into(),
            ],
            false,
        );
        module.add_function("yukino_region_alloc", fn_ty, None)
    }

    pub fn get_or_declare_destroy(&self, module: &LlvmModule<'ctx>) -> FunctionValue<'ctx> {
        if let Some(f) = module.get_function("yukino_region_destroy") {
            return f;
        }
        let fn_ty = self
            .context
            .void_type()
            .fn_type(&[self.ptr_type().into()], false);
        module.add_function("yukino_region_destroy", fn_ty, None)
    }

    pub fn get_or_declare_save_mark(&self, module: &LlvmModule<'ctx>) -> FunctionValue<'ctx> {
        if let Some(f) = module.get_function("yukino_region_save_mark") {
            return f;
        }
        let fn_ty = self.ptr_type().fn_type(&[self.ptr_type().into()], false);
        module.add_function("yukino_region_save_mark", fn_ty, None)
    }

    pub fn get_or_declare_restore_mark(&self, module: &LlvmModule<'ctx>) -> FunctionValue<'ctx> {
        if let Some(f) = module.get_function("yukino_region_restore_mark") {
            return f;
        }
        let fn_ty = self
            .context
            .void_type()
            .fn_type(&[self.ptr_type().into(), self.ptr_type().into()], false);
        module.add_function("yukino_region_restore_mark", fn_ty, None)
    }

    pub fn declare_intrinsics(&self, module: &LlvmModule<'ctx>) {
        self.get_or_declare_init(module);
        self.get_or_declare_alloc(module);
        self.get_or_declare_destroy(module);
        self.get_or_declare_save_mark(module);
        self.get_or_declare_restore_mark(module);
    }

    pub fn transform_function_type(
        &self,
        param_types: &[BasicTypeEnum<'ctx>],
        return_type: &Type,
        llvm_ret: Option<BasicTypeEnum<'ctx>>,
        is_cfun: bool,
    ) -> (FunctionType<'ctx>, bool) {
        self.transform_function_type_with_dest(param_types, return_type, llvm_ret, is_cfun, false)
    }

    pub fn transform_function_type_with_dest(
        &self,
        param_types: &[BasicTypeEnum<'ctx>],
        return_type: &Type,
        llvm_ret: Option<BasicTypeEnum<'ctx>>,
        is_cfun: bool,
        has_regional_ref: bool,
    ) -> (FunctionType<'ctx>, bool) {
        let needs_dest = !is_cfun && (Self::is_regional_type(return_type) || has_regional_ref);
        let mut final_params = Vec::with_capacity(param_types.len() + 1);

        if needs_dest {
            final_params.push(self.ptr_type().into());
        }
        for pt in param_types {
            final_params.push((*pt).into());
        }

        let fn_ty = match llvm_ret {
            Some(ret) => ret.fn_type(&final_params, false),
            None => self.context.void_type().fn_type(&final_params, false),
        };
        (fn_ty, needs_dest)
    }

    pub fn emit_region_prologue(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        initial_capacity: u64,
        name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        let region_alloca = builder
            .build_alloca(self.region_type(), name)
            .map_err(|e| e.to_string())?;

        let init_fn = self.get_or_declare_init(module);
        let cap_val = self.context.i64_type().const_int(initial_capacity, false);

        builder
            .build_call(init_fn, &[region_alloca.into(), cap_val.into()], "")
            .map_err(|e| e.to_string())?;

        Ok(region_alloca)
    }

    pub fn emit_region_epilogue(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        region_ptr: PointerValue<'ctx>,
    ) -> Result<(), String> {
        let destroy_fn = self.get_or_declare_destroy(module);
        builder
            .build_call(destroy_fn, &[region_ptr.into()], "")
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn emit_fast_path_alloc(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        current_fn: FunctionValue<'ctx>,
        region_ptr: PointerValue<'ctx>,
        size: u64,
        align: u64,
    ) -> Result<PointerValue<'ctx>, String> {
        let effective_align = align.max(1).next_power_of_two();
        let aligned_size = (size + effective_align - 1) & !(effective_align - 1);
        let aligned_size_val = self.context.i64_type().const_int(aligned_size, false);
        let orig_size_val = self.context.i64_type().const_int(size, false);
        let align_val = self.context.i64_type().const_int(effective_align, false);

        let cur_gep = builder
            .build_struct_gep(self.region_type(), region_ptr, 1, "cur_gep")
            .map_err(|e| e.to_string())?;
        let cur_ptr = builder
            .build_load(self.ptr_type(), cur_gep, "cur_ptr")
            .map_err(|e| e.to_string())?
            .into_pointer_value();

        let end_gep = builder
            .build_struct_gep(self.region_type(), region_ptr, 2, "end_gep")
            .map_err(|e| e.to_string())?;
        let end_ptr = builder
            .build_load(self.ptr_type(), end_gep, "end_ptr")
            .map_err(|e| e.to_string())?
            .into_pointer_value();

        let i64_type = self.context.i64_type();
        let cur_int = builder
            .build_ptr_to_int(cur_ptr, i64_type, "cur_int")
            .map_err(|e| e.to_string())?;

        let align_add = self.context.i64_type().const_int(effective_align - 1, false);
        let align_mask = self.context.i64_type().const_int(!(effective_align - 1), false);
        let cur_plus = builder
            .build_int_add(cur_int, align_add, "cur_plus")
            .map_err(|e| e.to_string())?;
        let aligned_cur = builder
            .build_and(cur_plus, align_mask, "aligned_cur")
            .map_err(|e| e.to_string())?;

        let new_int = builder
            .build_int_add(aligned_cur, aligned_size_val, "new_int")
            .map_err(|e| e.to_string())?;
        let end_int = builder
            .build_ptr_to_int(end_ptr, i64_type, "end_int")
            .map_err(|e| e.to_string())?;

        let fits = builder
            .build_int_compare(inkwell::IntPredicate::ULE, new_int, end_int, "fits")
            .map_err(|e| e.to_string())?;

        let fast_bb = self.context.append_basic_block(current_fn, "bump_fast");
        let slow_bb = self.context.append_basic_block(current_fn, "bump_slow");
        let merge_bb = self.context.append_basic_block(current_fn, "bump_merge");

        builder
            .build_conditional_branch(fits, fast_bb, slow_bb)
            .map_err(|e| e.to_string())?;

        // fast path
        builder.position_at_end(fast_bb);
        let new_ptr = builder
            .build_int_to_ptr(new_int, self.ptr_type(), "bumped_ptr")
            .map_err(|e| e.to_string())?;
        builder
            .build_store(cur_gep, new_ptr)
            .map_err(|e| e.to_string())?;
        let aligned_cur_ptr = builder
            .build_int_to_ptr(aligned_cur, self.ptr_type(), "aligned_cur_ptr")
            .map_err(|e| e.to_string())?;
        builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| e.to_string())?;

        // slow path
        builder.position_at_end(slow_bb);
        let alloc_fn = self.get_or_declare_alloc(module);
        let slow_call = builder
            .build_call(
                alloc_fn,
                &[region_ptr.into(), orig_size_val.into(), align_val.into()],
                "slow_ptr",
            )
            .map_err(|e| e.to_string())?;
        let slow_res = slow_call
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "expected return value from alloc".to_string())?
            .into_pointer_value();
        builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| e.to_string())?;

        // merge
        builder.position_at_end(merge_bb);
        let phi = builder
            .build_phi(self.ptr_type(), "alloc_ptr")
            .map_err(|e| e.to_string())?;
        phi.add_incoming(&[(&aligned_cur_ptr, fast_bb), (&slow_res, slow_bb)]);

        Ok(phi.as_basic_value().into_pointer_value())
    }

    pub fn emit_dynamic_fast_path_alloc(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        current_fn: FunctionValue<'ctx>,
        region_ptr: PointerValue<'ctx>,
        size_val: IntValue<'ctx>,
        align: u64,
    ) -> Result<PointerValue<'ctx>, String> {
        let effective_align = align.max(1).next_power_of_two();
        let align_val = self.context.i64_type().const_int(effective_align, false);

        let cur_gep = builder
            .build_struct_gep(self.region_type(), region_ptr, 1, "cur_gep")
            .map_err(|e| e.to_string())?;
        let cur_ptr = builder
            .build_load(self.ptr_type(), cur_gep, "cur_ptr")
            .map_err(|e| e.to_string())?
            .into_pointer_value();

        let end_gep = builder
            .build_struct_gep(self.region_type(), region_ptr, 2, "end_gep")
            .map_err(|e| e.to_string())?;
        let end_ptr = builder
            .build_load(self.ptr_type(), end_gep, "end_ptr")
            .map_err(|e| e.to_string())?
            .into_pointer_value();

        let i64_type = self.context.i64_type();
        let cur_int = builder
            .build_ptr_to_int(cur_ptr, i64_type, "cur_int")
            .map_err(|e| e.to_string())?;

        let one_val = self.context.i64_type().const_int(1, false);
        let align_minus_one = builder
            .build_int_sub(align_val, one_val, "align_m1")
            .map_err(|e| e.to_string())?;
        let align_mask = builder
            .build_not(align_minus_one, "align_mask")
            .map_err(|e| e.to_string())?;
        let cur_plus = builder
            .build_int_add(cur_int, align_minus_one, "cur_plus")
            .map_err(|e| e.to_string())?;
        let aligned_cur = builder
            .build_and(cur_plus, align_mask, "aligned_cur")
            .map_err(|e| e.to_string())?;

        let new_int = builder
            .build_int_add(aligned_cur, size_val, "new_int")
            .map_err(|e| e.to_string())?;
        let end_int = builder
            .build_ptr_to_int(end_ptr, i64_type, "end_int")
            .map_err(|e| e.to_string())?;

        let fits = builder
            .build_int_compare(inkwell::IntPredicate::ULE, new_int, end_int, "fits")
            .map_err(|e| e.to_string())?;

        let fast_bb = self.context.append_basic_block(current_fn, "bump_fast");
        let slow_bb = self.context.append_basic_block(current_fn, "bump_slow");
        let merge_bb = self.context.append_basic_block(current_fn, "bump_merge");

        builder
            .build_conditional_branch(fits, fast_bb, slow_bb)
            .map_err(|e| e.to_string())?;

        // fast path
        builder.position_at_end(fast_bb);
        let new_ptr = builder
            .build_int_to_ptr(new_int, self.ptr_type(), "bumped_ptr")
            .map_err(|e| e.to_string())?;
        builder
            .build_store(cur_gep, new_ptr)
            .map_err(|e| e.to_string())?;
        let aligned_cur_ptr = builder
            .build_int_to_ptr(aligned_cur, self.ptr_type(), "aligned_cur_ptr")
            .map_err(|e| e.to_string())?;
        builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| e.to_string())?;

        // slow path
        builder.position_at_end(slow_bb);
        let alloc_fn = self.get_or_declare_alloc(module);
        let slow_call = builder
            .build_call(
                alloc_fn,
                &[region_ptr.into(), size_val.into(), align_val.into()],
                "slow_ptr",
            )
            .map_err(|e| e.to_string())?;
        let slow_res = slow_call
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "expected return value from alloc".to_string())?
            .into_pointer_value();
        builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| e.to_string())?;

        // merge
        builder.position_at_end(merge_bb);
        let phi = builder
            .build_phi(self.ptr_type(), "alloc_ptr")
            .map_err(|e| e.to_string())?;
        phi.add_incoming(&[(&aligned_cur_ptr, fast_bb), (&slow_res, slow_bb)]);

        Ok(phi.as_basic_value().into_pointer_value())
    }

    pub fn emit_loop_save_mark(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        region_ptr: PointerValue<'ctx>,
    ) -> Result<PointerValue<'ctx>, String> {
        let save_fn = self.get_or_declare_save_mark(module);
        let call = builder
            .build_call(save_fn, &[region_ptr.into()], "loop_mark")
            .map_err(|e| e.to_string())?;
        let res = call
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "expected mark pointer".to_string())?
            .into_pointer_value();
        Ok(res)
    }

    pub fn emit_loop_restore_mark(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        region_ptr: PointerValue<'ctx>,
        mark: PointerValue<'ctx>,
    ) -> Result<(), String> {
        let restore_fn = self.get_or_declare_restore_mark(module);
        builder
            .build_call(restore_fn, &[region_ptr.into(), mark.into()], "")
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn select_region_for_escape(
        &self,
        escape_kind: EscapeKind,
        local_region: PointerValue<'ctx>,
        dest_region: Option<PointerValue<'ctx>>,
    ) -> PointerValue<'ctx> {
        match escape_kind {
            EscapeKind::NoEscape => local_region,
            EscapeKind::LoopEscape | EscapeKind::EscapeToCaller | EscapeKind::GlobalEscape => {
                dest_region.unwrap_or(local_region)
            }
        }
    }
}
