#![deny(unsafe_code)]

use std::collections::HashMap;

use crate::expr::{Expr, ExprKind};
use crate::item::{ItemKind, Module, Param};
use crate::stmt::{Block, Stmt, StmtKind};

use yukic_span::Span;

#[derive(Clone, Debug, PartialEq)]
pub struct ParamInfo {
    pub name: String,
    pub default_val: Option<Expr>,
    pub is_variadic: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DefaultEntry {
    pub params: Vec<ParamInfo>,
    pub is_free_fn: bool,
}

pub type DefaultArgsMap = HashMap<String, DefaultEntry>;

fn register_param_defaults(
    name: &str,
    params: &[Param],
    prefix: Option<&str>,
    is_free_fn: bool,
    out: &mut DefaultArgsMap,
) {
    if params.is_empty() { return; }
    let param_infos: Vec<ParamInfo> = params
        .iter()
        .map(|p| ParamInfo {
            name: p.name.name.clone(),
            default_val: p.default_val.clone(),
            is_variadic: p.is_variadic,
        })
        .collect();
    let entry = DefaultEntry {
        params: param_infos,
        is_free_fn,
    };
    out.insert(name.to_string(), entry.clone());
    if let Some(p) = prefix {
        out.insert(format!("{p}::{name}"), entry);
    }
}

pub fn collect_module_defaults(module: &Module, prefix: Option<&str>, out: &mut DefaultArgsMap) {
    for item in &module.items {
        match &item.kind {
            ItemKind::Fun(decl) => register_param_defaults(&decl.name.name, &decl.params, prefix, true, out),
            ItemKind::Class(decl) => {
                for m in &decl.methods {
                    // methods register only under the qualified name:
                    // bare names collide across classes and leak defaults
                    register_param_defaults(
                        &format!("{}::{}", decl.name.name, m.name.name),
                        &m.params,
                        prefix,
                        false,
                        out,
                    );
                    if m.name.name == "init" {
                        register_param_defaults(&decl.name.name, &m.params, prefix, false, out);
                        if let Some(p) = prefix {
                            register_param_defaults(&format!("{p}::{}", decl.name.name), &m.params, prefix, false, out);
                        }
                    }
                }
            }
            ItemKind::Struct(decl) => {
                for m in &decl.methods {
                    register_param_defaults(
                        &format!("{}::{}", decl.name.name, m.name.name),
                        &m.params,
                        prefix,
                        false,
                        out,
                    );
                    if m.name.name == "init" {
                        register_param_defaults(&decl.name.name, &m.params, prefix, false, out);
                        if let Some(p) = prefix {
                            register_param_defaults(&format!("{p}::{}", decl.name.name), &m.params, prefix, false, out);
                        }
                    }
                }
                for sm in &decl.static_methods {
                    register_param_defaults(
                        &format!("{}::{}", decl.name.name, sm.name.name),
                        &sm.params,
                        prefix,
                        false,
                        out,
                    );
                }
            }
            _ => {}
        }
    }
}

pub fn expand_default_args(target_module: &mut Module, source_modules: &[Module]) {
    let mut map = DefaultArgsMap::new();
    for src in source_modules {
        collect_module_defaults(src, None, &mut map);
    }
    collect_module_defaults(target_module, None, &mut map);

    for item in &mut target_module.items {
        match &mut item.kind {
            ItemKind::Fun(decl) => expand_block_defaults(&mut decl.body, &map),
            ItemKind::Class(decl) => {
                for m in &mut decl.methods {
                    expand_block_defaults(&mut m.body, &map);
                }
            }
            ItemKind::Struct(decl) => {
                for m in &mut decl.methods {
                    expand_block_defaults(&mut m.body, &map);
                }
                for sm in &mut decl.static_methods {
                    expand_block_defaults(&mut sm.body, &map);
                }
            }
            _ => {}
        }
    }
}

pub fn expand_block_defaults(block: &mut Block, map: &DefaultArgsMap) {
    for stmt in &mut block.stmts {
        expand_stmt_defaults(stmt, map);
    }
}

fn expand_stmt_defaults(stmt: &mut Stmt, map: &DefaultArgsMap) {
    match &mut stmt.kind {
        StmtKind::Expr(e) => expand_expr_defaults(e, map),
        StmtKind::Set { init, .. } => expand_expr_defaults(init, map),
        StmtKind::Static(decl) => expand_expr_defaults(&mut decl.init, map),
        StmtKind::Rewrite { value, .. } => expand_expr_defaults(value, map),
        StmtKind::Return(Some(e)) => expand_expr_defaults(e, map),
        StmtKind::Recur(args) => {
            for arg in args {
                expand_expr_defaults(arg, map);
            }
        }
        StmtKind::Defer(b) => expand_block_defaults(b, map),
        _ => {}
    }
}

pub fn expand_expr_defaults(expr: &mut Expr, map: &DefaultArgsMap) {
    match &mut expr.kind {
        ExprKind::Unary(_, op) => expand_expr_defaults(op, map),
        ExprKind::Binary(_, left, right) => {
            expand_expr_defaults(left, map);
            expand_expr_defaults(right, map);
        }
        ExprKind::Ternary(cond, then_e, else_e) => {
            expand_expr_defaults(cond, map);
            expand_expr_defaults(then_e, map);
            expand_expr_defaults(else_e, map);
        }
        ExprKind::Call(callee, args) | ExprKind::ClassInit(callee, args) => {
            expand_expr_defaults(callee, map);
            for arg in args.iter_mut() {
                expand_expr_defaults(arg, map);
            }
            let callee_name = match &callee.kind {
                ExprKind::Ident(id) => Some(id.name.clone()),
                ExprKind::Path(idents) => idents.last().map(|id| id.name.clone()),
                _ => None,
            };
            if let Some(name) = callee_name {
                fill_missing_arguments(args, &name, map, false);
            }
        }
        ExprKind::MethodCall(target, _method, args) => {
            // method defaults are filled in codegen where the receiver class
            // is known; bare-name lookup here leaks defaults across classes
            expand_expr_defaults(target, map);
            for arg in args.iter_mut() {
                expand_expr_defaults(arg, map);
            }
        }
        ExprKind::Block(block) | ExprKind::RegionBlock(block) => {
            expand_block_defaults(block, map);
        }
        ExprKind::If {
            cond,
            then_branch,
            elif_branches,
            else_branch,
        } => {
            expand_expr_defaults(cond, map);
            expand_block_defaults(then_branch, map);
            for (elif_c, elif_b) in elif_branches {
                expand_expr_defaults(elif_c, map);
                expand_block_defaults(elif_b, map);
            }
            if let Some(eb) = else_branch {
                expand_block_defaults(eb, map);
            }
        }
        ExprKind::While { cond, body, .. } => {
            expand_expr_defaults(cond, map);
            expand_block_defaults(body, map);
        }
        ExprKind::Loop { body, .. } => expand_block_defaults(body, map),
        ExprKind::For { iter, body, .. } => {
            expand_expr_defaults(iter, map);
            expand_block_defaults(body, map);
        }
        ExprKind::Switch { target, arms } => {
            expand_expr_defaults(target, map);
            for arm in arms {
                expand_block_defaults(&mut arm.body, map);
            }
        }
        ExprKind::ArrayInit(elems) | ExprKind::TupleInit(elems) => {
            for elem in elems {
                expand_expr_defaults(elem, map);
            }
        }
        ExprKind::StructInit(_, fields) => {
            for field in fields {
                expand_expr_defaults(&mut field.value, map);
            }
        }
        ExprKind::IndexAccess(target, idx) => {
            expand_expr_defaults(target, map);
            expand_expr_defaults(idx, map);
        }
        ExprKind::FieldAccess(target, _) => expand_expr_defaults(target, map),
        ExprKind::NamedArg(_, val) => expand_expr_defaults(val, map),
        _ => {}
    }
}

fn fill_missing_arguments(
    args: &mut Vec<Expr>,
    name: &str,
    map: &DefaultArgsMap,
    is_method_call: bool,
) {
    let Some(entry) = map.get(name) else {
        return;
    };
    let params: &[ParamInfo] = if is_method_call && entry.is_free_fn {
        if entry.params.is_empty() {
            &[]
        } else {
            &entry.params[1..]
        }
    } else {
        &entry.params[..]
    };

    let has_named = args.iter().any(|a| matches!(a.kind, ExprKind::NamedArg(_, _)));
    if has_named {
        let mut pos_args = Vec::new();
        let mut named_args: Vec<(String, Expr)> = Vec::new();
        for arg in args.drain(..) {
            if let ExprKind::NamedArg(id, val) = arg.kind {
                named_args.push((id.name, *val));
            } else {
                pos_args.push(arg);
            }
        }

        let mut slots: Vec<Option<Expr>> = vec![None; params.len()];
        for (i, arg) in pos_args.into_iter().enumerate() {
            if i < params.len() {
                slots[i] = Some(arg);
            }
        }

        for (pname, val) in named_args {
            if let Some(idx) = params.iter().position(|p| p.name == pname) {
                if slots[idx].is_none() {
                    slots[idx] = Some(val);
                }
            }
        }

        let empty_span = Span::new(yukic_span::FileId(0), yukic_span::BytePos(0), yukic_span::BytePos(0));
        for (i, p) in params.iter().enumerate() {
            if slots[i].is_none() {
                if let Some(def_val) = &p.default_val {
                    slots[i] = Some(def_val.clone());
                } else if p.is_variadic {
                    slots[i] = Some(Expr::new(ExprKind::ArrayInit(vec![]), empty_span));
                }
            } else if p.is_variadic {
                if let Some(ref mut expr) = slots[i] {
                    if !matches!(expr.kind, ExprKind::ArrayInit(_) | ExprKind::Spread(_)) {
                        let inner = expr.clone();
                        let span = inner.span;
                        *expr = Expr::new(ExprKind::ArrayInit(vec![inner]), span);
                    }
                }
            }
        }

        for slot in slots {
            if let Some(expr) = slot {
                args.push(expr);
            }
        }
        return;
    }

    let has_variadic = params.last().map_or(false, |p| p.is_variadic);
    if has_variadic {
        let var_idx = params.len() - 1;
        let empty_span = args.last().map_or_else(
            || Span::new(yukic_span::FileId(0), yukic_span::BytePos(0), yukic_span::BytePos(0)),
            |a| a.span,
        );
        if args.len() < var_idx {
            for p in &params[args.len()..var_idx] {
                if let Some(def_expr) = &p.default_val {
                    args.push(def_expr.clone());
                }
            }
            args.push(Expr::new(ExprKind::ArrayInit(vec![]), empty_span));
        } else if args.len() == var_idx {
            args.push(Expr::new(ExprKind::ArrayInit(vec![]), empty_span));
        } else if args.len() == var_idx + 1 {
            if matches!(args[var_idx].kind, ExprKind::Spread(_)) {
                if let ExprKind::Spread(inner) = args.remove(var_idx).kind {
                    args.push(*inner);
                }
            } else {
                let elem = args.remove(var_idx);
                let span = elem.span;
                args.push(Expr::new(ExprKind::ArrayInit(vec![elem]), span));
            }
        } else {
            let start_span = args[var_idx].span;
            let end_span = args.last().unwrap().span;
            let drained: Vec<Expr> = args.drain(var_idx..).collect();
            let span = Span::new(start_span.file_id, start_span.start, end_span.end);
            args.push(Expr::new(ExprKind::ArrayInit(drained), span));
        }
        return;
    }

    if args.len() >= params.len() {
        return;
    }
    let can_fill = params[args.len()..].iter().all(|p| p.default_val.is_some());
    if !can_fill {
        return;
    }
    for p in &params[args.len()..] {
        if let Some(def_expr) = &p.default_val {
            args.push(def_expr.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::LiteralValue;
    use crate::ident::Ident;
    use crate::item::{FunDecl, Item, ItemId, Visibility};
    use crate::stmt::Stmt;
    use yukic_span::{BytePos, FileId, Span};

    #[test]
    fn test_expand_default_args() {
        let sp = Span::new(FileId(0), BytePos(0), BytePos(0));
        let def_val = Expr::new(ExprKind::Literal(LiteralValue::Int(42)), sp);
        let param1 = Param {
            name: Ident::new("a", sp),
            is_ref: false,
            is_mut: false,
            is_variadic: false,
            ty: None,
            default_val: None,
            span: sp,
        };
        let param2 = Param {
            name: Ident::new("b", sp),
            is_ref: false,
            is_mut: false,
            is_variadic: false,
            ty: None,
            default_val: Some(def_val.clone()),
            span: sp,
        };

        let fun_decl = FunDecl {
            name: Ident::new("add", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: vec![param1, param2],
            return_type: None,
            body: Block::new(Vec::new(), sp),
            span: sp,
        };

        let call_expr = Expr::new(
            ExprKind::Call(
                Box::new(Expr::new(ExprKind::Ident(Ident::new("add", sp)), sp)),
                vec![Expr::new(ExprKind::Literal(LiteralValue::Int(10)), sp)],
            ),
            sp,
        );

        let main_decl = FunDecl {
            name: Ident::new("main", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: Vec::new(),
            return_type: None,
            body: Block::new(vec![Stmt::new(StmtKind::Expr(call_expr), sp)], sp),
            span: sp,
        };

        let mut module = Module::new(FileId(0));
        module.items.push(Item::new(ItemId(0), Visibility::Public, ItemKind::Fun(fun_decl), sp));
        module.items.push(Item::new(ItemId(1), Visibility::Public, ItemKind::Fun(main_decl), sp));

        expand_default_args(&mut module, &[]);

        if let ItemKind::Fun(ref main_fn) = module.items[1].kind {
            if let StmtKind::Expr(ref expr) = main_fn.body.stmts[0].kind {
                if let ExprKind::Call(_, ref args) = expr.kind {
                    assert_eq!(args.len(), 2);
                    assert_eq!(args[1], def_val);
                } else {
                    panic!("expected Call");
                }
            } else {
                panic!("expected Expr stmt");
            }
        } else {
            panic!("expected Fun item");
        }
    }

    #[test]
    fn test_expand_variadic_args() {
        let sp = Span::new(FileId(0), BytePos(0), BytePos(0));
        let param_prefix = Param {
            name: Ident::new("prefix", sp),
            is_ref: false,
            is_mut: false,
            is_variadic: false,
            ty: None,
            default_val: None,
            span: sp,
        };
        let param_var = Param {
            name: Ident::new("nums", sp),
            is_ref: false,
            is_mut: false,
            is_variadic: true,
            ty: None,
            default_val: None,
            span: sp,
        };

        let fun_decl = FunDecl {
            name: Ident::new("log_all", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: vec![param_prefix, param_var],
            return_type: None,
            body: Block::new(Vec::new(), sp),
            span: sp,
        };

        let call_expr = Expr::new(
            ExprKind::Call(
                Box::new(Expr::new(ExprKind::Ident(Ident::new("log_all", sp)), sp)),
                vec![
                    Expr::new(ExprKind::Literal(LiteralValue::String("msg".into())), sp),
                    Expr::new(ExprKind::Literal(LiteralValue::Int(1)), sp),
                    Expr::new(ExprKind::Literal(LiteralValue::Int(2)), sp),
                ],
            ),
            sp,
        );

        let call_empty = Expr::new(
            ExprKind::Call(
                Box::new(Expr::new(ExprKind::Ident(Ident::new("log_all", sp)), sp)),
                vec![Expr::new(ExprKind::Literal(LiteralValue::String("msg".into())), sp)],
            ),
            sp,
        );

        let main_decl = FunDecl {
            name: Ident::new("main", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: Vec::new(),
            return_type: None,
            body: Block::new(
                vec![
                    Stmt::new(StmtKind::Expr(call_expr), sp),
                    Stmt::new(StmtKind::Expr(call_empty), sp),
                ],
                sp,
            ),
            span: sp,
        };

        let mut module = Module::new(FileId(0));
        module.items.push(Item::new(ItemId(0), Visibility::Public, ItemKind::Fun(fun_decl), sp));
        module.items.push(Item::new(ItemId(1), Visibility::Public, ItemKind::Fun(main_decl), sp));

        expand_default_args(&mut module, &[]);

        if let ItemKind::Fun(ref main_fn) = module.items[1].kind {
            if let StmtKind::Expr(ref expr) = main_fn.body.stmts[0].kind {
                if let ExprKind::Call(_, ref args) = expr.kind {
                    assert_eq!(args.len(), 2);
                    assert!(matches!(args[1].kind, ExprKind::ArrayInit(ref elems) if elems.len() == 2));
                } else {
                    panic!("expected Call");
                }
            }
            if let StmtKind::Expr(ref expr) = main_fn.body.stmts[1].kind {
                if let ExprKind::Call(_, ref args) = expr.kind {
                    assert_eq!(args.len(), 2);
                    assert!(matches!(args[1].kind, ExprKind::ArrayInit(ref elems) if elems.is_empty()));
                } else {
                    panic!("expected Call");
                }
            }
        }
    }

    #[test]
    fn test_named_args_expansion() {
        let sp = Span::new(FileId(0), BytePos(0), BytePos(0));
        let param_a = Param {
            name: Ident::new("a", sp),
            is_ref: false,
            is_mut: false,
            is_variadic: false,
            ty: None,
            default_val: None,
            span: sp,
        };
        let param_b = Param {
            name: Ident::new("b", sp),
            is_ref: false,
            is_mut: false,
            is_variadic: false,
            ty: None,
            default_val: Some(Expr::new(ExprKind::Literal(LiteralValue::Int(20)), sp)),
            span: sp,
        };
        let param_c = Param {
            name: Ident::new("c", sp),
            is_ref: false,
            is_mut: false,
            is_variadic: false,
            ty: None,
            default_val: Some(Expr::new(ExprKind::Literal(LiteralValue::Int(30)), sp)),
            span: sp,
        };

        let fun_decl = FunDecl {
            name: Ident::new("calc", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: vec![param_a, param_b, param_c],
            return_type: None,
            body: Block::new(Vec::new(), sp),
            span: sp,
        };

        let call_named = Expr::new(
            ExprKind::Call(
                Box::new(Expr::new(ExprKind::Ident(Ident::new("calc", sp)), sp)),
                vec![
                    Expr::new(
                        ExprKind::NamedArg(
                            Ident::new("c", sp),
                            Box::new(Expr::new(ExprKind::Literal(LiteralValue::Int(99)), sp)),
                        ),
                        sp,
                    ),
                    Expr::new(
                        ExprKind::NamedArg(
                            Ident::new("a", sp),
                            Box::new(Expr::new(ExprKind::Literal(LiteralValue::Int(11)), sp)),
                        ),
                        sp,
                    ),
                ],
            ),
            sp,
        );

        let main_decl = FunDecl {
            name: Ident::new("main", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: Vec::new(),
            return_type: None,
            body: Block::new(vec![Stmt::new(StmtKind::Expr(call_named), sp)], sp),
            span: sp,
        };

        let mut module = Module::new(FileId(0));
        module.items.push(Item::new(ItemId(0), Visibility::Public, ItemKind::Fun(fun_decl), sp));
        module.items.push(Item::new(ItemId(1), Visibility::Public, ItemKind::Fun(main_decl), sp));

        expand_default_args(&mut module, &[]);

        let ItemKind::Fun(ref main_fn) = module.items[1].kind else { panic!("expected Fun"); };
        let StmtKind::Expr(ref expr) = main_fn.body.stmts[0].kind else { panic!("expected StmtExpr"); };
        let ExprKind::Call(_, ref args) = expr.kind else { panic!("expected Call"); };
        assert_eq!(args.len(), 3);
        assert_eq!(args[0].kind, ExprKind::Literal(LiteralValue::Int(11)));
        assert_eq!(args[1].kind, ExprKind::Literal(LiteralValue::Int(20)));
        assert_eq!(args[2].kind, ExprKind::Literal(LiteralValue::Int(99)));
    }
}

