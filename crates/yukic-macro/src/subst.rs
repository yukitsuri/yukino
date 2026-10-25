#![deny(unsafe_code)]
#![allow(
    clippy::result_large_err,
    clippy::struct_field_names,
    clippy::wildcard_imports
)]

use indexmap::IndexMap;
use yukic_ast::*;
use yukic_diagnostics::Diagnostic;
use yukic_diagnostics::code::ErrorCode;
use yukic_span::Span;

pub fn substitute_macro(
    decl: &MacroDefDecl,
    args: &[MacroArg],
    call_span: Span,
) -> Result<MacroBody, Diagnostic> {
    let bindings = bind_arguments(decl, args, call_span)?;
    let mut expanded_body = decl.body.clone();

    substitute_in_body(&mut expanded_body, &bindings);
    Ok(expanded_body)
}

struct MacroBindings {
    expr_map: IndexMap<String, Expr>,
    block_map: IndexMap<String, Block>,
    variadic_map: IndexMap<String, Vec<Expr>>,
}

fn bind_arguments(
    decl: &MacroDefDecl,
    args: &[MacroArg],
    call_span: Span,
) -> Result<MacroBindings, Diagnostic> {
    let mut expr_map = IndexMap::new();
    let mut block_map = IndexMap::new();
    let mut variadic_map = IndexMap::new();

    let has_variadic = decl
        .params
        .last()
        .is_some_and(|p| p.kind == MacroParamKind::Variadic);
    let fixed_count = if has_variadic {
        decl.params.len() - 1
    } 

    else { decl.params.len() };

    if !has_variadic && args.len() != fixed_count {
        return Err(Diagnostic::error(
            ErrorCode::E0306,
            format!(
                "macro `{}` expects {} argument(s), found {}",
                decl.name.name,
                fixed_count,
                args.len()
            ),
            call_span,
        ));
    }

    if has_variadic && args.len() < fixed_count {
        return Err(Diagnostic::error(
            ErrorCode::E0306,
            format!(
                "macro `{}` expects at least {} argument(s), found {}",
                decl.name.name,
                fixed_count,
                args.len()
            ),
            call_span,
        ));
    }

    for (i, param) in decl.params.iter().enumerate().take(fixed_count) {
        match &args[i] {
            MacroArg::Expr(e) => {
                expr_map.insert(param.name.name.clone(), e.clone());
            }
            MacroArg::Block(b) => {
                block_map.insert(param.name.name.clone(), b.clone());
            }
            MacroArg::Tokens(_) => {}
        }
    }

    if has_variadic {
        let var_param = &decl.params[fixed_count];
        let mut var_exprs = Vec::new();
        for arg in &args[fixed_count..] {
            match arg {
                MacroArg::Expr(e) => var_exprs.push(e.clone()),
                MacroArg::Block(b) => {
                    var_exprs.push(Expr::new(ExprKind::Block(b.clone()), b.span));
                }
                MacroArg::Tokens(_) => {}
            }
        }
        variadic_map.insert(var_param.name.name.clone(), var_exprs);
    }

    Ok(MacroBindings {
        expr_map,
        block_map,
        variadic_map,
    })
}

fn substitute_in_body(body: &mut MacroBody, bindings: &MacroBindings) {
    match body {
        MacroBody::Block(block) => substitute_in_block(block, bindings),
        MacroBody::Expr(expr) => substitute_in_expr(expr, bindings),
    }
}

fn substitute_in_block(block: &mut Block, bindings: &MacroBindings) {
    let mut new_stmts = Vec::new();

    for stmt in &block.stmts {
        match &stmt.kind {
            StmtKind::Expr(Expr {
                kind: ExprKind::Ident(ident),
                ..
            }) if bindings.block_map.contains_key(&ident.name) => {
                let blk = &bindings.block_map[&ident.name];
                new_stmts.extend(blk.stmts.clone());
            }
            _ => {
                let mut cloned = stmt.clone();
                substitute_in_stmt(&mut cloned, bindings);
                new_stmts.push(cloned);
            }
        }
    }

    block.stmts = new_stmts;
}

fn substitute_in_stmt(stmt: &mut Stmt, bindings: &MacroBindings) {
    match &mut stmt.kind {
        StmtKind::Set { init, .. } => substitute_in_expr(init, bindings),
        StmtKind::Static(decl) => substitute_in_expr(&mut decl.init, bindings),
        StmtKind::Rewrite { value, .. } => substitute_in_expr(value, bindings),
        StmtKind::Expr(expr) | StmtKind::Return(Some(expr)) => substitute_in_expr(expr, bindings),
        StmtKind::Recur(exprs) => {
            for e in exprs {
                substitute_in_expr(e, bindings);
            }
        }
        StmtKind::Defer(block) => substitute_in_block(block, bindings),
        _ => {}
    }
}

fn substitute_in_expr(expr: &mut Expr, bindings: &MacroBindings) {
    match &mut expr.kind {
        ExprKind::Ident(ident) => {
            if let Some(replacement) = bindings.expr_map.get(&ident.name) {
                *expr = replacement.clone();
            } else if let Some(blk) = bindings.block_map.get(&ident.name) {
                *expr = Expr::new(ExprKind::Block(blk.clone()), blk.span);
            }
        }
        ExprKind::Unary(_, inner) => substitute_in_expr(inner, bindings),
        ExprKind::Binary(_, lhs, rhs) => {
            substitute_in_expr(lhs, bindings);
            substitute_in_expr(rhs, bindings);
        }
        ExprKind::Ternary(cond, then_e, else_e) => {
            substitute_in_expr(cond, bindings);
            substitute_in_expr(then_e, bindings);
            substitute_in_expr(else_e, bindings);
        }
        ExprKind::Call(callee, args) | ExprKind::ClassInit(callee, args) => {
            substitute_in_expr(callee, bindings);
            for arg in args {
                substitute_in_expr(arg, bindings);
            }
        }
        ExprKind::MethodCall(recv, _, args) => {
            substitute_in_expr(recv, bindings);
            for arg in args {
                substitute_in_expr(arg, bindings);
            }
        }
        ExprKind::Block(block) | ExprKind::RegionBlock(block) => {
            substitute_in_block(block, bindings);
        }
        ExprKind::ArrayInit(items) => {
            let mut new_items = Vec::new();
            for item in items.iter() {
                if let ExprKind::Spread(inner) = &item.kind
                    && let ExprKind::Ident(id) = &inner.kind
                    && let Some(var_items) = bindings.variadic_map.get(&id.name)
                {
                    new_items.extend(var_items.clone());
                } else {
                    let mut cloned = item.clone();
                    substitute_in_expr(&mut cloned, bindings);
                    new_items.push(cloned);
                }
            }
            *items = new_items;
        }
        ExprKind::InterpolatedString(parts) => {
            for part in parts {
                if let StringPart::Expr(e) = part {
                    substitute_in_expr(e, bindings);
                }
            }
        }
        ExprKind::If {
            cond,
            then_branch,
            elif_branches,
            else_branch,
        } => {
            substitute_in_expr(cond, bindings);
            substitute_in_block(then_branch, bindings);
            for (elif_cond, elif_body) in elif_branches {
                substitute_in_expr(elif_cond, bindings);
                substitute_in_block(elif_body, bindings);
            }
            if let Some(else_b) = else_branch {
                substitute_in_block(else_b, bindings);
            }
        }
        ExprKind::For { iter, body, .. } => {
            substitute_in_expr(iter, bindings);
            substitute_in_block(body, bindings);
        }
        ExprKind::Ref { expr, .. } | ExprKind::NamedArg(_, expr) => {
            substitute_in_expr(expr, bindings);
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
    fn test_substitute_simple_expr() {
        let decl = MacroDefDecl {
            name: Ident::new("square", dummy_span()),
            params: vec![MacroParam {
                name: Ident::new("x", dummy_span()),
                kind: MacroParamKind::Expr,
                span: dummy_span(),
            }],
            body: MacroBody::Expr(Box::new(Expr::new(
                ExprKind::Binary(
                    BinaryOp::Mul,
                    Box::new(Expr::new(
                        ExprKind::Ident(Ident::new("x", dummy_span())),
                        dummy_span(),
                    )),
                    Box::new(Expr::new(
                        ExprKind::Ident(Ident::new("x", dummy_span())),
                        dummy_span(),
                    )),
                ),
                dummy_span(),
            ))),
            span: dummy_span(),
        };

        let arg = MacroArg::Expr(Expr::new(
            ExprKind::Literal(LiteralValue::Int(5)),
            dummy_span(),
        ));
        let expanded = substitute_macro(&decl, &[arg], dummy_span()).unwrap();

        if let MacroBody::Expr(e) = expanded {
            if let ExprKind::Binary(BinaryOp::Mul, lhs, rhs) = e.kind {
                assert_eq!(lhs.kind, ExprKind::Literal(LiteralValue::Int(5)));
                assert_eq!(rhs.kind, ExprKind::Literal(LiteralValue::Int(5)));
            } else {
                panic!("expected binary mul");
            }
        }
    }

    #[test]
    fn test_substitute_variadic_unrolling() {
        let decl = MacroDefDecl {
            name: Ident::new("make_vec", dummy_span()),
            params: vec![MacroParam {
                name: Ident::new("items", dummy_span()),
                kind: MacroParamKind::Variadic,
                span: dummy_span(),
            }],
            body: MacroBody::Expr(Box::new(Expr::new(
                ExprKind::ArrayInit(vec![Expr::new(
                    ExprKind::Spread(Box::new(Expr::new(
                        ExprKind::Ident(Ident::new("items", dummy_span())),
                        dummy_span(),
                    ))),
                    dummy_span(),
                )]),
                dummy_span(),
            ))),
            span: dummy_span(),
        };

        let args = vec![
            MacroArg::Expr(Expr::new(
                ExprKind::Literal(LiteralValue::Int(1)),
                dummy_span(),
            )),
            MacroArg::Expr(Expr::new(
                ExprKind::Literal(LiteralValue::Int(2)),
                dummy_span(),
            )),
            MacroArg::Expr(Expr::new(
                ExprKind::Literal(LiteralValue::Int(3)),
                dummy_span(),
            )),
        ];

        let expanded = substitute_macro(&decl, &args, dummy_span()).unwrap();
        if let MacroBody::Expr(e) = expanded {
            if let ExprKind::ArrayInit(items) = e.kind {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0].kind, ExprKind::Literal(LiteralValue::Int(1)));
                assert_eq!(items[1].kind, ExprKind::Literal(LiteralValue::Int(2)));
                assert_eq!(items[2].kind, ExprKind::Literal(LiteralValue::Int(3)));
            } else {
                panic!("expected array init");
            }
        }
    }
}
