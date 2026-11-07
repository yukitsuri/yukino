use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroSpan {
    pub file_id: u32,
    pub start: u32,
    pub end: u32,
    pub line: u32,
    pub col: u32,
}

impl MacroSpan {
    pub fn new(file_id: u32, start: u32, end: u32, line: u32, col: u32) -> Self {
        Self {
            file_id,
            start,
            end,
            line,
            col,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacroKind {
    FunctionLike,
    Mutation,
    Attribute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacroDiagnosticSeverity {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroDiagnostic {
    pub severity: MacroDiagnosticSeverity,
    pub message: String,
    pub span: Option<MacroSpan>,
}

impl MacroDiagnostic {
    pub fn error(message: impl Into<String>, span: Option<MacroSpan>) -> Self {
        Self {
            severity: MacroDiagnosticSeverity::Error,
            message: message.into(),
            span,
        }
    }

    pub fn warning(message: impl Into<String>, span: Option<MacroSpan>) -> Self {
        Self {
            severity: MacroDiagnosticSeverity::Warning,
            message: message.into(),
            span,
        }
    }

    pub fn note(message: impl Into<String>, span: Option<MacroSpan>) -> Self {
        Self {
            severity: MacroDiagnosticSeverity::Note,
            message: message.into(),
            span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroRequest {
    pub protocol_version: u32,
    pub macro_name: String,
    pub kind: MacroKind,
    pub input_tokens: String,
    pub attr_args: Option<String>,
    pub target_triple: String,
    pub profile: String,
    pub working_dir: Option<String>,
}

impl MacroRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        macro_name: impl Into<String>,
        kind: MacroKind,
        input_tokens: impl Into<String>,
        attr_args: Option<String>,
        target_triple: impl Into<String>,
        profile: impl Into<String>,
        working_dir: Option<String>,
    ) -> Self {
        Self {
            protocol_version: crate::MACRO_PROTOCOL_VERSION,
            macro_name: macro_name.into(),
            kind,
            input_tokens: input_tokens.into(),
            attr_args,
            target_triple: target_triple.into(),
            profile: profile.into(),
            working_dir,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroResponse {
    pub success: bool,
    pub output_tokens: Option<String>,
    pub tracked_files: Vec<String>,
    pub diagnostics: Vec<MacroDiagnostic>,
    pub error_message: Option<String>,
}

impl MacroResponse {
    pub fn success(output_tokens: impl Into<String>, tracked_files: Vec<String>) -> Self {
        Self {
            success: true,
            output_tokens: Some(output_tokens.into()),
            tracked_files,
            diagnostics: Vec::new(),
            error_message: None,
        }
    }

    pub fn failure(error_message: impl Into<String>) -> Self {
        let msg = error_message.into();
        Self {
            success: false,
            output_tokens: None,
            tracked_files: Vec::new(),
            diagnostics: vec![MacroDiagnostic::error(msg.clone(), None)],
            error_message: Some(msg),
        }
    }
}
