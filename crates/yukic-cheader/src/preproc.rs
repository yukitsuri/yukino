#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use yukic_target::{Architecture, DataModel, OperatingSystem, TargetTriple};

use crate::const_eval::eval_preproc_if_expr;

#[derive(Clone, Debug)]
pub struct PreprocessConfig {
    pub target: TargetTriple,
    pub include_dirs: Vec<PathBuf>,
    pub defines: Vec<(String, String)>,
}

impl Default for PreprocessConfig {
    fn default() -> Self {
        Self {
            target: TargetTriple::host(),
            include_dirs: Vec::new(),
            defines: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PreprocessOutput {
    pub source: String,
    pub defines: HashMap<String, String>,
    pub tracked_files: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreprocessError {
    pub message: String,
    pub file: PathBuf,
    pub line: usize,
}

impl std::fmt::Display for PreprocessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}: {}",
            self.file.display(),
            self.line,
            self.message
        )
    }
}

impl std::error::Error for PreprocessError {}

#[derive(Clone, Debug)]
enum MacroDef {
    Object(String),
    Function {
        params: Vec<String>,
        variadic: bool,
        body: String,
    },
}

#[derive(Clone, Copy, Debug)]
struct CondFrame {
    parent_active: bool,
    branch_taken: bool,
    active: bool,
}

struct Preprocessor<'a> {
    cfg: &'a PreprocessConfig,
    macros: HashMap<String, MacroDef>,
    preset_only: HashSet<String>,
    once_files: HashSet<PathBuf>,
    tracked_files: Vec<PathBuf>,
    tracked_set: HashSet<PathBuf>,
    out: String,
    include_depth: usize,
}

const MAX_INCLUDE_DEPTH: usize = 64;
const MAX_MACRO_EXPAND_DEPTH: usize = 64;

pub fn preprocess_file(
    entry_path: &Path,
    cfg: &PreprocessConfig,
) -> Result<PreprocessOutput, PreprocessError> {
    let mut macros = default_target_macros(cfg.target);
    let mut preset_only: HashSet<String> = macros.keys().cloned().collect();
    for (name, val) in &cfg.defines {
        macros.insert(name.clone(), MacroDef::Object(val.clone()));
        preset_only.remove(name);
    }
    let mut pp = Preprocessor {
        cfg,
        macros,
        preset_only,
        once_files: HashSet::new(),
        tracked_files: Vec::new(),
        tracked_set: HashSet::new(),
        out: String::new(),
        include_depth: 0,
    };
    pp.process_disk_file(entry_path)?;
    let defines = pp
        .macros
        .into_iter()
        .filter(|(k, _)| !pp.preset_only.contains(k))
        .filter_map(|(k, v)| match v {
            MacroDef::Object(body) => Some((k, body)),
            MacroDef::Function { .. } => None,
        })
        .collect();
    Ok(PreprocessOutput {
        source: pp.out,
        defines,
        tracked_files: pp.tracked_files,
    })
}

pub fn preprocess_str(
    virtual_path: &Path,
    content: &str,
    cfg: &PreprocessConfig,
) -> Result<PreprocessOutput, PreprocessError> {
    let mut macros = default_target_macros(cfg.target);
    let mut preset_only: HashSet<String> = macros.keys().cloned().collect();
    for (name, val) in &cfg.defines {
        macros.insert(name.clone(), MacroDef::Object(val.clone()));
        preset_only.remove(name);
    }
    let mut pp = Preprocessor {
        cfg,
        macros,
        preset_only,
        once_files: HashSet::new(),
        tracked_files: Vec::new(),
        tracked_set: HashSet::new(),
        out: String::new(),
        include_depth: 0,
    };
    pp.process_source(virtual_path, content)?;
    let defines = pp
        .macros
        .into_iter()
        .filter(|(k, _)| !pp.preset_only.contains(k))
        .filter_map(|(k, v)| match v {
            MacroDef::Object(body) => Some((k, body)),
            MacroDef::Function { .. } => None,
        })
        .collect();
    Ok(PreprocessOutput {
        source: pp.out,
        defines,
        tracked_files: pp.tracked_files,
    })
}

impl Preprocessor<'_> {
    fn record_tracked(&mut self, path: &Path) {
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if self.tracked_set.insert(canonical.clone()) {
            self.tracked_files.push(canonical);
        }
    }

    fn process_disk_file(&mut self, path: &Path) -> Result<(), PreprocessError> {
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if self.once_files.contains(&canonical) {
            return Ok(());
        }
        let content = std::fs::read_to_string(path).map_err(|e| PreprocessError {
            message: format!("failed to read C header '{}': {e}", path.display()),
            file: path.to_path_buf(),
            line: 0,
        })?;
        self.record_tracked(&canonical);
        self.process_source(&canonical, &content)
    }

    fn process_source(&mut self, file: &Path, raw: &str) -> Result<(), PreprocessError> {
        if self.include_depth >= MAX_INCLUDE_DEPTH {
            return Err(PreprocessError {
                message: "maximum #include nesting depth exceeded".to_string(),
                file: file.to_path_buf(),
                line: 1,
            });
        }
        self.include_depth += 1;
        let cleaned = strip_comments_and_join_lines(raw);
        let mut cond_stack: Vec<CondFrame> = Vec::new();
        let file_dir = file.parent().unwrap_or_else(|| Path::new("."));

        for (idx, line) in cleaned.lines().enumerate() {
            let line_no = idx + 1;
            let trimmed = line.trim_start();
            if let Some(after_hash) = trimmed.strip_prefix('#') {
                self.handle_directive(
                    file,
                    file_dir,
                    line_no,
                    after_hash.trim(),
                    &mut cond_stack,
                )?;
                self.out.push('\n');
            } else if is_active(&cond_stack) {
                let expanded = self.expand_line(line, file, line_no);
                self.out.push_str(&expanded);
                self.out.push('\n');
            } else {
                self.out.push('\n');
            }
        }

        self.include_depth -= 1;
        if !cond_stack.is_empty() {
            return Err(PreprocessError {
                message: "unterminated #if/#ifdef/#ifndef block".to_string(),
                file: file.to_path_buf(),
                line: cleaned.lines().count(),
            });
        }
        Ok(())
    }

    fn handle_directive(
        &mut self,
        file: &Path,
        file_dir: &Path,
        line_no: usize,
        dir: &str,
        cond_stack: &mut Vec<CondFrame>,
    ) -> Result<(), PreprocessError> {
        let (kw, rest) = split_first_word(dir);
        let parent_active = is_active(cond_stack);

        match kw {
            "ifdef" => {
                let (name, _) = split_first_word(rest);
                let cond = !name.is_empty() && self.macros.contains_key(name);
                let active = parent_active && cond;
                cond_stack.push(CondFrame {
                    parent_active,
                    branch_taken: active,
                    active,
                });
            }
            "ifndef" => {
                let (name, _) = split_first_word(rest);
                let cond = !name.is_empty() && !self.macros.contains_key(name);
                let active = parent_active && cond;
                cond_stack.push(CondFrame {
                    parent_active,
                    branch_taken: active,
                    active,
                });
            }
            "if" => {
                let cond = if parent_active {
                    self.eval_if_condition(rest, file, line_no)
                } else {
                    false
                };
                let active = parent_active && cond;
                cond_stack.push(CondFrame {
                    parent_active,
                    branch_taken: active,
                    active,
                });
            }
            "elif" => {
                let frame = cond_stack.last_mut().ok_or_else(|| PreprocessError {
                    message: "unexpected #elif without matching #if".to_string(),
                    file: file.to_path_buf(),
                    line: line_no,
                })?;
                if frame.parent_active && !frame.branch_taken {
                    let macros = &self.macros;
                    let expanded = expand_if_expr(rest, macros, file, line_no);
                    let cond = eval_preproc_if_expr(
                        &expanded,
                        &|name| macros.contains_key(name),
                        self.cfg.target,
                    );
                    frame.active = cond;
                    frame.branch_taken = cond;
                } else {
                    frame.active = false;
                }
            }
            "elifdef" => {
                let frame = cond_stack.last_mut().ok_or_else(|| PreprocessError {
                    message: "unexpected #elifdef without matching #if".to_string(),
                    file: file.to_path_buf(),
                    line: line_no,
                })?;
                if frame.parent_active && !frame.branch_taken {
                    let (name, _) = split_first_word(rest);
                    let cond = !name.is_empty() && self.macros.contains_key(name);
                    frame.active = cond;
                    frame.branch_taken = cond;
                } else {
                    frame.active = false;
                }
            }
            "elifndef" => {
                let frame = cond_stack.last_mut().ok_or_else(|| PreprocessError {
                    message: "unexpected #elifndef without matching #if".to_string(),
                    file: file.to_path_buf(),
                    line: line_no,
                })?;
                if frame.parent_active && !frame.branch_taken {
                    let (name, _) = split_first_word(rest);
                    let cond = !name.is_empty() && !self.macros.contains_key(name);
                    frame.active = cond;
                    frame.branch_taken = cond;
                } else {
                    frame.active = false;
                }
            }
            "else" => {
                let frame = cond_stack.last_mut().ok_or_else(|| PreprocessError {
                    message: "unexpected #else without matching #if".to_string(),
                    file: file.to_path_buf(),
                    line: line_no,
                })?;
                frame.active = frame.parent_active && !frame.branch_taken;
                frame.branch_taken = true;
            }
            "endif" => {
                if cond_stack.pop().is_none() {
                    return Err(PreprocessError {
                        message: "unexpected #endif without matching #if".to_string(),
                        file: file.to_path_buf(),
                        line: line_no,
                    });
                }
            }
            _ if !parent_active => {}
            "define" => self.parse_define(rest),
            "undef" => {
                let (name, _) = split_first_word(rest);
                if !name.is_empty() {
                    self.macros.remove(name);
                    self.preset_only.remove(name);
                }
            }
            "pragma" => {
                let trimmed = rest.trim();
                if trimmed == "once" {
                    self.once_files.insert(file.to_path_buf());
                } else if trimmed.starts_with("pack") {
                    self.out.push_str("#pragma ");
                    self.out.push_str(trimmed);
                }
            }
            "include" => {
                let expanded_arg = self.expand_line(rest, file, line_no);
                self.handle_include(file, file_dir, line_no, expanded_arg.trim())?;
            }
            "error" => {
                return Err(PreprocessError {
                    message: format!("#error {rest}"),
                    file: file.to_path_buf(),
                    line: line_no,
                });
            }
            _ => {}
        }
        Ok(())
    }

    fn eval_if_condition(&self, expr: &str, file: &Path, line_no: usize) -> bool {
        let expanded = expand_if_expr(expr, &self.macros, file, line_no);
        eval_preproc_if_expr(
            &expanded,
            &|name| self.macros.contains_key(name),
            self.cfg.target,
        )
    }

    fn parse_define(&mut self, rest: &str) {
        let rest = rest.trim_start();
        let name_end = rest
            .find(|c: char| !is_ident_continue(c))
            .unwrap_or(rest.len());
        if name_end == 0 {
            return;
        }
        let name = rest[..name_end].to_string();
        self.preset_only.remove(&name);
        let after_name = &rest[name_end..];
        if let Some(after_open) = after_name.strip_prefix('(')
            && let Some(close_idx) = after_open.find(')')
        {
            let param_str = &after_open[..close_idx];
            let body = after_open[close_idx + 1..].trim().to_string();
            let mut params = Vec::new();
            let mut variadic = false;
            for part in param_str.split(',') {
                let p = part.trim();
                if p.is_empty() {
                    continue;
                }
                if p == "..." {
                    variadic = true;
                } else if let Some(named_var) = p.strip_suffix("...") {
                    let clean = named_var.trim();
                    if !clean.is_empty() {
                        params.push(clean.to_string());
                    }
                    variadic = true;
                } else {
                    params.push(p.to_string());
                }
            }
            self.macros.insert(
                name,
                MacroDef::Function {
                    params,
                    variadic,
                    body,
                },
            );
            return;
        }
        let body = after_name.trim().to_string();
        self.macros.insert(name, MacroDef::Object(body));
    }

    fn handle_include(
        &mut self,
        file: &Path,
        file_dir: &Path,
        line_no: usize,
        arg: &str,
    ) -> Result<(), PreprocessError> {
        let (target_name, is_system) = if let Some(stripped) = arg.strip_prefix('"')
            && let Some(end) = stripped.find('"')
        {
            (&stripped[..end], false)
        } else if let Some(stripped) = arg.strip_prefix('<')
            && let Some(end) = stripped.find('>')
        {
            (&stripped[..end], true)
        } else {
            return Err(PreprocessError {
                message: format!("malformed #include target: {arg}"),
                file: file.to_path_buf(),
                line: line_no,
            });
        };

        if !is_system {
            let local_candidate = file_dir.join(target_name);
            if local_candidate.is_file() {
                return self.process_disk_file(&local_candidate);
            }
        }

        for dir in &self.cfg.include_dirs {
            let candidate = dir.join(target_name);
            if candidate.is_file() {
                return self.process_disk_file(&candidate);
            }
        }

        if let Some(builtin) = builtin_standard_header(target_name, self.cfg.target) {
            let virtual_path = PathBuf::from(format!("<{target_name}>"));
            if self.once_files.insert(virtual_path.clone()) {
                return self.process_source(&virtual_path, &builtin);
            }
            return Ok(());
        }

        Err(PreprocessError {
            message: format!("header '{target_name}' not found"),
            file: file.to_path_buf(),
            line: line_no,
        })
    }

    fn expand_line(&self, line: &str, file: &Path, line_no: usize) -> String {
        let mut hidden = HashSet::new();
        expand_text(line, &self.macros, &mut hidden, file, line_no, 0)
    }
}

fn is_active(stack: &[CondFrame]) -> bool {
    stack.last().is_none_or(|f| f.active)
}

fn split_first_word(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    let end = s.find(char::is_whitespace).unwrap_or(s.len());
    (&s[..end], s[end..].trim_start())
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn strip_comments_and_join_lines(raw: &str) -> String {
    let mut joined = String::with_capacity(raw.len());
    let normalized = raw.replace("\r\n", "\n");
    let chars: Vec<char> = normalized.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() && chars[i + 1] == '\n' {
            i += 2;
            continue;
        }
        joined.push(chars[i]);
        i += 1;
    }

    let mut out = String::with_capacity(joined.len());
    let jchars: Vec<char> = joined.chars().collect();
    let mut j = 0;
    while j < jchars.len() {
        let c = jchars[j];
        if c == '"' || c == '\'' {
            let quote = c;
            out.push(c);
            j += 1;
            while j < jchars.len() {
                let sc = jchars[j];
                out.push(sc);
                j += 1;
                if sc == '\\' && j < jchars.len() {
                    out.push(jchars[j]);
                    j += 1;
                } else if sc == quote {
                    break;
                }
            }
            continue;
        }
        if c == '/' && j + 1 < jchars.len() && jchars[j + 1] == '/' {
            j += 2;
            while j < jchars.len() && jchars[j] != '\n' {
                j += 1;
            }
            continue;
        }
        if c == '/' && j + 1 < jchars.len() && jchars[j + 1] == '*' {
            j += 2;
            out.push(' ');
            while j < jchars.len() {
                if jchars[j] == '\n' {
                    out.push('\n');
                    j += 1;
                } else if jchars[j] == '*' && j + 1 < jchars.len() && jchars[j + 1] == '/' {
                    j += 2;
                    break;
                } else {
                    j += 1;
                }
            }
            continue;
        }
        out.push(c);
        j += 1;
    }
    out
}

fn expand_if_expr(
    expr: &str,
    macros: &HashMap<String, MacroDef>,
    file: &Path,
    line_no: usize,
) -> String {
    let mut out = String::new();
    let bytes = expr.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if expr[i..].starts_with("defined") {
            let after = i + 7;
            let prev_ok = i == 0 || !is_ident_continue(bytes[i - 1] as char);
            let next_ok = after >= bytes.len() || !is_ident_continue(bytes[after] as char);
            if prev_ok && next_ok {
                let mut pos = after;
                while pos < bytes.len() && (bytes[pos] as char).is_ascii_whitespace() {
                    pos += 1;
                }
                if pos < bytes.len() && bytes[pos] == b'(' {
                    if let Some(close_rel) = expr[pos..].find(')') {
                        out.push_str(&expr[i..=pos + close_rel]);
                        i = pos + close_rel + 1;
                        continue;
                    }
                } else if pos < bytes.len() && is_ident_start(bytes[pos] as char) {
                    let mut end = pos + 1;
                    while end < bytes.len() && is_ident_continue(bytes[end] as char) {
                        end += 1;
                    }
                    out.push_str(&expr[i..end]);
                    i = end;
                    continue;
                }
            }
        }
        let ch = expr[i..].chars().next().unwrap_or(' ');
        if is_ident_start(ch) {
            let mut end = i + ch.len_utf8();
            while end < expr.len() {
                let next_c = expr[end..].chars().next().unwrap_or(' ');
                if !is_ident_continue(next_c) {
                    break;
                }
                end += next_c.len_utf8();
            }
            let ident = &expr[i..end];
            if macros.contains_key(ident) {
                let mut slice_end = end;
                if let Some(MacroDef::Function { .. }) = macros.get(ident) {
                    let ws = skip_ws(expr, end);
                    if expr[ws..].starts_with('(')
                        && let Some((_, close_idx)) = parse_macro_call_args(expr, ws)
                    {
                        slice_end = close_idx + 1;
                    }
                }
                let mut hidden = HashSet::new();
                let expanded =
                    expand_text(&expr[i..slice_end], macros, &mut hidden, file, line_no, 0);
                out.push_str(&expanded);
                i = slice_end;
                continue;
            }
            out.push_str(ident);
            i = end;
            continue;
        }
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn expand_text(
    input: &str,
    macros: &HashMap<String, MacroDef>,
    hidden: &mut HashSet<String>,
    file: &Path,
    line_no: usize,
    depth: usize,
) -> String {
    if depth >= MAX_MACRO_EXPAND_DEPTH {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        let ch = input[i..].chars().next().unwrap_or(' ');
        if ch == '"' || ch == '\'' {
            let end = scan_quoted_literal(input, i);
            out.push_str(&input[i..end]);
            i = end;
            continue;
        }
        if is_ident_start(ch) {
            let end = scan_ident_end(input, i);
            let ident = &input[i..end];
            match ident {
                "__LINE__" => {
                    out.push_str(&line_no.to_string());
                    i = end;
                    continue;
                }
                "__FILE__" => {
                    let escaped = file.display().to_string().replace('\\', "\\\\");
                    out.push('"');
                    out.push_str(&escaped);
                    out.push('"');
                    i = end;
                    continue;
                }
                _ => {}
            }
            if !hidden.contains(ident)
                && let Some(def) = macros.get(ident)
            {
                match def {
                    MacroDef::Object(body) => {
                        hidden.insert(ident.to_string());
                        let replaced = expand_text(body, macros, hidden, file, line_no, depth + 1);
                        hidden.remove(ident);
                        out.push_str(&replaced);
                        i = end;
                        continue;
                    }
                    MacroDef::Function {
                        params,
                        variadic,
                        body,
                    } => {
                        let open_idx = skip_ws(input, end);
                        if input[open_idx..].starts_with('(')
                            && let Some((raw_args, close_idx)) =
                                parse_macro_call_args(input, open_idx)
                        {
                            let substituted = substitute_fn_macro(
                                params, *variadic, body, &raw_args, macros, hidden, file, line_no,
                                depth,
                            );
                            hidden.insert(ident.to_string());
                            let replaced =
                                expand_text(&substituted, macros, hidden, file, line_no, depth + 1);
                            hidden.remove(ident);
                            out.push_str(&replaced);
                            i = close_idx + 1;
                            continue;
                        }
                    }
                }
            }
            out.push_str(ident);
            i = end;
            continue;
        }
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn substitute_fn_macro(
    params: &[String],
    variadic: bool,
    body: &str,
    raw_args: &[String],
    macros: &HashMap<String, MacroDef>,
    hidden: &mut HashSet<String>,
    file: &Path,
    line_no: usize,
    depth: usize,
) -> String {
    let mut arg_map: HashMap<&str, String> = HashMap::new();
    for (idx, param) in params.iter().enumerate() {
        let val = raw_args.get(idx).cloned().unwrap_or_default();
        arg_map.insert(param.as_str(), val);
    }
    let va_args = if variadic && raw_args.len() > params.len() {
        raw_args[params.len()..].join(", ")
    } else {
        String::new()
    };

    let mut out = String::with_capacity(body.len());
    let mut i = 0;
    while i < body.len() {
        let ch = body[i..].chars().next().unwrap_or(' ');
        if ch == '"' || ch == '\'' {
            let end = scan_quoted_literal(body, i);
            out.push_str(&body[i..end]);
            i = end;
            continue;
        }
        // ## __VA_ARGS__
        if ch == ',' {
            let after_comma = skip_ws(body, i + 1);
            if body[after_comma..].starts_with("##") {
                let after_paste = skip_ws(body, after_comma + 2);
                if body[after_paste..].starts_with("__VA_ARGS__") {
                    let end_va = after_paste + "__VA_ARGS__".len();
                    let next_ok = end_va == body.len()
                        || !is_ident_continue(body[end_va..].chars().next().unwrap_or(' '));
                    if next_ok {
                        if !va_args.trim().is_empty() {
                            out.push_str(", ");
                            out.push_str(&va_args);
                        }
                        i = end_va;
                        continue;
                    }
                }
            }
        }

        if body[i..].starts_with("##") {
            while out.ends_with(char::is_whitespace) {
                out.pop();
            }
            i = skip_ws(body, i + 2);
            continue;
        }

        if ch == '#' {
            let after_hash = skip_ws(body, i + 1);
            if after_hash < body.len()
                && is_ident_start(body[after_hash..].chars().next().unwrap_or(' '))
            {
                let end = scan_ident_end(body, after_hash);
                let p_name = &body[after_hash..end];
                let val = if p_name == "__VA_ARGS__" {
                    Some(va_args.as_str())
                } else {
                    arg_map.get(p_name).map(String::as_str)
                };
                if let Some(raw_val) = val {
                    out.push('"');
                    let escaped = raw_val.trim().replace('\\', "\\\\").replace('"', "\\\"");
                    out.push_str(&escaped);
                    out.push('"');
                    i = end;
                    continue;
                }
            }
        }
        if is_ident_start(ch) {
            let end = scan_ident_end(body, i);
            let ident = &body[i..end];
            let next_non_ws = skip_ws(body, end);
            let is_pasted = out.ends_with("##") || body[next_non_ws..].starts_with("##");
            if ident == "__VA_ARGS__" && variadic {
                if is_pasted {
                    out.push_str(&va_args);
                } else {
                    out.push_str(&expand_text(&va_args, macros, hidden, file, line_no, depth + 1));
                }
                i = end;
                continue;
            }
            if let Some(raw_val) = arg_map.get(ident) {
                if is_pasted {
                    out.push_str(raw_val.trim());
                } else {
                    out.push_str(&expand_text(
                        raw_val.trim(),
                        macros,
                        hidden,
                        file,
                        line_no,
                        depth + 1,
                    ));
                }
                i = end;
                continue;
            }
            out.push_str(ident);
            i = end;
            continue;
        }
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn scan_quoted_literal(input: &str, start: usize) -> usize {
    let quote = input[start..].chars().next().unwrap_or('"');
    let mut i = start + quote.len_utf8();
    while i < input.len() {
        let ch = input[i..].chars().next().unwrap_or(quote);
        i += ch.len_utf8();
        if ch == '\\' && i < input.len() {
            let esc = input[i..].chars().next().unwrap_or(' ');
            i += esc.len_utf8();
        } else if ch == quote {
            break;
        }
    }
    i
}

fn scan_ident_end(input: &str, start: usize) -> usize {
    let mut i = start;
    while i < input.len() {
        let ch = input[i..].chars().next().unwrap_or(' ');
        if !is_ident_continue(ch) {
            break;
        }
        i += ch.len_utf8();
    }
    i
}

fn skip_ws(input: &str, mut pos: usize) -> usize {
    while pos < input.len() {
        let ch = input[pos..].chars().next().unwrap_or(' ');
        if !ch.is_ascii_whitespace() {
            break;
        }
        pos += ch.len_utf8();
    }
    pos
}

fn parse_macro_call_args(input: &str, open_idx: usize) -> Option<(Vec<String>, usize)> {
    if !input[open_idx..].starts_with('(') {
        return None;
    }
    let mut args = Vec::new();
    let mut current = String::new();
    let mut depth = 1usize;
    let mut i = open_idx + 1;
    while i < input.len() {
        let ch = input[i..].chars().next()?;
        if ch == '"' || ch == '\'' {
            let end = scan_quoted_literal(input, i);
            current.push_str(&input[i..end]);
            i = end;
            continue;
        }
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    if !args.is_empty() || !current.trim().is_empty() {
                        args.push(current.trim().to_string());
                    }
                    return Some((args, i));
                }
                current.push(ch);
            }
            ',' if depth == 1 => {
                args.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
        i += ch.len_utf8();
    }
    None
}

fn default_target_macros(target: TargetTriple) -> HashMap<String, MacroDef> {
    let spec = target.spec();
    let mut m = HashMap::new();
    let mut set = |k: &str, v: &str| {
        m.insert(k.to_string(), MacroDef::Object(v.to_string()));
    };
    set("__STDC__", "1");
    set("__STDC_HOSTED__", "1");
    set("__STDC_VERSION__", "201112L");
    set("__GNUC__", "12");
    set("__BYTE_ORDER__", "1234");
    set("__ORDER_LITTLE_ENDIAN__", "1234");
    set("__ORDER_BIG_ENDIAN__", "4321");
    set("__CHAR_BIT__", "8");
    set("__SIZEOF_INT__", "4");
    set("__SIZEOF_SHORT__", "2");
    set("__SIZEOF_LONG_LONG__", "8");
    set("__SIZEOF_FLOAT__", "4");
    set("__SIZEOF_DOUBLE__", "8");

    let ptr_bytes = (spec.pointer_width / 8).to_string();
    set("__SIZEOF_POINTER__", &ptr_bytes);
    set("__SIZEOF_SIZE_T__", &ptr_bytes);
    match spec.data_model {
        DataModel::Lp64 => {
            set("__LP64__", "1");
            set("_LP64", "1");
            set("__SIZEOF_LONG__", "8");
        }
        DataModel::Llp64 => {
            set("_LLP64", "1");
            set("__LLP64__", "1");
            set("__SIZEOF_LONG__", "4");
        }
        DataModel::Ilp32 => {
            set("__ILP32__", "1");
            set("__SIZEOF_LONG__", "4");
        }
    }
    match spec.arch {
        Architecture::X86_64 => {
            set("__x86_64__", "1");
            set("__x86_64", "1");
            set("__amd64__", "1");
            set("__amd64", "1");
            if spec.os == OperatingSystem::Windows {
                set("_M_X64", "100");
                set("_M_AMD64", "100");
            }
        }
        Architecture::Aarch64 => {
            set("__aarch64__", "1");
        }
        Architecture::Armv7 => {
            set("__arm__", "1");
        }
    }
    match spec.os {
        OperatingSystem::Linux => {
            set("__linux__", "1");
            set("__linux", "1");
            set("linux", "1");
            set("__unix__", "1");
            set("__unix", "1");
        }
        OperatingSystem::Android => {
            set("__ANDROID__", "1");
            set("__linux__", "1");
            set("__unix__", "1");
        }
        OperatingSystem::Windows => {
            set("_WIN32", "1");
            set("__WIN32__", "1");
            if spec.pointer_width == 64 {
                set("_WIN64", "1");
            }
        }
    }
    m
}

fn builtin_standard_header(name: &str, target: TargetTriple) -> Option<String> {
    let spec = target.spec();
    let long_is_64 = spec.data_model == DataModel::Lp64;
    let ptr_is_64 = spec.pointer_width == 64;
    let long_max = if long_is_64 {
        "9223372036854775807L"
    } else {
        "2147483647L"
    };
    let ulong_max = if long_is_64 {
        "18446744073709551615UL"
    } else {
        "4294967295UL"
    };
    match name {
        "stdint.h" => {
            let int64_ty = if long_is_64 {
                "typedef signed long int64_t;\ntypedef unsigned long uint64_t;\n"
            } else {
                "typedef signed long long int64_t;\ntypedef unsigned long long uint64_t;\n"
            };
            let ptr_ty = if ptr_is_64 {
                "typedef int64_t intptr_t;\ntypedef uint64_t uintptr_t;\n"
            } else {
                "typedef int32_t intptr_t;\ntypedef uint32_t uintptr_t;\n"
            };
            Some(format!(
                "#pragma once\n\
                 typedef signed char int8_t;\n\
                 typedef unsigned char uint8_t;\n\
                 typedef signed short int16_t;\n\
                 typedef unsigned short uint16_t;\n\
                 typedef signed int int32_t;\n\
                 typedef unsigned int uint32_t;\n\
                 {int64_ty}\
                 {ptr_ty}\
                 typedef int64_t intmax_t;\n\
                 typedef uint64_t uintmax_t;\n\
                 #define INT8_MIN (-128)\n\
                 #define INT8_MAX 127\n\
                 #define UINT8_MAX 255\n\
                 #define INT16_MIN (-32768)\n\
                 #define INT16_MAX 32767\n\
                 #define UINT16_MAX 65535\n\
                 #define INT32_MIN (-2147483648)\n\
                 #define INT32_MAX 2147483647\n\
                 #define UINT32_MAX 4294967295U\n\
                 #define INT64_MAX 9223372036854775807LL\n\
                 #define INT64_MIN (-9223372036854775807LL - 1)\n\
                 #define UINT64_MAX 18446744073709551615ULL\n\
                 #define SIZE_MAX UINT64_MAX\n"
            ))
        }
        "stddef.h" => {
            let size_decl = if long_is_64 {
                "typedef unsigned long size_t;\ntypedef signed long ptrdiff_t;\n"
            } else if ptr_is_64 {
                "typedef unsigned long long size_t;\ntypedef signed long long ptrdiff_t;\n"
            } else {
                "typedef unsigned int size_t;\ntypedef signed int ptrdiff_t;\n"
            };
            Some(format!(
                "#pragma once\n\
                 {size_decl}\
                 typedef int wchar_t;\n\
                 #ifndef NULL\n\
                 #define NULL 0\n\
                 #endif\n"
            ))
        }
        "stdbool.h" => Some(
            "#pragma once\n\
             #define bool _Bool\n\
             #define true 1\n\
             #define false 0\n\
             #define __bool_true_false_are_defined 1\n"
                .to_string(),
        ),
        "limits.h" => Some(format!(
            "#pragma once\n\
             #define CHAR_BIT 8\n\
             #define SCHAR_MIN (-128)\n\
             #define SCHAR_MAX 127\n\
             #define UCHAR_MAX 255\n\
             #define SHRT_MIN (-32768)\n\
             #define SHRT_MAX 32767\n\
             #define USHRT_MAX 65535\n\
             #define INT_MIN (-2147483648)\n\
             #define INT_MAX 2147483647\n\
             #define UINT_MAX 4294967295U\n\
             #define LONG_MAX {long_max}\n\
             #define LONG_MIN (-{long_max} - 1)\n\
             #define ULONG_MAX {ulong_max}\n\
             #define LLONG_MAX 9223372036854775807LL\n\
             #define LLONG_MIN (-9223372036854775807LL - 1)\n\
             #define ULLONG_MAX 18446744073709551615ULL\n"
        )),
        "float.h" => Some(
            "#pragma once\n\
             #define FLT_MANT_DIG 24\n\
             #define DBL_MANT_DIG 53\n\
             #define FLT_DIG 6\n\
             #define DBL_DIG 15\n\
             #define FLT_MIN 1.17549435e-38F\n\
             #define FLT_MAX 3.40282347e+38F\n\
             #define DBL_MIN 2.2250738585072014e-308\n\
             #define DBL_MAX 1.7976931348623157e+308\n"
                .to_string(),
        ),
        "inttypes.h" => Some("#pragma once\n#include <stdint.h>\n".to_string()),
        "stdarg.h" => Some(
            "#pragma once\n\
             typedef void *va_list;\n\
             typedef void *__gnuc_va_list;\n"
                .to_string(),
        ),
        "sys/types.h" => {
            let ssize_ty = if long_is_64 {
                "typedef signed long ssize_t;\n"
            } else if ptr_is_64 {
                "typedef signed long long ssize_t;\n"
            } else {
                "typedef signed int ssize_t;\n"
            };
            Some(format!(
                "#pragma once\n\
                 #include <stddef.h>\n\
                 #include <stdint.h>\n\
                 {ssize_ty}\
                 typedef int64_t off_t;\n\
                 typedef int32_t pid_t;\n\
                 typedef uint32_t uid_t;\n\
                 typedef uint32_t gid_t;\n\
                 typedef uint32_t mode_t;\n"
            ))
        }
        "stdio.h" => Some(
            "#pragma once\n\
             #include <stddef.h>\n\
             #include <stdarg.h>\n\
             typedef struct _IO_FILE FILE;\n"
                .to_string(),
        ),
        "stdlib.h" => Some(
            "#pragma once\n\
             #include <stddef.h>\n\
             void *malloc(size_t size);\n\
             void *calloc(size_t nmemb, size_t size);\n\
             void *realloc(void *ptr, size_t size);\n\
             void free(void *ptr);\n"
                .to_string(),
        ),
        "string.h" => Some(
            "#pragma once\n\
             #include <stddef.h>\n\
             void *memcpy(void *dest, const void *src, size_t n);\n\
             void *memset(void *s, int c, size_t n);\n\
             size_t strlen(const char *s);\n"
                .to_string(),
        ),
        _ => None,
    }
}
