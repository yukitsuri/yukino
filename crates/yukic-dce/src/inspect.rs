#![deny(unsafe_code)]

use std::fmt::Write;
use std::fs;
use std::path::Path;
use yukic_ast::{AttributeArgs, AttributeNode, ItemKind, Module};

pub fn dump_attributes(module: &Module, path: &Path, project_name: &str) -> std::io::Result<()> {
    let json = generate_attributes_json(module, project_name);
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, json)
}

pub fn generate_attributes_json(module: &Module, project_name: &str) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    let _ = writeln!(out, "  \"project\": \"{}\",", escape_json(project_name));
    out.push_str("  \"declarations\": [\n");

    let mut first_decl = true;

    append_declaration(
        &mut out,
        &mut first_decl,
        "module",
        "<module>",
        &module.module_attrs,
    );

    for item in &module.items {
        let (kind, name, attrs) = match &item.kind {
            ItemKind::Fun(d) => ("function", d.name.as_str(), &d.attrs),
            ItemKind::Class(d) => ("class", d.name.as_str(), &item.attrs),
            ItemKind::Struct(d) => ("struct", d.name.as_str(), &item.attrs),
            ItemKind::Cfun(d) => ("cfun", d.name.as_str(), &d.yukino_attrs),
            ItemKind::Cstruct(d) => ("cstruct", d.name.as_str(), &item.attrs),
            ItemKind::Interface(d) => ("interface", d.name.as_str(), &item.attrs),
            ItemKind::Enum(d) => ("enum", d.name.as_str(), &item.attrs),
            ItemKind::TypeAlias(d) => ("type_alias", d.name.as_str(), &item.attrs),
            ItemKind::Static(d) => ("static", d.name.as_str(), &item.attrs),
            ItemKind::Macro(d) => ("macro", d.name.as_str(), &item.attrs),
            ItemKind::Import(_) | ItemKind::Alias(_) | ItemKind::Externc(_) | ItemKind::Error => {
                continue;
            }
        };

        append_declaration(&mut out, &mut first_decl, kind, name, attrs);
        append_member_attributes(&mut out, &mut first_decl, item);
    }

    out.push_str("\n  ]\n}\n");
    out
}

fn append_member_attributes(out: &mut String, first_decl: &mut bool, item: &yukic_ast::Item) {
    match &item.kind {
        ItemKind::Class(decl) => {
            for field in &decl.fields {
                append_declaration(out, first_decl, "field", field.name.as_str(), &field.attrs);
            }
            for method in &decl.methods {
                append_declaration(
                    out,
                    first_decl,
                    "method",
                    method.name.as_str(),
                    &method.attrs,
                );
            }
            for accessor in &decl.accessors {
                append_declaration(
                    out,
                    first_decl,
                    "property_accessor",
                    accessor.name.as_str(),
                    &accessor.attrs,
                );
            }
        }
        ItemKind::Struct(decl) => {
            for field in &decl.fields {
                append_declaration(out, first_decl, "field", field.name.as_str(), &field.attrs);
            }
            for method in &decl.methods {
                append_declaration(
                    out,
                    first_decl,
                    "method",
                    method.name.as_str(),
                    &method.attrs,
                );
            }
            for method in &decl.static_methods {
                append_declaration(
                    out,
                    first_decl,
                    "static_method",
                    method.name.as_str(),
                    &method.attrs,
                );
            }
        }
        ItemKind::Cstruct(decl) => {
            for field in &decl.fields {
                append_declaration(out, first_decl, "field", field.name.as_str(), &field.attrs);
            }
        }
        ItemKind::Interface(decl) => {
            for method in &decl.methods {
                append_declaration(
                    out,
                    first_decl,
                    "method",
                    method.name.as_str(),
                    &method.attrs,
                );
            }
        }
        ItemKind::Enum(decl) => {
            for variant in &decl.variants {
                if let yukic_ast::EnumVariant::Adt(_, fields) = variant {
                    for field in fields {
                        append_declaration(
                            out,
                            first_decl,
                            "field",
                            field.name.as_str(),
                            &field.attrs,
                        );
                    }
                }
            }
        }
        _ => {}
    }
}

fn append_declaration(
    out: &mut String,
    first_decl: &mut bool,
    kind: &str,
    name: &str,
    attrs: &[AttributeNode],
) {
    if attrs.is_empty() { return; }
    if !*first_decl { out.push_str(",\n"); }
    *first_decl = false;

    out.push_str("    {\n");
    let _ = writeln!(out, "      \"kind\": \"{kind}\",");
    let _ = writeln!(out, "      \"name\": \"{}\",", escape_json(name));
    out.push_str("      \"attributes\": [\n");

    for (i, attr) in attrs.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        format_attribute(attr, out, "        ");
    }
    out.push_str("\n      ]\n    }");
}

fn format_attribute(attr: &AttributeNode, out: &mut String, indent: &str) {
    let _ = writeln!(out, "{indent}{{");
    let _ = writeln!(
        out,
        "{indent}  \"name\": \"{}\",",
        escape_json(&attr.path_str())
    );
    let _ = writeln!(out, "{indent}  \"negated\": {},", attr.negated);
    let _ = write!(out, "{indent}  \"args\": ");

    match &attr.args {
        AttributeArgs::None => out.push_str("null\n"),
        AttributeArgs::Positional(exprs) => {
            let _ = writeln!(out, "\"[{} positional args]\"", exprs.len());
        }
        AttributeArgs::Named(pairs) => {
            let _ = writeln!(out, "\"[{} named args]\"", pairs.len());
        }
    }
    let _ = write!(out, "{indent}}}");
}

fn escape_json(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}
