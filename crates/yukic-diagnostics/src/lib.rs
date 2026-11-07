#![deny(unsafe_code)]

pub mod code;
pub mod emitter;
pub mod filter;
pub mod model;
pub mod render;

pub use code::{DiagnosticCode, ErrorCode, PanicCode, WarningCode};
pub use emitter::{ColorChoice, DiagnosticEmitter};
pub use filter::{WarningFilter, WarningGroup};
pub use model::{Diagnostic, DiagnosticLabel, DiagnosticSeverity, LabelStyle, Suggestion};
pub use render::render_diagnostic;

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_span::{BytePos, SourceMap, Span};

    #[test]
    fn test_diagnostic_codes_parsing() {
        assert_eq!(
            "E0101".parse::<DiagnosticCode>().unwrap(),
            DiagnosticCode::Error(ErrorCode::E0101)
        );
        assert_eq!(
            "W0001".parse::<DiagnosticCode>().unwrap(),
            DiagnosticCode::Warning(WarningCode::W0001)
        );
        assert_eq!(
            "R0101".parse::<DiagnosticCode>().unwrap(),
            DiagnosticCode::Panic(PanicCode::R0101)
        );
        assert_eq!(ErrorCode::E0501.name(), "use_after_move");
        assert_eq!(WarningCode::W0415.name(), "suggest_switch");
        assert_eq!(PanicCode::R0102.name(), "division_by_zero");
    }

    #[test]
    fn test_lexer_diagnostic_rendering() {
        let mut map = SourceMap::new();
        let src = "let x = @123;\n";
        let file_id = map.add_file("src/lexer_test.ykn", src.to_string());
        let span = Span::new(file_id, BytePos(8), BytePos(9));

        let diag = Diagnostic::error(ErrorCode::E0101, "unexpected character '@'", span);
        let rendered = render_diagnostic(&diag, &map, false);

        assert!(rendered.contains("error[E0101]: unexpected character '@'"));
        assert!(rendered.contains("--> src/lexer_test.ykn:1:9"));
        assert!(rendered.contains("1 | let x = @123;"));
        assert!(rendered.contains('^'));
    }

    #[test]
    fn test_type_diagnostic_rendering() {
        let mut map = SourceMap::new();
        let src = "let count: int32 = \"not_a_number\";\n";
        let file_id = map.add_file("src/type_test.ykn", src.to_string());
        let span = Span::new(file_id, BytePos(19), BytePos(33));

        let diag = Diagnostic::error(
            ErrorCode::E0401,
            "type mismatch: expected 'int32', found 'string'",
            span,
        )
        .with_note("expected integer type for variable 'count'")
        .with_suggestion(span, "42", "replace string literal with an integer");

        let rendered = render_diagnostic(&diag, &map, false);
        assert!(rendered.contains("error[E0401]: type mismatch"));
        assert!(rendered.contains("--> src/type_test.ykn:1:20"));
        assert!(rendered.contains("^^^^^^^^^^^^^^"));
        assert!(rendered.contains("= note: expected integer type for variable 'count'"));
        assert!(rendered.contains("= help: replace string literal with an integer"));
        assert!(rendered.contains("42"));
    }

    #[test]
    fn test_borrow_checker_diagnostic_rendering() {
        let mut map = SourceMap::new();
        let src = "let file = open_file();\nlet a = file;\nlet b = file;\n";
        let file_id = map.add_file("src/borrow_test.ykn", src.to_string());

        let move_span = Span::new(file_id, BytePos(32), BytePos(36));
        let use_span = Span::new(file_id, BytePos(46), BytePos(50));

        let diag = Diagnostic::error(ErrorCode::E0501, "use of moved value 'file'", use_span)
            .with_secondary_label(move_span, "value moved here")
            .with_primary_label(use_span, "value used here after move")
            .with_note("'file' has affine type 'File' which does not implement Copy")
            .with_suggestion(
                move_span,
                "ref file",
                "borrow 'file' with 'ref' instead of transferring ownership",
            );

        let rendered = render_diagnostic(&diag, &map, false);
        assert!(rendered.contains("error[E0501]: use of moved value 'file'"));
        assert!(rendered.contains("--> src/borrow_test.ykn:3:9"));
        assert!(rendered.contains("value moved here"));
        assert!(rendered.contains("value used here after move"));
        assert!(rendered.contains("= note: 'file' has affine type 'File'"));
        assert!(rendered.contains("= help: borrow 'file' with 'ref'"));
    }

    #[test]
    fn test_emitter_tracking_and_colors() {
        let mut map = SourceMap::new();
        let src = "let mut x = 10;\n";
        let file_id = map.add_file("src/warn_test.ykn", src.to_string());
        let span = Span::new(file_id, BytePos(8), BytePos(9));

        let mut emitter = DiagnosticEmitter::new(ColorChoice::Never);
        let warn = Diagnostic::warning(WarningCode::W0001, "unused variable 'x'", span)
            .with_suggestion(
                span,
                "_x",
                "if this is intentional, prefix with an underscore",
            );

        let mut buf = Vec::new();
        emitter.emit(warn, &map, &mut buf).unwrap();

        assert_eq!(emitter.warning_count(), 1);
        assert_eq!(emitter.error_count(), 0);
        assert!(!emitter.has_errors());

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("warning[W0001]: unused variable 'x'"));
        assert!(output.contains("= help: if this is intentional, prefix with an underscore"));
    }

    #[test]
    fn test_warning_suppression_by_name_and_group() {
        let mut map = SourceMap::new();
        let src = "let x = 1;\nlet y = 2;\n";
        let file_id = map.add_file("src/suppress_test.ykn", src.to_string());
        let span_x = Span::new(file_id, BytePos(4), BytePos(5));
        let span_y = Span::new(file_id, BytePos(15), BytePos(16));

        let mut emitter = DiagnosticEmitter::new(ColorChoice::Never);
        emitter.filter_mut().ignore_name("unused_symbol");

        let warn1 = Diagnostic::warning(WarningCode::W0001, "unused variable 'x'", span_x);
        let warn2 = Diagnostic::warning(WarningCode::W0002, "unused import 'y'", span_y);

        let mut buf = Vec::new();
        emitter.emit(warn1, &map, &mut buf).unwrap();
        emitter.emit(warn2, &map, &mut buf).unwrap();

        assert_eq!(emitter.warning_count(), 0);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_warning_suppression_all() {
        let mut map = SourceMap::new();
        let src = "let x = 1;\nlet y = 2;\n";
        let file_id = map.add_file("src/suppress_all_test.ykn", src.to_string());
        let span_x = Span::new(file_id, BytePos(4), BytePos(5));
        let span_y = Span::new(file_id, BytePos(15), BytePos(16));

        let mut emitter = DiagnosticEmitter::new(ColorChoice::Never);
        emitter.filter_mut().suppress_all();

        let warn1 = Diagnostic::warning(WarningCode::W0001, "unused variable 'x'", span_x);
        let warn2 = Diagnostic::warning(WarningCode::W0415, "suggest switch", span_y);

        let mut buf = Vec::new();
        emitter.emit(warn1, &map, &mut buf).unwrap();
        emitter.emit(warn2, &map, &mut buf).unwrap();

        assert_eq!(emitter.warning_count(), 0);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_warning_escalated_to_error_by_deny() {
        let mut map = SourceMap::new();
        let src = "let x = 1;\n";
        let file_id = map.add_file("src/deny_test.ykn", src.to_string());
        let span_x = Span::new(file_id, BytePos(4), BytePos(5));

        let mut emitter = DiagnosticEmitter::new(ColorChoice::Never);
        emitter.filter_mut().deny_name("unused_variable");

        let warn = Diagnostic::warning(WarningCode::W0001, "unused variable 'x'", span_x);
        let mut buf = Vec::new();
        emitter.emit(warn, &map, &mut buf).unwrap();

        assert_eq!(emitter.warning_count(), 0);
        assert_eq!(emitter.error_count(), 1);
        assert!(emitter.has_errors());
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("error[W0001]: unused variable 'x'"));
    }

    #[test]
    fn test_diagnostic_colored_rgb_output() {
        let mut map = SourceMap::new();
        let src = "let x = 1;\n";
        let file_id = map.add_file("src/color_test.ykn", src.to_string());
        let span = Span::new(file_id, BytePos(4), BytePos(5));

        let warn = Diagnostic::warning(WarningCode::W0001, "unused variable 'x'", span);
        let rendered_warn = render_diagnostic(&warn, &map, true);
        assert!(rendered_warn.contains("\x1b[38;2;255;140;0m"));

        let err = Diagnostic::error(ErrorCode::E0101, "bad token", span);
        let rendered_err = render_diagnostic(&err, &map, true);
        assert!(rendered_err.contains("\x1b[38;2;239;68;68m"));
    }

    #[test]
    fn test_multiline_span_diagnostic_rendering() {
        let mut map = SourceMap::new();
        let src = "if x == 1 {\n    do_a();\n} elif x == 2 {\n    do_b();\n}\n";
        let file_id = map.add_file("src/test.ykn", src.to_string());
        let span = Span::new(file_id, BytePos(0), BytePos(src.len() as u32));

        let diag = Diagnostic::warning(
            WarningCode::W0415,
            "chained 'if-elif' comparisons on 'x' can be simplified with 'switch'",
            span,
        );
        let rendered = render_diagnostic(&diag, &map, false);

        assert!(rendered.contains("1 | if x == 1 {"));
        assert!(rendered.contains("  | ^^^^^^^^^^^"));
        assert!(!rendered.contains("^^^^^^^^^^^^"));
    }
}
