#![deny(unsafe_code)]

use inkwell::AddressSpace;
use inkwell::context::Context;
use inkwell::targets::TargetData;
use inkwell::types::{BasicTypeEnum, IntType, PointerType, StructType};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use yukic_target::{DataModel, TargetTriple};
use yukic_types::{CIntKind, Type};

use crate::registry::TypeRegistry;

pub struct TypeLowerer<'a, 'ctx> {
    pub context: &'ctx Context,
    pub target_data: &'a TargetData,
    pub target: TargetTriple,
    pub registry: &'a TypeRegistry,
    cached_structs: RefCell<HashMap<String, StructType<'ctx>>>,
    cached_cstructs: RefCell<HashMap<String, StructType<'ctx>>>,
    cached_enums: RefCell<HashMap<String, StructType<'ctx>>>,
}

impl<'a, 'ctx> TypeLowerer<'a, 'ctx> {
    pub fn new(
        context: &'ctx Context,
        target_data: &'a TargetData,
        target: TargetTriple,
        registry: &'a TypeRegistry,
    ) -> Self {
        Self {
            context,
            target_data,
            target,
            registry,
            cached_structs: RefCell::new(HashMap::new()),
            cached_cstructs: RefCell::new(HashMap::new()),
            cached_enums: RefCell::new(HashMap::new()),
        }
    }

    pub fn ptr_type(&self) -> PointerType<'ctx> {
        self.context.ptr_type(AddressSpace::default())
    }

    pub fn string_type(&self) -> StructType<'ctx> {
        self.context.struct_type(
            &[self.ptr_type().into(), self.context.i64_type().into()],
            false,
        )
    }

    pub fn slice_type(&self) -> StructType<'ctx> {
        self.context.struct_type(
            &[
                self.ptr_type().into(),
                self.context.i64_type().into(),
                self.context.i64_type().into(),
            ],
            false,
        )
    }

    pub fn interface_type(&self) -> StructType<'ctx> {
        self.context
            .struct_type(&[self.ptr_type().into(), self.ptr_type().into()], false)
    }

    pub fn closure_type(&self) -> StructType<'ctx> {
        self.context
            .struct_type(&[self.ptr_type().into(), self.ptr_type().into()], false)
    }

    pub fn weak_type(&self) -> StructType<'ctx> {
        self.context
            .struct_type(&[self.ptr_type().into(), self.ptr_type().into()], false)
    }

    pub fn enum_struct_type(&self, name: Option<&str>, max_payload_bytes: u64) -> StructType<'ctx> {
        let size = u32::try_from(max_payload_bytes).unwrap_or(u32::MAX);
        let payload_ty = self.context.i8_type().array_type(size);
        let fields = [self.context.i32_type().into(), payload_ty.into()];
        if let Some(n) = name {
            let st = self.context.opaque_struct_type(&format!("enum.{n}"));
            st.set_body(&fields, false);
            st
        } 

        else { self.context.struct_type(&fields, false) }
    }

    pub fn union_struct_type(&self, max_payload_bytes: u64) -> StructType<'ctx> {
        let size = u32::try_from(max_payload_bytes).unwrap_or(u32::MAX);
        let payload_ty = self.context.i8_type().array_type(size);
        self.context
            .struct_type(&[self.context.i32_type().into(), payload_ty.into()], false)
    }

    pub fn is_npo_pointer_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::Class(_)
                | Type::Ptr(_)
                | Type::Shared(_)
                | Type::Dynlib
                | Type::Ref { .. }
                | Type::Map(_, _)
        )
    }

    pub fn lower_c_int(&self, kind: CIntKind) -> IntType<'ctx> {
        match kind {
            CIntKind::Char | CIntKind::SChar | CIntKind::UChar => self.context.i8_type(),
            CIntKind::Short | CIntKind::UShort => self.context.i16_type(),
            CIntKind::Int | CIntKind::UInt => self.context.i32_type(),
            CIntKind::Long | CIntKind::ULong => match self.target.spec().data_model {
                DataModel::Lp64 => self.context.i64_type(),
                DataModel::Llp64 | DataModel::Ilp32 => self.context.i32_type(),
            },
            CIntKind::LongLong | CIntKind::ULongLong => self.context.i64_type(),
            CIntKind::SizeT | CIntKind::SSizeT => match self.target.spec().pointer_width {
                64 => self.context.i64_type(),
                32 => self.context.i32_type(),
                _ => self.context.ptr_sized_int_type(self.target_data, None),
            },
        }
    }

    pub fn lower_type(&self, ty: &Type) -> Result<BasicTypeEnum<'ctx>, String> {
        match ty {
            Type::Int8 | Type::Uint8 => Ok(self.context.i8_type().into()),
            Type::Int16 | Type::Uint16 => Ok(self.context.i16_type().into()),
            Type::Int32 | Type::Uint32 | Type::Char => Ok(self.context.i32_type().into()),
            Type::Int64 | Type::Uint64 => Ok(self.context.i64_type().into()),
            Type::Float32 => Ok(self.context.f32_type().into()),
            Type::Float64 => Ok(self.context.f64_type().into()),
            Type::Bool => Ok(self.context.bool_type().into()),
            Type::CInt(k) => Ok(self.lower_c_int(*k).into()),
            Type::String => Ok(self.string_type().into()),
            Type::Bytes | Type::Array(_) => Ok(self.slice_type().into()),
            Type::Interface(_) => Ok(self.interface_type().into()),
            Type::Weak(_) => Ok(self.weak_type().into()),
            Type::Class(_)
            | Type::Ptr(_)
            | Type::Shared(_)
            | Type::Dynlib
            | Type::Ref { .. }
            | Type::Map(_, _)
            | Type::Null
            | Type::TypeVar(_) => Ok(self.ptr_type().into()),
            Type::Fn { is_cfun, .. } => {
                if *is_cfun { Ok(self.ptr_type().into()) } 
                else { Ok(self.closure_type().into()) }
            }
            Type::Tuple(elems) => self.lower_tuple(elems),
            Type::Struct(name) => self.lower_named_struct(name).map(Into::into),
            Type::Cstruct(name) => self.lower_named_cstruct(name).map(Into::into),
            Type::Enum(name) => self.lower_named_enum(name).map(Into::into),
            Type::Union(members) => self.lower_union(members),
            Type::Void | Type::Never => Err(format!(
                "Type '{ty}' does not have a basic value representation"
            )),
            Type::Infer(id) => Err(format!("Uninferred type variable ?{id} in codegen")),
            Type::Error => Err("Cannot generate code for erroneous type".to_string()),
        }
    }

    fn lower_tuple(&self, elems: &[Type]) -> Result<BasicTypeEnum<'ctx>, String> {
        let mut field_types = Vec::with_capacity(elems.len());
        for elem in elems {
            field_types.push(self.lower_type(elem)?);
        }
        Ok(self.context.struct_type(&field_types, false).into())
    }

    pub fn lower_named_struct(&self, name: &str) -> Result<StructType<'ctx>, String> {
        if let Some(cached) = self.cached_structs.borrow().get(name) { return Ok(*cached); }

        if let Some(existing) = self.context.get_struct_type(&format!("struct.{name}")) {
            self.cached_structs
                .borrow_mut()
                .insert(name.to_string(), existing);
            return Ok(existing);
        }

        let st = self.context.opaque_struct_type(&format!("struct.{name}"));
        self.cached_structs
            .borrow_mut()
            .insert(name.to_string(), st);

        if let Some(layout) = self.registry.structs.get(name) {
            let mut field_types = Vec::with_capacity(layout.fields.len());
            for (_, field_ty) in &layout.fields {
                field_types.push(self.lower_type(field_ty)?);
            }
            st.set_body(&field_types, false);
            Ok(st)
        } else {
            st.set_body(&[], false);
            Ok(st)
        }
    }

    pub fn lower_named_cstruct(&self, name: &str) -> Result<StructType<'ctx>, String> {
        if let Some(cached) = self.cached_cstructs.borrow().get(name) {
            return Ok(*cached);
        }

        if let Some(existing) = self.context.get_struct_type(&format!("cstruct.{name}")) {
            self.cached_cstructs
                .borrow_mut()
                .insert(name.to_string(), existing);
            return Ok(existing);
        }

        let st = self.context.opaque_struct_type(&format!("cstruct.{name}"));
        self.cached_cstructs
            .borrow_mut()
            .insert(name.to_string(), st);

        if let Some(layout) = self.registry.cstructs.get(name) {
            let mut field_types = Vec::with_capacity(layout.fields.len());
            for (_, field_ty) in &layout.fields {
                field_types.push(self.lower_type(field_ty)?);
            }
            st.set_body(&field_types, layout.is_packed);
            Ok(st)
        } else {
            st.set_body(&[], false);
            Ok(st)
        }
    }

    pub fn lower_named_enum(&self, name: &str) -> Result<StructType<'ctx>, String> {
        if let Some(cached) = self.cached_enums.borrow().get(name) {
            return Ok(*cached);
        }

        if let Some(existing) = self.context.get_struct_type(&format!("enum.{name}")) {
            self.cached_enums
                .borrow_mut()
                .insert(name.to_string(), existing);
            return Ok(existing);
        }

        let max_payload = if let Some(layout) = self.registry.enums.get(name) {
            let mut max_bytes = 0u64;
            for variant in &layout.variants {
                if !variant.payload.is_empty() {
                    let mut payload_fields = Vec::with_capacity(variant.payload.len());
                    for p in &variant.payload {
                        payload_fields.push(self.lower_type(p)?);
                    }
                    let payload_st = self.context.struct_type(&payload_fields, false);
                    let bytes = self.target_data.get_abi_size(&payload_st);
                    max_bytes = max_bytes.max(bytes);
                }
            }
            max_bytes
        } else { 0 };

        let st = self.enum_struct_type(Some(name), max_payload);
        self.cached_enums.borrow_mut().insert(name.to_string(), st);
        Ok(st)
    }

    pub fn lower_union(&self, members: &BTreeSet<Type>) -> Result<BasicTypeEnum<'ctx>, String> {
        // null ptr opt for nullable refs types
        if members.len() == 2 && members.contains(&Type::Null) {
            let inner = members
                .iter()
                .find(|m| **m != Type::Null)
                .ok_or_else(|| "Missing non-null variant".to_string())?;

            if self.is_npo_pointer_type(inner) {
                return Ok(self.ptr_type().into());
            }

            let inner_llvm = self.lower_type(inner)?;
            let st = self
                .context
                .struct_type(&[self.context.bool_type().into(), inner_llvm], false);
            return Ok(st.into());
        }

        // { i32 tag, [max_size x i8] payload }
        let mut max_bytes = 0u64;
        for member in members {
            if *member != Type::Null {
                let llvm_ty = self.lower_type(member)?;
                let bytes = self.target_data.get_abi_size(&llvm_ty);
                max_bytes = max_bytes.max(bytes);
            }
        }
        Ok(self.union_struct_type(max_bytes).into())
    }

    pub fn lower_return_type(&self, ty: &Type) -> Result<Option<BasicTypeEnum<'ctx>>, String> {
        match ty {
            Type::Void | Type::Never => Ok(None),
            _ => self.lower_type(ty).map(Some),
        }
    }

    pub fn abi_size_of(&self, ty: &Type) -> Result<u64, String> {
        let llvm_ty = self.lower_type(ty)?;
        Ok(self.target_data.get_abi_size(&llvm_ty))
    }

    pub fn abi_align_of(&self, ty: &Type) -> Result<u32, String> {
        let llvm_ty = self.lower_type(ty)?;
        Ok(self.target_data.get_abi_alignment(&llvm_ty))
    }

    pub fn class_instance_type(&self, name: &str) -> Result<StructType<'ctx>, String> {
        let full_name = format!("class.{name}");
        if let Some(existing) = self.context.get_struct_type(&full_name) {
            return Ok(existing);
        }
        let st = self.context.opaque_struct_type(&full_name);
        let mut fields = vec![self.ptr_type().into()]; // vtable ptr at index 0

        if let Some(layout) = self.registry.classes.get(name) {
            for (_, field_ty) in &layout.fields {
                fields.push(self.lower_type(field_ty)?);
            }
        }
        st.set_body(&fields, false);
        Ok(st)
    }

    pub fn shared_box_type(&self, inner: &Type) -> Result<StructType<'ctx>, String> {
        let inner_llvm = self.lower_type(inner)?;
        Ok(self.context.struct_type(
            &[
                self.context.i64_type().into(),
                self.context.i64_type().into(),
                inner_llvm,
            ],
            false,
        ))
    }
}
