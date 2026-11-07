#![deny(unsafe_code)]

use inkwell::AddressSpace;
use inkwell::builder::{Builder, BuilderError};
use inkwell::context::Context;
use inkwell::module::Module as LlvmModule;
use inkwell::types::{BasicTypeEnum, FunctionType, PointerType, StructType};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValueEnum, CallSiteValue, FunctionValue, PointerValue, StructValue,
};
use yukic_types::Type;

use crate::region::RegionManager;
use crate::types::TypeLowerer;

#[derive(Clone, Debug, PartialEq)]
pub struct CapturedVar {
    pub name: String,
    pub ty: Type,
}

impl CapturedVar {
    pub fn new(name: impl Into<String>, ty: Type) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClosureEnvLayout<'ctx> {
    pub struct_type: StructType<'ctx>,
    pub field_names: Vec<String>,
    pub field_types: Vec<BasicTypeEnum<'ctx>>,
    pub size_bytes: u64,
    pub align_bytes: u64,
}

#[derive(Clone, Copy)]
pub struct ClosureManager<'ctx> {
    context: &'ctx Context,
}

impl<'ctx> ClosureManager<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self { context }
    }

    pub fn ptr_type(&self) -> PointerType<'ctx> {
        self.context.ptr_type(AddressSpace::default())
    }

    pub fn closure_fat_type(&self) -> StructType<'ctx> {
        self.context
            .struct_type(&[self.ptr_type().into(), self.ptr_type().into()], false)
    }

    pub fn create_env_layout(
        &self,
        name: &str,
        captures: &[CapturedVar],
        lowerer: &TypeLowerer<'_, 'ctx>,
    ) -> Result<ClosureEnvLayout<'ctx>, String> {
        let mut field_names = Vec::with_capacity(captures.len());
        let mut field_types = Vec::with_capacity(captures.len());

        for cap in captures {
            field_names.push(cap.name.clone());
            let llvm_t = lowerer.lower_type(&cap.ty)?;
            field_types.push(llvm_t);
        }

        let struct_type = self
            .context
            .opaque_struct_type(&format!("struct.env_{name}"));
        struct_type.set_body(&field_types, false);

        let size_bytes = lowerer.target_data.get_bit_size(&struct_type) / 8;
        let align_bytes = u64::from(lowerer.target_data.get_abi_alignment(&struct_type));
        let safe_size = if size_bytes == 0 { 8 } else { size_bytes };
        let safe_align = if align_bytes == 0 { 8 } else { align_bytes };

        Ok(ClosureEnvLayout {
            struct_type,
            field_names,
            field_types,
            size_bytes: safe_size,
            align_bytes: safe_align,
        })
    }

    #[allow(clippy::too_many_arguments, clippy::cast_possible_truncation)]
    pub fn allocate_and_init_env(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        current_fn: FunctionValue<'ctx>,
        region: &RegionManager<'ctx>,
        region_ptr: PointerValue<'ctx>,
        layout: &ClosureEnvLayout<'ctx>,
        values: &[BasicValueEnum<'ctx>],
    ) -> Result<PointerValue<'ctx>, String> {
        if values.len() != layout.field_types.len() {
            return Err(format!(
                "capture count mismatch: expected {}, got {}",
                layout.field_types.len(),
                values.len()
            ));
        }

        let alloc_ptr = region.emit_fast_path_alloc(
            module,
            builder,
            current_fn,
            region_ptr,
            layout.size_bytes,
            layout.align_bytes,
        )?;

        for (idx, val) in values.iter().enumerate() {
            let field_gep = builder
                .build_struct_gep(
                    layout.struct_type,
                    alloc_ptr,
                    idx as u32,
                    &format!("cap_gep_{idx}"),
                )
                .map_err(|e| e.to_string())?;
            builder
                .build_store(field_gep, *val)
                .map_err(|e| e.to_string())?;
        }

        Ok(alloc_ptr)
    }

    pub fn make_closure_fat_pointer(
        &self,
        builder: &Builder<'ctx>,
        fn_ptr: PointerValue<'ctx>,
        env_ptr: PointerValue<'ctx>,
        name: &str,
    ) -> Result<StructValue<'ctx>, String> {
        let fat_ty = self.closure_fat_type();
        let undef = fat_ty.get_undef();
        let with_fn = builder
            .build_insert_value(undef, fn_ptr, 0, &format!("{name}_fn"))
            .map_err(|e| e.to_string())?
            .into_struct_value();
        let with_env = builder
            .build_insert_value(with_fn, env_ptr, 1, &format!("{name}_fat"))
            .map_err(|e| e.to_string())?
            .into_struct_value();
        Ok(with_env)
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn unpack_environment(
        &self,
        builder: &Builder<'ctx>,
        env_ptr: PointerValue<'ctx>,
        layout: &ClosureEnvLayout<'ctx>,
    ) -> Result<Vec<(String, BasicValueEnum<'ctx>)>, String> {
        let mut unpacked = Vec::with_capacity(layout.field_names.len());

        for (idx, (name, ty)) in layout
            .field_names
            .iter()
            .zip(layout.field_types.iter())
            .enumerate()
        {
            let field_gep = builder
                .build_struct_gep(
                    layout.struct_type,
                    env_ptr,
                    idx as u32,
                    &format!("{name}_gep"),
                )
                .map_err(|e| e.to_string())?;
            let val = builder
                .build_load(*ty, field_gep, name)
                .map_err(|e| e.to_string())?;
            unpacked.push((name.clone(), val));
        }

        Ok(unpacked)
    }

    pub fn emit_closure_call(
        &self,
        builder: &Builder<'ctx>,
        closure_fat: StructValue<'ctx>,
        fn_type: FunctionType<'ctx>,
        args: &[BasicMetadataValueEnum<'ctx>],
        dest_region: Option<PointerValue<'ctx>>,
        name: &str,
    ) -> Result<CallSiteValue<'ctx>, BuilderError> {
        let fn_ptr = builder
            .build_extract_value(closure_fat, 0, &format!("{name}_fn_ptr"))?
            .into_pointer_value();
        let env_ptr = builder
            .build_extract_value(closure_fat, 1, &format!("{name}_env_ptr"))?
            .into_pointer_value();

        let mut actual_args: Vec<BasicMetadataValueEnum<'ctx>> = Vec::with_capacity(args.len() + 2);
        if let Some(dest) = dest_region {
            actual_args.push(dest.into());
        }
        actual_args.push(env_ptr.into());
        actual_args.extend_from_slice(args);

        let call_site = builder.build_indirect_call(fn_type, fn_ptr, &actual_args, name)?;
        call_site.set_call_convention(inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32);
        Ok(call_site)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn emit_closure_invoke(
        &self,
        builder: &Builder<'ctx>,
        closure_fat: StructValue<'ctx>,
        fn_type: FunctionType<'ctx>,
        args: &[BasicValueEnum<'ctx>],
        then_block: inkwell::basic_block::BasicBlock<'ctx>,
        unwind_block: inkwell::basic_block::BasicBlock<'ctx>,
        dest_region: Option<PointerValue<'ctx>>,
        name: &str,
    ) -> Result<CallSiteValue<'ctx>, BuilderError> {
        let fn_ptr = builder
            .build_extract_value(closure_fat, 0, &format!("{name}_fn_ptr"))?
            .into_pointer_value();
        let env_ptr = builder
            .build_extract_value(closure_fat, 1, &format!("{name}_env_ptr"))?
            .into_pointer_value();

        let mut actual_args: Vec<BasicValueEnum<'ctx>> = Vec::with_capacity(args.len() + 2);
        if let Some(dest) = dest_region {
            actual_args.push(dest.into());
        }
        actual_args.push(env_ptr.into());
        actual_args.extend_from_slice(args);

        let call_site = builder.build_indirect_invoke(
            fn_type,
            fn_ptr,
            &actual_args,
            then_block,
            unwind_block,
            name,
        )?;
        call_site.set_call_convention(inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32);
        Ok(call_site)
    }
}
