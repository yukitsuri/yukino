#![deny(unsafe_code)]

use std::fmt::Write;
use std::path::{Path, PathBuf};

use yukic_ast::attr::CAttributeNode;
use yukic_ast::item::{CfunDecl, CstructDecl, ExterncItem, ItemKind, Visibility};
use yukic_ast::types::{TypeNode, TypeNodeKind};
use yukic_ast::Module as AstModule;

use crate::DriverConfig;
use crate::cli::OutputType;

pub fn c_type_to_string(ty: &TypeNode) -> String {
    match &ty.kind {
        TypeNodeKind::Ident(id) => match id.name.as_str() {
            "int8" => "int8_t".to_string(),
            "int16" => "int16_t".to_string(),
            "int32" => "int32_t".to_string(),
            "int64" | "c_ssize_t" => "int64_t".to_string(),
            "uint8" => "uint8_t".to_string(),
            "uint16" => "uint16_t".to_string(),
            "uint32" => "uint32_t".to_string(),
            "uint64" => "uint64_t".to_string(),
            "c_char" => "char".to_string(),
            "c_schar" => "signed char".to_string(),
            "c_uchar" => "unsigned char".to_string(),
            "c_short" => "short".to_string(),
            "c_ushort" => "unsigned short".to_string(),
            "c_int" => "int".to_string(),
            "c_uint" => "unsigned int".to_string(),
            "c_long" => "long".to_string(),
            "c_ulong" => "unsigned long".to_string(),
            "c_longlong" => "long long".to_string(),
            "c_ulonglong" => "unsigned long long".to_string(),
            "c_size_t" => "size_t".to_string(),
            "c_float" | "float32" => "float".to_string(),
            "c_double" | "float64" => "double".to_string(),
            "bool" => "bool".to_string(),
            "void" => "void".to_string(),
            other => other.to_string(),
        },
        TypeNodeKind::Pointer(inner) => match &inner.kind {
            TypeNodeKind::Ident(id) if id.name == "void" => "void*".to_string(),
            _ => format!("{}*", c_type_to_string(inner)),
        },
        TypeNodeKind::Ref(inner) => format!("{}*", c_type_to_string(inner)),
        TypeNodeKind::Array(elem, Some(size)) => format!("{}[{}]", c_type_to_string(elem), size),
        TypeNodeKind::Array(elem, None) => format!("{}*", c_type_to_string(elem)),
        TypeNodeKind::Path(segments) => segments
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join("_"),
        _ => "void*".to_string(),
    }
}

fn format_c_field(name: &str, ty: &TypeNode) -> String {
    if let TypeNodeKind::Array(elem, Some(size)) = &ty.kind {
        format!("    {} {}[{}];", c_type_to_string(elem), name, size)
    } else {
        format!("    {} {};", c_type_to_string(ty), name)
    }
}

fn has_packed_attribute(attrs: &[CAttributeNode]) -> bool {
    attrs.iter().any(|a| a.name.name == "packed")
}

pub fn format_cstruct(cstruct: &CstructDecl) -> String {
    let mut out = String::new();
    let is_packed = has_packed_attribute(&cstruct.c_attrs);

    if is_packed {
        out.push_str("#pragma pack(push, 1)\n");
    }

    let _ = writeln!(out, "typedef struct {} {{", cstruct.name.name);
    for field in &cstruct.fields {
        let _ = writeln!(out, "{}", format_c_field(&field.name.name, &field.ty));
    }
    let _ = write!(out, "}} {};", cstruct.name.name);

    if is_packed {
        out.push_str("\n#pragma pack(pop)");
    }

    out
}

pub fn format_cfun(cfun: &CfunDecl) -> String {
    let fn_name = cfun
        .symbol_override
        .as_ref()
        .map_or(cfun.name.name.as_str(), |id| id.name.as_str());

    let ret = cfun
        .return_type
        .as_ref()
        .map_or_else(|| "void".to_string(), c_type_to_string);

    let params_str = if cfun.params.is_empty() {
        "void".to_string()
    } else {
        cfun.params
            .iter()
            .map(|p| {
                if let TypeNodeKind::Array(elem, Some(size)) = &p.ty.kind {
                    format!("{} {}[{}]", c_type_to_string(elem), p.name.name, size)
                } else {
                    format!("{} {}", c_type_to_string(&p.ty), p.name.name)
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    };

    format!("{ret} {fn_name}({params_str});")
}

pub fn clean_guard_name(name: &str) -> String {
    let mut guard = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            guard.push(ch.to_ascii_uppercase());
        } else {
            guard.push('_');
        }
    }
    let trimmed = guard.trim_matches('_');
    if trimmed.is_empty() {
        "YUKINO_GENERATED_H".to_string()
    } else {
        format!("YUKINO_{trimmed}_H")
    }
}

pub fn generate_c_header_from_ast(ast: &AstModule, lib_name: &str) -> String {
    let guard = clean_guard_name(lib_name);
    let mut exported_structs = Vec::new();
    let mut exported_funs = Vec::new();

    for item in &ast.items {
        if item.visibility != Visibility::Public {
            continue;
        }
        match &item.kind {
            ItemKind::Cstruct(cs) => exported_structs.push(cs),
            ItemKind::Cfun(cf) => exported_funs.push(cf),
            ItemKind::Externc(block) => {
                for ext in &block.items {
                    match ext {
                        ExterncItem::Cstruct(cs) => exported_structs.push(cs),
                        ExterncItem::Cfun(cf) => exported_funs.push(cf),
                    }
                }
            }
            _ => {}
        }
    }

    let mut out = String::new();
    let _ = write!(out, "#ifndef {guard}\n#define {guard}\n\n");
    out.push_str("#include <stdint.h>\n#include <stdbool.h>\n#include <stddef.h>\n\n");
    out.push_str("#ifdef __cplusplus\nextern \"C\" {\n#endif\n\n");

    if !exported_structs.is_empty() {
        for cs in exported_structs {
            out.push_str(&format_cstruct(cs));
            out.push_str("\n\n");
        }
    }

    if !exported_funs.is_empty() {
        for cf in exported_funs {
            out.push_str(&format_cfun(cf));
            out.push('\n');
        }
        out.push('\n');
    }

    out.push_str("#ifdef __cplusplus\n}\n#endif\n\n");
    let _ = writeln!(out, "#endif /* {guard} */");
    out
}

fn llvm_type_to_c(ty: &inkwell::types::BasicTypeEnum) -> String {
    use inkwell::types::BasicTypeEnum::{FloatType, IntType};
    match ty {
        IntType(it) => match it.get_bit_width() {
            1 => "bool".to_string(),
            8 => "int8_t".to_string(),
            16 => "int16_t".to_string(),
            32 => "int32_t".to_string(),
            _ => "int64_t".to_string(),
        },
        FloatType(_) => "double".to_string(),
        _ => "void*".to_string(),
    }
}

fn llvm_metadata_type_to_c(ty: &inkwell::types::BasicMetadataTypeEnum) -> String {
    use inkwell::types::BasicMetadataTypeEnum::{FloatType, IntType};
    match ty {
        IntType(it) => match it.get_bit_width() {
            1 => "bool".to_string(),
            8 => "int8_t".to_string(),
            16 => "int16_t".to_string(),
            32 => "int32_t".to_string(),
            _ => "int64_t".to_string(),
        },
        FloatType(_) => "double".to_string(),
        _ => "void*".to_string(),
    }
}

pub fn generate_c_header_from_llvm(module: &inkwell::module::Module, lib_name: &str) -> String {
    let guard = clean_guard_name(lib_name);
    let mut out = String::new();
    let _ = write!(out, "#ifndef {guard}\n#define {guard}\n\n");
    out.push_str("#include <stdint.h>\n#include <stdbool.h>\n#include <stddef.h>\n\n");
    out.push_str("#ifdef __cplusplus\nextern \"C\" {\n#endif\n\n");

    for f in module.get_functions() {
        let name = f.get_name().to_str().unwrap_or("");
        if name.starts_with("llvm.")
            || name.starts_with("_Y_")
            || name.starts_with("yukino_")
            || name == "main"
        {
            continue;
        }
        if f.get_linkage() == inkwell::module::Linkage::External {
            let fn_type = f.get_type();
            let ret = match fn_type.get_return_type() {
                Some(r) => llvm_type_to_c(&r),
                None => "void".to_string(),
            };
            let params = fn_type
                .get_param_types()
                .iter()
                .enumerate()
                .map(|(i, p)| format!("{} arg{i}", llvm_metadata_type_to_c(p)))
                .collect::<Vec<_>>();
            let params_str = if params.is_empty() {
                "void".to_string()
            } else {
                params.join(", ")
            };
            let _ = writeln!(out, "{ret} {name}({params_str});");
        }
    }

    out.push_str("\n#ifdef __cplusplus\n}\n#endif\n\n");
    let _ = writeln!(out, "#endif /* {guard} */");
    out
}

pub fn resolve_header_path(config: &DriverConfig, out_path: &Path) -> Option<PathBuf> {
    if let Some(header) = &config.emit_header {
        return Some(header.clone());
    }

    if config.output_type == OutputType::Static || config.output_type == OutputType::Shared {
        let stem = out_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("lib");
        let clean_name = stem.strip_prefix("lib").unwrap_or(stem);
        let header_name = format!("{clean_name}.h");
        return Some(out_path.with_file_name(header_name));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_ast::Ident;
    use yukic_ast::item::{CParam, FieldDef, Item, ItemId};
    use yukic_span::Span;

    fn make_ident(name: &str) -> Ident {
        Ident {
            name: name.to_string(),
            span: Span::DUMMY,
        }
    }

    fn make_type(name: &str) -> TypeNode {
        TypeNode::new(TypeNodeKind::Ident(make_ident(name)), Span::DUMMY)
    }

    #[test]
    fn test_c_type_to_string() {
        assert_eq!(c_type_to_string(&make_type("int32")), "int32_t");
        assert_eq!(c_type_to_string(&make_type("uint64")), "uint64_t");
        assert_eq!(c_type_to_string(&make_type("c_char")), "char");
        assert_eq!(c_type_to_string(&make_type("c_size_t")), "size_t");
        assert_eq!(c_type_to_string(&make_type("bool")), "bool");

        let ptr_i32 = TypeNode::new(
            TypeNodeKind::Pointer(Box::new(make_type("int32"))),
            Span::DUMMY,
        );
        assert_eq!(c_type_to_string(&ptr_i32), "int32_t*");

        let ptr_void = TypeNode::new(
            TypeNodeKind::Pointer(Box::new(make_type("void"))),
            Span::DUMMY,
        );
        assert_eq!(c_type_to_string(&ptr_void), "void*");
    }

    #[test]
    fn test_format_cstruct_normal_and_packed() {
        let cs = CstructDecl {
            name: make_ident("Point"),
            c_attrs: Vec::new(),
            fields: vec![
                FieldDef {
                    name: make_ident("x"),
                    attrs: Vec::new(),
                    ty: make_type("int32"),
                    default_val: None,
                    span: Span::DUMMY,
                },
                FieldDef {
                    name: make_ident("y"),
                    attrs: Vec::new(),
                    ty: make_type("int32"),
                    default_val: None,
                    span: Span::DUMMY,
                },
            ],
            span: Span::DUMMY,
        };

        let formatted = format_cstruct(&cs);
        assert!(formatted.contains("typedef struct Point {"));
        assert!(formatted.contains("    int32_t x;"));
        assert!(formatted.contains("    int32_t y;"));
        assert!(formatted.contains("} Point;"));
        assert!(!formatted.contains("#pragma pack"));

        let mut packed_cs = cs;
        packed_cs.c_attrs.push(CAttributeNode {
            name: make_ident("packed"),
            arg: None,
            span: Span::DUMMY,
        });
        let packed_formatted = format_cstruct(&packed_cs);
        assert!(packed_formatted.contains("#pragma pack(push, 1)"));
        assert!(packed_formatted.contains("#pragma pack(pop)"));
    }

    #[test]
    fn test_format_cfun() {
        let cf = CfunDecl {
            name: make_ident("calc"),
            c_attrs: Vec::new(),
            yukino_attrs: Vec::new(),
            params: vec![
                CParam {
                    name: make_ident("a"),
                    ty: make_type("int32"),
                    span: Span::DUMMY,
                },
                CParam {
                    name: make_ident("b"),
                    ty: make_type("int32"),
                    span: Span::DUMMY,
                },
            ],
            return_type: Some(make_type("int32")),
            symbol_override: None,
            body: None,
            span: Span::DUMMY,
        };

        assert_eq!(format_cfun(&cf), "int32_t calc(int32_t a, int32_t b);");

        let mut cf_alias = cf;
        cf_alias.symbol_override = Some(make_ident("yuki_calc"));
        assert_eq!(
            format_cfun(&cf_alias),
            "int32_t yuki_calc(int32_t a, int32_t b);"
        );
    }

    #[test]
    fn test_generate_c_header_from_ast() {
        let mut ast = AstModule::default();
        let cs = CstructDecl {
            name: make_ident("Vector2"),
            c_attrs: Vec::new(),
            fields: vec![
                FieldDef {
                    name: make_ident("x"),
                    attrs: Vec::new(),
                    ty: make_type("float64"),
                    default_val: None,
                    span: Span::DUMMY,
                },
                FieldDef {
                    name: make_ident("y"),
                    attrs: Vec::new(),
                    ty: make_type("float64"),
                    default_val: None,
                    span: Span::DUMMY,
                },
            ],
            span: Span::DUMMY,
        };
        ast.items.push(Item::new(
            ItemId(0),
            Visibility::Public,
            ItemKind::Cstruct(cs),
            Span::DUMMY,
        ));

        let cf = CfunDecl {
            name: make_ident("vec_add"),
            c_attrs: Vec::new(),
            yukino_attrs: Vec::new(),
            params: vec![
                CParam {
                    name: make_ident("a"),
                    ty: make_type("Vector2"),
                    span: Span::DUMMY,
                },
                CParam {
                    name: make_ident("b"),
                    ty: make_type("Vector2"),
                    span: Span::DUMMY,
                },
            ],
            return_type: Some(make_type("Vector2")),
            symbol_override: None,
            body: None,
            span: Span::DUMMY,
        };
        ast.items.push(Item::new(
            ItemId(1),
            Visibility::Public,
            ItemKind::Cfun(cf),
            Span::DUMMY,
        ));

        let header = generate_c_header_from_ast(&ast, "math");
        assert!(header.contains("#ifndef YUKINO_MATH_H"));
        assert!(header.contains("#define YUKINO_MATH_H"));
        assert!(header.contains("#include <stdint.h>"));
        assert!(header.contains("extern \"C\" {"));
        assert!(header.contains("typedef struct Vector2 {"));
        assert!(header.contains("Vector2 vec_add(Vector2 a, Vector2 b);"));
        assert!(header.contains("#endif /* YUKINO_MATH_H */"));
    }

    #[test]
    fn test_resolve_header_path() {
        let mut config = DriverConfig::new("src/lib.ykn".to_string());
        config.output_type = OutputType::Static;
        let out_a = Path::new("bin/libmath.a");
        assert_eq!(
            resolve_header_path(&config, out_a),
            Some(PathBuf::from("bin/math.h"))
        );

        config.output_type = OutputType::Shared;
        let out_so = Path::new("bin/libmath.so");
        assert_eq!(
            resolve_header_path(&config, out_so),
            Some(PathBuf::from("bin/math.h"))
        );

        config.output_type = OutputType::App;
        let out_app = Path::new("bin/app");
        assert_eq!(resolve_header_path(&config, out_app), None);

        config.emit_header = Some(PathBuf::from("include/custom.h"));
        assert_eq!(
            resolve_header_path(&config, out_app),
            Some(PathBuf::from("include/custom.h"))
        );
    }
}
