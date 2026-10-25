#![deny(unsafe_code)]

use yukic_ast::expr::{BinaryOp, Expr, ExprKind, LiteralValue};
use yukic_ast::item::{FunDecl, MethodDecl};
use yukic_ast::stmt::{Block, StmtKind};

use crate::region::RegionManager;
use crate::registry::TypeRegistry;

pub fn fun_needs_local_region(
    fun: &FunDecl,
    has_dest: bool,
    registry: &TypeRegistry,
) -> bool {
    if fun.attrs.iter().any(|a| a.is("stable")) {
        return false;
    }
    if has_dest {
        return true;
    }
    block_requires_region(&fun.body, registry)
}

pub fn method_needs_local_region(
    method: &MethodDecl,
    has_dest: bool,
    registry: &TypeRegistry,
) -> bool {
    if method.attrs.iter().any(|a| a.is("stable")) {
        return false;
    }
    if has_dest {
        return true;
    }
    block_requires_region(&method.body, registry)
}

pub fn block_requires_region(block: &Block, registry: &TypeRegistry) -> bool {
    for stmt in &block.stmts {
        if stmt_requires_region(&stmt.kind, registry) {
            return true;
        }
    }
    false
}

fn stmt_requires_region(stmt: &StmtKind, registry: &TypeRegistry) -> bool {
    match stmt {
        StmtKind::Defer(_) | StmtKind::MacroCall(_) => true,
        StmtKind::Set { init, .. } => expr_requires_region(init, registry),
        StmtKind::Rewrite { value, .. } => expr_requires_region(value, registry),
        StmtKind::Expr(expr) | StmtKind::Return(Some(expr)) => {
            expr_requires_region(expr, registry)
        }
        StmtKind::Recur(args) => args.iter().any(|arg| expr_requires_region(arg, registry)),
        _ => false,
    }
}

fn is_string_expr(expr: &Expr) -> bool {
    matches!(
        &expr.kind,
        ExprKind::Literal(LiteralValue::String(_)) | ExprKind::InterpolatedString(_)
    )
}

fn is_regional_cast_target(expr: &Expr) -> bool {
    if let ExprKind::Literal(LiteralValue::String(s)) = &expr.kind {
        let ty = yukic_types::convert::parse_primitive_or_named(s);
        RegionManager::is_regional_type(&ty)
    } else {
        false
    }
}

fn call_returns_regional_type(callee: &Expr, args: &[Expr], registry: &TypeRegistry) -> bool {
    let name = match &callee.kind {
        ExprKind::Ident(id) => Some(id.name.as_str()),
        ExprKind::Path(idents) => {
            if idents.len() == 2 && idents[0].name == "rt" {
                Some(idents[1].name.as_str())
            } else {
                idents.last().map(|id| id.name.as_str())
            }
        }
        _ => None,
    };
    if let Some(id_name) = name {
        if id_name == "cast" && args.len() >= 2 && is_regional_cast_target(&args[1]) {
            return true;
        }
        let mangled = registry.resolve_fun_symbol(id_name, args.len());
        let fn_ty = registry
            .functions
            .get(id_name)
            .or_else(|| mangled.as_ref().and_then(|m| registry.functions.get(m)))
            .or_else(|| registry.fn_types.get(id_name))
            .or_else(|| mangled.as_ref().and_then(|m| registry.fn_types.get(m)));
        if let Some(ret_ty) = fn_ty {
            if RegionManager::is_regional_type(ret_ty) {
                return true;
            }
        }
    }
    false
}

fn method_call_returns_regional_type(method_name: &str, registry: &TypeRegistry) -> bool {
    let suffix = format!("::{method_name}");
    for (fn_name, ret_ty) in &registry.functions {
        if (fn_name.ends_with(&suffix) || fn_name.contains(&format!("::{method_name}_")))
            && RegionManager::is_regional_type(ret_ty)
        {
            return true;
        }
    }
    false
}

fn expr_requires_region(expr: &Expr, registry: &TypeRegistry) -> bool {
    match &expr.kind {
        ExprKind::InterpolatedString(_)
        | ExprKind::ArrayInit(_)
        | ExprKind::ArrayAlloc(..)
        | ExprKind::MapInit(_)
        | ExprKind::TupleInit(_)
        | ExprKind::AnonymousFun(_)
        | ExprKind::Lambda(_, _)
        | ExprKind::RegionBlock(_)
        | ExprKind::StructInit(_, _)
        | ExprKind::ClassInit(_, _) => true,
        ExprKind::Unary(_, inner) => expr_requires_region(inner, registry),
        ExprKind::Binary(op, lhs, rhs) => {
            if *op == BinaryOp::Add && (is_string_expr(lhs) || is_string_expr(rhs)) {
                return true;
            }
            expr_requires_region(lhs, registry) || expr_requires_region(rhs, registry)
        }
        ExprKind::Ternary(c, t, e) => {
            expr_requires_region(c, registry)
                || expr_requires_region(t, registry)
                || expr_requires_region(e, registry)
        }
        ExprKind::Call(callee, args) => {
            if call_returns_regional_type(callee, args, registry) {
                return true;
            }
            expr_requires_region(callee, registry)
                || args.iter().any(|a| expr_requires_region(a, registry))
        }
        ExprKind::MethodCall(target, method, args) => {
            if method.name == "push" || method.name == "reserve" {
                return true;
            }
            if method_call_returns_regional_type(&method.name, registry) {
                return true;
            }
            expr_requires_region(target, registry)
                || args.iter().any(|a| expr_requires_region(a, registry))
        }
        ExprKind::If { cond, then_branch, elif_branches, else_branch } => {
            expr_requires_region(cond, registry)
                || block_requires_region(then_branch, registry)
                || elif_branches.iter().any(|(e, b)| expr_requires_region(e, registry) || block_requires_region(b, registry))
                || else_branch.as_ref().is_some_and(|b| block_requires_region(b, registry))
        }
        ExprKind::While { cond, body, .. } | ExprKind::For { iter: cond, body, .. } => {
            expr_requires_region(cond, registry) || block_requires_region(body, registry)
        }
        ExprKind::Loop { body, .. } | ExprKind::Block(body) => block_requires_region(body, registry),
        ExprKind::Switch { target, arms } => {
            expr_requires_region(target, registry)
                || arms.iter().any(|a| block_requires_region(&a.body, registry))
        }
        ExprKind::IndexAccess(target, index) => {
            expr_requires_region(target, registry) || expr_requires_region(index, registry)
        }
        ExprKind::SliceAccess(target, start, end, _) => {
            expr_requires_region(target, registry)
                || start.as_ref().is_some_and(|e| expr_requires_region(e, registry))
                || end.as_ref().is_some_and(|e| expr_requires_region(e, registry))
        }
        ExprKind::FieldAccess(target, _) | ExprKind::Ref { expr: target, .. } => {
            expr_requires_region(target, registry)
        }
        ExprKind::Sus { body, catches, finally_branch } => {
            block_requires_region(body, registry)
                || catches.iter().any(|c| block_requires_region(&c.body, registry))
                || finally_branch.as_ref().is_some_and(|f| block_requires_region(f, registry))
        }
        _ => false,
    }
}
