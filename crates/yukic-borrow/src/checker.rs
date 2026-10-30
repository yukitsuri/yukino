#![deny(unsafe_code)]
#![allow(clippy::collapsible_if, clippy::wildcard_imports)]

use std::collections::HashMap;

use yukic_ast::*;
use yukic_diagnostics::Diagnostic;
use yukic_diagnostics::code::ErrorCode;
use yukic_span::Span;
use yukic_types::{Type, TypeParam, ast_type_to_type};

use crate::affinity::TypeAffinity;
use crate::escape::{EscapeAnalysis, EscapeInfo};
use crate::state::{BorrowRecord, StateEnv, VarState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveContext {
    None,
    Move,
    Return,
}

impl MoveContext {
    pub fn is_move(self) -> bool {
        matches!(self, Self::Move | Self::Return)
    }

    pub fn is_return(self) -> bool {
        matches!(self, Self::Return)
    }
}

#[derive(Clone)]
pub struct BorrowChecker {
    pub env: StateEnv,
    pub escape_info: EscapeInfo,
    pub diagnostics: Vec<Diagnostic>,
    pub fn_returns: HashMap<String, Type>,
    pub fn_types: HashMap<String, Type>,
    pub type_fields: HashMap<String, HashMap<String, Type>>,
    loop_depth: usize,
    loop_scope_threshold: usize,
}

impl BorrowChecker {
    pub fn new() -> Self {
        let mut fn_returns = HashMap::new();
        fn_returns.insert("typeof".to_string(), Type::String);
        fn_returns.insert("__typeof_intrinsic__".to_string(), Type::String);
        fn_returns.insert("raw_args".to_string(), Type::String);
        fn_returns.insert("args".to_string(), Type::Array(Box::new(Type::String)));
        fn_returns.insert("exit".to_string(), Type::Never);
        fn_returns.insert("panic".to_string(), Type::Never);
        fn_returns.insert("assert".to_string(), Type::Void);
        fn_returns.insert("assert_eq".to_string(), Type::Void);
        fn_returns.insert("assert_ne".to_string(), Type::Void);

        Self {
            env: StateEnv::new(),
            escape_info: EscapeInfo::new(),
            diagnostics: Vec::new(),
            fn_returns,
            fn_types: HashMap::new(),
            type_fields: HashMap::new(),
            loop_depth: 0,
            loop_scope_threshold: 0,
        }
    }

    pub fn check_module(&mut self, module: &Module) -> Result<(), Vec<Diagnostic>> {
        self.register_item_signatures(&module.items);

        for item in &module.items {
            match &item.kind {
                ItemKind::Fun(decl) => self.check_fun(decl),
                ItemKind::Class(decl) => self.check_class(decl),
                ItemKind::Static(decl) => {
                    let ty = match &decl.ty {
                        Some(t) => ast_type_to_type(t),
                        None => self.infer_expr_type(&decl.init).unwrap_or(Type::Error),
                    };
                    self.check_expr(&decl.init, MoveContext::None);
                    self.env.declare_var(decl.name.name.clone(), ty, VarState::Owned, decl.name.span, false);
                }
                _ => {}
            }
        }

        let (escape_info, escape_diags) = EscapeAnalysis::analyze_module(module);
        self.escape_info = escape_info;
        self.diagnostics.extend(escape_diags);

        if self.diagnostics.iter().any(Diagnostic::is_error) {
            Err(self.diagnostics.clone())
        } else {
            Ok(())
        }
    }

    fn register_item_signatures(&mut self, items: &[Item]) {
        for item in items {
            self.register_item_sig(&item.kind);
        }
    }

    fn register_item_sig(&mut self, kind: &ItemKind) {
        match kind {
            ItemKind::Fun(decl) => {
                let ret = decl.return_type.as_ref().map_or(Type::Void, ast_type_to_type);
                self.fn_returns.insert(decl.name.name.clone(), ret.clone());
                let params = decl
                    .params
                    .iter()
                    .map(|p| {
                        TypeParam::new(
                            p.ty.as_ref().map_or(Type::Infer(0), ast_type_to_type),
                            p.is_ref,
                            p.is_mut,
                        )
                    })
                    .collect();
                self.fn_types.insert(
                    decl.name.name.clone(),
                    Type::Fn {
                        params,
                        ret: Box::new(ret),
                        is_cfun: false,
                    },
                );
            }
            ItemKind::Cfun(decl) => {
                let ret = decl.return_type.as_ref().map_or(Type::Void, ast_type_to_type);
                self.fn_returns.insert(decl.name.name.clone(), ret.clone());
                let params = decl
                    .params
                    .iter()
                    .map(|p| TypeParam::value(ast_type_to_type(&p.ty)))
                    .collect();
                self.fn_types.insert(
                    decl.name.name.clone(),
                    Type::Fn {
                        params,
                        ret: Box::new(ret),
                        is_cfun: true,
                    },
                );
            }
            ItemKind::Externc(block) => self.register_externc_sig(block),
            ItemKind::Class(decl) => {
                let mut fields = HashMap::new();
                for f in &decl.fields {
                    fields.insert(f.name.name.clone(), ast_type_to_type(&f.ty));
                }
                self.type_fields.insert(decl.name.name.clone(), fields);
                self.register_class_sig(decl);
            }
            ItemKind::Struct(decl) => {
                let mut fields = HashMap::new();
                for f in &decl.fields {
                    fields.insert(f.name.name.clone(), ast_type_to_type(&f.ty));
                }
                self.type_fields.insert(decl.name.name.clone(), fields);
            }
            ItemKind::Cstruct(decl) => {
                let mut fields = HashMap::new();
                for f in &decl.fields {
                    fields.insert(f.name.name.clone(), ast_type_to_type(&f.ty));
                }
                self.type_fields.insert(decl.name.name.clone(), fields);
            }
            _ => {}
        }
    }

    fn register_externc_sig(&mut self, block: &ExterncBlock) {
        for ext_item in &block.items {
            match ext_item {
                ExterncItem::Cfun(decl) => {
                    let ret = decl.return_type.as_ref().map_or(Type::Void, ast_type_to_type);
                    self.fn_returns.insert(decl.name.name.clone(), ret.clone());
                    let params = decl
                        .params
                        .iter()
                        .map(|p| TypeParam::value(ast_type_to_type(&p.ty)))
                        .collect();
                    self.fn_types.insert(
                        decl.name.name.clone(),
                        Type::Fn {
                            params,
                            ret: Box::new(ret),
                            is_cfun: true,
                        },
                    );
                }
                ExterncItem::Cstruct(decl) => {
                    let mut fields = HashMap::new();
                    for f in &decl.fields {
                        fields.insert(f.name.name.clone(), ast_type_to_type(&f.ty));
                    }
                    self.type_fields.insert(decl.name.name.clone(), fields);
                }
            }
        }
    }

    fn register_class_sig(&mut self, decl: &ClassDecl) {
        self.fn_returns.insert(decl.name.name.clone(), Type::Class(decl.name.name.clone()));
        for method in &decl.methods {
            let ret = method.return_type.as_ref().map_or(Type::Void, ast_type_to_type);
            self.fn_returns.insert(format!("{}::{}", decl.name.name, method.name.name), ret.clone());
            self.fn_returns.insert(method.name.name.clone(), ret.clone());
            let params = method
                .params
                .iter()
                .map(|p| {
                    TypeParam::new(
                        p.ty.as_ref().map_or(Type::Infer(0), ast_type_to_type),
                        p.is_ref,
                        p.is_mut,
                    )
                })
                .collect();
            let sig_ty = Type::Fn {
                params,
                ret: Box::new(ret),
                is_cfun: false,
            };
            self.fn_types.insert(format!("{}::{}", decl.name.name, method.name.name), sig_ty.clone());
            self.fn_types.insert(method.name.name.clone(), sig_ty);
        }
    }

    pub fn check_fun(&mut self, decl: &FunDecl) {
        self.env.push_scope();

        for param in &decl.params {
            let ty = match &param.ty {
                Some(ty_node) => {
                    let mut t = ast_type_to_type(ty_node);
                    if param.is_variadic && !matches!(t, Type::Array(_)) {
                        t = Type::Array(Box::new(t));
                    }
                    t
                }
                None => Type::Int64,
            };
            let state = if param.is_ref {
                if param.is_mut {
                    VarState::Owned
                } else {
                    VarState::BorrowedShared {
                        count: 1,
                        borrows: vec![param.span],
                    }
                }
            } else {
                VarState::Owned
            };
            self.env
                .declare_var(param.name.name.clone(), ty, state, param.span, param.is_ref);
        }

        self.check_block(&decl.body);

        if let Err(diags) = self.env.pop_scope() {
            self.diagnostics.extend(diags);
        }
    }

    fn check_class(&mut self, decl: &ClassDecl) {
        for method in &decl.methods {
            self.check_method(decl, method);
        }
    }

    pub fn check_method(&mut self, class_decl: &ClassDecl, decl: &MethodDecl) {
        self.env.push_scope();

        if decl.this_param != yukic_ast::item::ThisParam::None {
            let is_ref = decl.this_param == yukic_ast::item::ThisParam::Ref;
            let state = if is_ref {
                VarState::BorrowedShared {
                    count: 1,
                    borrows: vec![decl.span],
                }
            } else {
                VarState::Owned
            };
            self.env.declare_var(
                "this".to_string(),
                Type::Class(class_decl.name.name.clone()),
                state,
                decl.span,
                is_ref,
            );
        }

        for param in &decl.params {
            let ty = match &param.ty {
                Some(ty_node) => {
                    let mut t = ast_type_to_type(ty_node);
                    if param.is_variadic && !matches!(t, Type::Array(_)) {
                        t = Type::Array(Box::new(t));
                    }
                    t
                }
                None => Type::Int64,
            };
            let state = if param.is_ref {
                if param.is_mut {
                    VarState::Owned
                } else {
                    VarState::BorrowedShared {
                        count: 1,
                        borrows: vec![param.span],
                    }
                }
            } else {
                VarState::Owned
            };
            self.env
                .declare_var(param.name.name.clone(), ty, state, param.span, param.is_ref);
        }

        self.check_block(&decl.body);

        if let Err(diags) = self.env.pop_scope() {
            self.diagnostics.extend(diags);
        }
    }

    pub fn check_block(&mut self, block: &Block) -> bool {
        self.env.push_scope();
        let mut diverged = false;
        for stmt in &block.stmts {
            if diverged {
                break;
            }
            diverged = self.check_stmt(stmt);
        }
        if let Err(diags) = self.env.pop_scope() {
            self.diagnostics.extend(diags);
        }
        diverged
    }

    pub fn check_stmt(&mut self, stmt: &Stmt) -> bool {
        match &stmt.kind {
            StmtKind::Set { target, ty, init } => {
                self.check_stmt_set(target, ty.as_ref(), init);
                false
            }
            StmtKind::Static(decl) => {
                let declared_ty = match &decl.ty {
                    Some(t) => Some(ast_type_to_type(t)),
                    None => self.infer_expr_type(&decl.init),
                };
                self.check_expr(&decl.init, MoveContext::None);
                let final_ty = declared_ty.unwrap_or(Type::Error);
                self.env.declare_var(decl.name.name.clone(), final_ty, VarState::Owned, decl.name.span, false);
                false
            }
            StmtKind::Rewrite {
                target,
                op: _,
                value,
            } => {
                self.check_stmt_rewrite(target, value);
                false
            }
            StmtKind::Expr(e) => {
                self.check_expr(e, MoveContext::None)
            }
            StmtKind::Return(Some(e)) => {
                self.check_expr(e, MoveContext::Return);
                true
            }
            StmtKind::Return(None) => {
                true
            }
            StmtKind::Recur(exprs) => {
                for e in exprs {
                    self.check_expr(e, MoveContext::Return);
                }
                true
            }
            StmtKind::Defer(b) => {
                self.check_block(b);
                false
            }
            _ => false,
        }
    }

    fn check_stmt_set(&mut self, target: &BindingTarget, ty: Option<&TypeNode>, init: &Expr) {
        let declared_ty = match ty {
            Some(t) => Some(ast_type_to_type(t)),
            None => self.infer_expr_type(init),
        };

        match target {
            BindingTarget::Ident(id) => {
                if let ExprKind::Ident(src_id) = &init.kind {
                    self.try_move_var(&src_id.name, init.span, false);
                } else if let ExprKind::Ref { mutable, expr } = &init.kind {
                    if let ExprKind::Ident(target_id) = &expr.kind {
                        self.try_borrow_var(&target_id.name, *mutable, id.name.clone(), init.span);
                    } else { self.check_expr(expr, MoveContext::None); }
                } else { self.check_expr(init, MoveContext::None); }

                let final_ty = if let Some(t) = declared_ty { t
                } else {
                    let diag = Diagnostic::error(
                        ErrorCode::E0402,
                        format!("cannot infer type for variable `{}`", id.name),
                        id.span,
                    )
                    .with_note("consider giving this variable an explicit type annotation, e.g. `set name: <type> = ...`");
                    self.diagnostics.push(diag);
                    Type::Error
                };

                self.env
                    .declare_var(id.name.clone(), final_ty, VarState::Owned, id.span, false);
            }
            BindingTarget::Tuple(targets) => {
                self.check_expr(init, MoveContext::None);
                if declared_ty.is_none() {
                    let diag = Diagnostic::error(
                        ErrorCode::E0402,
                        "cannot infer type for tuple destructuring",
                        init.span,
                    )
                    .with_note("consider giving the target an explicit type annotation");
                    self.diagnostics.push(diag);
                }
                for sub in targets {
                    if let BindingTarget::Ident(id) = sub {
                        self.env.declare_var(
                            id.name.clone(),
                            Type::Int64,
                            VarState::Owned,
                            id.span,
                            false,
                        );
                    }
                }
            }
            _ => {
                self.check_expr(init, MoveContext::None);
            }
        }
    }

    fn check_stmt_rewrite(&mut self, target: &LValue, value: &Expr) {
        if let LValue::Ident(id) = target {
            self.env.mark_borrow_mutated(&id.name);
            let is_borrowed = if let Some(var) = self.env.get_var(&id.name) {
                matches!(
                    var.state,
                    VarState::BorrowedShared { .. } | VarState::BorrowedMut { .. }
                )
            } else { false };

            if is_borrowed {
                let diag = Diagnostic::error(
                    ErrorCode::E0503,
                    format!("cannot mutate `{}` while borrowed", id.name),
                    id.span,
                );
                self.diagnostics.push(diag);
                return;
            }

            // reinit restores moved vars back to owned
            if let Some(var) = self.env.get_var_mut(&id.name) {
                var.state = VarState::Owned;
            }

            if let ExprKind::Ident(src_id) = &value.kind {
                self.try_move_var(&src_id.name, value.span, false);
            } else if let ExprKind::Ref { mutable, expr } = &value.kind {
                if let ExprKind::Ident(target_id) = &expr.kind {
                    self.try_borrow_var(&target_id.name, *mutable, id.name.clone(), value.span);
                } else { self.check_expr(expr, MoveContext::None); }
            } else { self.check_expr(value, MoveContext::None); }
        } else if let LValue::Field(recv, _) | LValue::Index(recv, _) = target {
            if let ExprKind::Ident(id) = &recv.kind {
                self.env.mark_borrow_mutated(&id.name);
            }
            self.check_expr(value, MoveContext::None);
        }
    }

    pub fn check_expr(&mut self, expr: &Expr, move_ctx: MoveContext) -> bool {
        match &expr.kind {
            ExprKind::Ident(id) => {
                self.check_ident_expr(id, expr.span, move_ctx);
                false
            }
            ExprKind::Call(callee, args) | ExprKind::ClassInit(callee, args) => {
                self.check_call_expr(callee, args);
                matches!(self.infer_expr_type(expr), Some(Type::Never))
            }
            ExprKind::MethodCall(recv, _, args)
            | ExprKind::QualifiedMethodCall(recv, _, _, args)
            | ExprKind::SafeCall(recv, _, args) => {
                self.check_method_call_expr(recv, args);
                matches!(self.infer_expr_type(expr), Some(Type::Never))
            }
            ExprKind::Ref {
                mutable,
                expr: inner,
            } => {
                if let ExprKind::Ident(id) = &inner.kind {
                    self.try_borrow_var(&id.name, *mutable, String::new(), expr.span);
                } else { self.check_expr(inner, MoveContext::None); }
                false
            }
            ExprKind::Binary(_, lhs, rhs) => {
                let l = self.check_expr(lhs, MoveContext::None);
                let r = self.check_expr(rhs, MoveContext::None);
                l || r
            }
            ExprKind::Unary(_, inner) => {
                self.check_expr(inner, MoveContext::None)
            }
            ExprKind::Ternary(cond, then_e, else_e) => {
                self.check_expr(cond, MoveContext::None);
                let t = self.check_expr(then_e, MoveContext::None);
                let e = self.check_expr(else_e, MoveContext::None);
                t && e
            }
            ExprKind::ArrayInit(elems) | ExprKind::TupleInit(elems) => {
                let mut div = false;
                for elem in elems {
                    if self.check_expr(elem, MoveContext::Move) {
                        div = true;
                    }
                }
                div
            }
            ExprKind::ArrayAlloc(..) => false,
            ExprKind::FieldAccess(target, _) => {
                self.check_expr(target, MoveContext::None);
                false
            }
            ExprKind::IndexAccess(target, index) => {
                self.check_expr(target, MoveContext::None);
                self.check_expr(index, MoveContext::None);
                false
            }
            ExprKind::SliceAccess(target, start, end, _) => {
                self.check_expr(target, MoveContext::None);
                if let Some(s) = start {
                    self.check_expr(s, MoveContext::None);
                }
                if let Some(e) = end {
                    self.check_expr(e, MoveContext::None);
                }
                false
            }
            ExprKind::Block(b) | ExprKind::RegionBlock(b) => {
                self.check_block(b)
            }
            ExprKind::If {
                cond,
                then_branch,
                elif_branches,
                else_branch,
            } => {
                self.check_if_expr(cond, then_branch, elif_branches, else_branch.as_ref())
            }
            ExprKind::Switch { target, arms } => {
                self.check_switch_expr(target, arms)
            }
            ExprKind::While { cond, body, .. } => {
                self.check_while_expr(cond, body);
                false
            }
            ExprKind::For { iter, body, .. } => {
                self.check_for_expr(iter, body);
                false
            }
            ExprKind::Loop { body, .. } => {
                self.check_loop_expr(body);
                false
            }
            ExprKind::PtrCast(inner, _) => {
                self.check_expr(inner, move_ctx)
            }
            ExprKind::NamedArg(_, inner) => {
                self.check_expr(inner, move_ctx)
            }
            ExprKind::InterpolatedString(parts) => {
                let mut div = false;
                for part in parts {
                    if let StringPart::Expr(e) = part {
                        if self.check_expr(e, MoveContext::None) {
                            div = true;
                        }
                    }
                }
                div
            }
            _ => false,
        }
    }

    fn check_ident_expr(&mut self, id: &Ident, span: Span, move_ctx: MoveContext) {
        if let Some(var) = self.env.get_var(&id.name) {
            if let VarState::Moved { moved_at } = &var.state {
                let diag = Diagnostic::error(
                    ErrorCode::E0501,
                    format!("use of moved value `{}`", id.name),
                    span,
                )
                .with_note(format!("value was moved at line {}", moved_at.start.0));
                self.diagnostics.push(diag);
                return;
            }
        }

        if move_ctx.is_move() {
            self.try_move_var(&id.name, span, move_ctx.is_return());
        }
    }

    fn check_call_expr(&mut self, callee: &Expr, args: &[Expr]) {
        self.check_expr(callee, MoveContext::None);
        let mut temp_borrows = Vec::new();
        for arg in args {
            if let ExprKind::Ref { mutable, expr } = &arg.kind {
                if let ExprKind::Ident(id) = &expr.kind {
                    self.try_borrow_var(&id.name, *mutable, String::new(), arg.span);
                    if *mutable { self.env.mark_borrow_mutated(&id.name); }
                    temp_borrows.push(id.name.clone());
                } else { self.check_expr(expr, MoveContext::None); }
            } else if let ExprKind::Ident(id) = &arg.kind {
                self.try_move_var(&id.name, arg.span, false);
            } else { self.check_expr(arg, MoveContext::None); }
        }
        for name in temp_borrows {
            self.env.release_temp_borrow(&name);
        }
    }

    fn check_method_call_expr(&mut self, recv: &Expr, args: &[Expr]) {
        if let ExprKind::Ident(id) = &recv.kind {
            self.env.mark_borrow_mutated(&id.name);
            if let Some(var) = self.env.get_var(&id.name) {
                if let VarState::Moved { moved_at } = &var.state {
                    let diag = Diagnostic::error(
                        ErrorCode::E0501,
                        format!("use of moved value `{}`", id.name),
                        recv.span,
                    )
                    .with_note(format!("value was moved at line {}", moved_at.start.0));
                    self.diagnostics.push(diag);
                }
            }
        } else {
            self.check_expr(recv, MoveContext::None);
        }

        let mut temp_borrows = Vec::new();
        for arg in args {
            if let ExprKind::Ref { mutable, expr } = &arg.kind {
                if let ExprKind::Ident(id) = &expr.kind {
                    self.try_borrow_var(&id.name, *mutable, String::new(), arg.span);
                    if *mutable { self.env.mark_borrow_mutated(&id.name); }
                    temp_borrows.push(id.name.clone());
                } else { self.check_expr(expr, MoveContext::None); }
            } else if let ExprKind::Ident(id) = &arg.kind {
                self.try_move_var(&id.name, arg.span, false);
            } else { self.check_expr(arg, MoveContext::None); }
        }
        for name in temp_borrows {
            self.env.release_temp_borrow(&name);
        }
    }


    fn check_if_expr(
        &mut self,
        cond: &Expr,
        then_branch: &Block,
        elif_branches: &[(Expr, Block)],
        else_branch: Option<&Block>,
    ) -> bool {
        self.check_expr(cond, MoveContext::None);

        let base_env = self.env.clone();
        let mut surviving_envs = Vec::new();

        let then_diverged = self.check_block(then_branch);
        if !then_diverged {
            surviving_envs.push(self.env.clone());
        }

        for (c, b) in elif_branches {
            self.env = base_env.clone();
            self.check_expr(c, MoveContext::None);
            let elif_diverged = self.check_block(b);
            if !elif_diverged {
                surviving_envs.push(self.env.clone());
            }
        }

        if let Some(else_b) = else_branch {
            self.env = base_env.clone();
            let else_diverged = self.check_block(else_b);
            if !else_diverged {
                surviving_envs.push(self.env.clone());
            }
        } else {
            surviving_envs.push(base_env.clone());
        }

        if surviving_envs.is_empty() {
            true
        } else if surviving_envs.len() == 1 {
            self.env = surviving_envs.pop().unwrap();
            false
        } else {
            self.env = StateEnv::join_branch_envs(&base_env, &surviving_envs);
            false
        }
    }

    fn check_switch_expr(&mut self, target: &Expr, arms: &[SwitchArm]) -> bool {
        self.check_expr(target, MoveContext::None);
        let base_env = self.env.clone();
        let mut surviving_envs = Vec::new();

        for arm in arms {
            self.env = base_env.clone();
            let arm_diverged = self.check_block(&arm.body);
            if !arm_diverged {
                surviving_envs.push(self.env.clone());
            }
        }

        if surviving_envs.is_empty() && !arms.is_empty() {
            true
        } else if surviving_envs.len() == 1 {
            self.env = surviving_envs.pop().unwrap();
            false
        } else {
            self.env = StateEnv::join_branch_envs(&base_env, &surviving_envs);
            false
        }
    }

    fn check_while_expr(&mut self, cond: &Expr, body: &Block) {
        let old_threshold = self.loop_scope_threshold;
        if self.loop_depth == 0 {
            self.loop_scope_threshold = self.env.current_depth + 1;
        }
        self.loop_depth += 1;

        self.check_expr(cond, MoveContext::None);
        self.check_block(body);

        self.loop_depth -= 1;
        if self.loop_depth == 0 {
            self.loop_scope_threshold = old_threshold;
        }
    }

    fn check_for_expr(&mut self, iter: &Expr, body: &Block) {
        let old_threshold = self.loop_scope_threshold;
        if self.loop_depth == 0 {
            self.loop_scope_threshold = self.env.current_depth + 1;
        }
        self.loop_depth += 1;

        self.check_expr(iter, MoveContext::None);
        self.check_block(body);

        self.loop_depth -= 1;
        if self.loop_depth == 0 {
            self.loop_scope_threshold = old_threshold;
        }
    }

    fn check_loop_expr(&mut self, body: &Block) {
        let old_threshold = self.loop_scope_threshold;
        if self.loop_depth == 0 {
            self.loop_scope_threshold = self.env.current_depth + 1;
        }
        self.loop_depth += 1;

        self.check_block(body);

        self.loop_depth -= 1;
        if self.loop_depth == 0 {
            self.loop_scope_threshold = old_threshold;
        }
    }

    pub fn try_move_var(&mut self, name: &str, span: Span, is_return: bool) {
        if let Some(var) = self.env.get_var(name) {
            if var.is_ref {
                let diag = Diagnostic::error(
                    ErrorCode::E0502,
                    format!("cannot move `{name}` because it is borrowed"),
                    span,
                );
                self.diagnostics.push(diag);
                return;
            }

            if TypeAffinity::is_copy(&var.ty) { return; }

            match &var.state {
                VarState::Moved { moved_at } => {
                    let diag = Diagnostic::error(
                        ErrorCode::E0501,
                        format!("use of moved value `{name}`"),
                        span,
                    )
                    .with_note(format!("value was moved at line {}", moved_at.start.0));
                    self.diagnostics.push(diag);
                    return;
                }
                VarState::BorrowedShared { .. } | VarState::BorrowedMut { .. } => {
                    let diag = Diagnostic::error(
                        ErrorCode::E0502,
                        format!("cannot move `{name}` because it is borrowed"),
                        span,
                    );
                    self.diagnostics.push(diag);
                    return;
                }
                VarState::Owned => {}
                VarState::Uninitialized => return,
            }

            if !is_return && self.loop_depth > 0 && var.scope_depth < self.loop_scope_threshold {
                let diag = Diagnostic::error(
                    ErrorCode::E0505,
                    format!("cannot move `{name}` out of outer scope inside a loop"),
                    span,
                );
                self.diagnostics.push(diag);
                return;
            }
        }

        if let Some(var) = self.env.get_var_mut(name) {
            var.state = VarState::Moved { moved_at: span };
        }
    }

    pub fn try_borrow_var(
        &mut self,
        target_name: &str,
        mutable: bool,
        borrower: String,
        span: Span,
    ) {
        let mut target_scope = 0;

        if let Some(var) = self.env.get_var(target_name) {
            target_scope = var.scope_depth;
            match &var.state {
                VarState::Moved { moved_at } => {
                    let diag = Diagnostic::error(
                        ErrorCode::E0501,
                        format!("cannot borrow moved value `{target_name}`"),
                        span,
                    )
                    .with_note(format!("value was moved at line {}", moved_at.start.0));
                    self.diagnostics.push(diag);
                    return;
                }
                VarState::BorrowedMut { .. } => {
                    let diag = Diagnostic::error(
                        ErrorCode::E0502,
                        format!(
                            "cannot borrow `{target_name}` as {} because it is already borrowed as mutable",
                            if mutable { "mutable" } else { "immutable" }
                        ),
                        span,
                    );
                    self.diagnostics.push(diag);
                    return;
                }
                VarState::BorrowedShared { .. } => {
                    if mutable {
                        let diag = Diagnostic::error(
                            ErrorCode::E0502,
                            format!(
                                "cannot borrow `{target_name}` as mutable because it is already borrowed as immutable"
                            ),
                            span,
                        );
                        self.diagnostics.push(diag);
                        return;
                    }
                }
                VarState::Owned => {}
                VarState::Uninitialized => return,
            }
        }

        let borrower_scope = if borrower.is_empty() { self.env.current_depth } 
        else if let Some(b) = self.env.get_var(&borrower) { b.scope_depth } 
        else { self.env.current_depth };

        self.env.record_borrow(BorrowRecord {
            borrower,
            borrower_scope,
            target: target_name.to_string(),
            target_scope,
            mutable,
            span,
            used_mutably: false,
        });
    }

    fn infer_expr_type(&self, expr: &Expr) -> Option<Type> {
        match &expr.kind {
            ExprKind::Literal(lit) => self.infer_literal_type(lit),
            ExprKind::InterpolatedString(_) => Some(Type::String),
            ExprKind::ArrayInit(elems) => {
                let elem_ty = elems.first().and_then(|e| self.infer_expr_type(e)).unwrap_or(Type::Int64);
                Some(Type::Array(Box::new(elem_ty)))
            }
            ExprKind::ArrayAlloc(elem_ty, _) => {
                Some(Type::Array(Box::new(ast_type_to_type(elem_ty))))
            }
            ExprKind::TupleInit(elems) => {
                let tys: Option<Vec<Type>> = elems.iter().map(|e| self.infer_expr_type(e)).collect();
                tys.map(Type::Tuple)
            }
            ExprKind::Ref { mutable, expr } => self.infer_expr_type(expr).map(|inner| Type::Ref {
                inner: Box::new(inner),
                mutable: *mutable,
            }),
            ExprKind::PtrOf(id) => self.env.get_var(&id.name).map(|v| Type::Ptr(Box::new(v.ty.clone()))),
            ExprKind::PtrCast(_, ty_node) => Some(ast_type_to_type(ty_node)),
            ExprKind::Binary(op, left, right) => self.infer_binary_type(op, left, right),
            ExprKind::Unary(op, operand) => match op {
                UnaryOp::Not => Some(Type::Bool),
                _ => self.infer_expr_type(operand),
            },
            ExprKind::Ident(id) => self
                .env
                .get_var(&id.name)
                .map(|v| v.ty.clone())
                .or_else(|| self.fn_types.get(&id.name).cloned()),
            ExprKind::This | ExprKind::Super => self.env.get_var("this").map(|v| v.ty.clone()),
            ExprKind::Call(callee, args) => self.infer_call_type(callee, args),
            ExprKind::MethodCall(target, method, args) => self.infer_method_call_type(target, method, args),
            ExprKind::FieldAccess(target, field) => {
                let target_ty = self.infer_expr_type(target)?;
                let base_name = match &target_ty {
                    Type::Class(name) | Type::Struct(name) | Type::Cstruct(name) => Some(name.clone()),
                    Type::Ref { inner, .. } => match &**inner {
                        Type::Class(name) | Type::Struct(name) | Type::Cstruct(name) => Some(name.clone()),
                        _ => None,
                    },
                    Type::Union(members) => {
                        let mut found = None;
                        for m in members {
                            if let Type::Class(name) | Type::Struct(name) | Type::Cstruct(name) = m {
                                found = Some(name.clone());
                                break;
                            }
                        }
                        found
                    }
                    _ => None,
                };
                base_name.as_deref().and_then(|name| {
                    self.type_fields.get(name).and_then(|fields| fields.get(&field.name).cloned())
                })
            }
            ExprKind::IndexAccess(target, _) => match self.infer_expr_type(target)? {
                Type::Array(inner) => Some(*inner),
                Type::String | Type::Bytes => Some(Type::Uint8),
                _ => None,
            },
            ExprKind::SliceAccess(target, _, _, _) => self.infer_expr_type(target),
            ExprKind::StructInit(name, _) => Some(Type::Struct(name.name.clone())),
            ExprKind::ClassInit(callee, _) => {
                if let ExprKind::Ident(id) = &callee.kind {
                    Some(Type::Class(id.name.clone()))
                } else if let ExprKind::Path(parts) = &callee.kind {
                    parts.last().map(|id| Type::Class(id.name.clone()))
                } else {
                    None
                }
            }
            ExprKind::Ternary(_, then_expr, else_expr) => {
                self.infer_expr_type(then_expr).or_else(|| self.infer_expr_type(else_expr))
            }
            ExprKind::Block(b) | ExprKind::RegionBlock(b) => {
                let mut temp_checker = self.clone();
                temp_checker.env.push_scope();
                for stmt in &b.stmts[..b.stmts.len().saturating_sub(1)] {
                    temp_checker.check_stmt(stmt);
                }
                b.stmts.last().and_then(|s| match &s.kind {
                    StmtKind::Expr(e) => temp_checker.infer_expr_type(e),
                    _ => None,
                })
            }
            ExprKind::AnonymousFun(fun) => {
                let params = fun
                    .params
                    .iter()
                    .map(|p| {
                        let ty = p
                            .ty
                            .as_ref()
                            .map(ast_type_to_type)
                            .unwrap_or(Type::Int64);
                        yukic_types::TypeParam::new(ty, p.is_ref, p.is_mut)
                    })
                    .collect();
                let ret = fun
                    .return_type
                    .as_ref()
                    .map(ast_type_to_type)
                    .or_else(|| {
                        let mut temp = self.clone();
                        temp.env.push_scope();
                        for p in &fun.params {
                            let ty = p.ty.as_ref().map(ast_type_to_type).unwrap_or(Type::Int64);
                            temp.env.declare_var(
                                p.name.name.clone(),
                                ty,
                                crate::state::VarState::Owned,
                                p.span,
                                p.is_ref,
                            );
                        }
                        fun.body.stmts.iter().find_map(|s| match &s.kind {
                            StmtKind::Return(Some(e)) => temp.infer_expr_type(e),
                            _ => None,
                        })
                    })
                    .unwrap_or(Type::Void);
                Some(Type::Fn {
                    params,
                    ret: Box::new(ret),
                    is_cfun: false,
                })
            }
            ExprKind::Lambda(params, body) => {
                let ps = params
                    .iter()
                    .map(|_| yukic_types::TypeParam::value(Type::Int64))
                    .collect();
                let ret = self.infer_expr_type(body).unwrap_or(Type::Int64);
                Some(Type::Fn {
                    params: ps,
                    ret: Box::new(ret),
                    is_cfun: false,
                })
            }
            _ => None,
        }
    }

    fn infer_literal_type(&self, lit: &LiteralValue) -> Option<Type> {
        match lit {
            LiteralValue::Int(_) => Some(Type::Int64),
            LiteralValue::UInt(_) => Some(Type::Uint64),
            LiteralValue::Float(_) => Some(Type::Float64),
            LiteralValue::Bool(_) => Some(Type::Bool),
            LiteralValue::Char(_) => Some(Type::Char),
            LiteralValue::String(_) => Some(Type::String),
            LiteralValue::Bytes(_) => Some(Type::Bytes),
            LiteralValue::Null => Some(Type::Null),
        }
    }

    fn infer_binary_type(&self, op: &BinaryOp, left: &Expr, right: &Expr) -> Option<Type> {
        match op {
            BinaryOp::Eq
            | BinaryOp::NotEq
            | BinaryOp::Lt
            | BinaryOp::LtEq
            | BinaryOp::Gt
            | BinaryOp::GtEq
            | BinaryOp::And
            | BinaryOp::Or => Some(Type::Bool),
            _ => {
                let lt = self.infer_expr_type(left);
                let rt = self.infer_expr_type(right);
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
        }
    }

    fn infer_call_type(&self, callee: &Expr, args: &[Expr]) -> Option<Type> {
        if matches!(&callee.kind, ExprKind::Super) {
            return Some(Type::Void);
        }
        let callee_id = match &callee.kind {
            ExprKind::Ident(id) => Some(id.name.as_str()),
            ExprKind::Path(segments) if segments.len() == 2 && segments[0].name == "rt" => {
                Some(segments[1].name.as_str())
            }
            ExprKind::Path(segments) if segments.len() == 2 && segments[0].name == "extern" => {
                Some(segments[1].name.as_str())
            }
            _ => None,
        };
        if let Some(name) = callee_id {
            if name == "cast" && args.len() >= 2 {
                if let ExprKind::Literal(LiteralValue::String(s)) = &args[1].kind {
                    return Some(yukic_types::convert::parse_primitive_or_named(s));
                }
                if let ExprKind::Call(c, c_args) = &args[1].kind {
                    let is_typeof = match &c.kind {
                        ExprKind::Ident(cid) => cid.name == "typeof" || cid.name == "__typeof_intrinsic__",
                        ExprKind::Path(parts) if parts.len() == 2 && parts[0].name == "rt" => {
                            parts[1].name == "typeof" || parts[1].name == "__typeof_intrinsic__"
                        }
                        _ => false,
                    };
                    if is_typeof && !c_args.is_empty() {
                        return self.infer_expr_type(&c_args[0]);
                    }
                }
            }
            if name == "ptr_of" && !args.is_empty() {
                if let ExprKind::Ident(target_id) = &args[0].kind {
                    return self.env.get_var(&target_id.name).map(|v| Type::Ptr(Box::new(v.ty.clone())));
                }
            }
            if name == "from_raw_parts" || name == "string::from_raw_parts" {
                if args.len() == 2 {
                    return Some(Type::String);
                } else if args.len() >= 3 {
                    if let Some(Type::Ptr(inner)) = self.infer_expr_type(&args[0]) {
                        if *inner == Type::Uint8 && name == "from_raw_parts" {
                            return Some(Type::Bytes);
                        }
                        return Some(Type::Array(inner));
                    }
                    return Some(Type::Bytes);
                }
            }
            if let Some(ret) = self.fn_returns.get(name) {
                return Some(ret.clone());
            }
        }
        if let Some(callee_ty) = self.infer_expr_type(callee) {
            match callee_ty {
                Type::Fn { ret, .. } => return Some(*ret),
                Type::Ptr(inner) => match *inner {
                    Type::Fn { ret, .. } => return Some(*ret),
                    Type::Void => return Some(Type::Int64),
                    _ => {}
                },
                _ => {}
            }
        }
        if let ExprKind::Path(segments) = &callee.kind {
            if segments.len() == 2 && segments[0].name == "shared" && segments[1].name == "new" {
                let inner = args.first().and_then(|a| self.infer_expr_type(a)).unwrap_or(Type::Int64);
                return Some(Type::Shared(Box::new(inner)));
            }
            let name = segments.last().map(|s| s.name.as_str()).unwrap_or("");
            return self.fn_returns.get(name).cloned();
        }
        None
    }

    fn infer_method_call_type(&self, target: &Expr, method: &Ident, args: &[Expr]) -> Option<Type> {
        if let Some(target_ty) = self.infer_expr_type(target) {
            let actual_ty = match target_ty {
                Type::Ref { inner, .. } => *inner,
                other => other,
            };
            if actual_ty == Type::String {
                if let Some(ret) = self.infer_string_method_type(&method.name) {
                    return Some(ret);
                }
            } else if let Type::Array(inner) = &actual_ty {
                if let Some(ret) = self.infer_array_method_type(&method.name, inner) {
                    return Some(ret);
                }
            } else if actual_ty == Type::Bytes {
                match method.name.as_str() {
                    "is_empty" => return Some(Type::Bool),
                    "len" | "cap" => return Some(Type::Int64),
                    "as_ptr" => return Some(Type::Ptr(Box::new(Type::Uint8))),
                    "cp" => return Some(Type::Bytes),
                    "push" | "del" | "clear" | "reserve" => return Some(Type::Void),
                    "pop" => return Some(Type::nullable(Type::Uint8)),
                    _ => {}
                }
            }
        }
        match method.name.as_str() {
            "len" | "cap" => Some(Type::Int64),
            "as_ptr" => Some(Type::Ptr(Box::new(Type::Uint8))),
            "new" => {
                if let ExprKind::Ident(id) = &target.kind {
                    if id.name == "shared" {
                        let inner = args.first().and_then(|a| self.infer_expr_type(a)).unwrap_or(Type::Int64);
                        return Some(Type::Shared(Box::new(inner)));
                    }
                    if id.name == "weak" {
                        let inner = args.first().and_then(|a| self.infer_expr_type(a)).unwrap_or(Type::Int64);
                        return Some(Type::Weak(Box::new(inner)));
                    }
                }
                self.fn_returns.get(&method.name).cloned()
            }
            _ => self.fn_returns.get(&method.name).cloned(),
        }
    }

    fn infer_string_method_type(&self, method: &str) -> Option<Type> {
        match method {
            "len" | "cap" | "count" | "to_i64" | "to_int64" => Some(Type::Int64),
            "index_of" | "last_index_of" | "index_of_from" => Some(Type::nullable(Type::Int64)),
            "is_empty" | "eq" | "contains" => Some(Type::Bool),
            "starts_with" | "ends_with" => Some(Type::nullable(Type::Bool)),
            "as_ptr" => Some(Type::Ptr(Box::new(Type::Uint8))),
            "char_at" => Some(Type::Char),
            "byte_at" => Some(Type::Uint8),
            "to_f64" | "to_float64" => Some(Type::Float64),
            "trim" | "trim_start" | "trim_end" | "upper" | "lower" | "slice" | "replace"
            | "repeat" | "reverse" | "pad_start" | "pad_end" | "join" | "cp" => Some(Type::String),
            "split" | "lines" => Some(Type::Array(Box::new(Type::String))),
            "to_bytes" => Some(Type::Bytes),
            _ => None,
        }
    }

    fn infer_array_method_type(&self, method: &str, elem_ty: &Type) -> Option<Type> {
        match method {
            "len" | "cap" => Some(Type::Int64),
            "is_empty" => Some(Type::Bool),
            "as_ptr" => Some(Type::Ptr(Box::new(elem_ty.clone()))),
            "cp" => Some(Type::Array(Box::new(elem_ty.clone()))),
            "push" | "del" | "clear" | "reserve" => Some(Type::Void),
            "pop" => Some(Type::nullable(elem_ty.clone())),
            _ => None,
        }
    }
}

impl Default for BorrowChecker {
    fn default() -> Self { Self::new() }
}
