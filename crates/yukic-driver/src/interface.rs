#![deny(unsafe_code)]

use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use yukic_ast::{
    AccessorKind, AttributeArgs, AttributeNode, CAttributeNode, CfunDecl, ClassDecl, CstructDecl,
    EnumDecl, EnumVariant, Expr, ExprKind, ExterncItem, FunDecl, ImportDecl, ImportPath,
    ImportPattern, InterfaceDecl, Item, ItemId, ItemKind, LiteralValue, MethodDecl, Module, Param,
    PathPrefix, PropertyAccessor, StaticDecl, StaticMethodDecl, StructDecl, ThisParam,
    TypeAliasDecl, Visibility,
};
use yukic_codegen::{TypeRegistry, itable_symbol_name, mangle_fun_decl, method_symbol_name, should_mangle_cfun, should_mangle_fun};
use yukic_diagnostics::Diagnostic;
use yukic_lexer::Lexer;
use yukic_parser::Parser;
use yukic_span::{SourceMap, Span};
use yukic_target::TargetTriple;

use crate::DriverConfig;
use crate::cli::OutputType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedExportEntry {
    pub name: String,
    pub mangled: String,
    pub signature: String,
    pub abi: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedExportManifest {
    pub format_version: u32,
    pub package_name: String,
    pub yukic_version: String,
    pub target: String,
    pub abi_hash: String,
    pub exports: Vec<SharedExportEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct YkniPackageMetadata {
    pub format_version: u32,
    pub yukic_version: String,
    pub target: Option<String>,
    pub abi_hash: String,
    pub package_name: Option<String>,
    pub package_version: Option<String>,
    pub package_kind: Option<String>,
    pub link_systems: Vec<String>,
    pub link_archives: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedInterfacePackage {
    pub path: PathBuf,
    pub meta: YkniPackageMetadata,
    pub modules: Vec<(String, Module)>,
}

pub struct InterfaceModuleInput<'a> {
    pub name: &'a str,
    pub ast: &'a Module,
    pub is_external: bool,
}

pub fn resolve_interface_path(config: &DriverConfig, out_path: &Path) -> Option<PathBuf> {
    if let Some(ref p) = config.emit_interface {
        return Some(p.clone());
    }
    if config.output_type == OutputType::Static || config.output_type == OutputType::Shared {
        let stem = out_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("lib");
        let clean_name = stem.strip_prefix("lib").unwrap_or(stem);
        return Some(out_path.with_file_name(format!("{clean_name}.ykni")));
    }
    None
}

pub fn generate_ykni_and_shared_metadata(
    modules: &[InterfaceModuleInput<'_>],
    combined_ast: &Module,
    source_map: &SourceMap,
    config: &DriverConfig,
    target: TargetTriple,
    pkg_name: &str,
    pkg_version: &str,
) -> (String, String) {
    let mut registry = TypeRegistry::new();
    for m in modules {
        registry.register_module(m.ast);
    }
    registry.register_module(combined_ast);
    registry.canonicalize_registered_types();

    let mut body = String::new();
    let mut exports = Vec::new();

    for m in modules.iter().filter(|m| !m.is_external) {
        let mod_text = format_module_namespace(m.name, m.ast, source_map, &registry, &mut exports);
        if !mod_text.trim().is_empty() {
            body.push_str(&mod_text);
            body.push('\n');
        }
    }

    let mut hasher = blake3::Hasher::new();
    hasher.update(target.as_str().as_bytes());
    hasher.update(b"|");
    hasher.update(crate::VERSION.as_bytes());
    hasher.update(b"|");
    hasher.update(body.as_bytes());
    let abi_hash = hasher.finalize().to_hex()[..16].to_string();

    let kind_str = match config.output_type {
        OutputType::Static => "static",
        OutputType::Shared => "shared",
        OutputType::App => "app",
    };

    let mut header = String::new();
    let _ = writeln!(header, "#![ykni::version(1)]");
    let _ = writeln!(
        header,
        "#![ykni::abi(yukic = \"{}\", target = \"{}\", abi_hash = \"{}\")]",
        escape_str(crate::VERSION),
        escape_str(target.as_str()),
        abi_hash
    );
    let _ = writeln!(
        header,
        "#![ykni::package(name = \"{}\", version = \"{}\", kind = \"{}\")]",
        escape_str(pkg_name),
        escape_str(pkg_version),
        kind_str
    );
    if !config.link_systems.is_empty() || !config.link_archives.is_empty() {
        let sys_list = config
            .link_systems
            .iter()
            .map(|s| format!("\"{}\"", escape_str(s)))
            .collect::<Vec<_>>()
            .join(", ");
        let arch_list = config
            .link_archives
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
            .filter(|n| !n.starts_with("libyukino_") && !n.starts_with("libstd") && !n.starts_with("libyukistd_"))
            .map(|n| format!("\"{}\"", escape_str(n)))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            header,
            "#![ykni::link(systems = [{sys_list}], archives = [{arch_list}])]"
        );
    }
    header.push('\n');
    header.push_str(&body);

    let manifest = SharedExportManifest {
        format_version: 1,
        package_name: pkg_name.to_string(),
        yukic_version: crate::VERSION.to_string(),
        target: target.as_str().to_string(),
        abi_hash,
        exports,
    };
    let metadata_json = serde_json::to_string(&manifest).unwrap_or_else(|_| "{}".to_string());

    (header, metadata_json)
}

pub fn parse_ykni_file(
    path: &Path,
    source_map: &mut SourceMap,
) -> Result<ParsedInterfacePackage, Vec<Diagnostic>> {
    let src = fs::read_to_string(path).map_err(|e| {
        vec![Diagnostic::error(
            yukic_diagnostics::code::ErrorCode::E0701,
            format!("failed to read interface file '{}': {e}", path.display()),
            Span::DUMMY,
        )]
    })?;
    let file_id = source_map.add_file(path, src);
    let source = source_map
        .get_file(file_id)
        .map_or("", |f| f.src.as_str())
        .to_string();
    let lexer = Lexer::new(file_id, &source);
    let (tokens, mut diags) = lexer.tokenize();
    let mut parser = Parser::with_file_id(tokens, file_id);
    let (parsed, mut p_diags) = parser.parse_interface_module();
    diags.append(&mut p_diags);
    if diags.iter().any(Diagnostic::is_error) {
        return Err(diags);
    }

    let meta = parse_metadata_attrs(&parsed.module_attrs);
    let fallback_pkg = meta.package_name.clone().unwrap_or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("prebuilt")
            .to_string()
    });

    let mut modules = Vec::new();
    let mut loose_items = Vec::new();
    for item in parsed.items {
        if let ItemKind::Namespace(ns) = item.kind {
            let mod_name = ns
                .path
                .iter()
                .map(|id| id.name.as_str())
                .collect::<Vec<_>>()
                .join(".");
            let mut m = Module::new(file_id);
            m.module_attrs = parsed.module_attrs.clone();
            m.items = renumber_items(ns.items);
            modules.push((mod_name, m));
        } else {
            loose_items.push(item);
        }
    }

    if !loose_items.is_empty() {
        let mut m = Module::new(file_id);
        m.module_attrs = parsed.module_attrs;
        m.items = renumber_items(loose_items);
        modules.push((fallback_pkg, m));
    }

    Ok(ParsedInterfacePackage {
        path: path.to_path_buf(),
        meta,
        modules,
    })
}

fn renumber_items(mut items: Vec<Item>) -> Vec<Item> {
    for (idx, it) in items.iter_mut().enumerate() {
        it.id = ItemId(u32::try_from(idx).unwrap_or(u32::MAX));
    }
    items
}

fn format_module_namespace(
    mod_name: &str,
    ast: &Module,
    source_map: &SourceMap,
    registry: &TypeRegistry,
    exports: &mut Vec<SharedExportEntry>,
) -> String {
    let mut inner = String::new();
    for item in &ast.items {
        if let ItemKind::Import(ref imp) = item.kind {
            if item.visibility == Visibility::Public {
                let _ = writeln!(inner, "    {}", format_import(imp));
            }
            continue;
        }
        if item.visibility != Visibility::Public {
            continue;
        }
        if let Some(chunk) = format_public_item(item, mod_name, source_map, registry, exports) {
            for line in chunk.lines() {
                if line.is_empty() {
                    inner.push('\n');
                } else {
                    let _ = writeln!(inner, "    {line}");
                }
            }
            inner.push('\n');
        }
    }
    if inner.trim().is_empty() {
        return String::new();
    }
    format!("namespace {mod_name} {{\n{inner}}}\n")
}

fn format_public_item(
    item: &Item,
    mod_name: &str,
    source_map: &SourceMap,
    registry: &TypeRegistry,
    exports: &mut Vec<SharedExportEntry>,
) -> Option<String> {
    match &item.kind {
        ItemKind::Static(st) => Some(format_static(st, source_map)),
        ItemKind::TypeAlias(ta) => Some(format_type_alias(ta)),
        ItemKind::Enum(en) => Some(format_enum(en, source_map)),
        ItemKind::Struct(st) => Some(format_struct(st, source_map, registry, exports)),
        ItemKind::Cstruct(cs) => Some(format_cstruct(cs)),
        ItemKind::Cvar(cv) => {
            exports.push(SharedExportEntry {
                name: cv.name.name.clone(),
                mangled: cv.name.name.clone(),
                signature: format!("cvar {}: {}", cv.name.name, cv.ty),
                abi: "cdecl".to_string(),
                kind: "cvar".to_string(),
            });
            Some(format_cvar(cv))
        }
        ItemKind::Cfun(cf) => Some(format_cfun(cf, registry, exports)),
        ItemKind::Externc(block) => {
            let mut out = String::new();
            for ext in &block.items {
                match ext {
                    ExterncItem::Cstruct(cs) => {
                        let _ = writeln!(out, "{}", format_cstruct(cs));
                    }
                    ExterncItem::Cvar(cv) => {
                        exports.push(SharedExportEntry {
                            name: cv.name.name.clone(),
                            mangled: cv.name.name.clone(),
                            signature: format!("cvar {}: {}", cv.name.name, cv.ty),
                            abi: "cdecl".to_string(),
                            kind: "cvar".to_string(),
                        });
                        let _ = writeln!(out, "{}", format_cvar(cv));
                    }
                    ExterncItem::Cfun(cf) => {
                        let _ = writeln!(out, "{}", format_cfun(cf, registry, exports));
                    }
                }
            }
            if out.is_empty() { None } else { Some(out.trim_end().to_string()) }
        }
        ItemKind::Interface(iface) => Some(format_interface(iface, source_map)),
        ItemKind::Fun(fun) => Some(format_fun(fun, mod_name, source_map, registry, exports)),
        ItemKind::Class(cls) => Some(format_class(cls, source_map, registry, exports)),
        _ => None,
    }
}

fn format_import(imp: &ImportDecl) -> String {
    let mut s = format!("public load {}", format_import_path(&imp.path));
    if let Some(ref clause) = imp.symbols {
        let items = clause
            .items
            .iter()
            .enumerate()
            .map(|(idx, it)| {
                let pat = format_import_pattern(&it.pattern);
                if let Some(alias) = clause.aliases.get(idx)
                    && alias.name != pat
                    && !it.pattern.is_wildcard()
                {
                    format!("{pat} as {}", alias.name)
                } else {
                    pat
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(s, ": {items}");
    }
    s
}

fn format_import_path(p: &ImportPath) -> String {
    let mut s = match &p.prefix {
        PathPrefix::Relative(0) => ".".to_string(),
        PathPrefix::Relative(n) => "..".repeat(*n),
        PathPrefix::Root => "root".to_string(),
        PathPrefix::Package(id) => id.name.clone(),
    };
    for (idx, seg) in p.segments.iter().enumerate() {
        let need_dot = match &p.prefix {
            PathPrefix::Relative(_) if idx == 0 => false,
            _ => true,
        };
        if need_dot {
            s.push('.');
        }
        s.push_str(&format_import_pattern(&seg.pattern));
        if seg.force_folder {
            s.push('!');
        }
    }
    s
}

fn format_import_pattern(pat: &ImportPattern) -> String {
    match pat {
        ImportPattern::Exact(id) => id.name.clone(),
        ImportPattern::All => "*".to_string(),
        ImportPattern::Prefix(id) => format!("{}*", id.name),
        ImportPattern::Suffix(id) => format!("*{}", id.name),
    }
}

fn format_static(st: &StaticDecl, source_map: &SourceMap) -> String {
    let ty_part = st.ty.as_ref().map_or(String::new(), |t| format!(": {t}"));
    format!("public static {}{ty_part} = {}", st.name.name, format_expr(&st.init, source_map))
}

fn format_type_alias(ta: &TypeAliasDecl) -> String {
    let gens = format_generics(&ta.generics);
    format!("public type {}{gens} = {}", ta.name.name, ta.target)
}

fn format_enum(en: &EnumDecl, source_map: &SourceMap) -> String {
    let mut out = format!("public enum {} {{\n", en.name.name);
    for v in &en.variants {
        match v {
            EnumVariant::Simple(id) => {
                let _ = writeln!(out, "    {},", id.name);
            }
            EnumVariant::WithValue(id, expr) => {
                let _ = writeln!(out, "    {} = {},", id.name, format_expr(expr, source_map));
            }
            EnumVariant::Adt(id, fields) => {
                let fs = fields
                    .iter()
                    .map(|f| format!("{}: {}", f.name.name, f.ty))
                    .collect::<Vec<_>>()
                    .join(", ");
                let _ = writeln!(out, "    {}({fs}),", id.name);
            }
        }
    }
    out.push('}');
    out
}

fn format_cstruct(cs: &CstructDecl) -> String {
    let mut out = String::new();
    let cattr = format_c_attrs(&cs.c_attrs);
    if !cattr.is_empty() {
        let _ = writeln!(out, "{cattr}");
    }
    let kw = if cs.is_union { "cunion" } else { "cstruct" };
    let _ = writeln!(out, "public {kw} {} {{", cs.name.name);
    for f in &cs.fields {
        let _ = writeln!(out, "    {}: {}", f.name.name, f.ty);
    }
    out.push('}');
    out
}

fn format_cvar(cv: &yukic_ast::CvarDecl) -> String {
    let cattr = format_c_attrs(&cv.c_attrs);
    if cattr.is_empty() {
        format!("public cvar {}: {};", cv.name.name, cv.ty)
    } else {
        format!("{cattr} public cvar {}: {};", cv.name.name, cv.ty)
    }
}

fn format_cfun(
    cf: &CfunDecl,
    registry: &TypeRegistry,
    exports: &mut Vec<SharedExportEntry>,
) -> String {
    let sym = if let Some(ref ov) = cf.symbol_override {
        ov.name.clone()
    } else if !should_mangle_cfun(&cf.name.name, &cf.yukino_attrs) {
        cf.name.name.clone()
    } else {
        yukic_codegen::mangle_cfun_decl(cf, registry)
    };
    let mut params_str = cf
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name.name, p.ty))
        .collect::<Vec<_>>();
    if cf.is_variadic {
        params_str.push("...".to_string());
    }
    let ret = cf.return_type.as_ref().map_or(String::new(), |r| format!(" -> {r}"));
    let sig_str = format!(
        "cfun({}){}",
        cf.params
            .iter()
            .map(|p| p.ty.to_string())
            .collect::<Vec<_>>()
            .join(", "),
        if ret.is_empty() { " -> void".to_string() } else { ret.clone() }
    );
    exports.push(SharedExportEntry {
        name: cf.name.name.clone(),
        mangled: sym.clone(),
        signature: sig_str,
        abi: "cdecl".to_string(),
        kind: "cfun".to_string(),
    });
    let mut out = format!("#[ykni::symbol(\"{}\")]\n", escape_str(&sym));
    let cattr = format_c_attrs(&cf.c_attrs);
    if !cattr.is_empty() {
        let _ = write!(out, "{cattr} ");
    }
    let as_clause = cf
        .symbol_override
        .as_ref()
        .map_or(String::new(), |ov| format!(" as {}", ov.name));
    let _ = write!(
        out,
        "public cfun {}({}){ret}{as_clause}",
        cf.name.name,
        params_str.join(", ")
    );
    out
}

fn format_fun(
    fun: &FunDecl,
    _mod_name: &str,
    source_map: &SourceMap,
    registry: &TypeRegistry,
    exports: &mut Vec<SharedExportEntry>,
) -> String {
    let mut out = String::new();
    let keep_body = !fun.generics.is_empty()
        || fun.attrs.iter().any(|a| a.is("inline") || a.is("inline_always"));
    for attr in &fun.attrs {
        if attr.is("ykni::symbol") || attr.is("ykni::inline_body") {
            continue;
        }
        let _ = writeln!(out, "{}", format_attr(attr, source_map));
    }

    let sym = if let Some(s) = extract_attr_str(&fun.attrs, "ykni::symbol") {
        s
    } else if !should_mangle_fun(&fun.name.name, &fun.attrs) {
        fun.name.name.clone()
    } else {
        mangle_fun_decl(fun, registry)
    };

    let sig_params = fun
        .params
        .iter()
        .map(format_param_type_sig)
        .collect::<Vec<_>>()
        .join(", ");
    let sig_ret = fun
        .return_type
        .as_ref()
        .map_or_else(|| "void".to_string(), ToString::to_string);
    exports.push(SharedExportEntry {
        name: fun.name.name.clone(),
        mangled: sym.clone(),
        signature: format!("fun({sig_params}) -> {sig_ret}"),
        abi: "yukino_fastcall".to_string(),
        kind: "fun".to_string(),
    });

    if !keep_body {
        let _ = writeln!(out, "#[ykni::symbol(\"{}\")]", escape_str(&sym));
    } else {
        let _ = writeln!(out, "#[ykni::inline_body]");
    }

    let gens = format_generics(&fun.generics);
    let params = format_params(&fun.params, source_map);
    let ret = fun
        .return_type
        .as_ref()
        .map_or(String::new(), |r| format!(" -> {r}"));

    if keep_body {
        if let Some(ref body) = fun.body
            && let Some(slice) = source_map.slice_span(body.span)
        {
            let _ = write!(out, "public fun {}{gens}({params}){ret} {slice}", fun.name.name);
            return out;
        }
    }

    let _ = write!(out, "public fun {}{gens}({params}){ret}", fun.name.name);
    out
}

fn format_struct(
    st: &StructDecl,
    source_map: &SourceMap,
    registry: &TypeRegistry,
    exports: &mut Vec<SharedExportEntry>,
) -> String {
    let mut out = format!("public struct {} {{\n", st.name.name);
    for f in &st.fields {
        let def = f
            .default_val
            .as_ref()
            .map_or(String::new(), |e| format!(" = {}", format_expr(e, source_map)));
        let _ = writeln!(out, "    {}: {}{def}", f.name.name, f.ty);
    }
    for m in &st.methods {
        let m_str = format_method(&st.name.name, m, source_map, registry, exports);
        for line in m_str.lines() {
            let _ = writeln!(out, "    {line}");
        }
    }
    for sm in &st.static_methods {
        let s_str = format_static_method(&st.name.name, sm, source_map, exports);
        for line in s_str.lines() {
            let _ = writeln!(out, "    {line}");
        }
    }
    out.push('}');
    out
}

fn format_interface(iface: &InterfaceDecl, source_map: &SourceMap) -> String {
    let gens = format_generics(&iface.generics);
    let mut out = format!("public interface {}{gens} {{\n", iface.name.name);
    for m in &iface.methods {
        let params = format_params(&m.params, source_map);
        let ret = m
            .return_type
            .as_ref()
            .map_or(String::new(), |r| format!(" -> {r}"));
        let _ = writeln!(out, "    fun {}({params}){ret}", m.name.name);
    }
    out.push('}');
    out
}

fn format_class(
    cls: &ClassDecl,
    source_map: &SourceMap,
    registry: &TypeRegistry,
    exports: &mut Vec<SharedExportEntry>,
) -> String {
    let mut out = String::new();
    let vtable_sym = format!("_Y_VTABLE_{}", cls.name.name);
    let _ = writeln!(out, "#[ykni::vtable(\"{vtable_sym}\")]");
    for iface in &cls.interfaces {
        let it_sym = itable_symbol_name(&cls.name.name, &iface.name);
        let _ = writeln!(
            out,
            "#[ykni::itable(\"{}\", \"{}\")]",
            escape_str(&iface.name),
            escape_str(&it_sym)
        );
    }
    let locked = if cls.is_locked { "locked " } else { "" };
    let base = cls
        .base_class
        .as_ref()
        .map_or(String::new(), |b| format!(" extends {}", b.name));
    let with_ifaces = if cls.interfaces.is_empty() {
        String::new()
    } else {
        let list = cls
            .interfaces
            .iter()
            .map(|i| i.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        format!(" with {list}")
    };
    let _ = writeln!(
        out,
        "public {locked}class {}{base}{with_ifaces} {{",
        cls.name.name
    );
    for f in &cls.fields {
        let pub_kw = if f.is_pub { "public " } else { "" };
        let static_kw = if f.is_static { "static " } else { "" };
        let def = f
            .default_val
            .as_ref()
            .map_or(String::new(), |e| format!(" = {}", format_expr(e, source_map)));
        let _ = writeln!(out, "    {pub_kw}{static_kw}{}: {}{def}", f.name.name, f.ty);
    }
    for m in &cls.methods {
        let m_str = format_method(&cls.name.name, m, source_map, registry, exports);
        for line in m_str.lines() {
            let _ = writeln!(out, "    {line}");
        }
    }
    for acc in &cls.accessors {
        let a_str = format_accessor(&cls.name.name, acc, exports);
        for line in a_str.lines() {
            let _ = writeln!(out, "    {line}");
        }
    }
    out.push('}');
    out
}

fn format_method(
    owner: &str,
    method: &MethodDecl,
    source_map: &SourceMap,
    _registry: &TypeRegistry,
    exports: &mut Vec<SharedExportEntry>,
) -> String {
    let sym = extract_attr_str(&method.attrs, "ykni::symbol")
        .unwrap_or_else(|| method_symbol_name(owner, &method.name.name, method.params.len()));
    let sig_params = method
        .params
        .iter()
        .map(format_param_type_sig)
        .collect::<Vec<_>>()
        .join(", ");
    let sig_ret = method
        .return_type
        .as_ref()
        .map_or_else(|| "void".to_string(), ToString::to_string);
    exports.push(SharedExportEntry {
        name: format!("{owner}::{}", method.name.name),
        mangled: sym.clone(),
        signature: format!("fun({sig_params}) -> {sig_ret}"),
        abi: "yukino_fastcall".to_string(),
        kind: "method".to_string(),
    });

    let mut out = format!("#[ykni::symbol(\"{}\")]\n", escape_str(&sym));
    for attr in &method.attrs {
        if !attr.is("ykni::symbol") {
            let _ = writeln!(out, "{}", format_attr(attr, source_map));
        }
    }
    let this_prefix = match method.this_param {
        ThisParam::Ref => {
            if method.params.is_empty() {
                "ref this".to_string()
            } else {
                "ref this, ".to_string()
            }
        }
        ThisParam::Value | ThisParam::None => String::new(),
    };
    let params = format_params(&method.params, source_map);
    let ret = method
        .return_type
        .as_ref()
        .map_or(String::new(), |r| format!(" -> {r}"));
    if method.is_operator {
        let _ = write!(
            out,
            "public operator fun {}({this_prefix}{params}){ret}",
            method.name.name
        );
    } else {
        let _ = write!(
            out,
            "public fun {}({this_prefix}{params}){ret}",
            method.name.name
        );
    }
    out
}

fn format_static_method(
    owner: &str,
    method: &StaticMethodDecl,
    source_map: &SourceMap,
    exports: &mut Vec<SharedExportEntry>,
) -> String {
    let sym = method_symbol_name(owner, &method.name.name, method.params.len());
    let sig_params = method
        .params
        .iter()
        .map(format_param_type_sig)
        .collect::<Vec<_>>()
        .join(", ");
    let sig_ret = method
        .return_type
        .as_ref()
        .map_or_else(|| "void".to_string(), ToString::to_string);
    exports.push(SharedExportEntry {
        name: format!("{owner}::{}", method.name.name),
        mangled: sym.clone(),
        signature: format!("fun({sig_params}) -> {sig_ret}"),
        abi: "yukino_fastcall".to_string(),
        kind: "static_method".to_string(),
    });
    let mut out = format!("#[ykni::symbol(\"{}\")]\n", escape_str(&sym));
    let params = format_params(&method.params, source_map);
    let ret = method
        .return_type
        .as_ref()
        .map_or(String::new(), |r| format!(" -> {r}"));
    let _ = write!(out, "static fun {}({params}){ret}", method.name.name);
    out
}

fn format_accessor(
    owner: &str,
    acc: &PropertyAccessor,
    exports: &mut Vec<SharedExportEntry>,
) -> String {
    match &acc.kind {
        AccessorKind::Get => {
            let sym = format!("_Y_{owner}___get_{}", acc.name.name);
            exports.push(SharedExportEntry {
                name: format!("{owner}.get_{}", acc.name.name),
                mangled: sym.clone(),
                signature: format!("fun() -> {}", acc.ty),
                abi: "yukino_fastcall".to_string(),
                kind: "accessor".to_string(),
            });
            format!("#[ykni::symbol(\"{sym}\")]\npublic fun get {}() -> {}", acc.name.name, acc.ty)
        }
        AccessorKind::Set(param) => {
            let sym = format!("_Y_{owner}___set_{}", acc.name.name);
            exports.push(SharedExportEntry {
                name: format!("{owner}.set_{}", acc.name.name),
                mangled: sym.clone(),
                signature: format!("fun({}) -> void", acc.ty),
                abi: "yukino_fastcall".to_string(),
                kind: "accessor".to_string(),
            });
            format!(
                "#[ykni::symbol(\"{sym}\")]\npublic fun set {}({}: {})",
                acc.name.name, param.name, acc.ty
            )
        }
    }
}

fn format_generics(generics: &[yukic_ast::GenericParam]) -> String {
    if generics.is_empty() {
        return String::new();
    }
    let items = generics
        .iter()
        .map(|g| {
            if g.bounds.is_empty() {
                g.name.name.clone()
            } else {
                let bs = g.bounds.iter().map(ToString::to_string).collect::<Vec<_>>().join(" + ");
                format!("{}: {bs}", g.name.name)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("<{items}>")
}

fn format_params(params: &[Param], source_map: &SourceMap) -> String {
    params
        .iter()
        .map(|p| {
            let mut s = String::new();
            if p.is_variadic {
                s.push_str("...");
            }
            if p.is_ref {
                s.push_str("ref ");
            }
            if p.is_mut {
                s.push('~');
            }
            s.push_str(&p.name.name);
            if let Some(ref ty) = p.ty {
                let _ = write!(s, ": {ty}");
            }
            if let Some(ref def) = p.default_val {
                let _ = write!(s, " = {}", format_expr(def, source_map));
            }
            s
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_param_type_sig(p: &Param) -> String {
    let mut s = String::new();
    if p.is_ref {
        s.push_str("ref ");
    }
    if p.is_mut {
        s.push('~');
    }
    if let Some(ref ty) = p.ty {
        s.push_str(&ty.to_string());
    } else {
        s.push_str("void");
    }
    s
}

fn format_c_attrs(attrs: &[CAttributeNode]) -> String {
    if attrs.is_empty() {
        return String::new();
    }
    let parts = attrs
        .iter()
        .map(|a| {
            if let Some(ref arg) = a.arg {
                format!("{}=\"{}\"", a.name.name, escape_str(arg))
            } else {
                a.name.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("|{parts}|")
}

fn format_attr(attr: &AttributeNode, source_map: &SourceMap) -> String {
    if let Some(slice) = source_map.slice_span(attr.span)
        && slice.starts_with('#')
    {
        return slice.to_string();
    }
    let neg = if attr.negated { "!" } else { "" };
    let path = attr.path_str();
    match &attr.args {
        AttributeArgs::None => format!("#[{neg}{path}]"),
        AttributeArgs::Positional(args) => {
            let list = args
                .iter()
                .map(|a| format_expr(a, source_map))
                .collect::<Vec<_>>()
                .join(", ");
            format!("#[{neg}{path}({list})]")
        }
        AttributeArgs::Named(pairs) => {
            let list = pairs
                .iter()
                .map(|(k, v)| format!("{} = {}", k.name, format_expr(v, source_map)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("#[{neg}{path}({list})]")
        }
    }
}

fn format_expr(expr: &Expr, source_map: &SourceMap) -> String {
    if let Some(slice) = source_map.slice_span(expr.span)
        && !slice.is_empty()
    {
        return slice.to_string();
    }
    match &expr.kind {
        ExprKind::Literal(lit) => match lit {
            LiteralValue::Int(v) => v.to_string(),
            LiteralValue::UInt(v) => v.to_string(),
            LiteralValue::Float(v) => format!("{v}"),
            LiteralValue::String(s) => format!("\"{}\"", escape_str(s)),
            LiteralValue::Char(c) => format!("'{c}'"),
            LiteralValue::Bool(b) => b.to_string(),
            LiteralValue::Null => "null".to_string(),
            _ => "0".to_string(),
        },
        ExprKind::Ident(id) => id.name.clone(),
        ExprKind::ArrayInit(items) => {
            let inner = items
                .iter()
                .map(|e| format_expr(e, source_map))
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{inner}]")
        }
        _ => "null".to_string(),
    }
}

fn parse_metadata_attrs(attrs: &[AttributeNode]) -> YkniPackageMetadata {
    let mut meta = YkniPackageMetadata {
        format_version: 1,
        ..Default::default()
    };
    for attr in attrs {
        match attr.path_str().as_str() {
            "ykni::version" => {
                if let AttributeArgs::Positional(vals) = &attr.args
                    && let Some(first) = vals.first()
                    && let ExprKind::Literal(LiteralValue::Int(v)) = first.kind
                {
                    meta.format_version = u32::try_from(v).unwrap_or(1);
                }
            }
            "ykni::abi" => {
                for (k, v) in attr_named_args(attr) {
                    if let Some(s) = expr_as_string(v) {
                        match k {
                            "yukic" => meta.yukic_version = s,
                            "target" => meta.target = Some(s),
                            "abi_hash" => meta.abi_hash = s,
                            _ => {}
                        }
                    }
                }
            }
            "ykni::package" => {
                for (k, v) in attr_named_args(attr) {
                    if let Some(s) = expr_as_string(v) {
                        match k {
                            "name" => meta.package_name = Some(s),
                            "version" => meta.package_version = Some(s),
                            "kind" => meta.package_kind = Some(s),
                            _ => {}
                        }
                    }
                }
            }
            "ykni::link" => {
                for (k, v) in attr_named_args(attr) {
                    if let ExprKind::ArrayInit(elems) = &v.kind {
                        let strs: Vec<String> = elems.iter().filter_map(expr_as_string).collect();
                        match k {
                            "systems" => meta.link_systems.extend(strs),
                            "archives" => meta.link_archives.extend(strs),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
    meta
}

fn attr_named_args(attr: &AttributeNode) -> Vec<(&str, &Expr)> {
    match &attr.args {
        AttributeArgs::Named(pairs) => pairs.iter().map(|(k, v)| (k.name.as_str(), v)).collect(),
        AttributeArgs::Positional(vals) => vals
            .iter()
            .filter_map(|v| match &v.kind {
                ExprKind::NamedArg(k, inner) => Some((k.name.as_str(), inner.as_ref())),
                _ => None,
            })
            .collect(),
        AttributeArgs::None => Vec::new(),
    }
}

fn expr_as_string(expr: &Expr) -> Option<String> {
    if let ExprKind::Literal(LiteralValue::String(s)) = &expr.kind {
        Some(s.clone())
    } else {
        None
    }
}

fn extract_attr_str(attrs: &[AttributeNode], path: &str) -> Option<String> {
    for attr in attrs {
        if attr.is(path)
            && let AttributeArgs::Positional(vals) = &attr.args
            && let Some(first) = vals.first()
            && let Some(s) = expr_as_string(first)
        {
            return Some(s);
        }
    }
    None
}

fn escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_ast::{Block, CParam, Ident, TypeNode};
    use yukic_span::{BytePos, FileId};

    #[test]
    fn test_ykni_roundtrip_generation_and_parsing() {
        let sp = Span::new(FileId(0), BytePos(0), BytePos(0));
        let mut module = Module::new(FileId(0));
        module.items.push(Item::new(
            ItemId(0),
            Visibility::Public,
            ItemKind::Fun(FunDecl {
                name: Ident::new("add_nums", sp),
                attrs: Vec::new(),
                generics: Vec::new(),
                params: vec![
                    Param {
                        name: Ident::new("a", sp),
                        is_ref: false,
                        is_mut: false,
                        is_variadic: false,
                        ty: Some(TypeNode::new(yukic_ast::TypeNodeKind::Ident(Ident::new("int32", sp)), sp)),
                        default_val: None,
                        span: sp,
                    },
                    Param {
                        name: Ident::new("b", sp),
                        is_ref: false,
                        is_mut: false,
                        is_variadic: false,
                        ty: Some(TypeNode::new(yukic_ast::TypeNodeKind::Ident(Ident::new("int32", sp)), sp)),
                        default_val: Some(Expr::new(ExprKind::Literal(LiteralValue::Int(1)), sp)),
                        span: sp,
                    },
                ],
                return_type: Some(TypeNode::new(yukic_ast::TypeNodeKind::Ident(Ident::new("int32", sp)), sp)),
                body: Some(Block::new(Vec::new(), sp)),
                span: sp,
            }),
            sp,
        ));
        module.items.push(Item::new(
            ItemId(1),
            Visibility::Public,
            ItemKind::Cfun(CfunDecl {
                name: Ident::new("native_ping", sp),
                c_attrs: Vec::new(),
                yukino_attrs: Vec::new(),
                params: vec![CParam {
                    name: Ident::new("x", sp),
                    ty: TypeNode::new(yukic_ast::TypeNodeKind::Ident(Ident::new("c_int", sp)), sp),
                    span: sp,
                }],
                is_variadic: false,
                return_type: Some(TypeNode::new(yukic_ast::TypeNodeKind::Ident(Ident::new("c_int", sp)), sp)),
                symbol_override: None,
                body: None,
                span: sp,
            }),
            sp,
        ));

        let mut cfg = DriverConfig::new("lib.ykn".into()).with_output_type(OutputType::Shared);
        cfg.link_systems.push("m".to_string());
        let sm = SourceMap::new();
        let (ykni_text, meta_json) = generate_ykni_and_shared_metadata(
            &[InterfaceModuleInput {
                name: "mathlib",
                ast: &module,
                is_external: false,
            }],
            &module,
            &sm,
            &cfg,
            TargetTriple::host(),
            "mathlib",
            "0.2.0",
        );

        assert!(ykni_text.contains("#![ykni::version(1)]"));
        assert!(ykni_text.contains("namespace mathlib"));
        assert!(ykni_text.contains("#[ykni::symbol(\"__yukifun_add_nums_int32_int32__int32\")]"));
        assert!(ykni_text.contains("public fun add_nums(a: int32, b: int32 = 1) -> int32"));

        let manifest: SharedExportManifest = serde_json::from_str(&meta_json).expect("valid metadata json");
        assert_eq!(manifest.package_name, "mathlib");
        assert_eq!(manifest.exports.len(), 2);
        assert_eq!(manifest.exports[0].abi, "yukino_fastcall");
        assert_eq!(manifest.exports[1].abi, "cdecl");

        let tmp = std::env::temp_dir().join(format!("yukino_ykni_roundtrip_{}.ykni", std::process::id()));
        fs::write(&tmp, &ykni_text).unwrap();
        let mut sm2 = SourceMap::new();
        let parsed = parse_ykni_file(&tmp, &mut sm2).expect("parse generated ykni");
        let _ = fs::remove_file(&tmp);

        assert_eq!(parsed.meta.package_name.as_deref(), Some("mathlib"));
        assert_eq!(parsed.meta.target.as_deref(), Some(TargetTriple::host().as_str()));
        assert_eq!(parsed.meta.link_systems, vec!["m".to_string()]);
        assert_eq!(parsed.modules.len(), 1);
        assert_eq!(parsed.modules[0].0, "mathlib");
        assert_eq!(parsed.modules[0].1.items.len(), 2);
    }
}
