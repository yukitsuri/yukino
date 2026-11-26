#![deny(unsafe_code)]

use crate::region::RegionManager;
use crate::registry::TypeRegistry;
use crate::types::TypeLowerer;
use inkwell::AddressSpace;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::builder::{Builder, BuilderError};
use inkwell::context::Context;
use inkwell::module::{Linkage, Module as LlvmModule};
use inkwell::targets::TargetData;
use inkwell::types::{
    AnyTypeEnum, BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType, StructType,
};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValueEnum, CallSiteValue, FunctionValue, InstructionValue,
    IntValue, PointerValue, StructValue,
};
use yukic_ast::attr::CAttributeNode;
use yukic_target::{Architecture, OperatingSystem, TargetTriple};
use yukic_types::Type;

#[derive(Clone, Debug)]
pub enum CAbiArgMode<'ctx> {
    Direct(BasicTypeEnum<'ctx>),
    Coerced {
        parts: Vec<BasicTypeEnum<'ctx>>,
        orig_ty: BasicTypeEnum<'ctx>,
    },
    IndirectPtr {
        pointee: BasicTypeEnum<'ctx>,
        align: u32,
        is_byval: bool,
    },
}

#[derive(Clone, Debug)]
pub enum CAbiRetMode<'ctx> {
    Void,
    Direct(BasicTypeEnum<'ctx>),
    Coerced {
        abi_ty: BasicTypeEnum<'ctx>,
        orig_ty: BasicTypeEnum<'ctx>,
    },
    Sret {
        pointee: BasicTypeEnum<'ctx>,
        align: u32,
    },
}

#[derive(Clone, Debug)]
pub struct CAbiSignature<'ctx> {
    pub fn_type: FunctionType<'ctx>,
    pub ret_mode: CAbiRetMode<'ctx>,
    pub arg_modes: Vec<CAbiArgMode<'ctx>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EightbyteClass {
    Empty,
    FloatOnly,
    IntOrMixed,
}

pub fn build_c_signature<'ctx>(
    context: &'ctx Context,
    target: TargetTriple,
    target_data: &TargetData,
    registry: &TypeRegistry,
    param_sem_tys: &[Type],
    ret_sem_ty: &Type,
    is_variadic: bool,
) -> Result<CAbiSignature<'ctx>, String> {
    let lowerer = TypeLowerer::new(context, target_data, target, registry);
    let ret_canon = registry.canonicalize_type(ret_sem_ty);
    let ret_mode = classify_c_ret(context, target, target_data, registry, &lowerer, &ret_canon)?;

    let mut llvm_params: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::new();
    if matches!(ret_mode, CAbiRetMode::Sret { .. }) {
        llvm_params.push(context.ptr_type(AddressSpace::default()).into());
    }

    let mut arg_modes = Vec::with_capacity(param_sem_tys.len());
    for raw_pty in param_sem_tys {
        let pty = registry.canonicalize_type(raw_pty);
        let mode = classify_c_arg(context, target, target_data, registry, &lowerer, &pty)?;
        match &mode {
            CAbiArgMode::Direct(t) => llvm_params.push((*t).into()),
            CAbiArgMode::Coerced { parts, .. } => {
                for p in parts {
                    llvm_params.push((*p).into());
                }
            }
            CAbiArgMode::IndirectPtr { .. } => {
                llvm_params.push(context.ptr_type(AddressSpace::default()).into());
            }
        }
        arg_modes.push(mode);
    }

    let fn_type = match &ret_mode {
        CAbiRetMode::Void | CAbiRetMode::Sret { .. } => {
            context.void_type().fn_type(&llvm_params, is_variadic)
        }
        CAbiRetMode::Direct(t) | CAbiRetMode::Coerced { abi_ty: t, .. } => {
            t.fn_type(&llvm_params, is_variadic)
        }
    };

    Ok(CAbiSignature {
        fn_type,
        ret_mode,
        arg_modes,
    })
}

pub fn offset_ptr_bytes<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    base: PointerValue<'ctx>,
    byte_offset: u64,
    name: &str,
) -> Result<PointerValue<'ctx>, String> {
    if byte_offset == 0 {
        return Ok(base);
    }
    let i64_ty = context.i64_type();
    let ptr_ty = context.ptr_type(AddressSpace::default());
    let addr = builder
        .build_ptr_to_int(base, i64_ty, "cabi_addr")
        .map_err(|e| e.to_string())?;
    let next = builder
        .build_int_add(addr, i64_ty.const_int(byte_offset, false), "cabi_next")
        .map_err(|e| e.to_string())?;
    builder
        .build_int_to_ptr(next, ptr_ty, name)
        .map_err(|e| e.to_string())
}

fn basic_to_any_type(ty: BasicTypeEnum<'_>) -> AnyTypeEnum<'_> {
    match ty {
        BasicTypeEnum::ArrayType(t) => t.into(),
        BasicTypeEnum::FloatType(t) => t.into(),
        BasicTypeEnum::IntType(t) => t.into(),
        BasicTypeEnum::PointerType(t) => t.into(),
        BasicTypeEnum::StructType(t) => t.into(),
        BasicTypeEnum::VectorType(t) => t.into(),
        BasicTypeEnum::ScalableVectorType(t) => t.into(),
    }
}

pub fn apply_c_signature_attrs<'ctx>(
    context: &'ctx Context,
    fn_val: FunctionValue<'ctx>,
    sig: &CAbiSignature<'ctx>,
) {
    let mut llvm_idx: u32 = 0;
    if let CAbiRetMode::Sret { pointee, align } = &sig.ret_mode {
        let sret_kind = Attribute::get_named_enum_kind_id("sret");
        if sret_kind != 0 {
            let attr = context.create_type_attribute(sret_kind, basic_to_any_type(*pointee));
            fn_val.add_attribute(AttributeLoc::Param(0), attr);
        }
        let noalias_kind = Attribute::get_named_enum_kind_id("noalias");
        if noalias_kind != 0 {
            fn_val.add_attribute(AttributeLoc::Param(0), context.create_enum_attribute(noalias_kind, 0));
        }
        let align_kind = Attribute::get_named_enum_kind_id("align");
        if align_kind != 0 && *align > 0 {
            fn_val.add_attribute(
                AttributeLoc::Param(0),
                context.create_enum_attribute(align_kind, u64::from(*align)),
            );
        }
        llvm_idx = 1;
    }

    for mode in &sig.arg_modes {
        match mode {
            CAbiArgMode::Direct(_) => llvm_idx += 1,
            CAbiArgMode::Coerced { parts, .. } => {
                llvm_idx += u32::try_from(parts.len()).unwrap_or(0);
            }
            CAbiArgMode::IndirectPtr {
                pointee,
                align,
                is_byval,
            } => {
                if *is_byval {
                    let byval_kind = Attribute::get_named_enum_kind_id("byval");
                    if byval_kind != 0 {
                        let attr =
                            context.create_type_attribute(byval_kind, basic_to_any_type(*pointee));
                        fn_val.add_attribute(AttributeLoc::Param(llvm_idx), attr);
                    }
                }
                let align_kind = Attribute::get_named_enum_kind_id("align");
                if align_kind != 0 && *align > 0 {
                    fn_val.add_attribute(
                        AttributeLoc::Param(llvm_idx),
                        context.create_enum_attribute(align_kind, u64::from(*align)),
                    );
                }
                llvm_idx += 1;
            }
        }
    }
}

pub fn apply_c_callsite_attrs<'ctx>(
    context: &'ctx Context,
    call: CallSiteValue<'ctx>,
    sig: &CAbiSignature<'ctx>,
) {
    let mut llvm_idx: u32 = 0;
    if let CAbiRetMode::Sret { pointee, .. } = &sig.ret_mode {
        let sret_kind = Attribute::get_named_enum_kind_id("sret");
        if sret_kind != 0 {
            call.add_attribute(
                AttributeLoc::Param(0),
                context.create_type_attribute(sret_kind, basic_to_any_type(*pointee)),
            );
        }
        llvm_idx = 1;
    }
    for mode in &sig.arg_modes {
        match mode {
            CAbiArgMode::Direct(_) => llvm_idx += 1,
            CAbiArgMode::Coerced { parts, .. } => {
                llvm_idx += u32::try_from(parts.len()).unwrap_or(0);
            }
            CAbiArgMode::IndirectPtr {
                pointee, is_byval, ..
            } => {
                if *is_byval {
                    let byval_kind = Attribute::get_named_enum_kind_id("byval");
                    if byval_kind != 0 {
                        call.add_attribute(
                            AttributeLoc::Param(llvm_idx),
                            context.create_type_attribute(byval_kind, basic_to_any_type(*pointee)),
                        );
                    }
                }
                llvm_idx += 1;
            }
        }
    }
}

fn c_aggregate_name<'a>(ty: &'a Type, registry: &TypeRegistry) -> Option<&'a str> {
    match ty {
        Type::Cstruct(n) => Some(n.as_str()),
        Type::Struct(n) if registry.cstructs.contains_key(n) => Some(n.as_str()),
        _ => None,
    }
}

fn int_type_for_byte_len<'ctx>(context: &'ctx Context, len: u64) -> BasicTypeEnum<'ctx> {
    match len {
        0 | 1 => context.i8_type().into(),
        2 => context.i16_type().into(),
        3..=4 => context.i32_type().into(),
        _ => context.i64_type().into(),
    }
}

fn collect_byte_classes(
    ty: &Type,
    offset: u64,
    target_data: &TargetData,
    lowerer: &TypeLowerer<'_, '_>,
    registry: &TypeRegistry,
    classes: &mut Vec<EightbyteClass>,
) -> Result<(), String> {
    if let Some(name) = c_aggregate_name(ty, registry)
        && let Some(layout) = registry.cstructs.get(name).cloned()
    {
        if layout.is_union {
            for (_, fty) in &layout.fields {
                collect_byte_classes(fty, offset, target_data, lowerer, registry, classes)?;
            }
            return Ok(());
        }
        let st = lowerer.lower_named_cstruct(name)?;
        let st_layout = target_data;
        for (idx, (_, fty)) in layout.fields.iter().enumerate() {
            let f_off = st_layout
                .offset_of_element(&st, idx as u32)
                .unwrap_or(0);
            collect_byte_classes(fty, offset + f_off, target_data, lowerer, registry, classes)?;
        }
        return Ok(());
    }
    if let Type::FixedArray(elem, len) = ty {
        let elem_llvm = lowerer.lower_type(elem)?;
        let step = target_data.get_abi_size(&elem_llvm).max(1);
        for i in 0..*len {
            let f_off = offset + (i as u64) * step;
            collect_byte_classes(elem, f_off, target_data, lowerer, registry, classes)?;
        }
        return Ok(());
    }
    let llvm_ty = lowerer.lower_type(ty)?;
    let size = target_data.get_store_size(&llvm_ty).max(1);
    let cls = match ty {
        Type::Float32 | Type::Float64 => EightbyteClass::FloatOnly,
        _ => EightbyteClass::IntOrMixed,
    };
    let start_eb = usize::try_from(offset / 8).unwrap_or(0);
    let end_eb = usize::try_from((offset + size - 1) / 8).unwrap_or(start_eb);
    for eb in start_eb..=end_eb {
        if let Some(slot) = classes.get_mut(eb) {
            *slot = match (*slot, cls) {
                (EightbyteClass::Empty, c) | (c, EightbyteClass::Empty) => c,
                (EightbyteClass::IntOrMixed, _) | (_, EightbyteClass::IntOrMixed) => {
                    EightbyteClass::IntOrMixed
                }
                (EightbyteClass::FloatOnly, EightbyteClass::FloatOnly) => EightbyteClass::FloatOnly,
            };
        }
    }
    Ok(())
}

fn classify_sysv_x86_64_chunks<'ctx>(
    context: &'ctx Context,
    target_data: &TargetData,
    lowerer: &TypeLowerer<'_, 'ctx>,
    registry: &TypeRegistry,
    ty: &Type,
    size: u64,
) -> Result<Vec<BasicTypeEnum<'ctx>>, String> {
    if size == 0 {
        return Ok(Vec::new());
    }
    let num_chunks = usize::try_from(size.div_ceil(8)).unwrap_or(1);
    let mut classes = vec![EightbyteClass::Empty; num_chunks];
    collect_byte_classes(ty, 0, target_data, lowerer, registry, &mut classes)?;

    let mut parts = Vec::with_capacity(num_chunks);
    for (idx, cls) in classes.iter().enumerate() {
        let start = (idx as u64) * 8;
        let rem = size.saturating_sub(start).min(8);
        let chunk_ty: BasicTypeEnum<'ctx> = match cls {
            EightbyteClass::FloatOnly => {
                if rem > 4 {
                    // Two floats (`<2 x float>`) or one `double`: check whether float32 vector fits.
                    context.f64_type().into()
                } else {
                    context.f32_type().into()
                }
            }
            EightbyteClass::IntOrMixed | EightbyteClass::Empty => int_type_for_byte_len(context, rem),
        };
        parts.push(chunk_ty);
    }
    // Check if an all-f32 eightbyte actually holds f32s (uses <2 x float> on x86_64).
    if let Some(name) = c_aggregate_name(ty, registry)
        && let Some(layout) = registry.cstructs.get(name)
        && !layout.is_union
    {
        let st = lowerer.lower_named_cstruct(name)?;
        for (eb_idx, part) in parts.iter_mut().enumerate() {
            if *part == BasicTypeEnum::FloatType(context.f64_type()) {
                let eb_start = (eb_idx as u64) * 8;
                let eb_end = eb_start + 8;
                let mut all_f32 = false;
                for (f_idx, (_, fty)) in layout.fields.iter().enumerate() {
                    let f_off = target_data.offset_of_element(&st, f_idx as u32).unwrap_or(0);
                    if f_off >= eb_start && f_off < eb_end {
                        if matches!(fty, Type::Float32) {
                            all_f32 = true;
                        } else if matches!(fty, Type::Float64) {
                            all_f32 = false;
                            break;
                        }
                    }
                }
                if all_f32 {
                    *part = context.f32_type().vec_type(2).into();
                }
            }
        }
    }
    Ok(parts)
}

fn classify_c_arg<'ctx>(
    context: &'ctx Context,
    target: TargetTriple,
    target_data: &TargetData,
    registry: &TypeRegistry,
    lowerer: &TypeLowerer<'_, 'ctx>,
    ty: &Type,
) -> Result<CAbiArgMode<'ctx>, String> {
    if let Some(name) = c_aggregate_name(ty, registry) {
        let orig_ty: BasicTypeEnum<'ctx> = lowerer.lower_named_cstruct(name)?.into();
        let size = target_data.get_store_size(&orig_ty);
        let align = target_data.get_abi_alignment(&orig_ty).max(1);
        let spec = target.spec();

        if spec.os == OperatingSystem::Windows {
            if matches!(size, 1 | 2 | 4 | 8) {
                return Ok(CAbiArgMode::Coerced {
                    parts: vec![int_type_for_byte_len(context, size)],
                    orig_ty,
                });
            }
            return Ok(CAbiArgMode::IndirectPtr {
                pointee: orig_ty,
                align,
                is_byval: false,
            });
        }

        if spec.arch == Architecture::Aarch64 {
            if size == 0 {
                return Ok(CAbiArgMode::Coerced {
                    parts: Vec::new(),
                    orig_ty,
                });
            }
            if size <= 16 {
                let mut parts = Vec::new();
                parts.push(int_type_for_byte_len(context, size.min(8)));
                if size > 8 {
                    parts.push(int_type_for_byte_len(context, size - 8));
                }
                return Ok(CAbiArgMode::Coerced { parts, orig_ty });
            }
            return Ok(CAbiArgMode::IndirectPtr {
                pointee: orig_ty,
                align,
                is_byval: false,
            });
        }

        // x86_64 SysV ABI
        if size <= 16 {
            let parts = classify_sysv_x86_64_chunks(context, target_data, lowerer, registry, ty, size)?;
            return Ok(CAbiArgMode::Coerced { parts, orig_ty });
        }
        return Ok(CAbiArgMode::IndirectPtr {
            pointee: orig_ty,
            align,
            is_byval: true,
        });
    }

    Ok(CAbiArgMode::Direct(lowerer.lower_type(ty)?))
}

fn classify_c_ret<'ctx>(
    context: &'ctx Context,
    target: TargetTriple,
    target_data: &TargetData,
    registry: &TypeRegistry,
    lowerer: &TypeLowerer<'_, 'ctx>,
    ty: &Type,
) -> Result<CAbiRetMode<'ctx>, String> {
    if matches!(ty, Type::Void | Type::Never) {
        return Ok(CAbiRetMode::Void);
    }
    if let Some(name) = c_aggregate_name(ty, registry) {
        let orig_ty: BasicTypeEnum<'ctx> = lowerer.lower_named_cstruct(name)?.into();
        let size = target_data.get_store_size(&orig_ty);
        let align = target_data.get_abi_alignment(&orig_ty).max(1);
        let spec = target.spec();

        if spec.os == OperatingSystem::Windows {
            if matches!(size, 1 | 2 | 4 | 8) {
                return Ok(CAbiRetMode::Coerced {
                    abi_ty: int_type_for_byte_len(context, size),
                    orig_ty,
                });
            }
            return Ok(CAbiRetMode::Sret {
                pointee: orig_ty,
                align,
            });
        }

        if spec.arch == Architecture::Aarch64 {
            if size == 0 {
                return Ok(CAbiRetMode::Void);
            }
            if size <= 16 {
                let abi_ty: BasicTypeEnum<'ctx> = if size <= 8 {
                    int_type_for_byte_len(context, size)
                } else {
                    context
                        .struct_type(
                            &[
                                context.i64_type().into(),
                                int_type_for_byte_len(context, size - 8),
                            ],
                            false,
                        )
                        .into()
                };
                return Ok(CAbiRetMode::Coerced { abi_ty, orig_ty });
            }
            return Ok(CAbiRetMode::Sret {
                pointee: orig_ty,
                align,
            });
        }

        if size == 0 {
            return Ok(CAbiRetMode::Void);
        }
        if size <= 16 {
            let parts =
                classify_sysv_x86_64_chunks(context, target_data, lowerer, registry, ty, size)?;
            let abi_ty = if parts.len() == 1 {
                parts[0]
            } else {
                context.struct_type(&parts, false).into()
            };
            return Ok(CAbiRetMode::Coerced { abi_ty, orig_ty });
        }
        return Ok(CAbiRetMode::Sret {
            pointee: orig_ty,
            align,
        });
    }

    match lowerer.lower_return_type(ty)? {
        Some(t) => Ok(CAbiRetMode::Direct(t)),
        None => Ok(CAbiRetMode::Void),
    }
}

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
