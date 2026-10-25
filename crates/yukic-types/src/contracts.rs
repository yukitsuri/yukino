#![deny(unsafe_code)]
#![allow(clippy::collapsible_if)]

use std::collections::{HashMap, HashSet};
use yukic_ast::{CfunDecl, Expr, ExprKind, FunDecl, LiteralValue, TypeAliasDecl};
use yukic_diagnostics::Diagnostic;
use yukic_diagnostics::code::ErrorCode;
use yukic_span::Span;

use crate::convert::ast_type_to_type;
use crate::types::Type;

pub struct SemanticContracts;

impl SemanticContracts {
    pub fn is_c_abi_compatible(ty: &Type) -> bool {
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
                | Type::Void
                | Type::Never
                | Type::Ptr(_)
                | Type::Fn { is_cfun: true, .. }
                | Type::CInt(_)
                | Type::Cstruct(_)
                | Type::Dynlib
        )
    }

    pub fn validate_cfun(decl: &CfunDecl) -> Result<(), Diagnostic> {
        for param in &decl.params {
            let ty = ast_type_to_type(&param.ty);
            if !Self::is_c_abi_compatible(&ty) {
                let diag = Diagnostic::error(
                    ErrorCode::E0701,
                    format!("type `{ty}` is not C ABI compatible in cfun declaration"),
                    param.span,
                );
                return Err(diag);
            }
        }

        if let Some(ret_ty_node) = &decl.return_type {
            let ret_ty = ast_type_to_type(ret_ty_node);
            if !Self::is_c_abi_compatible(&ret_ty) {
                let diag = Diagnostic::error(
                    ErrorCode::E0701,
                    format!("return type `{ret_ty}` is not C ABI compatible in cfun declaration"),
                    ret_ty_node.span,
                );
                return Err(diag);
            }
        }

        Ok(())
    }

    pub fn validate_test_modifiers(decl: &FunDecl) -> Result<(), Diagnostic> {
        let has_test = decl.attrs.iter().any(|a| a.is("test"));
        for attr in &decl.attrs {
            let name = attr.path_str();
            if (name == "should_panic" || name == "ignore" || name == "timeout_ms") && !has_test {
                return Err(Diagnostic::error(
                    ErrorCode::E0429,
                    format!(
                        "test modifier '#[{name}]' can only be applied to functions with '#[test]'"
                    ),
                    attr.span,
                ));
            }
        }
        Ok(())
    }

    pub fn validate_thunk_decl(decl: &FunDecl) -> Result<(), Diagnostic> {
        let thunk_attr = decl.attrs.iter().find(|a| a.is("thunk"));
        let Some(attr) = thunk_attr else {
            return Ok(());
        };
        if !matches!(attr.args, yukic_ast::AttributeArgs::None) {
            return Err(Diagnostic::error(
                ErrorCode::E0401,
                "attribute '#[thunk]' does not accept arguments".to_string(),
                attr.span,
            ));
        }
        for param in &decl.params {
            if let Some(ty_node) = &param.ty {
                let ty = ast_type_to_type(ty_node);
                if !Self::is_c_abi_compatible(&ty) {
                    return Err(Diagnostic::error(
                        ErrorCode::E0701,
                        format!("parameter type `{ty}` is not C ABI compatible in #[thunk] function"),
                        param.span,
                    ));
                }
            }
        }
        if let Some(ret_ty_node) = &decl.return_type {
            let ret_ty = ast_type_to_type(ret_ty_node);
            if !Self::is_c_abi_compatible(&ret_ty) {
                return Err(Diagnostic::error(
                    ErrorCode::E0701,
                    format!("return type `{ret_ty}` is not C ABI compatible in #[thunk] function"),
                    ret_ty_node.span,
                ));
            }
        }
        Ok(())
    }

    pub fn validate_stable_decl(decl: &FunDecl, calls_impure: bool) -> Result<(), Diagnostic> {
        let is_stable = decl.attrs.iter().any(|a| a.is("stable"));
        if is_stable && calls_impure {
            return Err(Diagnostic::error(
                ErrorCode::E0417,
                format!(
                    "stable function '{}' calls impure function or performs side-effects",
                    decl.name.name
                ),
                decl.span,
            ));
        }
        Ok(())
    }

    pub fn check_alias_cycles(aliases: &[TypeAliasDecl]) -> Result<(), Diagnostic> {
        let mut map: HashMap<&str, (Vec<&str>, Span)> = HashMap::new();
        for alias in aliases {
            let mut idents = Vec::new();
            collect_target_idents(&alias.target, &mut idents);
            map.insert(alias.name.name.as_str(), (idents, alias.span));
        }

        for (name, (_, span)) in &map {
            let mut visited = HashSet::new();
            if has_cycle(name, name, &map, &mut visited) {
                let diag = Diagnostic::error(
                    ErrorCode::E0407,
                    format!("recursive type alias detected for `{name}`"),
                    *span,
                );
                return Err(diag);
            }
        }

        Ok(())
    }

    pub fn validate_cast(
        val_expr: &Expr,
        val_ty: &Type,
        target_expr: &Expr,
        target_ty: &Type,
        span: Span,
    ) -> Result<(), Diagnostic> {

        if !Self::is_constant_target(target_expr) {
            let diag = Diagnostic::error(
                ErrorCode::E0431,
                "target type in `cast` must be a compile-time string literal or `typeof`",
                target_expr.span,
            );
            return Err(diag);
        }

        if let ExprKind::Literal(LiteralValue::Int(n)) = &val_expr.kind {
            if target_ty.is_integer() && !target_ty.fits_i64(*n) {
                let msg = match target_ty.bit_width() {
                    Some(b) => {
                        format!("value `{n}` exceeds {b} bits for target type `{target_ty}`")
                    }
                    None => format!("value `{n}` exceeds range of `{target_ty}`"),
                };
                let diag = Diagnostic::error(ErrorCode::E0430, msg, span);
                return Err(diag);
            }
        }

        if !Self::is_cast_compatible(val_ty, target_ty) {
            let diag = Diagnostic::error(
                ErrorCode::E0430,
                format!("invalid type cast from `{val_ty}` to `{target_ty}`"),
                span,
            );
            return Err(diag);
        }

        Ok(())
    }

    fn is_constant_target(expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::Literal(LiteralValue::String(_)) => true,
            ExprKind::Call(callee, _) => {
                if let ExprKind::Ident(id) = &callee.kind {
                    id.name == "typeof" || id.name == "__typeof_intrinsic__"
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn is_cast_compatible(src: &Type, target: &Type) -> bool {
        if src == target {
            return true;
        }

        // numeric conversions
        if src.is_numeric() && target.is_numeric() {
            return true;
        }

        // numbers to/from string
        if (src.is_numeric() && *target == Type::String)
            || (*src == Type::String && target.is_numeric())
        {
            return true;
        }

        // char <-> string, char <-> integer, string <-> char[]
        if (*src == Type::Char && *target == Type::String)
            || (*src == Type::String && *target == Type::Char)
        {
            return true;
        }
        if (*src == Type::Char && target.is_integer())
            || (src.is_integer() && *target == Type::Char)
        {
            return true;
        }
        if (*src == Type::String && *target == Type::Array(Box::new(Type::Char)))
            || (*src == Type::Array(Box::new(Type::Char)) && *target == Type::String)
        {
            return true;
        }

        // ptr and func ptr convers
        if matches!(src, Type::Ptr(_) | Type::Fn { .. } | Type::Null)
            && matches!(target, Type::Ptr(_) | Type::Fn { .. })
        {
            return true;
        }
        if matches!(src, Type::Ptr(_) | Type::Fn { .. })
            && matches!(
                target,
                Type::Uint64 | Type::Int64 | Type::Float64 | Type::String
            )
        {
            return true;
        }
        if matches!(
            src,
            Type::Uint64 | Type::Int64 | Type::Float64 | Type::String
        ) && matches!(target, Type::Ptr(_) | Type::Fn { .. })
        {
            return true;
        }

        // bytes <-> string
        if (*src == Type::Bytes && *target == Type::String)
            || (*src == Type::String && *target == Type::Bytes)
        {
            return true;
        }

        // bool conversions
        if matches!(src, Type::Bool) && (target.is_integer() || *target == Type::String) {
            return true;
        }
        if (src.is_integer() || *src == Type::String) && matches!(target, Type::Bool) {
            return true;
        }

        false
    }
}

fn collect_target_idents<'a>(node: &'a yukic_ast::types::TypeNode, idents: &mut Vec<&'a str>) {
    use yukic_ast::types::TypeNodeKind;
    match &node.kind {
        TypeNodeKind::Ident(id) => idents.push(id.name.as_str()),
        TypeNodeKind::Path(path) => {
            if let Some(first) = path.first() {
                idents.push(first.name.as_str());
            }
        }
        TypeNodeKind::Pointer(inner)
        | TypeNodeKind::Ref(inner)
        | TypeNodeKind::Array(inner, _)
        | TypeNodeKind::Optional(inner) => collect_target_idents(inner, idents),
        TypeNodeKind::Map(k, v) => {
            collect_target_idents(k, idents);
            collect_target_idents(v, idents);
        }
        TypeNodeKind::Tuple(members) | TypeNodeKind::Union(members) => {
            for m in members {
                collect_target_idents(m, idents);
            }
        }
        TypeNodeKind::Fun(params, ret) | TypeNodeKind::CFun(params, ret) => {
            for p in params {
                collect_target_idents(p, idents);
            }
            collect_target_idents(ret, idents);
        }
        TypeNodeKind::Generic(name, args) => {
            idents.push(name.name.as_str());
            for arg in args {
                collect_target_idents(arg, idents);
            }
        }
        TypeNodeKind::Result(ok, err) => {
            collect_target_idents(ok, idents);
            collect_target_idents(err, idents);
        }
    }
}

fn has_cycle<'a>(
    curr: &'a str,
    root: &'a str,
    map: &HashMap<&'a str, (Vec<&'a str>, Span)>,
    visited: &mut HashSet<&'a str>,
) -> bool {
    if let Some((next_idents, _)) = map.get(curr) {
        for next in next_idents {
            if *next == root {
                return true;
            }
            if visited.insert(*next) {
                if has_cycle(*next, root, map, visited) {
                    return true;
                }
                visited.remove(*next);
            }
        }
    }
    false
}
