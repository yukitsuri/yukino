#![deny(unsafe_code)]
#![allow(
    clippy::wildcard_imports,
    clippy::unnecessary_wraps,
    clippy::cast_possible_wrap,
    clippy::map_unwrap_or
)]

use indexmap::IndexMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use yukic_ast::*;
use yukic_diagnostics::Diagnostic;
use yukic_diagnostics::code::ErrorCode;
use yukic_span::SourceMap;

/// cfg for comptime builtin macro eval
#[derive(Clone, Debug, Default)]
pub struct BuiltinMacroConfig {
    pub base_dir: Option<PathBuf>,
    pub manifest_path: Option<PathBuf>,
    pub manifest_value: Option<toml::Value>,
    pub workspace_manifest_path: Option<PathBuf>,
    pub workspace_manifest_value: Option<toml::Value>,
    pub host_manifest_path: Option<PathBuf>,
    pub host_manifest_value: Option<toml::Value>,
    pub host_workspace_path: Option<PathBuf>,
    pub host_workspace_value: Option<toml::Value>,
    pub is_host: bool,
    pub project_env: IndexMap<String, String>,
    pub connected_libs: Vec<String>,
    pub features: Vec<String>,
    pub profile_name: String,
    pub yukic_version: String,
    pub target_triple: String,
    pub is_test: bool,
    pub macro_runners: std::collections::HashMap<String, PathBuf>,
    pub macro_timeout: Option<std::time::Duration>,
}

impl BuiltinMacroConfig {
    pub fn is_lib_connected(&self, lib_name: &str) -> bool {
        let trimmed = lib_name.strip_prefix("lib").unwrap_or(lib_name);
        self.connected_libs.iter().any(|lib| {
            lib == lib_name || lib.strip_prefix("lib").unwrap_or(lib) == trimmed
        })
    }

    pub fn get_manifest_for_scope(&self, scope: i32) -> Option<&toml::Value> {
        match scope {
            1 => self.manifest_value.as_ref(),
            2 => self.workspace_manifest_value.as_ref(),
            3 => {
                if self.is_host || self.host_workspace_value.is_none() {
                    self.workspace_manifest_value.as_ref()
                } else {
                    self.host_workspace_value.as_ref()
                }
            }
            4 => {
                if self.is_host || self.host_manifest_value.is_none() {
                    self.manifest_value.as_ref()
                } else {
                    self.host_manifest_value.as_ref()
                }
            }
            _ => self.manifest_value.as_ref(),
        }
    }

    pub fn is_lib_connected_in_manifest(manifest: &toml::Value, lib_name: &str) -> bool {
        let trimmed = lib_name.strip_prefix("lib").unwrap_or(lib_name);
        let check_table = |tbl: &toml::map::Map<String, toml::Value>| -> bool {
            for (key, _) in tbl {
                let key_trimmed = key.strip_prefix("lib").unwrap_or(key);
                if key == lib_name || key_trimmed == trimmed {
                    return true;
                }
            }
            false
        };

        if let Some(deps) = manifest.get("dependencies").and_then(toml::Value::as_table) {
            if check_table(deps) {
                return true;
            }
        }
        if let Some(ws) = manifest.get("workspace").and_then(toml::Value::as_table) {
            if let Some(deps) = ws.get("dependencies").and_then(toml::Value::as_table) {
                if check_table(deps) {
                    return true;
                }
            }
        }
        if let Some(bdeps) = manifest.get("build-dependencies").and_then(toml::Value::as_table) {
            if check_table(bdeps) {
                return true;
            }
        }
        false
    }

    pub fn is_lib_connected_for_scope(&self, lib_name: &str, scope: i32) -> bool {
        if (scope == 1 || scope < 1 || scope > 4) && self.is_lib_connected(lib_name) {
            return true;
        }
        if (scope == 4 || scope == 3) && self.is_host && self.is_lib_connected(lib_name) {
            return true;
        }
        if let Some(manifest) = self.get_manifest_for_scope(scope) {
            Self::is_lib_connected_in_manifest(manifest, lib_name)
        } else {
            false
        }
    }
}

pub fn is_builtin_macro(name: &str) -> bool {
    matches!(
        name,
        "include_strings"
            | "include_str"
            | "include_string"
            | "include_bytes"
            | "file"
            | "line"
            | "col"
            | "typeof"
            | "yukienv"
            | "hostenv"
            | "compile_error"
            | "is_lib_connected"
            | "get_features"
            | "get_profileconfig"
            | "get_yukic_version"
            | "get_target"
            | "cfg"
            | "compile_print"
    )
}

pub fn eval_builtin_macro(
    call: &MacroCallNode,
    source_map: Option<&SourceMap>,
    config: &BuiltinMacroConfig,
) -> Result<Expr, Diagnostic> {
    let name = if call.path.len() == 2 && call.path[0].name == "rt" {
        call.path[1].name.as_str()
    } else {
        call.path
            .first()
            .map(|id| id.name.as_str())
            .unwrap_or_default()
    };

    match name {
        "include_strings" | "include_str" | "include_string" => {
            eval_include_strings(call, source_map, config)
        }
        "include_bytes" => eval_include_bytes(call, source_map, config),
        "file" => eval_file_macro(call, source_map),
        "line" => eval_line_macro(call, source_map),
        "col" => eval_col_macro(call, source_map),
        "typeof" => eval_typeof_macro(call),
        "yukienv" => eval_yukienv_macro(call, config),
        "hostenv" => eval_hostenv_macro(call),
        "compile_error" => eval_compile_error(call),
        "is_lib_connected" => eval_is_lib_connected_macro(call, config),
        "get_features" => eval_get_features_macro(call, config),
        "get_profileconfig" => eval_get_profileconfig_macro(call, config),
        "get_yukic_version" => eval_get_yukic_version_macro(call, config),
        "get_target" => eval_get_target_macro(call, config),
        "cfg" => eval_cfg_macro(call, config),
        "compile_print" => eval_compile_print(call, source_map, config),
        _ => Err(Diagnostic::error(
            ErrorCode::E0306,
            format!("unknown built-in macro `{name}!`"),
            call.span,
        )),
    }
}

fn extract_string_literal(call: &MacroCallNode, macro_name: &str) -> Result<String, Diagnostic> {
    if call.args.is_empty() {
        return Err(Diagnostic::error(
            ErrorCode::E0306,
            format!("{macro_name}! expects 1 argument, found 0"),
            call.span,
        ));
    }
    match &call.args[0] {
        MacroArg::Expr(Expr {
            kind: ExprKind::Literal(LiteralValue::String(s)),
            ..
        }) => Ok(s.clone()),
        _ => Err(Diagnostic::error(
            ErrorCode::E0306,
            format!("{macro_name}! expects a string literal argument"),
            call.span,
        )),
    }
}

fn extract_scope_arg(call: &MacroCallNode, arg_idx: usize) -> Result<i32, Diagnostic> {
    if call.args.len() <= arg_idx {
        return Ok(1);
    }
    match &call.args[arg_idx] {
        MacroArg::Expr(Expr {
            kind: ExprKind::Literal(LiteralValue::Int(i)),
            ..
        }) => Ok(*i as i32),
        MacroArg::Expr(Expr {
            kind: ExprKind::Literal(LiteralValue::UInt(u)),
            ..
        }) => Ok(*u as i32),
        _ => Err(Diagnostic::error(
            ErrorCode::E0306,
            "macro scope argument must be an integer (1, 2, 3, 4)",
            call.span,
        )),
    }
}

fn find_enclosing_config_dir(
    call: &MacroCallNode,
    source_map: Option<&SourceMap>,
    config: &BuiltinMacroConfig,
) -> Option<PathBuf> {
    if let Some(sm) = source_map
        && let Some(src_file) = sm.get_file(call.span.file_id)
    {
        let mut curr = src_file.path.parent();
        while let Some(dir) = curr {
            if dir.join("yuki.toml").is_file()
                || dir.join("lib.toml").is_file()
                || dir.join("yukispace.toml").is_file()
            {
                return Some(dir.to_path_buf());
            }
            curr = dir.parent();
        }
    }

    if let Some(ref m_path) = config.manifest_path {
        if let Some(parent) = m_path.parent() {
            return Some(parent.to_path_buf());
        }
    }

    if let Some(ref base) = config.base_dir {
        return Some(base.clone());
    }

    if let Some(sm) = source_map
        && let Some(src_file) = sm.get_file(call.span.file_id)
        && let Some(parent) = src_file.path.parent()
    {
        return Some(parent.to_path_buf());
    }

    None
}

fn resolve_path(
    rel_path: &str,
    call: &MacroCallNode,
    source_map: Option<&SourceMap>,
    config: &BuiltinMacroConfig,
) -> PathBuf {
    let target = Path::new(rel_path);
    if target.is_absolute() {
        return target.to_path_buf();
    }

    if let Some(config_dir) = find_enclosing_config_dir(call, source_map, config) {
        return config_dir.join(target);
    }

    target.to_path_buf()
}

fn eval_include_strings(
    call: &MacroCallNode,
    source_map: Option<&SourceMap>,
    config: &BuiltinMacroConfig,
) -> Result<Expr, Diagnostic> {
    let path_str = extract_string_literal(call, "include_strings")?;
    let path = resolve_path(&path_str, call, source_map, config);

    let bytes = fs::read(&path).map_err(|_| {
        Diagnostic::error(
            ErrorCode::E0307,
            format!("include file not found: `{path_str}`"),
            call.span,
        )
    })?;

    match std::str::from_utf8(&bytes) {
        Ok(text) => Ok(Expr::new(
            ExprKind::Literal(LiteralValue::String(text.to_string())),
            call.span,
        )),
        Err(err) => Err(Diagnostic::error(
            ErrorCode::E0306,
            format!(
                "file `{path_str}` contains invalid utf-8 bytes at offset {}",
                err.valid_up_to()
            ),
            call.span,
        )),
    }
}

fn eval_include_bytes(
    call: &MacroCallNode,
    source_map: Option<&SourceMap>,
    config: &BuiltinMacroConfig,
) -> Result<Expr, Diagnostic> {
    let path_str = extract_string_literal(call, "include_bytes")?;
    let path = resolve_path(&path_str, call, source_map, config);

    let bytes = fs::read(&path).map_err(|_| {
        Diagnostic::error(
            ErrorCode::E0307,
            format!("include file not found: `{path_str}`"),
            call.span,
        )
    })?;

    Ok(Expr::new(
        ExprKind::Literal(LiteralValue::Bytes(bytes)),
        call.span,
    ))
}

fn eval_file_macro(
    call: &MacroCallNode,
    source_map: Option<&SourceMap>,
) -> Result<Expr, Diagnostic> {
    let filename = source_map
        .and_then(|sm| sm.get_file(call.span.file_id))
        .map(|f| f.path.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    Ok(Expr::new(
        ExprKind::Literal(LiteralValue::String(filename)),
        call.span,
    ))
}

fn eval_line_macro(
    call: &MacroCallNode,
    source_map: Option<&SourceMap>,
) -> Result<Expr, Diagnostic> {
    let line = source_map
        .and_then(|sm| sm.lookup_line_column(call.span))
        .map_or(1, |(l, _)| l);

    Ok(Expr::new(
        ExprKind::Literal(LiteralValue::Int(line as i64)),
        call.span,
    ))
}

fn eval_col_macro(
    call: &MacroCallNode,
    source_map: Option<&SourceMap>,
) -> Result<Expr, Diagnostic> {
    let col = source_map
        .and_then(|sm| sm.lookup_line_column(call.span))
        .map_or(1, |(_, c)| c);

    Ok(Expr::new(
        ExprKind::Literal(LiteralValue::Int(col as i64)),
        call.span,
    ))
}

fn parse_path_segments(path: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in path.chars() {

        if ch == '"' { in_quotes = !in_quotes; } 
        else if ch == '.' && !in_quotes {
            if !current.is_empty() {
                segments.push(current.trim_matches('"').to_string());
                current.clear();
            }
        } 

        else { current.push(ch); }
    }
    
    if !current.is_empty() {
        segments.push(current.trim_matches('"').to_string());
    }
    
    segments
}

fn toml_value_to_string(val: &toml::Value) -> String {
    match val {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Datetime(dt) => dt.to_string(),
        toml::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(toml_value_to_string).collect();
            format!("[{}]", items.join(", "))
        }
        toml::Value::Table(tbl) => toml::to_string(tbl).unwrap_or_default(),
    }
}

fn query_toml_path(root: &toml::Value, path: &str) -> Option<String> {
    let segments = parse_path_segments(path);
    let mut current = root;
    for seg in &segments {
        match current {
            toml::Value::Table(tbl) => {
                current = tbl.get(seg)?;
            }
            toml::Value::Array(arr) => {
                let idx = seg.parse::<usize>().ok()?;
                current = arr.get(idx)?;
            }
            _ => return None,
        }
    }
    Some(toml_value_to_string(current))
}

fn eval_yukienv_macro(
    call: &MacroCallNode,
    config: &BuiltinMacroConfig,
) -> Result<Expr, Diagnostic> {
    if call.args.is_empty() || call.args.len() > 2 {
        return Err(Diagnostic::error(
            ErrorCode::E0306,
            format!("yukienv! expects 1 or 2 arguments, found {}", call.args.len()),
            call.span,
        ));
    }

    let key = extract_string_literal(call, "yukienv")?;
    let scope = extract_scope_arg(call, 1)?;

    if scope == 1 || scope < 1 || scope > 4 {
        if let Some(val) = config.project_env.get(&key) {
            return Ok(Expr::new(
                ExprKind::Literal(LiteralValue::String(val.clone())),
                call.span,
            ));
        }
    }

    if let Some(manifest) = config.get_manifest_for_scope(scope)
        && let Some(val) = query_toml_path(manifest, &key)
    {
        return Ok(Expr::new(
            ExprKind::Literal(LiteralValue::String(val)),
            call.span,
        ));
    }

    let scope_name = match scope {
        1 => "current package manifest",
        2 => "current workspace manifest",
        3 => "host application workspace manifest",
        4 => "host application manifest",
        _ => "current package manifest",
    };

    Err(Diagnostic::error(
        ErrorCode::E0306,
        format!("yukienv key `{key}` not found in {scope_name} (scope {scope})"),
        call.span,
    ))
}

fn eval_is_lib_connected_macro(
    call: &MacroCallNode,
    config: &BuiltinMacroConfig,
) -> Result<Expr, Diagnostic> {
    if call.args.is_empty() || call.args.len() > 2 {
        return Err(Diagnostic::error(
            ErrorCode::E0306,
            format!("is_lib_connected! expects 1 or 2 arguments, found {}", call.args.len()),
            call.span,
        ));
    }

    let lib_name = extract_string_literal(call, "is_lib_connected")?;
    let scope = extract_scope_arg(call, 1)?;
    let connected = config.is_lib_connected_for_scope(&lib_name, scope);
    Ok(Expr::new(
        ExprKind::Literal(LiteralValue::Bool(connected)),
        call.span,
    ))
}

fn eval_get_features_macro(
    call: &MacroCallNode,
    config: &BuiltinMacroConfig,
) -> Result<Expr, Diagnostic> {
    if !call.args.is_empty() {
        return Err(Diagnostic::error(
            ErrorCode::E0306,
            format!("get_features! expects 0 arguments, found {}", call.args.len()),
            call.span,
        ));
    }
    let elements: Vec<Expr> = config
        .features
        .iter()
        .map(|f| Expr::new(ExprKind::Literal(LiteralValue::String(f.clone())), call.span))
        .collect();
    Ok(Expr::new(ExprKind::ArrayInit(elements), call.span))
}

fn eval_get_profileconfig_macro(
    call: &MacroCallNode,
    config: &BuiltinMacroConfig,
) -> Result<Expr, Diagnostic> {
    if !call.args.is_empty() {
        return Err(Diagnostic::error(
            ErrorCode::E0306,
            format!("get_profileconfig! expects 0 arguments, found {}", call.args.len()),
            call.span,
        ));
    }
    let prof = if config.profile_name.is_empty() {
        "debug"
    } else {
        &config.profile_name
    };
    Ok(Expr::new(
        ExprKind::Literal(LiteralValue::String(prof.to_string())),
        call.span,
    ))
}

fn eval_get_yukic_version_macro(
    call: &MacroCallNode,
    config: &BuiltinMacroConfig,
) -> Result<Expr, Diagnostic> {
    if !call.args.is_empty() {
        return Err(Diagnostic::error(
            ErrorCode::E0306,
            format!("get_yukic_version! expects 0 arguments, found {}", call.args.len()),
            call.span,
        ));
    }
    let ver = if config.yukic_version.is_empty() {
        env!("CARGO_PKG_VERSION")
    } else {
        &config.yukic_version
    };
    Ok(Expr::new(
        ExprKind::Literal(LiteralValue::String(ver.to_string())),
        call.span,
    ))
}

fn eval_get_target_macro(
    call: &MacroCallNode,
    config: &BuiltinMacroConfig,
) -> Result<Expr, Diagnostic> {
    if !call.args.is_empty() {
        return Err(Diagnostic::error(
            ErrorCode::E0306,
            format!("get_target! expects 0 arguments, found {}", call.args.len()),
            call.span,
        ));
    }
    let target = if config.target_triple.is_empty() {
        "unknown"
    } else {
        &config.target_triple
    };
    Ok(Expr::new(
        ExprKind::Literal(LiteralValue::String(target.to_string())),
        call.span,
    ))
}

fn eval_hostenv_macro(call: &MacroCallNode) -> Result<Expr, Diagnostic> {
    let var_name = extract_string_literal(call, "hostenv")?;

    match std::env::var(&var_name) {
        Ok(val) => Ok(Expr::new(
            ExprKind::Literal(LiteralValue::String(val)),
            call.span,
        )),
        Err(_) => Err(Diagnostic::error(
            ErrorCode::E0306,
            format!("hostenv environment variable `{var_name}` is not set"),
            call.span,
        )),
    }
}

fn eval_typeof_macro(call: &MacroCallNode) -> Result<Expr, Diagnostic> {
    if call.args.len() != 1 {
        return Err(Diagnostic::error(
            ErrorCode::E0306,
            format!("typeof! expects 1 argument, found {}", call.args.len()),
            call.span,
        ));
    }
    let arg = match &call.args[0] {
        MacroArg::Expr(e) => e.clone(),
        _ => {
            return Err(Diagnostic::error(
                ErrorCode::E0306,
                "typeof! expects an expression argument".to_string(),
                call.span,
            ));
        }
    };
    Ok(Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(
                ExprKind::Ident(Ident::new("__typeof_intrinsic__", call.span)),
                call.span,
            )),
            vec![arg],
        ),
        call.span,
    ))
}

fn eval_compile_error(call: &MacroCallNode) -> Result<Expr, Diagnostic> {
    let msg = extract_string_literal(call, "compile_error")?;
    Err(Diagnostic::error(ErrorCode::E0306, msg, call.span))
}

fn eval_compile_print(
    call: &MacroCallNode,
    source_map: Option<&SourceMap>,
    config: &BuiltinMacroConfig,
) -> Result<Expr, Diagnostic> {
    if call.args.is_empty() {
        return Err(Diagnostic::error(
            ErrorCode::E0306,
            "compile_print! expects at least 1 argument",
            call.span,
        ));
    }

    let mut output = String::new();
    for arg in &call.args {
        match arg {
            MacroArg::Expr(expr) => {
                format_macro_expr(expr, source_map, config, &mut output)?;
            }
            _ => {}
        }
    }

    print!("{output}");
    let _ = std::io::stdout().flush();

    Ok(Expr::new(
        ExprKind::Literal(LiteralValue::String(output)),
        call.span,
    ))
}

fn format_macro_expr(
    expr: &Expr,
    source_map: Option<&SourceMap>,
    config: &BuiltinMacroConfig,
    out: &mut String,
) -> Result<(), Diagnostic> {
    match &expr.kind {
        ExprKind::Literal(LiteralValue::String(s)) => {
            out.push_str(s);
        }
        ExprKind::Literal(LiteralValue::Int(i)) => {
            out.push_str(&i.to_string());
        }
        ExprKind::Literal(LiteralValue::UInt(u)) => {
            out.push_str(&u.to_string());
        }
        ExprKind::Literal(LiteralValue::Float(f)) => {
            out.push_str(&f.to_string());
        }
        ExprKind::Literal(LiteralValue::Bool(b)) => {
            out.push_str(&b.to_string());
        }
        ExprKind::Ident(id) => {
            if let Some(val) = config.project_env.get(&id.name) {
                out.push_str(val);
            } else if id.name == "target" {
                out.push_str(&config.target_triple);
            } else if id.name == "version" {
                out.push_str(&config.yukic_version);
            } else if id.name == "profile" {
                out.push_str(&config.profile_name);
            } else {
                out.push_str(&id.name);
            }
        }
        ExprKind::InterpolatedString(parts) => {
            for part in parts {
                match part {
                    StringPart::Lit(s) => out.push_str(s),
                    StringPart::Expr(sub_expr) => {
                        format_macro_expr(sub_expr, source_map, config, out)?;
                    }
                }
            }
        }
        ExprKind::MacroCall(inner_call) => {
            let eval_res = eval_builtin_macro(inner_call, source_map, config)?;
            format_macro_expr(&eval_res, source_map, config, out)?;
        }
        ExprKind::Binary(BinaryOp::Add, left, right) => {
            format_macro_expr(left, source_map, config, out)?;
            format_macro_expr(right, source_map, config, out)?;
        }
        _ => {
            if let Some(sm) = source_map {
                if let Some(snippet) = sm.slice_span(expr.span) {
                    out.push_str(snippet);
                }
            }
        }
    }
    Ok(())
}

fn eval_cfg_macro(
    call: &MacroCallNode,
    config: &BuiltinMacroConfig,
) -> Result<Expr, Diagnostic> {
    if call.args.is_empty() {
        return Err(Diagnostic::error(
            ErrorCode::E0306,
            "cfg! expects at least 1 argument",
            call.span,
        ));
    }

    let target = yukic_target::TargetTriple::parse(&config.target_triple)
        .unwrap_or_else(|_| yukic_target::TargetTriple::host());

    let mut result = true;
    for arg in &call.args {
        let MacroArg::Expr(expr) = arg else {
            return Err(Diagnostic::error(
                ErrorCode::E0306,
                "cfg! expects expression argument",
                call.span,
            ));
        };
        match eval_cfg_expr_in_macro(expr, &target, &config.features, config.is_test) {
            Ok(true) => {}
            Ok(false) => {
                result = false;
                break;
            }
            Err(msg) => {
                return Err(Diagnostic::error(
                    ErrorCode::E0306,
                    format!("cfg! error: {msg}"),
                    expr.span,
                ));
            }
        }
    }

    Ok(Expr::new(
        ExprKind::Literal(LiteralValue::Bool(result)),
        call.span,
    ))
}

fn eval_cfg_expr_in_macro(
    expr: &Expr,
    target: &yukic_target::TargetTriple,
    features: &[String],
    is_test: bool,
) -> Result<bool, String> {
    match &expr.kind {
        ExprKind::NamedArg(name, value) => {
            eval_cfg_predicate_in_macro(name.as_str(), value, target, features)
        }
        ExprKind::Binary(BinaryOp::Eq, lhs, rhs) => {
            if let ExprKind::Ident(name) = &lhs.kind {
                eval_cfg_predicate_in_macro(name.as_str(), rhs, target, features)
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
                        if !eval_cfg_expr_in_macro(arg, target, features, is_test)? {
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
                        if eval_cfg_expr_in_macro(arg, target, features, is_test)? {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
                "not" => {
                    if args.len() != 1 {
                        return Err("cfg not(...) expects exactly 1 argument".to_string());
                    }
                    Ok(!eval_cfg_expr_in_macro(&args[0], target, features, is_test)?)
                }
                "feature" | "target_os" | "target_arch" | "target_env" | "target" => {
                    if args.len() != 1 {
                        return Err(format!("cfg {callee_name}(...) expects exactly 1 argument"));
                    }
                    eval_cfg_predicate_in_macro(callee_name, &args[0], target, features)
                }
                other => Err(format!("unknown cfg combinator '{other}'")),
            }
        }
        ExprKind::Ident(id) => {
            match id.as_str() {
                "test" => Ok(is_test),
                "debug_assertions" => Ok(true),
                other => Ok(features.iter().any(|f| f == other)),
            }
        }
        _ => Err("invalid cfg expression".to_string()),
    }
}

fn eval_cfg_predicate_in_macro(
    key: &str,
    value: &Expr,
    target: &yukic_target::TargetTriple,
    features: &[String],
) -> Result<bool, String> {
    let s = match &value.kind {
        ExprKind::Literal(LiteralValue::String(s)) => s.as_str(),
        _ => return Err(format!("cfg predicate '{key}' expects a string literal value")),
    };

    let spec = target.spec();
    match key {
        "feature" => Ok(features.iter().any(|f| f == s)),
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
            Ok(target.as_str() == s)
        }
        other => Err(format!("unknown cfg key '{other}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_span::{BytePos, FileId, Span};

    fn dummy_span() -> Span {
        Span::new(FileId(0), BytePos(0), BytePos(10))
    }

    fn make_call(name: &str, args: Vec<MacroArg>) -> MacroCallNode {
        MacroCallNode {
            path: vec![Ident::new(name, dummy_span())],
            delimiter: MacroDelimiter::Paren,
            args,
            span: dummy_span(),
        }
    }

    #[test]
    fn test_hostenv_macro() {
        let call = make_call(
            "hostenv",
            vec![MacroArg::Expr(Expr::new(
                ExprKind::Literal(LiteralValue::String("PATH".into())),
                dummy_span(),
            ))],
        );
        let res = eval_builtin_macro(&call, None, &BuiltinMacroConfig::default()).unwrap();
        if let ExprKind::Literal(LiteralValue::String(val)) = res.kind {
            assert!(!val.is_empty());
        } else {
            panic!("expected string literal from hostenv!(PATH)");
        }
    }

    #[test]
    fn test_yukienv_macro() {
        let mut config = BuiltinMacroConfig::default();
        config
            .project_env
            .insert("bin.name".into(), "my_app".into());

        let call = make_call(
            "yukienv",
            vec![MacroArg::Expr(Expr::new(
                ExprKind::Literal(LiteralValue::String("bin.name".into())),
                dummy_span(),
            ))],
        );
        let res = eval_builtin_macro(&call, None, &config).unwrap();
        assert_eq!(
            res.kind,
            ExprKind::Literal(LiteralValue::String("my_app".into()))
        );
    }

    #[test]
    fn test_compile_error_macro() {
        let call = make_call(
            "compile_error",
            vec![MacroArg::Expr(Expr::new(
                ExprKind::Literal(LiteralValue::String("custom failure".into())),
                dummy_span(),
            ))],
        );
        let err = eval_builtin_macro(&call, None, &BuiltinMacroConfig::default()).unwrap_err();
        assert_eq!(err.message, "custom failure");
    }

    #[test]
    fn test_compile_print_macro() {
        let mut config = BuiltinMacroConfig::default();
        config.target_triple = "x86_64-unknown-linux-gnu".into();

        // 1. Literal string
        let call_lit = make_call(
            "compile_print",
            vec![MacroArg::Expr(Expr::new(
                ExprKind::Literal(LiteralValue::String("hello compile_print\n".into())),
                dummy_span(),
            ))],
        );
        let res_lit = eval_builtin_macro(&call_lit, None, &config).unwrap();
        assert_eq!(
            res_lit.kind,
            ExprKind::Literal(LiteralValue::String("hello compile_print\n".into()))
        );

        // 2. Interpolated string with nested get_target!() call
        let call_interp = make_call(
            "compile_print",
            vec![MacroArg::Expr(Expr::new(
                ExprKind::InterpolatedString(vec![
                    StringPart::Lit("target is: ".into()),
                    StringPart::Expr(Expr::new(
                        ExprKind::MacroCall(MacroCallNode {
                            path: vec![Ident::new("get_target", dummy_span())],
                            delimiter: MacroDelimiter::Paren,
                            args: Vec::new(),
                            span: dummy_span(),
                        }),
                        dummy_span(),
                    )),
                ]),
                dummy_span(),
            ))],
        );
        let res_interp = eval_builtin_macro(&call_interp, None, &config).unwrap();
        assert_eq!(
            res_interp.kind,
            ExprKind::Literal(LiteralValue::String("target is: x86_64-unknown-linux-gnu".into()))
        );
    }

    #[test]
    fn test_include_file_not_found() {
        let call = make_call(
            "include_strings",
            vec![MacroArg::Expr(Expr::new(
                ExprKind::Literal(LiteralValue::String("non_existent_file.txt".into())),
                dummy_span(),
            ))],
        );
        let err = eval_builtin_macro(&call, None, &BuiltinMacroConfig::default()).unwrap_err();
        assert_eq!(
            err.code,
            yukic_diagnostics::DiagnosticCode::Error(ErrorCode::E0307)
        );
    }

    #[test]
    fn test_yukienv_nested_toml_query() {
        let manifest_str = r#"
[bin]
name = "demo_app"
version = "1.2.3"

[config.release]
opt_level = 3

[libs.runtime.std]
link = "https://example.com/std.tar.zst"
"#;
        let toml_val: toml::Value = toml::from_str(manifest_str).expect("parse toml");
        let config = BuiltinMacroConfig {
            manifest_value: Some(toml_val),
            ..Default::default()
        };

        let call_name = make_call(
            "yukienv",
            vec![MacroArg::Expr(Expr::new(
                ExprKind::Literal(LiteralValue::String("bin.name".into())),
                dummy_span(),
            ))],
        );
        let res_name = eval_builtin_macro(&call_name, None, &config).unwrap();
        assert_eq!(res_name.kind, ExprKind::Literal(LiteralValue::String("demo_app".into())));

        let call_opt = make_call(
            "yukienv",
            vec![MacroArg::Expr(Expr::new(
                ExprKind::Literal(LiteralValue::String("config.release.opt_level".into())),
                dummy_span(),
            ))],
        );
        let res_opt = eval_builtin_macro(&call_opt, None, &config).unwrap();
        assert_eq!(res_opt.kind, ExprKind::Literal(LiteralValue::String("3".into())));

        let call_link = make_call(
            "yukienv",
            vec![MacroArg::Expr(Expr::new(
                ExprKind::Literal(LiteralValue::String("libs.runtime.std.link".into())),
                dummy_span(),
            ))],
        );
        let res_link = eval_builtin_macro(&call_link, None, &config).unwrap();
        assert_eq!(
            res_link.kind,
            ExprKind::Literal(LiteralValue::String("https://example.com/std.tar.zst".into()))
        );
    }

    #[test]
    fn test_is_lib_connected_macro() {
        let config = BuiltinMacroConfig {
            connected_libs: vec!["std".into(), "math_lib".into(), "libcore".into()],
            ..Default::default()
        };

        let call1 = make_call(
            "is_lib_connected",
            vec![MacroArg::Expr(Expr::new(
                ExprKind::Literal(LiteralValue::String("std".into())),
                dummy_span(),
            ))],
        );
        let res1 = eval_builtin_macro(&call1, None, &config).unwrap();
        assert_eq!(res1.kind, ExprKind::Literal(LiteralValue::Bool(true)));

        let call2 = make_call(
            "is_lib_connected",
            vec![MacroArg::Expr(Expr::new(
                ExprKind::Literal(LiteralValue::String("math_lib".into())),
                dummy_span(),
            ))],
        );
        let res2 = eval_builtin_macro(&call2, None, &config).unwrap();
        assert_eq!(res2.kind, ExprKind::Literal(LiteralValue::Bool(true)));

        let call3 = make_call(
            "is_lib_connected",
            vec![MacroArg::Expr(Expr::new(
                ExprKind::Literal(LiteralValue::String("unknown_dep".into())),
                dummy_span(),
            ))],
        );
        let res3 = eval_builtin_macro(&call3, None, &config).unwrap();
        assert_eq!(res3.kind, ExprKind::Literal(LiteralValue::Bool(false)));
    }

    #[test]
    fn test_yukienv_and_is_lib_connected_scopes() {
        let self_m: toml::Value = toml::from_str(r#"
[package]
name = "my_dep"
[dependencies]
local_lib = { path = "../local" }
"#).unwrap();

        let self_ws: toml::Value = toml::from_str(r#"
[workspace]
members = ["my_dep", "other"]
"#).unwrap();

        let host_ws: toml::Value = toml::from_str(r#"
[workspace]
host_space = "true"
"#).unwrap();

        let host_m: toml::Value = toml::from_str(r#"
[bin]
name = "host_app"
[dependencies]
host_lib = "1.0"
"#).unwrap();

        let config = BuiltinMacroConfig {
            manifest_value: Some(self_m),
            workspace_manifest_value: Some(self_ws),
            host_workspace_value: Some(host_ws),
            host_manifest_value: Some(host_m),
            is_host: false,
            ..Default::default()
        };

        // Scope 1: self manifest
        let call_1 = make_call("yukienv", vec![
            MacroArg::Expr(Expr::new(ExprKind::Literal(LiteralValue::String("package.name".into())), dummy_span())),
            MacroArg::Expr(Expr::new(ExprKind::Literal(LiteralValue::Int(1)), dummy_span())),
        ]);
        assert_eq!(eval_builtin_macro(&call_1, None, &config).unwrap().kind, ExprKind::Literal(LiteralValue::String("my_dep".into())));

        // Scope 2: self workspace
        let call_2 = make_call("yukienv", vec![
            MacroArg::Expr(Expr::new(ExprKind::Literal(LiteralValue::String("workspace.members.0".into())), dummy_span())),
            MacroArg::Expr(Expr::new(ExprKind::Literal(LiteralValue::Int(2)), dummy_span())),
        ]);
        assert_eq!(eval_builtin_macro(&call_2, None, &config).unwrap().kind, ExprKind::Literal(LiteralValue::String("my_dep".into())));

        // Scope 3: host workspace
        let call_3 = make_call("yukienv", vec![
            MacroArg::Expr(Expr::new(ExprKind::Literal(LiteralValue::String("workspace.host_space".into())), dummy_span())),
            MacroArg::Expr(Expr::new(ExprKind::Literal(LiteralValue::Int(3)), dummy_span())),
        ]);
        assert_eq!(eval_builtin_macro(&call_3, None, &config).unwrap().kind, ExprKind::Literal(LiteralValue::String("true".into())));

        // Scope 4: host manifest
        let call_4 = make_call("yukienv", vec![
            MacroArg::Expr(Expr::new(ExprKind::Literal(LiteralValue::String("bin.name".into())), dummy_span())),
            MacroArg::Expr(Expr::new(ExprKind::Literal(LiteralValue::Int(4)), dummy_span())),
        ]);
        assert_eq!(eval_builtin_macro(&call_4, None, &config).unwrap().kind, ExprKind::Literal(LiteralValue::String("host_app".into())));

        // Scope else (e.g. 99): self manifest
        let call_else = make_call("yukienv", vec![
            MacroArg::Expr(Expr::new(ExprKind::Literal(LiteralValue::String("package.name".into())), dummy_span())),
            MacroArg::Expr(Expr::new(ExprKind::Literal(LiteralValue::Int(99)), dummy_span())),
        ]);
        assert_eq!(eval_builtin_macro(&call_else, None, &config).unwrap().kind, ExprKind::Literal(LiteralValue::String("my_dep".into())));

        // is_lib_connected with scope:
        // scope 1: local_lib is in self_m
        let call_c1 = make_call("is_lib_connected", vec![
            MacroArg::Expr(Expr::new(ExprKind::Literal(LiteralValue::String("local_lib".into())), dummy_span())),
            MacroArg::Expr(Expr::new(ExprKind::Literal(LiteralValue::Int(1)), dummy_span())),
        ]);
        assert_eq!(eval_builtin_macro(&call_c1, None, &config).unwrap().kind, ExprKind::Literal(LiteralValue::Bool(true)));

        // scope 4: host_lib is in host_m
        let call_c4 = make_call("is_lib_connected", vec![
            MacroArg::Expr(Expr::new(ExprKind::Literal(LiteralValue::String("host_lib".into())), dummy_span())),
            MacroArg::Expr(Expr::new(ExprKind::Literal(LiteralValue::Int(4)), dummy_span())),
        ]);
        assert_eq!(eval_builtin_macro(&call_c4, None, &config).unwrap().kind, ExprKind::Literal(LiteralValue::Bool(true)));

        // scope 1: host_lib is NOT in self_m
        let call_c1_fail = make_call("is_lib_connected", vec![
            MacroArg::Expr(Expr::new(ExprKind::Literal(LiteralValue::String("host_lib".into())), dummy_span())),
            MacroArg::Expr(Expr::new(ExprKind::Literal(LiteralValue::Int(1)), dummy_span())),
        ]);
        assert_eq!(eval_builtin_macro(&call_c1_fail, None, &config).unwrap().kind, ExprKind::Literal(LiteralValue::Bool(false)));
    }

    #[test]
    fn test_get_features_macro() {
        let config = BuiltinMacroConfig {
            features: vec!["feat_a".into(), "feat_b".into()],
            ..Default::default()
        };

        let call = make_call("get_features", vec![]);
        let res = eval_builtin_macro(&call, None, &config).unwrap();
        if let ExprKind::ArrayInit(items) = res.kind {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].kind, ExprKind::Literal(LiteralValue::String("feat_a".into())));
            assert_eq!(items[1].kind, ExprKind::Literal(LiteralValue::String("feat_b".into())));
        } else {
            panic!("expected ArrayInit");
        }
    }

    #[test]
    fn test_get_profileconfig_and_version_and_target_macros() {
        let config = BuiltinMacroConfig {
            profile_name: "release".into(),
            yukic_version: "0.1.0".into(),
            target_triple: "x86_64-unknown-linux-gnu".into(),
            ..Default::default()
        };

        let call_prof = make_call("get_profileconfig", vec![]);
        let res_prof = eval_builtin_macro(&call_prof, None, &config).unwrap();
        assert_eq!(res_prof.kind, ExprKind::Literal(LiteralValue::String("release".into())));

        let call_ver = make_call("get_yukic_version", vec![]);
        let res_ver = eval_builtin_macro(&call_ver, None, &config).unwrap();
        assert_eq!(res_ver.kind, ExprKind::Literal(LiteralValue::String("0.1.0".into())));

        let call_tgt = make_call("get_target", vec![]);
        let res_tgt = eval_builtin_macro(&call_tgt, None, &config).unwrap();
        assert_eq!(
            res_tgt.kind,
            ExprKind::Literal(LiteralValue::String("x86_64-unknown-linux-gnu".into()))
        );
    }

    #[test]
    fn test_typeof_macro() {
        let config = BuiltinMacroConfig::default();
        let target_expr = Expr::new(
            ExprKind::Ident(Ident::new("x", dummy_span())),
            dummy_span(),
        );
        let call = make_call("typeof", vec![MacroArg::Expr(target_expr)]);
        let res = eval_builtin_macro(&call, None, &config).unwrap();
        match res.kind {
            ExprKind::Call(callee, args) => {
                assert_eq!(callee.kind, ExprKind::Ident(Ident::new("__typeof_intrinsic__", dummy_span())));
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected Call, found {other:?}"),
        }
    }

    #[test]
    fn test_cfg_macro_evaluation() {
        let mut config = BuiltinMacroConfig::default();
        config.target_triple = "x86_64-unknown-linux-gnu".into();
        config.features = vec!["ssl".into(), "fast_math".into()];

        // cfg!(target_os = "linux") -> true
        let arg_linux = Expr::new(
            ExprKind::NamedArg(
                Ident::new("target_os", dummy_span()),
                Box::new(Expr::new(ExprKind::Literal(LiteralValue::String("linux".into())), dummy_span())),
            ),
            dummy_span(),
        );
        let call = make_call("cfg", vec![MacroArg::Expr(arg_linux)]);
        let res = eval_builtin_macro(&call, None, &config).unwrap();
        assert_eq!(res.kind, ExprKind::Literal(LiteralValue::Bool(true)));

        // cfg!(target_os = "windows") -> false
        let arg_win = Expr::new(
            ExprKind::NamedArg(
                Ident::new("target_os", dummy_span()),
                Box::new(Expr::new(ExprKind::Literal(LiteralValue::String("windows".into())), dummy_span())),
            ),
            dummy_span(),
        );
        let call_win = make_call("cfg", vec![MacroArg::Expr(arg_win)]);
        let res_win = eval_builtin_macro(&call_win, None, &config).unwrap();
        assert_eq!(res_win.kind, ExprKind::Literal(LiteralValue::Bool(false)));

        // cfg!(all(target_os = "linux", feature = "ssl")) -> true
        let arg_feat = Expr::new(
            ExprKind::NamedArg(
                Ident::new("feature", dummy_span()),
                Box::new(Expr::new(ExprKind::Literal(LiteralValue::String("ssl".into())), dummy_span())),
            ),
            dummy_span(),
        );
        let all_call = Expr::new(
            ExprKind::Call(
                Box::new(Expr::new(ExprKind::Ident(Ident::new("all", dummy_span())), dummy_span())),
                vec![
                    Expr::new(
                        ExprKind::NamedArg(
                            Ident::new("target_os", dummy_span()),
                            Box::new(Expr::new(ExprKind::Literal(LiteralValue::String("linux".into())), dummy_span())),
                        ),
                        dummy_span(),
                    ),
                    arg_feat,
                ],
            ),
            dummy_span(),
        );
        let call_all = make_call("cfg", vec![MacroArg::Expr(all_call)]);
        let res_all = eval_builtin_macro(&call_all, None, &config).unwrap();
        assert_eq!(res_all.kind, ExprKind::Literal(LiteralValue::Bool(true)));
    }
}
