#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet};

use yukic_ast::{
    AttributeArgs, AttributeNode, AttributePath, BinaryOp, Expr, ExprKind, Item, ItemKind,
    LiteralValue, Module,
};
use yukic_target::{Architecture, TargetTriple};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalConfig {
    pub target: TargetTriple,
    pub features: HashSet<String>,
    pub is_test: bool,
}

impl ConditionalConfig {
    pub fn new(target: TargetTriple, features: impl IntoIterator<Item = String>) -> Self {
        Self {
            target,
            features: features.into_iter().collect(),
            is_test: false,
        }
    }

    pub fn with_test(mut self, is_test: bool) -> Self {
        self.is_test = is_test;
        self
    }
}

pub fn filter_module(module: &mut Module, config: &ConditionalConfig) -> Result<(), String> {
    desugar_module_cfg_attrs(module, config)?;

    if !attributes_match(&module.module_attrs, config)? {
        module.items.clear();
        return Ok(());
    }

    let mut active_items = Vec::with_capacity(module.items.len());
    for mut item in module.items.drain(..) {
        let attrs = item_attributes(&item);
        if attributes_match(attrs, config)? {
            filter_item_members(&mut item, config)?;
            active_items.push(item);
        }
    }
    validate_symbol_collisions(&active_items)?;
    validate_attributes(module, &active_items)?;
    module.items = active_items;
    Ok(())
}

fn desugar_module_cfg_attrs(module: &mut Module, config: &ConditionalConfig) -> Result<(), String> {
    desugar_attrs_vec(&mut module.module_attrs, config)?;
    for item in &mut module.items {
        desugar_item_cfg_attrs(item, config)?;
    }
    Ok(())
}

fn desugar_item_cfg_attrs(item: &mut Item, config: &ConditionalConfig) -> Result<(), String> {
    desugar_attrs_vec(&mut item.attrs, config)?;
    match &mut item.kind {
        ItemKind::Fun(decl) => {
            desugar_attrs_vec(&mut decl.attrs, config)?;
        }
        ItemKind::Cfun(decl) => {
            desugar_attrs_vec(&mut decl.yukino_attrs, config)?;
        }
        ItemKind::Class(decl) => {
            for field in &mut decl.fields {
                desugar_attrs_vec(&mut field.attrs, config)?;
            }
            for method in &mut decl.methods {
                desugar_attrs_vec(&mut method.attrs, config)?;
            }
            for accessor in &mut decl.accessors {
                desugar_attrs_vec(&mut accessor.attrs, config)?;
            }
        }
        ItemKind::Struct(decl) => {
            for field in &mut decl.fields {
                desugar_attrs_vec(&mut field.attrs, config)?;
            }
            for method in &mut decl.methods {
                desugar_attrs_vec(&mut method.attrs, config)?;
            }
            for method in &mut decl.static_methods {
                desugar_attrs_vec(&mut method.attrs, config)?;
            }
        }
        ItemKind::Cstruct(decl) => {
            for field in &mut decl.fields {
                desugar_attrs_vec(&mut field.attrs, config)?;
            }
        }
        ItemKind::Interface(decl) => {
            for method in &mut decl.methods {
                desugar_attrs_vec(&mut method.attrs, config)?;
            }
        }
        ItemKind::Externc(decl) => {
            for nested in &mut decl.items {
                if let yukic_ast::ExterncItem::Cfun(cfun) = nested {
                    desugar_attrs_vec(&mut cfun.yukino_attrs, config)?;
                }
            }
        }
        ItemKind::Enum(decl) => {
            for variant in &mut decl.variants {
                if let yukic_ast::EnumVariant::Adt(_, fields) = variant {
                    for field in fields {
                        desugar_attrs_vec(&mut field.attrs, config)?;
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn desugar_attrs_vec(attrs: &mut Vec<AttributeNode>, config: &ConditionalConfig) -> Result<(), String> {
    let mut desugared = Vec::with_capacity(attrs.len());
    for attr in attrs.drain(..) {
        if attr.is("cfg_attr") {
            let (cond_met, derived) = eval_cfg_attr(&attr, config)?;
            if cond_met {
                desugared.extend(derived);
            }
        } else {
            desugared.push(attr);
        }
    }
    *attrs = desugared;
    Ok(())
}

fn eval_cfg_attr(
    attr: &AttributeNode,
    config: &ConditionalConfig,
) -> Result<(bool, Vec<AttributeNode>), String> {
    let AttributeArgs::Positional(exprs) = &attr.args else {
        return Err("attribute 'cfg_attr' expects at least 2 arguments: condition and attribute".to_string());
    };
    if exprs.len() < 2 {
        return Err("attribute 'cfg_attr' expects at least 2 arguments: condition and attribute".to_string());
    }

    let cond_met = eval_cfg_expr(&exprs[0], config)?;
    if !cond_met {
        return Ok((false, Vec::new()));
    }

    let mut derived = Vec::with_capacity(exprs.len() - 1);
    for expr in &exprs[1..] {
        derived.push(expr_to_attribute_node(expr)?);
    }
    Ok((true, derived))
}

fn expr_to_attribute_node(expr: &Expr) -> Result<AttributeNode, String> {
    match &expr.kind {
        ExprKind::Ident(id) => Ok(AttributeNode::new(
            AttributePath::from_ident(id.clone()),
            AttributeArgs::None,
            expr.span,
        )),
        ExprKind::Path(segments) => Ok(AttributeNode::new(
            AttributePath::new(segments.clone(), expr.span),
            AttributeArgs::None,
            expr.span,
        )),
        ExprKind::Call(callee, args) => {
            let path = match &callee.kind {
                ExprKind::Ident(id) => AttributePath::from_ident(id.clone()),
                ExprKind::Path(segments) => AttributePath::new(segments.clone(), callee.span),
                _ => return Err("invalid attribute name inside cfg_attr".to_string()),
            };
            let attr_args = if args.is_empty() {
                AttributeArgs::None
            } else if args.iter().all(|a| matches!(a.kind, ExprKind::NamedArg(..))) {
                let mut named = Vec::new();
                for a in args {
                    if let ExprKind::NamedArg(id, val) = &a.kind {
                        named.push((id.clone(), *val.clone()));
                    }
                }
                AttributeArgs::Named(named)
            } else {
                AttributeArgs::Positional(args.clone())
            };
            Ok(AttributeNode::new(path, attr_args, expr.span))
        }
        _ => Err("invalid attribute syntax inside cfg_attr".to_string()),
    }
}

fn item_attributes(item: &Item) -> &[AttributeNode] {
    match &item.kind {
        ItemKind::Fun(decl) => &decl.attrs,
        ItemKind::Cfun(decl) => &decl.yukino_attrs,
        _ => &item.attrs,
    }
}

fn filter_item_members(item: &mut Item, config: &ConditionalConfig) -> Result<(), String> {
    match &mut item.kind {
        ItemKind::Class(decl) => {
            filter_members(&mut decl.fields, |field| &field.attrs, config)?;
            filter_members(&mut decl.methods, |method| &method.attrs, config)?;
            filter_members(&mut decl.accessors, |accessor| &accessor.attrs, config)?;
        }
        ItemKind::Struct(decl) => {
            filter_members(&mut decl.fields, |field| &field.attrs, config)?;
            filter_members(&mut decl.methods, |method| &method.attrs, config)?;
            filter_members(&mut decl.static_methods, |method| &method.attrs, config)?;
        }
        ItemKind::Cstruct(decl) => {
            filter_members(&mut decl.fields, |field| &field.attrs, config)?;
        }
        ItemKind::Interface(decl) => {
            filter_members(&mut decl.methods, |method| &method.attrs, config)?;
        }
        ItemKind::Externc(decl) => {
            filter_members(&mut decl.items, externc_item_attributes, config)?;
        }
        ItemKind::Enum(decl) => {
            for variant in &mut decl.variants {
                if let yukic_ast::EnumVariant::Adt(_, fields) = variant {
                    filter_members(fields, |field| &field.attrs, config)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn externc_item_attributes(item: &yukic_ast::ExterncItem) -> &[AttributeNode] {
    match item {
        yukic_ast::ExterncItem::Cfun(decl) => &decl.yukino_attrs,
        yukic_ast::ExterncItem::Cstruct(_) => &[],
    }
}

fn filter_members<T>(
    members: &mut Vec<T>,
    attributes: impl Fn(&T) -> &[AttributeNode],
    config: &ConditionalConfig,
) -> Result<(), String> {
    let mut active_members = Vec::with_capacity(members.len());
    for member in members.drain(..) {
        if attributes_match(attributes(&member), config)? {
            active_members.push(member);
        }
    }
    *members = active_members;
    Ok(())
}

fn attributes_match(attrs: &[AttributeNode], config: &ConditionalConfig) -> Result<bool, String> {
    for attr in attrs {
        let Some(matches) = condition_match(attr, config)? else {
            continue;
        };

        if matches == attr.negated { return Ok(false); }
    }
    Ok(true)
}

fn condition_match(
    attr: &AttributeNode,
    config: &ConditionalConfig,
) -> Result<Option<bool>, String> {
    match attr.path_str().as_str() {
        "cfg" => Ok(Some(eval_cfg_attribute(attr, config)?)),
        "target" => Ok(Some(target_matches(attr, config)?)),
        "feature" => Ok(Some(feature_matches(attr, config)?)),
        "arch" => Ok(Some(arch_matches(attr, config)?)),
        _ => Ok(None),
    }
}

pub fn eval_cfg_attribute(attr: &AttributeNode, config: &ConditionalConfig) -> Result<bool, String> {
    match &attr.args {
        AttributeArgs::None => Err("attribute 'cfg' expects arguments".to_string()),
        AttributeArgs::Named(pairs) => {
            for (key, val) in pairs {
                if !eval_cfg_predicate(key.as_str(), val, config)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        AttributeArgs::Positional(exprs) => {
            for expr in exprs {
                if !eval_cfg_expr(expr, config)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}

pub fn eval_cfg_expr(expr: &Expr, config: &ConditionalConfig) -> Result<bool, String> {
    match &expr.kind {
        ExprKind::NamedArg(name, value) => {
            eval_cfg_predicate(name.as_str(), value, config)
        }
        ExprKind::Binary(BinaryOp::Eq, lhs, rhs) => {
            if let ExprKind::Ident(name) = &lhs.kind {
                eval_cfg_predicate(name.as_str(), rhs, config)
            } else {
                Err("cfg predicate expects identifier on left-hand side".to_string())
            }
        }
        ExprKind::Call(callee, args) => {
            let callee_name = match &callee.kind {
                ExprKind::Ident(id) => id.as_str(),
                _ => return Err("invalid cfg combinator".to_string()),
            };
            match callee_name {
                "all" => {
                    for arg in args {
                        if !eval_cfg_expr(arg, config)? {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                }
                "any" => {
                    if args.is_empty() {
                        return Ok(false);
                    }
                    for arg in args {
                        if eval_cfg_expr(arg, config)? {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
                "not" => {
                    if args.len() != 1 {
                        return Err("cfg not(...) expects exactly 1 argument".to_string());
                    }
                    Ok(!eval_cfg_expr(&args[0], config)?)
                }
                "feature" | "target_os" | "target_arch" | "target_env" | "target" => {
                    if args.len() != 1 {
                        return Err(format!("cfg {callee_name}(...) expects exactly 1 argument"));
                    }
                    eval_cfg_predicate(callee_name, &args[0], config)
                }
                other => Err(format!("unknown cfg combinator '{other}'")),
            }
        }
        ExprKind::Ident(id) => {
            match id.as_str() {
                "test" => Ok(config.is_test),
                "debug_assertions" => Ok(true),
                other => Ok(config.features.contains(other)),
            }
        }
        _ => Err("invalid cfg expression".to_string()),
    }
}

fn eval_cfg_predicate(key: &str, value: &Expr, config: &ConditionalConfig) -> Result<bool, String> {
    let s = string_literal(value).map_err(|()| {
        format!("cfg predicate '{key}' expects a string literal value")
    })?;

    let spec = config.target.spec();
    match key {
        "feature" => Ok(config.features.contains(&s)),
        "target_os" => {
            let os_str = match spec.os {
                yukic_target::OperatingSystem::Linux => "linux",
                yukic_target::OperatingSystem::Windows => "windows",
                yukic_target::OperatingSystem::Android => "android",
            };
            Ok(os_str == s)
        }
        "target_arch" => {
            let matched = match spec.arch {
                yukic_target::Architecture::X86_64 => s == "x86_64",
                yukic_target::Architecture::Aarch64 => s == "arm64" || s == "aarch64",
                yukic_target::Architecture::Armv7 => s == "arm" || s == "armv7",
            };
            Ok(matched)
        }
        "target_env" => {
            let env_str = match spec.env {
                yukic_target::Environment::Gnu => "gnu",
                yukic_target::Environment::Msvc => "msvc",
                yukic_target::Environment::Musl => "musl",
                yukic_target::Environment::Bionic => "bionic",
            };
            Ok(env_str == s)
        }
        "target_pointer_width" => {
            Ok(spec.pointer_width.to_string() == s)
        }
        "target" => {
            Ok(config.target.as_str() == s)
        }
        other => Err(format!("unknown cfg key '{other}'")),
    }
}

fn validate_attributes(module: &Module, items: &[Item]) -> Result<(), String> {
    for attr in &module.module_attrs {
        validate_attribute(attr, AttributeUse::Module)?;
    }
    for item in items {
        let usage = attribute_use(item);
        for attr in item_attributes(item) {
            validate_attribute(attr, usage)?;
        }
        validate_member_attributes(item)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum AttributeUse {
    Fun { is_cfun: bool, has_body: bool },
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Field,
    Module,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeCategory {
    Builtin,
    Config,
    Tool(String),
    Mutation,
    InertOrMacro,
    Unknown,
}

#[derive(Debug, Clone, Default)]
pub struct InertAttributeRegistry {
    known_namespaces: HashSet<String>,
    known_attributes: HashSet<String>,
}

impl InertAttributeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtin_defaults() -> Self {
        let mut reg = Self::default();
        reg.register_namespace("json");
        reg.register_namespace("serde");
        reg.register_namespace("toml");
        reg.register_namespace("yaml");
        reg.register_namespace("db");
        reg.register_namespace("sql");
        reg.register_namespace("http");
        reg.register_namespace("route");
        reg.register_namespace("cli");
        reg
    }

    pub fn register_namespace(&mut self, ns: impl Into<String>) {
        self.known_namespaces.insert(ns.into());
    }

    pub fn register_attribute(&mut self, attr: impl Into<String>) {
        self.known_attributes.insert(attr.into());
    }

    pub fn is_inert(&self, path: &AttributePath) -> bool {
        if self.known_attributes.contains(&path.as_str()) {
            return true;
        }
        if let Some(first) = path.first() {
            if self.known_namespaces.contains(first.as_str()) {
                return true;
            }
        }
        false
    }
}

pub fn classify_attribute(attr: &AttributeNode) -> AttributeCategory {
    if attr.path.segments.len() > 1 {
        if attr.path.starts_with("lint") {
            AttributeCategory::Tool("lint".to_string())
        } else {
            AttributeCategory::InertOrMacro
        }
    } else {
        let name = attr.path_str();
        match name.as_str() {
            "target" | "feature" | "arch" | "cfg" | "cfg_attr" => AttributeCategory::Config,
            "mutation" => AttributeCategory::Mutation,
            "mangle" | "stable" | "inline" | "no_inline" | "cold" | "noreturn"
            | "inline_always" | "inline_never" | "test" | "should_panic" | "ignore"
            | "timeout_ms" | "deprecated" | "ignore_warn" | "warns_ignore" | "thunk" => {
                AttributeCategory::Builtin
            }
            _ => AttributeCategory::Unknown,
        }
    }
}

fn attribute_use(item: &Item) -> AttributeUse {
    match &item.kind {
        ItemKind::Fun(_) => AttributeUse::Fun {
            is_cfun: false,
            has_body: true,
        },
        ItemKind::Cfun(decl) => AttributeUse::Fun {
            is_cfun: true,
            has_body: decl.body.is_some(),
        },
        ItemKind::Class(_) => AttributeUse::Class,
        ItemKind::Struct(_) => AttributeUse::Struct,
        ItemKind::Enum(_) => AttributeUse::Enum,
        ItemKind::Interface(_) => AttributeUse::Interface,
        _ => AttributeUse::Other,
    }
}

fn validate_member_attributes(item: &Item) -> Result<(), String> {
    match &item.kind {
        ItemKind::Class(decl) => {
            validate_member_list(&decl.fields, |field| &field.attrs, AttributeUse::Field)?;
            validate_member_list(&decl.methods, |method| &method.attrs, AttributeUse::Method)?;
            validate_member_list(
                &decl.accessors,
                |accessor| &accessor.attrs,
                AttributeUse::Method,
            )?;
        }
        ItemKind::Struct(decl) => {
            validate_member_list(&decl.fields, |field| &field.attrs, AttributeUse::Field)?;
            validate_member_list(&decl.methods, |method| &method.attrs, AttributeUse::Method)?;
            validate_member_list(
                &decl.static_methods,
                |method| &method.attrs,
                AttributeUse::Method,
            )?;
        }
        ItemKind::Cstruct(decl) => {
            validate_member_list(&decl.fields, |field| &field.attrs, AttributeUse::Field)?;
        }
        ItemKind::Interface(decl) => {
            for method in &decl.methods {
                let usage = AttributeUse::Method;
                for attr in &method.attrs {
                    validate_attribute(attr, usage)?;
                }
            }
        }
        ItemKind::Externc(decl) => {
            for nested in &decl.items {
                if let yukic_ast::ExterncItem::Cfun(function) = nested {
                    for attr in &function.yukino_attrs {
                        validate_attribute(
                            attr,
                            AttributeUse::Fun {
                                is_cfun: true,
                                has_body: function.body.is_some(),
                            },
                        )?;
                    }
                }
            }
        }
        ItemKind::Enum(decl) => {
            for variant in &decl.variants {
                if let yukic_ast::EnumVariant::Adt(_, fields) = variant {
                    validate_member_list(fields, |field| &field.attrs, AttributeUse::Field)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_member_list<T>(
    members: &[T],
    attributes: impl Fn(&T) -> &[AttributeNode],
    usage: AttributeUse,
) -> Result<(), String> {
    for member in members {
        for attr in attributes(member) {
            validate_attribute(attr, usage)?;
        }
    }
    Ok(())
}

fn validate_attribute(
    attr: &AttributeNode,
    usage: AttributeUse,
) -> Result<(), String> {
    let name = attr.path_str();
    if matches!(name.as_str(), "target" | "feature" | "arch") {
        return Ok(());
    }
    if matches!(usage, AttributeUse::Module) {
        let is_tool = matches!(classify_attribute(attr), AttributeCategory::Tool(_));
        let allowed = is_tool
            || matches!(
                name.as_str(),
                "deprecated"
                    | "stable"
                    | "inline"
                    | "no_inline"
                    | "inline_never"
                    | "inline_always"
                    | "cold"
                    | "noreturn"
                    | "ignore_warn"
                    | "warns_ignore"
            );
        if !allowed {
            return Err(format!("attribute '{name}' is not allowed at module level"));
        }
    }
    if attr.negated
        && name != "mangle"
        && !matches!(name.as_str(), "target" | "feature" | "arch" | "cfg")
    {
        return Err(format!("attribute '!{name}' cannot be negated"));
    }

    match classify_attribute(attr) {
        AttributeCategory::Config => Ok(()),
        AttributeCategory::Mutation => validate_mutation_attribute(attr, usage),
        AttributeCategory::Tool(tool) if tool == "lint" => validate_lint_attribute(attr, usage),
        AttributeCategory::Tool(other) => Err(format!("unknown tool attribute '{other}'")),
        AttributeCategory::InertOrMacro => Ok(()),
        AttributeCategory::Builtin => validate_builtin_attribute(attr, usage),
        AttributeCategory::Unknown => Err(format!("unknown attribute '{name}'")),
    }
}

fn validate_mutation_attribute(attr: &AttributeNode, usage: AttributeUse) -> Result<(), String> {
    match usage {
        AttributeUse::Struct | AttributeUse::Class | AttributeUse::Enum => {
            if matches!(attr.args, AttributeArgs::None) {
                return Err("attribute 'mutation' expects at least one mutation identifier".to_string());
            }
            Ok(())
        }
        _ => Err("attribute 'mutation' is only allowed on struct, class, or enum".to_string()),
    }
}

fn validate_lint_attribute(attr: &AttributeNode, _usage: AttributeUse) -> Result<(), String> {
    match &attr.args {
        AttributeArgs::None => Ok(()),
        AttributeArgs::Positional(exprs) => {
            for expr in exprs {
                string_or_ident_literal(expr).map_err(|()| {
                    format!("attribute '{}' expects string or identifier lint names", attr.path_str())
                })?;
            }
            Ok(())
        }
        AttributeArgs::Named(_) => Err(format!(
            "attribute '{}' does not accept named arguments",
            attr.path_str()
        )),
    }
}

fn string_or_ident_literal(expr: &Expr) -> Result<String, ()> {
    match &expr.kind {
        ExprKind::Literal(LiteralValue::String(value)) => Ok(value.clone()),
        ExprKind::Ident(id) => Ok(id.name.clone()),
        _ => Err(()),
    }
}

fn validate_builtin_attribute(attr: &AttributeNode, usage: AttributeUse) -> Result<(), String> {
    let name = attr.path_str();
    if name == "mangle" {
        require_fun(&name, usage, true)?;
        require_no_args(attr)?;
        return Ok(());
    }

    match name.as_str() {
        "stable" => require_fun(&name, usage, false)?,
        "inline" => {
            if let AttributeUse::Fun {
                is_cfun: true,
                has_body: false,
            } = usage
            {
                return Err("attribute 'inline' requires a cfun body".to_string());
            }
            require_fun(&name, usage, true)?;
        }
        "no_inline" | "cold" | "noreturn" | "inline_always" | "inline_never" => {
            require_fun(&name, usage, false)?;
        }
        "thunk" => {
            require_fun(&name, usage, false)?;
            require_no_args(attr)?;
        }
        "test" => {
            require_top_level_fun(&name, usage)?;
            require_no_args(attr)?;
        }
        "should_panic" | "ignore" => {
            require_top_level_fun(&name, usage)?;
            optional_string(attr, &name)?;
        }
        "timeout_ms" => {
            require_top_level_fun(&name, usage)?;
            require_positive_integer(attr, &name)?;
        }
        "deprecated" => optional_string(attr, &name)?,
        "ignore_warn" | "warns_ignore" => {
            string_values(attr, &name).map(|_| ())?;
        }
        _ => return Err(format!("unknown attribute '{name}'")),
    }

    Ok(())
}

fn require_fun(name: &str, usage: AttributeUse, allow_cfun: bool) -> Result<(), String> {
    match usage {
        AttributeUse::Fun { is_cfun, .. } if allow_cfun || !is_cfun => Ok(()),
        AttributeUse::Method | AttributeUse::Module => Ok(()),
        _ => Err(format!("attribute '{name}' is only allowed on fun")),
    }
}

fn require_top_level_fun(name: &str, usage: AttributeUse) -> Result<(), String> {
    if matches!(usage, AttributeUse::Fun { is_cfun: false, .. }) {
        Ok(())
    } else {
        Err(format!("attribute '{name}' is only allowed on fun"))
    }
}

fn require_no_args(attr: &AttributeNode) -> Result<(), String> {
    if matches!(attr.args, AttributeArgs::None) {
        Ok(())
    } else {
        Err(format!(
            "attribute '{}' does not accept arguments",
            attr.path_str()
        ))
    }
}

fn optional_string(attr: &AttributeNode, name: &str) -> Result<(), String> {
    match &attr.args {
        AttributeArgs::None => Ok(()),
        AttributeArgs::Positional(values) if values.len() == 1 => string_literal(&values[0])
            .map(|_| ())
            .map_err(|()| format!("attribute '{name}' expects a string")),
        _ => Err(format!("attribute '{name}' expects an optional string")),
    }
}

fn require_positive_integer(attr: &AttributeNode, name: &str) -> Result<(), String> {
    let AttributeArgs::Positional(values) = &attr.args else {
        return Err(format!("attribute '{name}' expects a positive integer"));
    };
    if values.len() != 1 {
        return Err(format!("attribute '{name}' expects a positive integer"));
    }
    match values[0].kind {
        ExprKind::Literal(LiteralValue::Int(value)) if value > 0 => Ok(()),
        ExprKind::Literal(LiteralValue::UInt(value)) if value > 0 => Ok(()),
        _ => Err(format!("attribute '{name}' expects a positive integer")),
    }
}

fn target_matches(attr: &AttributeNode, config: &ConditionalConfig) -> Result<bool, String> {
    let values = string_values(attr, "target")?;
    for value in &values {
        TargetTriple::parse(value).map_err(|error| error.to_string())?;
    }
    Ok(values.iter().any(|value| value == config.target.as_str()))
}

fn feature_matches(attr: &AttributeNode, config: &ConditionalConfig) -> Result<bool, String> {
    let values = string_values(attr, "feature")?;
    Ok(values.iter().all(|value| config.features.contains(value)))
}

fn arch_matches(attr: &AttributeNode, config: &ConditionalConfig) -> Result<bool, String> {
    let values = string_values(attr, "arch")?;
    if values.len() != 1 {
        return Err(format!(
            "attribute '{}' expects one architecture",
            attr.path_str()
        ));
    }
    let expected = match values[0].as_str() {
        "x86_64" => Architecture::X86_64,
        "arm64" => Architecture::Aarch64,
        other => return Err(format!("unsupported target architecture '{other}'")),
    };
    Ok(config.target.spec().arch == expected)
}

fn string_values(attr: &AttributeNode, name: &str) -> Result<Vec<String>, String> {
    let expr = match &attr.args {
        AttributeArgs::Positional(values) if values.len() == 1 => &values[0],
        _ => {
            return Err(format!(
                "attribute '{name}' expects a string or an array of strings"
            ));
        }
    };

    match &expr.kind {
        ExprKind::Literal(LiteralValue::String(value)) => Ok(vec![value.clone()]),
        ExprKind::ArrayInit(values) => values
            .iter()
            .map(string_literal)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|()| format!("attribute '{name}' expects an array of strings")),
        _ => Err(format!(
            "attribute '{name}' expects a string or an array of strings"
        )),
    }
}

fn string_literal(expr: &Expr) -> Result<String, ()> {
    match &expr.kind {
        ExprKind::Literal(LiteralValue::String(value)) => Ok(value.clone()),
        _ => Err(()),
    }
}

fn validate_symbol_collisions(items: &[Item]) -> Result<(), String> {
    let mut names = HashMap::new();
    for item in items {
        let Some(name) = item_name(item) else { continue; };
        if let Some(previous_span) = names.insert(name.clone(), item.span) {
            return Err(format!(
                "E0304: name '{name}' is defined more than once (previous declaration at {previous_span:?})"
            ));
        }
    }
    Ok(())
}

fn item_name(item: &Item) -> Option<String> {
    match &item.kind {
        ItemKind::Fun(decl) => Some(decl.name.name.clone()),
        ItemKind::Cfun(decl) => Some(decl.name.name.clone()),
        ItemKind::Class(decl) => Some(decl.name.name.clone()),
        ItemKind::Struct(decl) => Some(decl.name.name.clone()),
        ItemKind::Cstruct(decl) => Some(decl.name.name.clone()),
        ItemKind::Interface(decl) => Some(decl.name.name.clone()),
        ItemKind::Enum(decl) => Some(decl.name.name.clone()),
        ItemKind::TypeAlias(decl) => Some(decl.name.name.clone()),
        ItemKind::Static(decl) => Some(decl.name.name.clone()),
        ItemKind::Macro(decl) => Some(decl.name.name.clone()),
        ItemKind::Import(_) | ItemKind::Alias(_) | ItemKind::Externc(_) | ItemKind::Error => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_ast::{Block, FunDecl, Ident, ItemId, Visibility};
    use yukic_span::{BytePos, FileId, Span};

    fn span() -> Span {
        Span::new(FileId(0), BytePos(0), BytePos(1))
    }

    fn attribute(name: &str, value: &str, negated: bool) -> AttributeNode {
        let expr = Expr::new(
            ExprKind::Literal(LiteralValue::String(value.to_string())),
            span(),
        );
        if negated {
            AttributeNode::new_negated(
                Ident::new(name, span()),
                AttributeArgs::Positional(vec![expr]),
                span(),
            )
        } else {
            AttributeNode::new(
                Ident::new(name, span()),
                AttributeArgs::Positional(vec![expr]),
                span(),
            )
        }
    }

    fn function(name: &str, attrs: Vec<AttributeNode>) -> Item {
        Item::new(
            ItemId(0),
            Visibility::Local,
            ItemKind::Fun(FunDecl {
                name: Ident::new(name, span()),
                attrs,
                generics: Vec::new(),
                params: Vec::new(),
                return_type: None,
                body: Block::new(Vec::new(), span()),
                span: span(),
            }),
            span(),
        )
    }

    #[test]
    fn filters_by_target_and_negation() {
        let target = TargetTriple::X86_64UnknownLinuxGnu;
        let config = ConditionalConfig::new(target, Vec::new());
        let mut module = Module::new(FileId(0));
        module.items.push(function(
            "linux",
            vec![attribute("target", target.as_str(), false)],
        ));
        module.items.push(function(
            "other",
            vec![attribute("target", target.as_str(), true)],
        ));

        filter_module(&mut module, &config).expect("conditions should be valid");

        assert_eq!(module.items.len(), 1);
        assert!(matches!(&module.items[0].kind, ItemKind::Fun(fun) if fun.name.name == "linux"));
    }

    #[test]
    fn allows_same_name_when_only_one_condition_matches() {
        let target = TargetTriple::X86_64UnknownLinuxGnu;
        let config = ConditionalConfig::new(target, Vec::new());
        let mut module = Module::new(FileId(0));
        module.items.push(function(
            "platform_fn",
            vec![attribute("target", target.as_str(), false)],
        ));
        module.items.push(function(
            "platform_fn",
            vec![attribute(
                "target",
                TargetTriple::X86_64PcWindowsGnu.as_str(),
                false,
            )],
        ));

        filter_module(&mut module, &config).expect("inactive collision should be removed");
        assert_eq!(module.items.len(), 1);
    }

    #[test]
    fn rejects_same_name_when_conditions_overlap() {
        let target = TargetTriple::X86_64UnknownLinuxGnu;
        let config = ConditionalConfig::new(target, Vec::new());
        let mut module = Module::new(FileId(0));
        module.items.push(function("duplicate", Vec::new()));
        module.items.push(function("duplicate", Vec::new()));

        let error = filter_module(&mut module, &config).expect_err("active collision must fail");
        assert!(error.contains("E0304"));
    }

    #[test]
    fn filters_by_feature_and_negation() {
        let target = TargetTriple::X86_64UnknownLinuxGnu;
        let config = ConditionalConfig::new(target, vec!["ssl".to_string()]);
        let mut module = Module::new(FileId(0));
        module.items.push(function(
            "secure",
            vec![attribute("feature", "ssl", false)],
        ));
        module.items.push(function(
            "plain",
            vec![attribute("feature", "ssl", true)],
        ));

        filter_module(&mut module, &config).expect("features should be valid");
        assert_eq!(module.items.len(), 1);
        assert!(matches!(&module.items[0].kind, ItemKind::Fun(fun) if fun.name.name == "secure"));
    }

    #[test]
    fn filters_by_arch_and_negation() {
        let target = TargetTriple::X86_64UnknownLinuxGnu;
        let config = ConditionalConfig::new(target, Vec::new());
        let mut module = Module::new(FileId(0));
        module.items.push(function("x86", vec![attribute("arch", "x86_64", false)]));
        module.items.push(function("arm", vec![attribute("arch", "arm64", false)]));

        filter_module(&mut module, &config).expect("arch should be valid");
        assert_eq!(module.items.len(), 1);
        assert!(matches!(&module.items[0].kind, ItemKind::Fun(fun) if fun.name.name == "x86"));
    }

    #[test]
    fn rejects_negating_non_conditional_attribute() {
        let target = TargetTriple::X86_64UnknownLinuxGnu;
        let config = ConditionalConfig::new(target, Vec::new());
        let mut module = Module::new(FileId(0));
        let attr = AttributeNode::new_negated(
            Ident::new("stable", span()),
            AttributeArgs::None,
            span(),
        );
        module.items.push(function("pure_fn", vec![attr]));

        let error = filter_module(&mut module, &config).expect_err("negated stable should fail");
        assert!(error.contains("cannot be negated"));
    }

    #[test]
    fn test_attribute_classification_and_inert_registry() {
        let span = span();
        let target_attr = AttributeNode::new(Ident::new("target", span), AttributeArgs::None, span);
        assert_eq!(classify_attribute(&target_attr), AttributeCategory::Config);

        let inline_attr = AttributeNode::new(Ident::new("inline", span), AttributeArgs::None, span);
        assert_eq!(classify_attribute(&inline_attr), AttributeCategory::Builtin);

        let mutation_attr = AttributeNode::new(Ident::new("mutation", span), AttributeArgs::None, span);
        assert_eq!(classify_attribute(&mutation_attr), AttributeCategory::Mutation);

        let lint_attr = AttributeNode::new(
            AttributePath::new(vec![Ident::new("lint", span), Ident::new("ignore", span)], span),
            AttributeArgs::None,
            span,
        );
        assert_eq!(classify_attribute(&lint_attr), AttributeCategory::Tool("lint".to_string()));

        let json_attr = AttributeNode::new(
            AttributePath::new(vec![Ident::new("json", span), Ident::new("rename", span)], span),
            AttributeArgs::None,
            span,
        );
        assert_eq!(classify_attribute(&json_attr), AttributeCategory::InertOrMacro);

        let unknown_attr = AttributeNode::new(Ident::new("custom_unknown", span), AttributeArgs::None, span);
        assert_eq!(classify_attribute(&unknown_attr), AttributeCategory::Unknown);

        let reg = InertAttributeRegistry::with_builtin_defaults();
        assert!(reg.is_inert(&json_attr.path));
    }

    #[test]
    fn allows_lint_and_inert_attributes_and_rejects_unknown() {
        let target = TargetTriple::X86_64UnknownLinuxGnu;
        let config = ConditionalConfig::new(target, Vec::new());
        let mut module = Module::new(FileId(0));
        let span = span();

        let lint_attr = AttributeNode::new(
            AttributePath::new(vec![Ident::new("lint", span), Ident::new("ignore", span)], span),
            AttributeArgs::Positional(vec![Expr::new(
                ExprKind::Literal(LiteralValue::String("dead_code".to_string())),
                span,
            )]),
            span,
        );
        let inert_attr = AttributeNode::new(
            AttributePath::new(vec![Ident::new("json", span), Ident::new("rename", span)], span),
            AttributeArgs::Positional(vec![Expr::new(
                ExprKind::Literal(LiteralValue::String("id".to_string())),
                span,
            )]),
            span,
        );

        module.items.push(function("valid_fn", vec![lint_attr, inert_attr]));
        filter_module(&mut module, &config).expect("lint and inert attributes should pass");

        let mut invalid_module = Module::new(FileId(0));
        let unknown = AttributeNode::new(Ident::new("unknown_attr", span), AttributeArgs::None, span);
        invalid_module.items.push(function("invalid_fn", vec![unknown]));
        let err = filter_module(&mut invalid_module, &config).expect_err("unknown attribute should fail");
        assert!(err.contains("unknown attribute 'unknown_attr'"));
    }

    #[test]
    fn validates_mutation_attribute_targets() {
        use yukic_ast::StructDecl;
        let target = TargetTriple::X86_64UnknownLinuxGnu;
        let config = ConditionalConfig::new(target, Vec::new());
        let span = span();

        let mut_attr = AttributeNode::new(
            Ident::new("mutation", span),
            AttributeArgs::Positional(vec![Expr::new(
                ExprKind::Ident(Ident::new("ToString", span)),
                span,
            )]),
            span,
        );

        let mut module = Module::new(FileId(0));
        module.items.push(Item::with_attrs(
            ItemId(0),
            Visibility::Local,
            vec![mut_attr.clone()],
            ItemKind::Struct(StructDecl {
                name: Ident::new("User", span),
                fields: Vec::new(),
                methods: Vec::new(),
                static_methods: Vec::new(),
                span,
            }),
            span,
        ));
        filter_module(&mut module, &config).expect("mutation on struct should be allowed");

        let mut invalid_module = Module::new(FileId(0));
        invalid_module.items.push(function("bad_fn", vec![mut_attr]));
        let err = filter_module(&mut invalid_module, &config).expect_err("mutation on function should fail");
        assert!(err.contains("attribute 'mutation' is only allowed on struct, class, or enum"));
    }

    #[test]
    fn test_cfg_predicates_and_boolean_combinators() {
        let span = span();
        let target = TargetTriple::X86_64UnknownLinuxGnu;
        let config = ConditionalConfig::new(target, vec!["ssl".to_string(), "gui".to_string()]);

        // cfg(target_os = "linux")
        let os_expr = Expr::new(
            ExprKind::NamedArg(
                Ident::new("target_os", span),
                Box::new(Expr::new(ExprKind::Literal(LiteralValue::String("linux".to_string())), span)),
            ),
            span,
        );
        assert!(eval_cfg_expr(&os_expr, &config).unwrap());

        // cfg(target_os = "windows")
        let win_expr = Expr::new(
            ExprKind::NamedArg(
                Ident::new("target_os", span),
                Box::new(Expr::new(ExprKind::Literal(LiteralValue::String("windows".to_string())), span)),
            ),
            span,
        );
        assert!(!eval_cfg_expr(&win_expr, &config).unwrap());

        // cfg(all(target_os = "linux", feature = "ssl"))
        let feat_expr = Expr::new(
            ExprKind::NamedArg(
                Ident::new("feature", span),
                Box::new(Expr::new(ExprKind::Literal(LiteralValue::String("ssl".to_string())), span)),
            ),
            span,
        );
        let all_call = Expr::new(
            ExprKind::Call(
                Box::new(Expr::new(ExprKind::Ident(Ident::new("all", span)), span)),
                vec![os_expr.clone(), feat_expr.clone()],
            ),
            span,
        );
        assert!(eval_cfg_expr(&all_call, &config).unwrap());

        // cfg(not(feature = "missing"))
        let missing_feat = Expr::new(
            ExprKind::NamedArg(
                Ident::new("feature", span),
                Box::new(Expr::new(ExprKind::Literal(LiteralValue::String("missing".to_string())), span)),
            ),
            span,
        );
        let not_call = Expr::new(
            ExprKind::Call(
                Box::new(Expr::new(ExprKind::Ident(Ident::new("not", span)), span)),
                vec![missing_feat.clone()],
            ),
            span,
        );
        assert!(eval_cfg_expr(&not_call, &config).unwrap());

        // cfg(any(target_os = "windows", feature = "ssl"))
        let any_call = Expr::new(
            ExprKind::Call(
                Box::new(Expr::new(ExprKind::Ident(Ident::new("any", span)), span)),
                vec![win_expr.clone(), feat_expr.clone()],
            ),
            span,
        );
        assert!(eval_cfg_expr(&any_call, &config).unwrap());

        // cfg(any(target_os = "windows", feature = "missing")) -> false
        let any_false = Expr::new(
            ExprKind::Call(
                Box::new(Expr::new(ExprKind::Ident(Ident::new("any", span)), span)),
                vec![win_expr.clone(), missing_feat.clone()],
            ),
            span,
        );
        assert!(!eval_cfg_expr(&any_false, &config).unwrap());
    }

    #[test]
    fn test_cfg_attr_desugaring() {
        let span = span();
        let target = TargetTriple::X86_64UnknownLinuxGnu;
        let config = ConditionalConfig::new(target, vec!["serialize".to_string()]);

        // #[cfg_attr(feature = "serialize", inline)]
        let cond_expr = Expr::new(
            ExprKind::NamedArg(
                Ident::new("feature", span),
                Box::new(Expr::new(ExprKind::Literal(LiteralValue::String("serialize".to_string())), span)),
            ),
            span,
        );
        let target_attr = Expr::new(ExprKind::Ident(Ident::new("inline", span)), span);
        let cfg_attr_node = AttributeNode::new(
            Ident::new("cfg_attr", span),
            AttributeArgs::Positional(vec![cond_expr, target_attr]),
            span,
        );

        // #[cfg_attr(target_os = "windows", cold)] -> condition false
        let false_cond = Expr::new(
            ExprKind::NamedArg(
                Ident::new("target_os", span),
                Box::new(Expr::new(ExprKind::Literal(LiteralValue::String("windows".to_string())), span)),
            ),
            span,
        );
        let cold_attr = Expr::new(ExprKind::Ident(Ident::new("cold", span)), span);
        let false_cfg_attr = AttributeNode::new(
            Ident::new("cfg_attr", span),
            AttributeArgs::Positional(vec![false_cond, cold_attr]),
            span,
        );

        let mut module = Module::new(FileId(0));
        module.items.push(function("process", vec![cfg_attr_node, false_cfg_attr]));

        filter_module(&mut module, &config).expect("cfg_attr should desugar cleanly");
        assert_eq!(module.items.len(), 1);
        if let ItemKind::Fun(f) = &module.items[0].kind {
            assert_eq!(f.attrs.len(), 1);
            assert_eq!(f.attrs[0].path_str(), "inline");
        } else {
            panic!("expected function");
        }
    }
}
