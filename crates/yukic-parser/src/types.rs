#![deny(unsafe_code)]

use crate::stream::TokenStream;
use yukic_ast::{GenericParam, Ident, TypeNode, TypeNodeKind};
use yukic_diagnostics::ErrorCode;
use yukic_lexer::TokenKind;
use yukic_span::Span;

impl TokenStream<'_> {
    pub fn parse_type(&mut self) -> TypeNode {
        let start_span = self.curr_span();
        let mut base_ty = self.parse_primary_type();

        while self.check_kind(|k| matches!(k, TokenKind::Pipe)) && !self.is_c_attribute_lookahead() {
            self.bump();
            let next_ty = self.parse_primary_type();
            let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
            base_ty = match base_ty.kind {
                TypeNodeKind::Union(mut variants) => {
                    variants.push(next_ty);
                    TypeNode::new(TypeNodeKind::Union(variants), span)
                }
                _ => TypeNode::new(TypeNodeKind::Union(vec![base_ty, next_ty]), span),
            };
        }

        base_ty
    }

    fn is_c_attribute_lookahead(&self) -> bool {
        if !matches!(self.peek().kind, TokenKind::Pipe) {
            return false;
        }

        let mut i = 1;
        while let Some(tok) = self.tokens.get(self.cursor + i) {
            match &tok.kind {
                TokenKind::Pipe => {
                    let after = self.tokens.get(self.cursor + i + 1);
                    return match after.map(|t| &t.kind) {
                        Some(
                            TokenKind::Cfun
                            | TokenKind::Cstruct
                            | TokenKind::Fun
                            | TokenKind::Struct
                            | TokenKind::Class
                            | TokenKind::Public
                            | TokenKind::Locked
                            | TokenKind::AttrStart,
                        ) => true,
                        _ => false,
                    };
                }
                TokenKind::Ident(_)
                | TokenKind::Eq
                | TokenKind::StringLiteral(_)
                | TokenKind::IntLiteral { .. }
                | TokenKind::Comma => {
                    i += 1;
                }
                _ => return false,
            }
        }
        false
    }

    fn parse_primary_type(&mut self) -> TypeNode {
        let start_span = self.curr_span();
        let mut ty = self.parse_atomic_type();

        loop {
            if self.eat_kind(|k| matches!(k, TokenKind::Question)) {
                let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
                ty = TypeNode::new(TypeNodeKind::Optional(Box::new(ty)), span);
            } else if self.eat_kind(|k| matches!(k, TokenKind::LBracket)) {
                let size = self.parse_array_size();
                self.expect_token(&TokenKind::RBracket, "']' to close array type");
                let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
                ty = TypeNode::new(TypeNodeKind::Array(Box::new(ty), size), span);
            } 

            else { break; }
        }

        ty
    }

    fn parse_array_size(&mut self) -> Option<usize> {
        if let TokenKind::IntLiteral { value, .. } = self.peek().kind {
            let val = usize::try_from(value).ok();
            self.bump();
            val
        } 

        else { None }
    }

    fn parse_atomic_type(&mut self) -> TypeNode {
        let span = self.curr_span();
        if self.eat_kind(|k| matches!(k, TokenKind::Ref)) {
            let _is_mut = self.eat_kind(|k| matches!(k, TokenKind::Tilde));
            let inner = self.parse_type();
            let end_span = self.prev_span();
            return TypeNode::new(
                TypeNodeKind::Ref(Box::new(inner)),
                Span::new(span.file_id, span.start, end_span.end),
            );
        }

        if self.eat_kind(|k| matches!(k, TokenKind::Fun)) {
            return self.parse_fun_type(span, false);
        }

        if self.eat_kind(|k| matches!(k, TokenKind::Cfun)) {
            return self.parse_fun_type(span, true);
        }

        if self.eat_kind(|k| matches!(k, TokenKind::LParen)) {
            return self.parse_tuple_type(span);
        }

        self.parse_named_or_generic_type()
    }

    fn parse_fun_type(&mut self, start_span: Span, is_cfun: bool) -> TypeNode {
        self.expect_token(&TokenKind::LParen, "'(' for function type parameters");
        let mut params = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RParen)) {
            params.push(self.parse_type());
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }
        self.expect_token(&TokenKind::RParen, "')' after function type parameters");

        let ret_type = if self.eat_kind(|k| matches!(k, TokenKind::Arrow)) {
            self.parse_type()
        } else {
            TypeNode::new(
                TypeNodeKind::Ident(Ident::new("void", self.prev_span())),
                self.prev_span(),
            )
        };

        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        let kind = if is_cfun {
            TypeNodeKind::CFun(params, Box::new(ret_type))
        } else {
            TypeNodeKind::Fun(params, Box::new(ret_type))
        };
        TypeNode::new(kind, span)
    }

    fn parse_tuple_type(&mut self, start_span: Span) -> TypeNode {
        let mut types = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RParen)) {
            types.push(self.parse_type());
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }
        self.expect_token(&TokenKind::RParen, "')' to close tuple type");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        TypeNode::new(TypeNodeKind::Tuple(types), span)
    }

    fn parse_named_or_generic_type(&mut self) -> TypeNode {
        let start_span = self.curr_span();
        let name_tok = self.bump();

        let ident_name = match name_tok.kind {
            TokenKind::Ident(name) => name.to_string(),
            TokenKind::NullLiteral => "null".to_string(),
            _ => {
                self.error(
                    ErrorCode::E0201,
                    format!("expected type name, found {:?}", name_tok.kind),
                    start_span,
                );
                return TypeNode::new(
                    TypeNodeKind::Ident(Ident::new("<error>", start_span)),
                    start_span,
                );
            }
        };

        let ident = Ident::new(ident_name, start_span);
        if self.eat_kind(|k| matches!(k, TokenKind::DoubleColon)) {
            return self.parse_path_type(ident, start_span);
        }

        if self.check_kind(|k| matches!(k, TokenKind::Lt)) {
            return self.parse_generic_type(ident, start_span);
        }

        TypeNode::new(TypeNodeKind::Ident(ident), start_span)
    }

    fn parse_path_type(&mut self, first: Ident, start_span: Span) -> TypeNode {
        let mut segments = vec![first];
        loop {
            let next_tok = self.bump();
            if let TokenKind::Ident(seg) = next_tok.kind {
                segments.push(Ident::new(seg, next_tok.span));
            } else {
                self.error(
                    ErrorCode::E0201,
                    "expected identifier in type path",
                    next_tok.span,
                );
                break;
            }
            if !self.eat_kind(|k| matches!(k, TokenKind::DoubleColon)) {
                break;
            }
        }
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        TypeNode::new(TypeNodeKind::Path(segments), span)
    }

    pub(crate) fn parse_generic_type(&mut self, ident: Ident, start_span: Span) -> TypeNode {
        self.bump(); // consume '<'
        let mut args = Vec::new();
        while !self.is_eof() && !self.check_gt() {
            args.push(self.parse_type());
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }
        self.expect_gt("'>' to close generic type arguments");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);

        match ident.name.as_str() {
            "ptr" if args.len() == 1 => {
                TypeNode::new(TypeNodeKind::Pointer(Box::new(args.remove(0))), span)
            }
            "map" if args.len() == 2 => {
                let v = args.remove(1);
                let k = args.remove(0);
                TypeNode::new(TypeNodeKind::Map(Box::new(k), Box::new(v)), span)
            }
            "result" if args.len() == 2 => {
                let e = args.remove(1);
                let t = args.remove(0);
                TypeNode::new(TypeNodeKind::Result(Box::new(t), Box::new(e)), span)
            }
            _ => TypeNode::new(TypeNodeKind::Generic(ident, args), span),
        }
    }

    pub fn parse_generic_params(&mut self) -> Vec<GenericParam> {
        if !self.eat_kind(|k| matches!(k, TokenKind::Lt)) {
            return Vec::new();
        }

        let mut params = Vec::new();
        while !self.is_eof() && !self.check_gt() {
            let span = self.curr_span();
            let tok = self.bump();
            let name = if let TokenKind::Ident(id) = tok.kind {
                Ident::new(id, span)
            } else {
                self.error(ErrorCode::E0201, "expected generic parameter name", span);
                Ident::new("<error>", span)
            };

            let mut bounds = Vec::new();
            if self.eat_kind(|k| matches!(k, TokenKind::Colon)) {
                loop {
                    bounds.push(self.parse_type());
                    if !self.eat_kind(|k| matches!(k, TokenKind::Plus)) {
                        break;
                    }
                }
            }

            let param_span = Span::new(span.file_id, span.start, self.prev_span().end);
            params.push(GenericParam {
                name,
                bounds,
                span: param_span,
            });

            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }

        self.expect_gt("'>' to close generic parameters");
        params
    }

    pub fn expect_token(&mut self, expected: &TokenKind<'_>, msg: &str) -> Option<Span> {
        let span = self.curr_span();
        if self.eat_kind(|k| std::mem::discriminant(k) == std::mem::discriminant(expected)) {
            Some(span)
        } else {
            let code = match expected {
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => ErrorCode::E0202,
                _ => ErrorCode::E0201,
            };
            self.error(
                code,
                format!("expected {msg}, found {:?}", self.peek().kind),
                span,
            );
            None
        }
    }
}
