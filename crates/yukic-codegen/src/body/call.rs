#![deny(unsafe_code)]

use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::module::Linkage;
use inkwell::types::{BasicMetadataTypeEnum, BasicType};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, IntValue, PointerValue, StructValue};
use yukic_ast::expr::{Expr, ExprKind, LiteralValue};
use yukic_ast::ident::Ident;
use yukic_types::Type;
use yukic_types::convert::ast_type_to_type;

use super::BodyCompiler;

impl<'ctx> BodyCompiler<'_, 'ctx> {
    pub fn compile_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if matches!(&callee.kind, ExprKind::Super) {
            return self.compile_super_method_call(&Ident::new("init", callee.span), args);
        }

        if let Some(res) = self.try_compile_builtin(callee, args)? {
            return Ok(res);
        }

        let mut arg_vals: Vec<BasicValueEnum<'ctx>> = Vec::with_capacity(args.len());
        for arg in args {
            let val = self.compile_expr(arg)?;
            arg_vals.push(val);
        }

        match &callee.kind {
            ExprKind::Ident(ident) => {
                if self.registry.classes.contains_key(&ident.name) {
                    return self.compile_class_instantiation(&ident.name, &arg_vals);
                }
                if self.lookup_local(&ident.name).is_some() {
                    return self.compile_indirect_call(callee, &arg_vals);
                }
                self.resolve_and_emit_fn_call(&ident.name, &arg_vals)
            }
            ExprKind::Path(idents) => {
                if idents.len() == 1 {
                    if let Some(first) = idents.first() {
                        if self.lookup_local(&first.name).is_some() {
                            return self.compile_indirect_call(callee, &arg_vals);
                        }
                    }
                }
                if idents.len() == 2 && idents[0].name == "extern" {
                    let fn_name = &idents[1].name;
                    let c_sym = self
                        .registry
                        .c_symbols
                        .get(fn_name)
                        .cloned()
                        .unwrap_or_else(|| fn_name.clone());
                    return self.resolve_and_emit_c_fn_call(&c_sym, &arg_vals);
                }
                let name = idents.last().map_or("", |id| &id.name);
                self.resolve_and_emit_fn_call(name, &arg_vals)
            }
            _ => self.compile_indirect_call(callee, &arg_vals),
        }
    }

    pub fn compile_class_init(
        &mut self,
        callee: &Expr,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let class_name = match &callee.kind {
            ExprKind::Ident(ident) => ident.name.clone(),
            ExprKind::Path(idents) => idents.last().map_or("", |id| &id.name).to_string(),
            _ => return Err("expected class name in class init".to_string()),
        };
        let mut arg_vals: Vec<BasicValueEnum<'ctx>> = Vec::with_capacity(args.len());
        for arg in args {
            let val = self.compile_expr(arg)?;
            arg_vals.push(val);
        }
        self.compile_class_instantiation(&class_name, &arg_vals)
    }

    pub fn compile_method_call(
        &mut self,
        target: &Expr,
        method: &Ident,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let args = self.fill_method_default_args(target, method, args);
        if matches!(&target.kind, ExprKind::Super) {
            return self.compile_super_method_call(method, &args);
        }
        let sem_ty = self.expr_semantic_ty(target);
        let actual_ty = match &sem_ty {
            Some(Type::Ref { inner, .. }) => Some(inner.as_ref()),
            other => other.as_ref(),
        };
        if let Some(Type::Class(class_name)) = actual_ty {
            let target_val = self.compile_expr(target)?;
            return self.compile_class_instance_method_call(class_name, target_val, method, &args);
        }
        if let Some(Type::Interface(iface_name)) = actual_ty {
            let target_val = self.compile_expr(target)?;
            return self.compile_interface_method_call(iface_name, target_val, method, &args);
        }
        if matches!(actual_ty, Some(Type::Array(_) | Type::Bytes)) {
            match method.name.as_str() {
                "push" => return self.compile_slice_push(target, actual_ty, &args),
                "pop" => return self.compile_slice_pop(target, actual_ty),
                "del" => return self.compile_slice_del(target, actual_ty, &args),
                "clear" => return self.compile_slice_clear(target),
                "reserve" => return self.compile_slice_reserve(target, actual_ty, &args),
                _ => {}
            }
        }
        let target_val = self.compile_expr(target)?;
        self.compile_method_call_on_value(target_val, sem_ty.as_ref(), method, &args)
    }

    // append trailing default arguments for a class method call; defaults
    // are keyed by "Class::method" because bare names collide across classes
    fn fill_method_default_args(
        &self,
        target: &Expr,
        method: &Ident,
        args: &[Expr],
    ) -> Vec<Expr> {
        let sem_ty = self.expr_semantic_ty(target);
        let unwrapped = sem_ty.as_ref().map(|t| t.unwrap_nullable());
        let Some(Type::Class(class_name)) = unwrapped.as_ref() else {
            return args.to_vec();
        };
        if args
            .iter()
            .any(|a| matches!(a.kind, ExprKind::NamedArg(_, _)))
        {
            return args.to_vec();
        }
        let key = format!("{class_name}::{}", method.name);
        let Some(entry) = self.registry.defaults.get(&key) else {
            return args.to_vec();
        };
        if entry.is_free_fn
            || entry.params.iter().any(|p| p.is_variadic)
            || args.len() >= entry.params.len()
            || entry.params[args.len()].default_val.is_none()
        {
            return args.to_vec();
        }
        let mut filled = args.to_vec();
        while filled.len() < entry.params.len() {
            let idx = filled.len();
            match &entry.params[idx].default_val {
                Some(default_val) => filled.push(default_val.clone()),
                None => break,
            }
        }
        filled
    }

    pub fn compile_class_instance_method_call_values(
        &mut self,
        class_name: &str,
        target_val: BasicValueEnum<'ctx>,
        method_name: &str,
        arg_vals: &[BasicValueEnum<'ctx>],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let mut all_args = Vec::with_capacity(arg_vals.len() + 1);
        all_args.push(target_val);
        all_args.extend_from_slice(arg_vals);

        let sym = self
            .find_class_method(class_name, method_name, arg_vals.len())
            .unwrap_or_else(|| {
                crate::mangle::method_symbol_name(class_name, method_name, arg_vals.len())
            });
        self.resolve_and_emit_fn_call(&sym, &all_args)
    }

    fn compile_class_instance_method_call(
        &mut self,
        class_name: &str,
        target_val: BasicValueEnum<'ctx>,
        method: &Ident,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let mut arg_vals: Vec<BasicValueEnum<'ctx>> = Vec::with_capacity(args.len());
        for arg in args {
            let val = self.compile_expr(arg)?;
            arg_vals.push(val);
        }
        self.compile_class_instance_method_call_values(
            class_name,
            target_val,
            &method.name,
            &arg_vals,
        )
    }

    pub fn compile_method_call_on_value(
        &mut self,
        target_val: BasicValueEnum<'ctx>,
        target_ty: Option<&Type>,
        method: &Ident,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let actual_ty = match target_ty {
            Some(Type::Ref { inner, .. }) => Some(inner.as_ref()),
            other => other,
        };
        let unwrapped = actual_ty.map(|t| t.unwrap_nullable());
        if let Some(Type::Interface(iface_name)) = unwrapped.as_ref() {
            return self.compile_interface_method_call(iface_name, target_val, method, args);
        }
        if let Some(Type::Class(class_name)) = unwrapped.as_ref() {
            return self.compile_class_instance_method_call(class_name, target_val, method, args);
        }
        match method.name.as_str() {
            "as_ptr" => {
                let ptr = self.intrinsic.emit_slice_as_ptr(self.builder, target_val)?;
                Ok(ptr.into())
            }
            "len" => {
                let len = self.intrinsic.emit_slice_len(self.builder, target_val)?;
                Ok(len.into())
            }
            "cap" => {
                let cap = self.intrinsic.emit_slice_cap(self.builder, target_val)?;
                Ok(cap.into())
            }
            "is_empty" => {
                let len = self.intrinsic.emit_slice_len(self.builder, target_val)?;
                let zero = self.context.i64_type().const_zero();
                let cmp = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, len, zero, "is_empty")
                    .map_err(|e| e.to_string())?;
                Ok(cmp.into())
            }
            "cp" => self.compile_cp_intrinsic(target_val, target_ty),
            _ => self.compile_class_or_virtual_method(target_val, method, args),
        }
    }

    fn extract_interface_fat_pointer(
        &self,
        target_val: BasicValueEnum<'ctx>,
        iface_name: &str,
    ) -> Result<(inkwell::values::PointerValue<'ctx>, inkwell::values::PointerValue<'ctx>), String> {
        let fat_val = match target_val {
            BasicValueEnum::StructValue(sv) => sv,
            BasicValueEnum::PointerValue(pv) => {
                let fat_ty = self.vtable.interface_fat_type();
                self.builder
                    .build_load(fat_ty, pv, "iface.fat")
                    .map_err(|e| e.to_string())?
                    .into_struct_value()
            }
            _ => return Err(format!("expected interface fat pointer for '{iface_name}'")),
        };
        let obj_ptr = self
            .builder
            .build_extract_value(fat_val, 0, "iface.obj_ptr")
            .map_err(|e| e.to_string())?
            .into_pointer_value();
        let itable_ptr = self
            .builder
            .build_extract_value(fat_val, 1, "iface.itable_ptr")
            .map_err(|e| e.to_string())?
            .into_pointer_value();
        Ok((obj_ptr, itable_ptr))
    }

    fn resolve_interface_method_slot(
        &self,
        iface_name: &str,
        method: &Ident,
        args_count: usize,
    ) -> Result<(u32, Type, Vec<(Type, bool)>), String> {
        let mangled = crate::mangle::mangle_method_name(&method.name, args_count);
        let base_name = iface_name.split('<').next().unwrap_or(iface_name);
        if let Some(decl) = self.vtable.get_interface_decl(base_name) {
            let idx = decl
                .methods
                .iter()
                .position(|m| {
                    m.name.name == method.name
                        || crate::mangle::mangle_method_name(&m.name.name, m.params.len()) == method.name
                        || m.name.name == mangled
                })
                .ok_or_else(|| {
                    format!("method '{}' not found in interface '{iface_name}'", method.name)
                })?;
            let m_def = &decl.methods[idx];
            let raw_ret = m_def
                .return_type
                .as_ref()
                .map_or(Type::Void, |t| self.registry.canonicalize_type(&ast_type_to_type(t)));
            let raw_pts: Vec<(Type, bool)> = m_def
                .params
                .iter()
                .map(|p| {
                    (
                        p.ty
                            .as_ref()
                            .map_or(Type::Infer(0), |t| self.registry.canonicalize_type(&ast_type_to_type(t))),
                        p.is_ref,
                    )
                })
                .collect();

            let (ret, pts) = if iface_name.contains('<') && !decl.generics.is_empty() {
                let (_, args) = yukic_types::parse_type_generic_args(iface_name);
                let mapping: std::collections::HashMap<String, Type> = decl
                    .generics
                    .iter()
                    .map(|g| g.name.name.clone())
                    .zip(args)
                    .collect();
                let s_ret = raw_ret.substitute(&mapping);
                let s_pts = raw_pts
                    .into_iter()
                    .map(|(t, is_ref)| (t.substitute(&mapping), is_ref))
                    .collect();
                (s_ret, s_pts)
            } else {
                (raw_ret, raw_pts)
            };

            return Ok((u32::try_from(idx).map_err(|e| e.to_string())?, ret, pts));
        }
        if let Some(slot) = self
            .vtable
            .get_interface_slot(base_name, mangled)
            .or_else(|| self.vtable.get_interface_slot(base_name, &method.name))
        {
            let ret = self
                .registry
                .functions
                .get(&format!("{base_name}::{}", method.name))
                .or_else(|| self.registry.functions.get(&format!("{base_name}::{mangled}")))
                .cloned()
                .unwrap_or(Type::Void);
            return Ok((slot, ret, Vec::new()));
        }
        Err(format!("unknown interface or method '{iface_name}::{}'", method.name))
    }

    pub fn compile_interface_method_call(
        &mut self,
        iface_name: &str,
        target_val: BasicValueEnum<'ctx>,
        method: &Ident,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let (obj_ptr, itable_ptr) = self.extract_interface_fat_pointer(target_val, iface_name)?;
        let (slot_index, ret_ty, param_tys) =
            self.resolve_interface_method_slot(iface_name, method, args.len())?;
        let field_tys = vec![self.ptr_type().into(); (slot_index as usize) + 1];
        let itable_ty = self.context.struct_type(&field_tys, false);
        let slot_gep = self
            .builder
            .build_struct_gep(itable_ty, itable_ptr, slot_index, "islot_gep")
            .map_err(|e| e.to_string())?;
        let fn_ptr = self
            .builder
            .build_load(self.ptr_type(), slot_gep, "imethod_ptr")
            .map_err(|e| e.to_string())?
            .into_pointer_value();

        let mut call_args: Vec<BasicValueEnum<'ctx>> = Vec::with_capacity(args.len() + 2);
        if crate::region::RegionManager::is_regional_type(&ret_ty) {
            let reg = self
                .dest_region
                .or(self.region_ptr)
                .map_or_else(|| self.ptr_type().const_null().into(), Into::into);
            call_args.push(reg);
        }
        call_args.push(obj_ptr.into());
        for (i, arg) in args.iter().enumerate() {
            let val = self.compile_expr(arg)?;
            let coerced = match param_tys.get(i) {
                Some((pty, is_ref_param)) if !matches!(pty, Type::Infer(_)) => {
                    if *is_ref_param {
                        // ref params travel as raw pointers
                        self.coerce_type(val, self.ptr_type().into())?
                    } else {
                        let llvm_pty = self.lower_type(pty)?;
                        self.coerce_type(val, llvm_pty)?
                    }
                }
                _ => val,
            };
            call_args.push(coerced);
        }

        let is_void = ret_ty == Type::Void;
        let llvm_ret = if is_void { None } else { Some(self.lower_type(&ret_ty)?) };
        let meta_param_tys: Vec<BasicMetadataTypeEnum<'ctx>> =
            call_args.iter().map(|a| a.get_type().into()).collect();
        let fn_type = match llvm_ret {
            Some(rt) => rt.fn_type(&meta_param_tys, false),
            None => self.context.void_type().fn_type(&meta_param_tys, false),
        };
        self.emit_indirect_method_call(fn_type, fn_ptr, &call_args, is_void)
    }

    fn emit_indirect_method_call(
        &mut self,
        fn_type: inkwell::types::FunctionType<'ctx>,
        fn_ptr: inkwell::values::PointerValue<'ctx>,
        args: &[BasicValueEnum<'ctx>],
        is_void: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let call_conv = inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32;
        let unwind_bb = self.unwind_dest.or_else(|| {
            if self.has_active_cleanups() {
                self.get_or_create_cleanup_unwind_bb().ok()
            } else {
                None
            }
        });

        if let Some(dest) = unwind_bb {
            let personality = self.get_or_declare_personality();
            self.fn_val.set_personality_function(personality);
            let next_bb = self.context.append_basic_block(self.fn_val, "i_inv.cont");
            let inv = self
                .builder
                .build_indirect_invoke(fn_type, fn_ptr, args, next_bb, dest, "i_inv")
                .map_err(|e| e.to_string())?;
            inv.set_call_convention(call_conv);
            self.builder.position_at_end(next_bb);
            if is_void {
                Ok(self.context.i64_type().const_zero().into())
            } else {
                let dummy = self.context.i64_type().const_zero().into();
                Ok(inv.try_as_basic_value().basic().unwrap_or(dummy))
            }
        } else {
            let meta_args: Vec<BasicMetadataValueEnum<'ctx>> = args.iter().map(|&a| a.into()).collect();
            let call = self
                .builder
                .build_indirect_call(fn_type, fn_ptr, &meta_args, "i_call")
                .map_err(|e| e.to_string())?;
            call.set_call_convention(call_conv);
            if is_void {
                Ok(self.context.i64_type().const_zero().into())
            } else {
                let dummy = self.context.i64_type().const_zero().into();
                Ok(call.try_as_basic_value().basic().unwrap_or(dummy))
            }
        }
    }

    fn compile_slice_push(
        &mut self,
        target: &Expr,
        actual_ty: Option<&Type>,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let arg = args.first().ok_or("`push` requires 1 argument")?;
        let elem_val = self.compile_expr(arg)?;
        let slice_ptr = self.compile_lvalue_ptr(target)?;
        let reg = self
            .dest_region
            .or(self.region_ptr)
            .ok_or("slice push requires an active region")?;

        let elem_sem_ty = match actual_ty {
            Some(Type::Array(inner)) => inner.as_ref(),
            Some(Type::Bytes) => &Type::Uint8,
            _ => &Type::Int64,
        };
        let expected_llvm_ty = self.lower_type(elem_sem_ty)?;
        let coerced_elem = self.coerce_type(elem_val, expected_llvm_ty)?;

        let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
        let lowerer = crate::types::TypeLowerer::new(self.context, &td, self.target, self.registry);
        let elem_size = lowerer.abi_size_of(elem_sem_ty).unwrap_or(8).max(1);
        let align = lowerer.abi_align_of(elem_sem_ty).unwrap_or(1) as u64;

        let elem_alloca = self
            .builder
            .build_alloca(expected_llvm_ty, "elem.push.tmp")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(elem_alloca, coerced_elem)
            .map_err(|e| e.to_string())?;

        let layout = crate::intrinsic::SliceElemLayout {
            size: self.context.i64_type().const_int(elem_size, false),
            align: self.context.i64_type().const_int(align, false),
        };

        self.intrinsic.emit_slice_push(
            self.module,
            self.builder,
            reg,
            slice_ptr,
            elem_alloca,
            layout,
        )?;
        Ok(self.context.i64_type().const_zero().into())
    }

    fn compile_slice_pop(
        &mut self,
        target: &Expr,
        actual_ty: Option<&Type>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let elem_ty = match actual_ty {
            Some(Type::Array(inner)) => inner.as_ref(),
            Some(Type::Bytes) => &Type::Uint8,
            _ => &Type::Int64,
        };
        let nullable_ty = Type::nullable(elem_ty.clone());
        let res_llvm_ty = self.lower_type(&nullable_ty)?;

        let slice_ptr = self.compile_lvalue_ptr(target)?;
        let slice_ty = self.intrinsic.slice_type();
        let len_gep = self
            .builder
            .build_struct_gep(slice_ty, slice_ptr, 1, "pop.len.gep")
            .map_err(|e| e.to_string())?;
        let len = self
            .builder
            .build_load(self.context.i64_type(), len_gep, "pop.len")
            .map_err(|e| e.to_string())?
            .into_int_value();
        let zero = self.context.i64_type().const_zero();
        let is_zero = self
            .builder
            .build_int_compare(IntPredicate::EQ, len, zero, "pop.is_zero")
            .map_err(|e| e.to_string())?;

        let f = self.fn_val;
        let empty_bb = self.context.append_basic_block(f, "pop.empty");
        let non_empty_bb = self.context.append_basic_block(f, "pop.non_empty");
        let merge_bb = self.context.append_basic_block(f, "pop.merge");

        self.builder
            .build_conditional_branch(is_zero, empty_bb, non_empty_bb)
            .map_err(|e| e.to_string())?;

        self.builder.position_at_end(empty_bb);
        let null_val = self.coerce_type(self.ptr_type().const_null().into(), res_llvm_ty)?;
        let empty_end_bb = self.builder.get_insert_block().unwrap();
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| e.to_string())?;

        self.builder.position_at_end(non_empty_bb);
        let one = self.context.i64_type().const_int(1, false);
        let new_len = self
            .builder
            .build_int_sub(len, one, "pop.new_len")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(len_gep, new_len)
            .map_err(|e| e.to_string())?;

        let elem_llvm_ty = self.lower_type(elem_ty)?;
        let elem_ptr = self.intrinsic.emit_slice_elem_ptr(
            self.builder,
            slice_ptr.into(),
            new_len,
            elem_llvm_ty,
        )?;
        let elem_val = self
            .builder
            .build_load(elem_llvm_ty, elem_ptr, "pop.elem")
            .map_err(|e| e.to_string())?;
        let some_val = self.coerce_type(elem_val, res_llvm_ty)?;
        let non_empty_end_bb = self.builder.get_insert_block().unwrap();
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| e.to_string())?;

        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(res_llvm_ty, "pop.res")
            .map_err(|e| e.to_string())?;
        phi.add_incoming(&[(&null_val, empty_end_bb), (&some_val, non_empty_end_bb)]);
        Ok(phi.as_basic_value())
    }

    fn compile_slice_del(
        &mut self,
        target: &Expr,
        actual_ty: Option<&Type>,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let arg = args.first().ok_or("`del` requires 1 argument")?;
        let idx_val = self.compile_expr(arg)?;
        let idx_int = self
            .coerce_type(idx_val, self.context.i64_type().into())?
            .into_int_value();
        let slice_ptr = self.compile_lvalue_ptr(target)?;

        let elem_sem_ty = match actual_ty {
            Some(Type::Array(inner)) => inner.as_ref(),
            Some(Type::Bytes) => &Type::Uint8,
            _ => &Type::Int64,
        };
        let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
        let lowerer = crate::types::TypeLowerer::new(self.context, &td, self.target, self.registry);
        let elem_size = lowerer.abi_size_of(elem_sem_ty).unwrap_or(8).max(1);

        let elem_size_val = self.context.i64_type().const_int(elem_size, false);
        self.intrinsic
            .emit_slice_del(self.module, self.builder, slice_ptr, idx_int, elem_size_val)?;
        Ok(self.context.i64_type().const_zero().into())
    }

    fn compile_slice_clear(
        &mut self,
        target: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let slice_ptr = self.compile_lvalue_ptr(target)?;
        let slice_ty = self.intrinsic.slice_type();
        let len_gep = self
            .builder
            .build_struct_gep(slice_ty, slice_ptr, 1, "slice.len.gep")
            .map_err(|e| e.to_string())?;
        let zero = self.context.i64_type().const_zero();
        self.builder
            .build_store(len_gep, zero)
            .map_err(|e| e.to_string())?;
        Ok(self.context.i64_type().const_zero().into())
    }

    fn compile_slice_reserve(
        &mut self,
        target: &Expr,
        actual_ty: Option<&Type>,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let arg = args.first().ok_or("`reserve` requires 1 argument")?;
        let add_val = self.compile_expr(arg)?;
        let add_int = self
            .coerce_type(add_val, self.context.i64_type().into())?
            .into_int_value();
        let slice_ptr = self.compile_lvalue_ptr(target)?;
        let reg = self
            .dest_region
            .or(self.region_ptr)
            .ok_or("slice reserve requires an active region")?;

        let elem_sem_ty = match actual_ty {
            Some(Type::Array(inner)) => inner.as_ref(),
            Some(Type::Bytes) => &Type::Uint8,
            _ => &Type::Int64,
        };
        let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
        let lowerer = crate::types::TypeLowerer::new(self.context, &td, self.target, self.registry);
        let elem_size = lowerer.abi_size_of(elem_sem_ty).unwrap_or(8).max(1);
        let align = lowerer.abi_align_of(elem_sem_ty).unwrap_or(1) as u64;

        let layout = crate::intrinsic::SliceElemLayout {
            size: self.context.i64_type().const_int(elem_size, false),
            align: self.context.i64_type().const_int(align, false),
        };

        self.intrinsic.emit_slice_reserve(
            self.module,
            self.builder,
            reg,
            slice_ptr,
            add_int,
            layout,
        )?;
        Ok(self.context.i64_type().const_zero().into())
    }

    fn compile_cp_intrinsic(
        &mut self,
        target_val: BasicValueEnum<'ctx>,
        target_ty: Option<&Type>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let actual_ty = match target_ty {
            Some(Type::Ref { inner, .. }) => Some(inner.as_ref()),
            other => other,
        };
        match actual_ty {
            Some(Type::Array(elem_ty)) => {
                let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
                let lowerer = crate::types::TypeLowerer::new(self.context, &td, self.target, self.registry);
                let elem_size = lowerer.abi_size_of(elem_ty).unwrap_or(8).max(1);
                let align = lowerer.abi_align_of(elem_ty).unwrap_or(1) as u64;
                self.clone_slice_value(target_val, elem_size, align)
            }
            Some(Type::Bytes) => self.clone_slice_value(target_val, 1, 1),
            _ => {
                if target_val.is_struct_value() {
                    let sv = target_val.into_struct_value();
                    if sv.get_type().count_fields() == 3 {
                        return self.clone_slice_value(target_val, 8, 8);
                    }
                }
                Ok(target_val)
            }
        }
    }

    fn clone_slice_value(
        &mut self,
        target_val: BasicValueEnum<'ctx>,
        elem_size: u64,
        align: u64,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let src_ptr = self.intrinsic.emit_slice_as_ptr(self.builder, target_val)?;
        let len = self.intrinsic.emit_slice_len(self.builder, target_val)?;
        let i64_ty = self.context.i64_type();
        let elem_size_val = i64_ty.const_int(elem_size, false);
        let byte_len = self
            .builder
            .build_int_mul(len, elem_size_val, "cp_byte_len")
            .map_err(|e| e.to_string())?;

        let reg = self
            .dest_region
            .or(self.region_ptr)
            .ok_or("array cloning requires an active region")?;
        let buf = self.region.emit_dynamic_fast_path_alloc(
            self.module,
            self.builder,
            self.fn_val,
            reg,
            byte_len,
            align.max(1),
        )?;

        self.builder
            .build_memcpy(buf, align.max(1) as u32, src_ptr, align.max(1) as u32, byte_len)
            .map_err(|e| e.to_string())?;

        let new_slice = self
            .intrinsic
            .emit_slice_from_raw_parts(self.builder, buf, len, len)?;
        Ok(new_slice.into())
    }

    fn try_compile_builtin(
        &mut self,
        callee: &Expr,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let name = match &callee.kind {
            ExprKind::Ident(id) => Some(id.name.as_str()),
            ExprKind::Path(idents) => {
                if idents.len() == 2 && idents[0].name == "extern" {
                    return Ok(None);
                }
                if idents.len() == 2 && idents[0].name == "rt" {
                    Some(idents[1].name.as_str())
                } else if idents.len() >= 2 && idents.last().is_some_and(|id| id.name == "from_raw_parts") {
                    Some("from_raw_parts")
                } else {
                    idents.last().map(|id| id.name.as_str())
                }
            }
            _ => return Ok(None),
        };

        let Some(name) = name else {
            return Ok(None);
        };

        match name {
            "args" => self.compile_args_builtin().map(Some),
            "raw_args" => self.compile_raw_args_builtin().map(Some),
            "exit" => self.compile_exit_builtin(args).map(Some),
            "cast" => self.compile_cast_builtin(args).map(Some),
            "typeof" | "__typeof_intrinsic__" => self.compile_typeof_builtin(args).map(Some),
            "panic" => self.compile_panic_builtin(args).map(Some),
            "assert" => self.compile_assert_builtin(args).map(Some),
            "assert_eq" => self.compile_assert_eq_builtin(args, true).map(Some),
            "assert_ne" => self.compile_assert_eq_builtin(args, false).map(Some),
            "from_raw_parts" => self.compile_from_raw_parts_builtin(args).map(Some),
            "ptr_of" => self.compile_ptr_of_builtin(args).map(Some),
            _ => Ok(None),
        }
    }

    fn compile_args_builtin(&mut self) -> Result<BasicValueEnum<'ctx>, String> {
        let reg = self
            .dest_region
            .or(self.region_ptr)
            .ok_or("`args()` requires active region")?;
        self.intrinsic.emit_args(self.module, self.builder, reg)
    }

    fn compile_raw_args_builtin(&mut self) -> Result<BasicValueEnum<'ctx>, String> {
        let reg = self
            .dest_region
            .or(self.region_ptr)
            .ok_or("`raw_args()` requires active region")?;
        self.intrinsic.emit_raw_args(self.module, self.builder, reg)
    }

    fn compile_exit_builtin(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        let code_val = if let Some(arg) = args.first() {
            let val = self.compile_expr(arg)?;
            if val.is_int_value() {
                let iv = val.into_int_value();
                let width = iv.get_type().get_bit_width();
                match width.cmp(&32) {
                    std::cmp::Ordering::Greater => self
                        .builder
                        .build_int_truncate(iv, self.context.i32_type(), "exit_code")
                        .map_err(|e| e.to_string())?,
                    std::cmp::Ordering::Less => self
                        .builder
                        .build_int_s_extend(iv, self.context.i32_type(), "exit_code")
                        .map_err(|e| e.to_string())?,
                    std::cmp::Ordering::Equal => iv,
                }
            } else {
                self.context.i32_type().const_zero()
            }
        } else {
            self.context.i32_type().const_zero()
        };
        self.intrinsic.emit_exit(self.module, self.builder, code_val)?;
        Ok(self.context.i64_type().const_zero().into())
    }

    fn compile_cast_builtin(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() < 2 {
            return Err("`cast` requires 2 arguments: (value, target_type)".to_string());
        }
        let val_val = self.compile_expr(&args[0])?;
        let target_ty = match &args[1].kind {
            ExprKind::Literal(LiteralValue::String(s)) => {
                yukic_types::convert::parse_primitive_or_named(s)
            }
            ExprKind::Ident(id) => {
                yukic_types::convert::parse_primitive_or_named(&id.name)
            }
            ExprKind::Call(callee, inner_args) => {
                if let ExprKind::Ident(id) = &callee.kind {
                    if (id.name == "typeof" || id.name == "__typeof_intrinsic__") && !inner_args.is_empty() {
                        self.deduce_expr_type(&inner_args[0], None)?
                    } else {
                        return Err("invalid target in `cast`".to_string());
                    }
                } else {
                    return Err("invalid target in `cast`".to_string());
                }
            }
            _ => return Err("`cast` target must be a string literal or `typeof`".to_string()),
        };
        let src_ty = self.deduce_expr_type(&args[0], Some(val_val))?;

        let reg = self.dest_region.or(self.region_ptr);
        self.intrinsic.emit_cast(
            self.module,
            self.builder,
            val_val,
            &src_ty,
            &target_ty,
            reg,
        )
    }

    fn compile_typeof_builtin(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        let ty = if let Some(arg) = args.first() {
            let val = self.compile_expr(arg).ok();
            self.deduce_expr_type(arg, val)?
        } else {
            Type::Void
        };
        let sv = self.rodata.emit_typeof(self.module, &ty);
        Ok(sv.into())
    }

    fn compile_panic_builtin(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        let (msg_ptr, msg_len, kind_ptr, kind_len, has_code, code, payload_ptr, payload_len) =
            self.extract_panic_info(args)?;

        if self.is_abort_strategy() {
            let abort_fn = self.get_or_declare_panic_abort_fn();
            let meta_args: [BasicMetadataValueEnum<'ctx>; 6] = [
                msg_ptr.into(),
                msg_len.into(),
                kind_ptr.into(),
                kind_len.into(),
                has_code.into(),
                code.into(),
            ];
            self.builder
                .build_call(abort_fn, &meta_args, "")
                .map_err(|e| e.to_string())?;
            self.builder.build_unreachable().map_err(|e| e.to_string())?;
            return Ok(self.context.i64_type().const_zero().into());
        }

        let panic_fn = self.get_or_declare_typed_panic_fn();
        let unwind_bb = self.unwind_dest.or_else(|| {
            if self.has_active_cleanups() {
                self.get_or_create_cleanup_unwind_bb().ok()
            } else {
                None
            }
        });

        let call_args: [BasicValueEnum<'ctx>; 8] = [
            msg_ptr.into(),
            msg_len.into(),
            kind_ptr.into(),
            kind_len.into(),
            has_code.into(),
            code.into(),
            payload_ptr.into(),
            payload_len.into(),
        ];

        if let Some(unwind) = unwind_bb {
            let personality = self.get_or_declare_personality();
            self.fn_val.set_personality_function(personality);
            let cont_bb = self.context.append_basic_block(self.fn_val, "panic.unreach");
            self.builder
                .build_invoke(panic_fn, &call_args, cont_bb, unwind, "")
                .map_err(|e| e.to_string())?;
            self.builder.position_at_end(cont_bb);
            self.builder.build_unreachable().map_err(|e| e.to_string())?;
        } else {
            let meta_args: [BasicMetadataValueEnum<'ctx>; 8] = [
                msg_ptr.into(),
                msg_len.into(),
                kind_ptr.into(),
                kind_len.into(),
                has_code.into(),
                code.into(),
                payload_ptr.into(),
                payload_len.into(),
            ];
            self.builder
                .build_call(panic_fn, &meta_args, "")
                .map_err(|e| e.to_string())?;
            self.builder.build_unreachable().map_err(|e| e.to_string())?;
        }
        Ok(self.context.i64_type().const_zero().into())
    }

    fn extract_panic_info(
        &mut self,
        args: &[Expr],
    ) -> Result<
        (
            inkwell::values::PointerValue<'ctx>,
            IntValue<'ctx>,
            inkwell::values::PointerValue<'ctx>,
            IntValue<'ctx>,
            IntValue<'ctx>,
            IntValue<'ctx>,
            inkwell::values::PointerValue<'ctx>,
            IntValue<'ctx>,
        ),
        String,
    > {
        let null_p = self.context.ptr_type(AddressSpace::default()).const_null();
        let zero_64 = self.context.i64_type().const_zero();
        let zero_32 = self.context.i32_type().const_zero();

        let default_kind = self.rodata.emit_string(self.module, "Panic");
        let default_kind_p = self.builder.build_extract_value(default_kind, 0, "k_p").map_err(|e| e.to_string())?.into_pointer_value();
        let default_kind_l = self.builder.build_extract_value(default_kind, 1, "k_l").map_err(|e| e.to_string())?.into_int_value();

        let Some(arg) = args.first() else {
            return Ok((null_p, zero_64, default_kind_p, default_kind_l, zero_32, zero_32, null_p, zero_64));
        };

        let val = self.compile_expr(arg)?;
        let sem_ty = self.deduce_expr_type(arg, Some(val)).ok().or_else(|| self.expr_semantic_ty(arg));

        let BasicValueEnum::StructValue(sv) = val else {
            return Ok((null_p, zero_64, default_kind_p, default_kind_l, zero_32, zero_32, null_p, zero_64));
        };

        let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
        let lowerer = crate::types::TypeLowerer::new(self.context, &td, self.target, self.registry);
        let byte_size = sem_ty.as_ref().and_then(|t| lowerer.abi_size_of(t).ok()).unwrap_or(0);
        let (payload_ptr, payload_len) = if byte_size > 0 {
            let alloca = self.builder.build_alloca(val.get_type(), "panic_payload").map_err(|e| e.to_string())?;
            self.builder.build_store(alloca, val).map_err(|e| e.to_string())?;
            (alloca, self.context.i64_type().const_int(byte_size, false))
        } else {
            (null_p, zero_64)
        };

        if let Some(Type::Struct(struct_name)) = &sem_ty {
            let kind_sv = self.rodata.emit_string(self.module, struct_name);
            let kind_p = self.builder.build_extract_value(kind_sv, 0, "kp").map_err(|e| e.to_string())?.into_pointer_value();
            let kind_l = self.builder.build_extract_value(kind_sv, 1, "kl").map_err(|e| e.to_string())?.into_int_value();

            if let Some(layout) = self.registry.structs.get(struct_name).cloned() {
                let (msg_p, msg_l) = self.extract_struct_message(sv, &layout)?;
                let (has_c, code_v) = self.extract_struct_code(sv, &layout)?;
                return Ok((msg_p, msg_l, kind_p, kind_l, has_c, code_v, payload_ptr, payload_len));
            }
        }

        let f0 = self.builder.build_extract_value(sv, 0, "p_f0").map_err(|e| e.to_string())?;
        if f0.is_struct_value() {
            let inner_sv = f0.into_struct_value();
            if inner_sv.get_type().count_fields() >= 2 {
                let p = self.builder.build_extract_value(inner_sv, 0, "p_str_ptr").map_err(|e| e.to_string())?;
                let l = self.builder.build_extract_value(inner_sv, 1, "p_str_len").map_err(|e| e.to_string())?;
                if p.is_pointer_value() && l.is_int_value() {
                    let (has_c, code_v) = self.extract_second_field_code(sv)?;
                    return Ok((p.into_pointer_value(), l.into_int_value(), default_kind_p, default_kind_l, has_c, code_v, payload_ptr, payload_len));
                }
            }
        }

        if f0.is_pointer_value() && sv.get_type().count_fields() >= 2 {
            let f1 = self.builder.build_extract_value(sv, 1, "p_f1").map_err(|e| e.to_string())?;
            if f1.is_int_value() {
                return Ok((f0.into_pointer_value(), f1.into_int_value(), default_kind_p, default_kind_l, zero_32, zero_32, payload_ptr, payload_len));
            }
        }

        Ok((null_p, zero_64, default_kind_p, default_kind_l, zero_32, zero_32, payload_ptr, payload_len))
    }

    fn extract_struct_message(
        &mut self,
        sv: StructValue<'ctx>,
        layout: &crate::registry::StructLayout,
    ) -> Result<(inkwell::values::PointerValue<'ctx>, IntValue<'ctx>), String> {
        let null_p = self.context.ptr_type(AddressSpace::default()).const_null();
        let zero_64 = self.context.i64_type().const_zero();
        let msg_field = layout.fields.iter().enumerate().find(|(_, (n, t))| {
            (n == "message" || n == "msg") || *t == Type::String
        });
        let Some((idx, _)) = msg_field else {
            return Ok((null_p, zero_64));
        };
        let f_val = self.builder.build_extract_value(sv, idx as u32, "m_f").map_err(|e| e.to_string())?;
        if !f_val.is_struct_value() {
            return Ok((null_p, zero_64));
        }
        let inner = f_val.into_struct_value();
        if inner.get_type().count_fields() < 2 {
            return Ok((null_p, zero_64));
        }
        let p = self.builder.build_extract_value(inner, 0, "mp").map_err(|e| e.to_string())?;
        let l = self.builder.build_extract_value(inner, 1, "ml").map_err(|e| e.to_string())?;
        if p.is_pointer_value() && l.is_int_value() {
            Ok((p.into_pointer_value(), l.into_int_value()))
        } else {
            Ok((null_p, zero_64))
        }
    }

    fn extract_struct_code(
        &mut self,
        sv: StructValue<'ctx>,
        layout: &crate::registry::StructLayout,
    ) -> Result<(IntValue<'ctx>, IntValue<'ctx>), String> {
        let zero_32 = self.context.i32_type().const_zero();
        let one_32 = self.context.i32_type().const_int(1, false);
        let code_field = layout.fields.iter().enumerate().find(|(_, (n, t))| {
            n == "code" || matches!(t, Type::Int32 | Type::Int64 | Type::Uint32 | Type::Uint64)
        });
        let Some((idx, _)) = code_field else {
            return Ok((zero_32, zero_32));
        };
        let c_val = self.builder.build_extract_value(sv, idx as u32, "c_f").map_err(|e| e.to_string())?;
        if !c_val.is_int_value() {
            return Ok((zero_32, zero_32));
        }
        let int_val = c_val.into_int_value();
        let i32_val = if int_val.get_type().get_bit_width() > 32 {
            self.builder.build_int_truncate(int_val, self.context.i32_type(), "c_trunc").map_err(|e| e.to_string())?
        } else if int_val.get_type().get_bit_width() < 32 {
            self.builder.build_int_z_extend(int_val, self.context.i32_type(), "c_ext").map_err(|e| e.to_string())?
        } else {
            int_val
        };
        Ok((one_32, i32_val))
    }

    fn extract_second_field_code(
        &mut self,
        sv: StructValue<'ctx>,
    ) -> Result<(IntValue<'ctx>, IntValue<'ctx>), String> {
        let zero_32 = self.context.i32_type().const_zero();
        let one_32 = self.context.i32_type().const_int(1, false);
        if sv.get_type().count_fields() < 2 {
            return Ok((zero_32, zero_32));
        }
        let f1 = self.builder.build_extract_value(sv, 1, "p_f1").map_err(|e| e.to_string())?;
        if !f1.is_int_value() {
            return Ok((zero_32, zero_32));
        }
        let iv = f1.into_int_value();
        let c32 = if iv.get_type().get_bit_width() > 32 {
            self.builder.build_int_truncate(iv, self.context.i32_type(), "c_trunc").map_err(|e| e.to_string())?
        } else {
            iv
        };
        Ok((one_32, c32))
    }

    fn get_or_declare_panic_fn(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("yukino_raise_panic") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let fn_ty = self.context.void_type().fn_type(&[ptr_ty.into(), i64_ty.into()], false);
        let f = self.module.add_function("yukino_raise_panic", fn_ty, None);
        f.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32);
        f
    }

    fn get_or_declare_typed_panic_fn(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("yukino_raise_typed_panic") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let i32_ty = self.context.i32_type();
        let fn_ty = self.context.void_type().fn_type(
            &[
                ptr_ty.into(),
                i64_ty.into(),
                ptr_ty.into(),
                i64_ty.into(),
                i32_ty.into(),
                i32_ty.into(),
                ptr_ty.into(),
                i64_ty.into(),
            ],
            false,
        );
        let f = self.module.add_function("yukino_raise_typed_panic", fn_ty, None);
        f.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32);
        f
    }

    fn compile_assert_builtin(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        let cond_expr = args.first().ok_or("assert requires a condition")?;
        let cond_val = self.compile_expr(cond_expr)?;
        let cond_bool = self.to_bool(cond_val)?;

        let cont_bb = self.context.append_basic_block(self.fn_val, "assert_ok");
        let fail_bb = self.context.append_basic_block(self.fn_val, "assert_fail");
        self.builder
            .build_conditional_branch(cond_bool, cont_bb, fail_bb)
            .map_err(|e| e.to_string())?;

        self.builder.position_at_end(fail_bb);
        let panic_fn = self.get_or_declare_panic_fn();
        let null_p = self.context.ptr_type(AddressSpace::default()).const_null();
        let zero_l = self.context.i64_type().const_zero();
        self.builder
            .build_call(panic_fn, &[null_p.into(), zero_l.into()], "")
            .map_err(|e| e.to_string())?;
        self.builder.build_unreachable().map_err(|e| e.to_string())?;

        self.builder.position_at_end(cont_bb);
        Ok(self.context.i64_type().const_zero().into())
    }

    fn compile_assert_eq_builtin(
        &mut self,
        args: &[Expr],
        is_eq: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() < 2 {
            return Err("assert_eq requires 2 arguments".to_string());
        }
        let lhs = self.compile_expr(&args[0])?;
        let rhs = self.compile_expr(&args[1])?;
        let rhs = self.coerce_type(rhs, lhs.get_type())?;

        let pred = if is_eq {
            IntPredicate::EQ
        } else {
            IntPredicate::NE
        };
        let cmp_bool = if lhs.is_int_value() && rhs.is_int_value() {
            self.builder
                .build_int_compare(pred, lhs.into_int_value(), rhs.into_int_value(), "assert_cmp")
                .map_err(|e| e.to_string())?
        } else {
            self.context.bool_type().const_int(1, false)
        };

        let cont_bb = self.context.append_basic_block(self.fn_val, "assert_ok");
        let fail_bb = self.context.append_basic_block(self.fn_val, "assert_fail");
        self.builder
            .build_conditional_branch(cmp_bool, cont_bb, fail_bb)
            .map_err(|e| e.to_string())?;

        self.builder.position_at_end(fail_bb);
        let panic_fn = self.get_or_declare_panic_fn();
        let null_p = self.context.ptr_type(AddressSpace::default()).const_null();
        let zero_l = self.context.i64_type().const_zero();
        self.builder
            .build_call(panic_fn, &[null_p.into(), zero_l.into()], "")
            .map_err(|e| e.to_string())?;
        self.builder.build_unreachable().map_err(|e| e.to_string())?;

        self.builder.position_at_end(cont_bb);
        Ok(self.context.i64_type().const_zero().into())
    }

    fn compile_from_raw_parts_builtin(
        &mut self,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() == 2 {
            let ptr = self.compile_expr(&args[0])?.into_pointer_value();
            let len = self.compile_expr(&args[1])?.into_int_value();
            let sv = self.intrinsic.emit_string_from_raw_parts(self.builder, ptr, len)?;
            Ok(sv.into())
        } else if args.len() >= 3 {
            let ptr = self.compile_expr(&args[0])?.into_pointer_value();
            let len = self.compile_expr(&args[1])?.into_int_value();
            let cap = self.compile_expr(&args[2])?.into_int_value();
            let sv = self.intrinsic.emit_slice_from_raw_parts(self.builder, ptr, len, cap)?;
            Ok(sv.into())
        } else {
            Err("from_raw_parts requires 2 or 3 arguments".to_string())
        }
    }

    fn compile_ptr_of_builtin(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        let arg = args.first().ok_or("`ptr_of` requires an argument")?;
        match &arg.kind {
            ExprKind::Ident(id) => self
                .lookup_local(&id.name)
                .map(|local| local.ptr.into())
                .ok_or_else(|| "`ptr_of` variable not found".to_string()),
            _ => Err("`ptr_of` expects a local variable identifier".to_string()),
        }
    }

    pub fn deduce_expr_type(
        &self,
        expr: &Expr,
        val: Option<BasicValueEnum<'ctx>>,
    ) -> Result<Type, String> {
        if let Some(sem) = self.expr_semantic_ty(expr) {
            return Ok(sem);
        }

        match &expr.kind {
            ExprKind::Ident(id) => {
                if let Some(sem) = self.lookup_local(&id.name).and_then(|l| l.semantic_ty.as_ref()) {
                    return Ok(sem.clone());
                }
            }
            ExprKind::IndexAccess(target, _) => {
                if let ExprKind::Ident(id) = &target.kind {
                    let sem = self.lookup_local(&id.name).and_then(|l| l.semantic_ty.as_ref());
                    let actual = match sem {
                        Some(Type::Ref { inner, .. }) => Some(inner.as_ref()),
                        other => other,
                    };
                    if let Some(Type::Array(elem)) = actual {
                        return Ok(*elem.clone());
                    }
                }
            }
            ExprKind::Is(_, _) => return Ok(Type::Bool),
            ExprKind::Literal(lit) => match lit {
                LiteralValue::Int(_) => return Ok(Type::Int64),
                LiteralValue::UInt(_) => return Ok(Type::Uint64),
                LiteralValue::Float(_) => return Ok(Type::Float64),
                LiteralValue::String(_) => return Ok(Type::String),
                LiteralValue::Char(_) => return Ok(Type::Char),
                LiteralValue::Bool(_) => return Ok(Type::Bool),
                _ => {}
            },
            _ => {}
        }

        if let Some(v) = val {
            if v.is_struct_value() {
                let sv = v.into_struct_value();
                let num_fields = sv.get_type().count_fields();
                if num_fields == 2 {
                    return Ok(Type::String);
                } else if num_fields == 3 {
                    return Ok(Type::Bytes);
                }
            } else if v.is_int_value() {
                let bits = v.into_int_value().get_type().get_bit_width();
                return match bits {
                    1 => Ok(Type::Bool),
                    8 => Ok(Type::Uint8),
                    16 => Ok(Type::Int16),
                    32 => Ok(Type::Int32),
                    _ => Ok(Type::Int64),
                };
            } else if v.is_float_value() {
                return Ok(Type::Float64);
            } else if v.is_pointer_value() {
                return Ok(Type::Ptr(Box::new(Type::Uint8)));
            }
        }

        Ok(Type::Int64)
    }

    fn prepare_indirect_call(
        &mut self,
        callee: &Expr,
        arg_vals: &[BasicValueEnum<'ctx>],
        env_val: Option<PointerValue<'ctx>>,
    ) -> Result<(inkwell::types::FunctionType<'ctx>, u32, bool, Vec<BasicValueEnum<'ctx>>), String> {
        let sem_ty = self.expr_semantic_ty(callee);
        let fn_sig = match sem_ty.as_ref() {
            Some(Type::Fn { params, ret, is_cfun }) => {
                Some((params.clone(), (**ret).clone(), *is_cfun))
            }
            Some(Type::Ptr(inner)) => match &**inner {
                Type::Fn { params, ret, is_cfun } => {
                    Some((params.clone(), (**ret).clone(), *is_cfun))
                }
                _ => None,
            },
            _ => None,
        };

        if let Some((params, ret, is_cfun)) = fn_sig {
            let mut coerced = Vec::with_capacity(arg_vals.len() + 2);
            if !is_cfun && crate::region::RegionManager::is_regional_type(&ret) {
                let reg = self
                    .dest_region
                    .or(self.region_ptr)
                    .map_or_else(|| self.ptr_type().const_null().into(), Into::into);
                coerced.push(reg);
            }
            if !is_cfun {
                let env = env_val.unwrap_or_else(|| self.ptr_type().const_null());
                coerced.push(env.into());
            }
            for (i, &arg) in arg_vals.iter().enumerate() {
                if let Some(param) = params.get(i) {
                    if let Ok(target_ty) = self.lower_type(&param.ty) {
                        if let Ok(c) = self.coerce_type(arg, target_ty) {
                            coerced.push(c);
                            continue;
                        }
                    }
                }
                coerced.push(arg);
            }
            let param_types: Vec<BasicMetadataTypeEnum<'ctx>> =
                coerced.iter().map(|a| a.get_type().into()).collect();
            let is_void = ret == Type::Void;
            let fn_type = if is_void {
                self.context.void_type().fn_type(&param_types, false)
            } else {
                let ret_ty = self.lower_type(&ret)?;
                ret_ty.fn_type(&param_types, false)
            };
            let call_conv = if is_cfun {
                inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32
            } else {
                inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32
            };
            Ok((fn_type, call_conv, is_void, coerced))
        } else {
            let mut coerced = Vec::with_capacity(arg_vals.len() + 1);
            if let Some(env) = env_val {
                coerced.push(env.into());
            }
            coerced.extend_from_slice(arg_vals);
            let param_types: Vec<BasicMetadataTypeEnum<'ctx>> =
                coerced.iter().map(|a| a.get_type().into()).collect();
            let fn_type = self.context.i64_type().fn_type(&param_types, false);
            let call_conv = if env_val.is_some() {
                inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32
            } else {
                inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32
            };
            Ok((fn_type, call_conv, false, coerced))
        }
    }

    fn compile_indirect_call(
        &mut self,
        callee: &Expr,
        arg_vals: &[BasicValueEnum<'ctx>],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let callee_val = self.compile_expr(callee)?;
        let (fn_ptr, env_opt) = match callee_val {
            BasicValueEnum::PointerValue(pv) => (pv, None),
            BasicValueEnum::StructValue(sv) => {
                let fp = self
                    .builder
                    .build_extract_value(sv, 0, "closure.fn_ptr")
                    .map_err(|e| e.to_string())?
                    .into_pointer_value();
                let ep = self
                    .builder
                    .build_extract_value(sv, 1, "closure.env_ptr")
                    .map_err(|e| e.to_string())?
                    .into_pointer_value();
                (fp, Some(ep))
            }
            _ => return Err("expected function pointer for call".to_string()),
        };

        let (fn_type, call_conv, is_void, coerced_args) =
            self.prepare_indirect_call(callee, arg_vals, env_opt)?;

        let unwind_bb = self.unwind_dest.or_else(|| {
            if self.has_active_cleanups() {
                self.get_or_create_cleanup_unwind_bb().ok()
            } else {
                None
            }
        });

        if let Some(unwind) = unwind_bb {
            let personality = self.get_or_declare_personality();
            self.fn_val.set_personality_function(personality);

            let normal_bb = self.context.append_basic_block(self.fn_val, "ind_inv.cont");
            let inv = self
                .builder
                .build_indirect_invoke(fn_type, fn_ptr, &coerced_args, normal_bb, unwind, "ind_inv")
                .map_err(|e| e.to_string())?;
            inv.set_call_convention(call_conv);
            self.builder.position_at_end(normal_bb);
            if is_void {
                Ok(self.context.i64_type().const_zero().into())
            } else {
                let dummy = self.context.i64_type().const_zero().into();
                Ok(inv.try_as_basic_value().basic().unwrap_or(dummy))
            }
        } else {
            let meta_args: Vec<BasicMetadataValueEnum<'ctx>> =
                coerced_args.iter().map(|&a| a.into()).collect();
            let call = self
                .builder
                .build_indirect_call(fn_type, fn_ptr, &meta_args, "ind_call")
                .map_err(|e| e.to_string())?;
            call.set_call_convention(call_conv);
            if is_void {
                Ok(self.context.i64_type().const_zero().into())
            } else {
                let dummy = self.context.i64_type().const_zero().into();
                Ok(call.try_as_basic_value().basic().unwrap_or(dummy))
            }
        }
    }

    fn compile_class_or_virtual_method(
        &mut self,
        target_val: BasicValueEnum<'ctx>,
        method: &Ident,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let mut arg_vals: Vec<BasicValueEnum<'ctx>> = Vec::with_capacity(args.len() + 1);
        arg_vals.push(target_val);
        for arg in args {
            let val = self.compile_expr(arg)?;
            arg_vals.push(val);
        }
        self.resolve_and_emit_fn_call(&method.name, &arg_vals)
    }

    pub fn resolve_and_emit_c_fn_call(
        &mut self,
        c_symbol: &str,
        arg_vals: &[BasicValueEnum<'ctx>],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        self.emit_direct_fn_call(c_symbol, c_symbol, arg_vals)
    }

    pub fn resolve_and_emit_fn_call(
        &mut self,
        name: &str,
        arg_vals: &[BasicValueEnum<'ctx>],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let effective_name = self
            .registry
            .resolve_fun_symbol(name, arg_vals.len())
            .unwrap_or_else(|| name.to_string());
        self.emit_direct_fn_call(&effective_name, name, arg_vals)
    }

    fn emit_direct_fn_call(
        &mut self,
        effective_name: &str,
        sem_name: &str,
        arg_vals: &[BasicValueEnum<'ctx>],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let fn_val = match self.module.get_function(effective_name) {
            Some(f) => f,
            None => self.declare_external_function(effective_name, arg_vals),
        };

        let params = fn_val.get_params();

        let is_fastcc = fn_val.get_call_conventions()
            == inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32;
        let needs_dest = is_fastcc
            && params.len() == arg_vals.len() + 1
            && params
                .first()
                .is_some_and(|p| p.get_type().is_pointer_type());

        let mut coerced_args: Vec<BasicValueEnum<'ctx>> =
            Vec::with_capacity(arg_vals.len() + usize::from(needs_dest));
        if needs_dest {
            let reg = self
                .dest_region
                .or(self.region_ptr)
                .map_or_else(|| self.ptr_type().const_null().into(), Into::into);
            coerced_args.push(reg);
        }

        let fn_sem_params = self
            .registry
            .cfun_types
            .get(sem_name)
            .or_else(|| self.registry.fn_types.get(sem_name))
            .or_else(|| self.registry.fn_types.get(effective_name))
            .and_then(|t| match t {
                Type::Fn { params, .. } => Some(params.clone()),
                _ => None,
            });

        let offset = usize::from(needs_dest);
        for (i, &arg) in arg_vals.iter().enumerate() {
            if let Some(target_param) = params.get(i + offset) {
                let target_ty = target_param.get_type();
                let sem_param_ty = fn_sem_params
                    .as_ref()
                    .and_then(|pts| pts.get(i).map(|p| self.registry.canonicalize_type(&p.ty)));
                let coerced = match sem_param_ty {
                    Some(Type::Interface(iname)) if arg.is_pointer_value() => {
                        self.coerce_to_interface(arg.into_pointer_value(), &iname)?.into()
                    }
                    _ => self.coerce_type(arg, target_ty)?,
                };
                coerced_args.push(coerced);
            } else {
                coerced_args.push(arg);
            }
        }

        let unwind_bb = if self.is_abort_strategy() {
            None
        } else {
            self.unwind_dest.or_else(|| {
                if self.has_active_cleanups() {
                    self.get_or_create_cleanup_unwind_bb().ok()
                } else {
                    None
                }
            })
        };

        if let Some(unwind) = unwind_bb {
            let personality = self.get_or_declare_personality();
            self.fn_val.set_personality_function(personality);

            let normal_bb = self.context.append_basic_block(self.fn_val, "invoke.cont");
            let inv = self
                .builder
                .build_invoke(fn_val, &coerced_args, normal_bb, unwind, "inv")
                .map_err(|e| e.to_string())?;
            inv.set_call_convention(fn_val.get_call_conventions());

            self.builder.position_at_end(normal_bb);
            let dummy = self.context.i64_type().const_zero().into();
            Ok(inv.try_as_basic_value().basic().unwrap_or(dummy))
        } else {
            let meta_args: Vec<BasicMetadataValueEnum<'ctx>> =
                coerced_args.iter().map(|&a| a.into()).collect();
            let call = self
                .builder
                .build_call(fn_val, &meta_args, "")
                .map_err(|e| e.to_string())?;
            call.set_call_convention(fn_val.get_call_conventions());

            let dummy = self.context.i64_type().const_zero().into();
            Ok(call.try_as_basic_value().basic().unwrap_or(dummy))
        }
    }

    fn declare_external_function(
        &self,
        name: &str,
        arg_vals: &[BasicValueEnum<'ctx>],
    ) -> inkwell::values::FunctionValue<'ctx> {
        let is_cfun = name.starts_with("yukistd_")
            || name.starts_with("yukimacrosdk_")
            || name.starts_with("c_")
            || self.registry.cfun_types.contains_key(name)
            || self.registry.c_symbols.values().any(|s| s == name)
            || self
                .registry
                .fn_types
                .get(name)
                .is_some_and(|t| matches!(t, yukic_types::Type::Fn { is_cfun: true, .. }));
        let call_conv = if is_cfun {
            inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32
        } else {
            inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32
        };

        let ret_ty = self.registry.functions.get(name);
        let needs_dest = !is_cfun
            && ret_ty.is_some_and(|ret| crate::region::RegionManager::is_regional_type(ret));

        let mut param_types: Vec<BasicMetadataTypeEnum<'ctx>> =
            Vec::with_capacity(arg_vals.len() + usize::from(needs_dest));
        if needs_dest {
            param_types.push(self.ptr_type().into());
        }
        for a in arg_vals {
            param_types.push(a.get_type().into());
        }

        let fn_type = if let Some(ret) = ret_ty {
            let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
            let lowerer =
                crate::types::TypeLowerer::new(self.context, &td, self.target, self.registry);
            if let Ok(llvm_ret) = lowerer.lower_return_type(ret) {
                match llvm_ret {
                    Some(bty) => bty.fn_type(&param_types, false),
                    None => self.context.void_type().fn_type(&param_types, false),
                }
            } else {
                self.context.void_type().fn_type(&param_types, false)
            }
        } else {
            self.context.void_type().fn_type(&param_types, false)
        };
        let f = self.module.add_function(name, fn_type, Some(Linkage::External));
        f.set_call_conventions(call_conv);
        f
    }

    pub fn compile_super_method_call(
        &mut self,
        method: &Ident,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let Some(ref current_class) = self.current_class else {
            return Err("`super` can only be used inside a class method".to_string());
        };

        let layout = self
            .registry
            .classes
            .get(current_class)
            .ok_or_else(|| format!("class '{current_class}' not found in registry"))?;
        let Some(ref super_class) = layout.superclass else {
            return Err(format!("class '{current_class}' has no superclass"));
        };

        let mangled = crate::mangle::mangle_method_name(&method.name, args.len());
        let mut target_sym = None;
        let mut curr = Some(super_class.clone());
        while let Some(ancestor) = curr {
            let candidate = crate::mangle::method_symbol_name(&ancestor, &method.name, args.len());
            let qualified = format!("{ancestor}::{}", method.name);
            let qualified_mangled = format!("{ancestor}::{mangled}");
            if self.module.get_function(&candidate).is_some()
                || self.registry.functions.contains_key(&qualified)
                || self.registry.functions.contains_key(&qualified_mangled)
            {
                target_sym = Some(candidate);
                break;
            }
            if let Some(vt_layout) = self.vtable.get_layout(&ancestor) {
                if let Some(slot) = vt_layout
                    .slots
                    .iter()
                    .find(|s| s.method_name == mangled)
                {
                    target_sym = Some(slot.mangled_symbol.clone());
                    break;
                }
            }
            curr = self
                .registry
                .classes
                .get(&ancestor)
                .and_then(|l| l.superclass.clone());
        }

        let target_sym = target_sym.unwrap_or_else(|| {
            crate::mangle::method_symbol_name(super_class, &method.name, args.len())
        });
        let this_val = self.compile_ident(&Ident::new("this", method.span))?;

        let mut arg_vals = Vec::with_capacity(args.len() + 1);
        arg_vals.push(this_val);
        for arg in args {
            arg_vals.push(self.compile_expr(arg)?);
        }

        self.resolve_and_emit_fn_call(&target_sym, &arg_vals)
    }

    fn compile_class_instantiation(
        &mut self,
        class_name: &str,
        arg_vals: &[BasicValueEnum<'ctx>],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
        let lowerer =
            crate::types::TypeLowerer::new(self.context, &td, self.target, self.registry);
        let struct_ty = lowerer.class_instance_type(class_name)?;
        let size_bytes = td.get_abi_size(&struct_ty);

        let inst_ptr = if let Some(reg_ptr) = self.dest_region.or(self.region_ptr) {
            self.region.emit_fast_path_alloc(
                self.module,
                self.builder,
                self.fn_val,
                reg_ptr,
                size_bytes,
                8,
            )?
        } else {
            self.builder
                .build_alloca(struct_ty, "class_inst")
                .map_err(|e| e.to_string())?
        };

        // deterministic zero-init of all fields before the vtable and init()
        self.builder
            .build_store(inst_ptr, struct_ty.const_zero())
            .map_err(|e| e.to_string())?;

        let vt_name = format!("_Y_VTABLE_{class_name}");
        if let Some(vt_global) = self.module.get_global(&vt_name) {
            self.vtable
                .init_instance_vtable(self.builder, inst_ptr, vt_global)?;
        }

        let mut init_sym = None;
        let mut curr = Some(class_name.to_string());
        while let Some(ancestor) = curr {
            let candidate = format!("_Y_{ancestor}_init");
            let qualified = format!("{ancestor}::init");
            if self.module.get_function(&candidate).is_some()
                || self.registry.functions.contains_key(&qualified)
            {
                init_sym = Some(candidate);
                break;
            }
            curr = self
                .registry
                .classes
                .get(&ancestor)
                .and_then(|l| l.superclass.clone());
        }

        if let Some(sym) = init_sym {
            let mut init_args = Vec::with_capacity(arg_vals.len() + 1);
            init_args.push(inst_ptr.into());
            init_args.extend_from_slice(arg_vals);
            self.resolve_and_emit_fn_call(&sym, &init_args)?;
        }

        Ok(inst_ptr.into())
    }
}
