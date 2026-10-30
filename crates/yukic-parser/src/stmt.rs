#![deny(unsafe_code)]

use crate::stream::TokenStream;
use yukic_ast::{
    ArrayDestructElement, AssignOp, BindingTarget, Block, Expr, ExprKind, Ident, LValue,
    MapDestructField, StaticDecl, Stmt, StmtKind, TypeNodeKind,
};
use yukic_diagnostics::{ErrorCode, WarningCode};
use yukic_lexer::TokenKind;
use yukic_span::Span;

impl TokenStream<'_> {
    pub fn parse_block(&mut self) -> Block {
        let start_span = self.curr_span();
        self.expect_token(&TokenKind::LBrace, "'{' to begin block");

        let mut stmts = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RBrace)) {
            if self.eat_kind(|k| matches!(k, TokenKind::Semicolon)) {
                self.warning(WarningCode::W0202, "redundant semicolon", self.prev_span());
                continue;
            }

            let stmt = self.parse_stmt();
            stmts.push(stmt);

            if self.eat_kind(|k| matches!(k, TokenKind::Semicolon)) {
                while self.eat_kind(|k| matches!(k, TokenKind::Semicolon)) {
                    self.warning(WarningCode::W0202, "redundant semicolon", self.prev_span());
                }
            }
        }

        self.expect_token(&TokenKind::RBrace, "'}' to close block");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Block::new(stmts, span)
    }

    pub fn parse_stmt(&mut self) -> Stmt {
        let start_span = self.curr_span();
        let tok = self.peek().clone();

        match tok.kind {
            TokenKind::Set => self.parse_set_stmt(start_span),
            TokenKind::Rewrite => self.parse_rewrite_stmt(start_span),
            TokenKind::Static => self.parse_static_stmt(start_span),
            TokenKind::Return => self.parse_return_stmt(start_span),
            TokenKind::Recur => self.parse_recur_stmt(start_span),
            TokenKind::Break => self.parse_break_stmt(start_span),
            TokenKind::Continue => self.parse_continue_stmt(start_span),
            TokenKind::Defer => self.parse_defer_stmt(start_span),
            TokenKind::Space => {
                self.bump();
                Stmt::new(StmtKind::Space, start_span)
            }
            _ => self.parse_expr_or_macro_stmt(),
        }
    }

    fn parse_set_stmt(&mut self, start_span: Span) -> Stmt {
        self.bump(); // consume 'set'
        let target = self.parse_binding_target();

        let ty = if self.eat_kind(|k| matches!(k, TokenKind::Colon)) {
            Some(self.parse_type())
        } else {
            None
        };

        let init = if self.eat_kind(|k| matches!(k, TokenKind::Eq)) {
            self.parse_expr()
        } else if let Some(ref t) = ty {
            if let TypeNodeKind::Array(ref elem_ty, Some(size)) = t.kind {
                Expr::new(
                    ExprKind::ArrayAlloc(elem_ty.clone(), size),
                    t.span,
                )
            } else if let TypeNodeKind::Array(_, None) = t.kind {
                Expr::new(
                    ExprKind::ArrayInit(Vec::new()),
                    t.span,
                )
            } else {
                self.expect_token(&TokenKind::Eq, "'=' in set statement");
                self.parse_expr()
            }
        } else {
            self.expect_token(&TokenKind::Eq, "'=' in set statement");
            self.parse_expr()
        };

        let span = Span::new(start_span.file_id, start_span.start, init.span.end);
        Stmt::new(StmtKind::Set { target, ty, init }, span)
    }

    fn parse_binding_target(&mut self) -> BindingTarget {
        let first = self.parse_single_binding_target();
        if self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
            let mut targets = vec![first];
            loop {
                targets.push(self.parse_single_binding_target());
                if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                    break;
                }
            }
            return BindingTarget::Tuple(targets);
        }
        first
    }

    fn parse_single_binding_target(&mut self) -> BindingTarget {
        let span = self.curr_span();
        if self.eat_kind(|k| matches!(k, TokenKind::LParen)) {
            let mut targets = Vec::new();
            while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RParen)) {
                targets.push(self.parse_single_binding_target());
                if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                    break;
                }
            }
            self.expect_token(&TokenKind::RParen, "')' to close tuple destructuring");
            return BindingTarget::Tuple(targets);
        }

        if self.eat_kind(|k| matches!(k, TokenKind::LBracket)) {
            return self.parse_array_destructure(span);
        }

        if self.eat_kind(|k| matches!(k, TokenKind::LBrace)) {
            return self.parse_map_destructure(span);
        }

        BindingTarget::Ident(self.parse_ident_node())
    }

    fn parse_array_destructure(&mut self, _start_span: Span) -> BindingTarget {
        let mut elements = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RBracket)) {
            let el_span = self.curr_span();
            let is_rest = self.eat_kind(|k| matches!(k, TokenKind::Ellipsis));
            let target = self.parse_binding_target();
            elements.push(ArrayDestructElement {
                target,
                is_rest,
                span: Span::new(el_span.file_id, el_span.start, self.prev_span().end),
            });
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }
        self.expect_token(&TokenKind::RBracket, "']' to close array destructuring");
        BindingTarget::Array(elements)
    }

    fn parse_map_destructure(&mut self, _start_span: Span) -> BindingTarget {
        let mut fields = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RBrace)) {
            let f_span = self.curr_span();
            let field = self.parse_ident_node();
            let alias = if self.eat_kind(|k| matches!(k, TokenKind::Colon)) {
                Some(self.parse_binding_target())
            } 

            else { None };


            let default_val = if self.eat_kind(|k| matches!(k, TokenKind::Eq)) {
                Some(self.parse_expr())
            } else { None };


            fields.push(MapDestructField {
                field,
                alias,
                default_val,
                span: Span::new(f_span.file_id, f_span.start, self.prev_span().end),
            });
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }
        self.expect_token(&TokenKind::RBrace, "'}' to close struct destructuring");
        BindingTarget::Map(fields)
    }

    fn parse_rewrite_stmt(&mut self, start_span: Span) -> Stmt {
        self.bump(); // consume 'rewrite'
        let target_expr = self.parse_expr();
        let target = self.expr_to_lvalue(target_expr);

        let op = self.parse_assign_op();
        let value = self.parse_expr();
        let span = Span::new(start_span.file_id, start_span.start, value.span.end);
        Stmt::new(StmtKind::Rewrite { target, op, value }, span)
    }

    fn expr_to_lvalue(&mut self, expr: Expr) -> LValue {
        match expr.kind {
            ExprKind::Ident(id) => LValue::Ident(id),
            ExprKind::FieldAccess(target, field) => LValue::Field(target, field),
            ExprKind::IndexAccess(target, index) => LValue::Index(target, index),
            _ => {
                self.error(
                    ErrorCode::E0201,
                    "invalid LValue in rewrite statement",
                    expr.span,
                );
                LValue::Ident(Ident::new("<error>", expr.span))
            }
        }
    }

    fn parse_assign_op(&mut self) -> AssignOp {
        let span = self.curr_span();
        let tok = self.bump();
        match tok.kind {
            TokenKind::Eq => AssignOp::Eq,
            TokenKind::PlusEq => AssignOp::PlusEq,
            TokenKind::MinusEq => AssignOp::MinusEq,
            TokenKind::StarEq => AssignOp::StarEq,
            TokenKind::SlashEq => AssignOp::SlashEq,
            TokenKind::PercentEq => AssignOp::PercentEq,
            TokenKind::StarStarEq => AssignOp::StarStarEq,
            TokenKind::AmpEq => AssignOp::AmpEq,
            TokenKind::PipeEq => AssignOp::PipeEq,
            TokenKind::CaretEq => AssignOp::CaretEq,
            TokenKind::LtLtEq => AssignOp::ShlEq,
            TokenKind::GtGtEq => AssignOp::ShrEq,
            _ => {
                self.error(ErrorCode::E0201, "expected assignment operator", span);
                AssignOp::Eq
            }
        }
    }

    fn parse_static_stmt(&mut self, start_span: Span) -> Stmt {
        self.bump(); // consume 'static'
        let name = self.parse_ident_node();
        let ty = if self.eat_kind(|k| matches!(k, TokenKind::Colon)) {
            Some(self.parse_type())
        } else {
            None
        };
        self.expect_token(&TokenKind::Eq, "'=' in static declaration");
        let init = self.parse_expr();
        let span = Span::new(start_span.file_id, start_span.start, init.span.end);
        Stmt::new(
            StmtKind::Static(StaticDecl {
                name,
                ty,
                init,
                span,
            }),
            span,
        )
    }

    fn parse_return_stmt(&mut self, start_span: Span) -> Stmt {
        self.bump(); // consume 'return'
        if self
            .check_kind(|k| matches!(k, TokenKind::Semicolon | TokenKind::RBrace | TokenKind::Eof))
        {
            return Stmt::new(StmtKind::Return(None), start_span);
        }
        let val = self.parse_expr();
        let span = Span::new(start_span.file_id, start_span.start, val.span.end);
        Stmt::new(StmtKind::Return(Some(val)), span)
    }

    fn parse_recur_stmt(&mut self, start_span: Span) -> Stmt {
        self.bump(); // consume 'recur'
        self.expect_token(&TokenKind::LParen, "'(' after recur");
        let mut args = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RParen)) {
            args.push(self.parse_expr());
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }
        self.expect_token(&TokenKind::RParen, "')' to close recur argument list");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        Stmt::new(StmtKind::Recur(args), span)
    }

    fn parse_break_stmt(&mut self, start_span: Span) -> Stmt {
        self.bump(); // consume 'break'
        self.eat_kind(|k| matches!(k, TokenKind::Colon));
        let label = if let TokenKind::Ident(l) = self.peek().kind.clone() {
            let s = self.curr_span();
            self.bump();
            Some(Ident::new(l, s))
        } else {
            None
        };
        let end = label.as_ref().map_or(start_span.end, |l| l.span.end);
        Stmt::new(
            StmtKind::Break(label),
            Span::new(start_span.file_id, start_span.start, end),
        )
    }

    fn parse_continue_stmt(&mut self, start_span: Span) -> Stmt {
        self.bump(); // consume 'continue'
        self.eat_kind(|k| matches!(k, TokenKind::Colon));
        let label = if let TokenKind::Ident(l) = self.peek().kind.clone() {
            let s = self.curr_span();
            self.bump();
            Some(Ident::new(l, s))
        } else {
            None
        };
        let end = label.as_ref().map_or(start_span.end, |l| l.span.end);
        Stmt::new(
            StmtKind::Continue(label),
            Span::new(start_span.file_id, start_span.start, end),
        )
    }

    fn parse_defer_stmt(&mut self, start_span: Span) -> Stmt {
        self.bump(); // consume 'defer'
        let body = self.parse_block();
        if body.stmts.is_empty() {
            self.warning(
                WarningCode::W0203,
                "empty defer block; consider removing it",
                body.span,
            );
        }
        let span = Span::new(start_span.file_id, start_span.start, body.span.end);
        Stmt::new(StmtKind::Defer(body), span)
    }

    fn parse_expr_or_macro_stmt(&mut self) -> Stmt {
        let expr = self.parse_expr();
        let span = expr.span;
        if let ExprKind::MacroCall(call) = expr.kind {
            Stmt::new(StmtKind::MacroCall(call), span)
        } else {
            Stmt::new(StmtKind::Expr(expr), span)
        }
    }
}
