#![deny(unsafe_code)]

use yukic_diagnostics::{Diagnostic, ErrorCode};
use yukic_span::FileId;

use crate::cursor::Cursor;
use crate::span_util::make_span;
use crate::token::{Token, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringMode { Regular, Multiline, }

pub enum StringScanResult<'a> {
    Complete(Token<'a>),
    Interpolation {
        part: Token<'a>,
        interp_start: Token<'a>,
    },
}

pub fn scan_char_literal<'a>(
    cursor: &mut Cursor<'a>,
    file_id: FileId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Token<'a> {
    let start = cursor.pos();
    cursor.bump();
    let mut ch = None;
    let mut count = 0;

    while let Some(b) = cursor.peek() {
        if b == b'\'' { break; }
        if b == b'\n' {
            let span = make_span(file_id, start, cursor.pos());
            diagnostics.push(Diagnostic::error(
                ErrorCode::E0105,
                "unterminated character literal",
                span,
            ));
            return Token::new(TokenKind::CharLiteral('\0'), span);
        }
        let decoded = if b == b'\\' {
            cursor.bump();
            scan_escape_sequence(cursor, file_id, diagnostics)
        } else { cursor.bump_char().unwrap() };
        ch = Some(decoded);
        count += 1;
    }

    if cursor.eat_if(b'\'') {
        let span = make_span(file_id, start, cursor.pos());
        if count == 0 {
            diagnostics.push(Diagnostic::error(
                ErrorCode::E0106,
                "empty character literal",
                span,
            ));
            Token::new(TokenKind::CharLiteral('\0'), span)
        } else if count > 1 {
            diagnostics.push(Diagnostic::error(
                ErrorCode::E0107,
                "multi-character literal, use string instead",
                span,
            ));
            Token::new(TokenKind::CharLiteral(ch.unwrap()), span)
        } else {
            Token::new(TokenKind::CharLiteral(ch.unwrap()), span)
        }
    } else {
        let span = make_span(file_id, start, cursor.pos());
        diagnostics.push(Diagnostic::error(
            ErrorCode::E0105,
            "unterminated character literal",
            span,
        ));
        let c = ch.unwrap_or('\0');
        Token::new(TokenKind::CharLiteral(c), span)
    }
}

pub fn scan_string_literal<'a>(
    cursor: &mut Cursor<'a>,
    file_id: FileId,
    mode: StringMode,
    diagnostics: &mut Vec<Diagnostic>,
) -> StringScanResult<'a> {
    let start = cursor.pos();
    consume_opening_quotes(cursor, mode);
    scan_string_body(cursor, file_id, start, mode, diagnostics)
}

pub fn resume_string_scan<'a>(
    cursor: &mut Cursor<'a>,
    file_id: FileId,
    mode: StringMode,
    diagnostics: &mut Vec<Diagnostic>,
) -> StringScanResult<'a> {
    let start = cursor.pos();
    scan_string_body(cursor, file_id, start, mode, diagnostics)
}

fn consume_opening_quotes(cursor: &mut Cursor<'_>, mode: StringMode) {
    match mode {
        StringMode::Regular => { cursor.bump(); }
        StringMode::Multiline => {
            cursor.bump();
            cursor.bump();
            cursor.bump();
        }
    }
}

fn scan_string_body<'a>(
    cursor: &mut Cursor<'a>,
    file_id: FileId,
    start: usize,
    mode: StringMode,
    diagnostics: &mut Vec<Diagnostic>,
) -> StringScanResult<'a> {
    let mut buf = String::new();

    while !cursor.is_eof() {
        if is_closing_quote(cursor, mode) {
            consume_closing_quotes(cursor, mode);
            let span = make_span(file_id, start, cursor.pos());
            return StringScanResult::Complete(Token::new(TokenKind::StringLiteral(buf), span));
        }

        if check_interpolation(cursor) {
            let part_span = make_span(file_id, start, cursor.pos());
            let interp_start_pos = cursor.pos();
            cursor.bump();
            cursor.bump();
            let interp_span = make_span(file_id, interp_start_pos, cursor.pos());
            return StringScanResult::Interpolation {
                part: Token::new(TokenKind::StringPart(buf), part_span),
                interp_start: Token::new(TokenKind::InterpolationStart, interp_span),
            };
        }

        if mode == StringMode::Regular && cursor.peek() == Some(b'\n') {
            let span = make_span(file_id, start, cursor.pos());
            diagnostics.push(Diagnostic::error(
                ErrorCode::E0102,
                "unterminated string literal",
                span,
            ));
            return StringScanResult::Complete(Token::new(TokenKind::StringLiteral(buf), span));
        }

        if cursor.peek() == Some(b'\\') {
            cursor.bump();
            let esc = scan_escape_sequence(cursor, file_id, diagnostics);
            buf.push(esc);
        } else {
            buf.push(cursor.bump_char().unwrap());
        }
    }

    let span = make_span(file_id, start, cursor.pos());
    let err_code = match mode {
        StringMode::Regular => ErrorCode::E0102,
        StringMode::Multiline => ErrorCode::E0103,
    };
    diagnostics.push(Diagnostic::error(
        err_code,
        "unterminated string literal at end of file",
        span,
    ));
    StringScanResult::Complete(Token::new(TokenKind::StringLiteral(buf), span))
}

fn is_closing_quote(cursor: &Cursor<'_>, mode: StringMode) -> bool {
    match mode {
        StringMode::Regular => cursor.peek() == Some(b'"'),
        StringMode::Multiline => {
            cursor.peek() == Some(b'"')
                && cursor.peek_at(1) == Some(b'"')
                && cursor.peek_at(2) == Some(b'"')
        }
    }
}

fn consume_closing_quotes(cursor: &mut Cursor<'_>, mode: StringMode) {
    match mode {
        StringMode::Regular => { cursor.bump(); }
        StringMode::Multiline => {
            cursor.bump();
            cursor.bump();
            cursor.bump();
        }
    }
}

fn check_interpolation(cursor: &Cursor<'_>) -> bool {
    cursor.peek() == Some(b'$') && cursor.peek_at(1) == Some(b'{')
}

fn scan_escape_sequence(
    cursor: &mut Cursor<'_>,
    file_id: FileId,
    diagnostics: &mut Vec<Diagnostic>,
) -> char {
    let esc_start = cursor.pos().saturating_sub(1);
    match cursor.bump() {
        Some(b'n') => '\n',
        Some(b't') => '\t',
        Some(b'r') => '\r',
        Some(b'\\') => '\\',
        Some(b'"') => '"',
        Some(b'\'') => '\'',
        Some(b'0') => '\0',
        Some(b'$') => '$',
        Some(b'x') => scan_hex_escape(cursor, file_id, esc_start, diagnostics),
        Some(b'u') => scan_unicode_escape(cursor, file_id, esc_start, diagnostics),
        Some(other) => {
            let span = make_span(file_id, esc_start, cursor.pos());
            diagnostics.push(Diagnostic::error(
                ErrorCode::E0108,
                format!("invalid escape sequence '\\{}'", char::from(other)),
                span,
            ));
            char::from(other)
        }
        None => {
            let span = make_span(file_id, esc_start, cursor.pos());
            diagnostics.push(Diagnostic::error(
                ErrorCode::E0108,
                "incomplete escape sequence at end of file",
                span,
            ));
            '\\'
        }
    }
}

fn scan_hex_escape(
    cursor: &mut Cursor<'_>,
    file_id: FileId,
    esc_start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> char {
    let h1 = cursor.bump().unwrap_or(b'0');
    let h2 = cursor.bump().unwrap_or(b'0');
    let hex_str = format!("{}{}", char::from(h1), char::from(h2));
    if let Ok(v) = u8::from_str_radix(&hex_str, 16) {
        char::from(v)
    } else {
        let span = make_span(file_id, esc_start, cursor.pos());
        diagnostics.push(Diagnostic::error(
            ErrorCode::E0108,
            format!("invalid hex escape '\\x{hex_str}'"),
            span,
        ));
        '\0'
    }
}

fn scan_unicode_escape(
    cursor: &mut Cursor<'_>,
    file_id: FileId,
    esc_start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> char {
    if !cursor.eat_if(b'{') {
        let span = make_span(file_id, esc_start, cursor.pos());
        diagnostics.push(Diagnostic::error(
            ErrorCode::E0108,
            "expected '{' after '\\u'",
            span,
        ));
        return '\0';
    }

    let mut hex = String::new();
    while let Some(b) = cursor.peek() {
        if b == b'}' { cursor.bump(); break; }
        if b.is_ascii_hexdigit() { hex.push(char::from(cursor.bump().unwrap())); } 
        else { break; }
    }

    if hex.is_empty() || hex.len() > 6 {
        let span = make_span(file_id, esc_start, cursor.pos());
        diagnostics.push(Diagnostic::error(
            ErrorCode::E0108,
            "unicode escape must have 1-6 hex digits",
            span,
        ));
        return '\0';
    }

    if let Some(c) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) { c } 
    else {
        let span = make_span(file_id, esc_start, cursor.pos());
        diagnostics.push(Diagnostic::error(
            ErrorCode::E0108,
            format!("invalid unicode codepoint '\\u{{{hex}}}'"),
            span,
        ));
        '\0'
    }
}
