#![deny(unsafe_code)]

use inkwell::AddressSpace;
use inkwell::FloatPredicate;
use inkwell::IntPredicate;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, FunctionValue, IntValue, PointerValue, StructValue};
use yukic_ast::expr::{
    BinaryOp, CatchClause, Expr, ExprKind, FieldInit, LiteralValue, Pattern, PatternKind,
    StringPart, SwitchArm, UnaryOp,
};
use yukic_ast::ident::Ident;
use yukic_ast::stmt::Block;
use yukic_ast::types::TypeNode;
use yukic_types::Type;
use yukic_types::convert::ast_type_to_type;

use super::{BodyCompiler, LoopContext};
use crate::exception::{CleanupAction, ScopeKind};
use crate::registry::EnumVariantLayout;

impl<'ctx> BodyCompiler<'_, 'ctx> {
    pub fn compile_expr(&mut self, expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        match &expr.kind {
            ExprKind::Literal(lit) => Ok(self.compile_literal(lit)),
            ExprKind::Ident(ident) => self.compile_ident(ident),
            ExprKind::This | ExprKind::Super => self.compile_ident(&Ident::new("this", expr.span)),
            ExprKind::Path(idents) => {
                if idents.len() == 2 && idents[0].name == "extern" {
                    let fn_name = &idents[1].name;
                    let c_sym = self
                        .registry
                        .c_symbols
                        .get(fn_name)
                        .cloned()
                        .unwrap_or_else(|| fn_name.clone());
                    if let Some(fn_val) = self.module.get_function(&c_sym) {
                        return Ok(fn_val.as_global_value().as_pointer_value().into());
                    }
                }
                if idents.len() == 2 {
                    let enum_name = &idents[0].name;
                    let variant_name = &idents[1].name;
                    if let Some(enum_layout) = self.registry.enums.get(enum_name).cloned() {
                        if let Some(variant) = enum_layout.variants.iter().find(|v| v.name == *variant_name) {
                            return self.compile_enum_instantiation(enum_name, variant, &[]);
                        }
                    }
                }
                let name = idents.last().map_or("", |id| &id.name);
                self.compile_ident(&Ident::new(name, expr.span))
            }
            ExprKind::Call(callee, args) => self.compile_call(callee, args),
            ExprKind::ClassInit(callee, args) => self.compile_class_init(callee, args),
            ExprKind::MethodCall(target, method, args) => {
                self.compile_method_call(target, method, args)
            }
            ExprKind::Binary(op, left, right) => self.compile_binary(*op, left, right),
            ExprKind::Unary(op, operand) => self.compile_unary(*op, operand),
            ExprKind::Block(block) => {
                let val = self.compile_block(block)?;
                Ok(val.unwrap_or_else(|| self.context.i64_type().const_zero().into()))
            }
            ExprKind::RegionBlock(block) => {
                let val = self.compile_region_block(block)?;
                Ok(val.unwrap_or_else(|| self.context.i64_type().const_zero().into()))
            }
            ExprKind::If {
                cond,
                then_branch,
                elif_branches,
                else_branch,
            } => self.compile_if(cond, then_branch, elif_branches, else_branch.as_ref()),
            ExprKind::While { label, cond, body } => {
                self.compile_while(label.as_ref().map(|id| id.name.as_str()), cond, body)
            }
            ExprKind::Loop { label, body } => {
                self.compile_loop(label.as_ref().map(|id| id.name.as_str()), body)
            }
            ExprKind::For {
                label,
                index_var,
                item_var,
                iter,
                body,
            } => self.compile_for(
                label.as_ref().map(|id| id.name.as_str()),
                index_var.as_ref(),
                item_var,
                iter,
                body,
            ),
            ExprKind::IndexAccess(target, index) => self.compile_index_access(target, index),
            ExprKind::SliceAccess(target, start, end, inclusive) => {
                self.compile_slice_access(target, start.as_deref(), end.as_deref(), *inclusive)
            }
            ExprKind::Ternary(cond, then_e, else_e) => {
                self.compile_ternary(cond, then_e, else_e)
            }
            ExprKind::InterpolatedString(parts) => self.compile_interpolated_string(parts),
            ExprKind::Switch { target, arms } => self.compile_switch(target, arms),
            ExprKind::Sus {
                body,
                catches,
                finally_branch,
            } => self.compile_sus(body, catches, finally_branch.as_ref()),
            ExprKind::StructInit(name, fields) => self.compile_struct_init(name, fields),
            ExprKind::ArrayInit(elements) => self.compile_array_init(elements),
            ExprKind::ArrayAlloc(elem_ty, count) => self.compile_array_alloc(elem_ty, *count),
            ExprKind::TupleInit(elements) => self.compile_tuple_init(elements),
            ExprKind::FieldAccess(target, field) => self.compile_field_access(target, field),
            ExprKind::SafeCall(target, method, args) => {
                self.compile_safe_call(target, method, args)
            }
            ExprKind::PtrOf(ident) => self.compile_ptr_of(ident),
            ExprKind::PtrCast(target, ty_node) => self.compile_ptr_cast(target, ty_node),
            ExprKind::Ref { expr, .. } => self.compile_ref(expr),
            ExprKind::Is(target, ty_node) => self.compile_is(target, ty_node),
            ExprKind::Spread(inner) => self.compile_expr(inner),
            ExprKind::NamedArg(_, inner) => self.compile_expr(inner),
            ExprKind::AnonymousFun(fun) => self.compile_anonymous_fun(fun),
            ExprKind::Lambda(params, body) => self.compile_lambda(params, body),
            _ => Ok(self.context.i64_type().const_zero().into()),
        }
    }

    fn compile_literal(&mut self, lit: &LiteralValue) -> BasicValueEnum<'ctx> {
        match lit {
            LiteralValue::Int(v) => self
                .context
                .i64_type()
                .const_int((*v).cast_unsigned(), true)
                .into(),
            LiteralValue::UInt(v) => self.context.i64_type().const_int(*v, false).into(),
            LiteralValue::Float(v) => self.context.f64_type().const_float(*v).into(),
            LiteralValue::Bool(v) => {
                let int_val = u64::from(*v);
                self.context.bool_type().const_int(int_val, false).into()
            }
            LiteralValue::Char(v) => self
                .context
                .i32_type()
                .const_int(u64::from(*v), false)
                .into(),
            LiteralValue::Null => self.ptr_type().const_null().into(),
            LiteralValue::String(s) => self.rodata.emit_string(self.module, s).into(),
            LiteralValue::Bytes(b) => self.rodata.emit_bytes(self.module, b).into(),
        }
    }

    pub(crate) fn compile_ident(&self, ident: &Ident) -> Result<BasicValueEnum<'ctx>, String> {
        if let Some(var) = self.lookup_local(&ident.name) {
            return self
                .builder
                .build_load(var.ty, var.ptr, &ident.name)
                .map_err(|e| e.to_string());
        }
        if let Some(global) = self.module.get_global(&ident.name) {
            let bty = BasicTypeEnum::try_from(global.get_value_type())
                .map_err(|()| format!("invalid global type for '{}'", ident.name))?;
            return self
                .builder
                .build_load(bty, global.as_pointer_value(), &ident.name)
                .map_err(|e| e.to_string());
        }
        let effective_name = self
            .registry
            .resolve_fun_symbol(&ident.name, 0)
            .unwrap_or_else(|| ident.name.clone());
        if let Some(func) = self.module.get_function(&effective_name) {
            return Ok(func.as_global_value().as_pointer_value().into());
        }
        Err(format!("undefined identifier '{}'", ident.name))
    }

    fn compile_binary(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if op == BinaryOp::NullCoalesce {
            return self.compile_null_coalesce(left, right);
        }
        if op == BinaryOp::And {
            return self.compile_short_circuit_and(left, right);
        }
        if op == BinaryOp::Or {
            return self.compile_short_circuit_or(left, right);
        }
        let lhs = self.compile_expr(left)?;
        let rhs = self.compile_expr(right)?;
        if let Some(res) = self.try_compile_operator_binary(op, left, lhs, rhs)? {
            return Ok(res);
        }
        let left_ty = self.deduce_expr_type(left, Some(lhs)).ok();
        let right_ty = self.deduce_expr_type(right, Some(rhs)).ok();
        let is_unsigned = left_ty.as_ref().map(|t| t.is_unsigned_integer()).unwrap_or(false)
            || right_ty.as_ref().map(|t| t.is_unsigned_integer()).unwrap_or(false);
        self.compile_binary_values_typed(op, lhs, rhs, is_unsigned)
    }

    fn try_compile_operator_binary(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let left_ty = self.expr_semantic_ty(left);
        let Some(Type::Class(class_name)) = left_ty else {
            return Ok(None);
        };
        let op_name = crate::mangle::binary_op_to_symbol(op);
        if self.find_class_method(&class_name, op_name, 1).is_some() {
            let res = self.compile_class_instance_method_call_values(&class_name, lhs, op_name, &[rhs])?;
            return Ok(Some(res));
        }
        if op == BinaryOp::NotEq && self.find_class_method(&class_name, "==", 1).is_some() {
            let eq_val = self.compile_class_instance_method_call_values(&class_name, lhs, "==", &[rhs])?;
            let iv = eq_val.into_int_value();
            let ne_val = self.builder.build_not(iv, "op_ne").map_err(|e| e.to_string())?;
            return Ok(Some(ne_val.into()));
        }
        Ok(None)
    }

    fn compile_null_coalesce(
        &mut self,
        left: &Expr,
        right: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let lhs_val = self.compile_expr(left)?;

        if lhs_val.is_pointer_value() {
            let pv = lhs_val.into_pointer_value();
            let is_not_null = self
                .builder
                .build_int_compare(
                    IntPredicate::NE,
                    pv,
                    self.ptr_type().const_null(),
                    "coal.not_null",
                )
                .map_err(|e| e.to_string())?;

            let lhs_bb = self.builder.get_insert_block().unwrap();
            let rhs_bb = self.context.append_basic_block(self.fn_val, "coal.rhs");
            let merge_bb = self.context.append_basic_block(self.fn_val, "coal.merge");

            self.builder
                .build_conditional_branch(is_not_null, merge_bb, rhs_bb)
                .map_err(|e| e.to_string())?;

            self.builder.position_at_end(rhs_bb);
            let rhs_val = self.compile_expr(right)?;
            let rhs_end_bb = self.builder.get_insert_block().unwrap();
            self.builder
                .build_unconditional_branch(merge_bb)
                .map_err(|e| e.to_string())?;

            self.builder.position_at_end(merge_bb);
            let coerced_rhs = self.coerce_type(rhs_val, lhs_val.get_type())?;
            let phi = self
                .builder
                .build_phi(lhs_val.get_type(), "coal.res")
                .map_err(|e| e.to_string())?;
            phi.add_incoming(&[(&lhs_val, lhs_bb), (&coerced_rhs, rhs_end_bb)]);
            return Ok(phi.as_basic_value());
        }

        if let BasicValueEnum::StructValue(sv) = lhs_val {
            if sv.get_type().count_fields() == 2
                && sv.get_type().get_field_type_at_index(0) == Some(self.context.bool_type().into())
            {
                let is_some = self
                    .builder
                    .build_extract_value(sv, 0, "coal.is_some")
                    .map_err(|e| e.to_string())?
                    .into_int_value();
                let unwrapped_lhs = self
                    .builder
                    .build_extract_value(sv, 1, "coal.unwrapped")
                    .map_err(|e| e.to_string())?;

                let lhs_bb = self.builder.get_insert_block().unwrap();
                let rhs_bb = self.context.append_basic_block(self.fn_val, "coal.rhs");
                let merge_bb = self.context.append_basic_block(self.fn_val, "coal.merge");

                self.builder
                    .build_conditional_branch(is_some, merge_bb, rhs_bb)
                    .map_err(|e| e.to_string())?;

                self.builder.position_at_end(rhs_bb);
                let rhs_val = self.compile_expr(right)?;
                let rhs_end_bb = self.builder.get_insert_block().unwrap();
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|e| e.to_string())?;

                self.builder.position_at_end(merge_bb);

                if rhs_val.get_type() == BasicTypeEnum::StructType(sv.get_type()) {
                    let phi = self
                        .builder
                        .build_phi(sv.get_type(), "coal.res")
                        .map_err(|e| e.to_string())?;
                    phi.add_incoming(&[(&lhs_val, lhs_bb), (&rhs_val, rhs_end_bb)]);
                    return Ok(phi.as_basic_value());
                }

                let target_ty = unwrapped_lhs.get_type();
                let coerced_rhs = self.coerce_type(rhs_val, target_ty)?;
                let phi = self
                    .builder
                    .build_phi(target_ty, "coal.res")
                    .map_err(|e| e.to_string())?;
                phi.add_incoming(&[(&unwrapped_lhs, lhs_bb), (&coerced_rhs, rhs_end_bb)]);
                return Ok(phi.as_basic_value());
            }
        }

        Ok(lhs_val)
    }

    fn compile_short_circuit_and(
        &mut self,
        left: &Expr,
        right: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let lhs_val = self.compile_expr(left)?;
        if self.is_terminated() {
            return Ok(lhs_val);
        }
        let lhs_bool = self.to_bool(lhs_val)?;
        let lhs_end_bb = self.builder.get_insert_block().unwrap();

        let rhs_bb = self.context.append_basic_block(self.fn_val, "and.rhs");
        let merge_bb = self.context.append_basic_block(self.fn_val, "and.merge");

        self.builder
            .build_conditional_branch(lhs_bool, rhs_bb, merge_bb)
            .map_err(|e| e.to_string())?;

        self.builder.position_at_end(rhs_bb);
        let rhs_val = self.compile_expr(right)?;
        let rhs_end_bb = self.builder.get_insert_block().unwrap();

        let bool_ty = self.context.bool_type();
        if self.is_terminated() {
            self.builder.position_at_end(merge_bb);
            return Ok(bool_ty.const_zero().into());
        }

        let rhs_bool = self.to_bool(rhs_val)?;
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| e.to_string())?;

        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(bool_ty, "and.res")
            .map_err(|e| e.to_string())?;
        let false_val = bool_ty.const_zero();
        phi.add_incoming(&[(&false_val, lhs_end_bb), (&rhs_bool, rhs_end_bb)]);
        Ok(phi.as_basic_value())
    }

    fn compile_short_circuit_or(
        &mut self,
        left: &Expr,
        right: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let lhs_val = self.compile_expr(left)?;
        if self.is_terminated() {
            return Ok(lhs_val);
        }
        let lhs_bool = self.to_bool(lhs_val)?;
        let lhs_end_bb = self.builder.get_insert_block().unwrap();

        let rhs_bb = self.context.append_basic_block(self.fn_val, "or.rhs");
        let merge_bb = self.context.append_basic_block(self.fn_val, "or.merge");

        self.builder
            .build_conditional_branch(lhs_bool, merge_bb, rhs_bb)
            .map_err(|e| e.to_string())?;

        self.builder.position_at_end(rhs_bb);
        let rhs_val = self.compile_expr(right)?;
        let rhs_end_bb = self.builder.get_insert_block().unwrap();

        let bool_ty = self.context.bool_type();
        if self.is_terminated() {
            self.builder.position_at_end(merge_bb);
            return Ok(bool_ty.const_int(1, false).into());
        }

        let rhs_bool = self.to_bool(rhs_val)?;
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| e.to_string())?;

        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(bool_ty, "or.res")
            .map_err(|e| e.to_string())?;
        let true_val = bool_ty.const_int(1, false);
        phi.add_incoming(&[(&true_val, lhs_end_bb), (&rhs_bool, rhs_end_bb)]);
        Ok(phi.as_basic_value())
    }

    pub fn compile_binary_values(
        &mut self,
        op: BinaryOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        self.compile_binary_values_typed(op, lhs, rhs, false)
    }

    pub fn compile_binary_values_typed(
        &mut self,
        op: BinaryOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
        is_unsigned: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match (lhs, rhs) {
            (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                let (li, ri) = self.align_int_operands(l, r, is_unsigned)?;
                self.compile_int_binary(op, li, ri, is_unsigned)
            }
            (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                self.compile_float_binary(op, l, r)
            }
            (BasicValueEnum::PointerValue(l), BasicValueEnum::PointerValue(r)) => {
                let pred = match op {
                    BinaryOp::Eq => IntPredicate::EQ,
                    BinaryOp::NotEq => IntPredicate::NE,
                    _ => return Err("unsupported binary operation on pointers".to_string()),
                };
                self.builder
                    .build_int_compare(pred, l, r, "ptr_cmp")
                    .map_err(|e| e.to_string())
                    .map(Into::into)
            }
            (BasicValueEnum::StructValue(s), BasicValueEnum::PointerValue(p)) if p.is_null() => {
                self.compile_nullable_struct_null_cmp(s, op)
            }
            (BasicValueEnum::PointerValue(p), BasicValueEnum::StructValue(s)) if p.is_null() => {
                self.compile_nullable_struct_null_cmp(s, op)
            }
            (BasicValueEnum::StructValue(s), BasicValueEnum::IntValue(r)) => {
                self.compile_nullable_struct_scalar_cmp(s, r.into(), op)
            }
            (BasicValueEnum::IntValue(l), BasicValueEnum::StructValue(s)) => {
                self.compile_nullable_struct_scalar_cmp(s, l.into(), op)
            }
            (BasicValueEnum::StructValue(l), BasicValueEnum::StructValue(r)) => {
                let l_st = l.get_type();
                let r_st = r.get_type();
                if self.is_string_struct(l_st) && self.is_string_struct(r_st) {
                    if op == BinaryOp::Add {
                        return self.compile_string_concat(l, r);
                    }
                    if op == BinaryOp::Eq || op == BinaryOp::NotEq {
                        return self.compile_string_equality(l, r, op);
                    }
                    if matches!(op, BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq) {
                        return self.compile_string_ordering(l, r, op);
                    }
                }
                if op == BinaryOp::Add {
                    self.compile_string_concat(l, r)
                } else if op == BinaryOp::Eq || op == BinaryOp::NotEq {
                    self.compile_struct_equality(l, r, op)
                } else {
                    Err("unsupported binary operation on structs".to_string())
                }
            }
            _ => Err(format!("type mismatch in binary operation {:?}: {:?} vs {:?}", op, lhs, rhs)),
        }
    }

    fn compile_nullable_struct_null_cmp(
        &mut self,
        s: StructValue<'ctx>,
        op: BinaryOp,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if s.get_type().count_fields() == 2
            && s.get_type().get_field_type_at_index(0) == Some(self.context.bool_type().into())
        {
            let is_some = self
                .builder
                .build_extract_value(s, 0, "is_some")
                .map_err(|e| e.to_string())?
                .into_int_value();
            match op {
                BinaryOp::Eq => {
                    let zero = self.context.bool_type().const_zero();
                    self.builder
                        .build_int_compare(IntPredicate::EQ, is_some, zero, "is_null")
                        .map_err(|e| e.to_string())
                        .map(Into::into)
                }
                BinaryOp::NotEq => Ok(is_some.into()),
                _ => Err("unsupported comparison between nullable value and null".to_string()),
            }
        } else {
            Err("unsupported comparison on struct".to_string())
        }
    }

    fn compile_nullable_struct_scalar_cmp(
        &mut self,
        s: StructValue<'ctx>,
        scalar: BasicValueEnum<'ctx>,
        op: BinaryOp,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if s.get_type().count_fields() == 2
            && s.get_type().get_field_type_at_index(0) == Some(self.context.bool_type().into())
        {
            let is_some = self
                .builder
                .build_extract_value(s, 0, "is_some")
                .map_err(|e| e.to_string())?
                .into_int_value();
            let val = self
                .builder
                .build_extract_value(s, 1, "val")
                .map_err(|e| e.to_string())?;
            let val_cmp = self.compile_binary_values(op, val, scalar)?.into_int_value();
            match op {
                BinaryOp::Eq => self
                    .builder
                    .build_and(is_some, val_cmp, "opt_eq")
                    .map_err(|e| e.to_string())
                    .map(Into::into),
                BinaryOp::NotEq => {
                    let not_some = self
                        .builder
                        .build_not(is_some, "not_some")
                        .map_err(|e| e.to_string())?;
                    self.builder
                        .build_or(not_some, val_cmp, "opt_ne")
                        .map_err(|e| e.to_string())
                        .map(Into::into)
                }
                _ => Err("unsupported comparison between nullable and scalar".to_string()),
            }
        } else {
            Err(format!("type mismatch in nullable struct scalar cmp: {:?} vs {:?}", s, scalar))
        }
    }

    fn is_string_struct(&self, st: inkwell::types::StructType<'ctx>) -> bool {
        st == self.intrinsic.string_type()
            || (st.count_fields() == 2
                && st.get_field_type_at_index(0) == Some(self.ptr_type().into())
                && st.get_field_type_at_index(1) == Some(self.context.i64_type().into()))
    }

    fn is_nullable_struct(&self, st: inkwell::types::StructType<'ctx>) -> bool {
        st.count_fields() == 2
            && st.get_field_type_at_index(0) == Some(self.context.bool_type().into())
    }

    fn get_or_declare_memcmp(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("memcmp") {
            return f;
        }
        let i32_type = self.context.i32_type();
        let ptr_type = self.ptr_type();
        let i64_type = self.context.i64_type();
        let fn_type = i32_type.fn_type(&[ptr_type.into(), ptr_type.into(), i64_type.into()], false);
        self.module.add_function("memcmp", fn_type, None)
    }

    fn get_or_declare_yukistd_str_cmp(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("yukistd_str_cmp") {
            return f;
        }
        let i32_type = self.context.i32_type();
        let ptr_type = self.ptr_type();
        let i64_type = self.context.i64_type();
        let fn_type = i32_type.fn_type(
            &[ptr_type.into(), i64_type.into(), ptr_type.into(), i64_type.into()],
            false,
        );
        self.module.add_function("yukistd_str_cmp", fn_type, None)
    }

    fn compile_string_ordering(
        &mut self,
        l: StructValue<'ctx>,
        r: StructValue<'ctx>,
        op: BinaryOp,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let b = self.builder;
        let l_ptr = self.intrinsic.emit_slice_as_ptr(b, l.into())?;
        let l_len = self.intrinsic.emit_slice_len(b, l.into())?;
        let r_ptr = self.intrinsic.emit_slice_as_ptr(b, r.into())?;
        let r_len = self.intrinsic.emit_slice_len(b, r.into())?;

        let cmp_fn = self.get_or_declare_yukistd_str_cmp();
        let call = b
            .build_call(cmp_fn, &[l_ptr.into(), l_len.into(), r_ptr.into(), r_len.into()], "str_cmp_call")
            .map_err(|e| e.to_string())?;
        let res = call
            .try_as_basic_value()
            .basic()
            .ok_or("expected return value from yukistd_str_cmp")?
            .into_int_value();

        let zero = self.context.i32_type().const_zero();
        let pred = match op {
            BinaryOp::Lt => IntPredicate::SLT,
            BinaryOp::LtEq => IntPredicate::SLE,
            BinaryOp::Gt => IntPredicate::SGT,
            BinaryOp::GtEq => IntPredicate::SGE,
            _ => return Err("invalid string ordering operator".to_string()),
        };
        let cmp = b
            .build_int_compare(pred, res, zero, "str_ord")
            .map_err(|e| e.to_string())?;
        Ok(cmp.into())
    }

    fn compile_string_equality(
        &mut self,
        l: StructValue<'ctx>,
        r: StructValue<'ctx>,
        op: BinaryOp,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let b = self.builder;
        let l_ptr = self.intrinsic.emit_slice_as_ptr(b, l.into())?;
        let l_len = self.intrinsic.emit_slice_len(b, l.into())?;
        let r_ptr = self.intrinsic.emit_slice_as_ptr(b, r.into())?;
        let r_len = self.intrinsic.emit_slice_len(b, r.into())?;

        // 1. Length equality check
        let len_eq = b
            .build_int_compare(IntPredicate::EQ, l_len, r_len, "str_len_eq")
            .map_err(|e| e.to_string())?;

        let entry_bb = self.builder.get_insert_block().unwrap();
        let cmp_bytes_bb = self.context.append_basic_block(self.fn_val, "str_cmp_bytes");
        let merge_bb = self.context.append_basic_block(self.fn_val, "str_eq_merge");

        b.build_conditional_branch(len_eq, cmp_bytes_bb, merge_bb)
            .map_err(|e| e.to_string())?;

        // 2. In cmp_bytes_bb: identical buffer or empty string check
        b.position_at_end(cmp_bytes_bb);
        let zero_i64 = self.context.i64_type().const_zero();
        let is_empty = b
            .build_int_compare(IntPredicate::EQ, l_len, zero_i64, "str_is_empty")
            .map_err(|e| e.to_string())?;
        let ptr_eq = b
            .build_int_compare(IntPredicate::EQ, l_ptr, r_ptr, "str_ptr_eq")
            .map_err(|e| e.to_string())?;
        let same_buf = b.build_or(is_empty, ptr_eq, "str_same_buf").map_err(|e| e.to_string())?;

        let same_buf_bb = self.context.append_basic_block(self.fn_val, "str_same_buf_bb");
        let call_memcmp_bb = self.context.append_basic_block(self.fn_val, "str_call_memcmp");

        b.build_conditional_branch(same_buf, same_buf_bb, call_memcmp_bb)
            .map_err(|e| e.to_string())?;

        // same_buf_bb: true
        b.position_at_end(same_buf_bb);
        let true_val = self.context.bool_type().const_int(1, false);
        b.build_unconditional_branch(merge_bb).map_err(|e| e.to_string())?;
        let same_buf_end = self.builder.get_insert_block().unwrap();

        // call_memcmp_bb: call libc memcmp
        b.position_at_end(call_memcmp_bb);
        let memcmp_fn = self.get_or_declare_memcmp();
        let call = b
            .build_call(memcmp_fn, &[l_ptr.into(), r_ptr.into(), l_len.into()], "memcmp_call")
            .map_err(|e| e.to_string())?;
        let memcmp_res = call
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let zero_i32 = self.context.i32_type().const_zero();
        let bytes_match = b
            .build_int_compare(IntPredicate::EQ, memcmp_res, zero_i32, "bytes_match")
            .map_err(|e| e.to_string())?;
        b.build_unconditional_branch(merge_bb).map_err(|e| e.to_string())?;
        let call_memcmp_end = self.builder.get_insert_block().unwrap();

        // merge_bb: combine results with phi
        b.position_at_end(merge_bb);
        let phi = b
            .build_phi(self.context.bool_type(), "str_eq_res")
            .map_err(|e| e.to_string())?;

        let false_val = self.context.bool_type().const_zero();
        phi.add_incoming(&[
            (&false_val, entry_bb),
            (&true_val, same_buf_end),
            (&bytes_match, call_memcmp_end),
        ]);

        let is_eq = phi.as_basic_value().into_int_value();
        if op == BinaryOp::Eq {
            Ok(is_eq.into())
        } else {
            b.build_not(is_eq, "str_ne_res")
                .map_err(|e| e.to_string())
                .map(Into::into)
        }
    }

    fn compile_struct_equality(
        &mut self,
        l: StructValue<'ctx>,
        r: StructValue<'ctx>,
        op: BinaryOp,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let l_st = l.get_type();
        let r_st = r.get_type();

        if self.is_string_struct(l_st) && self.is_string_struct(r_st) {
            return self.compile_string_equality(l, r, op);
        }

        if self.is_nullable_struct(l_st) && self.is_nullable_struct(r_st) {
            let l_is_some = self
                .builder
                .build_extract_value(l, 0, "l_is_some")
                .map_err(|e| e.to_string())?
                .into_int_value();
            let r_is_some = self
                .builder
                .build_extract_value(r, 0, "r_is_some")
                .map_err(|e| e.to_string())?
                .into_int_value();
            let l_val = self
                .builder
                .build_extract_value(l, 1, "l_val")
                .map_err(|e| e.to_string())?;
            let r_val = self
                .builder
                .build_extract_value(r, 1, "r_val")
                .map_err(|e| e.to_string())?;

            let both_some = self
                .builder
                .build_and(l_is_some, r_is_some, "both_some")
                .map_err(|e| e.to_string())?;
            let val_cmp = self.compile_binary_values(BinaryOp::Eq, l_val, r_val)?.into_int_value();
            let some_and_eq = self
                .builder
                .build_and(both_some, val_cmp, "some_eq")
                .map_err(|e| e.to_string())?;

            let l_null = self
                .builder
                .build_not(l_is_some, "l_null")
                .map_err(|e| e.to_string())?;
            let r_null = self
                .builder
                .build_not(r_is_some, "r_null")
                .map_err(|e| e.to_string())?;
            let both_null = self
                .builder
                .build_and(l_null, r_null, "both_null")
                .map_err(|e| e.to_string())?;

            let is_eq = self
                .builder
                .build_or(both_null, some_and_eq, "opt_eq")
                .map_err(|e| e.to_string())?;

            return if op == BinaryOp::Eq {
                Ok(is_eq.into())
            } else {
                self.builder
                    .build_not(is_eq, "opt_ne")
                    .map_err(|e| e.to_string())
                    .map(Into::into)
            };
        }

        if self.is_nullable_struct(l_st) {
            let is_some = self
                .builder
                .build_extract_value(l, 0, "is_some")
                .map_err(|e| e.to_string())?
                .into_int_value();
            let val = self
                .builder
                .build_extract_value(l, 1, "val")
                .map_err(|e| e.to_string())?;
            let val_cmp = self.compile_binary_values(op, val, r.into())?.into_int_value();
            return match op {
                BinaryOp::Eq => self
                    .builder
                    .build_and(is_some, val_cmp, "opt_eq")
                    .map_err(|e| e.to_string())
                    .map(Into::into),
                BinaryOp::NotEq => {
                    let not_some = self
                        .builder
                        .build_not(is_some, "not_some")
                        .map_err(|e| e.to_string())?;
                    self.builder
                        .build_or(not_some, val_cmp, "opt_ne")
                        .map_err(|e| e.to_string())
                        .map(Into::into)
                }
                _ => Err("unsupported comparison between nullable and value".to_string()),
            };
        }

        if self.is_nullable_struct(r_st) {
            let is_some = self
                .builder
                .build_extract_value(r, 0, "is_some")
                .map_err(|e| e.to_string())?
                .into_int_value();
            let val = self
                .builder
                .build_extract_value(r, 1, "val")
                .map_err(|e| e.to_string())?;
            let val_cmp = self.compile_binary_values(op, l.into(), val)?.into_int_value();
            return match op {
                BinaryOp::Eq => self
                    .builder
                    .build_and(is_some, val_cmp, "opt_eq")
                    .map_err(|e| e.to_string())
                    .map(Into::into),
                BinaryOp::NotEq => {
                    let not_some = self
                        .builder
                        .build_not(is_some, "not_some")
                        .map_err(|e| e.to_string())?;
                    self.builder
                        .build_or(not_some, val_cmp, "opt_ne")
                        .map_err(|e| e.to_string())
                        .map(Into::into)
                }
                _ => Err("unsupported comparison between value and nullable".to_string()),
            };
        }

        Err("unsupported equality comparison between structs".to_string())
    }

    fn align_int_operands(
        &self,
        l: IntValue<'ctx>,
        r: IntValue<'ctx>,
        is_unsigned: bool,
    ) -> Result<(IntValue<'ctx>, IntValue<'ctx>), String> {
        let lw = l.get_type().get_bit_width();
        let rw = r.get_type().get_bit_width();
        if lw == rw {
            return Ok((l, r));
        }
        if lw > rw {
            let ext = if is_unsigned {
                self.builder
                    .build_int_z_extend(r, l.get_type(), "zext")
                    .map_err(|e| e.to_string())?
            } else {
                self.builder
                    .build_int_s_extend(r, l.get_type(), "sext")
                    .map_err(|e| e.to_string())?
            };
            Ok((l, ext))
        } else {
            let ext = if is_unsigned {
                self.builder
                    .build_int_z_extend(l, r.get_type(), "zext")
                    .map_err(|e| e.to_string())?
            } else {
                self.builder
                    .build_int_s_extend(l, r.get_type(), "sext")
                    .map_err(|e| e.to_string())?
            };
            Ok((ext, r))
        }
    }

    fn compile_int_binary(
        &self,
        op: BinaryOp,
        l: IntValue<'ctx>,
        r: IntValue<'ctx>,
        is_unsigned: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let b = self.builder;
        let val: BasicValueEnum<'ctx> = match op {
            BinaryOp::Add => b.build_int_add(l, r, "add").map_err(|e| e.to_string())?.into(),
            BinaryOp::Sub => b.build_int_sub(l, r, "sub").map_err(|e| e.to_string())?.into(),
            BinaryOp::Mul => b.build_int_mul(l, r, "mul").map_err(|e| e.to_string())?.into(),
            BinaryOp::Div => {
                if is_unsigned {
                    b.build_int_unsigned_div(l, r, "udiv").map_err(|e| e.to_string())?.into()
                } else {
                    b.build_int_signed_div(l, r, "sdiv").map_err(|e| e.to_string())?.into()
                }
            }
            BinaryOp::Rem => {
                if is_unsigned {
                    b.build_int_unsigned_rem(l, r, "urem").map_err(|e| e.to_string())?.into()
                } else {
                    b.build_int_signed_rem(l, r, "srem").map_err(|e| e.to_string())?.into()
                }
            }
            BinaryOp::BitAnd => b.build_and(l, r, "and").map_err(|e| e.to_string())?.into(),
            BinaryOp::BitOr => b.build_or(l, r, "or").map_err(|e| e.to_string())?.into(),
            BinaryOp::BitXor => b.build_xor(l, r, "xor").map_err(|e| e.to_string())?.into(),
            BinaryOp::Shl => b.build_left_shift(l, r, "shl").map_err(|e| e.to_string())?.into(),
            BinaryOp::Shr => {
                let sign_extend = !is_unsigned;
                b.build_right_shift(l, r, sign_extend, if is_unsigned { "lshr" } else { "shr" })
                    .map_err(|e| e.to_string())?
                    .into()
            }
            BinaryOp::Eq => b
                .build_int_compare(IntPredicate::EQ, l, r, "eq")
                .map_err(|e| e.to_string())?
                .into(),
            BinaryOp::NotEq => b
                .build_int_compare(IntPredicate::NE, l, r, "ne")
                .map_err(|e| e.to_string())?
                .into(),
            BinaryOp::Lt => {
                let pred = if is_unsigned { IntPredicate::ULT } else { IntPredicate::SLT };
                b.build_int_compare(pred, l, r, "lt").map_err(|e| e.to_string())?.into()
            }
            BinaryOp::LtEq => {
                let pred = if is_unsigned { IntPredicate::ULE } else { IntPredicate::SLE };
                b.build_int_compare(pred, l, r, "le").map_err(|e| e.to_string())?.into()
            }
            BinaryOp::Gt => {
                let pred = if is_unsigned { IntPredicate::UGT } else { IntPredicate::SGT };
                b.build_int_compare(pred, l, r, "gt").map_err(|e| e.to_string())?.into()
            }
            BinaryOp::GtEq => {
                let pred = if is_unsigned { IntPredicate::UGE } else { IntPredicate::SGE };
                b.build_int_compare(pred, l, r, "ge").map_err(|e| e.to_string())?.into()
            }
            BinaryOp::Pow => {
                let ipow_fn = self.get_or_declare_ipow64();
                let i64_ty = self.context.i64_type();
                let l_i64 = b
                    .build_int_cast(l, i64_ty, "l_ext")
                    .map_err(|e| e.to_string())?;
                let r_i64 = b
                    .build_int_cast(r, i64_ty, "r_ext")
                    .map_err(|e| e.to_string())?;
                let call = b
                    .build_call(ipow_fn, &[l_i64.into(), r_i64.into()], "ipow")
                    .map_err(|e| e.to_string())?;
                let res_i64 = call.try_as_basic_value().basic().unwrap().into_int_value();
                b.build_int_cast(res_i64, l.get_type(), "ipow_cast")
                    .map_err(|e| e.to_string())?
                    .into()
            }
            _ => return Err(format!("unsupported integer binary operation {op:?}")),
        };
        Ok(val)
    }

    fn compile_float_binary(
        &self,
        op: BinaryOp,
        l: inkwell::values::FloatValue<'ctx>,
        r: inkwell::values::FloatValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let b = self.builder;
        let val: BasicValueEnum<'ctx> = match op {
            BinaryOp::Add => b.build_float_add(l, r, "fadd").map_err(|e| e.to_string())?.into(),
            BinaryOp::Sub => b.build_float_sub(l, r, "fsub").map_err(|e| e.to_string())?.into(),
            BinaryOp::Mul => b.build_float_mul(l, r, "fmul").map_err(|e| e.to_string())?.into(),
            BinaryOp::Div => b.build_float_div(l, r, "fdiv").map_err(|e| e.to_string())?.into(),
            BinaryOp::Rem => b.build_float_rem(l, r, "frem").map_err(|e| e.to_string())?.into(),
            BinaryOp::Eq => b
                .build_float_compare(FloatPredicate::OEQ, l, r, "feq")
                .map_err(|e| e.to_string())?
                .into(),
            BinaryOp::NotEq => b
                .build_float_compare(FloatPredicate::ONE, l, r, "fne")
                .map_err(|e| e.to_string())?
                .into(),
            BinaryOp::Lt => b
                .build_float_compare(FloatPredicate::OLT, l, r, "flt")
                .map_err(|e| e.to_string())?
                .into(),
            BinaryOp::LtEq => b
                .build_float_compare(FloatPredicate::OLE, l, r, "fle")
                .map_err(|e| e.to_string())?
                .into(),
            BinaryOp::Gt => b
                .build_float_compare(FloatPredicate::OGT, l, r, "fgt")
                .map_err(|e| e.to_string())?
                .into(),
            BinaryOp::GtEq => b
                .build_float_compare(FloatPredicate::OGE, l, r, "fge")
                .map_err(|e| e.to_string())?
                .into(),
            BinaryOp::Pow => {
                let pow_fn = if l.get_type() == self.context.f32_type() {
                    self.get_or_declare_pow_f32()
                } else {
                    self.get_or_declare_pow_f64()
                };
                let call = b
                    .build_call(pow_fn, &[l.into(), r.into()], "fpow")
                    .map_err(|e| e.to_string())?;
                call.try_as_basic_value().basic().unwrap()
            }
            _ => return Err(format!("unsupported float binary operation {op:?}")),
        };
        Ok(val)
    }

    fn get_or_declare_pow_f64(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("llvm.pow.f64") {
            return f;
        }
        let f64_ty = self.context.f64_type();
        let fn_ty = f64_ty.fn_type(&[f64_ty.into(), f64_ty.into()], false);
        self.module.add_function("llvm.pow.f64", fn_ty, None)
    }

    fn get_or_declare_pow_f32(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("llvm.pow.f32") {
            return f;
        }
        let f32_ty = self.context.f32_type();
        let fn_ty = f32_ty.fn_type(&[f32_ty.into(), f32_ty.into()], false);
        self.module.add_function("llvm.pow.f32", fn_ty, None)
    }

    fn get_or_declare_ipow64(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("_Y_ipow64") {
            return f;
        }
        let i64_ty = self.context.i64_type();
        let fn_ty = i64_ty.fn_type(&[i64_ty.into(), i64_ty.into()], false);
        let f = self.module.add_function("_Y_ipow64", fn_ty, Some(inkwell::module::Linkage::Internal));
        let cur_bb = self.builder.get_insert_block();
        self.emit_ipow64_body(f);
        if let Some(bb) = cur_bb {
            self.builder.position_at_end(bb);
        }
        f
    }

    fn emit_ipow64_body(&self, f: FunctionValue<'ctx>) {
        let i64_ty = self.context.i64_type();
        let entry_bb = self.context.append_basic_block(f, "entry");
        let loop_bb = self.context.append_basic_block(f, "loop");
        let exit_bb = self.context.append_basic_block(f, "exit");

        self.builder.position_at_end(entry_bb);
        let base_arg = f.get_nth_param(0).unwrap().into_int_value();
        let exp_arg = f.get_nth_param(1).unwrap().into_int_value();
        let zero = i64_ty.const_zero();
        let one = i64_ty.const_int(1, false);

        let is_zero = self.builder.build_int_compare(IntPredicate::EQ, exp_arg, zero, "is_zero").unwrap();
        let non_pos = self.builder.build_select(is_zero, one, zero, "non_pos").unwrap().into_int_value();
        let is_pos = self.builder.build_int_compare(IntPredicate::SGT, exp_arg, zero, "is_pos").unwrap();
        self.builder.build_conditional_branch(is_pos, loop_bb, exit_bb).unwrap();

        let (final_res, loop_end) = self.emit_ipow64_loop(f, loop_bb, exit_bb, entry_bb, base_arg, exp_arg);

        self.builder.position_at_end(exit_bb);
        let phi_exit = self.builder.build_phi(i64_ty, "exit_val").unwrap();
        phi_exit.add_incoming(&[(&non_pos, entry_bb), (&final_res, loop_end)]);
        self.builder.build_return(Some(&phi_exit.as_basic_value())).unwrap();
    }

    fn emit_ipow64_loop(
        &self,
        f: FunctionValue<'ctx>,
        loop_bb: inkwell::basic_block::BasicBlock<'ctx>,
        exit_bb: inkwell::basic_block::BasicBlock<'ctx>,
        entry_bb: inkwell::basic_block::BasicBlock<'ctx>,
        base_arg: IntValue<'ctx>,
        exp_arg: IntValue<'ctx>,
    ) -> (IntValue<'ctx>, inkwell::basic_block::BasicBlock<'ctx>) {
        let i64_ty = self.context.i64_type();
        let odd_bb = self.context.append_basic_block(f, "odd");
        let next_bb = self.context.append_basic_block(f, "next");
        let zero = i64_ty.const_zero();
        let one = i64_ty.const_int(1, false);

        self.builder.position_at_end(loop_bb);
        let phi_res = self.builder.build_phi(i64_ty, "res").unwrap();
        let phi_base = self.builder.build_phi(i64_ty, "base").unwrap();
        let phi_exp = self.builder.build_phi(i64_ty, "exp").unwrap();
        phi_res.add_incoming(&[(&one, entry_bb)]);
        phi_base.add_incoming(&[(&base_arg, entry_bb)]);
        phi_exp.add_incoming(&[(&exp_arg, entry_bb)]);

        let cur_res = phi_res.as_basic_value().into_int_value();
        let cur_base = phi_base.as_basic_value().into_int_value();
        let cur_exp = phi_exp.as_basic_value().into_int_value();
        let bit = self.builder.build_and(cur_exp, one, "bit").unwrap();
        let is_odd = self.builder.build_int_compare(IntPredicate::NE, bit, zero, "is_odd").unwrap();
        self.builder.build_conditional_branch(is_odd, odd_bb, next_bb).unwrap();

        self.builder.position_at_end(odd_bb);
        let new_res = self.builder.build_int_mul(cur_res, cur_base, "mul_res").unwrap();
        self.builder.build_unconditional_branch(next_bb).unwrap();

        self.builder.position_at_end(next_bb);
        let phi_upd = self.builder.build_phi(i64_ty, "upd_res").unwrap();
        phi_upd.add_incoming(&[(&new_res, odd_bb), (&cur_res, loop_bb)]);
        let final_res = phi_upd.as_basic_value().into_int_value();
        let next_base = self.builder.build_int_mul(cur_base, cur_base, "sqr_base").unwrap();
        let next_exp = self.builder.build_right_shift(cur_exp, one, false, "shr_exp").unwrap();

        let has_more = self.builder.build_int_compare(IntPredicate::SGT, next_exp, zero, "has_more").unwrap();
        let loop_end = self.builder.get_insert_block().unwrap();
        self.builder.build_conditional_branch(has_more, loop_bb, exit_bb).unwrap();

        phi_res.add_incoming(&[(&final_res, loop_end)]);
        phi_base.add_incoming(&[(&next_base, loop_end)]);
        phi_exp.add_incoming(&[(&next_exp, loop_end)]);

        (final_res, loop_end)
    }

    fn compile_string_concat(
        &self,
        l: inkwell::values::StructValue<'ctx>,
        r: inkwell::values::StructValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let l_ptr = self.intrinsic.emit_slice_as_ptr(self.builder, l.into())?;
        let l_len = self.intrinsic.emit_slice_len(self.builder, l.into())?;
        let r_ptr = self.intrinsic.emit_slice_as_ptr(self.builder, r.into())?;
        let r_len = self.intrinsic.emit_slice_len(self.builder, r.into())?;

        let total_len = self
            .builder
            .build_int_add(l_len, r_len, "concat_len")
            .map_err(|e| e.to_string())?;

        let reg = self
            .dest_region
            .or(self.region_ptr)
            .ok_or("string concatenation requires an active region")?;
        let one = self.context.i64_type().const_int(1, false);
        let alloc_size = self
            .builder
            .build_int_add(total_len, one, "alloc_len")
            .map_err(|e| e.to_string())?;

        let buf = self
            .region
            .emit_dynamic_fast_path_alloc(self.module, self.builder, self.fn_val, reg, alloc_size, 1)?;

        self.builder
            .build_memcpy(buf, 1, l_ptr, 1, l_len)
            .map_err(|e| e.to_string())?;

        let i64_ty = self.context.i64_type();
        let base_int = self
            .builder
            .build_ptr_to_int(buf, i64_ty, "base_int")
            .map_err(|e| e.to_string())?;
        let second_addr = self
            .builder
            .build_int_add(base_int, l_len, "dest_r_addr")
            .map_err(|e| e.to_string())?;
        let second_dest = self
            .builder
            .build_int_to_ptr(second_addr, self.ptr_type(), "dest_r")
            .map_err(|e| e.to_string())?;

        self.builder
            .build_memcpy(second_dest, 1, r_ptr, 1, r_len)
            .map_err(|e| e.to_string())?;

        let str_val = self
            .intrinsic
            .emit_string_from_raw_parts(self.builder, buf, total_len)?;
        Ok(str_val.into())
    }

    fn compile_unary(
        &mut self,
        op: UnaryOp,
        operand: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let val = self.compile_expr(operand)?;
        if let Some(res) = self.try_compile_operator_unary(op, operand, val)? {
            return Ok(res);
        }
        match op {
            UnaryOp::Neg => match val {
                BasicValueEnum::IntValue(iv) => self
                    .builder
                    .build_int_neg(iv, "neg")
                    .map(Into::into)
                    .map_err(|e| e.to_string()),
                BasicValueEnum::FloatValue(fv) => self
                    .builder
                    .build_float_neg(fv, "fneg")
                    .map(Into::into)
                    .map_err(|e| e.to_string()),
                _ => Err("cannot negate non-numeric value".to_string()),
            },
            UnaryOp::Not | UnaryOp::BitNot => match val {
                BasicValueEnum::IntValue(iv) => self
                    .builder
                    .build_not(iv, "not")
                    .map(Into::into)
                    .map_err(|e| e.to_string()),
                _ => Err("cannot apply not to non-integer value".to_string()),
            },
        }
    }

    fn try_compile_operator_unary(
        &mut self,
        op: UnaryOp,
        operand: &Expr,
        val: BasicValueEnum<'ctx>,
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let operand_ty = self.expr_semantic_ty(operand);
        let Some(Type::Class(class_name)) = operand_ty else {
            return Ok(None);
        };
        let op_name = crate::mangle::unary_op_to_symbol(op);
        if self.find_class_method(&class_name, op_name, 0).is_some() {
            let res = self.compile_class_instance_method_call_values(&class_name, val, op_name, &[])?;
            return Ok(Some(res));
        }
        Ok(None)
    }

    fn create_entry_alloca(
        &self,
        ty: BasicTypeEnum<'ctx>,
        name: &str,
    ) -> Result<inkwell::values::PointerValue<'ctx>, String> {
        let entry_bb = self.fn_val.get_first_basic_block().ok_or("no entry basic block")?;
        let current_bb = self.builder.get_insert_block();
        if let Some(first_instr) = entry_bb.get_first_instruction() {
            self.builder.position_before(&first_instr);
        } else {
            self.builder.position_at_end(entry_bb);
        }
        let alloca = self.builder.build_alloca(ty, name).map_err(|e| e.to_string())?;
        if let Some(curr) = current_bb {
            self.builder.position_at_end(curr);
        }
        Ok(alloca)
    }

    fn store_if_result(
        &self,
        slot: inkwell::values::PointerValue<'ctx>,
        res_ty: BasicTypeEnum<'ctx>,
        val: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let to_store = if val.get_type() != res_ty {
            self.coerce_type(val, res_ty)?
        } else {
            val
        };
        self.builder.build_store(slot, to_store).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn compile_if(
        &mut self,
        cond: &Expr,
        then_branch: &Block,
        elif_branches: &[(Expr, Block)],
        else_branch: Option<&Block>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let cond_val = self.compile_expr(cond)?;
        let cond_bool = self.to_bool(cond_val)?;

        let then_bb = self.context.append_basic_block(self.fn_val, "if.then");
        let else_bb = self.context.append_basic_block(self.fn_val, "if.else");
        let merge_bb = self.context.append_basic_block(self.fn_val, "if.merge");

        self.builder
            .build_conditional_branch(cond_bool, then_bb, else_bb)
            .map_err(|e| e.to_string())?;

        let has_else = else_branch.is_some();
        let mut res_slot: Option<(PointerValue<'ctx>, BasicTypeEnum<'ctx>)> = None;

        self.builder.position_at_end(then_bb);
        let then_val = self.compile_block(then_branch)?;
        if !self.is_terminated() {
            if has_else {
                if let Some(tv) = then_val {
                    let ty = tv.get_type();
                    let slot = self.create_entry_alloca(ty, "if.res")?;
                    self.store_if_result(slot, ty, tv)?;
                    res_slot = Some((slot, ty));
                }
            }
            self.builder
                .build_unconditional_branch(merge_bb)
                .map_err(|e| e.to_string())?;
        }

        self.builder.position_at_end(else_bb);
        if elif_branches.is_empty() {
            if let Some(else_b) = else_branch {
                let else_val = self.compile_block(else_b)?;
                if !self.is_terminated() {
                    if let Some(ev) = else_val {
                        let (slot, ty) = match res_slot {
                            Some(s) => s,
                            None => {
                                let ty = ev.get_type();
                                let slot = self.create_entry_alloca(ty, "if.res")?;
                                res_slot = Some((slot, ty));
                                (slot, ty)
                            }
                        };
                        self.store_if_result(slot, ty, ev)?;
                    }
                    self.builder
                        .build_unconditional_branch(merge_bb)
                        .map_err(|e| e.to_string())?;
                }
            } else if !self.is_terminated() {
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|e| e.to_string())?;
            }
        } else {
            let (first_elif, rest_elifs) = &elif_branches[0];
            let ev = self.compile_if(first_elif, rest_elifs, &elif_branches[1..], else_branch)?;
            if !self.is_terminated() {
                if has_else {
                    let (slot, ty) = match res_slot {
                        Some(s) => s,
                        None => {
                            let ty = ev.get_type();
                            let slot = self.create_entry_alloca(ty, "if.res")?;
                            res_slot = Some((slot, ty));
                            (slot, ty)
                        }
                    };
                    self.store_if_result(slot, ty, ev)?;
                }
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|e| e.to_string())?;
            }
        }

        self.builder.position_at_end(merge_bb);
        if let Some((slot, ty)) = res_slot {
            let res_val = self
                .builder
                .build_load(ty, slot, "if.val")
                .map_err(|e| e.to_string())?;
            Ok(res_val)
        } else {
            Ok(self.context.i64_type().const_zero().into())
        }
    }

    fn compile_while(
        &mut self,
        label: Option<&str>,
        cond: &Expr,
        body: &Block,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let cond_bb = self.context.append_basic_block(self.fn_val, "while.cond");
        let body_bb = self.context.append_basic_block(self.fn_val, "while.body");
        let exit_bb = self.context.append_basic_block(self.fn_val, "while.exit");

        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|e| e.to_string())?;

        self.builder.position_at_end(cond_bb);
        let cond_val = self.compile_expr(cond)?;
        let cond_bool = self.to_bool(cond_val)?;
        self.builder
            .build_conditional_branch(cond_bool, body_bb, exit_bb)
            .map_err(|e| e.to_string())?;

        self.loop_stack.push(LoopContext {
            continue_bb: cond_bb,
            break_bb: exit_bb,
        });
        self.cleanups_stack.push_scope(ScopeKind::Loop {
            label: label.map(|s| s.to_string()),
            break_target: exit_bb,
            continue_target: cond_bb,
        });
        self.current_cleanup_bb = None;

        self.builder.position_at_end(body_bb);
        self.compile_block(body)?;
        if !self.is_terminated() {
            self.builder
                .build_unconditional_branch(cond_bb)
                .map_err(|e| e.to_string())?;
        }

        self.cleanups_stack.pop_scope();
        self.current_cleanup_bb = None;
        self.loop_stack.pop();
        self.builder.position_at_end(exit_bb);
        Ok(self.context.i64_type().const_zero().into())
    }

    fn compile_loop(
        &mut self,
        label: Option<&str>,
        body: &Block,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let body_bb = self.context.append_basic_block(self.fn_val, "loop.body");
        let exit_bb = self.context.append_basic_block(self.fn_val, "loop.exit");

        self.builder
            .build_unconditional_branch(body_bb)
            .map_err(|e| e.to_string())?;

        self.loop_stack.push(LoopContext {
            continue_bb: body_bb,
            break_bb: exit_bb,
        });
        self.cleanups_stack.push_scope(ScopeKind::Loop {
            label: label.map(|s| s.to_string()),
            break_target: exit_bb,
            continue_target: body_bb,
        });
        self.current_cleanup_bb = None;

        self.builder.position_at_end(body_bb);
        self.compile_block(body)?;
        if !self.is_terminated() {
            self.builder
                .build_unconditional_branch(body_bb)
                .map_err(|e| e.to_string())?;
        }

        self.cleanups_stack.pop_scope();
        self.current_cleanup_bb = None;
        self.loop_stack.pop();
        self.builder.position_at_end(exit_bb);
        Ok(self.context.i64_type().const_zero().into())
    }

    fn compile_for(
        &mut self,
        label: Option<&str>,
        index_var: Option<&Ident>,
        item_var: &Ident,
        iter: &Expr,
        body: &Block,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if let ExprKind::Binary(op, start_expr, end_expr) = &iter.kind {
            let is_range = *op == BinaryOp::Range;
            let is_range_inc = *op == BinaryOp::RangeInclusive;
            if is_range || is_range_inc {
                return self.compile_for_range(
                    label,
                    index_var,
                    item_var,
                    start_expr,
                    end_expr,
                    is_range_inc,
                    body,
                );
            }
        }
        Err("unsupported iterator in for loop".to_string())
    }

    fn compile_for_range(
        &mut self,
        label: Option<&str>,
        _index_var: Option<&Ident>,
        item_var: &Ident,
        start_expr: &Expr,
        end_expr: &Expr,
        inclusive: bool,
        body: &Block,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let start_val = self.compile_expr(start_expr)?;
        let end_val = self.compile_expr(end_expr)?;
        let i64_ty = self.context.i64_type();
        let start_int = self.coerce_type(start_val, i64_ty.into())?.into_int_value();
        let end_int = self.coerce_type(end_val, i64_ty.into())?.into_int_value();

        let alloca = self
            .builder
            .build_alloca(i64_ty, &item_var.name)
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(alloca, start_int)
            .map_err(|e| e.to_string())?;

        let cond_bb = self.context.append_basic_block(self.fn_val, "for.cond");
        let body_bb = self.context.append_basic_block(self.fn_val, "for.body");
        let step_bb = self.context.append_basic_block(self.fn_val, "for.step");
        let exit_bb = self.context.append_basic_block(self.fn_val, "for.exit");

        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|e| e.to_string())?;

        self.builder.position_at_end(cond_bb);
        let curr = self
            .builder
            .build_load(i64_ty, alloca, &item_var.name)
            .map_err(|e| e.to_string())?
            .into_int_value();
        let pred = if inclusive {
            IntPredicate::SLE
        } else {
            IntPredicate::SLT
        };
        let cond_bool = self
            .builder
            .build_int_compare(pred, curr, end_int, "for.cond")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_conditional_branch(cond_bool, body_bb, exit_bb)
            .map_err(|e| e.to_string())?;

        self.loop_stack.push(LoopContext {
            continue_bb: step_bb,
            break_bb: exit_bb,
        });
        self.cleanups_stack.push_scope(ScopeKind::Loop {
            label: label.map(|s| s.to_string()),
            break_target: exit_bb,
            continue_target: step_bb,
        });
        self.current_cleanup_bb = None;

        self.builder.position_at_end(body_bb);
        self.push_scope();
        self.add_local(item_var.name.clone(), alloca, i64_ty.into());
        self.compile_block(body)?;
        self.pop_scope();
        if !self.is_terminated() {
            self.builder
                .build_unconditional_branch(step_bb)
                .map_err(|e| e.to_string())?;
        }

        self.builder.position_at_end(step_bb);
        let curr_val = self
            .builder
            .build_load(i64_ty, alloca, "curr")
            .map_err(|e| e.to_string())?
            .into_int_value();
        let one = i64_ty.const_int(1, false);
        let next_val = self
            .builder
            .build_int_add(curr_val, one, "next")
            .map_err(|e| e.to_string())?;
        self.builder
            .build_store(alloca, next_val)
            .map_err(|e| e.to_string())?;
        self.builder
            .build_unconditional_branch(cond_bb)
            .map_err(|e| e.to_string())?;

        self.cleanups_stack.pop_scope();
        self.current_cleanup_bb = None;
        self.loop_stack.pop();
        self.builder.position_at_end(exit_bb);
        Ok(self.context.i64_type().const_zero().into())
    }

    fn compile_index_access(
        &mut self,
        target: &Expr,
        index: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let target_val = self.compile_expr(target)?;
        let index_val = self.compile_expr(index)?;
        if let Some(Type::Class(class_name)) = self.expr_semantic_ty(target) {
            if self.find_class_method(&class_name, "[]", 1).is_some() {
                return self.compile_class_instance_method_call_values(
                    &class_name,
                    target_val,
                    "[]",
                    &[index_val],
                );
            }
        }
        let i64_ty = self.context.i64_type();
        let idx = self.coerce_type(index_val, i64_ty.into())?.into_int_value();

        let (elem_ty, _elem_size) = self.resolve_index_layout(target, target_val)?;

        let elem_ptr = self
            .intrinsic
            .emit_slice_elem_ptr(self.builder, target_val, idx, elem_ty)?;
        self.builder
            .build_load(elem_ty, elem_ptr, "elem")
            .map_err(|e| e.to_string())
    }

    fn compile_slice_access(
        &mut self,
        target: &Expr,
        start: Option<&Expr>,
        end: Option<&Expr>,
        inclusive: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let target_val = self.compile_expr(target)?;
        let i64_ty = self.context.i64_type();

        let start_val = if let Some(s) = start {
            let s_val = self.compile_expr(s)?;
            self.coerce_type(s_val, i64_ty.into())?.into_int_value()
        } else {
            i64_ty.const_zero()
        };

        let end_val = if let Some(e) = end {
            let e_val = self.compile_expr(e)?;
            let mut end_int = self.coerce_type(e_val, i64_ty.into())?.into_int_value();
            if inclusive {
                end_int = self
                    .builder
                    .build_int_add(end_int, i64_ty.const_int(1, false), "inc_end")
                    .map_err(|err| err.to_string())?;
            }
            end_int
        } else {
            self.intrinsic.emit_slice_len(self.builder, target_val)?
        };

        let (_elem_ty, elem_size) = self.resolve_index_layout(target, target_val)?;
        let size_val = i64_ty.const_int(elem_size as u64, false);

        let is_string = matches!(self.expr_semantic_ty(target), Some(Type::String))
            || matches!(target_val, BasicValueEnum::StructValue(sv) if sv.get_type().count_fields() == 2);

        let slice_val = self.intrinsic.emit_subslice(
            self.builder,
            target_val,
            start_val,
            end_val,
            size_val,
            is_string,
        )?;

        Ok(slice_val.into())
    }

    pub(crate) fn resolve_index_layout(
        &self,
        target: &Expr,
        target_val: BasicValueEnum<'ctx>,
    ) -> Result<(BasicTypeEnum<'ctx>, usize), String> {
        let sem = self.expr_semantic_ty(target).or_else(|| {
            if let ExprKind::Ident(id) = &target.kind {
                self.lookup_local(&id.name).and_then(|l| l.semantic_ty.clone())
            } else {
                None
            }
        });
        let actual = match &sem {
            Some(Type::Ref { inner, .. }) => inner.as_ref(),
            Some(t) => t,
            None => &Type::Null,
        };
        if let Type::Array(elem) = actual {
            return self.elem_type_to_layout(elem);
        }

        if let BasicValueEnum::StructValue(sv) = target_val {
            let fields_count = sv.get_type().count_fields();
            if fields_count == 2 {
                return Ok((self.context.i8_type().into(), 1));
            }
        }

        Ok((self.context.i8_type().into(), 1))
    }

    fn elem_type_to_layout(&self, elem: &Type) -> Result<(BasicTypeEnum<'ctx>, usize), String> {
        match elem {
            Type::String => Ok((self.intrinsic.string_type().into(), 16)),
            Type::Int64 | Type::Uint64 => Ok((self.context.i64_type().into(), 8)),
            Type::Int32 | Type::Uint32 | Type::Char => Ok((self.context.i32_type().into(), 4)),
            Type::Int16 | Type::Uint16 => Ok((self.context.i16_type().into(), 2)),
            Type::Int8 | Type::Uint8 | Type::Bool => Ok((self.context.i8_type().into(), 1)),
            Type::Float64 => Ok((self.context.f64_type().into(), 8)),
            Type::Float32 => Ok((self.context.f32_type().into(), 4)),
            Type::Ptr(_) => Ok((self.intrinsic.ptr_type().into(), 8)),
            _ => {
                let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
                let lowerer =
                    crate::types::TypeLowerer::new(self.context, &td, self.target, self.registry);
                let llvm_ty = lowerer.lower_type(elem)?;
                let sz = usize::try_from(td.get_abi_size(&llvm_ty)).map_err(|e| e.to_string())?;
                Ok((llvm_ty, sz))
            }
        }
    }

    fn compile_ternary(
        &mut self,
        cond: &Expr,
        then_e: &Expr,
        else_e: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let cond_val = self.compile_expr(cond)?;
        let cond_bool = self.to_bool(cond_val)?;

        let then_bb = self.context.append_basic_block(self.fn_val, "tern.then");
        let else_bb = self.context.append_basic_block(self.fn_val, "tern.else");
        let merge_bb = self.context.append_basic_block(self.fn_val, "tern.merge");

        self.builder
            .build_conditional_branch(cond_bool, then_bb, else_bb)
            .map_err(|e| e.to_string())?;

        self.builder.position_at_end(then_bb);
        let then_val = self.compile_expr(then_e)?;
        let then_end = self.builder.get_insert_block().unwrap_or(then_bb);
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| e.to_string())?;

        self.builder.position_at_end(else_bb);
        let else_val = self.compile_expr(else_e)?;
        let else_end = self.builder.get_insert_block().unwrap_or(else_bb);
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| e.to_string())?;

        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(then_val.get_type(), "tern.val")
            .map_err(|e| e.to_string())?;
        phi.add_incoming(&[(&then_val, then_end), (&else_val, else_end)]);
        Ok(phi.as_basic_value())
    }

    fn compile_interpolated_string(
        &mut self,
        parts: &[StringPart],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if parts.is_empty() {
            return Ok(self.rodata.emit_string(self.module, "").into());
        }

        let mut acc: Option<inkwell::values::StructValue<'ctx>> = None;
        for part in parts {
            let part_str = match part {
                StringPart::Lit(s) => self.rodata.emit_string(self.module, s),
                StringPart::Expr(e) => {
                    let val = self.compile_expr(e)?;
                    let ty = self.deduce_expr_type(e, Some(val))?;
                    if ty == Type::String {
                        val.into_struct_value()
                    } else {
                        let casted = self.intrinsic.emit_cast(
                            self.module,
                            self.builder,
                            val,
                            &ty,
                            &Type::String,
                            self.dest_region.or(self.region_ptr),
                        )?;
                        casted.into_struct_value()
                    }
                }
            };

            acc = match acc {
                Some(prev) => {
                    let concatenated = self.compile_string_concat(prev, part_str)?;
                    Some(concatenated.into_struct_value())
                }
                None => Some(part_str),
            };
        }

        Ok(acc
            .unwrap_or_else(|| self.rodata.emit_string(self.module, ""))
            .into())
    }

    fn compile_value_equality(
        &self,
        l: BasicValueEnum<'ctx>,
        r: BasicValueEnum<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        if l.is_int_value() && r.is_int_value() {
            let li = l.into_int_value();
            let ri = r.into_int_value();
            let lb = li.get_type().get_bit_width();
            let rb = ri.get_type().get_bit_width();
            let r_cast = if lb == rb {
                ri
            } else {
                self.builder
                    .build_int_cast(ri, li.get_type(), "sw_cast")
                    .map_err(|e| e.to_string())?
            };
            self.builder
                .build_int_compare(IntPredicate::EQ, li, r_cast, "sw_eq")
                .map_err(|e| e.to_string())
        } else if l.is_float_value() && r.is_float_value() {
            self.builder
                .build_float_compare(FloatPredicate::OEQ, l.into_float_value(), r.into_float_value(), "sw_feq")
                .map_err(|e| e.to_string())
        } else {
            Ok(self.context.bool_type().const_int(1, false))
        }
    }

    pub(crate) fn compile_enum_instantiation(
        &mut self,
        enum_name: &str,
        variant: &EnumVariantLayout,
        arg_vals: &[BasicValueEnum<'ctx>],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
        let lowerer = crate::types::TypeLowerer::new(self.context, &td, self.target, self.registry);
        let enum_st = lowerer.lower_named_enum(enum_name)?;
        let alloca = self.create_entry_alloca(enum_st.into(), &format!("enum.{enum_name}.{}", variant.name))?;

        let zero = enum_st.const_zero();
        self.builder.build_store(alloca, zero).map_err(|e| e.to_string())?;

        let tag_ptr = self
            .builder
            .build_struct_gep(enum_st, alloca, 0, "tag_ptr")
            .map_err(|e| e.to_string())?;
        let tag_val = self.context.i32_type().const_int(variant.tag as u64, false);
        self.builder
            .build_store(tag_ptr, tag_val)
            .map_err(|e| e.to_string())?;

        if !variant.payload.is_empty() && !arg_vals.is_empty() {
            let mut payload_field_tys = Vec::with_capacity(variant.payload.len());
            for p in &variant.payload {
                payload_field_tys.push(lowerer.lower_type(p)?);
            }
            let payload_st = self.context.struct_type(&payload_field_tys, false);
            let payload_buf_ptr = self
                .builder
                .build_struct_gep(enum_st, alloca, 1, "payload_buf")
                .map_err(|e| e.to_string())?;
            let typed_payload_ptr = self
                .builder
                .build_bit_cast(
                    payload_buf_ptr,
                    self.context.ptr_type(inkwell::AddressSpace::default()),
                    "typed_payload",
                )
                .map_err(|e| e.to_string())?
                .into_pointer_value();

            for (idx, arg_val) in arg_vals.iter().enumerate() {
                if idx >= variant.payload.len() {
                    break;
                }
                let field_ptr = self
                    .builder
                    .build_struct_gep(payload_st, typed_payload_ptr, idx as u32, &format!("p_{idx}"))
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_store(field_ptr, *arg_val)
                    .map_err(|e| e.to_string())?;
            }
        }

        let val = self
            .builder
            .build_load(enum_st, alloca, "enum_val")
            .map_err(|e| e.to_string())?;
        Ok(val)
    }

    fn compile_pattern_match(
        &mut self,
        target_val: BasicValueEnum<'ctx>,
        pattern: &Pattern,
    ) -> Result<IntValue<'ctx>, String> {
        let bool_ty = self.context.bool_type();
        match &pattern.kind {
            PatternKind::Ident(id) if id.name != "else" => {
                let var_alloca = self
                    .builder
                    .build_alloca(target_val.get_type(), &id.name)
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_store(var_alloca, target_val)
                    .map_err(|e| e.to_string())?;
                self.add_local(id.name.clone(), var_alloca, target_val.get_type());
                Ok(bool_ty.const_int(1, false))
            }
            PatternKind::Literal(lit) => {
                let lit_val = self.compile_literal(lit);
                self.compile_value_equality(target_val, lit_val)
            }
            PatternKind::Variant(opt_enum, var_ident, _) => {
                let mut found_variant = None;
                if let Some(enum_id) = opt_enum {
                    if let Some(enum_layout) = self.registry.enums.get(&enum_id.name) {
                        if let Some(v) = enum_layout.variants.iter().find(|v| v.name == var_ident.name) {
                            found_variant = Some((enum_layout.name.clone(), v.clone()));
                        }
                    }
                } else {
                    for enum_layout in self.registry.enums.values() {
                        if let Some(v) = enum_layout.variants.iter().find(|v| v.name == var_ident.name) {
                            found_variant = Some((enum_layout.name.clone(), v.clone()));
                            break;
                        }
                    }
                }

                if let Some((enum_name, variant)) = found_variant {
                    let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
        let lowerer = crate::types::TypeLowerer::new(self.context, &td, self.target, self.registry);
                    let enum_st = lowerer.lower_named_enum(&enum_name)?;
                    let alloca = self.create_entry_alloca(enum_st.into(), "sw.enum_tgt")?;
                    self.builder.build_store(alloca, target_val).map_err(|e| e.to_string())?;

                    let tag_ptr = self.builder.build_struct_gep(enum_st, alloca, 0, "sw.tag_ptr").map_err(|e| e.to_string())?;
                    let tag_val = self.builder.build_load(self.context.i32_type(), tag_ptr, "sw.tag").map_err(|e| e.to_string())?.into_int_value();
                    let expected_tag = self.context.i32_type().const_int(variant.tag as u64, false);
                    self.builder.build_int_compare(inkwell::IntPredicate::EQ, tag_val, expected_tag, "sw.tag_cmp").map_err(|e| e.to_string())
                } else {
                    Err(format!("unknown enum variant '{}'", var_ident.name))
                }
            }
            PatternKind::Or(pats) => {
                let mut matched = bool_ty.const_int(0, false);
                for p in pats {
                    let sub = self.compile_pattern_match(target_val, p)?;
                    matched = self
                        .builder
                        .build_or(matched, sub, "or_match")
                        .map_err(|e| e.to_string())?;
                }
                Ok(matched)
            }
            _ => Ok(bool_ty.const_int(1, false)),
        }
    }

    fn unpack_pattern_bindings(
        &mut self,
        target_val: BasicValueEnum<'ctx>,
        pattern: &Pattern,
    ) -> Result<(), String> {
        match &pattern.kind {
            PatternKind::Ident(id) if id.name != "else" => {
                let var_alloca = self
                    .create_entry_alloca(target_val.get_type(), &id.name)?;
                self.builder
                    .build_store(var_alloca, target_val)
                    .map_err(|e| e.to_string())?;
                self.add_local(id.name.clone(), var_alloca, target_val.get_type());
            }
            PatternKind::Variant(opt_enum, var_ident, sub_pats) => {
                if sub_pats.is_empty() {
                    return Ok(());
                }
                let mut found_variant = None;
                if let Some(enum_id) = opt_enum {
                    if let Some(enum_layout) = self.registry.enums.get(&enum_id.name) {
                        if let Some(v) = enum_layout.variants.iter().find(|v| v.name == var_ident.name) {
                            found_variant = Some((enum_layout.name.clone(), v.clone()));
                        }
                    }
                } else {
                    for enum_layout in self.registry.enums.values() {
                        if let Some(v) = enum_layout.variants.iter().find(|v| v.name == var_ident.name) {
                            found_variant = Some((enum_layout.name.clone(), v.clone()));
                            break;
                        }
                    }
                }

                if let Some((enum_name, variant)) = found_variant {
                    if !variant.payload.is_empty() {
                        let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
        let lowerer = crate::types::TypeLowerer::new(self.context, &td, self.target, self.registry);
                        let enum_st = lowerer.lower_named_enum(&enum_name)?;
                        let alloca = self.create_entry_alloca(enum_st.into(), "sw.enum_bind")?;
                        self.builder.build_store(alloca, target_val).map_err(|e| e.to_string())?;

                        let mut payload_field_tys = Vec::with_capacity(variant.payload.len());
                        for p in &variant.payload {
                            payload_field_tys.push(lowerer.lower_type(p)?);
                        }
                        let payload_st = self.context.struct_type(&payload_field_tys, false);
                        let payload_buf_ptr = self.builder.build_struct_gep(enum_st, alloca, 1, "sw.payload_buf").map_err(|e| e.to_string())?;
                        let typed_payload_ptr = self.builder.build_bit_cast(
                            payload_buf_ptr,
                            self.context.ptr_type(inkwell::AddressSpace::default()),
                            "sw.typed_payload",
                        ).map_err(|e| e.to_string())?.into_pointer_value();

                        for (idx, sub_pat) in sub_pats.iter().enumerate() {
                            if idx >= variant.payload.len() {
                                break;
                            }
                            if let PatternKind::Ident(id) = &sub_pat.kind {
                                let fty = payload_field_tys[idx];
                                let field_ptr = self.builder.build_struct_gep(payload_st, typed_payload_ptr, idx as u32, &format!("{}.ptr", id.name)).map_err(|e| e.to_string())?;
                                let field_val = self.builder.build_load(fty, field_ptr, &id.name).map_err(|e| e.to_string())?;
                                let var_alloca = self.create_entry_alloca(fty, &id.name)?;
                                self.builder.build_store(var_alloca, field_val).map_err(|e| e.to_string())?;
                                self.add_local_with_ty(id.name.clone(), var_alloca, fty, Some(variant.payload[idx].clone()));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn compile_switch(
        &mut self,
        target: &Expr,
        arms: &[SwitchArm],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let target_val = self.compile_expr(target)?;
        let merge_bb = self.context.append_basic_block(self.fn_val, "sw.merge");

        let mut res_slot: Option<(PointerValue<'ctx>, BasicTypeEnum<'ctx>)> = None;

        for (i, arm) in arms.iter().enumerate() {
            let arm_bb = self.context.append_basic_block(self.fn_val, &format!("sw.arm_{i}"));
            let next_bb = self.context.append_basic_block(self.fn_val, &format!("sw.next_{i}"));

            let cond = self.compile_pattern_match(target_val, &arm.pattern)?;
            let final_cond = if let Some(guard) = &arm.guard {
                let g_val = self.compile_expr(guard)?;
                let g_bool = self.to_bool(g_val)?;
                self.builder
                    .build_and(cond, g_bool, "sw.guard")
                    .map_err(|e| e.to_string())?
            } else {
                cond
            };

            self.builder
                .build_conditional_branch(final_cond, arm_bb, next_bb)
                .map_err(|e| e.to_string())?;

            self.builder.position_at_end(arm_bb);
            self.push_scope();
            self.unpack_pattern_bindings(target_val, &arm.pattern)?;
            let val = self.compile_block(&arm.body)?;
            self.pop_scope();
            if !self.is_terminated() {
                if let Some(v) = val {
                    let (slot, ty) = match res_slot {
                        Some(s) => s,
                        None => {
                            let ty = v.get_type();
                            let slot = self.create_entry_alloca(ty, "sw.res")?;
                            res_slot = Some((slot, ty));
                            (slot, ty)
                        }
                    };
                    self.store_if_result(slot, ty, v)?;
                }
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|e| e.to_string())?;
            }

            self.builder.position_at_end(next_bb);
        }

        if !self.is_terminated() {
            self.builder
                .build_unconditional_branch(merge_bb)
                .map_err(|e| e.to_string())?;
        }

        self.builder.position_at_end(merge_bb);
        if let Some((slot, ty)) = res_slot {
            let res_val = self
                .builder
                .build_load(ty, slot, "sw.val")
                .map_err(|e| e.to_string())?;
            Ok(res_val)
        } else {
            Ok(self.context.i64_type().const_zero().into())
        }
    }

    fn compile_sus(
        &mut self,
        body: &Block,
        catches: &[CatchClause],
        finally_branch: Option<&Block>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if self.is_abort_strategy() {
            let body_res = self.compile_block(body)?;
            if !self.is_terminated() {
                if let Some(fb) = finally_branch {
                    self.compile_block(fb)?;
                }
            }
            return Ok(body_res.unwrap_or_else(|| self.context.i64_type().const_zero().into()));
        }

        let personality = self.get_or_declare_personality();
        self.fn_val.set_personality_function(personality);

        let sus_landing = self.context.append_basic_block(self.fn_val, "sus_landing");
        let sus_exit = self.context.append_basic_block(self.fn_val, "sus_exit");

        let old_unwind = self.unwind_dest;
        self.unwind_dest = Some(sus_landing);

        self.cleanups_stack.push_scope(ScopeKind::Sus);
        self.current_cleanup_bb = None;

        let body_res = self.compile_block(body)?;
        self.unwind_dest = old_unwind;
        self.current_cleanup_bb = None;

        if !self.is_terminated() {
            self.builder
                .build_unconditional_branch(sus_exit)
                .map_err(|e| e.to_string())?;
        }

        self.builder.position_at_end(sus_landing);

        let pad_type = self.context.struct_type(
            &[self.ptr_type().into(), self.context.i32_type().into()],
            false,
        );

        let mut clauses: Vec<BasicValueEnum<'ctx>> = Vec::new();
        for c in catches {
            if let Some(tn) = &c.error_type {
                let name = match &tn.kind {
                    yukic_ast::TypeNodeKind::Ident(id) => id.name.as_str(),
                    _ => "Error",
                };
                let ti = self.get_or_create_typeinfo(name);
                clauses.push(ti.as_pointer_value().into());
            }
        }
        clauses.push(self.ptr_type().const_null().into());

        let pad_val = self
            .builder
            .build_landing_pad(pad_type, personality, &clauses, true, "eh_info")
            .map_err(|e| e.to_string())?;

        let sus_cleanups = self.cleanups_stack.collect_sus_cleanups();
        let old_in_cleanup = self.in_cleanup;
        self.in_cleanup = true;
        for c in &sus_cleanups {
            self.execute_cleanup(c)?;
        }
        self.in_cleanup = old_in_cleanup;

        let exc_ptr = self
            .builder
            .build_extract_value(pad_val.into_struct_value(), 0, "exc_ptr")
            .map_err(|e| e.to_string())?
            .into_pointer_value();
        let selector = self
            .builder
            .build_extract_value(pad_val.into_struct_value(), 1, "selector")
            .map_err(|e| e.to_string())?
            .into_int_value();

        let typeid_fn = self.get_or_declare_eh_typeid();
        let mut resume_bb = None;

        for (i, c) in catches.iter().enumerate() {
            let catch_bb = self.context.append_basic_block(self.fn_val, &format!("sus_catch.{i}"));
            let is_last = i + 1 == catches.len();
            let next_chk = if c.error_type.is_some() && !is_last {
                Some(self.context.append_basic_block(self.fn_val, &format!("sus_chk.{i}")))
            } else {
                None
            };

            if let Some(tn) = &c.error_type {
                let name = match &tn.kind {
                    yukic_ast::TypeNodeKind::Ident(id) => id.name.as_str(),
                    _ => "Error",
                };
                let ti = self.get_or_create_typeinfo(name);
                let tid = self
                    .builder
                    .build_call(typeid_fn, &[ti.as_pointer_value().into()], "tid")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                let is_match = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, selector, tid, "is_match")
                    .map_err(|e| e.to_string())?;
                let fallback = next_chk.unwrap_or_else(|| {
                    let bb = self.context.append_basic_block(self.fn_val, "sus_resume");
                    resume_bb = Some(bb);
                    bb
                });
                self.builder
                    .build_conditional_branch(is_match, catch_bb, fallback)
                    .map_err(|e| e.to_string())?;
            } else {
                self.builder
                    .build_unconditional_branch(catch_bb)
                    .map_err(|e| e.to_string())?;
            }

            self.builder.position_at_end(catch_bb);
            self.setup_catch_scope(exc_ptr, c)?;
            self.compile_block(&c.body)?;
            self.pop_scope();

            if !self.is_terminated() {
                self.builder
                    .build_unconditional_branch(sus_exit)
                    .map_err(|e| e.to_string())?;
            }

            if let Some(chk_bb) = next_chk {
                self.builder.position_at_end(chk_bb);
            } else if c.error_type.is_none() {
                break;
            }
        }

        if let Some(r_bb) = resume_bb {
            self.builder.position_at_end(r_bb);
        }

        if !self.is_terminated() {
            let outer_cleanups = self.cleanups_stack.collect_return_cleanups();
            let old_in_cleanup = self.in_cleanup;
            self.in_cleanup = true;
            for c in &outer_cleanups {
                let already_ran = sus_cleanups.iter().any(|sc| matches!((sc, c),
                    (CleanupAction::Defer(b1), CleanupAction::Defer(b2)) if b1.span == b2.span
                        || matches!((sc, c), (CleanupAction::RegionRestoreMark { mark: m1, .. }, CleanupAction::RegionRestoreMark { mark: m2, .. }) if m1 == m2)
                ));
                if !already_ran {
                    self.execute_cleanup(c)?;
                }
            }
            if let Some(reg) = self.region_ptr {
                self.region.emit_region_epilogue(self.module, self.builder, reg)?;
            }
            self.in_cleanup = old_in_cleanup;

            self.builder
                .build_resume(pad_val)
                .map_err(|e| e.to_string())?;
        }

        self.cleanups_stack.pop_scope();
        self.current_cleanup_bb = None;

        self.builder.position_at_end(sus_exit);
        if let Some(finally_b) = finally_branch {
            self.compile_block(finally_b)?;
        }
        Ok(body_res.unwrap_or_else(|| self.context.i64_type().const_zero().into()))
    }

    fn setup_catch_scope(
        &mut self,
        exc_ptr: inkwell::values::PointerValue<'ctx>,
        c: &CatchClause,
    ) -> Result<(), String> {
        let ph = self.get_or_declare_panic_handled();
        self.builder
            .build_call(ph, &[], "")
            .map_err(|e| e.to_string())?;

        let gk = self.get_or_declare_get_kind();
        let kind_val = self
            .builder
            .build_call(gk, &[exc_ptr.into()], "kind")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .unwrap();

        self.push_scope();

        let errkind_alloca = self
            .builder
            .build_alloca(kind_val.get_type(), "errkind")
            .map_err(|e| e.to_string())?;
        self.builder.build_store(errkind_alloca, kind_val).map_err(|e| e.to_string())?;
        self.add_local_with_ty("errkind".to_string(), errkind_alloca, kind_val.get_type(), Some(Type::String));

        if let (Some(var), Some(tn)) = (&c.error_var, &c.error_type) {
            self.bind_catch_error_var(exc_ptr, var, tn)?;
        }

        Ok(())
    }

    fn bind_catch_error_var(
        &mut self,
        exc_ptr: inkwell::values::PointerValue<'ctx>,
        var: &Ident,
        error_type: &yukic_ast::TypeNode,
    ) -> Result<(), String> {
        let type_name = match &error_type.kind {
            yukic_ast::TypeNodeKind::Ident(id) => id.name.clone(),
            _ => "Error".to_string(),
        };

        let get_payload = self.get_or_declare_get_payload();
        let payload_ptr = self
            .builder
            .build_call(get_payload, &[exc_ptr.into()], "payload_ptr")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();

        let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
        let lowerer = crate::types::TypeLowerer::new(self.context, &td, self.target, self.registry);

        let struct_ty = self
            .context
            .get_struct_type(&format!("struct.{type_name}"))
            .or_else(|| self.context.get_struct_type(&format!("cstruct.{type_name}")))
            .or_else(|| lowerer.lower_named_struct(&type_name).ok())
            .or_else(|| lowerer.lower_named_cstruct(&type_name).ok())
            .ok_or_else(|| format!("unknown struct type '{type_name}' for catch error variable"))?;

        let alloca = self
            .builder
            .build_alloca(struct_ty, &var.name)
            .map_err(|e| e.to_string())?;

        let sem_ty = Type::Struct(type_name);
        let size = lowerer.abi_size_of(&sem_ty).unwrap_or(8).max(1);
        let size_val = self.context.i64_type().const_int(size, false);
        let align = lowerer.abi_align_of(&sem_ty).unwrap_or(8).max(1) as u32;

        self.builder
            .build_memcpy(alloca, align, payload_ptr, align, size_val)
            .map_err(|e| e.to_string())?;

        self.add_local_with_ty(var.name.clone(), alloca, struct_ty.into(), Some(sem_ty));
        Ok(())
    }

    fn compile_struct_init(
        &mut self,
        ident: &Ident,
        fields: &[FieldInit],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let mut field_vals = Vec::with_capacity(fields.len());
        for f in fields {
            let v = self.compile_expr(&f.value)?;
            field_vals.push((f.name.name.clone(), v));
        }

        let ordered = if let Some(layout) = self.registry.structs.get(&ident.name) {
            let mut list = Vec::with_capacity(layout.fields.len());
            for (fname, _) in &layout.fields {
                if let Some((_, v)) = field_vals.iter().find(|(n, _)| n == fname) {
                    list.push(*v);
                } else {
                    list.push(self.context.i64_type().const_zero().into());
                }
            }
            list
        } else if let Some(layout) = self.registry.cstructs.get(&ident.name) {
            // Order cstruct literal fields by declaration order and
            // zero-fill omitted ones so the value matches the C layout.
            let mut list = Vec::with_capacity(layout.fields.len());
            for (fname, _) in &layout.fields {
                if let Some((_, v)) = field_vals.iter().find(|(n, _)| n == fname) {
                    list.push(*v);
                } else {
                    list.push(self.context.i64_type().const_zero().into());
                }
            }
            list
        } else {
            field_vals.into_iter().map(|(_, v)| v).collect()
        };

        let types: Vec<BasicTypeEnum<'ctx>> = ordered.iter().map(BasicValueEnum::get_type).collect();
        let struct_ty = self
            .context
            .get_struct_type(&format!("struct.{}", ident.name))
            .or_else(|| self.context.get_struct_type(&format!("cstruct.{}", ident.name)))
            .or_else(|| {
                let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
                let lowerer = crate::types::TypeLowerer::new(
                    self.context,
                    &td,
                    self.target,
                    self.registry,
                );
                if self.registry.structs.contains_key(&ident.name) {
                    match lowerer.lower_type(&Type::Struct(ident.name.clone())) {
                        Ok(BasicTypeEnum::StructType(st)) => Some(st),
                        _ => None,
                    }
                } else if self.registry.cstructs.contains_key(&ident.name) {
                    match lowerer.lower_type(&Type::Cstruct(ident.name.clone())) {
                        Ok(BasicTypeEnum::StructType(st)) => Some(st),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .unwrap_or_else(|| self.context.struct_type(&types, false));
        let mut sv = struct_ty.get_undef();
        for (i, v) in ordered.iter().enumerate() {
            let field_idx = u32::try_from(i).map_err(|e| e.to_string())?;
            let val = if let Some(target_ty) = struct_ty.get_field_type_at_index(field_idx) {
                self.coerce_type(*v, target_ty)?
            } else {
                *v
            };
            sv = self
                .builder
                .build_insert_value(sv, val, field_idx, "st_init")
                .map_err(|e| e.to_string())?
                .into_struct_value();
        }
        Ok(sv.into())
    }

    pub(crate) fn compile_field_ptr(
        &mut self,
        target: &Expr,
        field: &Ident,
    ) -> Result<(PointerValue<'ctx>, BasicTypeEnum<'ctx>), String> {
        let idx = self.find_field_index(target, &field.name).unwrap_or(0);
        let field_idx = u32::try_from(idx).map_err(|e| e.to_string())?;

        // Struct value types: GEP directly at the storage slot.
        if let Some((struct_ty, base)) = self.struct_value_base(target) {
            let field_ty = struct_ty
                .get_field_type_at_index(field_idx)
                .unwrap_or_else(|| self.context.i64_type().into());
            let gep = self
                .builder
                .build_struct_gep(struct_ty, base, field_idx, &field.name)
                .map_err(|e| e.to_string())?;
            return Ok((gep, field_ty));
        }

        let target_val = self.compile_expr(target)?;
        if target_val.is_pointer_value() {
            let pv = target_val.into_pointer_value();
            let sem_ty = self.expr_semantic_ty(target);
            let actual_ty = match &sem_ty {
                Some(Type::Ref { inner, .. }) => Some(inner.as_ref()),
                other => other.as_ref(),
            };
            let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
            let lowerer =
                crate::types::TypeLowerer::new(self.context, &td, self.target, self.registry);
            if let Some(Type::Class(cname)) = actual_ty {
                let struct_ty = lowerer.class_instance_type(&cname)?;
                let field_ty = struct_ty
                    .get_field_type_at_index(field_idx)
                    .unwrap_or_else(|| self.context.i64_type().into());
                let gep = self
                    .builder
                    .build_struct_gep(struct_ty, pv, field_idx, &field.name)
                    .map_err(|e| e.to_string())?;
                return Ok((gep, field_ty));
            } else if let Some(Type::Struct(sname)) = actual_ty {
                let struct_ty = lowerer.lower_named_struct(&sname)?;
                let field_ty = struct_ty
                    .get_field_type_at_index(field_idx)
                    .unwrap_or_else(|| self.context.i64_type().into());
                let gep = self
                    .builder
                    .build_struct_gep(struct_ty, pv, field_idx, &field.name)
                    .map_err(|e| e.to_string())?;
                return Ok((gep, field_ty));
            } else if let Some(Type::Cstruct(sname)) = actual_ty {
                let struct_ty = lowerer.lower_named_cstruct(&sname)?;
                let field_ty = struct_ty
                    .get_field_type_at_index(field_idx)
                    .unwrap_or_else(|| self.context.i64_type().into());
                let gep = self
                    .builder
                    .build_struct_gep(struct_ty, pv, field_idx, &field.name)
                    .map_err(|e| e.to_string())?;
                return Ok((gep, field_ty));
            }
        }
        Err(format!("cannot get field pointer for '{}'", field.name))
    }

    fn struct_value_base(
        &mut self,
        target: &Expr,
    ) -> Option<(inkwell::types::StructType<'ctx>, PointerValue<'ctx>)> {
        match &target.kind {
            ExprKind::Ident(id) => {
                let var = self.lookup_local(&id.name)?;
                if !matches!(var.ty, BasicTypeEnum::StructType(_)) {
                    return None;
                }
                // Nullable structs use a wrapper layout; only plain structs go here.
                let is_struct_semantic = var
                    .semantic_ty
                    .as_ref()
                    .map(|t| match t {
                        Type::Ref { inner, .. } => matches!(inner.as_ref(), Type::Struct(_) | Type::Cstruct(_)),
                        _ => matches!(t, Type::Struct(_) | Type::Cstruct(_)),
                    })
                    .unwrap_or(true);
                if !is_struct_semantic {
                    return None;
                }
                Some((var.ty.into_struct_type(), var.ptr))
            }
            ExprKind::FieldAccess(inner_target, inner_field) => {
                // String/slice descriptors are StructTypes too; require a struct field.
                if !matches!(
                    self.expr_semantic_ty(target).as_ref(),
                    Some(Type::Struct(_)) | Some(Type::Cstruct(_)) | None
                ) {
                    return None;
                }
                let (gep, field_ty) = self.compile_field_ptr(inner_target, inner_field).ok()?;
                if let BasicTypeEnum::StructType(st) = field_ty {
                    return Some((st, gep));
                }
                None
            }
            ExprKind::IndexAccess(slice_target, index) => {
                let target_val = self.compile_expr(slice_target).ok()?;
                let index_val = self.compile_expr(index).ok()?;
                let i64_ty = self.context.i64_type();
                let idx = self.coerce_type(index_val, i64_ty.into()).ok()?.into_int_value();
                let (elem_ty, _elem_size) = self.resolve_index_layout(slice_target, target_val).ok()?;
                if let BasicTypeEnum::StructType(st) = elem_ty {
                    let elem_ptr = self.intrinsic.emit_slice_elem_ptr(self.builder, target_val, idx, elem_ty).ok()?;
                    return Some((st, elem_ptr));
                }
                None
            }
            _ => None,
        }
    }

    fn compile_field_access(
        &mut self,
        target: &Expr,
        field: &Ident,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if let Some(Type::Class(ref cname)) = self.expr_semantic_ty(target) {
            if self.registry.has_getter(cname, &field.name) {
                let target_val = self.compile_expr(target)?;
                let getter_name = format!("__get_{}", field.name);
                return self.compile_class_instance_method_call_values(
                    cname,
                    target_val,
                    &getter_name,
                    &[],
                );
            }
        }

        if let Ok((gep, field_ty)) = self.compile_field_ptr(target, field) {
            return self
                .builder
                .build_load(field_ty, gep, &field.name)
                .map_err(|e| e.to_string());
        }

        let target_val = self.compile_expr(target)?;
        let idx = self.find_field_index(target, &field.name).unwrap_or(0);
        let field_idx = u32::try_from(idx).map_err(|e| e.to_string())?;
        self.compile_field_access_on_value(target, target_val, field_idx, &field.name)
    }

    fn compile_field_access_on_value(
        &mut self,
        target: &Expr,
        target_val: BasicValueEnum<'ctx>,
        field_idx: u32,
        field_name: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if target_val.is_struct_value() {
            let sv = target_val.into_struct_value();
            self.builder
                .build_extract_value(sv, field_idx, field_name)
                .map_err(|e| e.to_string())
        } else if target_val.is_pointer_value() {
            let pv = target_val.into_pointer_value();
            let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
            let lowerer = crate::types::TypeLowerer::new(self.context, &td, self.target, self.registry);
            let sem_ty = self.expr_semantic_ty(target);
            let actual_ty = match &sem_ty {
                Some(Type::Ref { inner, .. }) => Some(inner.as_ref()),
                other => other.as_ref(),
            };
            let struct_ty = match actual_ty {
                Some(Type::Struct(sname)) => match lowerer.lower_type(&Type::Struct((*sname).clone())) {
                    Ok(BasicTypeEnum::StructType(st)) => Some(st),
                    _ => None,
                },
                Some(Type::Class(cname)) => lowerer.class_instance_type(cname).ok(),
                _ => None,
            };
            if let Some(st) = struct_ty {
                let field_ty = st
                    .get_field_type_at_index(field_idx)
                    .unwrap_or_else(|| self.context.i64_type().into());
                let gep = self
                    .builder
                    .build_struct_gep(st, pv, field_idx, field_name)
                    .map_err(|e| e.to_string())?;
                self.builder
                    .build_load(field_ty, gep, field_name)
                    .map_err(|e| e.to_string())
            } else {
                Ok(self.context.i64_type().const_zero().into())
            }
        } else {
            Ok(self.context.i64_type().const_zero().into())
        }
    }

    fn compile_safe_call(
        &mut self,
        target: &Expr,
        method: &Ident,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let target_val = self.compile_expr(target)?;

        let is_pointer = target_val.is_pointer_value();
        let is_nullable_struct = if let BasicValueEnum::StructValue(sv) = target_val {
            sv.get_type().count_fields() == 2
                && sv.get_type().get_field_type_at_index(0) == Some(self.context.bool_type().into())
        } else {
            false
        };

        if !is_pointer && !is_nullable_struct {
            if args.is_empty() && self.find_field_index(target, &method.name).is_some() {
                return self.compile_field_access(target, method);
            }
            return self.compile_method_call(target, method, args);
        }

        let is_not_null = if is_pointer {
            let pv = target_val.into_pointer_value();
            self.builder
                .build_int_compare(
                    IntPredicate::NE,
                    pv,
                    self.ptr_type().const_null(),
                    "safe.not_null",
                )
                .map_err(|e| e.to_string())?
        } else {
            let sv = target_val.into_struct_value();
            self.builder
                .build_extract_value(sv, 0, "safe.is_some")
                .map_err(|e| e.to_string())?
                .into_int_value()
        };

        let call_bb = self.context.append_basic_block(self.fn_val, "safe.call");
        let null_bb = self.context.append_basic_block(self.fn_val, "safe.null");
        let merge_bb = self.context.append_basic_block(self.fn_val, "safe.merge");

        self.builder
            .build_conditional_branch(is_not_null, call_bb, null_bb)
            .map_err(|e| e.to_string())?;

        // non-null path
        self.builder.position_at_end(call_bb);
        let unwrapped_val = if is_nullable_struct {
            let sv = target_val.into_struct_value();
            self.builder
                .build_extract_value(sv, 1, "safe.unwrapped")
                .map_err(|e| e.to_string())?
        } else {
            target_val
        };

        let raw_res = if args.is_empty()
            && let Some(idx) = self.find_field_index(target, &method.name)
        {
            let field_idx = u32::try_from(idx).map_err(|e| e.to_string())?;
            self.compile_field_access_on_value(target, unwrapped_val, field_idx, &method.name)?
        } else {
            let target_ty = self.expr_semantic_ty(target);
            self.compile_method_call_on_value(unwrapped_val, target_ty.as_ref(), method, args)?
        };
        let call_end_bb = self.builder.get_insert_block().unwrap();

        // coerce or wrap into nullable repr
        let (res_llvm_ty, some_val, null_val): (
            BasicTypeEnum<'ctx>,
            BasicValueEnum<'ctx>,
            BasicValueEnum<'ctx>,
        ) = if raw_res.is_pointer_value() {
            let null_p: BasicValueEnum<'ctx> = self.ptr_type().const_null().into();
            (self.ptr_type().into(), raw_res, null_p)
        } else if let BasicValueEnum::StructValue(sv) = raw_res {
            if sv.get_type().count_fields() == 2
                && sv.get_type().get_field_type_at_index(0) == Some(self.context.bool_type().into())
            {
                let null_s: BasicValueEnum<'ctx> = sv.get_type().const_zero().into();
                (sv.get_type().into(), raw_res, null_s)
            } else {
                let opt_st = self.context.struct_type(
                    &[self.context.bool_type().into(), sv.get_type().into()],
                    false,
                );
                let undef = opt_st.get_undef();
                let true_b = self.context.bool_type().const_int(1, false);
                let s1 = self
                    .builder
                    .build_insert_value(undef, true_b, 0, "")
                    .map_err(|e| e.to_string())?;
                let s2 = self
                    .builder
                    .build_insert_value(s1, raw_res, 1, "")
                    .map_err(|e| e.to_string())?
                    .into_struct_value();
                let null_s = opt_st.const_zero().into();
                (opt_st.into(), s2.into(), null_s)
            }
        } else {
            let opt_st = self.context.struct_type(
                &[self.context.bool_type().into(), raw_res.get_type()],
                false,
            );
            let undef = opt_st.get_undef();
            let true_b = self.context.bool_type().const_int(1, false);
            let s1 = self
                .builder
                .build_insert_value(undef, true_b, 0, "")
                .map_err(|e| e.to_string())?;
            let s2 = self
                .builder
                .build_insert_value(s1, raw_res, 1, "")
                .map_err(|e| e.to_string())?
                .into_struct_value();
            let null_s = opt_st.const_zero().into();
            (opt_st.into(), s2.into(), null_s)
        };

        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| e.to_string())?;

        // 3. null path
        self.builder.position_at_end(null_bb);
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| e.to_string())?;

        // 4. merge path
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(res_llvm_ty, "safe.res")
            .map_err(|e| e.to_string())?;
        phi.add_incoming(&[(&some_val, call_end_bb), (&null_val, null_bb)]);
        Ok(phi.as_basic_value())
    }

    fn compile_array_init(
        &mut self,
        elements: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        if elements.is_empty() {
            let null_ptr = ptr_ty.const_null();
            let zero = i64_ty.const_zero();
            let sv = self.intrinsic.emit_slice_from_raw_parts(self.builder, null_ptr, zero, zero)?;
            return Ok(sv.into());
        }

        let mut vals = Vec::with_capacity(elements.len());
        for elem in elements {
            vals.push(self.compile_expr(elem)?);
        }

        let count = elements.len() as u64;
        let count_val = i64_ty.const_int(count, false);
        let elem_val = vals[0];
        let elem_ty = elem_val.get_type();
        let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
        let elem_size = td.get_bit_size(&elem_ty) / 8;
        let elem_size = elem_size.max(1);
        let total_size = count * elem_size;

        let raw_ptr = if let Some(reg) = self.dest_region.or(self.region_ptr) {
            self.region.emit_fast_path_alloc(
                self.module,
                self.builder,
                self.fn_val,
                reg,
                total_size,
                td.get_abi_alignment(&elem_ty).max(1) as u64,
            )?
        } else {
            let arr_ty = elem_ty.array_type(count as u32);
            self.builder.build_alloca(arr_ty, "arr_init").map_err(|e| e.to_string())?
        };

        let raw_int = self.builder.build_ptr_to_int(raw_ptr, i64_ty, "raw_int").map_err(|e| e.to_string())?;
        for (i, v) in vals.iter().enumerate() {
            let offset = (i as u64) * elem_size;
            let offset_val = i64_ty.const_int(offset, false);
            let addr = self.builder.build_int_add(raw_int, offset_val, "slot_addr").map_err(|e| e.to_string())?;
            let slot_ptr = self.builder.build_int_to_ptr(addr, ptr_ty, "slot_ptr").map_err(|e| e.to_string())?;
            self.builder.build_store(slot_ptr, *v).map_err(|e| e.to_string())?;
        }

        let sv = self.intrinsic.emit_slice_from_raw_parts(self.builder, raw_ptr, count_val, count_val)?;
        Ok(sv.into())
    }

    fn compile_array_alloc(
        &mut self,
        elem_ty_node: &TypeNode,
        count: usize,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        if count == 0 {
            let null_ptr = ptr_ty.const_null();
            let zero = i64_ty.const_zero();
            let sv = self.intrinsic.emit_slice_from_raw_parts(self.builder, null_ptr, zero, zero)?;
            return Ok(sv.into());
        }

        let elem_semantic_ty = ast_type_to_type(elem_ty_node);
        let (elem_llvm_ty, elem_size) = self.elem_type_to_layout(&elem_semantic_ty)?;
        let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
        let align = td.get_abi_alignment(&elem_llvm_ty).max(1);
        let elem_size = elem_size.max(1) as u64;
        let count_u64 = count as u64;
        let total_size = count_u64 * elem_size;

        let raw_ptr = if let Some(reg) = self.dest_region.or(self.region_ptr) {
            self.region.emit_fast_path_alloc(
                self.module,
                self.builder,
                self.fn_val,
                reg,
                total_size,
                align as u64,
            )?
        } else {
            let arr_ty = elem_llvm_ty.array_type(count as u32);
            self.builder.build_alloca(arr_ty, "arr_alloc").map_err(|e| e.to_string())?
        };

        let total_size_val = i64_ty.const_int(total_size, false);
        let i8_zero = self.context.i8_type().const_zero();
        self.builder
            .build_memset(raw_ptr, align, i8_zero, total_size_val)
            .map_err(|e| e.to_string())?;

        let count_val = i64_ty.const_int(count_u64, false);
        let sv = self.intrinsic.emit_slice_from_raw_parts(self.builder, raw_ptr, count_val, count_val)?;
        Ok(sv.into())
    }

    fn compile_tuple_init(
        &mut self,
        elements: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let mut vals = Vec::with_capacity(elements.len());
        for elem in elements {
            vals.push(self.compile_expr(elem)?);
        }
        let types: Vec<BasicTypeEnum<'ctx>> = vals.iter().map(BasicValueEnum::get_type).collect();
        let struct_ty = self.context.struct_type(&types, false);
        let mut sv = struct_ty.get_undef();
        for (i, v) in vals.iter().enumerate() {
            let field_idx = u32::try_from(i).map_err(|e| e.to_string())?;
            sv = self
                .builder
                .build_insert_value(sv, *v, field_idx, "tup_init")
                .map_err(|e| e.to_string())?
                .into_struct_value();
        }
        Ok(sv.into())
    }

    fn find_field_index(&self, target: &Expr, field_name: &str) -> Option<usize> {
        if let Ok(idx) = field_name.parse::<usize>() {
            return Some(idx);
        }
        if let Some(sem) = self.expr_semantic_ty(target) {
            let actual = match &sem {
                Type::Ref { inner, .. } => inner.as_ref(),
                other => other,
            };
            let unwrapped = actual.unwrap_nullable();
            match unwrapped {
                Type::Struct(name) => {
                    if let Some(layout) = self.registry.structs.get(&name) {
                        return layout.fields.iter().position(|(f, _)| f == field_name);
                    }
                    // Uncanonicalized cstruct annotations arrive as Struct;
                    // resolve them against the cstruct registry as well.
                    if let Some(layout) = self.registry.cstructs.get(&name) {
                        return layout.fields.iter().position(|(f, _)| f == field_name);
                    }
                }
                Type::Cstruct(name) => {
                    if let Some(layout) = self.registry.cstructs.get(&name) {
                        return layout.fields.iter().position(|(f, _)| f == field_name);
                    }
                }
                Type::Class(name) => {
                    if let Some(layout) = self.registry.classes.get(&name) {
                        return layout.fields.iter().position(|(f, _)| f == field_name).map(|i| i + 1);
                    }
                }
                _ => {}
            }
        }
        for layout in self.registry.structs.values() {
            if let Some(pos) = layout.fields.iter().position(|(f, _)| f == field_name) {
                return Some(pos);
            }
        }
        for layout in self.registry.cstructs.values() {
            if let Some(pos) = layout.fields.iter().position(|(f, _)| f == field_name) {
                return Some(pos);
            }
        }
        for layout in self.registry.classes.values() {
            if let Some(pos) = layout.fields.iter().position(|(f, _)| f == field_name) {
                return Some(pos + 1);
            }
        }
        None
    }

    fn compile_ptr_of(&mut self, ident: &Ident) -> Result<BasicValueEnum<'ctx>, String> {
        let local = self
            .lookup_local(&ident.name)
            .ok_or_else(|| format!("`ptr_of` variable '{}' not found", ident.name))?;
        Ok(local.ptr.into())
    }

    fn compile_ref(&mut self, expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr = self.compile_lvalue_ptr(expr)?;
        Ok(ptr.into())
    }

    pub(crate) fn compile_lvalue_ptr(&mut self, expr: &Expr) -> Result<PointerValue<'ctx>, String> {
        match &expr.kind {
            ExprKind::Ident(ident) => {
                if let Some(var) = self.lookup_local(&ident.name) {
                    Ok(var.ptr)
                } else if let Some(global) = self.module.get_global(&ident.name) {
                    Ok(global.as_pointer_value())
                } else {
                    Err(format!("undefined identifier '{}'", ident.name))
                }
            }
            ExprKind::FieldAccess(target, field) => {
                let (gep, _) = self.compile_field_ptr(target, field)?;
                Ok(gep)
            }
            _ => {
                let val = self.compile_expr(expr)?;
                let alloca = self
                    .builder
                    .build_alloca(val.get_type(), "ref.tmp")
                    .map_err(|e| e.to_string())?;
                self.builder.build_store(alloca, val).map_err(|e| e.to_string())?;
                Ok(alloca)
            }
        }
    }

    fn compile_ptr_cast(
        &mut self,
        target: &Expr,
        ty_node: &TypeNode,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let val = self.compile_expr(target)?;
        let raw_ty = ast_type_to_type(ty_node);
        let ast_ty = self.registry.canonicalize_type(&raw_ty);
        let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
        let lowerer =
            crate::types::TypeLowerer::new(self.context, &td, self.target, self.registry);
        let target_llvm_ty = lowerer.lower_type(&ast_ty)?;
        self.coerce_type(val, target_llvm_ty)
    }

    fn compile_is(
        &mut self,
        target: &Expr,
        ty_node: &TypeNode,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let raw_target_ty = ast_type_to_type(ty_node);
        let target_ty = self.registry.canonicalize_type(&raw_target_ty);
        let val = self.compile_expr(target)?;
        let sem_ty = self
            .expr_semantic_ty(target)
            .map(|t| self.registry.canonicalize_type(&t));

        // bull check
        if target_ty == Type::Null {
            if val.is_pointer_value() {
                let pv = val.into_pointer_value();
                let null = self.ptr_type().const_null();
                let cmp = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, pv, null, "is_null")
                    .map_err(|e| e.to_string())?;
                return Ok(cmp.into());
            } else if let BasicValueEnum::StructValue(sv) = val {
                if sv.get_type().count_fields() == 2
                    && sv.get_type().get_field_type_at_index(0)
                        == Some(self.context.bool_type().into())
                {
                    let is_some = self
                        .builder
                        .build_extract_value(sv, 0, "is_some")
                        .map_err(|e| e.to_string())?
                        .into_int_value();
                    let zero = self.context.bool_type().const_zero();
                    let cmp = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, is_some, zero, "is_null")
                        .map_err(|e| e.to_string())?;
                    return Ok(cmp.into());
                }
            }
            return Ok(self.context.bool_type().const_zero().into());
        }

        // target is cls or interface
        if matches!(target_ty, Type::Class(_) | Type::Interface(_)) {
            let inst_ptr = if val.is_pointer_value() {
                Some(val.into_pointer_value())
            } else if let BasicValueEnum::StructValue(sv) = val {
                if sv.get_type().count_fields() == 2
                    && sv.get_type().get_field_type_at_index(0) == Some(self.ptr_type().into())
                    && sv.get_type().get_field_type_at_index(1) == Some(self.ptr_type().into())
                {
                    Some(
                        self.builder
                            .build_extract_value(sv, 0, "fat.inst")
                            .map_err(|e| e.to_string())?
                            .into_pointer_value(),
                    )
                } 

                else { None }

            } 

            else { None };

            let Some(inst_ptr) = inst_ptr else {
                return Ok(self.context.bool_type().const_zero().into());
            };

            let mut matching_type_ids: Vec<u64> = Vec::new();
            match &target_ty {
                Type::Class(target_class) => {
                    for (cname, _) in &self.registry.classes {
                        if self.registry.class_is_subclass_of(cname, target_class) {
                            if let Some(tid) = self.vtable.get_class_type_id(cname) {
                                matching_type_ids.push(tid);
                            }
                        }
                    }
                }
                Type::Interface(target_iface) => {
                    for (cname, _) in &self.registry.classes {
                        if self.registry.class_implements_interface(cname, target_iface) {
                            if let Some(tid) = self.vtable.get_class_type_id(cname) {
                                matching_type_ids.push(tid);
                            }
                        }
                    }
                }
                _ => {}
            }

            let curr_bb = self.builder.get_insert_block().unwrap();
            let check_bb = self.context.append_basic_block(self.fn_val, "is.check");
            let merge_bb = self.context.append_basic_block(self.fn_val, "is.merge");

            let is_not_null = self
                .builder
                .build_int_compare(
                    IntPredicate::NE,
                    inst_ptr,
                    self.ptr_type().const_null(),
                    "is.not_null",
                )
                .map_err(|e| e.to_string())?;

            self.builder
                .build_conditional_branch(is_not_null, check_bb, merge_bb)
                .map_err(|e| e.to_string())?;

            self.builder.position_at_end(check_bb);
            let vt_ptr = self
                .builder
                .build_load(self.ptr_type(), inst_ptr, "is.vt_ptr")
                .map_err(|e| e.to_string())?
                .into_pointer_value();
            let actual_tid = self
                .builder
                .build_load(self.context.i64_type(), vt_ptr, "is.actual_tid")
                .map_err(|e| e.to_string())?
                .into_int_value();

            let is_match = if matching_type_ids.is_empty() {
                self.context.bool_type().const_zero()
            } else {
                let mut acc = None;
                for tid in &matching_type_ids {
                    let expected = self.context.i64_type().const_int(*tid, false);
                    let eq = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, actual_tid, expected, "is.eq")
                        .map_err(|e| e.to_string())?;
                    acc = match acc {
                        None => Some(eq),
                        Some(prev) => Some(
                            self.builder
                                .build_or(prev, eq, "is.or")
                                .map_err(|e| e.to_string())?,
                        ),
                    };
                }
                acc.unwrap()
            };

            let check_end_bb = self.builder.get_insert_block().unwrap();
            self.builder
                .build_unconditional_branch(merge_bb)
                .map_err(|e| e.to_string())?;

            self.builder.position_at_end(merge_bb);
            let phi = self
                .builder
                .build_phi(self.context.bool_type(), "is.res")
                .map_err(|e| e.to_string())?;

            let false_val: BasicValueEnum<'ctx> = self.context.bool_type().const_zero().into();
            let true_res: BasicValueEnum<'ctx> = is_match.into();

            phi.add_incoming(&[(&false_val, curr_bb), (&true_res, check_end_bb)]);

            return Ok(phi.as_basic_value());
        }

        // nullable struct { bool is_some, inner }
        if let BasicValueEnum::StructValue(sv) = val {
            if sv.get_type().count_fields() == 2
                && sv.get_type().get_field_type_at_index(0)
                    == Some(self.context.bool_type().into())
            {
                let is_some = self
                    .builder
                    .build_extract_value(sv, 0, "is_some")
                    .map_err(|e| e.to_string())?
                    .into_int_value();
                if let Some(Type::Union(members)) = &sem_ty {
                    let non_null = members.iter().find(|m| **m != Type::Null);
                    if let Some(inner_ty) = non_null {
                        if inner_ty == &target_ty {
                            return Ok(is_some.into());
                        }
                    }
                }
                let inner_field_ty = sv.get_type().get_field_type_at_index(1).unwrap();
                if let Ok(expected_llvm) = self.lower_type(&target_ty) {
                    if inner_field_ty == expected_llvm {
                        return Ok(is_some.into());
                    }
                }
                return Ok(self.context.bool_type().const_zero().into());
            }
        }

        // tagged union { i32 tag, [N x i8] payload }
        if let Some(Type::Union(members)) = &sem_ty {
            if let Some(target_tag) = members.iter().position(|m| m == &target_ty) {
                if let BasicValueEnum::StructValue(sv) = val {
                    if sv.get_type().count_fields() == 2
                        && sv.get_type().get_field_type_at_index(0)
                            == Some(self.context.i32_type().into())
                    {
                        let actual_tag = self
                            .builder
                            .build_extract_value(sv, 0, "union.tag")
                            .map_err(|e| e.to_string())?
                            .into_int_value();
                        let expected_tag =
                            self.context.i32_type().const_int(target_tag as u64, false);
                        let cmp = self
                            .builder
                            .build_int_compare(IntPredicate::EQ, actual_tag, expected_tag, "is_variant")
                            .map_err(|e| e.to_string())?;
                        return Ok(cmp.into());
                    }
                }
            }
            return Ok(self.context.bool_type().const_zero().into());
        }

        if let Some(ref from_ty) = sem_ty {
            if from_ty == &target_ty {
                return Ok(self.context.bool_type().const_int(1, false).into());
            } else {
                return Ok(self.context.bool_type().const_zero().into());
            }
        }

        // LLVM
        if let Ok(expected_llvm) = self.lower_type(&target_ty) {
            if val.get_type() == expected_llvm {
                return Ok(self.context.bool_type().const_int(1, false).into());
            }
        }

        Ok(self.context.bool_type().const_zero().into())
    }

    fn compile_lambda(&mut self, params: &[Ident], body: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        let ast_params = params
            .iter()
            .map(|id| yukic_ast::Param {
                name: id.clone(),
                is_ref: false,
                is_mut: false,
                is_variadic: false,
                ty: None,
                default_val: None,
                span: id.span,
            })
            .collect();
        let ret_stmt = yukic_ast::stmt::Stmt::new(
            yukic_ast::stmt::StmtKind::Return(Some(body.clone())),
            body.span,
        );
        let block = yukic_ast::stmt::Block {
            stmts: vec![ret_stmt],
            span: body.span,
        };
        let decl = yukic_ast::item::FunDecl {
            name: Ident::new("<anonymous>", body.span),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: ast_params,
            return_type: None,
            body: block,
            span: body.span,
        };
        self.compile_anonymous_fun(&decl)
    }

    fn compile_anonymous_fun(
        &mut self,
        fun: &yukic_ast::item::FunDecl,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // 1. Analyze free variable captures
        let mut used_idents = std::collections::HashSet::new();
        for stmt in &fun.body.stmts {
            collect_ident_names_in_stmt(stmt, &mut used_idents);
        }
        let mut local_declared = std::collections::HashSet::new();
        collect_declared_local_names(&fun.body, &mut local_declared);
        for p in &fun.params {
            local_declared.insert(p.name.name.clone());
        }

        let mut captures: Vec<crate::closure::CapturedVar> = Vec::new();
        for name in used_idents {
            if local_declared.contains(&name) {
                continue;
            }
            if let Some(var) = self.lookup_local(&name) {
                let sem_ty = var.semantic_ty.clone().unwrap_or(Type::Int64);
                captures.push(crate::closure::CapturedVar::new(name, sem_ty));
            }
        }
        captures.sort_by(|a, b| a.name.cmp(&b.name));

        // 2. Read values of captured variables from current function
        let mut cap_vals = Vec::with_capacity(captures.len());
        for cap in &captures {
            let val = self.compile_ident(&Ident::new(&cap.name, fun.span))?;
            cap_vals.push(val);
        }

        // 3. Determine parameter types and return type
        let mut param_types = Vec::with_capacity(fun.params.len());
        for p in &fun.params {
            let pty = if let Some(t) = &p.ty {
                self.registry.canonicalize_type(&ast_type_to_type(t))
            } else {
                Type::Int64
            };
            param_types.push(self.lower_type(&pty)?);
        }

        let ret_ty = if let Some(t) = &fun.return_type {
            self.registry.canonicalize_type(&ast_type_to_type(t))
        } else {
            deduce_closure_return_type(fun, &captures, self).unwrap_or(Type::Void)
        };
        let is_void = ret_ty == Type::Void;
        let llvm_ret = if is_void {
            None
        } else {
            Some(self.lower_type(&ret_ty)?)
        };

        // 4. Generate unique closure function name
        let mut counter = 0;
        let closure_name = loop {
            let name = format!("__yuki_closure_{counter}");
            if self.module.get_function(&name).is_none() {
                break name;
            }
            counter += 1;
        };

        // 5. Construct closure function type: [dest_region?] + [env_ptr] + user_params
        let needs_dest = crate::region::RegionManager::is_regional_type(&ret_ty);
        let mut all_param_types: Vec<inkwell::types::BasicMetadataTypeEnum<'ctx>> = Vec::new();
        if needs_dest {
            all_param_types.push(self.ptr_type().into());
        }
        all_param_types.push(self.ptr_type().into());
        for pt in &param_types {
            all_param_types.push((*pt).into());
        }

        let fn_type = if let Some(ret) = llvm_ret {
            ret.fn_type(&all_param_types, false)
        } else {
            self.context.void_type().fn_type(&all_param_types, false)
        };

        let closure_fn = self.module.add_function(&closure_name, fn_type, None);
        closure_fn.set_linkage(inkwell::module::Linkage::Internal);
        closure_fn.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32);

        // 6. Build environment layout if captures exist
        let manager = crate::closure::ClosureManager::new(self.context);
        let td = inkwell::targets::TargetData::create(self.target.spec().data_layout);
        let lowerer = crate::types::TypeLowerer::new(self.context, &td, self.target, self.registry);
        let env_layout = if !captures.is_empty() {
            Some(manager.create_env_layout(&closure_name, &captures, &lowerer)?)
        } else {
            None
        };

        // 7. Save outer compiler state and compile closure body
        let old_fn = self.fn_val;
        let old_return_type = self.return_type;
        let old_is_main = self.is_main;
        let old_dest_region = self.dest_region;
        let old_region_ptr = self.region_ptr;
        let old_unwind = self.unwind_dest;
        let old_cleanup_bb = self.current_cleanup_bb;
        let old_in_cleanup = self.in_cleanup;
        let old_loop_stack = std::mem::take(&mut self.loop_stack);
        let old_scopes = std::mem::take(&mut self.scopes);
        let old_cleanups = std::mem::replace(&mut self.cleanups_stack, crate::exception::ScopeStack::new());
        let old_insert_bb = self.builder.get_insert_block();

        self.fn_val = closure_fn;
        self.return_type = llvm_ret;
        self.is_main = false;
        self.region_ptr = None;
        self.unwind_dest = None;
        self.current_cleanup_bb = None;
        self.in_cleanup = false;
        self.loop_stack = Vec::new();
        self.scopes = vec![std::collections::HashMap::new()];
        self.cleanups_stack.push_scope(ScopeKind::Function);

        let entry = self.context.append_basic_block(closure_fn, "entry");
        self.builder.position_at_end(entry);

        if needs_dest {
            let dest_arg = closure_fn.get_first_param().map(|p| p.into_pointer_value());
            if let Some(dest_p) = dest_arg {
                dest_p.set_name("_dest_region");
            }
            self.dest_region = dest_arg;
        } else {
            self.dest_region = None;
        }

        let local_reg = if crate::region_elision::fun_needs_local_region(fun, needs_dest, self.registry) {
            Some(self.region.emit_region_prologue(
                self.module,
                self.builder,
                crate::region::DEFAULT_LOCAL_REGION_CAPACITY,
                "_local_region",
            )?)
        } else {
            None
        };
        self.region_ptr = local_reg;

        // Unpack captures from %env_ptr
        if let Some(layout) = &env_layout {
            let env_param = closure_fn
                .get_nth_param(if needs_dest { 1 } else { 0 })
                .unwrap()
                .into_pointer_value();
            env_param.set_name("_env_ptr");
            let unpacked = manager.unpack_environment(self.builder, env_param, layout)?;
            for (name, val) in unpacked {
                let alloca = self
                    .builder
                    .build_alloca(val.get_type(), &name)
                    .map_err(|e| e.to_string())?;
                self.builder.build_store(alloca, val).map_err(|e| e.to_string())?;
                let sem_ty = captures.iter().find(|c| c.name == name).map(|c| c.ty.clone());
                self.scopes.last_mut().unwrap().insert(
                    name,
                    super::LocalVar {
                        ptr: alloca,
                        ty: val.get_type(),
                        semantic_ty: sem_ty,
                    },
                );
            }
        }

        // Bind closure parameters
        let first_user_idx = if needs_dest { 2 } else { 1 };
        for (i, p) in fun.params.iter().enumerate() {
            let param_val = closure_fn.get_nth_param((first_user_idx + i) as u32).unwrap();
            param_val.set_name(&p.name.name);
            let sem_ty = p
                .ty
                .as_ref()
                .map(|t| self.registry.canonicalize_type(&ast_type_to_type(t)));
            if p.is_ref {
                let unref_sem = match &sem_ty {
                    Some(Type::Ref { inner, .. }) => Some(*inner.clone()),
                    other => other.clone(),
                };
                let ptr = param_val.into_pointer_value();
                let inner_ty = if let Some(ref t) = unref_sem {
                    self.lower_type(t)?
                } else {
                    param_val.get_type()
                };
                self.scopes.last_mut().unwrap().insert(
                    p.name.name.clone(),
                    super::LocalVar {
                        ptr,
                        ty: inner_ty,
                        semantic_ty: unref_sem,
                    },
                );
                continue;
            }
            let alloca = self
                .builder
                .build_alloca(param_val.get_type(), &p.name.name)
                .map_err(|e| e.to_string())?;
            self.builder.build_store(alloca, param_val).map_err(|e| e.to_string())?;
            self.scopes.last_mut().unwrap().insert(
                p.name.name.clone(),
                super::LocalVar {
                    ptr: alloca,
                    ty: param_val.get_type(),
                    semantic_ty: sem_ty,
                },
            );
        }

        // Compile body
        let _ = self.compile_block(&fun.body)?;

        // Ensure terminator exists
        if let Some(curr_bb) = self.builder.get_insert_block() {
            if curr_bb.get_terminator().is_none() {
                if let Some(reg) = self.region_ptr {
                    self.region.emit_region_epilogue(self.module, self.builder, reg)?;
                }
                if is_void {
                    self.builder.build_return(None).map_err(|e| e.to_string())?;
                } else if let Some(ret) = llvm_ret {
                    let zero = ret.const_zero();
                    self.builder.build_return(Some(&zero)).map_err(|e| e.to_string())?;
                }
            }
        }

        // 8. Restore outer compiler state
        self.fn_val = old_fn;
        self.return_type = old_return_type;
        self.is_main = old_is_main;
        self.dest_region = old_dest_region;
        self.region_ptr = old_region_ptr;
        self.unwind_dest = old_unwind;
        self.current_cleanup_bb = old_cleanup_bb;
        self.in_cleanup = old_in_cleanup;
        self.loop_stack = old_loop_stack;
        self.scopes = old_scopes;
        self.cleanups_stack = old_cleanups;
        if let Some(bb) = old_insert_bb {
            self.builder.position_at_end(bb);
        }

        // 9. Allocate & initialize closure environment in outer function
        let env_ptr = if let Some(layout) = &env_layout {
            let reg_ptr = self
                .dest_region
                .or(self.region_ptr)
                .unwrap_or_else(|| self.ptr_type().const_null());
            manager.allocate_and_init_env(
                self.module,
                self.builder,
                self.fn_val,
                self.region,
                reg_ptr,
                layout,
                &cap_vals,
            )?
        } else {
            self.ptr_type().const_null()
        };

        let fat_ptr = manager.make_closure_fat_pointer(
            self.builder,
            closure_fn.as_global_value().as_pointer_value(),
            env_ptr,
            "closure",
        )?;
        Ok(fat_ptr.into())
    }
}

pub(crate) fn deduce_closure_return_type(
    fun: &yukic_ast::item::FunDecl,
    captures: &[crate::closure::CapturedVar],
    bc: &BodyCompiler<'_, '_>,
) -> Option<Type> {
    let mut locals: std::collections::HashMap<String, Type> = std::collections::HashMap::new();
    for cap in captures {
        locals.insert(cap.name.clone(), cap.ty.clone());
    }
    for p in &fun.params {
        let ty = if let Some(t) = &p.ty {
            bc.registry.canonicalize_type(&ast_type_to_type(t))
        } else {
            Type::Int64
        };
        locals.insert(p.name.name.clone(), ty);
    }

    for stmt in &fun.body.stmts {
        if let yukic_ast::stmt::StmtKind::Return(Some(expr)) = &stmt.kind {
            if let Some(ty) = deduce_closure_expr_type(expr, &locals, bc) {
                if ty != Type::Void {
                    return Some(ty);
                }
            }
        }
    }
    if let Some(last_stmt) = fun.body.stmts.last() {
        if let yukic_ast::stmt::StmtKind::Expr(expr) = &last_stmt.kind {
            if let Some(ty) = deduce_closure_expr_type(expr, &locals, bc) {
                return Some(ty);
            }
        }
    }
    None
}

pub(crate) fn deduce_closure_expr_type(
    expr: &Expr,
    locals: &std::collections::HashMap<String, Type>,
    bc: &BodyCompiler<'_, '_>,
) -> Option<Type> {
    if let Some(ty) = bc.expr_semantic_ty(expr) {
        return Some(ty);
    }
    match &expr.kind {
        ExprKind::Ident(id) => {
            if let Some(ty) = locals.get(&id.name) {
                return Some(ty.clone());
            }
            if let Some(l) = bc.lookup_local(&id.name) {
                if let Some(sem) = &l.semantic_ty {
                    return Some(sem.clone());
                }
            }
            bc.registry.fn_types.get(&id.name).cloned()
        }
        ExprKind::This => locals.get("this").cloned(),
        ExprKind::Literal(lit) => match lit {
            LiteralValue::Int(_) => Some(Type::Int64),
            LiteralValue::UInt(_) => Some(Type::Uint64),
            LiteralValue::Float(_) => Some(Type::Float64),
            LiteralValue::String(_) => Some(Type::String),
            LiteralValue::Char(_) => Some(Type::Char),
            LiteralValue::Bool(_) => Some(Type::Bool),
            LiteralValue::Bytes(_) => Some(Type::Bytes),
            LiteralValue::Null => Some(Type::Null),
        },
        ExprKind::InterpolatedString(_) => Some(Type::String),
        ExprKind::Unary(op, inner) => match op {
            UnaryOp::Not => Some(Type::Bool),
            _ => deduce_closure_expr_type(inner, locals, bc),
        },
        ExprKind::Binary(op, left, right) => match op {
            BinaryOp::Eq
            | BinaryOp::NotEq
            | BinaryOp::Lt
            | BinaryOp::LtEq
            | BinaryOp::Gt
            | BinaryOp::GtEq
            | BinaryOp::And
            | BinaryOp::Or => Some(Type::Bool),
            BinaryOp::Add => {
                let lt = deduce_closure_expr_type(left, locals, bc);
                let rt = deduce_closure_expr_type(right, locals, bc);
                if matches!(lt, Some(Type::String)) || matches!(rt, Some(Type::String)) {
                    Some(Type::String)
                } else {
                    lt.or(rt)
                }
            }
            _ => {
                let lt = deduce_closure_expr_type(left, locals, bc);
                let rt = deduce_closure_expr_type(right, locals, bc);
                match (lt, rt) {
                    (Some(l), Some(r)) => {
                        if matches!(&left.kind, ExprKind::Literal(LiteralValue::Int(_))) && r.is_integer() {
                            Some(r)
                        } else if matches!(&right.kind, ExprKind::Literal(LiteralValue::Int(_))) && l.is_integer() {
                            Some(l)
                        } else {
                            Some(l)
                        }
                    }
                    (Some(l), None) => Some(l),
                    (None, Some(r)) => Some(r),
                    _ => None,
                }
            }
        },
        ExprKind::Block(block) | ExprKind::RegionBlock(block) => {
            for stmt in &block.stmts {
                if let yukic_ast::stmt::StmtKind::Return(Some(e)) = &stmt.kind {
                    if let Some(ty) = deduce_closure_expr_type(e, locals, bc) {
                        if ty != Type::Void {
                            return Some(ty);
                        }
                    }
                }
            }
            if let Some(last) = block.stmts.last() {
                if let yukic_ast::stmt::StmtKind::Expr(e) = &last.kind {
                    return deduce_closure_expr_type(e, locals, bc);
                }
            }
            None
        }
        ExprKind::If { then_branch, elif_branches, else_branch, .. } => {
            if let Some(ty) = deduce_closure_expr_type(&Expr::new(ExprKind::Block(then_branch.clone()), expr.span), locals, bc) {
                return Some(ty);
            }
            for (_, elif_block) in elif_branches {
                if let Some(ty) = deduce_closure_expr_type(&Expr::new(ExprKind::Block(elif_block.clone()), expr.span), locals, bc) {
                    return Some(ty);
                }
            }
            if let Some(eb) = else_branch {
                if let Some(ty) = deduce_closure_expr_type(&Expr::new(ExprKind::Block(eb.clone()), expr.span), locals, bc) {
                    return Some(ty);
                }
            }
            None
        }
        ExprKind::Ternary(_, t, e) => {
            deduce_closure_expr_type(t, locals, bc)
                .or_else(|| deduce_closure_expr_type(e, locals, bc))
        }
        ExprKind::Call(callee, args) => {
            match &callee.kind {
                ExprKind::Ident(id) => {
                    if let Some(Type::Fn { ret, .. }) = locals.get(&id.name).or_else(|| bc.registry.fn_types.get(&id.name)) {
                        return Some((**ret).clone());
                    }
                    if let Some(ret) = bc.registry.functions.get(&id.name) {
                        return Some(ret.clone());
                    }
                    if let Some(sym) = bc.registry.resolve_fun_symbol(&id.name, args.len()) {
                        if let Some(ret) = bc.registry.functions.get(&sym) {
                            return Some(ret.clone());
                        }
                    }
                    None
                }
                _ => None,
            }
        }
        ExprKind::MethodCall(target, method, args) | ExprKind::SafeCall(target, method, args) => {
            let target_ty = deduce_closure_expr_type(target, locals, bc);
            let class_name = match &target_ty {
                Some(Type::Class(name)) | Some(Type::Struct(name)) | Some(Type::Interface(name)) => {
                    Some(name.as_str())
                }
                _ => None,
            };
            if let Some(cname) = class_name {
                let key = format!("{cname}::{}", method.name);
                let mangled = crate::mangle::mangle_method_name(&method.name, args.len());
                let mangled_key = format!("{cname}::{mangled}");
                if let Some(ret) = bc
                    .registry
                    .functions
                    .get(&key)
                    .or_else(|| bc.registry.functions.get(&mangled_key))
                {
                    return Some(ret.clone());
                }
                if let Some(Type::Fn { ret, .. }) = bc
                    .registry
                    .fn_types
                    .get(&key)
                    .or_else(|| bc.registry.fn_types.get(&mangled_key))
                {
                    return Some((**ret).clone());
                }
            }
            let suffix = format!("::{}", method.name);
            for (fn_name, ret_ty) in &bc.registry.functions {
                if fn_name.ends_with(&suffix) {
                    return Some(ret_ty.clone());
                }
            }
            None
        }
        ExprKind::ClassInit(callee, _) => {
            if let ExprKind::Ident(id) = &callee.kind {
                Some(Type::Class(id.name.clone()))
            } else {
                None
            }
        }
        ExprKind::StructInit(id, _) => Some(Type::Struct(id.name.clone())),
        ExprKind::ArrayInit(elements) => {
            if let Some(first) = elements.first() {
                let elem_ty = deduce_closure_expr_type(first, locals, bc).unwrap_or(Type::Infer(0));
                Some(Type::Array(Box::new(elem_ty)))
            } else {
                Some(Type::Array(Box::new(Type::Infer(0))))
            }
        }
        ExprKind::FieldAccess(target, field) => {
            let target_ty = deduce_closure_expr_type(target, locals, bc);
            match target_ty {
                Some(Type::Class(ref cname)) => {
                    if let Some(layout) = bc.registry.classes.get(cname) {
                        if let Some((_, fty)) = layout.fields.iter().find(|(n, _)| n == &field.name) {
                            return Some(fty.clone());
                        }
                    }
                }
                Some(Type::Struct(ref sname)) => {
                    if let Some(layout) = bc.registry.structs.get(sname) {
                        if let Some((_, fty)) = layout.fields.iter().find(|(n, _)| n == &field.name) {
                            return Some(fty.clone());
                        }
                    }
                }
                _ => {}
            }
            None
        }
        ExprKind::IndexAccess(target, _) => {
            if let Some(Type::Array(elem)) = deduce_closure_expr_type(target, locals, bc) {
                Some(*elem)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn collect_ident_names(expr: &Expr, idents: &mut std::collections::HashSet<String>) {
    match &expr.kind {
        ExprKind::Ident(id) => {
            idents.insert(id.name.clone());
        }
        ExprKind::Unary(_, inner) => collect_ident_names(inner, idents),
        ExprKind::Binary(_, l, r) => {
            collect_ident_names(l, idents);
            collect_ident_names(r, idents);
        }
        ExprKind::Ternary(c, t, e) => {
            collect_ident_names(c, idents);
            collect_ident_names(t, idents);
            collect_ident_names(e, idents);
        }
        ExprKind::Call(callee, args) => {
            collect_ident_names(callee, idents);
            for a in args {
                collect_ident_names(a, idents);
            }
        }
        ExprKind::MethodCall(target, _, args) | ExprKind::SafeCall(target, _, args) => {
            collect_ident_names(target, idents);
            for a in args {
                collect_ident_names(a, idents);
            }
        }
        ExprKind::IndexAccess(target, idx) => {
            collect_ident_names(target, idents);
            collect_ident_names(idx, idents);
        }
        ExprKind::FieldAccess(target, _) => collect_ident_names(target, idents),
        ExprKind::SliceAccess(target, s, e, _) => {
            collect_ident_names(target, idents);
            if let Some(x) = s {
                collect_ident_names(x, idents);
            }
            if let Some(x) = e {
                collect_ident_names(x, idents);
            }
        }
        ExprKind::Block(b) | ExprKind::RegionBlock(b) => {
            for s in &b.stmts {
                collect_ident_names_in_stmt(s, idents);
            }
        }
        ExprKind::If {
            cond,
            then_branch,
            elif_branches,
            else_branch,
        } => {
            collect_ident_names(cond, idents);
            for s in &then_branch.stmts {
                collect_ident_names_in_stmt(s, idents);
            }
            for (c, b) in elif_branches {
                collect_ident_names(c, idents);
                for s in &b.stmts {
                    collect_ident_names_in_stmt(s, idents);
                }
            }
            if let Some(eb) = else_branch {
                for s in &eb.stmts {
                    collect_ident_names_in_stmt(s, idents);
                }
            }
        }
        ExprKind::ArrayInit(elems) | ExprKind::TupleInit(elems) => {
            for e in elems {
                collect_ident_names(e, idents);
            }
        }
        ExprKind::StructInit(_, fields) => {
            for f in fields {
                collect_ident_names(&f.value, idents);
            }
        }
        ExprKind::InterpolatedString(parts) => {
            for p in parts {
                if let StringPart::Expr(e) = p {
                    collect_ident_names(e, idents);
                }
            }
        }
        _ => {}
    }
}

fn collect_ident_names_in_stmt(stmt: &yukic_ast::stmt::Stmt, idents: &mut std::collections::HashSet<String>) {
    match &stmt.kind {
        yukic_ast::stmt::StmtKind::Set { init, .. } => collect_ident_names(init, idents),
        yukic_ast::stmt::StmtKind::Rewrite { target, value, .. } => {
            collect_ident_names_in_lvalue(target, idents);
            collect_ident_names(value, idents);
        }
        yukic_ast::stmt::StmtKind::Return(Some(expr)) | yukic_ast::stmt::StmtKind::Expr(expr) => {
            collect_ident_names(expr, idents);
        }
        _ => {}
    }
}

fn collect_ident_names_in_lvalue(lval: &yukic_ast::stmt::LValue, idents: &mut std::collections::HashSet<String>) {
    match lval {
        yukic_ast::stmt::LValue::Ident(id) => {
            idents.insert(id.name.clone());
        }
        yukic_ast::stmt::LValue::Field(expr, _) => {
            collect_ident_names(expr, idents);
        }
        yukic_ast::stmt::LValue::Index(expr, idx) => {
            collect_ident_names(expr, idents);
            collect_ident_names(idx, idents);
        }
    }
}

fn collect_declared_in_target(target: &yukic_ast::stmt::BindingTarget, declared: &mut std::collections::HashSet<String>) {
    match target {
        yukic_ast::stmt::BindingTarget::Ident(id) => {
            declared.insert(id.name.clone());
        }
        yukic_ast::stmt::BindingTarget::Tuple(targets) => {
            for t in targets {
                collect_declared_in_target(t, declared);
            }
        }
        yukic_ast::stmt::BindingTarget::Array(elems) => {
            for elem in elems {
                collect_declared_in_target(&elem.target, declared);
            }
        }
        yukic_ast::stmt::BindingTarget::Map(fields) => {
            for f in fields {
                if let Some(alias) = &f.alias {
                    collect_declared_in_target(alias, declared);
                } else {
                    declared.insert(f.field.name.clone());
                }
            }
        }
    }
}

fn collect_declared_local_names(block: &Block, declared: &mut std::collections::HashSet<String>) {
    for s in &block.stmts {
        if let yukic_ast::stmt::StmtKind::Set { target, .. } = &s.kind {
            collect_declared_in_target(target, declared);
        }
    }
}
