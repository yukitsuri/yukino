#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet};

use crate::expr::{Expr, ExprKind};
use crate::item::{ItemKind, Module};
use crate::stmt::{BindingTarget, Block, Stmt, StmtKind};

pub type AliasMap = HashMap<String, String>;

pub fn substitute_module_aliases(module: &mut Module, aliases: &AliasMap) {
    if aliases.is_empty() { return; }
    for item in &mut module.items {
        match &mut item.kind {
            ItemKind::Fun(decl) => {
                let mut shadowed: HashSet<String> =
                    decl.params.iter().map(|p| p.name.name.clone()).collect();
                substitute_block_aliases(&mut decl.body, aliases, &mut shadowed);
            }
            ItemKind::Class(decl) => {
                for m in &mut decl.methods {
                    let mut shadowed: HashSet<String> =
                        m.params.iter().map(|p| p.name.name.clone()).collect();
                    substitute_block_aliases(&mut m.body, aliases, &mut shadowed);
                }
            }
            ItemKind::Struct(decl) => {
                for m in &mut decl.methods {
                    let mut shadowed: HashSet<String> =
                        m.params.iter().map(|p| p.name.name.clone()).collect();
                    substitute_block_aliases(&mut m.body, aliases, &mut shadowed);
                }
                for sm in &mut decl.static_methods {
                    let mut shadowed: HashSet<String> =
                        sm.params.iter().map(|p| p.name.name.clone()).collect();
                    substitute_block_aliases(&mut sm.body, aliases, &mut shadowed);
                }
            }
            ItemKind::Static(decl) => {
                let mut shadowed = HashSet::new();
                substitute_expr_aliases(&mut decl.init, aliases, &mut shadowed);
            }
            _ => {}
        }
    }
}

pub fn substitute_block_aliases(
    block: &mut Block,
    aliases: &AliasMap,
    shadowed: &mut HashSet<String>,
) {
    let saved = shadowed.clone();
    for stmt in &mut block.stmts { substitute_stmt_aliases(stmt, aliases, shadowed); }
    *shadowed = saved;
}

fn collect_target_bound_idents(target: &BindingTarget, out: &mut HashSet<String>) {
    match target {
        BindingTarget::Ident(id) => { out.insert(id.name.clone()); }
        BindingTarget::Tuple(targets) => {
            for t in targets {
                collect_target_bound_idents(t, out);
            }
        }
        BindingTarget::Array(elements) => {
            for el in elements {
                collect_target_bound_idents(&el.target, out);
            }
        }
        BindingTarget::Map(fields) => {
            for f in fields {
                if let Some(target) = &f.alias { collect_target_bound_idents(target, out); } 
                else { out.insert(f.field.name.clone()); }
            }
        }
    }
}

fn substitute_stmt_aliases(
    stmt: &mut Stmt,
    aliases: &AliasMap,
    shadowed: &mut HashSet<String>,
) {
    match &mut stmt.kind {
        StmtKind::Expr(e) => substitute_expr_aliases(e, aliases, shadowed),
        StmtKind::Set { target, init, .. } => {
            substitute_expr_aliases(init, aliases, shadowed);
            collect_target_bound_idents(target, shadowed);
        }
        StmtKind::Static(decl) => {
            substitute_expr_aliases(&mut decl.init, aliases, shadowed);
            shadowed.insert(decl.name.name.clone());
        }
        StmtKind::Rewrite { value, .. } => substitute_expr_aliases(value, aliases, shadowed),
        StmtKind::Return(Some(e)) => substitute_expr_aliases(e, aliases, shadowed),
        StmtKind::Recur(args) => {
            for arg in args { substitute_expr_aliases(arg, aliases, shadowed); }
        }
        StmtKind::Defer(b) => substitute_block_aliases(b, aliases, shadowed),
        _ => {}
    }
}

fn substitute_expr_branches(
    cond: &mut Expr,
    then_branch: &mut Block,
    elif_branches: &mut [(Expr, Block)],
    else_branch: &mut Option<Block>,
    aliases: &AliasMap,
    shadowed: &mut HashSet<String>,
) {
    substitute_expr_aliases(cond, aliases, shadowed);
    substitute_block_aliases(then_branch, aliases, shadowed);
    for (elif_c, elif_b) in elif_branches {
        substitute_expr_aliases(elif_c, aliases, shadowed);
        substitute_block_aliases(elif_b, aliases, shadowed);
    }
    if let Some(eb) = else_branch { substitute_block_aliases(eb, aliases, shadowed); }
}

pub fn substitute_expr_aliases(
    expr: &mut Expr,
    aliases: &AliasMap,
    shadowed: &mut HashSet<String>,
) {
    match &mut expr.kind {
        ExprKind::Ident(id) => {
            if !shadowed.contains(&id.name) && let Some(target) = aliases.get(&id.name)
            { id.name = target.clone(); }
        }
        ExprKind::StructInit(name, fields) => {
            if !shadowed.contains(&name.name)
                && let Some(target) = aliases.get(&name.name)
            {
                name.name = target.clone();
            }
            for f in fields {
                substitute_expr_aliases(&mut f.value, aliases, shadowed);
            }
        }
        ExprKind::Unary(_, op) => substitute_expr_aliases(op, aliases, shadowed),
        ExprKind::Binary(_, left, right) => {
            substitute_expr_aliases(left, aliases, shadowed);
            substitute_expr_aliases(right, aliases, shadowed);
        }
        ExprKind::Ternary(cond, then_e, else_e) => {
            substitute_expr_aliases(cond, aliases, shadowed);
            substitute_expr_aliases(then_e, aliases, shadowed);
            substitute_expr_aliases(else_e, aliases, shadowed);
        }
        ExprKind::Call(callee, args) | ExprKind::ClassInit(callee, args) => {
            substitute_expr_aliases(callee, aliases, shadowed);
            for arg in args.iter_mut() {
                substitute_expr_aliases(arg, aliases, shadowed);
            }
        }
        ExprKind::MethodCall(target, _, args) => {
            substitute_expr_aliases(target, aliases, shadowed);
            for arg in args.iter_mut() {
                substitute_expr_aliases(arg, aliases, shadowed);
            }
        }
        ExprKind::Block(block) | ExprKind::RegionBlock(block) => {
            substitute_block_aliases(block, aliases, shadowed);
        }
        ExprKind::If {
            cond,
            then_branch,
            elif_branches,
            else_branch,
        } => substitute_expr_branches(
            cond,
            then_branch,
            elif_branches,
            else_branch,
            aliases,
            shadowed,
        ),
        ExprKind::While { cond, body, .. } => {
            substitute_expr_aliases(cond, aliases, shadowed);
            substitute_block_aliases(body, aliases, shadowed);
        }
        ExprKind::Loop { body, .. } => substitute_block_aliases(body, aliases, shadowed),
        ExprKind::For {
            index_var,
            item_var,
            iter,
            body,
            ..
        } => {
            substitute_expr_aliases(iter, aliases, shadowed);
            let mut for_shadowed = shadowed.clone();
            if let Some(idx) = index_var {
                for_shadowed.insert(idx.name.clone());
            }
            for_shadowed.insert(item_var.name.clone());
            substitute_block_aliases(body, aliases, &mut for_shadowed);
        }
        ExprKind::Switch { target, arms } => {
            substitute_expr_aliases(target, aliases, shadowed);
            for arm in arms {
                substitute_block_aliases(&mut arm.body, aliases, shadowed);
            }
        }
        ExprKind::FieldAccess(target, _) => substitute_expr_aliases(target, aliases, shadowed),
        ExprKind::IndexAccess(target, index) => {
            substitute_expr_aliases(target, aliases, shadowed);
            substitute_expr_aliases(index, aliases, shadowed);
        }
        ExprKind::TupleInit(elems) => {
            for e in elems {
                substitute_expr_aliases(e, aliases, shadowed);
            }
        }
        ExprKind::ArrayInit(elems) => {
            for e in elems {
                substitute_expr_aliases(e, aliases, shadowed);
            }
        }
        ExprKind::MapInit(pairs) => {
            for (k, v) in pairs {
                substitute_expr_aliases(k, aliases, shadowed);
                substitute_expr_aliases(v, aliases, shadowed);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ident::Ident;
    use crate::item::{FunDecl, Item, ItemId, Visibility};
    use yukic_span::Span;

    #[test]
    fn test_substitute_aliases() {
        let mut module = Module::default();
        let fun = FunDecl {
            name: Ident {
                name: "test_fn".into(),
                span: Span::DUMMY,
            },
            attrs: vec![],
            generics: vec![],
            params: vec![],
            return_type: None,
            body: Block {
                stmts: vec![Stmt {
                    kind: StmtKind::Expr(Expr::new(
                        ExprKind::Call(
                            Box::new(Expr::new(
                                ExprKind::Ident(Ident {
                                    name: "prln".into(),
                                    span: Span::DUMMY,
                                }),
                                Span::DUMMY,
                            )),
                            vec![],
                        ),
                        Span::DUMMY,
                    )),
                    span: Span::DUMMY,
                }],
                span: Span::DUMMY,
            },
            span: Span::DUMMY,
        };
        module.items.push(Item::new(
            ItemId(0),
            Visibility::Local,
            ItemKind::Fun(fun),
            Span::DUMMY,
        ));

        let mut aliases = AliasMap::new();
        aliases.insert("prln".to_string(), "println".to_string());
        substitute_module_aliases(&mut module, &aliases);

        if let ItemKind::Fun(f) = &module.items[0].kind {
            if let StmtKind::Expr(e) = &f.body.stmts[0].kind {
                if let ExprKind::Call(callee, _) = &e.kind {
                    if let ExprKind::Ident(id) = &callee.kind {
                        assert_eq!(id.name, "println");
                        return;
                    }
                }
            }
        }
        panic!("unexpected ast structure");
    }

    #[test]
    fn test_substitute_aliases_shadowed() {
        let mut module = Module::default();
        let fun = FunDecl {
            name: Ident {
                name: "test_fn".into(),
                span: Span::DUMMY,
            },
            attrs: vec![],
            generics: vec![],
            params: vec![crate::item::Param {
                name: Ident {
                    name: "prln".into(),
                    span: Span::DUMMY,
                },
                is_ref: false,
                is_mut: false,
                is_variadic: false,
                ty: None,
                default_val: None,
                span: Span::DUMMY,
            }],
            return_type: None,
            body: Block {
                stmts: vec![Stmt {
                    kind: StmtKind::Expr(Expr::new(
                        ExprKind::Call(
                            Box::new(Expr::new(
                                ExprKind::Ident(Ident {
                                    name: "prln".into(),
                                    span: Span::DUMMY,
                                }),
                                Span::DUMMY,
                            )),
                            vec![],
                        ),
                        Span::DUMMY,
                    )),
                    span: Span::DUMMY,
                }],
                span: Span::DUMMY,
            },
            span: Span::DUMMY,
        };
        module.items.push(Item::new(
            ItemId(0),
            Visibility::Local,
            ItemKind::Fun(fun),
            Span::DUMMY,
        ));

        let mut aliases = AliasMap::new();
        aliases.insert("prln".to_string(), "println".to_string());
        substitute_module_aliases(&mut module, &aliases);

        if let ItemKind::Fun(f) = &module.items[0].kind {
            if let StmtKind::Expr(e) = &f.body.stmts[0].kind {
                if let ExprKind::Call(callee, _) = &e.kind {
                    if let ExprKind::Ident(id) = &callee.kind {
                        assert_eq!(id.name, "prln");
                        return;
                    }
                }
            }
        }
        panic!("unexpected ast structure");
    }
}
