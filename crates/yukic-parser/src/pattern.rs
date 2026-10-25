#![deny(unsafe_code)]

use crate::stream::TokenStream;
use yukic_ast::{Expr, ExprKind, Ident, LiteralValue, Pattern, PatternKind};
use yukic_diagnostics::ErrorCode;
use yukic_lexer::TokenKind;
use yukic_span::Span;

impl TokenStream<'_> {
    pub fn parse_pattern(&mut self) -> Pattern {
        let start_span = self.curr_span();
        let mut pat = self.parse_single_pattern();

        if self.check_kind(|k| matches!(k, TokenKind::DotDot | TokenKind::DotDotEq)) {
            let inclusive = self.eat_kind(|k| matches!(k, TokenKind::DotDotEq));

            if !inclusive { self.bump(); }
            
            let end_expr = self.parse_expr();
            let start_expr = match pat.kind {
                PatternKind::Literal(lit) => Expr::new(ExprKind::Literal(lit), pat.span),
                PatternKind::Ident(id) => Expr::new(ExprKind::Ident(id), pat.span),
                _ => {
                    self.error(ErrorCode::E0201, "invalid start of range pattern", pat.span);
                    Expr::new(ExprKind::Error, pat.span)
                }
            };
            let span = Span::new(start_span.file_id, start_span.start, end_expr.span.end);
            pat = Pattern::new(
                PatternKind::Range(Box::new(start_expr), Box::new(end_expr), inclusive),
                span,
            );
        }

        if self.eat_kind(|k| matches!(k, TokenKind::Pipe)) {
            let mut or_pats = vec![pat];
            loop {
                or_pats.push(self.parse_single_pattern());
                if !self.eat_kind(|k| matches!(k, TokenKind::Pipe)) {
                    break;
                }
            }
            let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
            pat = Pattern::new(PatternKind::Or(or_pats), span);
        }

        pat
    }

    fn parse_single_pattern(&mut self) -> Pattern {
        let span = self.curr_span();
        let tok = self.peek().clone();

        match tok.kind {
            TokenKind::Ident("_") => {
                self.error(
                    ErrorCode::E0201,
                    "wildcard pattern '_' is not supported; use 'else' instead",
                    span,
                );
                self.bump();
                Pattern::new(PatternKind::Wildcard, span)
            }
            TokenKind::Else => {
                self.bump();
                Pattern::new(PatternKind::Wildcard, span)
            }
            TokenKind::IntLiteral { value, .. } => {
                self.bump();
                Pattern::new(
                    PatternKind::Literal(LiteralValue::Int(value.cast_signed())),
                    span,
                )
            }
            TokenKind::FloatLiteral { value, .. } => {
                self.bump();
                Pattern::new(PatternKind::Literal(LiteralValue::Float(value)), span)
            }
            TokenKind::StringLiteral(ref s) => {
                let s_clone = s.clone();
                self.bump();
                Pattern::new(PatternKind::Literal(LiteralValue::String(s_clone)), span)
            }
            TokenKind::CharLiteral(c) => {
                self.bump();
                Pattern::new(PatternKind::Literal(LiteralValue::Char(c)), span)
            }
            TokenKind::BoolLiteral(b) => {
                self.bump();
                Pattern::new(PatternKind::Literal(LiteralValue::Bool(b)), span)
            }
            TokenKind::NullLiteral => {
                self.bump();
                Pattern::new(PatternKind::Literal(LiteralValue::Null), span)
            }
            TokenKind::LParen => self.parse_tuple_pattern(span),
            TokenKind::Ident(name) => {
                let ident = Ident::new(name, span);
                self.bump();
                self.parse_ident_or_variant_pattern(ident, span)
            }
            _ => {
                self.error(ErrorCode::E0201, "expected pattern", span);
                self.bump();
                Pattern::new(PatternKind::Wildcard, span)
            }
        }
    }

    fn parse_tuple_pattern(&mut self, start_span: Span) -> Pattern {
        self.bump(); // consume '('
        let mut patterns = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RParen)) {
            patterns.push(self.parse_pattern());
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }
        self.expect_token(&TokenKind::RParen, "')' to close tuple pattern");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Pattern::new(PatternKind::Tuple(patterns), span)
    }

    fn parse_ident_or_variant_pattern(&mut self, ident: Ident, start_span: Span) -> Pattern {
        if self.eat_kind(|k| matches!(k, TokenKind::DoubleColon)) {
            let var_span = self.curr_span();
            let var_tok = self.bump();
            let var_name = if let TokenKind::Ident(name) = var_tok.kind {
                Ident::new(name, var_span)
            } else {
                self.error(ErrorCode::E0201, "expected variant name", var_span);
                Ident::new("<error>", var_span)
            };
            return self.parse_variant_pattern_payload(Some(ident), var_name, start_span);
        }

        if self.check_kind(|k| matches!(k, TokenKind::LParen)) {
            return self.parse_variant_pattern_payload(None, ident, start_span);
        }

        if self.check_kind(|k| matches!(k, TokenKind::LBrace)) {
            return self.parse_struct_pattern(ident, start_span);
        }

        Pattern::new(PatternKind::Ident(ident), start_span)
    }

    fn parse_variant_pattern_payload(
        &mut self,
        enum_name: Option<Ident>,
        variant: Ident,
        start_span: Span,
    ) -> Pattern {
        let mut payload = Vec::new();
        if self.eat_kind(|k| matches!(k, TokenKind::LParen)) {
            while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RParen)) {
                payload.push(self.parse_pattern());
                if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                    break;
                }
            }
            self.expect_token(&TokenKind::RParen, "')' to close variant pattern payload");
        }
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Pattern::new(PatternKind::Variant(enum_name, variant, payload), span)
    }

    fn parse_struct_pattern(&mut self, name: Ident, start_span: Span) -> Pattern {
        self.bump(); // consume '{'
        let mut fields = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RBrace)) {
            let f_span = self.curr_span();
            let f_tok = self.bump();
            let field_ident = if let TokenKind::Ident(f) = f_tok.kind {
                Ident::new(f, f_span)
            } else {
                self.error(
                    ErrorCode::E0201,
                    "expected field name in struct pattern",
                    f_span,
                );
                break;
            };

            let pat = if self.eat_kind(|k| matches!(k, TokenKind::Colon)) {
                self.parse_pattern()
            } else {
                Pattern::new(PatternKind::Ident(field_ident.clone()), f_span)
            };

            fields.push((field_ident, pat));
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }
        self.expect_token(&TokenKind::RBrace, "'}' to close struct pattern");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Pattern::new(PatternKind::Struct(name, fields), span)
    }
}
