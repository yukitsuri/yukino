#![deny(unsafe_code)]

use yukic_ast::types::{TypeNode, TypeNodeKind};

use crate::types::{CIntKind, Type, TypeParam};

pub fn ast_type_to_c_field_type(node: &TypeNode) -> Type {
    match &node.kind {
        TypeNodeKind::Array(elem, Some(len)) => {
            Type::FixedArray(Box::new(ast_type_to_c_field_type(elem)), *len)
        }
        TypeNodeKind::Pointer(inner) => Type::Ptr(Box::new(ast_type_to_c_field_type(inner))),
        _ => ast_type_to_type(node),
    }
}

pub fn ast_type_to_type(node: &TypeNode) -> Type {
    match &node.kind {
        TypeNodeKind::Ident(ident) => parse_primitive_or_named(&ident.name),
        TypeNodeKind::Path(path) => {
            let full_name = path
                .iter()
                .map(|id| id.name.as_str())
                .collect::<Vec<_>>()
                .join("::");
            parse_primitive_or_named(&full_name)
        }
        TypeNodeKind::Pointer(inner) => Type::Ptr(Box::new(ast_type_to_type(inner))),
        TypeNodeKind::Ref(inner) => Type::Ref {
            inner: Box::new(ast_type_to_type(inner)),
            mutable: false,
        },
        TypeNodeKind::Array(elem, _) => Type::Array(Box::new(ast_type_to_type(elem))),
        TypeNodeKind::Map(k, v) => {
            Type::Map(Box::new(ast_type_to_type(k)), Box::new(ast_type_to_type(v)))
        }
        TypeNodeKind::Tuple(elems) => Type::Tuple(elems.iter().map(ast_type_to_type).collect()),
        TypeNodeKind::Fun(params, ret) => Type::Fn {
            params: params
                .iter()
                .map(|p| match &p.kind {
                    TypeNodeKind::Ref(inner) => TypeParam::new(ast_type_to_type(inner), true, false),
                    _ => TypeParam::value(ast_type_to_type(p)),
                })
                .collect(),
            ret: Box::new(ast_type_to_type(ret)),
            is_cfun: false,
        },
        TypeNodeKind::CFun(params, ret) => Type::Fn {
            params: params
                .iter()
                .map(|p| match &p.kind {
                    TypeNodeKind::Ref(inner) => TypeParam::new(ast_type_to_type(inner), true, false),
                    _ => TypeParam::value(ast_type_to_type(p)),
                })
                .collect(),
            ret: Box::new(ast_type_to_type(ret)),
            is_cfun: true,
        },
        TypeNodeKind::Optional(inner) => Type::nullable(ast_type_to_type(inner)),
        TypeNodeKind::Union(members) => Type::union(members.iter().map(ast_type_to_type)),
        TypeNodeKind::Generic(name, args) => convert_generic(&name.name, args),
        TypeNodeKind::Result(ok, err) => Type::union([ast_type_to_type(ok), ast_type_to_type(err)]),
    }
}

fn convert_generic(name: &str, args: &[TypeNode]) -> Type {
    match name {
        "ptr" if !args.is_empty() => Type::Ptr(Box::new(ast_type_to_type(&args[0]))),
        "shared" if !args.is_empty() => Type::Shared(Box::new(ast_type_to_type(&args[0]))),
        "weak" if !args.is_empty() => Type::Weak(Box::new(ast_type_to_type(&args[0]))),
        "map" if args.len() >= 2 => Type::Map(
            Box::new(ast_type_to_type(&args[0])),
            Box::new(ast_type_to_type(&args[1])),
        ),
        _ => {
            let converted_args: Vec<Type> = args.iter().map(ast_type_to_type).collect();
            let args_str = converted_args
                .iter()
                .map(|t| t.display_name())
                .collect::<Vec<_>>()
                .join(", ");
            Type::Struct(format!("{name}<{args_str}>"))
        }
    }
}

pub fn parse_type_generic_args(s: &str) -> (String, Vec<Type>) {
    if let Some(open) = s.find('<') {
        if s.ends_with('>') {
            let base = s[..open].trim().to_string();
            let inside = &s[open + 1..s.len() - 1];
            let mut args = Vec::new();
            let mut depth = 0;
            let mut current = String::new();
            for ch in inside.chars() {
                match ch {
                    '<' => {
                        depth += 1;
                        current.push(ch);
                    }
                    '>' => {
                        depth -= 1;
                        current.push(ch);
                    }
                    ',' if depth == 0 => {
                        let trimmed = current.trim();
                        if !trimmed.is_empty() {
                            args.push(parse_primitive_or_named(trimmed));
                        }
                        current.clear();
                    }
                    _ => current.push(ch),
                }
            }
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                args.push(parse_primitive_or_named(trimmed));
            }
            return (base, args);
        }
    }
    (s.to_string(), Vec::new())
}

fn find_matching_paren(s: &str) -> Option<usize> {
    let mut paren_depth = 0usize;
    let mut angle_depth = 0usize;
    let mut bracket_depth = 0usize;
    for (idx, ch) in s.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => {
                if paren_depth == 0 && angle_depth == 0 && bracket_depth == 0 {
                    return Some(idx);
                }
                paren_depth = paren_depth.saturating_sub(1);
            }
            '<' => angle_depth += 1,
            '>' => angle_depth = angle_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            _ => {}
        }
    }
    None
}

fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut paren_depth = 0usize;
    let mut angle_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut start = 0usize;
    for (idx, ch) in s.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '<' => angle_depth += 1,
            '>' => angle_depth = angle_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ',' if paren_depth == 0 && angle_depth == 0 && bracket_depth == 0 => {
                parts.push(&s[start..idx]);
                start = idx + 1;
            }
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
}

fn parse_fn_param_type_str(raw: &str) -> TypeParam {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("ref ") {
        let rest = rest.trim_start();
        if let Some(inner) = rest.strip_prefix('~') {
            return TypeParam::new(parse_primitive_or_named(inner.trim()), true, true);
        }
        return TypeParam::new(parse_primitive_or_named(rest), true, false);
    }
    TypeParam::value(parse_primitive_or_named(trimmed))
}

fn parse_function_type_str(name: &str) -> Option<Type> {
    let trimmed = name.trim();
    let is_cfun = trimmed.starts_with("cfun(");
    if !is_cfun && !trimmed.starts_with("fun(") {
        return None;
    }
    let rest = if is_cfun { &trimmed[5..] } else { &trimmed[4..] };
    let rparen = find_matching_paren(rest)?;
    let params_str = &rest[..rparen];
    let after_paren = rest[rparen + 1..].trim();
    let ret_str = if let Some(stripped) = after_paren.strip_prefix("->") {
        stripped.trim()
    } else if after_paren.is_empty() {
        "void"
    } else {
        return None;
    };
    let params = if params_str.trim().is_empty() {
        Vec::new()
    } else {
        split_top_level_commas(params_str)
            .into_iter()
            .map(parse_fn_param_type_str)
            .collect()
    };
    let ret = Box::new(parse_primitive_or_named(ret_str));
    Some(Type::Fn { params, ret, is_cfun })
}

pub fn parse_primitive_or_named(name: &str) -> Type {
    if let Some(fn_ty) = parse_function_type_str(name) {
        return fn_ty;
    }
    if name.starts_with("ptr<") && name.ends_with('>') {
        let inner = &name[4..name.len() - 1];
        return Type::Ptr(Box::new(parse_primitive_or_named(inner.trim())));
    }
    if name.ends_with("[]") {
        let inner = &name[..name.len() - 2];
        return Type::Array(Box::new(parse_primitive_or_named(inner.trim())));
    }
    match name {
        "int8" => Type::Int8,
        "int16" => Type::Int16,
        "int32" => Type::Int32,
        "int64" => Type::Int64,
        "uint8" => Type::Uint8,
        "uint16" => Type::Uint16,
        "uint32" => Type::Uint32,
        "uint64" => Type::Uint64,
        "float32" => Type::Float32,
        "float64" => Type::Float64,
        "bool" | "c_bool" => Type::Bool,
        "char" => Type::Char,
        "string" => Type::String,
        "bytes" => Type::Bytes,
        "void" | "c_void" => Type::Void,
        "never" => Type::Never,
        "null" => Type::Null,
        "dynlib" => Type::Dynlib,
        "Arena" | "arena" => Type::Arena,
        "c_char" => Type::CInt(CIntKind::Char),
        "c_schar" => Type::CInt(CIntKind::SChar),
        "c_uchar" => Type::CInt(CIntKind::UChar),
        "c_short" => Type::CInt(CIntKind::Short),
        "c_ushort" => Type::CInt(CIntKind::UShort),
        "c_int" => Type::CInt(CIntKind::Int),
        "c_uint" => Type::CInt(CIntKind::UInt),
        "c_long" => Type::CInt(CIntKind::Long),
        "c_ulong" => Type::CInt(CIntKind::ULong),
        "c_longlong" => Type::CInt(CIntKind::LongLong),
        "c_ulonglong" => Type::CInt(CIntKind::ULongLong),
        "c_size_t" => Type::CInt(CIntKind::SizeT),
        "c_ssize_t" => Type::CInt(CIntKind::SSizeT),
        _ => Type::Struct(name.to_string()),
    }
}
