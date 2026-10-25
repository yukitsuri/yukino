#![deny(unsafe_code)]

use inkwell::targets::TargetData;
use inkwell::values::BasicValueEnum;
use yukic_ast::expr::{BinaryOp, Expr};
use yukic_ast::stmt::{AssignOp, BindingTarget, LValue, Stmt, StmtKind};
use yukic_ast::types::TypeNode;
use yukic_types::Type;
use yukic_types::convert::ast_type_to_type;

use super::BodyCompiler;
use crate::exception::CleanupAction;
use crate::types::TypeLowerer;

impl<'ctx> BodyCompiler<'_, 'ctx> {
    pub fn compile_stmt(&mut self, stmt: &Stmt) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if self.is_terminated() {
            return Ok(None);
        }
        match &stmt.kind {
            StmtKind::Set { target, ty, init } => {
                self.compile_set(target, ty.as_ref(), init)?;
                Ok(None)
            }
            StmtKind::Static(decl) => {
                self.compile_set(
                    &BindingTarget::Ident(decl.name.clone()),
                    decl.ty.as_ref(),
                    &decl.init,
                )?;
                Ok(None)
            }
            StmtKind::Rewrite { target, op, value } => {
                self.compile_rewrite(target, *op, value)?;
                Ok(None)
            }
            StmtKind::Expr(expr) => {
                let val = self.compile_expr(expr)?;
                Ok(Some(val))
            }
            StmtKind::Defer(block) => {
                let personality = self.get_or_declare_personality();
                self.fn_val.set_personality_function(personality);
                self.cleanups_stack
                    .add_cleanup(CleanupAction::Defer(block.clone()))?;
                self.current_cleanup_bb = None;
                Ok(None)
            }
            StmtKind::Return(expr) => {
                self.compile_return(expr.as_ref())?;
                Ok(None)
            }
            StmtKind::Break(label) => {
                self.compile_break(label.as_ref().map(|id| id.name.as_str()))?;
                Ok(None)
            }
            StmtKind::Continue(label) => {
                self.compile_continue(label.as_ref().map(|id| id.name.as_str()))?;
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn compile_set(
        &mut self,
        target: &BindingTarget,
        ty: Option<&TypeNode>,
        init: &Expr,
    ) -> Result<(), String> {
        let init_val = self.compile_expr(init)?;
        let (val_ty, sem_ty) = if let Some(type_node) = ty {
            let ast_ty = self.registry.canonicalize_type(&ast_type_to_type(type_node));
            let td = TargetData::create(self.target.spec().data_layout);
            let lowerer = TypeLowerer::new(self.context, &td, self.target, self.registry);
            (lowerer.lower_type(&ast_ty)?, Some(ast_ty))
        } else {
            (init_val.get_type(), self.expr_semantic_ty(init))
        };

        match target {
            BindingTarget::Ident(ident) => {
                let alloca = self
                    .builder
                    .build_alloca(val_ty, &ident.name)
                    .map_err(|e| e.to_string())?;
                let canon_sem_ty = sem_ty.as_ref().map(|t| self.registry.canonicalize_type(t));
                let coerced = if let Some(Type::Interface(ref iname)) = canon_sem_ty {
                    if init_val.is_pointer_value() {
                        self.coerce_to_interface(init_val.into_pointer_value(), iname)?.into()
                    } else {
                        self.coerce_type(init_val, val_ty)?
                    }
                } else {
                    self.coerce_type(init_val, val_ty)?
                };
                self.builder
                    .build_store(alloca, coerced)
                    .map_err(|e| e.to_string())?;
                self.add_local_with_ty(ident.name.clone(), alloca, val_ty, sem_ty);
                Ok(())
            }
            BindingTarget::Tuple(sub_targets) => {
                if let BasicValueEnum::StructValue(sv) = init_val {
                    for (i, sub) in sub_targets.iter().enumerate() {
                        let field_idx = u32::try_from(i).map_err(|e| e.to_string())?;
                        let field_val = self
                            .builder
                            .build_extract_value(sv, field_idx, "")
                            .map_err(|e| e.to_string())?;
                        if let BindingTarget::Ident(id) = sub {
                            let fty = field_val.get_type();
                            let alloca = self
                                .builder
                                .build_alloca(fty, &id.name)
                                .map_err(|e| e.to_string())?;
                            self.builder
                                .build_store(alloca, field_val)
                                .map_err(|e| e.to_string())?;
                            self.add_local(id.name.clone(), alloca, fty);
                        }
                    }
                }
                Ok(())
            }
            _ => Err("unsupported binding target in set statement".to_string()),
        }
    }

    fn compile_rewrite(
        &mut self,
        target: &LValue,
        op: AssignOp,
        value: &Expr,
    ) -> Result<(), String> {
        let rhs_val = self.compile_expr(value)?;
        match target {
            LValue::Ident(ident) => self.compile_rewrite_ident(ident, op, rhs_val),
            LValue::Field(target, field) => self.compile_rewrite_field(target, field, op, rhs_val),
            LValue::Index(target, index) => self.compile_rewrite_index(target, index, op, rhs_val),
        }
    }

    fn compile_rewrite_ident(
        &mut self,
        ident: &yukic_ast::Ident,
        op: AssignOp,
        rhs_val: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let local = self
            .lookup_local(&ident.name)
            .cloned()
            .ok_or_else(|| format!("variable '{}' not found", ident.name))?;

        let final_val = if op == AssignOp::Eq {
            self.coerce_type(rhs_val, local.ty)?
        } else if let Some(bin_op) = assign_op_to_binary_op(op) {
            let lhs_val = self
                .builder
                .build_load(local.ty, local.ptr, &ident.name)
                .map_err(|e| e.to_string())?;
            let res = self.compile_binary_op_with_type(
                bin_op,
                local.semantic_ty.as_ref(),
                lhs_val,
                rhs_val,
            )?;
            self.coerce_type(res, local.ty)?
        } else {
            return Err(format!("unsupported assignment operator {op:?}"));
        };

        self.builder
            .build_store(local.ptr, final_val)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn compile_rewrite_field(
        &mut self,
        target: &Expr,
        field: &yukic_ast::Ident,
        op: AssignOp,
        rhs_val: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        if let Some(Type::Class(ref cname)) = self.expr_semantic_ty(target) {
            if self.registry.has_setter(cname, &field.name) {
                let target_val = self.compile_expr(target)?;
                let final_val = if op == AssignOp::Eq {
                    rhs_val
                } else if let Some(bin_op) = assign_op_to_binary_op(op) {
                    let getter_name = format!("__get_{}", field.name);
                    let lhs_val = self.compile_class_instance_method_call_values(
                        cname,
                        target_val,
                        &getter_name,
                        &[],
                    )?;
                    self.compile_binary_op_with_type(bin_op, None, lhs_val, rhs_val)?
                } else {
                    return Err(format!("unsupported assignment operator {op:?}"));
                };
                let setter_name = format!("__set_{}", field.name);
                self.compile_class_instance_method_call_values(
                    cname,
                    target_val,
                    &setter_name,
                    &[final_val],
                )?;
                return Ok(());
            }
        }

        let (gep, field_ty) = self.compile_field_ptr(target, field)?;
        let final_val = if op == AssignOp::Eq {
            self.coerce_type(rhs_val, field_ty)?
        } else if let Some(bin_op) = assign_op_to_binary_op(op) {
            let lhs_val = self
                .builder
                .build_load(field_ty, gep, &field.name)
                .map_err(|e| e.to_string())?;
            let res = self.compile_binary_op_with_type(bin_op, None, lhs_val, rhs_val)?;
            self.coerce_type(res, field_ty)?
        } else {
            return Err(format!("unsupported assignment operator {op:?}"));
        };

        self.builder
            .build_store(gep, final_val)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn compile_rewrite_index(
        &mut self,
        target: &Expr,
        index: &Expr,
        op: AssignOp,
        rhs_val: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        if op != AssignOp::Eq {
            return Err(format!("unsupported assignment operator {op:?} for index"));
        }
        let target_val = self.compile_expr(target)?;
        let index_val = self.compile_expr(index)?;
        if let Some(Type::Class(class_name)) = self.expr_semantic_ty(target) {
            if self.find_class_method(&class_name, "[]=", 2).is_some() {
                self.compile_class_instance_method_call_values(
                    &class_name,
                    target_val,
                    "[]=",
                    &[index_val, rhs_val],
                )?;
                return Ok(());
            }
        }

        let i64_ty = self.context.i64_type();
        let idx = self.coerce_type(index_val, i64_ty.into())?.into_int_value();
        let (elem_ty, _elem_size) = self.resolve_index_layout(target, target_val)?;
        let elem_ptr = self
            .intrinsic
            .emit_slice_elem_ptr(self.builder, target_val, idx, elem_ty)?;
        let coerced_rhs = self.coerce_type(rhs_val, elem_ty)?;
        self.builder
            .build_store(elem_ptr, coerced_rhs)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn compile_binary_op_with_type(
        &mut self,
        op: BinaryOp,
        lhs_ty: Option<&Type>,
        lhs_val: BasicValueEnum<'ctx>,
        rhs_val: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if let Some(Type::Class(class_name)) = lhs_ty {
            let op_name = crate::mangle::binary_op_to_symbol(op);
            if self.find_class_method(class_name, op_name, 1).is_some() {
                return self.compile_class_instance_method_call_values(
                    class_name,
                    lhs_val,
                    op_name,
                    &[rhs_val],
                );
            }
        }
        let is_unsigned = lhs_ty.map(|t| t.is_unsigned_integer()).unwrap_or(false);
        self.compile_binary_values_typed(op, lhs_val, rhs_val, is_unsigned)
    }

    fn compile_return(&mut self, expr: Option<&Expr>) -> Result<(), String> {
        let ret_val = match expr {
            Some(e) => Some(self.compile_expr(e)?),
            None => None,
        };

        let cleanups = self.cleanups_stack.collect_return_cleanups();
        for cleanup in &cleanups {
            self.execute_cleanup(cleanup)?;
        }

        if let Some(reg) = self.region_ptr {
            self.region
                .emit_region_epilogue(self.module, self.builder, reg)?;
        }

        if self.is_main {
            let zero = self.context.i32_type().const_int(0, false);
            let exit_code = match ret_val {
                Some(v) => self.coerce_type(v, self.context.i32_type().into())?.into_int_value(),
                None => zero,
            };
            self.builder
                .build_return(Some(&exit_code))
                .map_err(|e| e.to_string())?;
        } else {
            match (ret_val, self.return_type) {
                (Some(val), Some(expected_ty)) => {
                    let coerced = self.coerce_type(val, expected_ty)?;
                    self.builder
                        .build_return(Some(&coerced))
                        .map_err(|e| e.to_string())?;
                }
                (Some(_), None) => {
                    self.builder
                        .build_return(None)
                        .map_err(|e| e.to_string())?;
                }
                (None, Some(expected_ty)) => {
                    let zero = expected_ty.const_zero();
                    self.builder
                        .build_return(Some(&zero))
                        .map_err(|e| e.to_string())?;
                }
                (None, None) => {
                    self.builder.build_return(None).map_err(|e| e.to_string())?;
                }
            }
        }

        Ok(())
    }

    fn compile_break(&mut self, label: Option<&str>) -> Result<(), String> {
        let (loop_cleanups, break_bb) = self
            .cleanups_stack
            .collect_loop_cleanups(true, label)
            .map_err(|e| e)?;

        for cleanup in &loop_cleanups {
            self.execute_cleanup(cleanup)?;
        }

        self.builder
            .build_unconditional_branch(break_bb)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn compile_continue(&mut self, label: Option<&str>) -> Result<(), String> {
        let (loop_cleanups, continue_bb) = self
            .cleanups_stack
            .collect_loop_cleanups(false, label)
            .map_err(|e| e)?;

        for cleanup in &loop_cleanups {
            self.execute_cleanup(cleanup)?;
        }

        self.builder
            .build_unconditional_branch(continue_bb)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn assign_op_to_binary_op(op: AssignOp) -> Option<BinaryOp> {
    match op {
        AssignOp::PlusEq => Some(BinaryOp::Add),
        AssignOp::MinusEq => Some(BinaryOp::Sub),
        AssignOp::StarEq => Some(BinaryOp::Mul),
        AssignOp::SlashEq => Some(BinaryOp::Div),
        AssignOp::PercentEq => Some(BinaryOp::Rem),
        AssignOp::StarStarEq => Some(BinaryOp::Pow),
        AssignOp::AmpEq => Some(BinaryOp::BitAnd),
        AssignOp::PipeEq => Some(BinaryOp::BitOr),
        AssignOp::CaretEq => Some(BinaryOp::BitXor),
        AssignOp::ShlEq => Some(BinaryOp::Shl),
        AssignOp::ShrEq => Some(BinaryOp::Shr),
        _ => None,
    }
}
