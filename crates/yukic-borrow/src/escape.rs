#![deny(unsafe_code)]
#![allow(
    clippy::collapsible_if,
    clippy::match_wildcard_for_single_variants,
    clippy::wildcard_imports
)]

use std::collections::{HashMap, HashSet};
use yukic_ast::*;
use yukic_diagnostics::Diagnostic;
use yukic_diagnostics::code::{ErrorCode, WarningCode};
use yukic_span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EscapeKind {
    NoEscape,
    EscapeToCaller,
    LoopEscape,
    GlobalEscape,
}

impl EscapeKind {
    #[must_use]
    pub fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::GlobalEscape, _) | (_, Self::GlobalEscape) => Self::GlobalEscape,
            (Self::EscapeToCaller, _) | (_, Self::EscapeToCaller) => Self::EscapeToCaller,
            (Self::LoopEscape, _) | (_, Self::LoopEscape) => Self::LoopEscape,
            (Self::NoEscape, Self::NoEscape) => Self::NoEscape,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EscapeInfo {
    pub var_escapes: HashMap<String, EscapeKind>,
    pub alloc_escapes: HashMap<Span, EscapeKind>,
}

impl EscapeInfo {
    pub fn new() -> Self { Self::default() }

    pub fn mark_var(&mut self, name: &str, kind: EscapeKind) {
        let entry = self
            .var_escapes
            .entry(name.to_string())
            .or_insert(EscapeKind::NoEscape);
        *entry = entry.join(kind);
    }

    pub fn mark_alloc(&mut self, span: Span, kind: EscapeKind) {
        let entry = self
            .alloc_escapes
            .entry(span)
            .or_insert(EscapeKind::NoEscape);
        *entry = entry.join(kind);
    }
}

pub struct EscapeAnalysis {
    pub info: EscapeInfo,
    pub diagnostics: Vec<Diagnostic>,
    current_depth: usize,
    loop_depth: usize,
    loop_scope_threshold: usize,
    params: HashSet<String>,
    local_vars: HashMap<String, usize>,
    var_allocs: HashMap<String, Span>,
    var_ref_targets: HashMap<String, String>,
}

impl EscapeAnalysis {
    pub fn new() -> Self {
        Self {
            info: EscapeInfo::new(),
            diagnostics: Vec::new(),
            current_depth: 0,
            loop_depth: 0,
            loop_scope_threshold: 0,
            params: HashSet::new(),
            local_vars: HashMap::new(),
            var_allocs: HashMap::new(),
            var_ref_targets: HashMap::new(),
        }
    }

    pub fn analyze_module(module: &Module) -> (EscapeInfo, Vec<Diagnostic>) {
        let mut analyzer = Self::new();
        for item in &module.items {
            match &item.kind {
                ItemKind::Fun(decl) => analyzer.analyze_fun(decl),
                ItemKind::Class(decl) => analyzer.analyze_class(decl),
                _ => {}
            }
        }
        (analyzer.info, analyzer.diagnostics)
    }

    pub fn analyze_fun(&mut self, decl: &FunDecl) {
        self.enter_scope();
        for param in &decl.params {
            self.params.insert(param.name.name.clone());
            self.local_vars
                .insert(param.name.name.clone(), self.current_depth);
        }
        self.analyze_block(&decl.body);
        self.exit_scope();
    }

    pub fn analyze_class(&mut self, decl: &ClassDecl) {
        for method in &decl.methods {
            self.analyze_method(method);
        }
    }

    pub fn analyze_method(&mut self, decl: &MethodDecl) {
        self.enter_scope();
        for param in &decl.params {
            self.params.insert(param.name.name.clone());
            self.local_vars
                .insert(param.name.name.clone(), self.current_depth);
        }
        self.analyze_block(&decl.body);
        self.exit_scope();
    }

    pub fn analyze_block(&mut self, block: &Block) {
        self.enter_scope();
        for stmt in &block.stmts {
            self.analyze_stmt(stmt);
        }
        self.exit_scope();
    }

    pub fn analyze_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Set { target, init, .. } => {
                self.analyze_stmt_set(target, init);
            }
            StmtKind::Static(decl) => {
                self.analyze_stmt_set(&BindingTarget::Ident(decl.name.clone()), &decl.init);
            }
            StmtKind::Rewrite { target, value, .. } => {
                self.analyze_stmt_rewrite(target, value);
            }
            StmtKind::Return(ret_expr) => {
                self.analyze_stmt_return(ret_expr.as_ref(), stmt.span);
            }
            StmtKind::Expr(expr) => { self.analyze_stmt_expr(expr); }
            StmtKind::Defer(b) => { self.analyze_block(b); }
            _ => {}
        }
    }

    fn analyze_stmt_set(&mut self, target: &BindingTarget, init: &Expr) {
        self.analyze_expr(init);
        if is_allocation_expr(init) {
            self.info.mark_alloc(init.span, EscapeKind::NoEscape);
        }

        if let BindingTarget::Ident(id) = target {
            self.local_vars.insert(id.name.clone(), self.current_depth);
            self.info.mark_var(&id.name, EscapeKind::NoEscape);

            if is_allocation_expr(init) {
                self.var_allocs.insert(id.name.clone(), init.span);
            } else if let ExprKind::Ident(src_id) = &init.kind {
                if let Some(&alloc_span) = self.var_allocs.get(&src_id.name) {
                    self.var_allocs.insert(id.name.clone(), alloc_span);
                }
            }

            if let ExprKind::Ref { expr, .. } = &init.kind {
                if let ExprKind::Ident(target_id) = &expr.kind {
                    self.var_ref_targets
                        .insert(id.name.clone(), target_id.name.clone());
                }
            }
        }
    }

    fn analyze_stmt_rewrite(&mut self, target: &LValue, value: &Expr) {
        self.analyze_expr(value);

        if self.loop_depth > 0 {
            match target {
                LValue::Ident(id) if self.is_outer_var(&id.name) => {
                    self.mark_loop_escape(value);
                }
                LValue::Field(base, _) | LValue::Index(base, _) => {
                    if let ExprKind::Ident(id) = &base.kind {
                        if self.is_outer_var(&id.name) {
                            self.mark_loop_escape(value);
                        }
                    }
                }
                _ => {}
            }
        }

        if let LValue::Field(base, _) = target {
            if is_shared_expr(base) {
                self.mark_global_escape(value);
            }
        }
    }

    fn analyze_stmt_return(&mut self, ret_expr: Option<&Expr>, span: Span) {
        let Some(expr) = ret_expr else {
            return;
        };

        if let ExprKind::Ref { expr: inner, .. } = &expr.kind {
            if let ExprKind::Ident(id) = &inner.kind {
                if self.is_local_non_param(&id.name) {
                    let diag = Diagnostic::error(
                        ErrorCode::E0508,
                        format!("cannot return reference to local variable `{}`", id.name),
                        expr.span,
                    )
                    .with_note("local variables are destroyed when function exits");
                    self.diagnostics.push(diag);
                    return;
                }
            } else {
                let diag = Diagnostic::error(
                    ErrorCode::E0508,
                    "cannot return reference to temporary value",
                    expr.span,
                );
                self.diagnostics.push(diag);
                return;
            }
        }

        if let ExprKind::Ident(id) = &expr.kind {
            if let Some(target) = self.var_ref_targets.get(&id.name) {
                if self.is_local_non_param(target) {
                    let diag = Diagnostic::error(
                        ErrorCode::E0508,
                        format!("cannot return reference to local variable `{target}`"),
                        span,
                    )
                    .with_note("local variables are destroyed when function exits");
                    self.diagnostics.push(diag);
                    return;
                }
            }
            self.info.mark_var(&id.name, EscapeKind::EscapeToCaller);
            if let Some(&alloc_span) = self.var_allocs.get(&id.name) {
                self.info.mark_alloc(alloc_span, EscapeKind::EscapeToCaller);
            }
        } else if is_allocation_expr(expr) {
            self.info.mark_alloc(expr.span, EscapeKind::EscapeToCaller);
        }

        self.analyze_expr(expr);
    }

    fn analyze_stmt_expr(&mut self, expr: &Expr) {
        if is_affine_unpinned_discard(expr) {
            let desc = describe_expr(expr);
            let diag = Diagnostic::warning(
                WarningCode::W0501,
                format!("discard of unpinned affine resource `{desc}`"),
                expr.span,
            )
            .with_note(
                "value is immediately dropped; consider binding with `set` or calling `.drop()`",
            );
            self.diagnostics.push(diag);
        }
        self.analyze_expr(expr);
    }

    pub fn analyze_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Call(callee, args) | ExprKind::ClassInit(callee, args) => {
                self.analyze_call(callee, args);
            }
            ExprKind::MethodCall(recv, method, args)
            | ExprKind::QualifiedMethodCall(recv, _, method, args)
            | ExprKind::SafeCall(recv, method, args) => {
                self.analyze_method_call(recv, method, args);
            }
            ExprKind::Binary(_, lhs, rhs) => {
                self.analyze_expr(lhs);
                self.analyze_expr(rhs);
            }
            ExprKind::Unary(_, inner) => {
                self.analyze_expr(inner);
            }
            ExprKind::Block(b) | ExprKind::RegionBlock(b) => {
                self.analyze_block(b);
            }
            ExprKind::While { cond, body, .. } => {
                self.analyze_while(cond, body);
            }
            ExprKind::For { iter, body, .. } => {
                self.analyze_for(iter, body);
            }
            ExprKind::Loop { body, .. } => {
                self.analyze_loop(body);
            }
            ExprKind::If {
                cond,
                then_branch,
                elif_branches,
                else_branch,
            } => {
                self.analyze_expr(cond);
                self.analyze_block(then_branch);
                for (c, b) in elif_branches {
                    self.analyze_expr(c);
                    self.analyze_block(b);
                }
                if let Some(b) = else_branch {
                    self.analyze_block(b);
                }
            }
            ExprKind::NamedArg(_, inner) => {
                self.analyze_expr(inner);
            }
            _ => {}
        }
    }

    fn analyze_call(&mut self, callee: &Expr, args: &[Expr]) {
        self.analyze_expr(callee);
        let is_shared_new = is_shared_call(callee);

        for arg in args {
            self.analyze_expr(arg);
            if is_shared_new { self.mark_global_escape(arg); }
        }
    }

    fn analyze_method_call(&mut self, recv: &Expr, method: &Ident, args: &[Expr]) {
        self.analyze_expr(recv);

        if let ExprKind::Ident(id) = &recv.kind {
            if id.name == "shared" && method.name == "new" { for arg in args { self.mark_global_escape(arg); } }
        }

        if self.loop_depth > 0 && is_collection_mutating_method(&method.name) {
            if let ExprKind::Ident(id) = &recv.kind {
                if self.is_outer_var(&id.name) {
                    for arg in args { self.mark_loop_escape(arg); }
                }
            }
        }

        for arg in args { self.analyze_expr(arg); }
    }

    fn analyze_while(&mut self, cond: &Expr, body: &Block) {
        let old_threshold = self.loop_scope_threshold;
        if self.loop_depth == 0 { self.loop_scope_threshold = self.current_depth + 1; }
        self.loop_depth += 1;

        self.analyze_expr(cond);
        self.analyze_block(body);

        self.loop_depth -= 1;
        if self.loop_depth == 0 { self.loop_scope_threshold = old_threshold; }
    }

    fn analyze_for(&mut self, iter: &Expr, body: &Block) {
        let old_threshold = self.loop_scope_threshold;
        if self.loop_depth == 0 {
            self.loop_scope_threshold = self.current_depth + 1;
        }
        self.loop_depth += 1;

        self.analyze_expr(iter);
        self.analyze_block(body);

        self.loop_depth -= 1;
        if self.loop_depth == 0 {
            self.loop_scope_threshold = old_threshold;
        }
    }

    fn analyze_loop(&mut self, body: &Block) {
        let old_threshold = self.loop_scope_threshold;
        if self.loop_depth == 0 {
            self.loop_scope_threshold = self.current_depth + 1;
        }
        self.loop_depth += 1;

        self.analyze_block(body);

        self.loop_depth -= 1;
        if self.loop_depth == 0 {
            self.loop_scope_threshold = old_threshold;
        }
    }

    fn mark_loop_escape(&mut self, expr: &Expr) {
        if let ExprKind::Ident(id) = &expr.kind {
            self.info.mark_var(&id.name, EscapeKind::LoopEscape);
            if let Some(&alloc_span) = self.var_allocs.get(&id.name) {
                self.info.mark_alloc(alloc_span, EscapeKind::LoopEscape);
            }
        } else {
            self.info.mark_alloc(expr.span, EscapeKind::LoopEscape);
        }
    }

    fn mark_global_escape(&mut self, expr: &Expr) {
        if let ExprKind::Ident(id) = &expr.kind {
            self.info.mark_var(&id.name, EscapeKind::GlobalEscape);
            if let Some(&alloc_span) = self.var_allocs.get(&id.name) {
                self.info.mark_alloc(alloc_span, EscapeKind::GlobalEscape);
            }
        } else {
            self.info.mark_alloc(expr.span, EscapeKind::GlobalEscape);
        }
    }

    fn is_outer_var(&self, name: &str) -> bool {
        if let Some(&depth) = self.local_vars.get(name) {
            self.loop_depth > 0 && depth < self.loop_scope_threshold
        } 

        else { false }
    }

    fn is_local_non_param(&self, name: &str) -> bool {
        self.local_vars.contains_key(name) && !self.params.contains(name)
    }

    fn enter_scope(&mut self) { self.current_depth += 1; }

    fn exit_scope(&mut self) {
        if self.current_depth > 0 { self.current_depth -= 1; }
    }
}

impl Default for EscapeAnalysis {
    fn default() -> Self { Self::new() }
}

fn is_allocation_expr(expr: &Expr) -> bool {
    matches!(
        &expr.kind,
        ExprKind::ArrayInit(_)
            | ExprKind::ArrayAlloc(..)
            | ExprKind::MapInit(_)
            | ExprKind::TupleInit(_)
            | ExprKind::StructInit(..)
            | ExprKind::ClassInit(..)
            | ExprKind::InterpolatedString(_)
    )
}

fn is_collection_mutating_method(name: &str) -> bool {
    matches!(
        name,
        "push"
            | "pop"
            | "del"
            | "clear"
            | "reserve"
            | "add"
            | "insert"
            | "append"
            | "set"
            | "put"
            | "extend"
            | "enqueue"
            | "push_back"
            | "push_front"
    )
}

fn is_shared_call(callee: &Expr) -> bool {
    match &callee.kind {
        ExprKind::Path(parts) => {
            if parts.len() >= 2 {
                parts[0].name == "shared" && parts[1].name == "new"
            } else { false }
        }
        ExprKind::Ident(id) => id.name == "shared" || id.name == "shared_new",
        _ => false,
    }
}

fn is_shared_expr(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Ident(id) => id.name.starts_with("shared_") || id.name == "shared",
        _ => false,
    }
}

fn is_affine_unpinned_discard(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::ArrayInit(_) | ExprKind::ArrayAlloc(..) | ExprKind::MapInit(_) | ExprKind::StructInit(..) => true,
        ExprKind::Call(callee, _) => match &callee.kind {
            ExprKind::Ident(id) => {
                id.name.starts_with("open_")
                    || id.name.starts_with("make_")
                    || id.name.ends_with("_handle")
                    || id.name.contains("buffer")
            }

            // if
            ExprKind::Path(parts) => {
                if let Some(last) = parts.last() {
                    last.name == "open" || last.name == "create" || last.name == "acquire"
                } 


                // if
                else { false }
            }

            // match
            _ => false,
        },
        ExprKind::MethodCall(_, method, _) => {
            method.name != "drop"
                && (method.name.starts_with("open") || method.name.starts_with("acquire"))
        }
        _ => false,
    }
}

fn describe_expr(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::ArrayInit(_) | ExprKind::ArrayAlloc(..) => "array".to_string(),
        ExprKind::MapInit(_) => "map".to_string(),
        ExprKind::StructInit(id, _) => id.name.clone(),
        ExprKind::ClassInit(callee, _) => {
            if let ExprKind::Ident(id) = &callee.kind {
                id.name.clone()
            } else {
                "class".to_string()
            }
        }
        ExprKind::Call(callee, _) => {
            if let ExprKind::Ident(id) = &callee.kind {
                id.name.clone()
            } else {
                "resource".to_string()
            }
        }
        ExprKind::MethodCall(_, method, _) => method.name.clone(),
        _ => "resource".to_string(),
    }
}
