#![deny(unsafe_code)]

use inkwell::AddressSpace;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module as LlvmModule};
use inkwell::types::{BasicTypeEnum, IntType, PointerType, StructType};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValueEnum, InstructionValue, IntValue, PointerValue, StructValue,
};
use inkwell::{FloatPredicate, IntPredicate};
use yukic_types::Type;

#[derive(Debug, Clone, Copy)]
pub struct SliceElemLayout<'ctx> {
    pub size: IntValue<'ctx>,
    pub align: IntValue<'ctx>,
}

pub struct IntrinsicManager<'ctx> {
    pub context: &'ctx Context,
}

impl<'ctx> IntrinsicManager<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self { context }
    }

    pub fn ptr_type(&self) -> PointerType<'ctx> {
        self.context.ptr_type(AddressSpace::default())
    }

    pub fn get_int_type(&self, bits: u32) -> IntType<'ctx> {
        match bits {
            1 => self.context.bool_type(),
            8 => self.context.i8_type(),
            16 => self.context.i16_type(),
            32 => self.context.i32_type(),
            _ => self.context.i64_type(),
        }
    }

    pub fn string_type(&self) -> StructType<'ctx> {
        self.context.struct_type(
            &[self.ptr_type().into(), self.context.i64_type().into()],
            false,
        )
    }

    pub fn slice_type(&self) -> StructType<'ctx> {
        let i64_ty = self.context.i64_type();
        self.context.struct_type(
            &[self.ptr_type().into(), i64_ty.into(), i64_ty.into()],
            false,
        )
    }

    pub fn declare_intrinsics(&self, module: &LlvmModule<'ctx>) {
        let void_ty = self.context.void_type();
        let i32_ty = self.context.i32_type();
        let i64_ty = self.context.i64_type();
        let i1_ty = self.context.bool_type();
        let f64_ty = self.context.f64_type();
        let ptr_ty = self.ptr_type();
        let str_ty = self.string_type();
        let slice_ty = self.slice_type();

        // exit(i32) -> void with noreturn attribute
        if module.get_function("exit").is_none() {
            let exit_fn_ty = void_ty.fn_type(&[i32_ty.into()], false);
            let exit_fn = module.add_function("exit", exit_fn_ty, None);
            let noreturn_kind = Attribute::get_named_enum_kind_id("noreturn");
            let noreturn_attr = self.context.create_enum_attribute(noreturn_kind, 0);
            exit_fn.add_attribute(AttributeLoc::Function, noreturn_attr);
        }

        // to-string helpers use an out pointer: 16-byte struct returns diverge on Win64
        Self::declare_sret_helper(
            self.context,
            module,
            str_ty,
            "yukino_rt_char_to_string",
            &[ptr_ty.into(), i32_ty.into()],
        );
        Self::declare_sret_helper(
            self.context,
            module,
            str_ty,
            "yukino_rt_int64_to_string",
            &[ptr_ty.into(), i64_ty.into()],
        );
        Self::declare_sret_helper(
            self.context,
            module,
            str_ty,
            "yukino_rt_uint64_to_string",
            &[ptr_ty.into(), i64_ty.into()],
        );
        Self::declare_sret_helper(
            self.context,
            module,
            str_ty,
            "yukino_rt_float64_to_string",
            &[ptr_ty.into(), f64_ty.into()],
        );
        Self::declare_sret_helper(
            self.context,
            module,
            str_ty,
            "yukino_rt_ptr_to_string",
            &[ptr_ty.into(), ptr_ty.into()],
        );
        Self::declare_sret_helper(
            self.context,
            module,
            str_ty,
            "yukino_rt_bool_to_string",
            &[ptr_ty.into(), i1_ty.into()],
        );

        // runtime cast helpers (from string)
        Self::declare_helper(
            module,
            "yukino_rt_string_to_char",
            i32_ty.fn_type(&[ptr_ty.into(), i64_ty.into()], false),
        );
        Self::declare_helper(
            module,
            "yukino_rt_string_to_int64",
            i64_ty.fn_type(&[ptr_ty.into(), i64_ty.into()], false),
        );
        Self::declare_helper(
            module,
            "yukino_rt_string_to_uint64",
            i64_ty.fn_type(&[ptr_ty.into(), i64_ty.into()], false),
        );
        Self::declare_helper(
            module,
            "yukino_rt_string_to_float64",
            f64_ty.fn_type(&[ptr_ty.into(), i64_ty.into()], false),
        );
        Self::declare_helper(
            module,
            "yukino_rt_string_to_ptr",
            ptr_ty.fn_type(&[ptr_ty.into(), i64_ty.into()], false),
        );
        Self::declare_helper(
            module,
            "yukino_rt_string_to_bool",
            i1_ty.fn_type(&[ptr_ty.into(), i64_ty.into()], false),
        );
        Self::declare_sret_helper(
            self.context,
            module,
            slice_ty,
            "yukino_rt_string_to_chars",
            &[ptr_ty.into(), ptr_ty.into(), i64_ty.into()],
        );
        Self::declare_sret_helper(
            self.context,
            module,
            str_ty,
            "yukino_rt_chars_to_string",
            &[ptr_ty.into(), ptr_ty.into(), i64_ty.into()],
        );

        // runtime args helpers
        let sret_kind = Attribute::get_named_enum_kind_id("sret");
        let sret_attr = self.context.create_type_attribute(sret_kind, slice_ty.into());
        if module.get_function("yukino_rt_get_args").is_none() {
            let fn_ty = void_ty.fn_type(
                &[ptr_ty.into(), ptr_ty.into(), i32_ty.into(), ptr_ty.into()],
                false,
            );
            let f = module.add_function("yukino_rt_get_args", fn_ty, None);
            f.add_attribute(AttributeLoc::Param(0), sret_attr);
        }
        Self::declare_sret_helper(
            self.context,
            module,
            str_ty,
            "yukino_rt_get_raw_args",
            &[ptr_ty.into(), i32_ty.into(), ptr_ty.into()],
        );

        // globals for main process args
        if module.get_global("_Y_argc").is_none() {
            let argc_var = module.add_global(i32_ty, Some(AddressSpace::default()), "_Y_argc");
            argc_var.set_initializer(&i32_ty.const_int(0, false));
            argc_var.set_linkage(Linkage::Common);
        }
        if module.get_global("_Y_argv").is_none() {
            let argv_var = module.add_global(ptr_ty, Some(AddressSpace::default()), "_Y_argv");
            argv_var.set_initializer(&ptr_ty.const_null());
            argv_var.set_linkage(Linkage::Common);
        }

        // runtime slice helpers
        Self::declare_helper(
            module,
            "yukino_rt_slice_push",
            void_ty.fn_type(
                &[
                    ptr_ty.into(),
                    ptr_ty.into(),
                    ptr_ty.into(),
                    i64_ty.into(),
                    i64_ty.into(),
                ],
                false,
            ),
        );
        Self::declare_helper(
            module,
            "yukino_rt_slice_del",
            void_ty.fn_type(&[ptr_ty.into(), i64_ty.into(), i64_ty.into()], false),
        );
        Self::declare_helper(
            module,
            "yukino_rt_slice_reserve",
            void_ty.fn_type(
                &[
                    ptr_ty.into(),
                    ptr_ty.into(),
                    i64_ty.into(),
                    i64_ty.into(),
                    i64_ty.into(),
                ],
                false,
            ),
        );

        // raw parts constructors
        Self::declare_sret_helper(
            self.context,
            module,
            str_ty,
            "yukino_rt_string_from_raw_parts",
            &[ptr_ty.into(), i64_ty.into()],
        );
        Self::declare_sret_helper(
            self.context,
            module,
            slice_ty,
            "yukino_rt_slice_from_raw_parts",
            &[ptr_ty.into(), i64_ty.into(), i64_ty.into()],
        );
    }

    /// Declares a void helper writing its struct result via an `sret` out pointer.
    fn declare_sret_helper(
        context: &'ctx Context,
        module: &LlvmModule<'ctx>,
        out_ty: inkwell::types::StructType<'ctx>,
        name: &str,
        args: &[inkwell::types::BasicMetadataTypeEnum<'ctx>],
    ) {
        if module.get_function(name).is_some() {
            return;
        }
        let void_ty = context.void_type();
        let ptr_ty = context.ptr_type(AddressSpace::default());
        let mut param_types: Vec<inkwell::types::BasicMetadataTypeEnum<'ctx>> =
            Vec::with_capacity(args.len() + 1);
        param_types.push(ptr_ty.into());
        param_types.extend_from_slice(args);
        let fn_ty = void_ty.fn_type(&param_types, false);
        let f = module.add_function(name, fn_ty, None);
        let sret_kind = Attribute::get_named_enum_kind_id("sret");
        let sret_attr = context.create_type_attribute(sret_kind, out_ty.into());
        f.add_attribute(AttributeLoc::Param(0), sret_attr);
    }

    fn declare_helper(
        module: &LlvmModule<'ctx>,
        name: &str,
        fn_type: inkwell::types::FunctionType<'ctx>,
    ) {
        if module.get_function(name).is_none() {
            module.add_function(name, fn_type, None);
        }
    }

    pub fn setup_main_args(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        argc_val: IntValue<'ctx>,
        argv_val: PointerValue<'ctx>,
    ) -> Result<(), String> {
        let argc_global = module
            .get_global("_Y_argc")
            .ok_or("_Y_argc global not declared")?;
        let argv_global = module
            .get_global("_Y_argv")
            .ok_or("_Y_argv global not declared")?;

        builder
            .build_store(argc_global.as_pointer_value(), argc_val)
            .map_err(|e| e.to_string())?;
        builder
            .build_store(argv_global.as_pointer_value(), argv_val)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn emit_exit(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        code_val: IntValue<'ctx>,
    ) -> Result<InstructionValue<'ctx>, String> {
        let exit_fn = module
            .get_function("exit")
            .ok_or("exit intrinsic function not declared")?;

        let call_site = builder
            .build_call(exit_fn, &[code_val.into()], "")
            .map_err(|e| e.to_string())?;
        let noreturn_kind = Attribute::get_named_enum_kind_id("noreturn");
        let noreturn_attr = self.context.create_enum_attribute(noreturn_kind, 0);
        call_site.add_attribute(AttributeLoc::Function, noreturn_attr);

        builder.build_unreachable().map_err(|e| e.to_string())
    }

    pub fn emit_args(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        region_ptr: PointerValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let get_args_fn = module
            .get_function("yukino_rt_get_args")
            .ok_or("yukino_rt_get_args not declared")?;
        let argc_global = module.get_global("_Y_argc").ok_or("_Y_argc not declared")?;
        let argv_global = module.get_global("_Y_argv").ok_or("_Y_argv not declared")?;

        let argc = builder
            .build_load(
                self.context.i32_type(),
                argc_global.as_pointer_value(),
                "argc_val",
            )
            .map_err(|e| e.to_string())?
            .into_int_value();
        let argv = builder
            .build_load(self.ptr_type(), argv_global.as_pointer_value(), "argv_val")
            .map_err(|e| e.to_string())?
            .into_pointer_value();

        let one_i32 = self.context.i32_type().const_int(1, false);
        let zero_i32 = self.context.i32_type().const_zero();
        let has_args = builder
            .build_int_compare(IntPredicate::SGT, argc, one_i32, "has_user_args")
            .map_err(|e| e.to_string())?;
        let sub_argc = builder
            .build_int_sub(argc, one_i32, "user_argc")
            .map_err(|e| e.to_string())?;
        let eff_argc = builder
            .build_select(has_args, sub_argc, zero_i32, "eff_argc")
            .map_err(|e| e.to_string())?;

        let i64_ty = self.context.i64_type();
        let ptr_size = i64_ty.const_int(8, false);
        let argv_int = builder
            .build_ptr_to_int(argv, i64_ty, "argv_int")
            .map_err(|e| e.to_string())?;
        let shifted_int = builder
            .build_int_add(argv_int, ptr_size, "shifted_int")
            .map_err(|e| e.to_string())?;
        let shifted_argv = builder
            .build_int_to_ptr(shifted_int, self.ptr_type(), "shifted_argv")
            .map_err(|e| e.to_string())?;
        let eff_argv = builder
            .build_select(has_args, shifted_argv, argv, "eff_argv")
            .map_err(|e| e.to_string())?;

        let sret_mem = builder
            .build_alloca(self.slice_type(), "args_sret")
            .map_err(|e| e.to_string())?;

        let call = builder
            .build_call(
                get_args_fn,
                &[
                    sret_mem.into(),
                    region_ptr.into(),
                    eff_argc.into(),
                    eff_argv.into(),
                ],
                "",
            )
            .map_err(|e| e.to_string())?;

        let sret_kind = Attribute::get_named_enum_kind_id("sret");
        let sret_attr = self
            .context
            .create_type_attribute(sret_kind, self.slice_type().into());
        call.add_attribute(AttributeLoc::Param(0), sret_attr);

        builder
            .build_load(self.slice_type(), sret_mem, "args_slice")
            .map_err(|e| e.to_string())
    }

    pub fn emit_raw_args(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        region_ptr: PointerValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let get_raw_fn = module
            .get_function("yukino_rt_get_raw_args")
            .ok_or("yukino_rt_get_raw_args not declared")?;
        let argc_global = module.get_global("_Y_argc").ok_or("_Y_argc not declared")?;
        let argv_global = module.get_global("_Y_argv").ok_or("_Y_argv not declared")?;

        let argc = builder
            .build_load(
                self.context.i32_type(),
                argc_global.as_pointer_value(),
                "argc_val",
            )
            .map_err(|e| e.to_string())?
            .into_int_value();
        let argv = builder
            .build_load(self.ptr_type(), argv_global.as_pointer_value(), "argv_val")
            .map_err(|e| e.to_string())?
            .into_pointer_value();

        let one_i32 = self.context.i32_type().const_int(1, false);
        let zero_i32 = self.context.i32_type().const_zero();
        let has_args = builder
            .build_int_compare(IntPredicate::SGT, argc, one_i32, "has_user_args")
            .map_err(|e| e.to_string())?;
        let sub_argc = builder
            .build_int_sub(argc, one_i32, "user_argc")
            .map_err(|e| e.to_string())?;
        let eff_argc = builder
            .build_select(has_args, sub_argc, zero_i32, "eff_argc")
            .map_err(|e| e.to_string())?;

        let i64_ty = self.context.i64_type();
        let ptr_size = i64_ty.const_int(8, false);
        let argv_int = builder
            .build_ptr_to_int(argv, i64_ty, "argv_int")
            .map_err(|e| e.to_string())?;
        let shifted_int = builder
            .build_int_add(argv_int, ptr_size, "shifted_int")
            .map_err(|e| e.to_string())?;
        let shifted_argv = builder
            .build_int_to_ptr(shifted_int, self.ptr_type(), "shifted_argv")
            .map_err(|e| e.to_string())?;
        let eff_argv = builder
            .build_select(has_args, shifted_argv, argv, "eff_argv")
            .map_err(|e| e.to_string())?;

        let str_ty = self.string_type();
        let out_slot = builder
            .build_alloca(str_ty, "raw_args_slot")
            .map_err(|e| e.to_string())?;
        builder
            .build_call(
                get_raw_fn,
                &[
                    out_slot.into(),
                    region_ptr.into(),
                    eff_argc.into(),
                    eff_argv.into(),
                ],
                "",
            )
            .map_err(|e| e.to_string())?;
        let result = builder
            .build_load(str_ty, out_slot, "raw_args_str")
            .map_err(|e| e.to_string())?;
        Ok(result)
    }

    pub fn emit_slice_len(
        &self,
        builder: &Builder<'ctx>,
        slice_or_str: BasicValueEnum<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        match slice_or_str {
            BasicValueEnum::StructValue(sv) => {
                let len = builder
                    .build_extract_value(sv, 1, "len")
                    .map_err(|e| e.to_string())?;
                Ok(len.into_int_value())
            }
            BasicValueEnum::PointerValue(pv) => {
                // len is at field index 1 in both string and slice types
                let slice_ty = self.slice_type();
                let gep = builder
                    .build_struct_gep(slice_ty, pv, 1, "len_gep")
                    .or_else(|_| {
                        let str_ty = self.string_type();
                        builder.build_struct_gep(str_ty, pv, 1, "len_gep")
                    })
                    .map_err(|e| e.to_string())?;
                let val = builder
                    .build_load(self.context.i64_type(), gep, "len")
                    .map_err(|e| e.to_string())?;
                Ok(val.into_int_value())
            }
            _ => Err("expected struct or pointer value for slice len".to_string()),
        }
    }

    pub fn emit_slice_cap(
        &self,
        builder: &Builder<'ctx>,
        slice_val: BasicValueEnum<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        match slice_val {
            BasicValueEnum::StructValue(sv) => {
                let cap = builder
                    .build_extract_value(sv, 2, "cap")
                    .map_err(|e| e.to_string())?;
                Ok(cap.into_int_value())
            }
            BasicValueEnum::PointerValue(pv) => {
                // cap is at field index 2 in slice layout
                let slice_ty = self.slice_type();
                let gep = builder
                    .build_struct_gep(slice_ty, pv, 2, "cap_gep")
                    .map_err(|e| e.to_string())?;
                let val = builder
                    .build_load(self.context.i64_type(), gep, "cap")
                    .map_err(|e| e.to_string())?;
                Ok(val.into_int_value())
            }
            _ => Err("expected struct or pointer value for slice cap".to_string()),
        }
    }

    pub fn emit_slice_as_ptr(
        &self,
        builder: &Builder<'ctx>,
        slice_or_str: BasicValueEnum<'ctx>,
    ) -> Result<PointerValue<'ctx>, String> {
        match slice_or_str {
            BasicValueEnum::StructValue(sv) => {
                let ptr = builder
                    .build_extract_value(sv, 0, "as_ptr")
                    .map_err(|e| e.to_string())?;
                Ok(ptr.into_pointer_value())
            }
            BasicValueEnum::PointerValue(pv) => {
                // buf ptr is at field index 0 in both str and slice layouts
                let slice_ty = self.slice_type();
                let gep = builder
                    .build_struct_gep(slice_ty, pv, 0, "ptr_gep")
                    .or_else(|_| {
                        let str_ty = self.string_type();
                        builder.build_struct_gep(str_ty, pv, 0, "ptr_gep")
                    })
                    .map_err(|e| e.to_string())?;
                let val = builder
                    .build_load(self.ptr_type(), gep, "as_ptr")
                    .map_err(|e| e.to_string())?;
                Ok(val.into_pointer_value())
            }
            _ => Err(format!("expected struct or pointer value for slice as_ptr, got {:?}", slice_or_str)),
        }
    }

    pub fn emit_slice_push(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        region_ptr: PointerValue<'ctx>,
        slice_ptr: PointerValue<'ctx>,
        elem_ptr: PointerValue<'ctx>,
        layout: SliceElemLayout<'ctx>,
    ) -> Result<(), String> {
        let push_fn = module
            .get_function("yukino_rt_slice_push")
            .ok_or("yukino_rt_slice_push not declared")?;
        builder
            .build_call(
                push_fn,
                &[
                    region_ptr.into(),
                    slice_ptr.into(),
                    elem_ptr.into(),
                    layout.size.into(),
                    layout.align.into(),
                ],
                "",
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn emit_slice_del(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        slice_ptr: PointerValue<'ctx>,
        index: IntValue<'ctx>,
        elem_size: IntValue<'ctx>,
    ) -> Result<(), String> {
        let del_fn = module
            .get_function("yukino_rt_slice_del")
            .ok_or("yukino_rt_slice_del not declared")?;
        builder
            .build_call(
                del_fn,
                &[slice_ptr.into(), index.into(), elem_size.into()],
                "",
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn emit_slice_reserve(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        region_ptr: PointerValue<'ctx>,
        slice_ptr: PointerValue<'ctx>,
        additional: IntValue<'ctx>,
        layout: SliceElemLayout<'ctx>,
    ) -> Result<(), String> {
        let reserve_fn = module
            .get_function("yukino_rt_slice_reserve")
            .ok_or("yukino_rt_slice_reserve not declared")?;
        builder
            .build_call(
                reserve_fn,
                &[
                    region_ptr.into(),
                    slice_ptr.into(),
                    additional.into(),
                    layout.size.into(),
                    layout.align.into(),
                ],
                "",
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn emit_string_from_raw_parts(
        &self,
        builder: &Builder<'ctx>,
        ptr_val: PointerValue<'ctx>,
        len_val: IntValue<'ctx>,
    ) -> Result<StructValue<'ctx>, String> {
        let str_ty = self.string_type();
        let s0 = str_ty.const_zero();
        let s1 = builder
            .build_insert_value(s0, ptr_val, 0, "str_ptr")
            .map_err(|e| e.to_string())?;
        let s2 = builder
            .build_insert_value(s1, len_val, 1, "str_len")
            .map_err(|e| e.to_string())?;
        Ok(s2.into_struct_value())
    }

    pub fn emit_slice_from_raw_parts(
        &self,
        builder: &Builder<'ctx>,
        ptr_val: PointerValue<'ctx>,
        len_val: IntValue<'ctx>,
        cap_val: IntValue<'ctx>,
    ) -> Result<StructValue<'ctx>, String> {
        let slice_ty = self.slice_type();
        let s0 = slice_ty.const_zero();
        let s1 = builder
            .build_insert_value(s0, ptr_val, 0, "slice_ptr")
            .map_err(|e| e.to_string())?;
        let s2 = builder
            .build_insert_value(s1, len_val, 1, "slice_len")
            .map_err(|e| e.to_string())?;
        let s3 = builder
            .build_insert_value(s2, cap_val, 2, "slice_cap")
            .map_err(|e| e.to_string())?;
        Ok(s3.into_struct_value())
    }

    #[allow(unsafe_code)]
    pub fn emit_slice_elem_ptr(
        &self,
        builder: &Builder<'ctx>,
        slice_or_str: BasicValueEnum<'ctx>,
        index: IntValue<'ctx>,
        elem_ty: BasicTypeEnum<'ctx>,
    ) -> Result<PointerValue<'ctx>, String> {
        let base_ptr = self.emit_slice_as_ptr(builder, slice_or_str)?;
        let i64_ty = self.context.i64_type();
        let idx_i64 = if index.get_type().get_bit_width() < 64 {
            builder
                .build_int_z_extend(index, i64_ty, "idx_i64")
                .map_err(|e| e.to_string())?
        } else {
            index
        };

        unsafe {
            builder
                .build_in_bounds_gep(elem_ty, base_ptr, &[idx_i64], "elem_ptr")
                .map_err(|e| e.to_string())
        }
    }

    pub fn emit_subslice(
        &self,
        builder: &Builder<'ctx>,
        slice_or_str: BasicValueEnum<'ctx>,
        start: IntValue<'ctx>,
        end: IntValue<'ctx>,
        elem_size: IntValue<'ctx>,
        is_string: bool,
    ) -> Result<StructValue<'ctx>, String> {
        let start_bytes = builder
            .build_int_mul(start, elem_size, "start_bytes")
            .map_err(|e| e.to_string())?;
        let sub_ptr = self.emit_slice_elem_ptr(builder, slice_or_str, start_bytes, self.context.i8_type().into())?;
        let i64_ty = self.context.i64_type();
        let start_i64 = if start.get_type().get_bit_width() < 64 {
            builder
                .build_int_z_extend(start, i64_ty, "start_i64")
                .map_err(|e| e.to_string())?
        } 


        else { start };

        let end_i64 = if end.get_type().get_bit_width() < 64 {
            builder
                .build_int_z_extend(end, i64_ty, "end_i64")
                .map_err(|e| e.to_string())?
        } 

        else { end };


        let sub_len = builder
            .build_int_sub(end_i64, start_i64, "sub_len")
            .map_err(|e| e.to_string())?;
        if is_string {
            self.emit_string_from_raw_parts(builder, sub_ptr, sub_len)
        } else {
            self.emit_slice_from_raw_parts(builder, sub_ptr, sub_len, sub_len)
        }
    }

    pub fn emit_cast(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        val: BasicValueEnum<'ctx>,
        src_ty: &Type,
        target_ty: &Type,
        region_ptr: Option<PointerValue<'ctx>>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if src_ty == target_ty {
            return Ok(val);
        }

        // const int bit width overflow validation
        let const_overflow = Self::try_extract_const_int(val, src_ty)
            .filter(|&n| target_ty.is_integer() && !target_ty.fits_i64(n));
        if let Some(n) = const_overflow {
            let bits = target_ty.bit_width().unwrap_or(64);
            return Err(format!(
                "value `{n}` exceeds {bits} bits for target type `{target_ty}`"
            ));
        }

        if (src_ty.is_integer() || *src_ty == Type::Char)
            && (target_ty.is_integer() || *target_ty == Type::Char)
        {
            return self.cast_int_to_int(builder, val.into_int_value(), src_ty, target_ty);
        }

        if src_ty.is_float() && target_ty.is_float() {
            return self.cast_float_to_float(builder, val.into_float_value(), src_ty, target_ty);
        }

        if src_ty.is_integer() && target_ty.is_float() {
            return self.cast_int_to_float(builder, val.into_int_value(), src_ty, target_ty);
        }

        if src_ty.is_float() && target_ty.is_integer() {
            return self.cast_float_to_int(builder, val.into_float_value(), target_ty);
        }

        if matches!(src_ty, Type::Ptr(_) | Type::Fn { .. })
            && matches!(target_ty, Type::Ptr(_) | Type::Fn { .. })

                        { return Ok(val); }

        if (matches!(src_ty, Type::Ptr(_) | Type::Fn { .. })) && target_ty.is_integer() {
            return self.cast_ptr_to_int(builder, val.into_pointer_value(), target_ty);
        }

        if src_ty.is_integer() && (matches!(target_ty, Type::Ptr(_) | Type::Fn { .. })) {
            return self.cast_int_to_ptr(builder, val.into_int_value());
        }

        if let (Type::Ptr(_), Type::Float64) = (src_ty, target_ty) {
            return self.cast_ptr_to_float64(builder, val.into_pointer_value());
        }

        if let (Type::Float64, Type::Ptr(_)) = (src_ty, target_ty) {
            return self.cast_float64_to_ptr(builder, val.into_float_value());
        }

        if (src_ty.is_numeric() || *src_ty == Type::Char || *src_ty == Type::Bool || matches!(src_ty, Type::Ptr(_)))
            && *target_ty == Type::String
        {
            let reg = region_ptr.ok_or("string conversion requires active destination region")?;
            return self.cast_to_string(module, builder, val, src_ty, reg);
        }

        if *src_ty == Type::String
            && (target_ty.is_numeric()
                || *target_ty == Type::Char
                || *target_ty == Type::Bool
                || matches!(target_ty, Type::Ptr(_)))
        {
            return self.cast_from_string(module, builder, val.into_struct_value(), target_ty);
        }

        if *src_ty == Type::Bytes && *target_ty == Type::String {
            return self.cast_bytes_to_string(builder, val.into_struct_value());
        }

        if *src_ty == Type::String && *target_ty == Type::Bytes {
            return self.cast_string_to_bytes(builder, val.into_struct_value());
        }

        if *src_ty == Type::String && *target_ty == Type::Array(Box::new(Type::Char)) {
            let reg = region_ptr.ok_or("char[] conversion requires active destination region")?;
            return self.cast_string_to_chars(module, builder, val.into_struct_value(), reg);
        }

        if *src_ty == Type::Array(Box::new(Type::Char)) && *target_ty == Type::String {
            let reg = region_ptr.ok_or("string conversion requires active destination region")?;
            return self.cast_chars_to_string(module, builder, val.into_struct_value(), reg);
        }

        if *src_ty == Type::Bool || *target_ty == Type::Bool {
            return self.cast_bool(builder, val, src_ty, target_ty);
        }

        // raw bitcast between samesize types
        if src_ty.bit_width() == target_ty.bit_width() && src_ty.bit_width().is_some() {
            let target_llvm_ty = self.target_llvm_type(target_ty)?;
            return builder
                .build_bit_cast(val, target_llvm_ty, "cast_bitcast")
                .map_err(|e| e.to_string());
        }

        Err(format!(
            "unsupported type cast from `{src_ty}` to `{target_ty}`"
        ))
    }

    fn try_extract_const_int(val: BasicValueEnum<'ctx>, ty: &Type) -> Option<i64> {
        if ty.is_integer() && val.is_int_value() {
            let int_val = val.into_int_value();
            if int_val.is_const() {
                return int_val.get_sign_extended_constant();
            }
        }
        None
    }

    fn target_llvm_type(&self, ty: &Type) -> Result<inkwell::types::BasicTypeEnum<'ctx>, String> {
        match ty {
            Type::Bool => Ok(self.context.bool_type().into()),
            Type::Int8 | Type::Uint8 => Ok(self.context.i8_type().into()),
            Type::Int16 | Type::Uint16 => Ok(self.context.i16_type().into()),
            Type::Int32 | Type::Uint32 | Type::Char => Ok(self.context.i32_type().into()),
            Type::Int64 | Type::Uint64 => Ok(self.context.i64_type().into()),
            Type::Float32 => Ok(self.context.f32_type().into()),
            Type::Float64 => Ok(self.context.f64_type().into()),
            Type::Ptr(_) => Ok(self.ptr_type().into()),
            Type::String => Ok(self.string_type().into()),
            Type::Bytes => Ok(self.slice_type().into()),
            Type::Array(_) => Ok(self.slice_type().into()),
            _ => Err(format!("no primitive LLVM type mapping for `{ty}`")),
        }
    }

    fn cast_int_to_int(
        &self,
        builder: &Builder<'ctx>,
        val: IntValue<'ctx>,
        src_ty: &Type,
        target_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let src_bits = src_ty.bit_width().unwrap_or(64);
        let target_bits = target_ty.bit_width().unwrap_or(64);
        let target_int_ty = self.get_int_type(target_bits);

        match src_bits.cmp(&target_bits) {
            std::cmp::Ordering::Greater => builder
                .build_int_truncate(val, target_int_ty, "cast_trunc")
                .map(Into::into)
                .map_err(|e| e.to_string()),
            std::cmp::Ordering::Less => {
                if src_ty.is_signed_integer() {
                    builder
                        .build_int_s_extend(val, target_int_ty, "cast_sext")
                        .map(Into::into)
                        .map_err(|e| e.to_string())
                } else {
                    builder
                        .build_int_z_extend(val, target_int_ty, "cast_zext")
                        .map(Into::into)
                        .map_err(|e| e.to_string())
                }
            }
            std::cmp::Ordering::Equal => Ok(val.into()),
        }
    }

    fn cast_float_to_float(
        &self,
        builder: &Builder<'ctx>,
        val: inkwell::values::FloatValue<'ctx>,
        src_ty: &Type,
        target_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if *src_ty == Type::Float32 && *target_ty == Type::Float64 {
            builder
                .build_float_ext(val, self.context.f64_type(), "cast_fpext")
                .map(Into::into)
                .map_err(|e| e.to_string())
        } else if *src_ty == Type::Float64 && *target_ty == Type::Float32 {
            builder
                .build_float_trunc(val, self.context.f32_type(), "cast_fptrunc")
                .map(Into::into)
                .map_err(|e| e.to_string())
        } else {
            Ok(val.into())
        }
    }

    fn cast_int_to_float(
        &self,
        builder: &Builder<'ctx>,
        val: IntValue<'ctx>,
        src_ty: &Type,
        target_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let flt_ty = if *target_ty == Type::Float32 { self.context.f32_type()} 
        else { self.context.f64_type() };

        if src_ty.is_signed_integer() {
            builder
                .build_signed_int_to_float(val, flt_ty, "cast_sitofp")
                .map(Into::into)
                .map_err(|e| e.to_string())
        } else {
            builder
                .build_unsigned_int_to_float(val, flt_ty, "cast_uitofp")
                .map(Into::into)
                .map_err(|e| e.to_string())
        }
    }

    fn cast_float_to_int(
        &self,
        builder: &Builder<'ctx>,
        val: inkwell::values::FloatValue<'ctx>,
        target_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let bits = target_ty.bit_width().unwrap_or(64);
        let int_ty = self.get_int_type(bits);

        if target_ty.is_signed_integer() {
            builder
                .build_float_to_signed_int(val, int_ty, "cast_fptosi")
                .map(Into::into)
                .map_err(|e| e.to_string())
        } else {
            builder
                .build_float_to_unsigned_int(val, int_ty, "cast_fptoui")
                .map(Into::into)
                .map_err(|e| e.to_string())
        }
    }

    fn cast_ptr_to_int(
        &self,
        builder: &Builder<'ctx>,
        val: PointerValue<'ctx>,
        target_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let bits = target_ty.bit_width().unwrap_or(64);
        let int_ty = self.get_int_type(bits);
        builder
            .build_ptr_to_int(val, int_ty, "cast_ptrtoint")
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    fn cast_int_to_ptr(
        &self,
        builder: &Builder<'ctx>,
        val: IntValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        builder
            .build_int_to_ptr(val, self.ptr_type(), "cast_inttoptr")
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    fn cast_ptr_to_float64(
        &self,
        builder: &Builder<'ctx>,
        val: PointerValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i64_ty = self.context.i64_type();
        let addr_int = builder
            .build_ptr_to_int(val, i64_ty, "ptr_addr")
            .map_err(|e| e.to_string())?;
        builder
            .build_unsigned_int_to_float(addr_int, self.context.f64_type(), "cast_ptr_to_f64")
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    fn cast_float64_to_ptr(
        &self,
        builder: &Builder<'ctx>,
        val: inkwell::values::FloatValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i64_ty = self.context.i64_type();
        let addr_int = builder
            .build_float_to_unsigned_int(val, i64_ty, "f64_to_addr")
            .map_err(|e| e.to_string())?;
        builder
            .build_int_to_ptr(addr_int, self.ptr_type(), "cast_f64_to_ptr")
            .map(Into::into)
            .map_err(|e| e.to_string())
    }

    fn cast_to_string(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        val: BasicValueEnum<'ctx>,
        src_ty: &Type,
        region_ptr: PointerValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let (fn_name, arg): (&str, BasicMetadataValueEnum<'ctx>) = match src_ty {
            Type::Char => ("yukino_rt_char_to_string", val.into()),
            Type::Int8 | Type::Int16 | Type::Int32 => {
                let extended = builder
                    .build_int_s_extend(val.into_int_value(), self.context.i64_type(), "ext_i64")
                    .map_err(|e| e.to_string())?;
                ("yukino_rt_int64_to_string", extended.into())
            }
            Type::Int64 => ("yukino_rt_int64_to_string", val.into()),
            Type::Uint8 | Type::Uint16 | Type::Uint32 => {
                let extended = builder
                    .build_int_z_extend(val.into_int_value(), self.context.i64_type(), "ext_u64")
                    .map_err(|e| e.to_string())?;
                ("yukino_rt_uint64_to_string", extended.into())
            }
            Type::Uint64 => ("yukino_rt_uint64_to_string", val.into()),
            Type::Float32 => {
                let extended = builder
                    .build_float_ext(val.into_float_value(), self.context.f64_type(), "ext_f64")
                    .map_err(|e| e.to_string())?;
                ("yukino_rt_float64_to_string", extended.into())
            }
            Type::Float64 => ("yukino_rt_float64_to_string", val.into()),
            Type::Ptr(_) => ("yukino_rt_ptr_to_string", val.into()),
            Type::Bool => ("yukino_rt_bool_to_string", val.into()),
            _ => return Err(format!("cannot convert `{src_ty}` to string")),
        };

        let helper_fn = module
            .get_function(fn_name)
            .ok_or_else(|| format!("{fn_name} not found"))?;
        let str_ty = self.string_type();
        let out_slot = builder
            .build_alloca(str_ty, "sret_slot")
            .map_err(|e| e.to_string())?;
        builder
            .build_call(helper_fn, &[out_slot.into(), region_ptr.into(), arg], "")
            .map_err(|e| e.to_string())?;
        let result = builder
            .build_load(str_ty, out_slot, "s_conv")
            .map_err(|e| e.to_string())?;
        Ok(result)
    }

    fn cast_from_string(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        str_val: StructValue<'ctx>,
        target_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let buf_ptr = builder
            .build_extract_value(str_val, 0, "s_buf")
            .map_err(|e| e.to_string())?;
        let len_val = builder
            .build_extract_value(str_val, 1, "s_len")
            .map_err(|e| e.to_string())?;

        let (fn_name, needs_narrowing): (&str, bool) = match target_ty {
            Type::Char => ("yukino_rt_string_to_char", false),
            Type::Int8 | Type::Int16 | Type::Int32 => ("yukino_rt_string_to_int64", true),
            Type::Int64 => ("yukino_rt_string_to_int64", false),
            Type::Uint8 | Type::Uint16 | Type::Uint32 => ("yukino_rt_string_to_uint64", true),
            Type::Uint64 => ("yukino_rt_string_to_uint64", false),
            Type::Float32 | Type::Float64 => ("yukino_rt_string_to_float64", false),
            Type::Ptr(_) => ("yukino_rt_string_to_ptr", false),
            Type::Bool => ("yukino_rt_string_to_bool", false),
            _ => return Err(format!("cannot convert string to `{target_ty}`")),
        };

        let helper_fn = module
            .get_function(fn_name)
            .ok_or_else(|| format!("{fn_name} not found"))?;
        let call = builder
            .build_call(helper_fn, &[buf_ptr.into(), len_val.into()], "parsed_val")
            .map_err(|e| e.to_string())?;
        let res = call
            .try_as_basic_value()
            .basic()
            .ok_or("helper returned void")?;

        if needs_narrowing && res.is_int_value() {
            let bits = target_ty.bit_width().unwrap_or(32);
            let int_ty = self.get_int_type(bits);
            builder
                .build_int_truncate(res.into_int_value(), int_ty, "parsed_trunc")
                .map(Into::into)
                .map_err(|e| e.to_string())
        } else if *target_ty == Type::Float32 && res.is_float_value() {
            builder
                .build_float_trunc(
                    res.into_float_value(),
                    self.context.f32_type(),
                    "parsed_f32",
                )
                .map(Into::into)
                .map_err(|e| e.to_string())
        } 

        else { Ok(res) }
    }

    fn cast_bytes_to_string(
        &self,
        builder: &Builder<'ctx>,
        bytes_val: StructValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let buf_ptr = builder
            .build_extract_value(bytes_val, 0, "b_buf")
            .map_err(|e| e.to_string())?;
        let len_val = builder
            .build_extract_value(bytes_val, 1, "b_len")
            .map_err(|e| e.to_string())?;

        let str_ty = self.string_type();
        let s0 = str_ty.const_zero();
        let s1 = builder
            .build_insert_value(s0, buf_ptr, 0, "s_ins_buf")
            .map_err(|e| e.to_string())?;
        let s2 = builder
            .build_insert_value(s1, len_val, 1, "s_ins_len")
            .map_err(|e| e.to_string())?;
        Ok(s2.into_struct_value().into())
    }

    fn cast_string_to_bytes(
        &self,
        builder: &Builder<'ctx>,
        str_val: StructValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let buf_ptr = builder
            .build_extract_value(str_val, 0, "s_buf")
            .map_err(|e| e.to_string())?;
        let len_val = builder
            .build_extract_value(str_val, 1, "s_len")
            .map_err(|e| e.to_string())?;

        let slice_ty = self.slice_type();
        let b0 = slice_ty.const_zero();
        let b1 = builder
            .build_insert_value(b0, buf_ptr, 0, "b_ins_buf")
            .map_err(|e| e.to_string())?;
        let b2 = builder
            .build_insert_value(b1, len_val, 1, "b_ins_len")
            .map_err(|e| e.to_string())?;
        let b3 = builder
            .build_insert_value(b2, len_val, 2, "b_ins_cap")
            .map_err(|e| e.to_string())?;
        Ok(b3.into_struct_value().into())
    }

    fn cast_string_to_chars(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        str_val: StructValue<'ctx>,
        region_ptr: PointerValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let buf_ptr = builder
            .build_extract_value(str_val, 0, "s_buf")
            .map_err(|e| e.to_string())?;
        let len_val = builder
            .build_extract_value(str_val, 1, "s_len")
            .map_err(|e| e.to_string())?;

        let helper_fn = module
            .get_function("yukino_rt_string_to_chars")
            .ok_or_else(|| "yukino_rt_string_to_chars not found".to_string())?;
        let slice_ty = self.slice_type();
        let out_slot = builder
            .build_alloca(slice_ty, "sret_slot")
            .map_err(|e| e.to_string())?;
        builder
            .build_call(
                helper_fn,
                &[out_slot.into(), region_ptr.into(), buf_ptr.into(), len_val.into()],
                "",
            )
            .map_err(|e| e.to_string())?;
        builder
            .build_load(slice_ty, out_slot, "chars_slice")
            .map_err(|e| e.to_string())
    }

    fn cast_chars_to_string(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        chars_val: StructValue<'ctx>,
        region_ptr: PointerValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let buf_ptr = builder
            .build_extract_value(chars_val, 0, "c_buf")
            .map_err(|e| e.to_string())?;
        let len_val = builder
            .build_extract_value(chars_val, 1, "c_len")
            .map_err(|e| e.to_string())?;

        let helper_fn = module
            .get_function("yukino_rt_chars_to_string")
            .ok_or_else(|| "yukino_rt_chars_to_string not found".to_string())?;
        let str_ty = self.string_type();
        let out_slot = builder
            .build_alloca(str_ty, "sret_slot")
            .map_err(|e| e.to_string())?;
        builder
            .build_call(
                helper_fn,
                &[out_slot.into(), region_ptr.into(), buf_ptr.into(), len_val.into()],
                "",
            )
            .map_err(|e| e.to_string())?;
        let result = builder
            .build_load(str_ty, out_slot, "s_conv")
            .map_err(|e| e.to_string())?;
        Ok(result)
    }

    fn cast_bool(
        &self,
        builder: &Builder<'ctx>,
        val: BasicValueEnum<'ctx>,
        src_ty: &Type,
        target_ty: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if *src_ty == Type::Bool && target_ty.is_integer() {
            let bits = target_ty.bit_width().unwrap_or(64);
            let int_ty = self.get_int_type(bits);
            return builder
                .build_int_z_extend(val.into_int_value(), int_ty, "bool_to_int")
                .map(Into::into)
                .map_err(|e| e.to_string());
        }

        if src_ty.is_integer() && *target_ty == Type::Bool {
            let int_val = val.into_int_value();
            let zero = int_val.get_type().const_zero();
            return builder
                .build_int_compare(IntPredicate::NE, int_val, zero, "int_to_bool")
                .map(Into::into)
                .map_err(|e| e.to_string());
        }

        if src_ty.is_float() && *target_ty == Type::Bool {
            let flt_val = val.into_float_value();
            let zero = flt_val.get_type().const_zero();
            return builder
                .build_float_compare(FloatPredicate::ONE, flt_val, zero, "flt_to_bool")
                .map(Into::into)
                .map_err(|e| e.to_string());
        }

        if matches!(src_ty, Type::Ptr(_)) && *target_ty == Type::Bool {
            let ptr_val = val.into_pointer_value();
            let null_p = self.ptr_type().const_null();
            return builder
                .build_int_compare(IntPredicate::NE, ptr_val, null_p, "ptr_to_bool")
                .map(Into::into)
                .map_err(|e| e.to_string());
        }

        Err(format!(
            "unsupported bool cast between `{src_ty}` and `{target_ty}`"
        ))
    }
}
