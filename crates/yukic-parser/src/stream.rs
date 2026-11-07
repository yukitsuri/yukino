#![deny(unsafe_code)]

use yukic_diagnostics::{Diagnostic, ErrorCode, WarningCode};
use yukic_lexer::{Token, TokenKind};
use yukic_span::{BytePos, FileId, Span};

pub struct TokenStream<'a> {
    pub(crate) tokens: Vec<Token<'a>>,
    pub(crate) cursor: usize,
    pub(crate) file_id: FileId,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) eof_token: Token<'a>,
}

impl<'a> TokenStream<'a> {
    pub fn new(tokens: Vec<Token<'a>>, file_id: FileId) -> Self {
        let eof_span = tokens
            .last()
            .map_or(Span::new(file_id, BytePos(0), BytePos(0)), |t| t.span);
        Self {
            tokens,
            cursor: 0,
            file_id,
            diagnostics: Vec::new(),
            eof_token: Token::new(TokenKind::Eof, eof_span),
        }
    }

    pub fn peek(&self) -> &Token<'a> { self.peek_at(0) }
    pub fn peek_at(&self, k: usize) -> &Token<'a> {
        let idx = self.cursor + k;
        if idx < self.tokens.len() { &self.tokens[idx]} 
        else { &self.eof_token }
    }

    pub fn bump(&mut self) -> Token<'a> {
        if self.cursor < self.tokens.len() {
            let tok = self.tokens[self.cursor].clone();
            self.cursor += 1;
            tok
        } 

        else { self.eof_token.clone() }
    }

    pub fn is_eof(&self) -> bool {
        self.cursor >= self.tokens.len() || self.peek().is_eof()
    }

    pub fn curr_span(&self) -> Span { self.peek().span }

    pub fn prev_span(&self) -> Span {
        if self.cursor > 0 && self.cursor <= self.tokens.len() {
            self.tokens[self.cursor - 1].span
        } else { self.curr_span() }
    }

    pub fn check_kind(&self, check: impl Fn(&TokenKind<'a>) -> bool) -> bool {
        check(&self.peek().kind)
    }

    pub fn check_kind_at(&self, k: usize, check: impl Fn(&TokenKind<'a>) -> bool) -> bool {
        check(&self.peek_at(k).kind)
    }

    pub fn eat_kind(&mut self, check: impl Fn(&TokenKind<'a>) -> bool) -> bool {
        if self.check_kind(check) {
            self.bump();
            true
        } 

        else { false }
    }

    pub fn emit(&mut self, diag: Diagnostic) { self.diagnostics.push(diag); }
    pub fn error(&mut self, code: ErrorCode, msg: impl Into<String>, span: Span) {
        self.emit(Diagnostic::error(code, msg, span));
    }

    pub fn check_gt(&self) -> bool {
        if self.cursor >= self.tokens.len() { return false; }
        matches!(
            self.tokens[self.cursor].kind,
            TokenKind::Gt | TokenKind::GtGt
        )
    }

    pub fn eat_gt(&mut self) -> bool {
        if self.cursor >= self.tokens.len() {
            return false;
        }
        match self.tokens[self.cursor].kind {
            TokenKind::Gt => {
                self.cursor += 1;
                true
            }
            TokenKind::GtGt => {
                let span = self.tokens[self.cursor].span;
                let mid = yukic_span::BytePos(u32::midpoint(span.start.0, span.end.0));
                self.tokens[self.cursor] =
                    Token::new(TokenKind::Gt, Span::new(span.file_id, mid, span.end));
                true
            }
            _ => false,
        }
    }

    pub fn expect_gt(&mut self, msg: &str) -> Option<Span> {
        let span = self.curr_span();
        if self.eat_gt() {
            Some(span)
        } else {
            self.error(
                ErrorCode::E0201,
                format!("expected {msg}, found {:?}", self.peek().kind),
                span,
            );
            None
        }
    }

    pub fn warning(&mut self, code: WarningCode, msg: impl Into<String>, span: Span) {
        self.emit(Diagnostic::warning(code, msg, span));
    }
}
