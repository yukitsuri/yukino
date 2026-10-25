#![deny(unsafe_code)]

use yukic_diagnostics::{Diagnostic, ErrorCode, WarningCode};
use yukic_span::FileId;

use crate::cursor::Cursor;
use crate::number::scan_number;
use crate::span_util::make_span;
use crate::string::{
    StringMode, StringScanResult, resume_string_scan, scan_char_literal, scan_string_literal,
};
use crate::token::{Token, TokenKind};
use crate::words::lookup_keyword;

pub struct Lexer<'a> {
    file_id: FileId,
    cursor: Cursor<'a>,
    diagnostics: Vec<Diagnostic>,
    interp_modes: Vec<StringMode>,
    interp_depths: Vec<usize>,
    pending_interp_start: Option<Token<'a>>,
    pending_string_part: Option<Token<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(file_id: FileId, source: &'a str) -> Self {
        Self {
            file_id,
            cursor: Cursor::new(source),
            diagnostics: Vec::new(),
            interp_modes: Vec::new(),
            interp_depths: Vec::new(),
            pending_interp_start: None,
            pending_string_part: None,
        }
    }

    pub fn tokenize(mut self) -> (Vec<Token<'a>>, Vec<Diagnostic>) {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let is_eof = tok.is_eof();
            tokens.push(tok);
            if is_eof { break; }
        }
        (tokens, self.diagnostics)
    }

    pub fn next_token(&mut self) -> Token<'a> {
        if let Some(tok) = self.pending_string_part.take() { return tok; }
        if let Some(tok) = self.pending_interp_start.take() { return tok; }
        self.skip_whitespace_and_comments();

        if self.cursor.is_eof() {
            let pos = self.cursor.pos();
            let span = make_span(self.file_id, pos, pos);
            return Token::new(TokenKind::Eof, span);
        }

        let start = self.cursor.pos();
        let b = self.cursor.peek().unwrap();

        if b == b'"' { return self.scan_string(); }
        if b == b'\'' {
            return scan_char_literal(&mut self.cursor, 
                self.file_id, &mut self.diagnostics);
        }
        if b.is_ascii_digit() {
            return scan_number(&mut self.cursor, 
                self.file_id, 
                &mut self.diagnostics);
        }
        if is_ident_start_byte(b) { return self.scan_ident(); }

        self.scan_operator_or_delimiter(start)
    }

    fn scan_string(&mut self) -> Token<'a> {
        let is_multi = self.cursor.peek_at(1) == Some(b'"') && self.cursor.peek_at(2) == Some(b'"');
        let mode = if is_multi { StringMode::Multiline } else { StringMode::Regular };

        match scan_string_literal(&mut self.cursor, self.file_id, mode, &mut self.diagnostics) {
            StringScanResult::Complete(tok) => tok,
            StringScanResult::Interpolation { part, interp_start } => {
                self.interp_modes.push(mode);
                self.interp_depths.push(1);
                self.pending_interp_start = Some(interp_start);
                part
            }
        }
    }

    fn scan_ident(&mut self) -> Token<'a> {
        let start = self.cursor.pos();
        let mut has_confusable = false;

        while let Some(ch) = self.cursor.peek_char() {
            if ch.is_alphanumeric() || ch == '_' {
                if is_confusable_or_bidi(ch) { has_confusable = true; }
                self.cursor.bump_char(); } else { break;
            }
        }

        if self.cursor.pos() == start {
            // consume the character: an empty token here would loop the tokenizer forever
            let ch = self.cursor.peek_char().expect("cursor is not at eof");
            self.cursor.bump_char();
            return self.handle_unrecognized_char(ch, start);
        }

        let span = make_span(self.file_id, start, self.cursor.pos());

        if has_confusable {
            self.diagnostics.push(Diagnostic::warning(
                WarningCode::W0102,
                "identifier contains confusable homoglyphs or bidirectional control characters",
                span,
            ));
        }

        let text = self.cursor.slice(start, self.cursor.pos());
        let kind = lookup_keyword(text).unwrap_or(TokenKind::Ident(text));
        Token::new(kind, span)
    }

    fn scan_operator_or_delimiter(&mut self, start: usize) -> Token<'a> {
        let b = self.cursor.bump().unwrap();
        match b {
            b'{' => self.handle_lbrace(start),
            b'}' => self.handle_rbrace(start),
            b'(' => self.make_token(TokenKind::LParen, start),
            b')' => self.make_token(TokenKind::RParen, start),
            b'[' => self.make_token(TokenKind::LBracket, start),
            b']' => self.make_token(TokenKind::RBracket, start),
            b';' => self.make_token(TokenKind::Semicolon, start),
            b',' => self.make_token(TokenKind::Comma, start),
            b'~' => self.make_token(TokenKind::Tilde, start),
            b'!' => self.scan_bang(start),
            b'#' => self.scan_hash(start),
            b'/' => self.scan_slash(start),
            b'|' => self.scan_pipe(start),
            b'?' => self.scan_question(start),
            b'-' => self.scan_minus(start),
            b'=' => self.scan_equal(start),
            b':' => self.scan_colon(start),
            b'.' => self.scan_dot(start),
            b'*' => self.scan_star(start),
            b'+' => self.scan_plus(start),
            b'%' => self.scan_percent(start),
            b'&' => self.scan_amp(start),
            b'^' => self.scan_caret(start),
            b'<' => self.scan_lt(start),
            b'>' => self.scan_gt(start),
            _ => self.handle_unrecognized_char(char::from(b), start),
        }
    }

    fn handle_lbrace(&mut self, start: usize) -> Token<'a> {
        if let Some(depth) = self.interp_depths.last_mut() { *depth += 1; }
        self.make_token(TokenKind::LBrace, start)
    }

    fn handle_rbrace(&mut self, start: usize) -> Token<'a> {
        if let Some(depth) = self.interp_depths.last_mut() {
            if *depth > 1 {
                *depth -= 1;
                return self.make_token(TokenKind::RBrace, start);
            }
            // depth 1
            self.interp_depths.pop();
            let mode = self.interp_modes.pop().unwrap();
            let interp_end = self.make_token(TokenKind::InterpolationEnd, start);

            match resume_string_scan(&mut self.cursor, self.file_id, mode, &mut self.diagnostics) {
                StringScanResult::Complete(tok) => {
                    let part = Token::new(
                        TokenKind::StringPart(extract_string_text(tok.kind)),
                        tok.span,
                    );
                    self.pending_string_part = Some(part);
                }
                StringScanResult::Interpolation { part, interp_start } => {
                    self.interp_modes.push(mode);
                    self.interp_depths.push(1);
                    self.pending_string_part = Some(part);
                    self.pending_interp_start = Some(interp_start);
                }
            }
            return interp_end;
        }
        self.make_token(TokenKind::RBrace, start)
    }

    fn scan_bang(&mut self, start: usize) -> Token<'a> {
        if self.cursor.peek() == Some(b'#') && self.cursor.peek_at(1) == Some(b'[') {
            self.cursor.bump();
            self.cursor.bump();
            self.make_token(TokenKind::ModuleAttrStart, start)
        } else if self.cursor.eat_if(b'=') { self.make_token(TokenKind::BangEq, start)
        } else { self.make_token(TokenKind::Bang, start) }
    }

    fn scan_hash(&mut self, start: usize) -> Token<'a> {
        if self.cursor.peek() == Some(b'!') && self.cursor.peek_at(1) == Some(b'[') {
            self.cursor.bump();
            self.cursor.bump();
            self.make_token(TokenKind::ModuleAttrStart, start)
        } else if self.cursor.eat_if(b'[') {
            self.make_token(TokenKind::AttrStart, start)
        } else {
            self.make_token(TokenKind::Hash, start)
        }
    }

    fn scan_slash(&mut self, start: usize) -> Token<'a> {
        if self.cursor.eat_if(b'=') { self.make_token(TokenKind::SlashEq, start) } 
        else { self.make_token(TokenKind::Slash, start) }
    }

    fn scan_pipe(&mut self, start: usize) -> Token<'a> {
        if self.cursor.eat_if(b'>') {
            self.make_token(TokenKind::PipeOp, start)
        } else if self.cursor.eat_if(b'|') {
            self.make_token(TokenKind::PipePipe, start)
        } else if self.cursor.eat_if(b'=') {
            self.make_token(TokenKind::PipeEq, start)
        } else {
            self.make_token(TokenKind::Pipe, start)
        }
    }

    fn scan_question(&mut self, start: usize) -> Token<'a> {
        if self.cursor.eat_if(b'.') { self.make_token(TokenKind::QuestionDot, start)
        } else if self.cursor.eat_if(b'?') { self.make_token(TokenKind::QuestionQuestion, start)
        } else { self.make_token(TokenKind::Question, start) }
    }

    fn scan_minus(&mut self, start: usize) -> Token<'a> {
        if self.cursor.eat_if(b'>') {
            self.make_token(TokenKind::Arrow, start)
        } else if self.cursor.eat_if(b'=') {
            self.make_token(TokenKind::MinusEq, start)
        } else {
            self.make_token(TokenKind::Minus, start)
        }
    }

    fn scan_equal(&mut self, start: usize) -> Token<'a> {
        if self.cursor.eat_if(b'>') {
            self.make_token(TokenKind::FatArrow, start)
        } else if self.cursor.eat_if(b'=') {
            self.make_token(TokenKind::EqEq, start)
        } else {
            self.make_token(TokenKind::Eq, start)
        }
    }

    fn scan_colon(&mut self, start: usize) -> Token<'a> {
        if self.cursor.eat_if(b':') {
            self.make_token(TokenKind::DoubleColon, start)
        } else {
            self.make_token(TokenKind::Colon, start)
        }
    }

    fn scan_dot(&mut self, start: usize) -> Token<'a> {
        if self.cursor.peek() == Some(b'.') {
            if self.cursor.peek_at(1) == Some(b'.') {
                self.cursor.bump();
                self.cursor.bump();
                self.make_token(TokenKind::Ellipsis, start)
            } else if self.cursor.peek_at(1) == Some(b'=') {
                self.cursor.bump();
                self.cursor.bump();
                self.make_token(TokenKind::DotDotEq, start)
            } else {
                self.cursor.bump();
                self.make_token(TokenKind::DotDot, start)
            }
        } else {
            self.make_token(TokenKind::Dot, start)
        }
    }

    fn scan_star(&mut self, start: usize) -> Token<'a> {
        if self.cursor.peek() == Some(b'*') {
            if self.cursor.peek_at(1) == Some(b'=') {
                self.cursor.bump();
                self.cursor.bump();
                self.make_token(TokenKind::StarStarEq, start)
            } else {
                self.cursor.bump();
                self.make_token(TokenKind::StarStar, start)
            }
        } else if self.cursor.eat_if(b'=') {
            self.make_token(TokenKind::StarEq, start)
        } else {
            self.make_token(TokenKind::Star, start)
        }
    }

    fn scan_plus(&mut self, start: usize) -> Token<'a> {
        if self.cursor.eat_if(b'=') {
            self.make_token(TokenKind::PlusEq, start)
        } else {
            self.make_token(TokenKind::Plus, start)
        }
    }

    fn scan_percent(&mut self, start: usize) -> Token<'a> {
        if self.cursor.eat_if(b'=') {
            self.make_token(TokenKind::PercentEq, start)
        } else {
            self.make_token(TokenKind::Percent, start)
        }
    }

    fn scan_amp(&mut self, start: usize) -> Token<'a> {
        if self.cursor.eat_if(b'&') {
            self.make_token(TokenKind::AmpAmp, start)
        } else if self.cursor.eat_if(b'=') {
            self.make_token(TokenKind::AmpEq, start)
        } else {
            self.make_token(TokenKind::Amp, start)
        }
    }

    fn scan_caret(&mut self, start: usize) -> Token<'a> {
        if self.cursor.eat_if(b'=') {
            self.make_token(TokenKind::CaretEq, start)
        } else {
            self.make_token(TokenKind::Caret, start)
        }
    }

    fn scan_lt(&mut self, start: usize) -> Token<'a> {
        if self.cursor.peek() == Some(b'<') {
            if self.cursor.peek_at(1) == Some(b'=') {
                self.cursor.bump();
                self.cursor.bump();
                self.make_token(TokenKind::LtLtEq, start)
            } else {
                self.cursor.bump();
                self.make_token(TokenKind::LtLt, start)
            }
        } else if self.cursor.eat_if(b'=') {
            self.make_token(TokenKind::LtEq, start)
        } else {
            self.make_token(TokenKind::Lt, start)
        }
    }

    fn scan_gt(&mut self, start: usize) -> Token<'a> {
        if self.cursor.peek() == Some(b'>') {
            if self.cursor.peek_at(1) == Some(b'=') {
                self.cursor.bump();
                self.cursor.bump();
                self.make_token(TokenKind::GtGtEq, start)
            } else {
                self.cursor.bump();
                self.make_token(TokenKind::GtGt, start)
            }
        } else if self.cursor.eat_if(b'=') {
            self.make_token(TokenKind::GtEq, start)
        } else {
            self.make_token(TokenKind::Gt, start)
        }
    }

    fn handle_unrecognized_char(&mut self, ch: char, start: usize) -> Token<'a> {
        let span = make_span(self.file_id, start, self.cursor.pos());
        self.diagnostics.push(Diagnostic::error(
            ErrorCode::E0112,
            format!("unrecognized character '{ch}'"),
            span,
        ));
        self.next_token()
    }

    fn make_token(&self, kind: TokenKind<'a>, start: usize) -> Token<'a> {
        let span = make_span(self.file_id, start, self.cursor.pos());
        Token::new(kind, span)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            if self.cursor.is_eof() { break; }
            if self.cursor.peek_char() == Some('\u{feff}') {
                // skip UTF-8 BOM, common in files edited on Windows
                self.cursor.bump_char();
                continue;
            }
            if self.cursor.peek_char().is_some_and(is_non_ascii_whitespace) {
                let start = self.cursor.pos();
                self.cursor.bump_char();
                let span = make_span(self.file_id, start, self.cursor.pos());
                self.diagnostics.push(Diagnostic::warning(
                    WarningCode::W0101,
                    "non-ASCII whitespace character detected",
                    span,
                ));
                continue;
            }
            if let Some(b) = self.cursor.peek() {
                if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' {
                    self.cursor.bump();
                    continue;
                }
                if b == b'/' {
                    if self.cursor.peek_at(1) == Some(b'/') {
                        skip_line_comment(&mut self.cursor);
                        continue;
                    }
                    if self.cursor.peek_at(1) == Some(b'*') {
                        self.skip_block_comment();
                        continue;
                    }
                }
            }
            break;
        }
    }

    fn skip_block_comment(&mut self) {
        let start = self.cursor.pos();
        self.cursor.bump();
        self.cursor.bump();
        let mut depth = 1usize;

        while !self.cursor.is_eof() {
            if self.cursor.peek() == Some(b'/') && self.cursor.peek_at(1) == Some(b'*') {
                self.cursor.bump();
                self.cursor.bump();
                depth += 1;
            } else if self.cursor.peek() == Some(b'*') && self.cursor.peek_at(1) == Some(b'/') {
                self.cursor.bump();
                self.cursor.bump();
                depth -= 1;
                if depth == 0 { return; }
            } else { self.cursor.bump_char(); }
        }

        let span = make_span(self.file_id, start, self.cursor.pos());
        self.diagnostics.push(Diagnostic::error(
            ErrorCode::E0104,
            "unterminated block comment at end of file",
            span,
        ));
    }
}

fn skip_line_comment(cursor: &mut Cursor<'_>) {
    while let Some(b) = cursor.peek() {
        if b == b'\n' { break; }
        cursor.bump();
    }
}

fn is_ident_start_byte(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b >= 0x80
}

fn is_non_ascii_whitespace(ch: char) -> bool {
    ch.is_whitespace() && !matches!(ch, ' ' | '\t' | '\r' | '\n')
}

fn is_confusable_or_bidi(ch: char) -> bool {
    matches!(ch, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

fn extract_string_text(kind: TokenKind<'_>) -> String {
    match kind {
        TokenKind::StringLiteral(s) | TokenKind::StringPart(s) => s,
        _ => String::new(),
    }
}
