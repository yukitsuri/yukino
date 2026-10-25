#![deny(unsafe_code)]

use yukic_ast::expr::{BinaryOp, UnaryOp};

pub fn mangle_operator_symbol(symbol: &str) -> Option<&'static str> {
    match symbol {
        "+" => Some("__op_add"),
        "-" => Some("__op_sub"),
        "*" => Some("__op_mul"),
        "/" => Some("__op_div"),
        "%" => Some("__op_rem"),
        "**" => Some("__op_pow"),
        "&" => Some("__op_bitand"),
        "|" => Some("__op_bitor"),
        "^" => Some("__op_bitxor"),
        "~" => Some("__op_bitnot"),
        "<<" => Some("__op_shl"),
        ">>" => Some("__op_shr"),
        "==" => Some("__op_eq"),
        "!=" => Some("__op_ne"),
        "<" => Some("__op_lt"),
        "<=" => Some("__op_le"),
        ">" => Some("__op_gt"),
        ">=" => Some("__op_ge"),
        "!" => Some("__op_not"),
        "[]" => Some("__op_index"),
        "[]=" => Some("__op_index_set"),
        ".." => Some("__op_range"),
        "..=" => Some("__op_range_inclusive"),
        _ => None,
    }
}

pub fn mangle_method_name(name: &str, param_count: usize) -> &str {
    if name == "-" && param_count == 0 { return "__op_neg"; }
    if let Some(mangled) = mangle_operator_symbol(name) { return mangled; }
    
    name
}

pub fn method_symbol_name(class_name: &str, method_name: &str, param_count: usize) -> String {
    let mangled = mangle_method_name(method_name, param_count);
    format!("_Y_{class_name}_{mangled}")
}

pub fn itable_symbol_name(class_name: &str, iface_name: &str) -> String {
    let sanitized = iface_name
        .replace('<', "_")
        .replace('>', "_")
        .replace(',', "_")
        .replace(' ', "");
    format!("_Y_ITABLE_{class_name}_{sanitized}")
}

pub fn binary_op_to_symbol(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
        BinaryOp::Pow => "**",
        BinaryOp::BitAnd | BinaryOp::And => "&",
        BinaryOp::BitOr | BinaryOp::Or => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
        BinaryOp::Eq => "==",
        BinaryOp::NotEq => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::LtEq => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::GtEq => ">=",
        BinaryOp::NullCoalesce => "??",
        BinaryOp::Range => "..",
        BinaryOp::RangeInclusive => "..=",
    }
}

pub fn unary_op_to_symbol(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::Not => "!",
        UnaryOp::BitNot => "~",
    }
}

pub fn mangle_type_str(ty: &yukic_types::Type) -> String {
    use yukic_types::Type;
    match ty {
        Type::Int8 => "int8".to_string(),
        Type::Int16 => "int16".to_string(),
        Type::Int32 => "int32".to_string(),
        Type::Int64 => "int64".to_string(),
        Type::Uint8 => "uint8".to_string(),
        Type::Uint16 => "uint16".to_string(),
        Type::Uint32 => "uint32".to_string(),
        Type::Uint64 => "uint64".to_string(),
        Type::Float32 => "float32".to_string(),
        Type::Float64 => "float64".to_string(),
        Type::Bool => "bool".to_string(),
        Type::Char => "char".to_string(),
        Type::String => "string".to_string(),
        Type::Bytes => "bytes".to_string(),
        Type::Void => "void".to_string(),
        Type::Never => "never".to_string(),
        Type::Null => "null".to_string(),
        Type::Tuple(members) => {
            let inner: Vec<String> = members.iter().map(mangle_type_str).collect();
            format!("tup_{}", inner.join("_"))
        }
        Type::Struct(name)
        | Type::Class(name)
        | Type::Interface(name)
        | Type::Enum(name)
        | Type::Cstruct(name) => name.replace("::", "_").replace('.', "_"),
        Type::Array(inner) => format!("arr_{}", mangle_type_str(inner)),
        Type::Map(k, v) => format!("map_{}_{}", mangle_type_str(k), mangle_type_str(v)),
        Type::Union(members) => {
            if members.len() == 2 && members.contains(&Type::Null) {
                let non_null = members.iter().find(|m| *m != &Type::Null).unwrap();
                format!("opt_{}", mangle_type_str(non_null))
            } else {
                let inner: Vec<String> = members.iter().map(mangle_type_str).collect();
                format!("union_{}", inner.join("_"))
            }
        }
        Type::Shared(inner) => format!("shared_{}", mangle_type_str(inner)),
        Type::Weak(inner) => format!("weak_{}", mangle_type_str(inner)),
        Type::Ptr(inner) => format!("ptr_{}", mangle_type_str(inner)),
        Type::Ref { inner, mutable } => {
            if *mutable {
                format!("mutref_{}", mangle_type_str(inner))
            } else {
                format!("ref_{}", mangle_type_str(inner))
            }
        }
        Type::CInt(k) => format!("cint_{k:?}").to_lowercase(),
        Type::Dynlib => "dynlib".to_string(),
        Type::Fn { params, ret, .. } => {
            let p_str: Vec<String> = params.iter().map(|p| mangle_type_str(&p.ty)).collect();
            format!("fn_{}__{}", p_str.join("_"), mangle_type_str(ret))
        }
        Type::TypeVar(name) => name.replace("::", "_"),
        Type::Infer(id) => format!("infer{id}"),
        Type::Error => "err".to_string(),
    }
}

pub fn mangle_fun_symbol(name: &str, params: &[yukic_types::Type], ret: &yukic_types::Type) -> String {
    let ret_str = mangle_type_str(ret);
    if params.is_empty() {
        format!("__yukifun_{name}___{ret_str}")
    } else {
        let args_str: Vec<String> = params.iter().map(mangle_type_str).collect();
        format!("__yukifun_{name}_{}__{ret_str}", args_str.join("_"))
    }
}

pub fn should_mangle_fun(name: &str, attrs: &[yukic_ast::attr::AttributeNode]) -> bool {
    if name == "main" {
        return false;
    }
    !attrs.iter().any(|a| a.is("mangle") && a.negated)
}

pub fn should_mangle_cfun(name: &str, attrs: &[yukic_ast::attr::AttributeNode]) -> bool {
    if name == "main" {
        return false;
    }
    attrs.iter().any(|a| a.is("mangle") && !a.negated)
}

pub fn fun_param_types(
    fun: &yukic_ast::item::FunDecl,
    registry: &crate::registry::TypeRegistry,
) -> Vec<yukic_types::Type> {
    fun.params
        .iter()
        .map(|p| {
            let mut ty = p
                .ty
                .as_ref()
                .map_or(yukic_types::Type::Void, |t| {
                    registry.canonicalize_type(&yukic_types::convert::ast_type_to_type(t))
                });
            if p.is_variadic && !matches!(ty, yukic_types::Type::Array(_)) {
                ty = yukic_types::Type::Array(Box::new(ty));
            }
            if p.is_ref {
                ty = yukic_types::Type::Ref {
                    inner: Box::new(ty),
                    mutable: p.is_mut,
                };
            }
            ty
        })
        .collect()
}

pub fn fun_return_type(
    fun: &yukic_ast::item::FunDecl,
    registry: &crate::registry::TypeRegistry,
) -> yukic_types::Type {
    fun.return_type
        .as_ref()
        .map_or(yukic_types::Type::Void, |t| {
            registry.canonicalize_type(&yukic_types::convert::ast_type_to_type(t))
        })
}

pub fn cfun_param_types(
    cfun: &yukic_ast::item::CfunDecl,
    registry: &crate::registry::TypeRegistry,
) -> Vec<yukic_types::Type> {
    cfun.params
        .iter()
        .map(|p| registry.canonicalize_type(&yukic_types::convert::ast_type_to_type(&p.ty)))
        .collect()
}

pub fn cfun_return_type(
    cfun: &yukic_ast::item::CfunDecl,
    registry: &crate::registry::TypeRegistry,
) -> yukic_types::Type {
    cfun.return_type
        .as_ref()
        .map_or(yukic_types::Type::Void, |t| {
            registry.canonicalize_type(&yukic_types::convert::ast_type_to_type(t))
        })
}

pub fn mangle_fun_decl(
    fun: &yukic_ast::item::FunDecl,
    registry: &crate::registry::TypeRegistry,
) -> String {
    let params = fun_param_types(fun, registry);
    let ret = fun_return_type(fun, registry);
    mangle_fun_symbol(&fun.name.name, &params, &ret)
}

pub fn mangle_cfun_decl(
    cfun: &yukic_ast::item::CfunDecl,
    registry: &crate::registry::TypeRegistry,
) -> String {
    let params = cfun_param_types(cfun, registry);
    let ret = cfun_return_type(cfun, registry);
    mangle_fun_symbol(&cfun.name.name, &params, &ret)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_ast::attr::{AttributeArgs, AttributeNode};
    use yukic_ast::ident::Ident;
    use yukic_span::Span;
    use yukic_types::Type;

    #[test]
    fn test_mangle_fun_symbol_empty_args() {
        assert_eq!(
            mangle_fun_symbol("get_count", &[], &Type::Int64),
            "__yukifun_get_count___int64"
        );
        assert_eq!(
            mangle_fun_symbol("run", &[], &Type::Void),
            "__yukifun_run___void"
        );
    }

    #[test]
    fn test_mangle_fun_symbol_with_args() {
        assert_eq!(
            mangle_fun_symbol("add", &[Type::Int32, Type::Int32], &Type::Int32),
            "__yukifun_add_int32_int32__int32"
        );
        assert_eq!(
            mangle_fun_symbol("print_msg", &[Type::String], &Type::Void),
            "__yukifun_print_msg_string__void"
        );
    }

    #[test]
    fn test_mangle_type_str_variants() {
        assert_eq!(mangle_type_str(&Type::Array(Box::new(Type::Int32))), "arr_int32");
        assert_eq!(mangle_type_str(&Type::nullable(Type::String)), "opt_string");
        assert_eq!(
            mangle_type_str(&Type::Ref {
                inner: Box::new(Type::Int32),
                mutable: true,
            }),
            "mutref_int32"
        );
    }

    #[test]
    fn test_should_mangle_rules() {
        let dummy = Span::DUMMY;
        assert!(!should_mangle_fun("main", &[]));
        assert!(should_mangle_fun("helper", &[]));

        let no_mangle_attr = AttributeNode::new_negated(Ident::new("mangle", dummy), AttributeArgs::None, dummy);
        assert!(!should_mangle_fun("helper", &[no_mangle_attr]));

        assert!(!should_mangle_cfun("c_helper", &[]));
        let mangle_attr = AttributeNode::new(Ident::new("mangle", dummy), AttributeArgs::None, dummy);
        assert!(should_mangle_cfun("c_helper", &[mangle_attr]));
    }
}
