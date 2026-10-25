#![deny(unsafe_code)]
#![allow(clippy::wildcard_imports, clippy::single_match_else)]

use std::path::{Path, PathBuf};

use yukic_ast::*;
use yukic_diagnostics::Diagnostic;
use yukic_diagnostics::code::ErrorCode;
use yukic_span::{BytePos, FileId, SourceMap, Span};
use yukino_macroproto::{MacroDiagnostic, MacroDiagnosticSeverity, MacroKind, MacroRequest};

use crate::builtin::{BuiltinMacroConfig, eval_builtin_macro, is_builtin_macro};
use crate::hygiene::apply_hygiene;
use crate::runner::MacroRunnerClient;
use crate::subst::substitute_macro;
use crate::table::MacroTable;

pub struct AstExpander<'a> {
    pub table: MacroTable,
    pub config: BuiltinMacroConfig,
    pub source_map: Option<&'a SourceMap>,
    pub recursion_limit: usize,
    pub diagnostics: Vec<Diagnostic>,
    pub tracked_files: Vec<String>,
    pub runner_client: MacroRunnerClient,
}

impl<'a> AstExpander<'a> {
    pub fn new(
        source_map: Option<&'a SourceMap>,
        config: BuiltinMacroConfig,
        recursion_limit: usize,
    ) -> Self {
        Self {
            table: MacroTable::new(),
            config,
            source_map,
            recursion_limit,
            diagnostics: Vec::new(),
            tracked_files: Vec::new(),
            runner_client: MacroRunnerClient::new(),
        }
    }

    pub fn expand_module(&mut self, module: &mut Module) {
        for item in &module.items {
            if let ItemKind::Macro(decl) = &item.kind {
                self.table.register(decl.name.name.clone(), decl.clone());
            }
        }

        let mut expanded_items = Vec::with_capacity(module.items.len());
        for item in module.items.drain(..) {
            self.expand_module_item(item, &mut expanded_items, 0);
        }
        module.items = expanded_items;
    }

    fn expand_module_item(&mut self, mut item: Item, out: &mut Vec<Item>, depth: usize) {
        if depth >= self.recursion_limit {
            self.diagnostics.push(Diagnostic::error(
                ErrorCode::E0306,
                format!("macro expansion limit ({}) exceeded", self.recursion_limit),
                item.span,
            ));
            out.push(item);
            return;
        }

        if self.try_expand_attribute_macro(&mut item, out, depth) {
            return;
        }

        if self.try_expand_mutation_macro(&mut item, out, depth) {
            return;
        }

        self.expand_item(&mut item);
        out.push(item);
    }

    fn try_expand_attribute_macro(
        &mut self,
        item: &mut Item,
        out: &mut Vec<Item>,
        depth: usize,
    ) -> bool {
        let attrs = item_attributes_mut(item);
        let attr_idx = attrs.iter().position(|a| {
            !is_builtin_or_config_attribute(a)
                && self.lookup_runner_by_name(&a.path_str()).is_some()
        });
        let Some(idx) = attr_idx else { return false; };

        let attr = attrs.remove(idx);
        let attr_name = attr.path_str();
        let runner_path = self.lookup_runner_by_name(&attr_name).cloned().unwrap();
        let attr_args = extract_attribute_args_str(&attr, self.source_map);
        let input_tokens = extract_item_source(item, self.source_map);

        match self.expand_runner_raw(
            &runner_path,
            &attr_name,
            MacroKind::Attribute,
            &input_tokens,
            attr_args,
            item.span,
        ) {
            Ok((output, tracked)) => {
                self.tracked_files.extend(tracked);
                let parsed_items = self.parse_output_as_items(&output, item.span);
                for gen_item in parsed_items {
                    self.expand_module_item(gen_item, out, depth + 1);
                }
            }
            Err(diag) => {
                self.diagnostics.push(diag);
                out.push(item.clone());
            }
        }
        true
    }

    fn try_expand_mutation_macro(
        &mut self,
        item: &mut Item,
        out: &mut Vec<Item>,
        depth: usize,
    ) -> bool {
        let attrs = item_attributes_mut(item);
        let mut_idx = attrs.iter().position(|a| a.is("mutation"));
        let Some(idx) = mut_idx else { return false; };

        let mut_attr = attrs.remove(idx);
        let mutation_names = extract_mutation_names(&mut_attr);
        let mut companion_items = Vec::new();

        for mut_name in mutation_names {
            if let Some(runner_path) = self.lookup_runner_by_name(&mut_name).cloned() {
                let input_tokens = extract_item_source(item, self.source_map);
                match self.expand_runner_raw(
                    &runner_path,
                    &mut_name,
                    MacroKind::Mutation,
                    &input_tokens,
                    None,
                    item.span,
                ) {
                    Ok((output, tracked)) => {
                        self.tracked_files.extend(tracked);
                        let parsed = self.parse_output_as_items(&output, item.span);
                        companion_items.extend(parsed);
                    }
                    Err(diag) => self.diagnostics.push(diag),
                }
            } else {
                self.diagnostics.push(Diagnostic::error(
                    ErrorCode::E0306,
                    format!("mutation macro `{mut_name}` is not defined"),
                    mut_attr.span,
                ));
            }
        }

        self.expand_item(item);
        out.push(item.clone());
        for comp_item in companion_items {
            self.expand_module_item(comp_item, out, depth + 1);
        }
        true
    }

    fn expand_item(&mut self, item: &mut Item) {
        match &mut item.kind {
            ItemKind::Fun(decl) => self.expand_block(&mut decl.body),
            ItemKind::Cfun(decl) => {
                if let Some(body) = &mut decl.body {
                    self.expand_block(body);
                }
            }
            ItemKind::Class(decl) => {
                for method in &mut decl.methods {
                    self.expand_block(&mut method.body);
                }
                for field in &mut decl.fields {
                    if let Some(init) = &mut field.default_val {
                        self.expand_expr(init, 0);
                    }
                }
            }
            ItemKind::Struct(decl) => {
                for method in &mut decl.methods {
                    self.expand_block(&mut method.body);
                }
                for s_method in &mut decl.static_methods {
                    self.expand_block(&mut s_method.body);
                }
                for field in &mut decl.fields {
                    if let Some(init) = &mut field.default_val {
                        self.expand_expr(init, 0);
                    }
                }
            }
            ItemKind::Static(decl) => self.expand_expr(&mut decl.init, 0),
            _ => {}
        }
    }

    pub fn expand_block(&mut self, block: &mut Block) {
        self.table.push_scope();

        for stmt in &block.stmts {
            if let StmtKind::MacroDef(decl) = &stmt.kind {
                self.table
                    .register_local(decl.name.name.clone(), decl.clone());
            }
        }

        let mut expanded_stmts = Vec::new();
        for stmt in &mut block.stmts {
            self.expand_stmt(stmt, &mut expanded_stmts);
        }
        block.stmts = expanded_stmts;

        self.table.pop_scope();
    }

    fn expand_stmt(&mut self, stmt: &mut Stmt, out: &mut Vec<Stmt>) {
        match &mut stmt.kind {
            StmtKind::MacroCall(call) => {
                self.expand_macro_call_stmt(call, out, 0);
            }
            StmtKind::Set { init, .. } => {
                self.expand_expr(init, 0);
                out.push(stmt.clone());
            }
            StmtKind::Static(decl) => {
                self.expand_expr(&mut decl.init, 0);
                out.push(stmt.clone());
            }
            StmtKind::Rewrite { value, .. } => {
                self.expand_expr(value, 0);
                out.push(stmt.clone());
            }
            StmtKind::Expr(expr) => {
                self.expand_expr(expr, 0);
                out.push(Stmt::new(StmtKind::Expr(expr.clone()), stmt.span));
            }
            StmtKind::Return(Some(expr)) => {
                self.expand_expr(expr, 0);
                out.push(stmt.clone());
            }
            StmtKind::Recur(exprs) => {
                for e in exprs {
                    self.expand_expr(e, 0);
                }
                out.push(stmt.clone());
            }
            StmtKind::Defer(block) => {
                self.expand_block(block);
                out.push(stmt.clone());
            }
            _ => out.push(stmt.clone()),
        }
    }

    fn expand_macro_call_stmt(&mut self, call: &MacroCallNode, out: &mut Vec<Stmt>, depth: usize) {
        if depth >= self.recursion_limit {
            self.diagnostics.push(Diagnostic::error(
                ErrorCode::E0306,
                format!("macro expansion limit ({}) exceeded", self.recursion_limit),
                call.span,
            ));
            return;
        }

        let first_name = call.path.first().map_or("", |id| id.name.as_str());
        let is_rt_builtin = call.path.len() == 2
            && call.path[0].name == "rt"
            && is_builtin_macro(&call.path[1].name);
        if is_builtin_macro(first_name) || is_rt_builtin {
            match eval_builtin_macro(call, self.source_map, &self.config) {
                Ok(expr) => out.push(Stmt::new(StmtKind::Expr(expr), call.span)),
                Err(diag) => self.diagnostics.push(diag),
            }
            return;
        }

        match self.table.lookup(&call.path) {
            Some(decl) => {
                let mut hygiene_decl = decl.clone();
                apply_hygiene(&mut hygiene_decl.body, &hygiene_decl.params);

                match substitute_macro(&hygiene_decl, &call.args, call.span) {
                    Ok(MacroBody::Block(mut blk)) => {
                        self.expand_block(&mut blk);
                        out.extend(blk.stmts);
                    }
                    Ok(MacroBody::Expr(mut expr)) => {
                        self.expand_expr(&mut expr, depth + 1);
                        out.push(Stmt::new(StmtKind::Expr(*expr), call.span));
                    }
                    Err(diag) => self.diagnostics.push(diag),
                }
            }
            None => {
                if let Some(runner_path) = self.lookup_runner(call).cloned() {
                    match self.expand_runner_macro(&runner_path, call, MacroKind::FunctionLike) {
                        Ok((output, tracked)) => {
                            self.tracked_files.extend(tracked);
                            self.splice_output_into_stmts(&output, call.span, out);
                        }
                        Err(diag) => self.diagnostics.push(diag),
                    }
                    return;
                }
                let path_str = call
                    .path
                    .iter()
                    .map(|id| id.name.as_str())
                    .collect::<Vec<_>>()
                    .join("::");
                self.diagnostics.push(Diagnostic::error(
                    ErrorCode::E0306,
                    format!("macro `{path_str}!` is not defined"),
                    call.span,
                ));
            }
        }
    }

    pub fn expand_expr(&mut self, expr: &mut Expr, depth: usize) {
        if let ExprKind::MacroCall(call) = &expr.kind {
            if depth >= self.recursion_limit {
                self.diagnostics.push(Diagnostic::error(
                    ErrorCode::E0306,
                    format!("macro expansion limit ({}) exceeded", self.recursion_limit),
                    call.span,
                ));
                expr.kind = ExprKind::Error;
                return;
            }

            let first_name = call.path.first().map_or("", |id| id.name.as_str());
            let is_rt_builtin = call.path.len() == 2
                && call.path[0].name == "rt"
                && is_builtin_macro(&call.path[1].name);
            if is_builtin_macro(first_name) || is_rt_builtin {
                match eval_builtin_macro(call, self.source_map, &self.config) {
                    Ok(evaluated) => *expr = evaluated,
                    Err(diag) => {
                        self.diagnostics.push(diag);
                        expr.kind = ExprKind::Error;
                    }
                }
                return;
            }

            match self.table.lookup(&call.path) {
                Some(decl) => {
                    let mut hygiene_decl = decl.clone();
                    apply_hygiene(&mut hygiene_decl.body, &hygiene_decl.params);

                    match substitute_macro(&hygiene_decl, &call.args, call.span) {
                        Ok(MacroBody::Expr(mut expanded)) => {
                            self.expand_expr(&mut expanded, depth + 1);
                            *expr = *expanded;
                        }
                        Ok(MacroBody::Block(mut blk)) => {
                            self.expand_block(&mut blk);
                            let span = blk.span;
                            expr.kind = ExprKind::Block(blk);
                            expr.span = span;
                        }
                        Err(diag) => {
                            self.diagnostics.push(diag);
                            expr.kind = ExprKind::Error;
                        }
                    }
                }
                None => {
                    if let Some(runner_path) = self.lookup_runner(call).cloned() {
                        match self.expand_runner_macro(&runner_path, call, MacroKind::FunctionLike) {
                            Ok((output, tracked)) => {
                                self.tracked_files.extend(tracked);
                                *expr = self.parse_output_as_expr(&output, call.span);
                            }
                            Err(diag) => {
                                self.diagnostics.push(diag);
                                expr.kind = ExprKind::Error;
                            }
                        }
                        return;
                    }
                    let path_str = call
                        .path
                        .iter()
                        .map(|id| id.name.as_str())
                        .collect::<Vec<_>>()
                        .join("::");
                    self.diagnostics.push(Diagnostic::error(
                        ErrorCode::E0306,
                        format!("macro `{path_str}!` is not defined"),
                        call.span,
                    ));
                    expr.kind = ExprKind::Error;
                }
            }
            return;
        }

        self.expand_expr_children(expr, depth);
    }

    fn expand_expr_children(&mut self, expr: &mut Expr, depth: usize) {
        match &mut expr.kind {
            ExprKind::Unary(_, inner) => self.expand_expr(inner, depth),
            ExprKind::Binary(_, lhs, rhs) => {
                self.expand_expr(lhs, depth);
                self.expand_expr(rhs, depth);
            }
            ExprKind::Ternary(cond, then_e, else_e) => {
                self.expand_expr(cond, depth);
                self.expand_expr(then_e, depth);
                self.expand_expr(else_e, depth);
            }
            ExprKind::Call(callee, args) | ExprKind::ClassInit(callee, args) => {
                self.expand_expr(callee, depth);
                for arg in args {
                    self.expand_expr(arg, depth);
                }
            }
            ExprKind::MethodCall(recv, _, args) => {
                self.expand_expr(recv, depth);
                for arg in args {
                    self.expand_expr(arg, depth);
                }
            }
            ExprKind::Block(block) | ExprKind::RegionBlock(block) => {
                self.expand_block(block);
            }
            ExprKind::ArrayInit(items) => {
                for item in items {
                    self.expand_expr(item, depth);
                }
            }
            ExprKind::InterpolatedString(parts) => {
                for part in parts {
                    if let StringPart::Expr(e) = part {
                        self.expand_expr(e, depth);
                    }
                }
            }
            ExprKind::If {
                cond,
                then_branch,
                elif_branches,
                else_branch,
            } => {
                self.expand_expr(cond, depth);
                self.expand_block(then_branch);
                for (c, b) in elif_branches {
                    self.expand_expr(c, depth);
                    self.expand_block(b);
                }
                if let Some(else_b) = else_branch {
                    self.expand_block(else_b);
                }
            }
            ExprKind::For { iter, body, .. } => {
                self.expand_expr(iter, depth);
                self.expand_block(body);
            }
            ExprKind::Ref { expr, .. } | ExprKind::NamedArg(_, expr) => {
                self.expand_expr(expr, depth);
            }
            _ => {}
        }
    }

    pub fn lookup_runner_by_name(&self, name: &str) -> Option<&PathBuf> {
        if let Some(r) = self.config.macro_runners.get(name) {
            return Some(r);
        }
        if let Some((first, _)) = name.split_once("::") {
            if let Some(r) = self.config.macro_runners.get(first) {
                return Some(r);
            }
        }
        if let Some((_, last)) = name.rsplit_once("::") {
            if let Some(r) = self.config.macro_runners.get(last) {
                return Some(r);
            }
        }
        None
    }

    fn lookup_runner(&self, call: &MacroCallNode) -> Option<&PathBuf> {
        let path_str = call
            .path
            .iter()
            .map(|id| id.name.as_str())
            .collect::<Vec<_>>()
            .join("::");
        self.lookup_runner_by_name(&path_str)
    }

    fn expand_runner_raw(
        &mut self,
        runner_path: &Path,
        macro_name: &str,
        kind: MacroKind,
        input_tokens: &str,
        attr_args: Option<String>,
        span: Span,
    ) -> Result<(String, Vec<String>), Diagnostic> {
        let working_dir = self.config.base_dir.as_ref().map(|p| p.display().to_string());
        let req = MacroRequest::new(
            macro_name,
            kind,
            input_tokens,
            attr_args,
            &self.config.target_triple,
            &self.config.profile_name,
            working_dir,
        );

        let res = self.runner_client.execute(runner_path, &req, span)?;
        for diag in res.diagnostics {
            self.diagnostics.push(convert_macro_diagnostic(diag, span));
        }
        if !res.success {
            let msg = res.error_message.unwrap_or_else(|| "macro runner failed".to_string());
            return Err(Diagnostic::error(ErrorCode::E0306, msg, span));
        }
        Ok((res.output_tokens.unwrap_or_default(), res.tracked_files))
    }

    fn expand_runner_macro(
        &mut self,
        runner_path: &Path,
        call: &MacroCallNode,
        kind: MacroKind,
    ) -> Result<(String, Vec<String>), Diagnostic> {
        let macro_name = call
            .path
            .iter()
            .map(|id| id.name.as_str())
            .collect::<Vec<_>>()
            .join("::");
        let input_tokens = self.extract_macro_call_input(call);
        self.expand_runner_raw(runner_path, &macro_name, kind, &input_tokens, None, call.span)
    }

    fn extract_macro_call_input(&self, call: &MacroCallNode) -> String {
        if let Some(src) = self.source_map.and_then(|sm| sm.slice_span(call.span)) {
            if let Some(open) = src.find(['(', '[', '{']) {
                if let Some(close) = src.rfind([')', ']', '}']) {
                    if close > open {
                        return src[open + 1..close].trim().to_string();
                    }
                }
            }
            return src.to_string();
        }
        let mut parts = Vec::new();
        for arg in &call.args {
            match arg {
                MacroArg::Expr(e) => parts.push(format!("{e:?}")),
                MacroArg::Block(_) => parts.push("{ space }".to_string()),
                MacroArg::Tokens(tokens) => {
                    let s: Vec<_> = tokens
                        .iter()
                        .map(|t| match t {
                            TokenTree::Token(tok, _) => tok.clone(),
                            TokenTree::Group(_, _) => "(...)".to_string(),
                        })
                        .collect();
                    parts.push(s.join(" "));
                }
            }
        }
        parts.join(", ")
    }

    fn parse_output_as_items(&mut self, output: &str, span: Span) -> Vec<Item> {
        let trimmed = output.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        let lexer = yukic_lexer::Lexer::new(span.file_id, trimmed);
        let (tokens, lex_diags) = lexer.tokenize();
        self.diagnostics.extend(lex_diags);

        let mut parser = yukic_parser::Parser::with_file_id(tokens, span.file_id);
        let (parsed_mod, parse_diags) = parser.parse_module();
        self.diagnostics.extend(parse_diags);
        parsed_mod.items
    }

    fn parse_output_as_expr(&mut self, output: &str, span: Span) -> Expr {
        let lexer = yukic_lexer::Lexer::new(span.file_id, output);
        let (tokens, lex_diags) = lexer.tokenize();
        self.diagnostics.extend(lex_diags);

        let mut parser = yukic_parser::Parser::with_file_id(tokens, span.file_id);
        let (expr, parse_diags) = parser.parse_expr();
        self.diagnostics.extend(parse_diags);
        expr
    }

    fn splice_output_into_stmts(&mut self, output: &str, span: Span, out: &mut Vec<Stmt>) {
        let lexer = yukic_lexer::Lexer::new(span.file_id, output);
        let (tokens, lex_diags) = lexer.tokenize();
        self.diagnostics.extend(lex_diags);

        let mut parser = yukic_parser::Parser::with_file_id(tokens, span.file_id);
        let (stmt, parse_diags) = parser.parse_stmt();
        self.diagnostics.extend(parse_diags);
        out.push(stmt);
    }
}

fn is_builtin_or_config_attribute(attr: &AttributeNode) -> bool {
    let name = attr.path_str();
    if attr.path.starts_with("lint") {
        return true;
    }
    matches!(
        name.as_str(),
        "target"
            | "feature"
            | "arch"
            | "cfg"
            | "cfg_attr"
            | "mutation"
            | "inline"
            | "no_inline"
            | "cold"
            | "noreturn"
            | "mangle"
            | "inline_always"
            | "inline_never"
            | "test"
            | "should_panic"
            | "ignore"
            | "timeout_ms"
            | "deprecated"
            | "ignore_warn"
            | "warns_ignore"
            | "stable"
    )
}

fn extract_name_from_expr(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Ident(id) => Some(id.name.clone()),
        ExprKind::Literal(LiteralValue::String(s)) => Some(s.clone()),
        ExprKind::Call(callee, _) => {
            if let ExprKind::Ident(id) = &callee.kind {
                Some(id.name.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn extract_mutation_names(attr: &AttributeNode) -> Vec<String> {
    let mut names = Vec::new();
    match &attr.args {
        AttributeArgs::Positional(exprs) => {
            for e in exprs {
                if let Some(name) = extract_name_from_expr(e) {
                    names.push(name);
                }
            }
        }
        AttributeArgs::Named(pairs) => {
            for (key, _) in pairs {
                names.push(key.name.clone());
            }
        }
        AttributeArgs::None => {}
    }
    names
}

fn extract_attribute_args_str(
    attr: &AttributeNode,
    source_map: Option<&SourceMap>,
) -> Option<String> {
    if let Some(src) = source_map.and_then(|sm| sm.slice_span(attr.span)) {
        if let Some(open) = src.find('(') {
            if let Some(close) = src.rfind(')') {
                if close > open {
                    return Some(src[open + 1..close].trim().to_string());
                }
            }
        }
    }
    match &attr.args {
        AttributeArgs::None => None,
        AttributeArgs::Positional(exprs) => {
            let parts: Vec<_> = exprs.iter().filter_map(extract_name_from_expr).collect();
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(", "))
            }
        }
        AttributeArgs::Named(pairs) => {
            let parts: Vec<_> = pairs.iter().map(|(k, _)| k.name.clone()).collect();
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(", "))
            }
        }
    }
}

fn extract_item_source(item: &Item, source_map: Option<&SourceMap>) -> String {
    if let Some(src) = source_map.and_then(|sm| sm.slice_span(item.span)) {
        return src.to_string();
    }
    match &item.kind {
        ItemKind::Fun(f) => format!("fun {}() {{ space }}", f.name.name),
        ItemKind::Struct(s) => format!("struct {} {{ space }}", s.name.name),
        ItemKind::Class(c) => format!("class {} {{ space }}", c.name.name),
        ItemKind::Enum(e) => format!("enum {} {{ space }}", e.name.name),
        _ => "/* item */".to_string(),
    }
}

fn item_attributes_mut<'a>(item: &'a mut Item) -> &'a mut Vec<AttributeNode> {
    match &mut item.kind {
        ItemKind::Fun(decl) => &mut decl.attrs,
        ItemKind::Cfun(decl) => &mut decl.yukino_attrs,
        _ => &mut item.attrs,
    }
}

fn convert_macro_diagnostic(diag: MacroDiagnostic, fallback_span: Span) -> Diagnostic {
    let span = if let Some(sp) = diag.span {
        Span::new(FileId(sp.file_id), BytePos(sp.start), BytePos(sp.end))
    } else {
        fallback_span
    };
    match diag.severity {
        MacroDiagnosticSeverity::Error => Diagnostic::error(ErrorCode::E0306, diag.message, span),
        MacroDiagnosticSeverity::Warning => {
            Diagnostic::warning(yukic_diagnostics::code::WarningCode::W0001, diag.message, span)
        }
        MacroDiagnosticSeverity::Note => {
            Diagnostic::warning(
                yukic_diagnostics::code::WarningCode::W0001,
                format!("note: {}", diag.message),
                span,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_span::{BytePos, FileId, Span};

    fn dummy_span() -> Span {
        Span::new(FileId(0), BytePos(0), BytePos(10))
    }

    #[test]
    fn test_expander_user_macro() {
        let mut module = Module::new(FileId(0));

        let macro_decl = MacroDefDecl {
            name: Ident::new("add_one", dummy_span()),
            params: vec![MacroParam {
                name: Ident::new("x", dummy_span()),
                kind: MacroParamKind::Expr,
                span: dummy_span(),
            }],
            body: MacroBody::Expr(Box::new(Expr::new(
                ExprKind::Binary(
                    BinaryOp::Add,
                    Box::new(Expr::new(
                        ExprKind::Ident(Ident::new("x", dummy_span())),
                        dummy_span(),
                    )),
                    Box::new(Expr::new(
                        ExprKind::Literal(LiteralValue::Int(1)),
                        dummy_span(),
                    )),
                ),
                dummy_span(),
            ))),
            span: dummy_span(),
        };
        module.items.push(Item::new(
            ItemId(0),
            Visibility::Local,
            ItemKind::Macro(macro_decl),
            dummy_span(),
        ));

        let fun_body = Block::new(
            vec![Stmt::new(
                StmtKind::Set {
                    target: BindingTarget::Ident(Ident::new("val", dummy_span())),
                    ty: None,
                    init: Expr::new(
                        ExprKind::MacroCall(MacroCallNode {
                            path: vec![Ident::new("add_one", dummy_span())],
                            delimiter: MacroDelimiter::Paren,
                            args: vec![MacroArg::Expr(Expr::new(
                                ExprKind::Literal(LiteralValue::Int(41)),
                                dummy_span(),
                            ))],
                            span: dummy_span(),
                        }),
                        dummy_span(),
                    ),
                },
                dummy_span(),
            )],
            dummy_span(),
        );

        module.items.push(Item::new(
            ItemId(1),
            Visibility::Public,
            ItemKind::Fun(FunDecl {
                name: Ident::new("main", dummy_span()),
                attrs: Vec::new(),
                generics: Vec::new(),
                params: Vec::new(),
                return_type: None,
                body: fun_body,
                span: dummy_span(),
            }),
            dummy_span(),
        ));

        let mut expander = AstExpander::new(None, BuiltinMacroConfig::default(), 128);
        expander.expand_module(&mut module);

        assert!(expander.diagnostics.is_empty());
        if let ItemKind::Fun(f) = &module.items[1].kind
            && let StmtKind::Set { init, .. } = &f.body.stmts[0].kind
        {
            if let ExprKind::Binary(BinaryOp::Add, lhs, rhs) = &init.kind {
                assert_eq!(lhs.kind, ExprKind::Literal(LiteralValue::Int(41)));
                assert_eq!(rhs.kind, ExprKind::Literal(LiteralValue::Int(1)));
            } else {
                panic!("expected binary add expression after expansion");
            }
        }
    }

    #[test]
    fn test_expander_recursion_limit() {
        let mut module = Module::new(FileId(0));
        let loop_macro = MacroDefDecl {
            name: Ident::new("loop_mac", dummy_span()),
            params: Vec::new(),
            body: MacroBody::Expr(Box::new(Expr::new(
                ExprKind::MacroCall(MacroCallNode {
                    path: vec![Ident::new("loop_mac", dummy_span())],
                    delimiter: MacroDelimiter::Paren,
                    args: Vec::new(),
                    span: dummy_span(),
                }),
                dummy_span(),
            ))),
            span: dummy_span(),
        };
        module.items.push(Item::new(
            ItemId(0),
            Visibility::Local,
            ItemKind::Macro(loop_macro),
            dummy_span(),
        ));

        let mut expr = Expr::new(
            ExprKind::MacroCall(MacroCallNode {
                path: vec![Ident::new("loop_mac", dummy_span())],
                delimiter: MacroDelimiter::Paren,
                args: Vec::new(),
                span: dummy_span(),
            }),
            dummy_span(),
        );

        let mut expander = AstExpander::new(None, BuiltinMacroConfig::default(), 5);
        expander.table.register(
            "loop_mac".into(),
            MacroDefDecl {
                name: Ident::new("loop_mac", dummy_span()),
                params: Vec::new(),
                body: MacroBody::Expr(Box::new(Expr::new(
                    ExprKind::MacroCall(MacroCallNode {
                        path: vec![Ident::new("loop_mac", dummy_span())],
                        delimiter: MacroDelimiter::Paren,
                        args: Vec::new(),
                        span: dummy_span(),
                    }),
                    dummy_span(),
                ))),
                span: dummy_span(),
            },
        );
        expander.expand_expr(&mut expr, 0);

        assert_eq!(expander.diagnostics.len(), 1);
        assert_eq!(
            expander.diagnostics[0].code,
            yukic_diagnostics::DiagnosticCode::Error(ErrorCode::E0306)
        );
        assert!(expander.diagnostics[0].message.contains("expansion limit"));
    }
}
