#![deny(unsafe_code)]

use std::collections::HashSet;

use yukic_ast::expr::{BinaryOp, Expr, ExprKind, LiteralValue, UnaryOp};
use yukic_ast::item::{FunDecl, MethodDecl};
use yukic_ast::stmt::{BindingTarget, Block, StmtKind};
use yukic_ast::types::{TypeNode, TypeNodeKind};

use crate::region::RegionManager;
use crate::registry::TypeRegistry;
use yukic_types::Type;

#[derive(Clone, Default)]
pub struct LocalScope {
    pub numeric_vars: HashSet<String>,
    pub numeric_arrays: HashSet<String>,
    pub string_vars: HashSet<String>,
}

fn is_type_node_numeric(ty: &TypeNode) -> bool {
    match &ty.kind {
        TypeNodeKind::Ident(id) => matches!(
            id.name.as_str(),
            "int" | "int8" | "int16" | "int32" | "int64" | "int128" | "isize"
                | "uint" | "uint8" | "uint16" | "uint32" | "uint64" | "uint128" | "usize"
                | "float" | "float32" | "float64"
                | "bool" | "char"
        ),
        TypeNodeKind::Pointer(_) => true,
        _ => false,
    }
}

fn is_type_node_string(ty: &TypeNode) -> bool {
    match &ty.kind {
        TypeNodeKind::Ident(id) => id.name == "string",
        _ => false,
    }
}

fn is_type_node_numeric_array(ty: &TypeNode) -> bool {
    match &ty.kind {
        TypeNodeKind::Array(inner, _) => is_type_node_numeric(inner),
        _ => false,
    }
}

fn is_type_numeric(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Int8
            | Type::Int16
            | Type::Int32
            | Type::Int64
            | Type::Uint8
            | Type::Uint16
            | Type::Uint32
            | Type::Uint64
            | Type::Float32
            | Type::Float64
            | Type::Bool
            | Type::Char
            | Type::Ptr(_)
    )
}

fn get_call_return_type(callee: &Expr, arg_count: usize, registry: &TypeRegistry) -> Option<Type> {
    let name = match &callee.kind {
        ExprKind::Ident(id) => Some(id.name.as_str()),
        ExprKind::Path(idents) => idents.last().map(|id| id.name.as_str()),
        _ => None,
    }?;
    let mangled = registry.resolve_fun_symbol(name, arg_count);
    let fn_full_ty = registry
        .fn_types
        .get(name)
        .or_else(|| mangled.as_ref().and_then(|m| registry.fn_types.get(m)));
    if let Some(Type::Fn { ret, .. }) = fn_full_ty {
        return Some((**ret).clone());
    }
    let fn_ty = registry
        .functions
        .get(name)
        .or_else(|| mangled.as_ref().and_then(|m| registry.functions.get(m)));
    fn_ty.cloned()
}

pub fn fun_needs_local_region(
    fun: &FunDecl,
    has_dest: bool,
    registry: &TypeRegistry,
) -> bool {
    if fun.attrs.iter().any(|a| a.is("stable") || a.is("no_region")) {
        return false;
    }
    if has_dest {
        return true;
    }
    let mut scope = LocalScope::default();
    for param in &fun.params {
        if let Some(ty) = &param.ty {
            if is_type_node_numeric(ty) {
                scope.numeric_vars.insert(param.name.name.clone());
            } else if is_type_node_numeric_array(ty) {
                scope.numeric_arrays.insert(param.name.name.clone());
            } else if is_type_node_string(ty) {
                scope.string_vars.insert(param.name.name.clone());
            }
        }
    }
    block_requires_region_scoped(&fun.body, &mut scope, registry)
}

pub fn method_needs_local_region(
    method: &MethodDecl,
    has_dest: bool,
    registry: &TypeRegistry,
) -> bool {
    if method.attrs.iter().any(|a| a.is("stable") || a.is("no_region")) {
        return false;
    }
    if has_dest {
        return true;
    }
    let mut scope = LocalScope::default();
    for param in &method.params {
        if let Some(ty) = &param.ty {
            if is_type_node_numeric(ty) {
                scope.numeric_vars.insert(param.name.name.clone());
            } else if is_type_node_numeric_array(ty) {
                scope.numeric_arrays.insert(param.name.name.clone());
            } else if is_type_node_string(ty) {
                scope.string_vars.insert(param.name.name.clone());
            }
        }
    }
    block_requires_region_scoped(&method.body, &mut scope, registry)
}

pub fn block_requires_region(block: &Block, registry: &TypeRegistry) -> bool {
    let mut scope = LocalScope::default();
    block_requires_region_scoped(block, &mut scope, registry)
}

fn block_requires_region_scoped(
    block: &Block,
    scope: &mut LocalScope,
    registry: &TypeRegistry,
) -> bool {
    for stmt in &block.stmts {
        if stmt_requires_region_scoped(&stmt.kind, scope, registry) {
            return true;
        }
    }
    false
}

fn stmt_requires_region_scoped(
    stmt: &StmtKind,
    scope: &mut LocalScope,
    registry: &TypeRegistry,
) -> bool {
    match stmt {
        StmtKind::Defer(_) | StmtKind::MacroCall(_) => true,
        StmtKind::Set { target, ty, init, .. } => {
            if expr_requires_region_scoped(init, scope, registry) {
                return true;
            }
            if let BindingTarget::Ident(name) = target {
                if let Some(ty_node) = ty {
                    if is_type_node_numeric(ty_node) {
                        scope.numeric_vars.insert(name.name.clone());
                    } else if is_type_node_numeric_array(ty_node) {
                        scope.numeric_arrays.insert(name.name.clone());
                    } else if is_type_node_string(ty_node) {
                        scope.string_vars.insert(name.name.clone());
                    }
                } else if is_definitely_numeric_with_scope(init, scope, registry) {
                    scope.numeric_vars.insert(name.name.clone());
                } else if is_string_expr_with_scope(init, scope) {
                    scope.string_vars.insert(name.name.clone());
                }
            }
            false
        }
        StmtKind::Rewrite { value, .. } => expr_requires_region_scoped(value, scope, registry),
        StmtKind::Expr(expr) | StmtKind::Return(Some(expr)) => {
            expr_requires_region_scoped(expr, scope, registry)
        }
        StmtKind::Recur(args) => {
            args.iter().any(|arg| expr_requires_region_scoped(arg, scope, registry))
        }
        _ => false,
    }
}

pub fn is_definitely_numeric(expr: &Expr) -> bool {
    let scope = LocalScope::default();
    let reg = TypeRegistry::default();
    is_definitely_numeric_with_scope(expr, &scope, &reg)
}

fn is_definitely_numeric_with_scope(
    expr: &Expr,
    scope: &LocalScope,
    registry: &TypeRegistry,
) -> bool {
    match &expr.kind {
        ExprKind::Literal(lit) => matches!(
            lit,
            LiteralValue::Int(_)
                | LiteralValue::UInt(_)
                | LiteralValue::Float(_)
                | LiteralValue::TypedInt(_, _)
                | LiteralValue::TypedUInt(_, _)
                | LiteralValue::TypedFloat(_, _)
                | LiteralValue::Bool(_)
                | LiteralValue::Char(_)
        ),
        ExprKind::Ident(id) => {
            scope.numeric_vars.contains(&id.name)
                || registry.static_vars.get(&id.name).is_some_and(is_type_numeric)
        }
        ExprKind::IndexAccess(target, _) => {
            if let ExprKind::Ident(id) = &target.kind {
                scope.numeric_arrays.contains(&id.name)
            } else {
                false
            }
        }
        ExprKind::Unary(op, inner) => {
            matches!(op, UnaryOp::Neg | UnaryOp::BitNot)
                && is_definitely_numeric_with_scope(inner, scope, registry)
        }
        ExprKind::Binary(op, lhs, rhs) => {
            matches!(
                op,
                BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem
                    | BinaryOp::Pow
                    | BinaryOp::BitAnd
                    | BinaryOp::BitOr
                    | BinaryOp::BitXor
                    | BinaryOp::Shl
                    | BinaryOp::Shr
            ) && is_definitely_numeric_with_scope(lhs, scope, registry)
                && is_definitely_numeric_with_scope(rhs, scope, registry)
        }
        ExprKind::Call(callee, args) => {
            if let Some(ret_ty) = get_call_return_type(callee, args.len(), registry) {
                is_type_numeric(&ret_ty)
            } else {
                false
            }
        }
        ExprKind::Ternary(_, t, e) => {
            is_definitely_numeric_with_scope(t, scope, registry)
                && is_definitely_numeric_with_scope(e, scope, registry)
        }
        _ => false,
    }
}

pub fn is_string_expr(expr: &Expr) -> bool {
    is_string_expr_with_scope(expr, &LocalScope::default())
}

fn is_string_expr_with_scope(expr: &Expr, scope: &LocalScope) -> bool {
    match &expr.kind {
        ExprKind::Literal(LiteralValue::String(_)) | ExprKind::InterpolatedString(_) => true,
        ExprKind::Ident(id) => scope.string_vars.contains(&id.name),
        ExprKind::Binary(BinaryOp::Add, lhs, rhs) => {
            is_string_expr_with_scope(lhs, scope) || is_string_expr_with_scope(rhs, scope)
        }
        _ => false,
    }
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
        if matches!(
            id_name,
            "_internal_region_used"
                | "_internal_region_cap"
                | "_internal_region_reset"
                | "_internal_export"
                | "region_used"
                | "region_cap"
                | "region_reset"
                | "export"
                | "raw_args"
        ) {
            return true;
        }
        if id_name == "cast" && args.len() >= 2 && is_regional_cast_target(&args[1]) {
            return true;
        }
        let mangled = registry.resolve_fun_symbol(id_name, args.len());
        let fn_full_ty = registry
            .fn_types
            .get(id_name)
            .or_else(|| mangled.as_ref().and_then(|m| registry.fn_types.get(m)));
        if let Some(Type::Fn { params, ret, is_cfun }) = fn_full_ty {
            if !is_cfun {
                if RegionManager::is_regional_type(ret) {
                    return true;
                }
                if params.iter().any(|p| {
                    (p.is_ref && RegionManager::is_regional_type(&p.ty))
                        || matches!(&p.ty, Type::Ref { inner, .. } if RegionManager::is_regional_type(inner))
                }) {
                    return true;
                }
            }
        }
        let fn_ty = registry
            .functions
            .get(id_name)
            .or_else(|| mangled.as_ref().and_then(|m| registry.functions.get(m)));
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
    for (fn_name, fn_ty) in &registry.fn_types {
        if fn_name.ends_with(&suffix) || fn_name.contains(&format!("::{method_name}_")) {
            if let Type::Fn { params, ret, is_cfun } = fn_ty {
                if !is_cfun {
                    if RegionManager::is_regional_type(ret) {
                        return true;
                    }
                    if params.iter().any(|p| {
                        (p.is_ref && RegionManager::is_regional_type(&p.ty))
                            || matches!(&p.ty, Type::Ref { inner, .. } if RegionManager::is_regional_type(inner))
                    }) {
                        return true;
                    }
                }
            }
        }
    }
    for (fn_name, ret_ty) in &registry.functions {
        if (fn_name.ends_with(&suffix) || fn_name.contains(&format!("::{method_name}_")))
            && RegionManager::is_regional_type(ret_ty)
        {
            return true;
        }
    }
    false
}

pub fn expr_requires_region(expr: &Expr, registry: &TypeRegistry) -> bool {
    expr_requires_region_scoped(expr, &LocalScope::default(), registry)
}

fn expr_requires_region_scoped(
    expr: &Expr,
    scope: &LocalScope,
    registry: &TypeRegistry,
) -> bool {
    match &expr.kind {
        ExprKind::InterpolatedString(_)
        | ExprKind::ArrayInit(_)
        | ExprKind::ArrayAlloc(..)
        | ExprKind::MapInit(_)
        | ExprKind::TupleInit(_)
        | ExprKind::AnonymousFun(_)
        | ExprKind::Lambda(_, _)
        | ExprKind::RegionBlock(_)
        | ExprKind::RegionScopedBlock(_, _)
        | ExprKind::StructInit(_, _)
        | ExprKind::ClassInit(_, _) => true,
        ExprKind::Unary(_, inner) => expr_requires_region_scoped(inner, scope, registry),
        ExprKind::Binary(op, lhs, rhs) => {
            if *op == BinaryOp::Add {
                if is_string_expr_with_scope(lhs, scope) || is_string_expr_with_scope(rhs, scope) {
                    return true;
                }
                if !is_definitely_numeric_with_scope(lhs, scope, registry)
                    || !is_definitely_numeric_with_scope(rhs, scope, registry)
                {
                    return true;
                }
            }
            expr_requires_region_scoped(lhs, scope, registry)
                || expr_requires_region_scoped(rhs, scope, registry)
        }
        ExprKind::Ternary(c, t, e) => {
            expr_requires_region_scoped(c, scope, registry)
                || expr_requires_region_scoped(t, scope, registry)
                || expr_requires_region_scoped(e, scope, registry)
        }
        ExprKind::Call(callee, args) => {
            if call_returns_regional_type(callee, args, registry) {
                return true;
            }
            expr_requires_region_scoped(callee, scope, registry)
                || args.iter().any(|a| expr_requires_region_scoped(a, scope, registry))
        }
        ExprKind::MethodCall(target, method, args) => {
            if method.name == "push" || method.name == "reserve" {
                return true;
            }
            if method_call_returns_regional_type(&method.name, registry) {
                return true;
            }
            expr_requires_region_scoped(target, scope, registry)
                || args.iter().any(|a| expr_requires_region_scoped(a, scope, registry))
        }
        ExprKind::If { cond, then_branch, elif_branches, else_branch } => {
            expr_requires_region_scoped(cond, scope, registry)
                || block_requires_region_scoped(then_branch, &mut scope.clone(), registry)
                || elif_branches.iter().any(|(e, b)| {
                    expr_requires_region_scoped(e, scope, registry)
                        || block_requires_region_scoped(b, &mut scope.clone(), registry)
                })
                || else_branch.as_ref().is_some_and(|b| {
                    block_requires_region_scoped(b, &mut scope.clone(), registry)
                })
        }
        ExprKind::While { cond, body, .. } => {
            expr_requires_region_scoped(cond, scope, registry)
                || block_requires_region_scoped(body, &mut scope.clone(), registry)
        }
        ExprKind::For { index_var, item_var, iter, body, .. } => {
            if expr_requires_region_scoped(iter, scope, registry) {
                return true;
            }
            let mut for_scope = scope.clone();
            if let Some(idx) = index_var {
                for_scope.numeric_vars.insert(idx.name.clone());
            }
            if matches!(
                &iter.kind,
                ExprKind::Binary(BinaryOp::Range | BinaryOp::RangeInclusive, ..)
            ) {
                for_scope.numeric_vars.insert(item_var.name.clone());
            }
            block_requires_region_scoped(body, &mut for_scope, registry)
        }
        ExprKind::Loop { body, .. } | ExprKind::Block(body) => {
            block_requires_region_scoped(body, &mut scope.clone(), registry)
        }
        ExprKind::Switch { target, arms } => {
            expr_requires_region_scoped(target, scope, registry)
                || arms.iter().any(|a| block_requires_region_scoped(&a.body, &mut scope.clone(), registry))
        }
        ExprKind::IndexAccess(target, index) => {
            expr_requires_region_scoped(target, scope, registry)
                || expr_requires_region_scoped(index, scope, registry)
        }
        ExprKind::SliceAccess(target, start, end, _) => {
            expr_requires_region_scoped(target, scope, registry)
                || start.as_ref().is_some_and(|e| expr_requires_region_scoped(e, scope, registry))
                || end.as_ref().is_some_and(|e| expr_requires_region_scoped(e, scope, registry))
        }
        ExprKind::FieldAccess(target, _) | ExprKind::Ref { expr: target, .. } => {
            expr_requires_region_scoped(target, scope, registry)
        }
        ExprKind::Sus { body, catches, finally_branch } => {
            block_requires_region_scoped(body, &mut scope.clone(), registry)
                || catches.iter().any(|c| block_requires_region_scoped(&c.body, &mut scope.clone(), registry))
                || finally_branch.as_ref().is_some_and(|f| block_requires_region_scoped(f, &mut scope.clone(), registry))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_ast::attr::{AttributeArgs, AttributeNode, AttributePath};
    use yukic_ast::expr::{Expr, ExprKind};
    use yukic_ast::ident::Ident;
    use yukic_ast::item::{FunDecl, Param};
    use yukic_ast::stmt::{Block, Stmt, StmtKind};
    use yukic_ast::types::{TypeNode, TypeNodeKind};
    use yukic_span::{BytePos, FileId, Span};

    fn dummy_span() -> Span {
        Span::new(FileId(0), BytePos(0), BytePos(0))
    }

    #[test]
    fn test_numeric_addition_elides_region() {
        let sp = dummy_span();
        let p1 = Param {
            name: Ident::new("a", sp),
            is_ref: false,
            is_mut: false,
            is_variadic: false,
            ty: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("int32", sp)), sp)),
            default_val: None,
            span: sp,
        };
        let p2 = Param {
            name: Ident::new("b", sp),
            is_ref: false,
            is_mut: false,
            is_variadic: false,
            ty: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("int32", sp)), sp)),
            default_val: None,
            span: sp,
        };
        let add_expr = Expr::new(
            ExprKind::Binary(
                BinaryOp::Add,
                Box::new(Expr::new(ExprKind::Ident(Ident::new("a", sp)), sp)),
                Box::new(Expr::new(ExprKind::Ident(Ident::new("b", sp)), sp)),
            ),
            sp,
        );
        let ret_stmt = Stmt::new(StmtKind::Return(Some(add_expr)), sp);
        let fun = FunDecl {
            name: Ident::new("add", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: vec![p1, p2],
            return_type: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("int32", sp)), sp)),
            body: Block::new(vec![ret_stmt], sp),
            span: sp,
        };
        let reg = TypeRegistry::default();
        assert!(!fun_needs_local_region(&fun, false, &reg));
    }

    #[test]
    fn test_no_region_attribute_suppresses_region() {
        let sp = dummy_span();
        let attr = AttributeNode::new(
            AttributePath::from_ident(Ident::new("no_region", sp)),
            AttributeArgs::None,
            sp,
        );
        let fun = FunDecl {
            name: Ident::new("fast_func", sp),
            attrs: vec![attr],
            generics: Vec::new(),
            params: Vec::new(),
            return_type: None,
            body: Block::new(Vec::new(), sp),
            span: sp,
        };
        let reg = TypeRegistry::default();
        assert!(!fun_needs_local_region(&fun, false, &reg));
    }

    #[test]
    fn test_string_concat_requires_region() {
        let sp = dummy_span();
        let p1 = Param {
            name: Ident::new("s1", sp),
            is_ref: false,
            is_mut: false,
            is_variadic: false,
            ty: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("string", sp)), sp)),
            default_val: None,
            span: sp,
        };
        let p2 = Param {
            name: Ident::new("s2", sp),
            is_ref: false,
            is_mut: false,
            is_variadic: false,
            ty: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("string", sp)), sp)),
            default_val: None,
            span: sp,
        };
        let add_expr = Expr::new(
            ExprKind::Binary(
                BinaryOp::Add,
                Box::new(Expr::new(ExprKind::Ident(Ident::new("s1", sp)), sp)),
                Box::new(Expr::new(ExprKind::Ident(Ident::new("s2", sp)), sp)),
            ),
            sp,
        );
        let ret_stmt = Stmt::new(StmtKind::Return(Some(add_expr)), sp);
        let fun = FunDecl {
            name: Ident::new("concat", sp),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: vec![p1, p2],
            return_type: Some(TypeNode::new(TypeNodeKind::Ident(Ident::new("string", sp)), sp)),
            body: Block::new(vec![ret_stmt], sp),
            span: sp,
        };
        let reg = TypeRegistry::default();
        assert!(fun_needs_local_region(&fun, false, &reg));
    }
}
