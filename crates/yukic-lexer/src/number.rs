#![deny(unsafe_code)]

use yukic_diagnostics::{Diagnostic, ErrorCode};
use yukic_span::{FileId, Span};

use crate::cursor::Cursor;
use crate::span_util::make_span;
use crate::token::{FloatSuffix, IntSuffix, Radix, Token, TokenKind};

pub fn scan_number<'a>(
    cursor: &mut Cursor<'a>,
    file_id: FileId,
    diagnostics: &mut Vec<Diagnostic>,
) -> Token<'a> {
    let start = cursor.pos();
    let radix = scan_radix_prefix(cursor);
    let mut is_float = false;
    let mut digits = String::new();

    scan_digits(cursor, radix, &mut digits);

    if digits.is_empty() {
        let span = make_span(file_id, start, cursor.pos());
        diagnostics.push(Diagnostic::error(
            ErrorCode::E0109,
            "missing digits after numeric prefix",
            span,
        ));
    }

    if radix == Radix::Decimal && check_float_dot(cursor) {
        cursor.bump();
        digits.push('.');
        is_float = true;
        scan_digits(cursor, Radix::Decimal, &mut digits);
    }

    if radix == Radix::Decimal && check_exponent(cursor) {
        cursor.bump();
        digits.push('e');
        is_float = true;
        if cursor.peek() == Some(b'+') || cursor.peek() == Some(b'-') {
            digits.push(char::from(cursor.bump().unwrap()));
        }
        let exp_start = digits.len();
        scan_digits(cursor, Radix::Decimal, &mut digits);
        if digits.len() == exp_start {
            let span = make_span(file_id, start, cursor.pos());
            diagnostics.push(Diagnostic::error(
                ErrorCode::E0110,
                "missing digits after exponent",
                span,
            ));
        }
    }

    let (int_suffix, float_suffix, unknown_suffix) = scan_suffix(cursor);
    let span = make_span(file_id, start, cursor.pos());

    if let Some(bad) = unknown_suffix {
        diagnostics.push(Diagnostic::error(
            ErrorCode::E0109,
            format!("unknown numeric suffix '{bad}'"),
            span,
        ));
    }

    let kind = if is_float || float_suffix.is_some() {
        parse_float(&digits, float_suffix, span, diagnostics)
    } else {
        parse_int(&digits, radix, int_suffix, span, diagnostics)
    };

    Token::new(kind, span)
}

fn scan_radix_prefix(cursor: &mut Cursor<'_>) -> Radix {
    if cursor.peek() == Some(b'0') {
        match cursor.peek_at(1) {
            Some(b'x' | b'X') => {
                cursor.bump();
                cursor.bump();
                Radix::Hex
            }
            Some(b'b' | b'B') => {
                cursor.bump();
                cursor.bump();
                Radix::Binary
            }
            Some(b'o' | b'O') => {
                cursor.bump();
                cursor.bump();
                Radix::Octal
            }
            _ => Radix::Decimal,
        }
    } else {
        Radix::Decimal
    }
}

fn scan_digits(cursor: &mut Cursor<'_>, radix: Radix, buf: &mut String) {
    while let Some(b) = cursor.peek() {
        if b == b'_' {
            cursor.bump();
            continue;
        }
        let valid = match radix {
            Radix::Binary => matches!(b, b'0'..=b'1'),
            Radix::Octal => matches!(b, b'0'..=b'7'),
            Radix::Decimal => b.is_ascii_digit(),
            Radix::Hex => b.is_ascii_hexdigit(),
        };
        if valid { buf.push(char::from(cursor.bump().unwrap()));} 
        else { break; }
    }
}

fn check_float_dot(cursor: &Cursor<'_>) -> bool {
    if cursor.peek() == Some(b'.') {
        let next = cursor.peek_at(1);
        matches!(next, Some(b'0'..=b'9'))
    } 


    else { false }
}

fn check_exponent(cursor: &Cursor<'_>) -> bool {
    matches!(cursor.peek(), Some(b'e' | b'E'))
}

fn scan_suffix<'a>(
    cursor: &mut Cursor<'a>,
) -> (Option<IntSuffix>, Option<FloatSuffix>, Option<&'a str>) {
    let start = cursor.pos();
    while let Some(b) = cursor.peek() {
        if b.is_ascii_alphanumeric() || b == b'_' { cursor.bump();} 
        else { break; }
    }
    if start == cursor.pos() { return (None, None, None); }
    let s = cursor.slice(start, cursor.pos());
    match s {
        "i8" => (Some(IntSuffix::I8), None, None), // i - integer
        "i16" => (Some(IntSuffix::I16), None, None),
        "i32" => (Some(IntSuffix::I32), None, None),
        "i64" => (Some(IntSuffix::I64), None, None),
        "u8" => (Some(IntSuffix::U8), None, None), // u - unsig int
        "u16" => (Some(IntSuffix::U16), None, None),
        "u32" => (Some(IntSuffix::U32), None, None),
        "u64" => (Some(IntSuffix::U64), None, None),
        "f32" => (None, Some(FloatSuffix::F32), None), // f - float
        "f64" => (None, Some(FloatSuffix::F64), None),
        _ => (None, None, Some(s)),
    }
}

fn parse_int(
    digits: &str,
    radix: Radix,
    suffix: Option<IntSuffix>,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> TokenKind<'static> {
    if let Ok(v) = u64::from_str_radix(digits, radix.as_u32()) {
        TokenKind::IntLiteral {
            value: v,
            suffix,
            radix,
        }
    } else {
        diagnostics.push(Diagnostic::error(
            ErrorCode::E0109,
            "integer literal exceeds 64-bit capacity",
            span,
        ));
        TokenKind::IntLiteral {
            value: 0,
            suffix,
            radix,
        }
    }
}

fn parse_float(
    digits: &str,
    suffix: Option<FloatSuffix>,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> TokenKind<'static> {
    if let Ok(v) = digits.parse::<f64>() {
        TokenKind::FloatLiteral { value: v, suffix }
    } else {
        diagnostics.push(Diagnostic::error(
            ErrorCode::E0110,
            "malformed float literal",
            span,
        ));
        TokenKind::FloatLiteral { value: 0.0, suffix }
    }
}
