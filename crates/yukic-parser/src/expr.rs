#![deny(unsafe_code)]

use crate::prec::{PREFIX_BP, infix_binding_power, token_to_binary_op};
use crate::stream::TokenStream;
use yukic_ast::{
    BinaryOp, CatchClause, Expr, ExprKind, FieldInit, Ident, LiteralValue, MacroArg, MacroCallNode,
    MacroDelimiter, StringPart, SwitchArm, UnaryOp,
};
use yukic_diagnostics::{ErrorCode, WarningCode};
use yukic_lexer::{IntSuffix, TokenKind};
use yukic_span::Span;

impl TokenStream<'_> {
    pub fn parse_expr(&mut self) -> Expr { self.parse_expr_with_bp(0, true) }
    pub fn parse_expr_no_struct(&mut self) -> Expr { self.parse_expr_with_bp(0, false) }

    pub fn parse_expr_with_bp(&mut self, min_bp: u8, allow_struct: bool) -> Expr {
        let mut lhs = self.parse_prefix(allow_struct);

        loop {
            lhs = self.parse_postfix(lhs, allow_struct);

            let peek_tok = self.peek();
            if (matches!(peek_tok.kind, TokenKind::DotDot)
                || matches!(peek_tok.kind, TokenKind::DotDotEq))
                && matches!(self.peek_at(1).kind, TokenKind::RBracket)
            {
                break;
            }

            let Some((left_bp, right_bp)) = infix_binding_power(&peek_tok.kind) else { break; };

            if left_bp < min_bp { break; }

            lhs = self.parse_infix(lhs, right_bp, allow_struct);
        }

        lhs
    }

    fn parse_prefix(&mut self, allow_struct: bool) -> Expr {
        let span = self.curr_span();
        let tok = self.peek().clone();

        match tok.kind {
            TokenKind::IntLiteral { value, suffix, .. } => {
                self.bump();
                let lit = match suffix {
                    Some(IntSuffix::U8 | IntSuffix::U16 | IntSuffix::U32 | IntSuffix::U64) => {
                        LiteralValue::UInt(value)
                    }
                    _ => LiteralValue::Int(value.cast_signed()),
                };
                Expr::new(ExprKind::Literal(lit), span)
            }
            TokenKind::FloatLiteral { value, .. } => {
                self.bump();
                Expr::new(ExprKind::Literal(LiteralValue::Float(value)), span)
            }
            TokenKind::StringLiteral(ref s) => {
                let text = s.clone();
                self.bump();
                Expr::new(ExprKind::Literal(LiteralValue::String(text)), span)
            }
            TokenKind::CharLiteral(c) => {
                self.bump();
                Expr::new(ExprKind::Literal(LiteralValue::Char(c)), span)
            }
            TokenKind::BoolLiteral(b) => {
                self.bump();
                Expr::new(ExprKind::Literal(LiteralValue::Bool(b)), span)
            }
            TokenKind::NullLiteral => {
                self.bump();
                Expr::new(ExprKind::Literal(LiteralValue::Null), span)
            }
            TokenKind::StringPart(ref s) => {
                let text = s.clone();
                self.parse_interpolated_string(Some(text), span)
            }
            TokenKind::InterpolationStart => self.parse_interpolated_string(None, span),
            TokenKind::This => {
                self.bump();
                Expr::new(ExprKind::This, span)
            }
            TokenKind::Super => {
                self.bump();
                Expr::new(ExprKind::Super, span)
            }
            TokenKind::Init => {
                self.bump();
                self.parse_class_init(span)
            }
            TokenKind::Ident(name) => {
                let id = Ident::new(name, span);
                self.bump();
                self.parse_ident_prefix(id, span, allow_struct)
            }
            TokenKind::Get => {
                let id = Ident::new("get", span);
                self.bump();
                self.parse_ident_prefix(id, span, allow_struct)
            }
            TokenKind::Minus => {
                self.bump();
                let operand = self.parse_expr_with_bp(PREFIX_BP, allow_struct);
                let end_span = self.prev_span();
                Expr::new(
                    ExprKind::Unary(UnaryOp::Neg, Box::new(operand)),
                    Span::new(span.file_id, span.start, end_span.end),
                )
            }
            TokenKind::Bang => {
                self.bump();
                let operand = self.parse_expr_with_bp(PREFIX_BP, allow_struct);
                let end_span = self.prev_span();
                Expr::new(
                    ExprKind::Unary(UnaryOp::Not, Box::new(operand)),
                    Span::new(span.file_id, span.start, end_span.end),
                )
            }
            TokenKind::Tilde => {
                self.bump();
                let operand = self.parse_expr_with_bp(PREFIX_BP, allow_struct);
                let end_span = self.prev_span();
                Expr::new(
                    ExprKind::Unary(UnaryOp::BitNot, Box::new(operand)),
                    Span::new(span.file_id, span.start, end_span.end),
                )
            }
            TokenKind::Ref => {
                self.bump();
                let mutable = self.eat_kind(|k| matches!(k, TokenKind::Tilde));
                let operand = self.parse_expr_with_bp(PREFIX_BP, allow_struct);
                let end_span = self.prev_span();
                Expr::new(
                    ExprKind::Ref {
                        mutable,
                        expr: Box::new(operand),
                    },
                    Span::new(span.file_id, span.start, end_span.end),
                )
            }
            TokenKind::Panic => {
                self.bump();
                let ident = Ident::new("panic", span);
                self.parse_ident_prefix(ident, span, allow_struct)
            }
            TokenKind::LParen => self.parse_paren_or_tuple(span),
            TokenKind::LBracket => self.parse_array_literal(span),
            TokenKind::LBrace => self.parse_map_or_block_expr(span),
            TokenKind::If => self.parse_if_expr(span),
            TokenKind::Switch => self.parse_switch_expr(span),
            TokenKind::While => self.parse_while_expr(span),
            TokenKind::For => self.parse_for_expr(span),
            TokenKind::Loop => self.parse_loop_expr(span),
            TokenKind::Sus => self.parse_sus_expr(span),
            TokenKind::Pipe => self.parse_closure_expr(span),
            TokenKind::Fun => self.parse_anonymous_fun(span),
            TokenKind::Region => self.parse_region_expr(span),
            TokenKind::Ellipsis => self.parse_spread_expr(span, allow_struct),
            TokenKind::Colon => self.parse_colon_expr(span),
            _ => {
                self.error(
                    ErrorCode::E0201,
                    format!("expected expression, found {:?}", tok.kind),
                    span,
                );
                self.bump();
                Expr::new(ExprKind::Error, span)
            }
        }
    }

    fn parse_ident_prefix(&mut self, ident: Ident, start_span: Span, allow_struct: bool) -> Expr {
        if self.check_kind(|k| matches!(k, TokenKind::Colon)) {
            match self.peek_at(1).kind {
                TokenKind::For => {
                    self.bump(); // consume ':'
                    return self.parse_for_expr_with_label(Some(ident), start_span);
                }
                TokenKind::While => {
                    self.bump(); // consume ':'
                    return self.parse_while_expr_with_label(Some(ident), start_span);
                }
                TokenKind::Loop => {
                    self.bump(); // consume ':'
                    return self.parse_loop_expr_with_label(Some(ident), start_span);
                }
                _ => {}
            }
        }

        if self.eat_kind(|k| matches!(k, TokenKind::DoubleColon)) {
            return self.parse_path_or_macro(ident, start_span);
        }

        if self.eat_kind(|k| matches!(k, TokenKind::Bang)) {
            return self.parse_macro_call(vec![ident], start_span);
        }

        if self.eat_kind(|k| matches!(k, TokenKind::FatArrow)) {
            return self.parse_arrow_body(vec![ident], start_span);
        }

        if ident.name == "ptr_of" && self.eat_kind(|k| matches!(k, TokenKind::LParen)) {
            let target = self.parse_ident_node();
            self.expect_token(&TokenKind::RParen, "')' after ptr_of argument");
            let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
            return Expr::new(ExprKind::PtrOf(target), span);
        }

        if ident.name == "cast" && self.check_kind(|k| matches!(k, TokenKind::Lt)) {
            self.bump();
            let ty = self.parse_type();
            self.expect_gt("'>' after cast type");
            self.expect_token(&TokenKind::LParen, "'(' for cast expression argument");
            let target = self.parse_expr();
            self.expect_token(&TokenKind::RParen, "')' to close cast argument");
            let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
            return Expr::new(ExprKind::PtrCast(Box::new(target), ty), span);
        }

        if allow_struct
            && self.check_kind(|k| matches!(k, TokenKind::LBrace))
            && self.is_struct_init_lookahead()
        {
            return self.parse_struct_init(ident, start_span);
        }

        Expr::new(ExprKind::Ident(ident), start_span)
    }

    fn is_struct_init_lookahead(&self) -> bool {
        match self.peek_at(1).kind {
            TokenKind::RBrace => true,
            TokenKind::Ident(_) => {
                matches!(self.peek_at(2).kind, TokenKind::Colon | TokenKind::Comma)
            }
            _ => false,
        }
    }

    fn parse_path_or_macro(&mut self, first: Ident, start_span: Span) -> Expr {
        let mut segments = vec![first];
        loop {
            let seg_span = self.curr_span();
            let seg_tok = self.bump();
            if let TokenKind::Ident(name) = seg_tok.kind {
                segments.push(Ident::new(name, seg_span));
            } else {
                self.error(ErrorCode::E0201, "expected identifier in path", seg_span); break;
            }
            if !self.eat_kind(|k| matches!(k, TokenKind::DoubleColon)) { break; }
        }

        if self.eat_kind(|k| matches!(k, TokenKind::Bang)) {
            return self.parse_macro_call(segments, start_span);
        }

        if segments.len() == 2 && segments[0].name == "rt" {
            if segments[1].name == "ptr_of" && self.eat_kind(|k| matches!(k, TokenKind::LParen)) {
                let target = self.parse_ident_node();
                self.expect_token(&TokenKind::RParen, "')' after ptr_of argument");
                let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
                return Expr::new(ExprKind::PtrOf(target), span);
            }
            if segments[1].name == "cast" && self.check_kind(|k| matches!(k, TokenKind::Lt)) {
                self.bump();
                let ty = self.parse_type();
                self.expect_gt("'>' after cast type");
                self.expect_token(&TokenKind::LParen, "'(' for cast expression argument");
                let target = self.parse_expr();
                self.expect_token(&TokenKind::RParen, "')' to close cast argument");
                let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
                return Expr::new(ExprKind::PtrCast(Box::new(target), ty), span);
            }
        }

        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Expr::new(ExprKind::Path(segments), span)
    }

    fn parse_macro_call(&mut self, path: Vec<Ident>, start_span: Span) -> Expr {
        let delim_tok = self.peek().clone();
        let delimiter = match delim_tok.kind {
            TokenKind::LParen => MacroDelimiter::Paren,
            TokenKind::LBrace => MacroDelimiter::Brace,
            TokenKind::LBracket => MacroDelimiter::Bracket,
            _ => {
                self.error(
                    ErrorCode::E0203,
                    "invalid macro delimiter, expected '(', '{', or '['",
                    delim_tok.span,
                );
                return Expr::new(ExprKind::Error, start_span);
            }
        };
        self.bump();

        let mut args = Vec::new();
        let close_kind = match delimiter {
            MacroDelimiter::Paren => TokenKind::RParen,
            MacroDelimiter::Brace => TokenKind::RBrace,
            MacroDelimiter::Bracket => TokenKind::RBracket,
        };

        while !self.is_eof()
            && !self
                .check_kind(|k| std::mem::discriminant(k) == std::mem::discriminant(&close_kind))
        {
            if matches!(self.peek().kind, TokenKind::Ident(_))
                && matches!(self.peek_at(1).kind, TokenKind::Eq)
            {
                let name = self.parse_ident_node();
                self.expect_token(&TokenKind::Eq, "'=' for named argument");
                let val = self.parse_expr();
                let span = Span::new(name.span.file_id, name.span.start, val.span.end);
                args.push(MacroArg::Expr(Expr::new(
                    ExprKind::NamedArg(name, Box::new(val)),
                    span,
                )));
            } else {
                args.push(MacroArg::Expr(self.parse_expr()));
            }
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }

        self.expect_token(&close_kind, "closing delimiter for macro call");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Expr::new(
            ExprKind::MacroCall(MacroCallNode {
                path,
                delimiter,
                args,
                span,
            }),
            span,
        )
    }

    fn parse_struct_init(&mut self, name: Ident, start_span: Span) -> Expr {
        self.bump(); // consume '{'
        let mut fields = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RBrace)) {
            let f_span = self.curr_span();
            let f_tok = self.bump();
            let f_name = if let TokenKind::Ident(n) = f_tok.kind {
                Ident::new(n, f_span)
            } else {
                self.error(
                    ErrorCode::E0201,
                    "expected field name in struct init",
                    f_span,
                );
                break;
            };
            self.expect_token(&TokenKind::Colon, "':' after field name");
            let val = self.parse_expr();
            let field_span = Span::new(f_span.file_id, f_span.start, self.prev_span().end);
            fields.push(FieldInit {
                name: f_name,
                value: val,
                span: field_span,
            });
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }
        self.expect_token(&TokenKind::RBrace, "'}' to close struct init");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Expr::new(ExprKind::StructInit(name, fields), span)
    }

    fn parse_paren_or_tuple(&mut self, start_span: Span) -> Expr {
        self.bump(); // consume '('
        if self.eat_kind(|k| matches!(k, TokenKind::RParen)) {
            let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
            if self.eat_kind(|k| matches!(k, TokenKind::FatArrow)) {
                return self.parse_arrow_body(Vec::new(), span);
            }
            return Expr::new(ExprKind::TupleInit(Vec::new()), span);
        }

        let first = self.parse_expr();
        if self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
            let mut items = vec![first];
            while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RParen)) {
                items.push(self.parse_expr());
                if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                    break;
                }
            }
            self.expect_token(&TokenKind::RParen, "')' to close tuple");
            let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
            return Expr::new(ExprKind::TupleInit(items), span);
        }

        self.expect_token(&TokenKind::RParen, "')' to close expression");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);

        if let ExprKind::Ident(ref id) = first.kind
            && self.eat_kind(|k| matches!(k, TokenKind::FatArrow))
        {
            let id = id.clone();
            return self.parse_arrow_body(vec![id], span);
        }

        Expr::new(first.kind, span)
    }

    fn parse_arrow_body(&mut self, params: Vec<Ident>, start_span: Span) -> Expr {
        if self.check_kind(|k| matches!(k, TokenKind::LBrace)) {
            self.error(
                ErrorCode::E0212,
                "arrow function cannot have a block body; use an expression or fun(...) { ... }",
                self.curr_span(),
            );
        }
        let body = self.parse_expr();
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Expr::new(ExprKind::Lambda(params, Box::new(body)), span)
    }

    fn parse_array_literal(&mut self, start_span: Span) -> Expr {
        self.bump(); // consume '['
        let mut elements = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RBracket)) {
            elements.push(self.parse_expr());
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }
        self.expect_token(&TokenKind::RBracket, "']' to close array literal");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Expr::new(ExprKind::ArrayInit(elements), span)
    }

    fn parse_block_expr(&mut self, start_span: Span) -> Expr {
        let block = self.parse_block();
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Expr::new(ExprKind::Block(block), span)
    }

    fn parse_map_or_block_expr(&mut self, span: Span) -> Expr {
        if self.is_map_init_lookahead() {
            self.parse_map_init(span)
        } else {
            self.parse_block_expr(span)
        }
    }

    fn is_map_init_lookahead(&self) -> bool {
        let mut i = 1;
        let mut depth = 0;
        let mut question_count = 0;

        while let Some(tok) = self.tokens.get(self.cursor + i) {
            match &tok.kind {
                TokenKind::RBrace
                | TokenKind::Semicolon
                | TokenKind::Set
                | TokenKind::Rewrite
                | TokenKind::Static
                | TokenKind::Return
                | TokenKind::Recur
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Defer
                | TokenKind::Space
                    if depth == 0 =>
                { return false; }
                TokenKind::LBrace | TokenKind::LParen | TokenKind::LBracket => depth += 1,
                TokenKind::RBrace | TokenKind::RParen | TokenKind::RBracket => { if depth > 0 { depth -= 1; } }
                TokenKind::Question if depth == 0 => { question_count += 1; }
                TokenKind::Colon if depth == 0 => {
                    if question_count > 0 { question_count -= 1;} 
                    else { return true; }
                }
                TokenKind::Eof => return false,
                _ => {}
            }
            
            i += 1;
            
        }
        
        false
    }

    fn parse_map_init(&mut self, start_span: Span) -> Expr {
        self.bump(); // consume '{'
        let mut entries = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RBrace)) {
            let key = self.parse_expr();
            self.expect_token(&TokenKind::Colon, "':' between map key and value");
            let val = self.parse_expr();
            entries.push((key, val));
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) { break; }
        }
        self.expect_token(&TokenKind::RBrace, "'}' to close map literal");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Expr::new(ExprKind::MapInit(entries), span)
    }

    fn parse_region_expr(&mut self, start_span: Span) -> Expr {
        self.bump(); // consume 'region'
        let block = self.parse_block();
        let span = Span::new(start_span.file_id, start_span.start, block.span.end);
        Expr::new(ExprKind::RegionBlock(block), span)
    }

    fn parse_spread_expr(&mut self, start_span: Span, allow_struct: bool) -> Expr {
        self.bump(); // consume '...'
        let operand = self.parse_expr_with_bp(PREFIX_BP, allow_struct);
        let end_span = self.prev_span();
        Expr::new(
            ExprKind::Spread(Box::new(operand)),
            Span::new(start_span.file_id, start_span.start, end_span.end),
        )
    }

    fn parse_colon_expr(&mut self, start_span: Span) -> Expr {
        if matches!(self.peek_at(1).kind, TokenKind::Ident(_)) {
            match self.peek_at(2).kind {
                TokenKind::For => {
                    self.bump(); // consume ':'
                    let label = self.parse_ident_node();
                    return self.parse_for_expr_with_label(Some(label), start_span);
                }
                TokenKind::While => {
                    self.bump(); // consume ':'
                    let label = self.parse_ident_node();
                    return self.parse_while_expr_with_label(Some(label), start_span);
                }
                TokenKind::Loop => {
                    self.bump(); // consume ':'
                    let label = self.parse_ident_node();
                    return self.parse_loop_expr_with_label(Some(label), start_span);
                }
                _ => {}
            }
        }
        let tok = self.peek().clone();
        self.error(
            ErrorCode::E0201,
            format!("expected expression, found {:?}", tok.kind),
            start_span,
        );
        self.bump();
        Expr::new(ExprKind::Error, start_span)
    }

    fn parse_anonymous_fun(&mut self, start_span: Span) -> Expr {
        self.bump(); // consume 'fun'
        let name = if let TokenKind::Ident(n) = &self.peek().kind {
            let span = self.curr_span();
            let name = Ident::new(*n, span);
            self.bump();
            name
        } else {
            Ident::new("<anonymous>", start_span)
        };
        let generics = self.parse_generic_params();
        self.expect_token(&TokenKind::LParen, "'(' for anonymous function parameters");
        let params = self.parse_anon_param_list();
        self.expect_token(&TokenKind::RParen, "')' to close parameters");

        let return_type = if self.eat_kind(|k| matches!(k, TokenKind::Arrow)) {
            Some(self.parse_type())
        } else { None };

        let body = if self.eat_kind(|k| matches!(k, TokenKind::FatArrow)) {
            if self.check_kind(|k| matches!(k, TokenKind::LBrace)) {
                self.parse_block()
            } else {
                let expr = self.parse_expr();
                let ret_stmt = yukic_ast::Stmt::new(
                    yukic_ast::StmtKind::Return(Some(expr.clone())),
                    expr.span,
                );
                yukic_ast::Block {
                    stmts: vec![ret_stmt],
                    span: expr.span,
                }
            }
        } else {
            self.parse_block()
        };
        let span = Span::new(start_span.file_id, start_span.start, body.span.end);
        let decl = yukic_ast::FunDecl {
            name,
            attrs: Vec::new(),
            generics,
            params,
            return_type,
            body,
            span,
        };
        Expr::new(ExprKind::AnonymousFun(Box::new(decl)), span)
    }

    fn parse_anon_param_list(&mut self) -> Vec<yukic_ast::Param> {
        let mut params = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RParen)) {
            let start_span = self.curr_span();
            let is_variadic = self.eat_kind(|k| matches!(k, TokenKind::Ellipsis));
            let is_ref = self.eat_kind(|k| matches!(k, TokenKind::Ref));
            let is_mut = self.eat_kind(|k| matches!(k, TokenKind::Tilde));
            let name = self.parse_ident_node();

            let ty = if self.eat_kind(|k| matches!(k, TokenKind::Colon)) {
                Some(self.parse_type())
            } else { None };

            let default_val = if self.eat_kind(|k| matches!(k, TokenKind::Eq)) {
                if is_variadic {
                    self.error(
                        ErrorCode::E0201,
                        "variadic parameter cannot have a default value",
                        self.prev_span(),
                    );
                }
                Some(self.parse_expr())
            } 

            else { None };

            let ty = if is_variadic {
                ty.map(|t| match t.kind {
                    yukic_ast::types::TypeNodeKind::Array(..) => t,
                    _ => yukic_ast::types::TypeNode::new(
                        yukic_ast::types::TypeNodeKind::Array(Box::new(t), None),
                        start_span,
                    ),
                })
            } 

            else { ty };

            let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
            params.push(yukic_ast::Param {
                name,
                is_ref,
                is_mut,
                is_variadic,
                ty,
                default_val,
                span,
            });
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) { break; }

            if is_variadic {
                self.error(
                    ErrorCode::E0201,
                    "variadic parameter must be the last parameter",
                    start_span,
                );
            }
        }
        params
    }

    fn parse_interpolated_string(&mut self, initial_lit: Option<String>, start_span: Span) -> Expr {
        let mut parts = Vec::new();
        if let Some(s) = initial_lit {
            self.bump(); // consume StringPart
            if !s.is_empty() {
                parts.push(StringPart::Lit(s));
            }
        }

        while !self.is_eof() && self.check_kind(|k| matches!(k, TokenKind::InterpolationStart)) {
            self.bump(); // consume InterpolationStart
            let expr = self.parse_expr();
            self.expect_token(
                &TokenKind::InterpolationEnd,
                "closing '}' for string interpolation",
            );
            parts.push(StringPart::Expr(expr));

            if let TokenKind::StringPart(ref s) = self.peek().kind {
                let text = s.clone();
                self.bump();
                if !text.is_empty() {
                    parts.push(StringPart::Lit(text));
                }
            }
        }

        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Expr::new(ExprKind::InterpolatedString(parts), span)
    }

    fn parse_closure_expr(&mut self, start_span: Span) -> Expr {
        self.bump(); // consume '|'
        let mut params = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::Pipe)) {
            let p_span = self.curr_span();
            let p_tok = self.bump();
            if let TokenKind::Ident(name) = p_tok.kind {
                params.push(Ident::new(name, p_span));
            } else {
                self.error(
                    ErrorCode::E0201,
                    "expected parameter name in lambda",
                    p_span,
                );
                break;
            }
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }
        self.expect_token(&TokenKind::Pipe, "'|' to close closure parameters");
        let body = self.parse_expr();
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Expr::new(ExprKind::Lambda(params, Box::new(body)), span)
    }

    fn parse_if_expr(&mut self, start_span: Span) -> Expr {
        self.bump(); // consume 'if'
        let cond_span = self.curr_span();
        let has_outer_paren = self.check_kind(|k| matches!(k, TokenKind::LParen));
        let cond = self.parse_expr_no_struct();

        if has_outer_paren && cond.span.start == cond_span.start {
            self.warning(
                WarningCode::W0205,
                "redundant parentheses around condition",
                cond.span,
            );
        }

        let then_branch = self.parse_block_with_empty_check();
        let mut elif_branches = Vec::new();

        while self.eat_kind(|k| matches!(k, TokenKind::Elif)) {
            let elif_cond = self.parse_expr_no_struct();
            let elif_body = self.parse_block_with_empty_check();
            elif_branches.push((elif_cond, elif_body));
        }

        let else_branch = if self.eat_kind(|k| matches!(k, TokenKind::Else)) {
            Some(self.parse_block_with_empty_check())
        } else {
            None
        };

        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Expr::new(
            ExprKind::If {
                cond: Box::new(cond),
                then_branch,
                elif_branches,
                else_branch,
            },
            span,
        )
    }

    fn parse_switch_expr(&mut self, start_span: Span) -> Expr {
        self.bump(); // consume 'switch'
        let target = self.parse_expr_no_struct();
        self.expect_token(&TokenKind::LBrace, "'{' after switch expression");

        let mut arms = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RBrace)) {
            let arm_span = self.curr_span();
            let pat = self.parse_pattern();
            let guard = if self.eat_kind(|k| matches!(k, TokenKind::If)) {
                Some(self.parse_expr_no_struct())
            } else {
                None
            };

            if !self.eat_kind(|k| matches!(k, TokenKind::Arrow)) {
                self.error(
                    ErrorCode::E0201,
                    "expected '->' after switch pattern",
                    self.curr_span(),
                );
            }

            let body = if self.check_kind(|k| matches!(k, TokenKind::LBrace)) {
                self.parse_block()
            } else {
                let expr = self.parse_expr();
                let b_span = expr.span;
                yukic_ast::Block::new(
                    vec![yukic_ast::Stmt::new(
                        yukic_ast::StmtKind::Expr(expr),
                        b_span,
                    )],
                    b_span,
                )
            };

            self.eat_kind(|k| matches!(k, TokenKind::Comma));
            let span = Span::new(arm_span.file_id, arm_span.start, self.prev_span().end);
            arms.push(SwitchArm {
                pattern: pat,
                guard,
                body,
                span,
            });
        }

        self.expect_token(&TokenKind::RBrace, "'}' to close switch body");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Expr::new(
            ExprKind::Switch {
                target: Box::new(target),
                arms,
            },
            span,
        )
    }

    fn parse_while_expr(&mut self, start_span: Span) -> Expr {
        self.parse_while_expr_with_label(None, start_span)
    }

    fn parse_while_expr_with_label(&mut self, outer_label: Option<Ident>, start_span: Span) -> Expr {
        self.bump(); // consume 'while'
        let inline_label = if self.check_kind(|k| matches!(k, TokenKind::Ident(_)))
            && matches!(self.peek_at(1).kind, TokenKind::Colon)
        {
            let id = self.parse_ident_node();
            self.bump(); // consume ':'
            Some(id)
        } else {
            None
        };
        let label = outer_label.or(inline_label);

        let cond_span = self.curr_span();
        let has_outer_paren = self.check_kind(|k| matches!(k, TokenKind::LParen));
        let cond = self.parse_expr_no_struct();

        if has_outer_paren && cond.span.start == cond_span.start {
            self.warning(
                WarningCode::W0205,
                "redundant parentheses around condition",
                cond.span,
            );
        }

        let body = self.parse_block_with_empty_check();
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Expr::new(
            ExprKind::While {
                label,
                cond: Box::new(cond),
                body,
            },
            span,
        )
    }

    fn parse_for_expr(&mut self, start_span: Span) -> Expr {
        self.parse_for_expr_with_label(None, start_span)
    }

    fn parse_for_expr_with_label(&mut self, outer_label: Option<Ident>, start_span: Span) -> Expr {
        self.bump(); // consume 'for'
        let first_ident = self.parse_ident_node();
        let (inline_label, index_var, item_var) =
            if self.eat_kind(|k| matches!(k, TokenKind::Colon)) {
                let next_ident = self.parse_ident_node();
                if self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                    let it_ident = self.parse_ident_node();
                    (Some(first_ident), Some(next_ident), it_ident)
                } else {
                    (Some(first_ident), None, next_ident)
                }
            } else if self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                let it_ident = self.parse_ident_node();
                (None, Some(first_ident), it_ident)
            } else {
                (None, None, first_ident)
            };

        let label = outer_label.or(inline_label);
        self.expect_token(&TokenKind::In, "'in' in for loop");
        let iter = self.parse_expr_no_struct();
        let body = self.parse_block_with_empty_check();
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Expr::new(
            ExprKind::For {
                label,
                index_var,
                item_var,
                iter: Box::new(iter),
                body,
            },
            span,
        )
    }

    fn parse_loop_expr(&mut self, start_span: Span) -> Expr {
        self.parse_loop_expr_with_label(None, start_span)
    }

    fn parse_loop_expr_with_label(&mut self, outer_label: Option<Ident>, start_span: Span) -> Expr {
        self.bump(); // consume 'loop'
        let inline_label = if self.check_kind(|k| matches!(k, TokenKind::Ident(_)))
            && matches!(self.peek_at(1).kind, TokenKind::Colon)
        {
            let id = self.parse_ident_node();
            self.bump(); // consume ':'
            Some(id)
        } else {
            None
        };
        let label = outer_label.or(inline_label);

        let body = self.parse_block_with_empty_check();
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Expr::new(ExprKind::Loop { label, body }, span)
    }

    fn parse_sus_expr(&mut self, start_span: Span) -> Expr {
        self.bump(); // consume 'sus'
        let body = self.parse_block_with_empty_check();
        let mut catches = Vec::new();

        while self.eat_kind(|k| matches!(k, TokenKind::Catch)) {
            let c_span = self.prev_span();
            let mut error_var = None;
            let mut error_type = None;

            if self.check_kind(|k| matches!(k, TokenKind::LParen)) {
                let p_span = self.curr_span();
                self.error(
                    ErrorCode::E0201,
                    "parentheses around catch parameter are not allowed; use 'catch <param>' instead",
                    p_span,
                );
                self.bump();
                if matches!(self.peek_at(0).kind, TokenKind::Ident(_))
                    && matches!(self.peek_at(1).kind, TokenKind::Colon)
                {
                    let id = self.parse_ident_node();
                    self.bump();
                    error_var = Some(id);
                    error_type = Some(self.parse_type());
                } else if !self.check_kind(|k| matches!(k, TokenKind::RParen)) {
                    error_type = Some(self.parse_type());
                }
                self.expect_token(&TokenKind::RParen, "')' to close catch clause parameter");
            } else if !self.check_kind(|k| matches!(k, TokenKind::LBrace)) {
                if matches!(self.peek_at(0).kind, TokenKind::Ident(_))
                    && matches!(self.peek_at(1).kind, TokenKind::Colon)
                {
                    let id = self.parse_ident_node();
                    self.bump();
                    error_var = Some(id);
                    error_type = Some(self.parse_type());
                } else {
                    error_type = Some(self.parse_type());
                }
            }

            let c_body = self.parse_block_with_empty_check();
            let span = Span::new(c_span.file_id, c_span.start, self.prev_span().end);
            catches.push(CatchClause {
                error_var,
                error_type,
                body: c_body,
                span,
            });
        }

        let finally_branch = if self.eat_kind(|k| matches!(k, TokenKind::Finally)) {
            Some(self.parse_block_with_empty_check())
        } else {
            None
        };

        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Expr::new(
            ExprKind::Sus {
                body,
                catches,
                finally_branch,
            },
            span,
        )
    }

    fn parse_infix(&mut self, lhs: Expr, right_bp: u8, allow_struct: bool) -> Expr {
        let op_tok = self.bump();
        let start_span = lhs.span;

        if let TokenKind::Question = op_tok.kind {
            let then_expr = self.parse_expr();
            self.expect_token(&TokenKind::Colon, "':' for ternary operator");
            let else_expr = self.parse_expr_with_bp(right_bp, allow_struct);
            let span = Span::new(start_span.file_id, start_span.start, else_expr.span.end);
            return Expr::new(
                ExprKind::Ternary(Box::new(lhs), Box::new(then_expr), Box::new(else_expr)),
                span,
            );
        }

        if let TokenKind::PipeOp = op_tok.kind {
            let rhs = self.parse_expr_with_bp(right_bp, allow_struct);
            let span = Span::new(start_span.file_id, start_span.start, rhs.span.end);
            return Expr::new(ExprKind::Pipe(Box::new(lhs), Box::new(rhs)), span);
        }

        if let TokenKind::Is = op_tok.kind {
            let ty = self.parse_type();
            let span = Span::new(start_span.file_id, start_span.start, ty.span.end);
            return Expr::new(ExprKind::Is(Box::new(lhs), ty), span);
        }

        if let Some(op) = token_to_binary_op(&op_tok.kind) {
            let rhs = self.parse_expr_with_bp(right_bp, allow_struct);
            let span = Span::new(start_span.file_id, start_span.start, rhs.span.end);
            return Expr::new(ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)), span);
        }

        lhs
    }

    fn parse_postfix(&mut self, mut lhs: Expr, _allow_struct: bool) -> Expr {
        loop {
            if self.check_kind(|k| matches!(k, TokenKind::LParen)) {
                lhs = self.parse_call_args(lhs);
            } else if self.eat_kind(|k| matches!(k, TokenKind::LBracket)) {
                lhs = self.parse_index_or_slice(lhs);
            } else if self.eat_kind(|k| matches!(k, TokenKind::Dot)) {
                lhs = self.parse_dot_access(lhs);
            } else if self.eat_kind(|k| matches!(k, TokenKind::QuestionDot)) {
                lhs = self.parse_safe_call(lhs);
            } else {
                break;
            }
        }
        lhs
    }

    fn parse_call_args(&mut self, callee: Expr) -> Expr {
        let args = self.parse_arg_list();
        let span = Span::new(callee.span.file_id, callee.span.start, self.prev_span().end);
        Expr::new(ExprKind::Call(Box::new(callee), args), span)
    }

    fn parse_index_or_slice(&mut self, target: Expr) -> Expr {
        let start_span = target.span;
        if self.eat_kind(|k| matches!(k, TokenKind::DotDot)) {
            let end_expr = if self.check_kind(|k| matches!(k, TokenKind::RBracket)) {
                None
            } else {
                Some(Box::new(self.parse_expr()))
            };
            self.expect_token(&TokenKind::RBracket, "']' to close slice");
            let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
            return Expr::new(
                ExprKind::SliceAccess(Box::new(target), None, end_expr, false),
                span,
            );
        }

        if self.eat_kind(|k| matches!(k, TokenKind::DotDotEq)) {
            let end_expr = Some(Box::new(self.parse_expr()));
            self.expect_token(&TokenKind::RBracket, "']' to close slice");
            let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
            return Expr::new(
                ExprKind::SliceAccess(Box::new(target), None, end_expr, true),
                span,
            );
        }

        let first = self.parse_expr();
        if let ExprKind::Binary(BinaryOp::Range, start, end) = first.kind {
            self.expect_token(&TokenKind::RBracket, "']' to close slice");
            let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
            return Expr::new(
                ExprKind::SliceAccess(Box::new(target), Some(start), Some(end), false),
                span,
            );
        }

        if let ExprKind::Binary(BinaryOp::RangeInclusive, start, end) = first.kind {
            self.expect_token(&TokenKind::RBracket, "']' to close slice");
            let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
            return Expr::new(
                ExprKind::SliceAccess(Box::new(target), Some(start), Some(end), true),
                span,
            );
        }

        if self.eat_kind(|k| matches!(k, TokenKind::DotDot)) {
            let end_expr = if self.check_kind(|k| matches!(k, TokenKind::RBracket)) {
                None
            } else {
                Some(Box::new(self.parse_expr()))
            };
            self.expect_token(&TokenKind::RBracket, "']' to close slice");
            let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
            return Expr::new(
                ExprKind::SliceAccess(Box::new(target), Some(Box::new(first)), end_expr, false),
                span,
            );
        }

        if self.eat_kind(|k| matches!(k, TokenKind::DotDotEq)) {
            let end_expr = Some(Box::new(self.parse_expr()));
            self.expect_token(&TokenKind::RBracket, "']' to close slice");
            let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
            return Expr::new(
                ExprKind::SliceAccess(Box::new(target), Some(Box::new(first)), end_expr, true),
                span,
            );
        }

        self.expect_token(&TokenKind::RBracket, "']' to close index");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Expr::new(
            ExprKind::IndexAccess(Box::new(target), Box::new(first)),
            span,
        )
    }

    fn parse_dot_access(&mut self, target: Expr) -> Expr {
        let id_span = self.curr_span();
        let tok = self.bump();
        let ident = if let TokenKind::Ident(name) = tok.kind {
            Ident::new(name, id_span)
        } else if let TokenKind::IntLiteral { value, .. } = tok.kind {
            Ident::new(value.to_string(), id_span)
        } else if matches!(tok.kind, TokenKind::Get) {
            Ident::new("get", id_span)
        } else if matches!(tok.kind, TokenKind::Set) {
            Ident::new("set", id_span)
        } else if matches!(tok.kind, TokenKind::Init) {
            Ident::new("init", id_span)
        } else {
            self.error(ErrorCode::E0201, "expected field or method name", id_span);
            return target;
        };

        if self.eat_kind(|k| matches!(k, TokenKind::DoubleColon)) {
            let method_span = self.curr_span();
            let m_tok = self.bump();
            let method = if let TokenKind::Ident(m) = m_tok.kind {
                Ident::new(m, method_span)
            } else if matches!(m_tok.kind, TokenKind::Get) {
                Ident::new("get", method_span)
            } else if matches!(m_tok.kind, TokenKind::Set) {
                Ident::new("set", method_span)
            } else if matches!(m_tok.kind, TokenKind::Init) {
                Ident::new("init", method_span)
            } else {
                self.error(
                    ErrorCode::E0201,
                    "expected method name after '::'",
                    method_span,
                );
                return target;
            };
            let args = self.parse_arg_list();
            let span = Span::new(target.span.file_id, target.span.start, self.prev_span().end);
            return Expr::new(
                ExprKind::QualifiedMethodCall(Box::new(target), ident, method, args),
                span,
            );
        }

        if self.check_kind(|k| matches!(k, TokenKind::LParen)) {
            let args = self.parse_arg_list();
            let span = Span::new(target.span.file_id, target.span.start, self.prev_span().end);
            return Expr::new(ExprKind::MethodCall(Box::new(target), ident, args), span);
        }

        let span = Span::new(target.span.file_id, target.span.start, id_span.end);
        Expr::new(ExprKind::FieldAccess(Box::new(target), ident), span)
    }

    fn parse_safe_call(&mut self, target: Expr) -> Expr {
        let id_span = self.curr_span();
        let tok = self.bump();
        let ident = if let TokenKind::Ident(name) = tok.kind {
            Ident::new(name, id_span)
        } else if matches!(tok.kind, TokenKind::Get) {
            Ident::new("get", id_span)
        } else if matches!(tok.kind, TokenKind::Set) {
            Ident::new("set", id_span)
        } else if matches!(tok.kind, TokenKind::Init) {
            Ident::new("init", id_span)
        } else {
            self.error(ErrorCode::E0201, "expected method name after '?.'", id_span);
            return target;
        };

        let args = if self.check_kind(|k| matches!(k, TokenKind::LParen)) {
            self.parse_arg_list()
        } else {
            Vec::new()
        };

        let span = Span::new(target.span.file_id, target.span.start, self.prev_span().end);
        Expr::new(ExprKind::SafeCall(Box::new(target), ident, args), span)
    }

    fn parse_arg_list(&mut self) -> Vec<Expr> {
        self.expect_token(&TokenKind::LParen, "'(' for argument list");
        let mut args = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RParen)) {
            if matches!(self.peek().kind, TokenKind::Ident(_))
                && matches!(self.peek_at(1).kind, TokenKind::Eq)
            {
                let name = self.parse_ident_node();
                self.expect_token(&TokenKind::Eq, "'=' for named argument");
                let val = self.parse_expr();
                let span = Span::new(name.span.file_id, name.span.start, val.span.end);
                args.push(Expr::new(ExprKind::NamedArg(name, Box::new(val)), span));
            } else {
                args.push(self.parse_expr());
            }
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }
        self.expect_token(&TokenKind::RParen, "')' to close argument list");
        args
    }

    fn parse_init_target(&mut self) -> Expr {
        let span = self.curr_span();
        let ident = self.parse_ident_node();
        if self.eat_kind(|k| matches!(k, TokenKind::DoubleColon)) {
            self.parse_path_or_macro(ident, span)
        } else {
            Expr::new(ExprKind::Ident(ident), span)
        }
    }

    fn parse_class_init(&mut self, start_span: Span) -> Expr {
        let target = self.parse_init_target();
        if self.check_kind(|k| matches!(k, TokenKind::Lt)) {
            self.bump();
            while !self.is_eof() && !self.check_gt() {
                let _ = self.parse_type();
                if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                    break;
                }
            }
            self.expect_gt("'>' to close generic type arguments");
        }
        let args = self.parse_arg_list();
        let end_span = self.prev_span();
        Expr::new(
            ExprKind::ClassInit(Box::new(target), args),
            Span::new(start_span.file_id, start_span.start, end_span.end),
        )
    }

    pub fn parse_ident_node(&mut self) -> Ident {
        let span = self.curr_span();
        let tok = self.bump();
        if let TokenKind::Ident(name) = tok.kind {
            Ident::new(name, span)
        } else if matches!(tok.kind, TokenKind::Init) {
            Ident::new("init", span)
        } else if matches!(tok.kind, TokenKind::Get) {
            Ident::new("get", span)
        } else if matches!(tok.kind, TokenKind::Set) {
            Ident::new("set", span)
        } else {
            self.error(ErrorCode::E0201, "expected identifier", span);
            Ident::new("<error>", span)
        }
    }

    pub fn parse_block_with_empty_check(&mut self) -> yukic_ast::Block {
        let block = self.parse_block();
        if block.stmts.is_empty() {
            self.warning(
                WarningCode::W0204,
                "empty block statement; consider using 'space' keyword if intentional",
                block.span,
            );
        }
        block
    }
}
