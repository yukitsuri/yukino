#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet};
use yukic_ast::attr::{AttributeArgs, AttributeNode};
use yukic_ast::expr::{Expr, ExprKind, LiteralValue, Pattern, PatternKind, StringPart};
use yukic_ast::ident::Ident;
use yukic_ast::item::{FunDecl, Item, ItemKind, MethodDecl, Visibility};
use yukic_ast::stmt::{BindingTarget, Block, LValue, Stmt, StmtKind};
use yukic_ast::types::{TypeNode, TypeNodeKind};
use yukic_diagnostics::code::WarningCode;
use yukic_diagnostics::emitter::DiagnosticEmitter;
use yukic_diagnostics::model::Diagnostic;
use yukic_span::{SourceMap, Span};

use crate::pipeline::LoadedModule;

pub fn run_lint_pass(
    modules: &[LoadedModule],
    source_map: &SourceMap,
    emitter: &mut DiagnosticEmitter,
) {
    configure_warning_filter(modules, emitter);
    check_unused_imports(modules, source_map, emitter);
    check_dead_functions(modules, source_map, emitter);
    check_unused_variables(modules, source_map, emitter);
    check_deprecated_usages(modules, source_map, emitter);
}

fn configure_warning_filter(modules: &[LoadedModule], emitter: &mut DiagnosticEmitter) {
    for m in modules {
        extract_filter_from_attrs(&m.ast.module_attrs, emitter);
        for item in &m.ast.items {
            extract_filter_from_attrs(&item.attrs, emitter);
            match &item.kind {
                ItemKind::Fun(decl) => extract_filter_from_attrs(&decl.attrs, emitter),
                ItemKind::Cfun(decl) => extract_filter_from_attrs(&decl.yukino_attrs, emitter),
                ItemKind::Class(decl) => {
                    for method in &decl.methods {
                        extract_filter_from_attrs(&method.attrs, emitter);
                    }
                }
                ItemKind::Struct(decl) => {
                    for method in &decl.methods {
                        extract_filter_from_attrs(&method.attrs, emitter);
                    }
                }
                _ => {}
            }
        }
    }
}

fn is_lint_ignore(name: &str) -> bool {
    name == "ignore_warn" || name == "warns_ignore" || name == "lint::ignore" || name == "lint::allow"
}

fn is_lint_deny(name: &str) -> bool {
    name == "deny_warn" || name == "warns_deny" || name == "lint::deny"
}

fn apply_lint_rule(attr: &AttributeNode, is_deny: bool, emitter: &mut DiagnosticEmitter) {
    match &attr.args {
        AttributeArgs::Positional(exprs) => {
            for expr in exprs {
                let rule = match &expr.kind {
                    ExprKind::Literal(LiteralValue::String(s)) => Some(s.as_str()),
                    ExprKind::Ident(id) => Some(id.name.as_str()),
                    _ => None,
                };
                if let Some(name) = rule {
                    if is_deny {
                        emitter.filter_mut().deny_name(name);
                    } else {
                        emitter.filter_mut().ignore_name(name);
                    }
                }
            }
        }
        AttributeArgs::Named(pairs) => {
            for (_, expr) in pairs {
                if let ExprKind::Literal(LiteralValue::String(s)) = &expr.kind {
                    if is_deny {
                        emitter.filter_mut().deny_name(s);
                    } else {
                        emitter.filter_mut().ignore_name(s);
                    }
                }
            }
        }
        AttributeArgs::None => {}
    }
}

fn extract_filter_from_attrs(attrs: &[AttributeNode], emitter: &mut DiagnosticEmitter) {
    for attr in attrs {
        let name = attr.path_str();
        if is_lint_ignore(&name) {
            apply_lint_rule(attr, false, emitter);
        } else if is_lint_deny(&name) {
            apply_lint_rule(attr, true, emitter);
        }
    }
}

fn check_unused_imports(
    modules: &[LoadedModule],
    source_map: &SourceMap,
    emitter: &mut DiagnosticEmitter,
) {
    for m in modules {
        if m.is_external {
            continue;
        }
        let mut usages = HashSet::new();
        for item in &m.ast.items {
            if !matches!(item.kind, ItemKind::Import(_)) {
                collect_ident_usages_item(item, &mut usages);
            }
        }
        for item in &m.ast.items {
            if let ItemKind::Alias(decl) = &item.kind {
                record_alias_usages(decl, &mut usages);
            }
        }
        for item in &m.ast.items {
            if let ItemKind::Import(decl) = &item.kind {
                check_module_unused_imports(decl, &usages, source_map, emitter);
            }
        }
    }
}

fn record_alias_usages(decl: &yukic_ast::AliasDecl, usages: &mut HashSet<String>) {
    if !usages.contains(&decl.alias.name) {
        return;
    }
    match &decl.kind {
        yukic_ast::AliasKind::Namespace(idents) => {
            for id in idents {
                usages.insert(id.name.clone());
            }
        }
        yukic_ast::AliasKind::Symbol { namespace, symbol } => {
            usages.insert(symbol.name.clone());
            for id in namespace {
                usages.insert(id.name.clone());
            }
        }
        yukic_ast::AliasKind::Direct(orig) => {
            usages.insert(orig.name.clone());
        }
    }
}

fn check_module_unused_imports(
    decl: &yukic_ast::ImportDecl,
    usages: &HashSet<String>,
    source_map: &SourceMap,
    emitter: &mut DiagnosticEmitter,
) {
    let Some(clause) = &decl.symbols else {
        return;
    };
    for (idx, sym_item) in clause.items.iter().enumerate() {
        let yukic_ast::ImportPattern::Exact(id) = &sym_item.pattern else {
            continue;
        };
        let imported_name = clause
            .aliases
            .get(idx)
            .map(|a| a.name.as_str())
            .unwrap_or(id.name.as_str());

        if !usages.contains(imported_name) {
            let diag = Diagnostic::warning(
                WarningCode::W0002,
                format!("unused import '{imported_name}'"),
                sym_item.span,
            );
            let _ = emitter.emit(diag, source_map, &mut std::io::stderr());
        }
    }
}

fn check_dead_functions(
    modules: &[LoadedModule],
    source_map: &SourceMap,
    emitter: &mut DiagnosticEmitter,
) {
    let mut all_usages = HashSet::new();
    for m in modules {
        for item in &m.ast.items {
            collect_ident_usages_item(item, &mut all_usages);
        }
    }

    for m in modules {
        if m.is_external {
            continue;
        }
        for item in &m.ast.items {
            if let ItemKind::Fun(fun) = &item.kind {
                if fun.name.name == "main" || item.visibility == Visibility::Public {
                    continue;
                }
                if fun.attrs.iter().any(|a| a.is("test")) {
                    if fun.attrs.iter().any(|a| a.is("ignore")) {
                        let diag = Diagnostic::warning(
                            WarningCode::W0423,
                            format!("test function '{}' is marked as ignored", fun.name.name),
                            fun.name.span,
                        );
                        let _ = emitter.emit(diag, source_map, &mut std::io::stderr());
                    }
                    continue;
                }
                if !all_usages.contains(&fun.name.name) {
                    let diag = Diagnostic::warning(
                        WarningCode::W0003,
                        format!("function '{}' is never used", fun.name.name),
                        fun.name.span,
                    );
                    let _ = emitter.emit(diag, source_map, &mut std::io::stderr());
                }
            }
        }
    }
}

fn check_unused_variables(
    modules: &[LoadedModule],
    source_map: &SourceMap,
    emitter: &mut DiagnosticEmitter,
) {
    for m in modules {
        if m.is_external {
            continue;
        }
        for item in &m.ast.items {
            match &item.kind {
                ItemKind::Fun(fun) => {
                    check_vars_in_block(&fun.body, source_map, emitter);
                }
                ItemKind::Cfun(cfun) => {
                    if let Some(body) = &cfun.body {
                        check_vars_in_block(body, source_map, emitter);
                    }
                }
                ItemKind::Class(cls) => {
                    for method in &cls.methods {
                        check_vars_in_block(&method.body, source_map, emitter);
                    }
                }
                _ => {}
            }
        }
    }
}

fn check_vars_in_block(
    block: &Block,
    source_map: &SourceMap,
    emitter: &mut DiagnosticEmitter,
) {
    for (i, stmt) in block.stmts.iter().enumerate() {
        let bound: Vec<Ident> = match &stmt.kind {
            StmtKind::Set { target, .. } => {
                let mut ids = Vec::new();
                collect_bound_idents(target, &mut ids);
                ids
            }
            StmtKind::Static(decl) => vec![decl.name.clone()],
            _ => Vec::new(),
        };

        if !bound.is_empty() {
            let mut subsequent_usages = HashSet::new();
            for subsequent in &block.stmts[i + 1..] {
                collect_ident_usages_stmt(subsequent, &mut subsequent_usages);
            }

            for id in bound {
                if !id.name.starts_with('_') && !subsequent_usages.contains(&id.name) {
                    let diag = Diagnostic::warning(
                        WarningCode::W0001,
                        format!("unused variable '{}'", id.name),
                        id.span,
                    );
                    let _ = emitter.emit(diag, source_map, &mut std::io::stderr());
                }
            }
        }
        check_vars_in_stmt(stmt, source_map, emitter);
    }
}

fn check_vars_in_stmt(
    stmt: &Stmt,
    source_map: &SourceMap,
    emitter: &mut DiagnosticEmitter,
) {
    match &stmt.kind {
        StmtKind::Expr(expr) => check_vars_in_expr(expr, source_map, emitter),
        StmtKind::Defer(blk) => check_vars_in_block(blk, source_map, emitter),
        _ => {}
    }
}

fn check_vars_in_expr(
    expr: &Expr,
    source_map: &SourceMap,
    emitter: &mut DiagnosticEmitter,
) {
    match &expr.kind {
        ExprKind::Block(blk) | ExprKind::RegionBlock(blk) | ExprKind::Loop { body: blk, .. } => {
            check_vars_in_block(blk, source_map, emitter);
        }
        ExprKind::If {
            then_branch,
            elif_branches,
            else_branch,
            ..
        } => {
            check_vars_in_block(then_branch, source_map, emitter);
            for (_, elif_blk) in elif_branches {
                check_vars_in_block(elif_blk, source_map, emitter);
            }
            if let Some(else_blk) = else_branch {
                check_vars_in_block(else_blk, source_map, emitter);
            }
        }
        ExprKind::While { body, .. } | ExprKind::For { body, .. } => {
            check_vars_in_block(body, source_map, emitter);
        }
        ExprKind::Switch { arms, .. } => {
            for arm in arms {
                check_vars_in_block(&arm.body, source_map, emitter);
            }
        }
        ExprKind::Sus {
            body,
            catches,
            finally_branch,
        } => {
            check_vars_in_block(body, source_map, emitter);
            for c in catches {
                check_vars_in_block(&c.body, source_map, emitter);
            }
            if let Some(fin) = finally_branch {
                check_vars_in_block(fin, source_map, emitter);
            }
        }
        _ => {}
    }
}

fn collect_bound_idents(target: &BindingTarget, out: &mut Vec<Ident>) {
    match target {
        BindingTarget::Ident(id) => out.push(id.clone()),
        BindingTarget::Tuple(targets) => {
            for t in targets {
                collect_bound_idents(t, out);
            }
        }
        BindingTarget::Array(elements) => {
            for elem in elements {
                collect_bound_idents(&elem.target, out);
            }
        }
        BindingTarget::Map(fields) => {
            for f in fields {
                if let Some(alias) = &f.alias {
                    collect_bound_idents(alias, out);
                } else {
                    out.push(f.field.clone());
                }
            }
        }
    }
}

fn collect_ident_usages_item(item: &Item, usages: &mut HashSet<String>) {
    match &item.kind {
        ItemKind::Fun(fun) => collect_ident_usages_fun(fun, usages),
        ItemKind::Cfun(cfun) => collect_ident_usages_cfun(cfun, usages),
        ItemKind::Class(cls) => {
            for method in &cls.methods {
                collect_ident_usages_method(method, usages);
            }
        }
        ItemKind::Static(st) => {
            if let Some(ty) = &st.ty {
                collect_ident_usages_type(ty, usages);
            }
            collect_ident_usages_expr(&st.init, usages);
        }
        _ => {}
    }
}

fn collect_ident_usages_cfun(cfun: &yukic_ast::item::CfunDecl, usages: &mut HashSet<String>) {
    for p in &cfun.params {
        collect_ident_usages_type(&p.ty, usages);
    }
    if let Some(ret) = &cfun.return_type {
        collect_ident_usages_type(ret, usages);
    }
    if let Some(body) = &cfun.body {
        collect_ident_usages_block(body, usages);
    }
}

fn collect_ident_usages_fun(fun: &FunDecl, usages: &mut HashSet<String>) {
    for p in &fun.params {
        if let Some(ty) = &p.ty {
            collect_ident_usages_type(ty, usages);
        }
        if let Some(def) = &p.default_val {
            collect_ident_usages_expr(def, usages);
        }
    }
    if let Some(ret) = &fun.return_type {
        collect_ident_usages_type(ret, usages);
    }
    collect_ident_usages_block(&fun.body, usages);
}

fn collect_ident_usages_method(method: &MethodDecl, usages: &mut HashSet<String>) {
    for p in &method.params {
        if let Some(ty) = &p.ty {
            collect_ident_usages_type(ty, usages);
        }
    }
    if let Some(ret) = &method.return_type {
        collect_ident_usages_type(ret, usages);
    }
    collect_ident_usages_block(&method.body, usages);
}

fn collect_ident_usages_block(block: &Block, usages: &mut HashSet<String>) {
    for stmt in &block.stmts {
        collect_ident_usages_stmt(stmt, usages);
    }
}

fn collect_ident_usages_stmt(stmt: &Stmt, usages: &mut HashSet<String>) {
    match &stmt.kind {
        StmtKind::Set { ty, init, .. } => {
            if let Some(t) = ty {
                collect_ident_usages_type(t, usages);
            }
            collect_ident_usages_expr(init, usages);
        }
        StmtKind::Static(decl) => {
            if let Some(t) = &decl.ty {
                collect_ident_usages_type(t, usages);
            }
            collect_ident_usages_expr(&decl.init, usages);
        }
        StmtKind::Rewrite { target, value, .. } => {
            collect_ident_usages_lvalue(target, usages);
            collect_ident_usages_expr(value, usages);
        }
        StmtKind::Expr(expr) | StmtKind::Return(Some(expr)) => {
            collect_ident_usages_expr(expr, usages);
        }
        StmtKind::Recur(exprs) => {
            for expr in exprs {
                collect_ident_usages_expr(expr, usages);
            }
        }
        StmtKind::Defer(blk) => collect_ident_usages_block(blk, usages),
        _ => {}
    }
}

fn collect_ident_usages_lvalue(lval: &LValue, usages: &mut HashSet<String>) {
    match lval {
        LValue::Ident(id) => {
            usages.insert(id.name.clone());
        }
        LValue::Field(expr, _) => collect_ident_usages_expr(expr, usages),
        LValue::Index(arr, idx) => {
            collect_ident_usages_expr(arr, usages);
            collect_ident_usages_expr(idx, usages);
        }
    }
}

fn collect_ident_usages_type(ty: &TypeNode, usages: &mut HashSet<String>) {
    match &ty.kind {
        TypeNodeKind::Ident(id) => {
            usages.insert(id.name.clone());
        }
        TypeNodeKind::Path(ids) => {
            for id in ids {
                usages.insert(id.name.clone());
            }
        }
        TypeNodeKind::Pointer(inner) | TypeNodeKind::Ref(inner) | TypeNodeKind::Optional(inner) => {
            collect_ident_usages_type(inner, usages);
        }
        TypeNodeKind::Array(elem, _) => collect_ident_usages_type(elem, usages),
        TypeNodeKind::Map(k, v) | TypeNodeKind::Result(k, v) => {
            collect_ident_usages_type(k, usages);
            collect_ident_usages_type(v, usages);
        }
        TypeNodeKind::Tuple(types) | TypeNodeKind::Union(types) => {
            for t in types {
                collect_ident_usages_type(t, usages);
            }
        }
        TypeNodeKind::Fun(params, ret) | TypeNodeKind::CFun(params, ret) => {
            for p in params {
                collect_ident_usages_type(p, usages);
            }
            collect_ident_usages_type(ret, usages);
        }
        TypeNodeKind::Generic(id, args) => {
            usages.insert(id.name.clone());
            for arg in args {
                collect_ident_usages_type(arg, usages);
            }
        }
    }
}

fn collect_ident_usages_expr(expr: &Expr, usages: &mut HashSet<String>) {
    match &expr.kind {
        ExprKind::Ident(id) | ExprKind::PtrOf(id) => {
            usages.insert(id.name.clone());
        }
        ExprKind::Path(ids) => {
            for id in ids {
                usages.insert(id.name.clone());
            }
        }
        ExprKind::Unary(_, inner) | ExprKind::Spread(inner) => {
            collect_ident_usages_expr(inner, usages);
        }
        ExprKind::Binary(_, l, r) | ExprKind::Pipe(l, r) => {
            collect_ident_usages_expr(l, usages);
            collect_ident_usages_expr(r, usages);
        }
        ExprKind::Ternary(c, t, e) => {
            collect_ident_usages_expr(c, usages);
            collect_ident_usages_expr(t, usages);
            collect_ident_usages_expr(e, usages);
        }
        ExprKind::Call(callee, args) => {
            collect_ident_usages_expr(callee, usages);
            for arg in args {
                collect_ident_usages_expr(arg, usages);
            }
        }
        ExprKind::MethodCall(recv, _, args)
        | ExprKind::QualifiedMethodCall(recv, _, _, args)
        | ExprKind::SafeCall(recv, _, args) => {
            collect_ident_usages_expr(recv, usages);
            for arg in args {
                collect_ident_usages_expr(arg, usages);
            }
        }
        ExprKind::FieldAccess(recv, _) => collect_ident_usages_expr(recv, usages),
        ExprKind::IndexAccess(arr, idx) => {
            collect_ident_usages_expr(arr, usages);
            collect_ident_usages_expr(idx, usages);
        }
        ExprKind::SliceAccess(arr, start, end, _) => {
            collect_ident_usages_expr(arr, usages);
            if let Some(s) = start {
                collect_ident_usages_expr(s, usages);
            }
            if let Some(e) = end {
                collect_ident_usages_expr(e, usages);
            }
        }
        ExprKind::StructInit(name, fields) => {
            usages.insert(name.name.clone());
            for f in fields {
                collect_ident_usages_expr(&f.value, usages);
            }
        }
        ExprKind::ClassInit(target, args) => {
            collect_ident_usages_expr(target, usages);
            for arg in args {
                collect_ident_usages_expr(arg, usages);
            }
        }
        ExprKind::ArrayInit(items) | ExprKind::TupleInit(items) => {
            for item in items {
                collect_ident_usages_expr(item, usages);
            }
        }
        ExprKind::ArrayAlloc(elem_ty, _) => collect_ident_usages_type(elem_ty, usages),
        ExprKind::MapInit(entries) => {
            for (k, v) in entries {
                collect_ident_usages_expr(k, usages);
                collect_ident_usages_expr(v, usages);
            }
        }
        ExprKind::Block(blk) | ExprKind::RegionBlock(blk) | ExprKind::Loop { body: blk, .. } => {
            collect_ident_usages_block(blk, usages);
        }
        ExprKind::If {
            cond,
            then_branch,
            elif_branches,
            else_branch,
        } => {
            collect_ident_usages_expr(cond, usages);
            collect_ident_usages_block(then_branch, usages);
            for (elif_c, elif_b) in elif_branches {
                collect_ident_usages_expr(elif_c, usages);
                collect_ident_usages_block(elif_b, usages);
            }
            if let Some(else_b) = else_branch {
                collect_ident_usages_block(else_b, usages);
            }
        }
        ExprKind::Switch { target, arms } => {
            collect_ident_usages_expr(target, usages);
            for arm in arms {
                collect_ident_usages_pattern(&arm.pattern, usages);
                if let Some(g) = &arm.guard {
                    collect_ident_usages_expr(g, usages);
                }
                collect_ident_usages_block(&arm.body, usages);
            }
        }
        ExprKind::While { cond, body, .. } => {
            collect_ident_usages_expr(cond, usages);
            collect_ident_usages_block(body, usages);
        }
        ExprKind::For { iter, body, .. } => {
            collect_ident_usages_expr(iter, usages);
            collect_ident_usages_block(body, usages);
        }
        ExprKind::Sus {
            body,
            catches,
            finally_branch,
        } => {
            collect_ident_usages_block(body, usages);
            for c in catches {
                if let Some(err_ty) = &c.error_type {
                    collect_ident_usages_type(err_ty, usages);
                }
                collect_ident_usages_block(&c.body, usages);
            }
            if let Some(fin) = finally_branch {
                collect_ident_usages_block(fin, usages);
            }
        }
        ExprKind::Lambda(_, body) => collect_ident_usages_expr(body, usages),
        ExprKind::AnonymousFun(fun) => collect_ident_usages_fun(fun, usages),
        ExprKind::PtrCast(expr, ty) | ExprKind::Is(expr, ty) => {
            collect_ident_usages_expr(expr, usages);
            collect_ident_usages_type(ty, usages);
        }
        ExprKind::Ref { expr, .. } | ExprKind::NamedArg(_, expr) => collect_ident_usages_expr(expr, usages),
        ExprKind::InterpolatedString(parts) => {
            for part in parts {
                if let StringPart::Expr(sub) = part {
                    collect_ident_usages_expr(sub, usages);
                }
            }
        }
        _ => {}
    }
}

fn collect_ident_usages_pattern(pattern: &Pattern, usages: &mut HashSet<String>) {
    match &pattern.kind {
        PatternKind::Struct(name, fields) => {
            usages.insert(name.name.clone());
            for (_, p) in fields {
                collect_ident_usages_pattern(p, usages);
            }
        }
        PatternKind::Variant(opt_enum, variant, patterns) => {
            if let Some(e) = opt_enum {
                usages.insert(e.name.clone());
            }
            usages.insert(variant.name.clone());
            for p in patterns {
                collect_ident_usages_pattern(p, usages);
            }
        }
        PatternKind::Tuple(patterns) | PatternKind::Or(patterns) => {
            for p in patterns {
                collect_ident_usages_pattern(p, usages);
            }
        }
        _ => {}
    }
}

/// Declaration counts and the message of the last `#[deprecated]` one.
#[derive(Clone, Debug, Default)]
struct DeprecatedDecl {
    kind_name: &'static str,
    name: String,
    message: Option<String>,
    declarations: usize,
    deprecated: usize,
}

#[derive(Default)]
struct DeprecationIndex {
    items: HashMap<String, DeprecatedDecl>,
    methods: HashMap<String, DeprecatedDecl>,
}

impl DeprecationIndex {
    fn record_item(&mut self, kind_name: &'static str, name: &str, attrs: &[AttributeNode]) {
        let entry = self.items.entry(name.to_string()).or_insert_with(|| DeprecatedDecl {
            kind_name,
            name: name.to_string(),
            message: None,
            declarations: 0,
            deprecated: 0,
        });
        entry.declarations += 1;
        if let Some(msg) = extract_deprecated_info(attrs) {
            entry.deprecated += 1;
            entry.message = msg;
        }
    }

    fn record_method(&mut self, kind_name: &'static str, name: &str, attrs: &[AttributeNode]) {
        let entry = self.methods.entry(name.to_string()).or_insert_with(|| DeprecatedDecl {
            kind_name,
            name: name.to_string(),
            message: None,
            declarations: 0,
            deprecated: 0,
        });
        entry.declarations += 1;
        if let Some(msg) = extract_deprecated_info(attrs) {
            entry.deprecated += 1;
            entry.message = msg;
        }
    }

    fn item_dep(&self, name: &str) -> Option<&DeprecatedDecl> {
        self.items
            .get(name)
            .filter(|s| s.deprecated > 0 && s.deprecated == s.declarations)
    }

    fn method_dep(&self, name: &str) -> Option<&DeprecatedDecl> {
        self.methods
            .get(name)
            .filter(|s| s.deprecated > 0 && s.deprecated == s.declarations)
    }

    fn is_empty(&self) -> bool {
        self.items.values().all(|s| s.deprecated == 0) && self.methods.values().all(|s| s.deprecated == 0)
    }
}

fn extract_deprecated_info(attrs: &[AttributeNode]) -> Option<Option<String>> {
    for a in attrs {
        if a.path_str() == "deprecated" {
            let message = match &a.args {
                AttributeArgs::Positional(exprs) if !exprs.is_empty() => match &exprs[0].kind {
                    ExprKind::Literal(LiteralValue::String(s)) => Some(s.clone()),
                    _ => None,
                },
                AttributeArgs::Named(pairs) => pairs.iter().find_map(|(_, expr)| match &expr.kind {
                    ExprKind::Literal(LiteralValue::String(s)) => Some(s.clone()),
                    _ => None,
                }),
                _ => None,
            };
            return Some(message);
        }
    }
    None
}

fn collect_deprecated_item(item: &Item, index: &mut DeprecationIndex) {
    match &item.kind {
        ItemKind::Fun(fun) => {
            let mut attrs = fun.attrs.clone();
            attrs.extend_from_slice(&item.attrs);
            index.record_item("function", &fun.name.name, &attrs);
        }
        ItemKind::Cfun(cfun) => {
            let mut attrs = cfun.yukino_attrs.clone();
            attrs.extend_from_slice(&item.attrs);
            index.record_item("function", &cfun.name.name, &attrs);
        }
        ItemKind::Struct(st) => {
            index.record_item("struct", &st.name.name, &item.attrs);
            for method in &st.methods {
                index.record_method("method", &method.name.name, &method.attrs);
            }
            for sm in &st.static_methods {
                index.record_method("static method", &sm.name.name, &sm.attrs);
            }
        }
        ItemKind::Class(cls) => {
            index.record_item("class", &cls.name.name, &item.attrs);
            for method in &cls.methods {
                index.record_method("method", &method.name.name, &method.attrs);
            }
        }
        ItemKind::Interface(iface) => {
            index.record_item("interface", &iface.name.name, &item.attrs);
            for method in &iface.methods {
                index.record_method("method", &method.name.name, &method.attrs);
            }
        }
        ItemKind::Enum(en) => {
            index.record_item("enum", &en.name.name, &item.attrs);
        }
        ItemKind::TypeAlias(alias) => {
            index.record_item("type alias", &alias.name.name, &item.attrs);
        }
        _ => {}
    }
}

fn collect_deprecated_decls(modules: &[LoadedModule]) -> DeprecationIndex {
    let mut index = DeprecationIndex::default();
    for m in modules {
        for item in &m.ast.items {
            collect_deprecated_item(item, &mut index);
        }
    }
    index
}

fn check_deprecated_usages(
    modules: &[LoadedModule],
    source_map: &SourceMap,
    emitter: &mut DiagnosticEmitter,
) {
    let index = collect_deprecated_decls(modules);
    if index.is_empty() {
        return;
    }

    let mut checker = DeprecatedUsageChecker {
        index: &index,
        source_map,
        emitter,
        in_deprecated_item: false,
    };

    for m in modules {
        if m.is_external {
            continue;
        }
        for item in &m.ast.items {
            checker.check_item(item);
        }
    }
}

struct DeprecatedUsageChecker<'a> {
    index: &'a DeprecationIndex,
    source_map: &'a SourceMap,
    emitter: &'a mut DiagnosticEmitter,
    in_deprecated_item: bool,
}

impl DeprecatedUsageChecker<'_> {
    fn emit_warning(&mut self, dep: &DeprecatedDecl, span: Span) {
        if self.in_deprecated_item {
            return;
        }
        let reason = match &dep.message {
            Some(msg) => format!(": {msg}"),
            None => String::new(),
        };
        let message = format!("{} '{}' is deprecated{reason}", dep.kind_name, dep.name);
        let diag = Diagnostic::warning(WarningCode::W0418, message, span);
        let _ = self.emitter.emit(diag, self.source_map, &mut std::io::stderr());
    }

    fn check_item(&mut self, item: &Item) {
        let item_is_dep = extract_deprecated_info(&item.attrs).is_some();
        match &item.kind {
            ItemKind::Fun(fun) => {
                let is_dep = item_is_dep || extract_deprecated_info(&fun.attrs).is_some();
                let prev = self.in_deprecated_item;
                if is_dep && fun.name.name != "main" {
                    self.in_deprecated_item = true;
                }
                for p in &fun.params {
                    if let Some(ty) = &p.ty {
                        self.check_type(ty);
                    }
                    if let Some(def) = &p.default_val {
                        self.check_expr(def);
                    }
                }
                if let Some(ret) = &fun.return_type {
                    self.check_type(ret);
                }
                self.check_block(&fun.body);
                self.in_deprecated_item = prev;
            }
            ItemKind::Cfun(cfun) => {
                let is_dep = item_is_dep || extract_deprecated_info(&cfun.yukino_attrs).is_some();
                let prev = self.in_deprecated_item;
                if is_dep {
                    self.in_deprecated_item = true;
                }
                for p in &cfun.params {
                    self.check_type(&p.ty);
                }
                if let Some(ret) = &cfun.return_type {
                    self.check_type(ret);
                }
                if let Some(body) = &cfun.body {
                    self.check_block(body);
                }
                self.in_deprecated_item = prev;
            }
            ItemKind::Class(cls) => {
                let prev_cls = self.in_deprecated_item;
                if item_is_dep {
                    self.in_deprecated_item = true;
                }
                for method in &cls.methods {
                    let is_dep = extract_deprecated_info(&method.attrs).is_some();
                    let prev_method = self.in_deprecated_item;
                    if is_dep {
                        self.in_deprecated_item = true;
                    }
                    for p in &method.params {
                        if let Some(ty) = &p.ty {
                            self.check_type(ty);
                        }
                    }
                    if let Some(ret) = &method.return_type {
                        self.check_type(ret);
                    }
                    self.check_block(&method.body);
                    self.in_deprecated_item = prev_method;
                }
                for accessor in &cls.accessors {
                    self.check_block(&accessor.body);
                }
                self.in_deprecated_item = prev_cls;
            }
            ItemKind::Struct(st) => {
                let prev_st = self.in_deprecated_item;
                if item_is_dep {
                    self.in_deprecated_item = true;
                }
                for method in &st.methods {
                    let is_dep = extract_deprecated_info(&method.attrs).is_some();
                    let prev_method = self.in_deprecated_item;
                    if is_dep {
                        self.in_deprecated_item = true;
                    }
                    for p in &method.params {
                        if let Some(ty) = &p.ty {
                            self.check_type(ty);
                        }
                    }
                    if let Some(ret) = &method.return_type {
                        self.check_type(ret);
                    }
                    self.check_block(&method.body);
                    self.in_deprecated_item = prev_method;
                }
                for sm in &st.static_methods {
                    let is_dep = extract_deprecated_info(&sm.attrs).is_some();
                    let prev_sm = self.in_deprecated_item;
                    if is_dep {
                        self.in_deprecated_item = true;
                    }
                    for p in &sm.params {
                        if let Some(ty) = &p.ty {
                            self.check_type(ty);
                        }
                    }
                    if let Some(ret) = &sm.return_type {
                        self.check_type(ret);
                    }
                    self.check_block(&sm.body);
                    self.in_deprecated_item = prev_sm;
                }
                self.in_deprecated_item = prev_st;
            }
            ItemKind::Interface(iface) => {
                for method in &iface.methods {
                    if let Some(body) = &method.default_body {
                        self.check_block(body);
                    }
                }
            }
            ItemKind::Static(st) => {
                if let Some(ty) = &st.ty {
                    self.check_type(ty);
                }
                self.check_expr(&st.init);
            }
            _ => {}
        }
    }

    fn check_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Set { ty, init, .. } => {
                if let Some(t) = ty {
                    self.check_type(t);
                }
                self.check_expr(init);
            }
            StmtKind::Static(decl) => {
                if let Some(t) = &decl.ty {
                    self.check_type(t);
                }
                self.check_expr(&decl.init);
            }
            StmtKind::Rewrite { target, value, .. } => {
                self.check_lvalue(target);
                self.check_expr(value);
            }
            StmtKind::Expr(expr) | StmtKind::Return(Some(expr)) => {
                self.check_expr(expr);
            }
            StmtKind::Recur(exprs) => {
                for expr in exprs {
                    self.check_expr(expr);
                }
            }
            StmtKind::Defer(blk) => self.check_block(blk),
            _ => {}
        }
    }

    fn check_lvalue(&mut self, lval: &LValue) {
        match lval {
            LValue::Ident(_) => {}
            LValue::Field(expr, _) => self.check_expr(expr),
            LValue::Index(arr, idx) => {
                self.check_expr(arr);
                self.check_expr(idx);
            }
        }
    }

    fn check_type(&mut self, ty: &TypeNode) {
        match &ty.kind {
            TypeNodeKind::Ident(id) => {
                if let Some(dep) = self.index.item_dep(&id.name) {
                    self.emit_warning(dep, id.span);
                }
            }
            TypeNodeKind::Path(ids) => {
                if let Some(first) = ids.first()
                    && let Some(dep) = self.index.item_dep(&first.name)
                {
                    self.emit_warning(dep, first.span);
                }
            }
            TypeNodeKind::Pointer(inner) | TypeNodeKind::Ref(inner) | TypeNodeKind::Optional(inner) => {
                self.check_type(inner);
            }
            TypeNodeKind::Array(elem, _) => self.check_type(elem),
            TypeNodeKind::Map(k, v) | TypeNodeKind::Result(k, v) => {
                self.check_type(k);
                self.check_type(v);
            }
            TypeNodeKind::Tuple(types) | TypeNodeKind::Union(types) => {
                for t in types {
                    self.check_type(t);
                }
            }
            TypeNodeKind::Fun(params, ret) | TypeNodeKind::CFun(params, ret) => {
                for p in params {
                    self.check_type(p);
                }
                self.check_type(ret);
            }
            TypeNodeKind::Generic(id, args) => {
                if let Some(dep) = self.index.item_dep(&id.name) {
                    self.emit_warning(dep, id.span);
                }
                for arg in args {
                    self.check_type(arg);
                }
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Ident(id) => {
                if let Some(dep) = self.index.item_dep(&id.name) {
                    if dep.kind_name == "static" || dep.kind_name == "enum" {
                        self.emit_warning(dep, id.span);
                    }
                }
            }
            ExprKind::Call(callee, args) => {
                match &callee.kind {
                    ExprKind::Ident(id) => {
                        if let Some(dep) = self.index.item_dep(&id.name) {
                            self.emit_warning(dep, id.span);
                        }
                    }
                    ExprKind::Path(segments) if !segments.is_empty() => {
                        let last = &segments[segments.len() - 1];
                        if let Some(dep) = self.index.item_dep(&last.name) {
                            self.emit_warning(dep, last.span);
                        } else if let Some(dep) = self.index.method_dep(&last.name) {
                            self.emit_warning(dep, last.span);
                        }
                    }
                    _ => {
                        self.check_expr(callee);
                    }
                }
                for arg in args {
                    self.check_expr(arg);
                }
            }
            ExprKind::MethodCall(recv, method_id, args)
            | ExprKind::QualifiedMethodCall(recv, _, method_id, args)
            | ExprKind::SafeCall(recv, method_id, args) => {
                self.check_expr(recv);
                if let Some(dep) = self.index.method_dep(&method_id.name) {
                    self.emit_warning(dep, method_id.span);
                }
                for arg in args {
                    self.check_expr(arg);
                }
            }
            ExprKind::FieldAccess(recv, _) => self.check_expr(recv),
            ExprKind::IndexAccess(arr, idx) => {
                self.check_expr(arr);
                self.check_expr(idx);
            }
            ExprKind::SliceAccess(arr, start, end, _) => {
                self.check_expr(arr);
                if let Some(s) = start {
                    self.check_expr(s);
                }
                if let Some(e) = end {
                    self.check_expr(e);
                }
            }
            ExprKind::StructInit(name, fields) => {
                if let Some(dep) = self.index.item_dep(&name.name) {
                    self.emit_warning(dep, name.span);
                }
                for f in fields {
                    self.check_expr(&f.value);
                }
            }
            ExprKind::ClassInit(target, args) => {
                if let ExprKind::Ident(id) = &target.kind {
                    if let Some(dep) = self.index.item_dep(&id.name) {
                        self.emit_warning(dep, id.span);
                    }
                } else {
                    self.check_expr(target);
                }
                for arg in args {
                    self.check_expr(arg);
                }
            }
            ExprKind::ArrayInit(items) | ExprKind::TupleInit(items) => {
                for item in items {
                    self.check_expr(item);
                }
            }
            ExprKind::ArrayAlloc(elem_ty, _) => self.check_type(elem_ty),
            ExprKind::MapInit(entries) => {
                for (k, v) in entries {
                    self.check_expr(k);
                    self.check_expr(v);
                }
            }
            ExprKind::Block(blk) | ExprKind::RegionBlock(blk) | ExprKind::Loop { body: blk, .. } => {
                self.check_block(blk);
            }
            ExprKind::If {
                cond,
                then_branch,
                elif_branches,
                else_branch,
            } => {
                self.check_expr(cond);
                self.check_block(then_branch);
                for (elif_c, elif_b) in elif_branches {
                    self.check_expr(elif_c);
                    self.check_block(elif_b);
                }
                if let Some(else_b) = else_branch {
                    self.check_block(else_b);
                }
            }
            ExprKind::Switch { target, arms } => {
                self.check_expr(target);
                for arm in arms {
                    self.check_pattern(&arm.pattern);
                    if let Some(g) = &arm.guard {
                        self.check_expr(g);
                    }
                    self.check_block(&arm.body);
                }
            }
            ExprKind::Unary(_, inner)
            | ExprKind::Spread(inner)
            | ExprKind::Ref { expr: inner, .. }
            | ExprKind::NamedArg(_, inner) => {
                self.check_expr(inner);
            }
            ExprKind::Binary(_, l, r) | ExprKind::Pipe(l, r) => {
                self.check_expr(l);
                self.check_expr(r);
            }
            ExprKind::Ternary(c, t, e) => {
                self.check_expr(c);
                self.check_expr(t);
                self.check_expr(e);
            }
            ExprKind::While { cond, body, .. } => {
                self.check_expr(cond);
                self.check_block(body);
            }
            ExprKind::For { iter, body, .. } => {
                self.check_expr(iter);
                self.check_block(body);
            }
            ExprKind::Sus { body, catches, finally_branch } => {
                self.check_block(body);
                for c in catches {
                    if let Some(err_ty) = &c.error_type {
                        self.check_type(err_ty);
                    }
                    self.check_block(&c.body);
                }
                if let Some(fin) = finally_branch {
                    self.check_block(fin);
                }
            }
            ExprKind::Lambda(_, body) => self.check_expr(body),
            ExprKind::AnonymousFun(fun) => self.check_block(&fun.body),
            ExprKind::PtrCast(expr, ty) | ExprKind::Is(expr, ty) => {
                self.check_expr(expr);
                self.check_type(ty);
            }
            ExprKind::InterpolatedString(parts) => {
                for part in parts {
                    if let StringPart::Expr(sub) = part {
                        self.check_expr(sub);
                    }
                }
            }
            _ => {}
        }
    }

    fn check_pattern(&mut self, pattern: &Pattern) {
        match &pattern.kind {
            PatternKind::Struct(name, fields) => {
                if let Some(dep) = self.index.item_dep(&name.name) {
                    self.emit_warning(dep, name.span);
                }
                for (_, p) in fields {
                    self.check_pattern(p);
                }
            }
            PatternKind::Variant(opt_enum, _, patterns) => {
                if let Some(e) = opt_enum
                    && let Some(dep) = self.index.item_dep(&e.name)
                {
                    self.emit_warning(dep, e.span);
                }
                for p in patterns {
                    self.check_pattern(p);
                }
            }
            PatternKind::Tuple(patterns) | PatternKind::Or(patterns) => {
                for p in patterns {
                    self.check_pattern(p);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_ast::item::{CParam, CfunDecl, ItemId};
    use yukic_diagnostics::emitter::ColorChoice;
    use yukic_lexer::Lexer;
    use yukic_parser::Parser;
    use yukic_span::Span;

    fn parse_module(source: &str, source_map: &mut SourceMap) -> LoadedModule {
        let file_id = source_map.add_file("test.ykn".to_string(), source.to_string());
        let lexer = Lexer::new(file_id, source);
        let (tokens, _) = lexer.tokenize();
        let mut parser = Parser::with_file_id(tokens, file_id);
        let (ast, _) = parser.parse_module();
        LoadedModule {
            name: "test".to_string(),
            path: std::path::PathBuf::from("test.ykn"),
            ast,
            is_external: false,
            tracked_files: Vec::new(),
        }
    }

    fn run(modules: Vec<LoadedModule>, source_map: &SourceMap) -> DiagnosticEmitter {
        let mut emitter = DiagnosticEmitter::new(ColorChoice::Never);
        run_lint_pass(&modules, source_map, &mut emitter);
        emitter
    }

    #[test]
    fn deprecated_functions_and_methods_are_reported() {
        let source = r#"
#[deprecated = "use beta"]
fun alpha() { space }

fun beta() { space }

class Widget {
    #[deprecated]
    fun legacy() { space }

    fun modern() { space }
}

fun main() {
    alpha()
    beta()
    thing.legacy()
    thing.modern()
}
"#;
        let mut source_map = SourceMap::new();
        let module = parse_module(source, &mut source_map);
        let emitter = run(vec![module], &source_map);

        let messages: Vec<&str> = emitter
            .diagnostics()
            .iter()
            .map(|diag| diag.message.as_str())
            .collect();
        assert_eq!(
            messages,
            vec![
                "function 'alpha' is deprecated: use beta",
                "method 'legacy' is deprecated",
            ]
        );
    }

    #[test]
    fn deprecated_usage_warning_can_be_suppressed() {
        let source = r#"
#[deprecated]
fun alpha() { space }

fun main() {
    alpha()
}
"#;
        let mut source_map = SourceMap::new();
        let module = parse_module(source, &mut source_map);
        let mut emitter = DiagnosticEmitter::new(ColorChoice::Never);
        emitter.filter_mut().ignore_name("deprecated_usage");
        run_lint_pass(&[module], &source_map, &mut emitter);
        assert_eq!(emitter.warning_count(), 0);
    }

    #[test]
    fn ambiguous_names_are_not_reported() {
        let mut source_map = SourceMap::new();
        let deprecated = parse_module(
            "#[deprecated]\nfun alpha() { space }\n",
            &mut source_map,
        );
        let mut fresh = parse_module("fun alpha() { space }\n", &mut source_map);
        fresh.is_external = false;
        let caller = parse_module("fun main() {\n    alpha()\n}\n", &mut source_map);

        let emitter = run(vec![deprecated, fresh, caller], &source_map);
        assert_eq!(emitter.warning_count(), 0);
    }

    #[test]
    fn module_level_deprecated_marks_all_functions() {
        let source = r#"
!#[deprecated = "module retired"]

fun one() { space }

fun main() {
    one()
}
"#;
        let mut source_map = SourceMap::new();
        let mut module = parse_module(source, &mut source_map);
        yukic_ast::propagate_module_attributes(&mut module.ast);
        let emitter = run(vec![module], &source_map);

        let messages: Vec<&str> = emitter
            .diagnostics()
            .iter()
            .map(|diag| diag.message.as_str())
            .collect();
        assert_eq!(
            messages,
            vec!["function 'one' is deprecated: module retired"]
        );
    }

    #[test]
    fn test_collect_cfun_usages() {
        let dummy_span = Span::DUMMY;
        let param = CParam {
            name: Ident::new("arg", dummy_span),
            ty: TypeNode {
                kind: TypeNodeKind::Ident(Ident::new("CustomParamType", dummy_span)),
                span: dummy_span,
            },
            span: dummy_span,
        };
        let cfun = CfunDecl {
            name: Ident::new("worker", dummy_span),
            c_attrs: Vec::new(),
            yukino_attrs: Vec::new(),
            params: vec![param],
            return_type: Some(TypeNode {
                kind: TypeNodeKind::Ident(Ident::new("CustomReturnType", dummy_span)),
                span: dummy_span,
            }),
            symbol_override: None,
            body: Some(Block {
                stmts: vec![Stmt {
                    kind: StmtKind::Expr(Expr {
                        kind: ExprKind::Ident(Ident::new("used_in_body", dummy_span)),
                        span: dummy_span,
                    }),
                    span: dummy_span,
                }],
                span: dummy_span,
            }),
            span: dummy_span,
        };
        let item = Item::new(ItemId(0), Visibility::Local, ItemKind::Cfun(cfun), dummy_span);
        let mut usages = HashSet::new();
        collect_ident_usages_item(&item, &mut usages);

        assert!(usages.contains("CustomParamType"));
        assert!(usages.contains("CustomReturnType"));
        assert!(usages.contains("used_in_body"));
    }

    #[test]
    fn test_lint_ignore_and_deny_attributes_configuration() {
        use yukic_ast::attr::AttributePath;
        use yukic_diagnostics::code::WarningCode;
        use yukic_diagnostics::emitter::DiagnosticEmitter;
        use yukic_diagnostics::model::Diagnostic;
        use yukic_diagnostics::ColorChoice;
        use yukic_span::FileId;

        let dummy_span = Span::DUMMY;
        let ignore_attr = AttributeNode::new(
            AttributePath::new(
                vec![Ident::new("lint", dummy_span), Ident::new("ignore", dummy_span)],
                dummy_span,
            ),
            AttributeArgs::Positional(vec![Expr {
                kind: ExprKind::Ident(Ident::new("unused_variable", dummy_span)),
                span: dummy_span,
            }]),
            dummy_span,
        );
        let deny_attr = AttributeNode::new(
            AttributePath::new(
                vec![Ident::new("lint", dummy_span), Ident::new("deny", dummy_span)],
                dummy_span,
            ),
            AttributeArgs::Positional(vec![Expr {
                kind: ExprKind::Literal(LiteralValue::String("unused_import".to_string())),
                span: dummy_span,
            }]),
            dummy_span,
        );

        let loaded = LoadedModule {
            name: "test_mod".to_string(),
            path: std::path::PathBuf::from("test.ykn"),
            ast: yukic_ast::Module {
                file_id: FileId(0),
                items: Vec::new(),
                module_attrs: vec![ignore_attr, deny_attr],
            },
            is_external: false,
            tracked_files: Vec::new(),
        };

        let mut emitter = DiagnosticEmitter::new(ColorChoice::Never);
        configure_warning_filter(&[loaded], &mut emitter);

        let warn1 = Diagnostic::warning(WarningCode::W0001, "unused variable", dummy_span);
        let warn2 = Diagnostic::warning(WarningCode::W0002, "unused import", dummy_span);
        let sm = SourceMap::new();
        let mut buf = Vec::new();

        emitter.emit(warn1, &sm, &mut buf).unwrap();
        assert_eq!(emitter.warning_count(), 0);
        assert_eq!(emitter.error_count(), 0);

        emitter.emit(warn2, &sm, &mut buf).unwrap();
        assert_eq!(emitter.error_count(), 1);
        assert!(emitter.has_errors());
    }
}
