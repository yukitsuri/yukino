#![deny(unsafe_code)]

use crate::region::RegionManager;
use inkwell::AddressSpace;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::builder::{Builder, BuilderError};
use inkwell::context::Context;
use inkwell::module::{Linkage, Module as LlvmModule};
use inkwell::types::{BasicTypeEnum, FunctionType, StructType};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValueEnum, CallSiteValue, FunctionValue, InstructionValue,
    IntValue, PointerValue, StructValue,
};
use yukic_ast::attr::CAttributeNode;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CAbiCallingConv {
    #[default]
    Cdecl,
    Stdcall,
    Fastcall,
    Thiscall,
    Win64,
    Sysv64,
}

impl CAbiCallingConv {
    pub fn as_llvm_call_conv(self) -> u32 {
        match self {
            Self::Cdecl => 0,
            Self::Stdcall => 64,
            Self::Fastcall => 65,
            Self::Thiscall => 70,
            Self::Sysv64 => 78,
            Self::Win64 => 79,
        }
    }

    pub fn from_attr_name(name: &str) -> Option<Self> {
        match name {
            "cdecl" => Some(Self::Cdecl),
            "stdcall" => Some(Self::Stdcall),
            "fastcall" => Some(Self::Fastcall),
            "thiscall" => Some(Self::Thiscall),
            "win64" => Some(Self::Win64),
            "sysv64" => Some(Self::Sysv64),
            _ => None,
        }
    }
}

pub struct CAbiManager<'ctx> {
    context: &'ctx Context,
}

impl<'ctx> CAbiManager<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self { context }
    }

    pub fn parse_calling_convention(attrs: &[CAttributeNode]) -> u32 {
        for attr in attrs {
            if let Some(conv) = CAbiCallingConv::from_attr_name(&attr.name.name) {
                return conv.as_llvm_call_conv();
            }
        }
        CAbiCallingConv::Cdecl.as_llvm_call_conv()
    }

    pub fn configure_cfun(
        &self,
        fn_val: FunctionValue<'ctx>,
        attrs: &[CAttributeNode],
        is_public: bool,
        has_body: bool,
    ) -> Result<(), String> {
        let call_conv = Self::parse_calling_convention(attrs);
        fn_val.set_call_conventions(call_conv);

        let is_weak = attrs.iter().any(|a| a.name.name == "weak");
        self.apply_linkage_and_visibility(fn_val, is_public, has_body, is_weak);
        self.apply_function_attributes(fn_val, attrs);
        Ok(())
    }

    pub fn apply_linkage_and_visibility(
        &self,
        fn_val: FunctionValue<'ctx>,
        is_public: bool,
        has_body: bool,
        is_weak: bool,
    ) {
        if is_weak {
            let linkage = if has_body {
                Linkage::WeakAny
            } else {
                Linkage::ExternalWeak
            };
            fn_val.set_linkage(linkage);
            return;
        }

        if is_public {
            fn_val.set_linkage(Linkage::External);
            fn_val
                .as_global_value()
                .set_visibility(inkwell::GlobalVisibility::Default);
        } else if has_body {
            fn_val.set_linkage(Linkage::External);
            fn_val
                .as_global_value()
                .set_visibility(inkwell::GlobalVisibility::Hidden);
        } else {
            fn_val.set_linkage(Linkage::External);
            fn_val
                .as_global_value()
                .set_visibility(inkwell::GlobalVisibility::Default);
        }
    }

    pub fn apply_function_attributes(&self, fn_val: FunctionValue<'ctx>, attrs: &[CAttributeNode]) {
        for attr in attrs {
            self.apply_single_attribute(fn_val, attr);
        }
    }

    fn apply_single_attribute(&self, fn_val: FunctionValue<'ctx>, attr: &CAttributeNode) {
        match attr.name.name.as_str() {
            "noreturn" => self.add_fn_enum_attr(fn_val, "noreturn"),
            "cold" => self.add_fn_enum_attr(fn_val, "cold"),
            "hot" => self.add_fn_enum_attr(fn_val, "hot"),
            "inline_always" => self.add_fn_enum_attr(fn_val, "alwaysinline"),
            "inline_never" => self.add_fn_enum_attr(fn_val, "noinline"),
            "returns_nonnull" => self.add_ret_enum_attr(fn_val, "nonnull"),
            "nonnull" => self.apply_nonnull_param_attr(fn_val, attr),
            "section" => {
                if let Some(sec) = &attr.arg {
                    fn_val
                        .as_global_value()
                        .set_section(Some(&format!(".text.{sec}")));
                }
            }
            _ => {}
        }
    }

    fn add_fn_enum_attr(&self, fn_val: FunctionValue<'ctx>, name: &str) {
        let kind_id = Attribute::get_named_enum_kind_id(name);
        if kind_id != 0 {
            let attr = self.context.create_enum_attribute(kind_id, 0);
            fn_val.add_attribute(AttributeLoc::Function, attr);
        }
    }

    fn add_ret_enum_attr(&self, fn_val: FunctionValue<'ctx>, name: &str) {
        let kind_id = Attribute::get_named_enum_kind_id(name);
        if kind_id != 0 {
            let attr = self.context.create_enum_attribute(kind_id, 0);
            fn_val.add_attribute(AttributeLoc::Return, attr);
        }
    }

    fn apply_nonnull_param_attr(&self, fn_val: FunctionValue<'ctx>, attr: &CAttributeNode) {
        let kind_id = Attribute::get_named_enum_kind_id("nonnull");
        if kind_id == 0 {
            return;
        }
        let attr_enum = self.context.create_enum_attribute(kind_id, 0);

        if let Some(arg) = &attr.arg {
            for part in arg.split_whitespace() {
                if let Ok(pos) = part.parse::<u32>() {
                    let total_params = fn_val.count_params();
                    if pos >= 1 && pos <= total_params {
                        fn_val.add_attribute(AttributeLoc::Param(pos - 1), attr_enum);
                    }
                }
            }
        } else {
            for (idx, param) in fn_val.get_params().iter().enumerate() {
                if param.is_pointer_value() {
                    let p_idx = u32::try_from(idx).unwrap_or(0);
                    fn_val.add_attribute(AttributeLoc::Param(p_idx), attr_enum);
                }
            }
        }
    }

    pub fn emit_cfun_call(
        &self,
        builder: &Builder<'ctx>,
        fn_val: FunctionValue<'ctx>,
        args: &[BasicMetadataValueEnum<'ctx>],
        call_conv: u32,
        name: &str,
    ) -> Result<CallSiteValue<'ctx>, BuilderError> {
        let call_site = builder.build_call(fn_val, args, name)?;
        call_site.set_call_convention(call_conv);
        Ok(call_site)
    }

    pub fn emit_indirect_cfun_call(
        &self,
        builder: &Builder<'ctx>,
        fn_type: FunctionType<'ctx>,
        fn_ptr: PointerValue<'ctx>,
        args: &[BasicMetadataValueEnum<'ctx>],
        call_conv: u32,
        name: &str,
    ) -> Result<CallSiteValue<'ctx>, BuilderError> {
        let call_site = builder.build_indirect_call(fn_type, fn_ptr, args, name)?;
        call_site.set_call_convention(call_conv);
        Ok(call_site)
    }

    pub fn as_fn_ptr(&self, fn_val: FunctionValue<'ctx>) -> PointerValue<'ctx> {
        fn_val.as_global_value().as_pointer_value()
    }

    pub fn emit_ptr_deref(
        &self,
        builder: &Builder<'ctx>,
        elem_type: BasicTypeEnum<'ctx>,
        ptr: PointerValue<'ctx>,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>, BuilderError> {
        builder.build_load(elem_type, ptr, name)
    }

    pub fn emit_ptr_deref_set(
        &self,
        builder: &Builder<'ctx>,
        ptr: PointerValue<'ctx>,
        val: BasicValueEnum<'ctx>,
    ) -> Result<InstructionValue<'ctx>, BuilderError> {
        builder.build_store(ptr, val)
    }

    pub fn emit_ptr_offset(
        &self,
        builder: &Builder<'ctx>,
        ptr: PointerValue<'ctx>,
        byte_offset: IntValue<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>, BuilderError> {
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let ptr_int = builder.build_ptr_to_int(ptr, i64_type, &format!("{name}_int"))?;
        let offset_int = builder.build_int_add(ptr_int, byte_offset, &format!("{name}_add"))?;
        builder.build_int_to_ptr(offset_int, ptr_type, name)
    }

    pub fn emit_ptr_is_null(
        &self,
        builder: &Builder<'ctx>,
        ptr: PointerValue<'ctx>,
        name: &str,
    ) -> Result<IntValue<'ctx>, BuilderError> {
        builder.build_is_null(ptr, name)
    }

    pub fn emit_ptr_to_int(
        &self,
        builder: &Builder<'ctx>,
        ptr: PointerValue<'ctx>,
        name: &str,
    ) -> Result<IntValue<'ctx>, BuilderError> {
        let i64_type = self.context.i64_type();
        builder.build_ptr_to_int(ptr, i64_type, name)
    }

    pub fn emit_int_to_ptr(
        &self,
        builder: &Builder<'ctx>,
        int_val: IntValue<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>, BuilderError> {
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        builder.build_int_to_ptr(int_val, ptr_type, name)
    }

    pub fn emit_null_ptr(&self) -> PointerValue<'ctx> {
        self.context.ptr_type(AddressSpace::default()).const_null()
    }

    pub fn emit_cstruct_alloca(
        &self,
        builder: &Builder<'ctx>,
        struct_type: StructType<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>, BuilderError> {
        builder.build_alloca(struct_type, name)
    }

    pub fn emit_cstruct_field_gep(
        &self,
        builder: &Builder<'ctx>,
        struct_type: StructType<'ctx>,
        ptr: PointerValue<'ctx>,
        field_idx: u32,
        name: &str,
    ) -> Result<PointerValue<'ctx>, BuilderError> {
        builder.build_struct_gep(struct_type, ptr, field_idx, name)
    }

    pub fn emit_cstruct_load_field(
        &self,
        builder: &Builder<'ctx>,
        struct_type: StructType<'ctx>,
        ptr: PointerValue<'ctx>,
        field_idx: u32,
        field_type: BasicTypeEnum<'ctx>,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>, BuilderError> {
        let field_gep =
            builder.build_struct_gep(struct_type, ptr, field_idx, &format!("{name}_gep"))?;
        builder.build_load(field_type, field_gep, name)
    }

    pub fn emit_cstruct_store_field(
        &self,
        builder: &Builder<'ctx>,
        struct_type: StructType<'ctx>,
        ptr: PointerValue<'ctx>,
        field_idx: u32,
        val: BasicValueEnum<'ctx>,
    ) -> Result<InstructionValue<'ctx>, BuilderError> {
        let field_gep = builder.build_struct_gep(struct_type, ptr, field_idx, "f_gep")?;
        builder.build_store(field_gep, val)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn emit_to_cstr(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        current_fn: FunctionValue<'ctx>,
        region: &RegionManager<'ctx>,
        region_ptr: PointerValue<'ctx>,
        str_val: StructValue<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        let i64_type = self.context.i64_type();
        let i8_type = self.context.i8_type();

        let buf_ptr = builder
            .build_extract_value(str_val, 0, "s_buf")
            .map_err(|e| e.to_string())?
            .into_pointer_value();
        let len_val = builder
            .build_extract_value(str_val, 1, "s_len")
            .map_err(|e| e.to_string())?
            .into_int_value();

        // alloc len + 1 bytes in curr reg
        let one = i64_type.const_int(1, false);
        let alloc_size = builder
            .build_int_add(len_val, one, "cstr_alloc_size")
            .map_err(|e| e.to_string())?;

        let dest_ptr = region
            .emit_dynamic_fast_path_alloc(module, builder, current_fn, region_ptr, alloc_size, 1)?;

        // copy bytes from str buf to allocd cstr buf
        builder
            .build_memcpy(dest_ptr, 1, buf_ptr, 1, len_val)
            .map_err(|e| e.to_string())?;

        let dest_int = builder
            .build_ptr_to_int(dest_ptr, i64_type, "dest_int")
            .map_err(|e| e.to_string())?;
        let null_addr = builder
            .build_int_add(dest_int, len_val, "null_addr")
            .map_err(|e| e.to_string())?;
        let null_ptr = builder
            .build_int_to_ptr(
                null_addr,
                self.context.ptr_type(AddressSpace::default()),
                "null_ptr",
            )
            .map_err(|e| e.to_string())?;

        builder
            .build_store(null_ptr, i8_type.const_int(0, false))
            .map_err(|e| e.to_string())?;

        let final_ptr = builder
            .build_pointer_cast(
                dest_ptr,
                self.context.ptr_type(AddressSpace::default()),
                name,
            )
            .map_err(|e| e.to_string())?;

        Ok(final_ptr)
    }

    pub fn emit_from_cstr(
        &self,
        module: &LlvmModule<'ctx>,
        builder: &Builder<'ctx>,
        cstr_ptr: PointerValue<'ctx>,
        name: &str,
    ) -> Result<StructValue<'ctx>, String> {
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());

        // decl strlen if !present: decl i64 @strlen(ptr)
        let strlen_fn = module.get_function("strlen").unwrap_or_else(|| {
            let fn_type = i64_type.fn_type(&[ptr_type.into()], false);
            module.add_function("strlen", fn_type, None)
        });

        let call_res = builder
            .build_call(strlen_fn, &[cstr_ptr.into()], "c_len")
            .map_err(|e| e.to_string())?;
        let len_val = call_res
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "strlen returned void".to_string())?
            .into_int_value();

        self.emit_from_cstr_n(builder, cstr_ptr, len_val, name)
            .map_err(|e| e.to_string())
    }

    pub fn emit_from_cstr_n(
        &self,
        builder: &Builder<'ctx>,
        cstr_ptr: PointerValue<'ctx>,
        len: IntValue<'ctx>,
        name: &str,
    ) -> Result<StructValue<'ctx>, BuilderError> {
        let str_ty = self.context.struct_type(
            &[
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.i64_type().into(),
            ],
            false,
        );

        let s0 = str_ty.const_zero();
        let with_ptr = builder.build_insert_value(s0, cstr_ptr, 0, &format!("{name}_ptr"))?;
        let with_len = builder.build_insert_value(with_ptr, len, 1, name)?;
        Ok(with_len.into_struct_value())
    }
}
