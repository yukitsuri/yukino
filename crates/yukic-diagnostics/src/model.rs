use yukic_span::Span;

use crate::code::DiagnosticCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Note,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LabelStyle {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel {
    pub span: Span,
    pub message: String,
    pub style: LabelStyle,
}

impl DiagnosticLabel {
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            style: LabelStyle::Primary,
        }
    }

    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            style: LabelStyle::Secondary,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub span: Span,
    pub replacement: String,
    pub description: String,
}

impl Suggestion {
    pub fn new(span: Span, replacement: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            span,
            replacement: replacement.into(),
            description: description.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub primary_span: Option<Span>,
    pub labels: Vec<DiagnosticLabel>,
    pub notes: Vec<String>,
    pub suggestions: Vec<Suggestion>,
}

impl Diagnostic {
    pub fn error(code: impl Into<DiagnosticCode>, message: impl Into<String>, span: Span) -> Self {
        Self {
            code: code.into(),
            severity: DiagnosticSeverity::Error,
            message: message.into(),
            primary_span: Some(span),
            labels: vec![DiagnosticLabel::primary(span, "")],
            notes: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn warning(
        code: impl Into<DiagnosticCode>,
        message: impl Into<String>,
        span: Span,
    ) -> Self {
        Self {
            code: code.into(),
            severity: DiagnosticSeverity::Warning,
            message: message.into(),
            primary_span: Some(span),
            labels: vec![DiagnosticLabel::primary(span, "")],
            notes: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_primary_label(mut self, span: Span, message: impl Into<String>) -> Self {
        if self.labels.len() == 1
            && self.labels[0].message.is_empty()
            && self.labels[0].span == span
        {
            self.labels[0].message = message.into();
        } else {
            self.labels.push(DiagnosticLabel::primary(span, message));
        }
        self
    }

    #[must_use]
    pub fn with_secondary_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(DiagnosticLabel::secondary(span, message));
        self
    }

    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    #[must_use]
    pub fn with_suggestion(
        mut self,
        span: Span,
        replacement: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        self.suggestions
            .push(Suggestion::new(span, replacement, description));
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == DiagnosticSeverity::Error
    }

    pub fn is_warning(&self) -> bool {
        self.severity == DiagnosticSeverity::Warning
    }
}
