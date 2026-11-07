use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use crate::project::ProjectError;

struct SymbolGroup {
    module: &'static str,
    symbols: &'static [&'static str],
}

static CORE_GROUPS: &[SymbolGroup] = &[
    SymbolGroup {
        module: "core.utils",
        symbols: &["cast", "assert", "assert_eq", "assert_ne"],
    },
    SymbolGroup {
        module: "core.panic",
        symbols: &["panic"],
    },
    SymbolGroup {
        module: "core.process",
        symbols: &["exit", "args", "raw_args"],
    },
    SymbolGroup {
        module: "core.mem",
        symbols: &[
            "ptr_of",
            "from_raw_parts",
            "region_used",
            "region_cap",
            "region_reset",
            "export",
            "persistent",
            "Arena",
        ],
    },
    SymbolGroup {
        module: "core.macro",
        symbols: &["typeof"],
    },
];

pub fn run_fix_1(target_dir: &Path) -> Result<(), ProjectError> {
    if !target_dir.exists() {
        return Err(ProjectError::Io(format!("target directory not found: {}", target_dir.display())));
    }
    fix_manifests(target_dir)?;
    fix_ykn_files(target_dir)?;
    Ok(())
}

fn fix_manifests(target_dir: &Path) -> Result<(), ProjectError> {
    let mut toml_files = Vec::new();
    collect_toml_files(target_dir, &mut toml_files);
    for toml_path in toml_files {
        fix_manifest_includes(&toml_path)?;
    }
    Ok(())
}

fn collect_toml_files(dir: &Path, acc: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if should_skip_path(&p) {
            continue;
        }
        if p.is_dir() {
            collect_toml_files(&p, acc);
        } else if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if name == "yuki.toml" || name == "lib.toml" {
                acc.push(p);
            }
        }
    }
}

fn fix_manifest_includes(path: &Path) -> Result<(), ProjectError> {
    let content = fs::read_to_string(path).map_err(|e| ProjectError::Io(e.to_string()))?;
    if content.contains("\"core\"") {
        return Ok(());
    }
    let updated = update_includes_in_toml(&content);
    if updated != content {
        fs::write(path, updated).map_err(|e| ProjectError::Io(e.to_string()))?;
    }
    Ok(())
}

fn update_includes_in_toml(content: &str) -> String {
    let mut lines = Vec::new();
    let mut in_section = false;
    let mut found_includes = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed == "[bin]" || trimmed == "[lib]" || trimmed == "[package]";
        }
        if in_section && trimmed.starts_with("includes =") {
            found_includes = true;
            lines.push(inject_core_into_includes_line(line));
            continue;
        }
        lines.push(line.to_string());
    }
    if !found_includes {
        return inject_includes_after_section_header(&lines);
    }
    lines.join("\n")
}

fn inject_core_into_includes_line(line: &str) -> String {
    if let Some(bracket_pos) = line.find('[') {
        let (prefix, rest) = line.split_at(bracket_pos + 1);
        if rest.trim_start().starts_with(']') {
            return format!("{prefix}\"core\"]");
        }
        return format!("{prefix}\"core\", {}", rest.trim_start());
    }
    line.to_string()
}

fn inject_includes_after_section_header(lines: &[String]) -> String {
    let mut result = Vec::new();
    for line in lines {
        result.push(line.clone());
        let trimmed = line.trim();
        if trimmed == "[bin]" || trimmed == "[lib]" || trimmed == "[package]" {
            result.push("includes = [\"core\", \"stdlib\"]".to_string());
        }
    }
    result.join("\n")
}

fn fix_ykn_files(target_dir: &Path) -> Result<(), ProjectError> {
    let mut ykn_files = Vec::new();
    collect_ykn_files(target_dir, &mut ykn_files);
    for file in ykn_files {
        fix_single_ykn_file(&file)?;
    }
    Ok(())
}

fn collect_ykn_files(dir: &Path, acc: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if should_skip_path(&p) {
            continue;
        }
        if p.is_dir() {
            collect_ykn_files(&p, acc);
        } else if p.extension().is_some_and(|ext| ext == "ykn") {
            acc.push(p);
        }
    }
}

fn should_skip_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains("yuki-work")
        || s.contains("yukino-work")
        || s.contains("target")
        || s.contains(".git")
        || s.contains("builtin/core")
}

fn fix_single_ykn_file(path: &Path) -> Result<(), ProjectError> {
    let content = fs::read_to_string(path).map_err(|e| ProjectError::Io(e.to_string()))?;
    if content.contains("!#[direct_internal_access]") {
        return Ok(());
    }
    let s1 = replace_legacy_rt_and_macros(&content);
    let s2 = replace_angle_bracket_casts(&s1);
    let s3 = inject_missing_core_loads(&s2);
    if s3 != content {
        fs::write(path, s3).map_err(|e| ProjectError::Io(e.to_string()))?;
    }
    Ok(())
}

pub fn replace_legacy_rt_and_macros(source: &str) -> String {
    let mut res = source
        .replace("rt::typeof!", "core::macro::typeof!")
        .replace("rt::typeof", "typeof")
        .replace("rt::cast", "cast")
        .replace("rt::exit", "exit")
        .replace("rt::args", "args")
        .replace("rt::raw_args", "raw_args")
        .replace("rt::ptr_of", "ptr_of")
        .replace("rt::region_used", "region_used")
        .replace("rt::region_capacity", "region_cap")
        .replace("rt::region_cap", "region_cap")
        .replace("rt::region_reset", "region_reset")
        .replace("include_string!", "include_str!")
        .replace("include_strings!", "include_str!");
    res = replace_bare_region_capacity(&res);
    res
}

fn replace_bare_region_capacity(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut pos = 0;
    while let Some(idx) = source[pos..].find("region_capacity") {
        let absolute = pos + idx;
        let before_ok = absolute == 0 || !is_ident_char(source.as_bytes()[absolute - 1]);
        let end_idx = absolute + "region_capacity".len();
        let after_ok = end_idx >= source.len() || !is_ident_char(source.as_bytes()[end_idx]);
        out.push_str(&source[pos..absolute]);
        if before_ok && after_ok {
            out.push_str("region_cap");
        } else {
            out.push_str("region_capacity");
        }
        pos = end_idx;
    }
    out.push_str(&source[pos..]);
    out
}

pub fn replace_angle_bracket_casts(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    let bytes = source.as_bytes();
    let len = bytes.len();

    while i < len {
        if i + 5 <= len && &bytes[i..i + 5] == b"cast<" {
            let before_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            if before_ok {
                if let Some((target_ty, expr, next_i)) = extract_cast_args(source, i + 4) {
                    out.push_str(&format!("cast({}, \"{}\")", expr.trim(), target_ty.trim()));
                    i = next_i;
                    continue;
                }
            }
        }
        let ch = match source[i..].chars().next() {
            Some(c) => c,
            None => break,
        };
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn extract_cast_args<'a>(source: &'a str, start_lt: usize) -> Option<(&'a str, &'a str, usize)> {
    let (target_ty, after_gt) = parse_balanced_angles(source, start_lt)?;
    let mut p = after_gt;
    let bytes = source.as_bytes();
    while p < bytes.len() && bytes[p].is_ascii_whitespace() {
        p += 1;
    }
    if p >= bytes.len() || bytes[p] != b'(' {
        return None;
    }
    let (expr, after_paren) = parse_balanced_parens(source, p)?;
    Some((target_ty, expr, after_paren))
}

fn parse_balanced_angles(source: &str, start: usize) -> Option<(&str, usize)> {
    let bytes = source.as_bytes();
    let mut depth = 0;
    let mut i = start;
    let inner_start = start + 1;

    while i < bytes.len() {
        let b = bytes[i];
        if b == b'<' {
            depth += 1;
        } else if b == b'>' {
            depth -= 1;
            if depth == 0 {
                return Some((&source[inner_start..i], i + 1));
            }
        }
        i += 1;
    }
    None
}

fn parse_balanced_parens(source: &str, start: usize) -> Option<(&str, usize)> {
    let bytes = source.as_bytes();
    let mut depth = 0;
    let mut i = start;
    let inner_start = start + 1;
    let mut in_str = false;

    while i < bytes.len() {
        let b = bytes[i];
        if b == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
            in_str = !in_str;
        } else if !in_str {
            if b == b'(' {
                depth += 1;
            } else if b == b')' {
                depth -= 1;
                if depth == 0 {
                    return Some((&source[inner_start..i], i + 1));
                }
            }
        }
        i += 1;
    }
    None
}

pub fn inject_missing_core_loads(source: &str) -> String {
    let needed = find_needed_core_imports(source);
    if needed.is_empty() {
        return source.to_string();
    }
    let insert_pos = find_load_insertion_point(source);
    let mut load_lines = String::new();
    for (module, syms) in needed {
        let sym_list = syms.into_iter().collect::<Vec<_>>().join(", ");
        load_lines.push_str(&format!("load {module}: {sym_list}\n"));
    }
    let mut out = String::new();
    out.push_str(&source[..insert_pos]);
    out.push_str(&load_lines);
    out.push_str(&source[insert_pos..]);
    out
}

fn find_needed_core_imports(source: &str) -> BTreeMap<&'static str, BTreeSet<String>> {
    let mut needed: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();
    for group in CORE_GROUPS {
        if is_module_fully_imported(source, group.module) {
            continue;
        }
        for &sym in group.symbols {
            if is_symbol_defined_in_file(source, sym) {
                continue;
            }
            if is_symbol_imported(source, sym) {
                continue;
            }
            if is_symbol_used(source, sym) {
                needed.entry(group.module).or_default().insert(sym.to_string());
            }
        }
    }
    needed
}

fn is_module_fully_imported(source: &str, module: &str) -> bool {
    let pattern1 = format!("load {module}\n");
    let pattern2 = format!("load {module}: *");
    source.contains(&pattern1) || source.contains(&pattern2) || source.contains("load core\n") || source.contains("load core: *")
}

fn is_symbol_imported(source: &str, sym: &str) -> bool {
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("load ") {
            if let Some((_, symbols)) = trimmed.split_once(':') {
                for s in symbols.split(',') {
                    if s.trim() == sym {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn is_symbol_defined_in_file(source: &str, sym: &str) -> bool {
    let prefixes = [
        format!("fun {sym}("),
        format!("fun {sym}<"),
        format!("cfun {sym}("),
        format!("class {sym}"),
        format!("struct {sym}"),
        format!("enum {sym}"),
        format!("static {sym}:"),
        format!("static {sym} ="),
    ];
    for p in &prefixes {
        if source.contains(p) {
            return true;
        }
    }
    false
}

fn is_symbol_used(source: &str, sym: &str) -> bool {
    if sym == "Arena" {
        return find_whole_word(source, "Arena");
    }
    let target = format!("{sym}(");
    let bytes = source.as_bytes();
    let mut pos = 0;
    while let Some(idx) = source[pos..].find(&target) {
        let abs = pos + idx;
        let before_ok = abs == 0 || !is_ident_char(bytes[abs - 1]);
        if before_ok {
            return true;
        }
        pos = abs + target.len();
    }
    if sym == "typeof" && source.contains("typeof!") {
        return true;
    }
    false
}

fn find_whole_word(source: &str, word: &str) -> bool {
    let bytes = source.as_bytes();
    let mut pos = 0;
    while let Some(idx) = source[pos..].find(word) {
        let abs = pos + idx;
        let before_ok = abs == 0 || !is_ident_char(bytes[abs - 1]);
        let end = abs + word.len();
        let after_ok = end >= bytes.len() || !is_ident_char(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        pos = end;
    }
    false
}

fn find_load_insertion_point(source: &str) -> usize {
    let mut pos = 0;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("load ") || trimmed.starts_with("public load ") {
            return pos;
        }
        pos += line.len() + 1;
    }

    let mut pos = 0;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("!#[") || trimmed.starts_with("#![") {
            pos += line.len() + 1;
            continue;
        }
        if trimmed.starts_with("//")
            || (trimmed.starts_with('#') && !trimmed.starts_with("#["))
            || trimmed.is_empty()
        {
            pos += line.len() + 1;
            continue;
        }
        break;
    }
    pos.min(source.len())
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_angle_bracket_casts() {
        let src = "set x = cast<int32>(10 + 20)";
        let res = replace_angle_bracket_casts(src);
        assert_eq!(res, "set x = cast(10 + 20, \"int32\")");

        let nested = "set p = cast<ptr<uint8>>(buffer)";
        let res_nested = replace_angle_bracket_casts(nested);
        assert_eq!(res_nested, "set p = cast(buffer, \"ptr<uint8>\")");
    }

    #[test]
    fn test_replace_legacy_rt() {
        let src = "rt::exit(0); rt::region_capacity(); include_string!(\"a\")";
        let res = replace_legacy_rt_and_macros(src);
        assert_eq!(res, "exit(0); region_cap(); include_str!(\"a\")");
    }

    #[test]
    fn test_inject_missing_core_loads() {
        let src = "fun main() {\n    panic(\"err\")\n}\n";
        let res = inject_missing_core_loads(src);
        assert!(res.contains("load core.panic: panic\n"));
    }
}
