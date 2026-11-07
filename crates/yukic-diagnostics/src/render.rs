use owo_colors::OwoColorize;
use std::fmt::Write as _;
use yukic_span::{SourceMap, Span};

use crate::model::{Diagnostic, DiagnosticSeverity, LabelStyle};

const COLOR_ERROR: owo_colors::Rgb = owo_colors::Rgb(239, 68, 68);
const COLOR_WARNING: owo_colors::Rgb = owo_colors::Rgb(255, 140, 0);

pub fn render_diagnostic(diag: &Diagnostic, source_map: &SourceMap, color: bool) -> String {
    let mut out = String::new();
    let header = render_header(diag, color);
    out.push_str(&header);
    out.push('\n');

    let gutter_width = compute_gutter_width(diag, source_map);

    if let Some(span) = diag.primary_span {
        let loc = render_location(span, source_map, gutter_width, color);
        out.push_str(&loc);
        out.push('\n');

        let snippet = render_snippet(diag, source_map, gutter_width, color);
        if !snippet.is_empty() {
            out.push_str(&snippet);
            out.push('\n');
        }
    }

    let extra = render_notes_and_suggestions(diag, gutter_width, color);
    if !extra.is_empty() {
        out.push_str(&extra);
        out.push('\n');
    }

    out
}

fn compute_gutter_width(diag: &Diagnostic, source_map: &SourceMap) -> usize {
    let mut max_line = 1;
    for label in &diag.labels {
        if let Some((line, _)) = source_map.lookup_line_column(label.span) {
            max_line = max_line.max(line);
        }
    }
    if let Some(span) = diag.primary_span {
        let pos = source_map.lookup_line_column(span);
        if let Some((line, _)) = pos {
            max_line = max_line.max(line);
        }
    }
    max_line.to_string().len().max(2)
}

fn render_header(diag: &Diagnostic, color: bool) -> String {
    let code_str = diag.code.as_str();
    let (prefix, text) = match diag.severity {
        DiagnosticSeverity::Error => ("error", format!("error[{code_str}]")),
        DiagnosticSeverity::Warning => ("warning", format!("warning[{code_str}]")),
        DiagnosticSeverity::Note => ("note", "note".to_string()),
        DiagnosticSeverity::Help => ("help", "help".to_string()),
    };

    if color {
        match diag.severity {
            DiagnosticSeverity::Error => {
                format!("{}: {}", text.color(COLOR_ERROR).bold(), diag.message.bold())
            }
            DiagnosticSeverity::Warning => {
                format!("{}: {}", text.color(COLOR_WARNING).bold(), diag.message.bold())
            }
            DiagnosticSeverity::Note => format!("{}: {}", text.cyan().bold(), diag.message.bold()),
            DiagnosticSeverity::Help => format!("{}: {}", text.green().bold(), diag.message.bold()),
        }
    } else {
        format!("{prefix}[{code_str}]: {}", diag.message)
    }
}

fn render_location(span: Span, source_map: &SourceMap, gutter_width: usize, color: bool) -> String {
    let file = source_map.get_file(span.file_id);
    let path_str = file.map_or("unknown", |f| f.path.to_str().unwrap_or("unknown"));
    let (line, col) = source_map.lookup_line_column(span).unwrap_or((1, 1));
    let arrow = if color {
        "-->".blue().bold().to_string()
    } else {
        "-->".to_string()
    };
    let loc_indent = " ".repeat(gutter_width);
    format!("{loc_indent}{arrow} {path_str}:{line}:{col}")
}

fn render_snippet(diag: &Diagnostic, source_map: &SourceMap, gutter_width: usize, color: bool) -> String {
    let mut out = String::new();
    let pipe_str = if color {
        "|".blue().bold().to_string()
    } else {
        "|".to_string()
    };
    let _ = writeln!(out, "{:>gutter_width$} {pipe_str}", "");

    let mut sorted_labels = diag.labels.clone();
    sorted_labels.sort_by_key(|l| l.span.start);

    for label in &sorted_labels {
        let Some(file) = source_map.get_file(label.span.file_id) else {
            continue;
        };
        let Some(start_loc) = file.lookup_line_col(label.span.start) else {
            continue;
        };
        let line = start_loc.line;
        let col = start_loc.col;
        let Some(line_src) = file.line_content(line) else {
            continue;
        };
        let line_num_styled = if color {
            format!("{line:>gutter_width$}")
                .blue()
                .bold()
                .to_string()
        } else {
            format!("{line:>gutter_width$}")
        };
        let _ = writeln!(out, "{line_num_styled} {pipe_str} {line_src}");

        let col_offset = col.saturating_sub(1);
        let indent = " ".repeat(col_offset);
        let line_chars = line_src.chars().count();
        let max_width_on_line = line_chars.saturating_sub(col_offset).max(1);

        let width = match file.lookup_line_col(label.span.end) {
            Some(end_loc) if end_loc.line == line => {
                end_loc.col.saturating_sub(col).max(1).min(max_width_on_line)
            }
            _ => max_width_on_line,
        };
        let caret_char = match label.style {
            LabelStyle::Primary => '^',
            LabelStyle::Secondary => '-',
        };
        let carets = caret_char.to_string().repeat(width);

        let colored_carets = if color {
            match label.style {
                LabelStyle::Primary => match diag.severity {
                    DiagnosticSeverity::Warning => carets.color(COLOR_WARNING).bold().to_string(),
                    _ => carets.color(COLOR_ERROR).bold().to_string(),
                },
                LabelStyle::Secondary => carets.blue().bold().to_string(),
            }
        } else {
            carets
        };

        let msg = if label.message.is_empty() {
            String::new()
        } else {
            format!(" {}", label.message)
        };
        let _ = writeln!(
            out,
            "{:>w$} {} {}{}{}",
            "",
            pipe_str,
            indent,
            colored_carets,
            msg,
            w = gutter_width
        );
    }

    let _ = write!(out, "{:>w$} {}", "", pipe_str, w = gutter_width);
    out
}

fn render_notes_and_suggestions(diag: &Diagnostic, gutter_width: usize, color: bool) -> String {
    let mut out = String::new();
    let eq_sign = if color {
        "=".blue().bold().to_string()
    } else {
        "=".to_string()
    };
    let pipe_str = if color {
        "|".blue().bold().to_string()
    } else {
        "|".to_string()
    };

    for note in &diag.notes {
        let note_tag = if color {
            "note".bold().to_string()
        } else {
            "note".to_string()
        };
        let _ = writeln!(
            out,
            "{:>w$} {} {}: {}",
            "",
            eq_sign,
            note_tag,
            note,
            w = gutter_width
        );
    }

    for sugg in &diag.suggestions {
        let help_tag = if color {
            "help".green().bold().to_string()
        } else {
            "help".to_string()
        };
        let _ = writeln!(
            out,
            "{:>w$} {} {}: {}",
            "",
            eq_sign,
            help_tag,
            sugg.description,
            w = gutter_width
        );
        let _ = writeln!(out, "{:>w$} {}", "", pipe_str, w = gutter_width);
        for line in sugg.replacement.lines() {
            let _ = writeln!(out, "{:>w$} {} {}", "", pipe_str, line, w = gutter_width);
        }
    }

    out.trim_end().to_string()
}
