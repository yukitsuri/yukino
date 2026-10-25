#![deny(unsafe_code)]
#![allow(clippy::wildcard_imports, clippy::match_same_arms)]

use indexmap::IndexMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use yukic_ast::*;

static GENSYM_COUNTER: AtomicUsize = AtomicUsize::new(1);

/// gen a hygienic symbol name
pub fn gensym(prefix: &str) -> String {
    let id = GENSYM_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}_g{id}")
}

pub fn apply_hygiene(body: &mut MacroBody, params: &[MacroParam]) {
    let param_names: Vec<&str> = params.iter().map(|p| p.name.name.as_str()).collect();
    let mut declared_names = Vec::new();

    collect_declared_variables(body, &param_names, &mut declared_names);

    if declared_names.is_empty() { return; } // my father

    let mut rename_map: IndexMap<String, String> = IndexMap::new();
    for name in declared_names {
        if !rename_map.contains_key(&name) {
            rename_map.insert(name.clone(), gensym(&name));
        }
    }

    rename_macro_body(body, &rename_map);
}

fn collect_declared_variables(body: &MacroBody, params: &[&str], out: &mut Vec<String>) {
    match body {
        MacroBody::Block(block) => {
            for stmt in &block.stmts {
                collect_stmt_bindings(stmt, params, out);
            }
        }
        MacroBody::Expr(_) => {}
    }
}

fn collect_stmt_bindings(stmt: &Stmt, params: &[&str], out: &mut Vec<String>) {
    match &stmt.kind {
        StmtKind::Set { target, .. } => collect_target_bindings(target, params, out),
        StmtKind::Static(decl) => {
            if !params.contains(&decl.name.name.as_str()) {
                out.push(decl.name.name.clone());
            }
        }
        _ => {}
    }
}

fn collect_target_bindings(target: &BindingTarget, params: &[&str], out: &mut Vec<String>) {
    match target {
        BindingTarget::Ident(ident) => {
            if !params.contains(&ident.name.as_str()) {
                out.push(ident.name.clone());
            }
        }
        BindingTarget::Tuple(targets) => {
            for t in targets {
                collect_target_bindings(t, params, out);
            }
        }
        BindingTarget::Array(elems) => {
            for elem in elems {
                collect_target_bindings(&elem.target, params, out);
            }
        }
        BindingTarget::Map(fields) => {
            for f in fields {
                if let Some(alias) = &f.alias {
                    collect_target_bindings(alias, params, out);
                } else if !params.contains(&f.field.name.as_str()) {
                    out.push(f.field.name.clone());
                }
            }
        }
    }
}

fn rename_macro_body(body: &mut MacroBody, rename_map: &IndexMap<String, String>) {
    match body {
        MacroBody::Block(block) => rename_block(block, rename_map),
        MacroBody::Expr(expr) => rename_expr(expr, rename_map),
    }
}

fn rename_block(block: &mut Block, rename_map: &IndexMap<String, String>) {
    for stmt in &mut block.stmts {
        rename_stmt(stmt, rename_map);
    }
}

fn rename_stmt(stmt: &mut Stmt, rename_map: &IndexMap<String, String>) {
    match &mut stmt.kind {
        StmtKind::Set { target, init, .. } => {
            rename_binding_target(target, rename_map);
            rename_expr(init, rename_map);
        }
        StmtKind::Static(decl) => {
            if let Some(new_name) = rename_map.get(&decl.name.name) {
                decl.name.name = new_name.clone();
            }
            rename_expr(&mut decl.init, rename_map);
        }
        StmtKind::Rewrite { target, value, .. } => {
            rename_lvalue(target, rename_map);
            rename_expr(value, rename_map);
        }
        StmtKind::Expr(expr) | StmtKind::Return(Some(expr)) => rename_expr(expr, rename_map),
        StmtKind::Recur(exprs) => {
            for e in exprs { rename_expr(e, rename_map); }
        }
        StmtKind::Defer(block) => rename_block(block, rename_map),
        _ => {}
    }
}

fn rename_binding_target(target: &mut BindingTarget, rename_map: &IndexMap<String, String>) {
    match target {
        BindingTarget::Ident(ident) => {
            if let Some(new_name) = rename_map.get(&ident.name) {
                ident.name = new_name.clone();
            }
        }
        BindingTarget::Tuple(targets) => {
            for t in targets {
                rename_binding_target(t, rename_map);
            }
        }
        BindingTarget::Array(elems) => {
            for elem in elems {
                rename_binding_target(&mut elem.target, rename_map);
            }
        }
        BindingTarget::Map(fields) => {
            for f in fields {
                if let Some(alias) = &mut f.alias {
                    rename_binding_target(alias, rename_map);
                } else if let Some(new_name) = rename_map.get(&f.field.name) {
                    f.field.name = new_name.clone();
                }
            }
        }
    }
}

fn rename_lvalue(lvalue: &mut LValue, rename_map: &IndexMap<String, String>) {
    match lvalue {
        LValue::Ident(ident) => {
            if let Some(new_name) = rename_map.get(&ident.name) {
                ident.name = new_name.clone();
            }
        }
        LValue::Field(expr, _) => rename_expr(expr, rename_map),
        LValue::Index(expr, idx) => {
            rename_expr(expr, rename_map);
            rename_expr(idx, rename_map);
        }
    }
}

fn rename_expr(expr: &mut Expr, rename_map: &IndexMap<String, String>) {
    match &mut expr.kind {
        ExprKind::Ident(ident) => {
            if let Some(new_name) = rename_map.get(&ident.name) {
                ident.name = new_name.clone();
            }
        }
        ExprKind::Unary(_, inner) => rename_expr(inner, rename_map),
        ExprKind::Binary(_, lhs, rhs) => {
            rename_expr(lhs, rename_map);
            rename_expr(rhs, rename_map);
        }
        ExprKind::Ternary(cond, then_e, else_e) => {
            rename_expr(cond, rename_map);
            rename_expr(then_e, rename_map);
            rename_expr(else_e, rename_map);
        }
        ExprKind::Call(callee, args) | ExprKind::ClassInit(callee, args) => {
            rename_expr(callee, rename_map);
            for arg in args {
                rename_expr(arg, rename_map);
            }
        }
        ExprKind::MethodCall(recv, _, args) => {
            rename_expr(recv, rename_map);
            for arg in args {
                rename_expr(arg, rename_map);
            }
        }
        ExprKind::Block(block) | ExprKind::RegionBlock(block) => {
            rename_block(block, rename_map);
        }
        ExprKind::ArrayInit(items) => {
            for item in items {
                rename_expr(item, rename_map);
            }
        }
        ExprKind::InterpolatedString(parts) => {
            for part in parts {
                if let StringPart::Expr(e) = part {
                    rename_expr(e, rename_map);
                }
            }
        }
        ExprKind::For {
            index_var,
            item_var,
            iter,
            body,
            ..
        } => {
            if let Some(new_name) = rename_map.get(&item_var.name) {
                item_var.name = new_name.clone();
            }
            if let Some(idx) = index_var
                && let Some(new_name) = rename_map.get(&idx.name)
            {
                idx.name = new_name.clone();
            }
            rename_expr(iter, rename_map);
            rename_block(body, rename_map);
        }
        ExprKind::Ref { expr, .. } | ExprKind::NamedArg(_, expr) => {
            rename_expr(expr, rename_map);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_span::{BytePos, FileId, Span};

    fn dummy_span() -> Span {
        Span::new(FileId(0), BytePos(0), BytePos(10))
    }

    #[test]
    fn test_gensym_generates_unique_names() {
        let s1 = gensym("tmp");
        let s2 = gensym("tmp");
        assert_ne!(s1, s2);
        assert!(s1.starts_with("tmp_g"));
    }

    #[test]
    fn test_hygiene_renames_internal_variable() {
        let block = Block::new(
            vec![
                Stmt::new(
                    StmtKind::Set {
                        target: BindingTarget::Ident(Ident::new("tmp", dummy_span())),
                        ty: None,
                        init: Expr::new(ExprKind::Literal(LiteralValue::Int(100)), dummy_span()),
                    },
                    dummy_span(),
                ),
                Stmt::new(
                    StmtKind::Expr(Expr::new(
                        ExprKind::Ident(Ident::new("tmp", dummy_span())),
                        dummy_span(),
                    )),
                    dummy_span(),
                ),
            ],
            dummy_span(),
        );

        let mut body = MacroBody::Block(block);
        let param = MacroParam {
            name: Ident::new("action", dummy_span()),
            kind: MacroParamKind::Expr,
            span: dummy_span(),
        };

        apply_hygiene(&mut body, &[param]);

        if let MacroBody::Block(b) = body {
            if let StmtKind::Set { target, .. } = &b.stmts[0].kind {
                if let BindingTarget::Ident(id) = target {
                    assert_ne!(id.name, "tmp");
                    assert!(id.name.starts_with("tmp_g"));
                } else {
                    panic!("expected ident target");
                }
            }
            if let StmtKind::Expr(e) = &b.stmts[1].kind {
                if let ExprKind::Ident(id) = &e.kind {
                    assert_ne!(id.name, "tmp");
                    assert!(id.name.starts_with("tmp_g"));
                } else {
                    panic!("expected ident expr");
                }
            }
        }
    }
}
