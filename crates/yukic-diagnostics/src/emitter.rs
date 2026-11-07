use std::io::Write;
use yukic_span::SourceMap;

use crate::filter::WarningFilter;
use crate::model::Diagnostic;
use crate::render::render_diagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChoice {
    Always,
    Never,
    Auto,
}

pub struct DiagnosticEmitter {
    color: ColorChoice,
    filter: WarningFilter,
    diagnostics: Vec<Diagnostic>,
    errors: usize,
    warnings: usize,
}

impl Default for DiagnosticEmitter { fn default() -> Self { Self::new(ColorChoice::Auto) } }

impl DiagnosticEmitter {
    pub fn new(color: ColorChoice) -> Self {
        Self {
            color,
            filter: WarningFilter::new(),
            diagnostics: Vec::new(),
            errors: 0,
            warnings: 0,
        }
    }

    pub fn set_filter(&mut self, filter: WarningFilter) {
        self.filter = filter;
    }

    pub fn filter_mut(&mut self) -> &mut WarningFilter {
        &mut self.filter
    }

    pub fn emit<W: Write>(
        &mut self,
        mut diag: Diagnostic,
        source_map: &SourceMap,
        writer: &mut W,
    ) -> std::io::Result<()> {
        if diag.is_warning() {
            if self.filter.is_denied(&diag.code) {
                diag.severity = crate::model::DiagnosticSeverity::Error;
            } else if self.filter.is_suppressed(&diag.code) {
                return Ok(());
            }
        }

        if diag.is_error() { self.errors += 1; } 
        else if diag.is_warning() { self.warnings += 1; }

        let use_color = match self.color {
            ColorChoice::Never => false,
            ColorChoice::Always | ColorChoice::Auto => true,
        };

        let rendered = render_diagnostic(&diag, source_map, use_color);
        writer.write_all(rendered.as_bytes())?;
        writer.write_all(b"\n")?;
        self.diagnostics.push(diag);
        Ok(())
    }

    pub fn error_count(&self) -> usize { self.errors }
    pub fn warning_count(&self) -> usize { self.warnings }
    pub fn has_errors(&self) -> bool { self.errors > 0 }
    pub fn diagnostics(&self) -> &[Diagnostic] { &self.diagnostics }
}
