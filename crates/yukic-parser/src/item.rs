#![deny(unsafe_code)]

use crate::stream::TokenStream;
use yukic_ast::{
    AccessorKind, AliasDecl, AliasKind, AttributeArgs, AttributeNode, AttributePath,
    CAttributeNode, CParam, CfunDecl, ClassDecl, ClassField, CstructDecl, EnumDecl, EnumVariant,
    Expr, ExprKind, ExterncBlock, ExterncItem, FieldDef, FunDecl, Ident, ImportDecl, ImportPath,
    ImportPathSegment, ImportPattern, ImportSymbolItem, ImportSymbolsClause, InterfaceDecl,
    InterfaceMethodDef, Item, ItemId, ItemKind, MacroBody,
    MacroDefDecl, MacroParam, MacroParamKind, MethodDecl, Module, Param, PathPrefix,
    PropertyAccessor, StaticDecl, StaticMethodDecl, StructDecl, ThisParam, TypeAliasDecl, TypeNode,
    TypeNodeKind, Visibility,
};
use yukic_diagnostics::{ErrorCode, WarningCode};
use yukic_lexer::TokenKind;
use yukic_span::Span;

impl TokenStream<'_> {
    pub fn parse_module(&mut self) -> Module {
        let mut module = Module::new(self.file_id);
        module.module_attrs = self.parse_module_attributes();

        let mut next_id = 0u32;
        while !self.is_eof() {
            if self.eat_kind(|k| matches!(k, TokenKind::Semicolon)) {
                continue;
            }
            if let Some(item) = self.parse_item(&mut next_id) {
                module.items.push(item);
            }
        }

        module
    }

    fn parse_module_attributes(&mut self) -> Vec<AttributeNode> {
        let mut attrs = Vec::new();
        while self.eat_kind(|k| matches!(k, TokenKind::ModuleAttrStart)) {
            let start_span = self.prev_span();
            attrs.extend(self.parse_attribute_group(start_span, "module attribute"));
        }
        attrs
    }

    pub fn parse_attributes(&mut self) -> Vec<AttributeNode> {
        let mut attrs = Vec::new();
        while self.eat_kind(|k| matches!(k, TokenKind::AttrStart)) {
            let start_span = self.prev_span();
            attrs.extend(self.parse_attribute_group(start_span, "attribute"));
        }
        attrs
    }

    fn parse_attribute_path(&mut self) -> AttributePath {
        let first = self.parse_ident_node();
        let mut segments = vec![first];
        while self.eat_kind(|k| matches!(k, TokenKind::DoubleColon)) {
            segments.push(self.parse_ident_node());
        }
        let span = Span::new(
            segments.first().unwrap().span.file_id,
            segments.first().unwrap().span.start,
            segments.last().unwrap().span.end,
        );
        AttributePath::new(segments, span)
    }

    fn parse_attribute_group(&mut self, start_span: Span, kind: &str) -> Vec<AttributeNode> {
        let mut attrs = Vec::new();
        loop {
            let negated = self.eat_kind(|k| matches!(k, TokenKind::Bang));
            let path = self.parse_attribute_path();
            let args = self.parse_attribute_args();
            let span = Span::new(start_span.file_id, start_span.start, self.curr_span().end);
            attrs.push(if negated {
                AttributeNode::new_negated(path, args, span)
            } else {
                AttributeNode::new(path, args, span)
            });

            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                let message = format!("']' to close {kind}");
                self.expect_token(&TokenKind::RBracket, &message);
                break;
            }
        }
        attrs
    }

    fn parse_attribute_args(&mut self) -> AttributeArgs {
        if self.eat_kind(|k| matches!(k, TokenKind::Eq)) {
            let expr = self.parse_expr();
            return AttributeArgs::Positional(vec![expr]);
        }

        if !self.eat_kind(|k| matches!(k, TokenKind::LParen)) {
            return AttributeArgs::None;
        }

        let mut positional = Vec::new();
        let mut named = Vec::new();

        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RParen)) {
            if let TokenKind::Ident(name) = self.peek().kind
                && self.check_kind_at(1, |k| matches!(k, TokenKind::Eq))
            {
                let id_span = self.curr_span();
                let ident = Ident::new(name, id_span);
                self.bump(); // ident
                self.bump(); // '='
                let val = self.parse_expr();
                let span = Span::new(id_span.file_id, id_span.start, val.span.end);
                positional.push(Expr::new(
                    ExprKind::NamedArg(ident.clone(), Box::new(val.clone())),
                    span,
                ));
                named.push((ident, val));
                if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                    break;
                }
                continue;
            }

            positional.push(self.parse_expr());
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }

        self.expect_token(&TokenKind::RParen, "')' to close attribute arguments");
        if !named.is_empty() && positional.len() == named.len() {
            AttributeArgs::Named(named)
        } else if !positional.is_empty() {
            AttributeArgs::Positional(positional)
        } else {
            AttributeArgs::None
        }
    }

    pub fn parse_c_attributes(&mut self) -> Vec<CAttributeNode> {
        if !self.eat_kind(|k| matches!(k, TokenKind::Pipe)) {
            return Vec::new();
        }

        let mut c_attrs = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::Pipe)) {
            let start_span = self.curr_span();
            let name = self.parse_ident_node();
            let arg = if self.eat_kind(|k| matches!(k, TokenKind::Eq)) {
                match &self.peek().kind {
                    TokenKind::StringLiteral(s) => {
                        let str_val = s.clone();
                        self.bump();
                        Some(str_val)
                    }
                    TokenKind::IntLiteral { value, .. } => {
                        let str_val = value.to_string();
                        self.bump();
                        Some(str_val)
                    }
                    _ => None,
                }
            } else {
                None
            };
            let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
            c_attrs.push(CAttributeNode::new(name, arg, span));

            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }

        self.expect_token(&TokenKind::Pipe, "'|' to close FFI attributes");
        self.validate_c_attributes(&c_attrs);
        c_attrs
    }

    fn validate_c_attributes(&mut self, attrs: &[CAttributeNode]) {
        let mut conv_count = 0;
        let mut has_cold = false;
        let mut has_hot = false;
        let mut has_inline_always = false;
        let mut has_inline_never = false;

        for attr in attrs {
            match attr.name.name.as_str() {
                "cdecl" | "stdcall" | "fastcall" | "sysv64" | "win64" => {
                    conv_count += 1;
                    if conv_count > 1 {
                        self.error(
                            ErrorCode::E0215,
                            "conflicting calling conventions in FFI attributes",
                            attr.span,
                        );
                    }
                }
                "cold" => has_cold = true,
                "hot" => has_hot = true,
                "inline_always" => has_inline_always = true,
                "inline_never" => has_inline_never = true,
                _ => {}
            }
        }

        if (has_cold && has_hot) || (has_inline_always && has_inline_never) {
            let span = attrs.first().map_or(self.curr_span(), |a| a.span);
            self.error(
                ErrorCode::E0216,
                "conflicting optimization hints in FFI attributes",
                span,
            );
        }
    }

    fn parse_item(&mut self, next_id: &mut u32) -> Option<Item> {
        let attrs = self.parse_attributes();
        let item_attrs = attrs.clone();
        let c_attrs = self.parse_c_attributes();

        let visibility = if self.eat_kind(|k| matches!(k, TokenKind::Public)) {
            Visibility::Public
        } else {
            Visibility::Local
        };

        let is_locked = self.eat_kind(|k| matches!(k, TokenKind::Locked));
        let start_span = self.curr_span();
        let tok = self.peek().clone();

        let kind = if is_locked {
            if self.check_kind(|k| matches!(k, TokenKind::Class)) {
                if !c_attrs.is_empty() {
                    self.validate_class_c_attrs(&c_attrs);
                }
                ItemKind::Class(self.parse_class_decl(true))
            } else {
                self.error(
                    ErrorCode::E0201,
                    format!("expected 'class' after 'locked', found {:?}", tok.kind),
                    start_span,
                );
                self.synchronize_item();
                return None;
            }
        } else {
            match tok.kind {
                TokenKind::Fun => {
                    if !c_attrs.is_empty() {
                        self.error(
                            ErrorCode::E0210,
                            "FFI attributes '|...|' forbidden on pure Yukino function",
                            start_span,
                        );
                    }
                    ItemKind::Fun(self.parse_fun_decl(attrs))
                }
                TokenKind::Cfun => ItemKind::Cfun(self.parse_cfun_decl(attrs, c_attrs)),
                TokenKind::Class => {
                    if !c_attrs.is_empty() {
                        self.validate_class_c_attrs(&c_attrs);
                    }
                    ItemKind::Class(self.parse_class_decl(false))
                }
                TokenKind::Struct => {
                    if !c_attrs.is_empty() {
                        self.error(
                            ErrorCode::E0210,
                            "FFI attributes forbidden on pure Yukino struct",
                            start_span,
                        );
                    }
                    ItemKind::Struct(self.parse_struct_decl())
                }
                TokenKind::Cstruct => ItemKind::Cstruct(self.parse_cstruct_decl(c_attrs)),
                TokenKind::Interface => ItemKind::Interface(self.parse_interface_decl()),
                TokenKind::Enum => ItemKind::Enum(self.parse_enum_decl()),
                TokenKind::Type => ItemKind::TypeAlias(self.parse_type_alias_decl()),
                TokenKind::Static => ItemKind::Static(self.parse_static_decl()),
                TokenKind::Externc => ItemKind::Externc(self.parse_externc_block()),
                TokenKind::Load => ItemKind::Import(self.parse_load_decl()),
                TokenKind::Alias => ItemKind::Alias(self.parse_alias_decl()),
                TokenKind::Define => ItemKind::Macro(self.parse_macro_def_decl()),
                TokenKind::Set | TokenKind::Rewrite => {
                    self.error(
                        ErrorCode::E0204,
                        "top level statement is forbidden outside functions",
                        start_span,
                    );
                    self.synchronize_item();
                    return None;
                }
                _ => {
                    self.error(
                        ErrorCode::E0201,
                        format!("expected top-level item declaration, found {:?}", tok.kind),
                        start_span,
                    );
                    self.synchronize_item();
                    return None;
                }
            }
        };

        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        let id = ItemId(*next_id);
        *next_id += 1;
        let item_attrs = match &kind {
            ItemKind::Fun(_) | ItemKind::Cfun(_) => Vec::new(),
            _ => item_attrs,
        };
        Some(Item::with_attrs(id, visibility, item_attrs, kind, span))
    }

    fn validate_class_c_attrs(&mut self, c_attrs: &[CAttributeNode]) {
        for attr in c_attrs {
            if attr.name.name == "packed" {
                self.error(
                    ErrorCode::E0217,
                    "invalid attribute '|packed|' on class; classes are managed reference types",
                    attr.span,
                );
            } else {
                self.error(
                    ErrorCode::E0210,
                    "FFI attributes forbidden on class",
                    attr.span,
                );
            }
        }
    }

    fn parse_fun_decl(&mut self, attrs: Vec<AttributeNode>) -> FunDecl {
        let start_span = self.curr_span();
        self.bump(); // consume 'fun'
        let name = self.parse_ident_node();
        let generics = self.parse_generic_params();

        self.expect_token(&TokenKind::LParen, "'(' for function parameters");
        let params = self.parse_param_list();
        self.expect_token(&TokenKind::RParen, "')' to close function parameters");

        let return_type = if self.eat_kind(|k| matches!(k, TokenKind::Arrow)) {
            Some(self.parse_type())
        } else {
            None
        };

        let body = self.parse_block();
        if body.stmts.is_empty() {
            self.warning(
                WarningCode::W0201,
                "empty function body; use 'space' keyword if intentional",
                body.span,
            );
        }

        self.validate_test_function(&name, &attrs, &generics, &params, return_type.as_ref());
        let span = Span::new(start_span.file_id, start_span.start, body.span.end);
        FunDecl {
            name,
            attrs,
            generics,
            params,
            return_type,
            body,
            span,
        }
    }

    fn validate_test_function(
        &mut self,
        name: &Ident,
        attrs: &[AttributeNode],
        generics: &[yukic_ast::GenericParam],
        params: &[Param],
        return_type: Option<&TypeNode>,
    ) {
        let is_test = attrs.iter().any(|a| a.is("test"));
        if !is_test {
            return;
        }

        if !params.is_empty() {
            self.error(
                ErrorCode::E0219,
                format!("test function '{}' cannot have parameters", name.name),
                name.span,
            );
        }

        if let Some(ret) = return_type {
            let is_void = match &ret.kind {
                TypeNodeKind::Ident(id) => id.name == "void",
                _ => false,
            };
            if !is_void {
                self.error(
                    ErrorCode::E0220,
                    format!("test function '{}' cannot return a value", name.name),
                    ret.span,
                );
            }
        }

        if !generics.is_empty() {
            self.error(
                ErrorCode::E0221,
                format!(
                    "test function '{}' cannot have generic parameters",
                    name.name
                ),
                name.span,
            );
        }
    }

    fn parse_cfun_decl(
        &mut self,
        yukino_attrs: Vec<AttributeNode>,
        c_attrs: Vec<CAttributeNode>,
    ) -> CfunDecl {
        let start_span = self.curr_span();
        self.bump(); // consume 'cfun'
        let name = self.parse_ident_node();

        if self.check_kind(|k| matches!(k, TokenKind::Lt)) {
            self.error(
                ErrorCode::E0206,
                "cfun declaration cannot have generic type parameters",
                self.curr_span(),
            );
            self.parse_generic_params();
        }

        self.expect_token(&TokenKind::LParen, "'(' for cfun parameters");
        let params = self.parse_cparam_list();
        self.expect_token(&TokenKind::RParen, "')' to close cfun parameters");

        let return_type = if self.eat_kind(|k| matches!(k, TokenKind::Arrow)) {
            Some(self.parse_type())
        } else {
            None
        };

        let symbol_override = if self.eat_kind(|k| matches!(k, TokenKind::As)) {
            Some(self.parse_ident_node())
        } else {
            None
        };

        let body = if self.check_kind(|k| matches!(k, TokenKind::LBrace)) {
            Some(self.parse_block())
        } else {
            None
        };

        let end = body.as_ref().map_or(self.prev_span().end, |b| b.span.end);
        let span = Span::new(start_span.file_id, start_span.start, end);
        CfunDecl {
            name,
            c_attrs,
            yukino_attrs,
            params,
            return_type,
            symbol_override,
            body,
            span,
        }
    }

    fn parse_param_list(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RParen)) {
            let start_span = self.curr_span();
            let is_variadic = self.eat_kind(|k| matches!(k, TokenKind::Ellipsis));
            let is_ref = self.eat_kind(|k| matches!(k, TokenKind::Ref));
            let is_mut = self.eat_kind(|k| matches!(k, TokenKind::Tilde));
            let name = self.parse_ident_node();

            let ty = if self.eat_kind(|k| matches!(k, TokenKind::Colon)) {
                Some(self.parse_type())
            } else {
                self.error(
                    ErrorCode::E0213,
                    format!(
                        "missing type annotation for function parameter '{}'",
                        name.name
                    ),
                    name.span,
                );
                None
            };

            let default_val = if self.eat_kind(|k| matches!(k, TokenKind::Eq)) {
                if is_variadic {
                    self.error(
                        ErrorCode::E0201,
                        "variadic parameter cannot have a default value",
                        self.prev_span(),
                    );
                }
                Some(self.parse_expr())
            } else {
                None
            };

            let ty = if is_variadic {
                ty.map(|t| match t.kind {
                    yukic_ast::types::TypeNodeKind::Array(..) => t,
                    _ => yukic_ast::types::TypeNode::new(
                        yukic_ast::types::TypeNodeKind::Array(Box::new(t), None),
                        start_span,
                    ),
                })
            } else {
                ty
            };

            let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
            params.push(Param {
                name,
                is_ref,
                is_mut,
                is_variadic,
                ty,
                default_val,
                span,
            });

            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }

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

    fn parse_cparam_list(&mut self) -> Vec<CParam> {
        let mut params = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RParen)) {
            let start_span = self.curr_span();
            if self.eat_kind(|k| matches!(k, TokenKind::Ellipsis)) {
                self.error(
                    ErrorCode::E0208,
                    "variadics forbidden in cfun declaration",
                    start_span,
                );
            }
            let name = self.parse_ident_node();
            self.expect_token(&TokenKind::Colon, "':' for cfun parameter type");
            let ty = self.parse_type();

            if self.eat_kind(|k| matches!(k, TokenKind::Eq)) {
                self.error(
                    ErrorCode::E0207,
                    "default parameter values forbidden in cfun declaration",
                    self.prev_span(),
                );
                self.parse_expr();
            }

            let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
            params.push(CParam { name, ty, span });

            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }
        params
    }

    fn parse_class_decl(&mut self, is_locked: bool) -> ClassDecl {
        let start_span = self.curr_span();
        self.bump(); // consume 'class'
        let name = self.parse_ident_node();
        let _generics = self.parse_generic_params();

        let base_class = if self.eat_kind(|k| matches!(k, TokenKind::Extends)) {
            Some(self.parse_ident_node())
        } else {
            None
        };

        let mut interfaces = Vec::new();
        if self.eat_kind(|k| matches!(k, TokenKind::With)) {
            loop {
                let id = self.parse_ident_node();
                if self.check_kind(|k| matches!(k, TokenKind::Lt)) {
                    let id_span = id.span;
                    let gen_ty = self.parse_generic_type(id, id_span);
                    interfaces.push(Ident::new(gen_ty.to_string(), gen_ty.span));
                } else {
                    interfaces.push(id);
                }
                if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                    break;
                }
            }
        }

        self.expect_token(&TokenKind::LBrace, "'{' to begin class body");
        let (fields, methods, accessors) = self.parse_class_body_members();
        self.expect_token(&TokenKind::RBrace, "'}' to close class body");

        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        ClassDecl {
            name,
            is_locked,
            base_class,
            interfaces,
            fields,
            methods,
            accessors,
            span,
        }
    }

    fn parse_class_body_members(
        &mut self,
    ) -> (Vec<ClassField>, Vec<MethodDecl>, Vec<PropertyAccessor>) {
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut accessors = Vec::new();

        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RBrace)) {
            if self.check_kind(|k| matches!(k, TokenKind::Cfun)) {
                self.error(
                    ErrorCode::E0209,
                    "cfun declaration is forbidden inside class",
                    self.curr_span(),
                );
                self.parse_cfun_decl(Vec::new(), Vec::new());
                continue;
            }

            let attrs = self.parse_attributes();
            let _visibility = self.eat_kind(|k| matches!(k, TokenKind::Public));
            if self.eat_kind(|k| matches!(k, TokenKind::Operator)) {
                self.eat_kind(|k| matches!(k, TokenKind::Fun));
                let mut method = self.parse_operator_method_decl(attrs);
                if method.this_param == ThisParam::None {
                    method.this_param = ThisParam::Value;
                }
                methods.push(method);
            } else if self.eat_kind(|k| matches!(k, TokenKind::Fun)) {
                if self.check_kind(|k| matches!(k, TokenKind::Get))
                    && !self.check_kind_at(1, |k| matches!(k, TokenKind::LParen))
                {
                    self.bump();
                    accessors.push(self.parse_getter_accessor(attrs));
                } else if self.check_kind(|k| matches!(k, TokenKind::Set))
                    && !self.check_kind_at(1, |k| matches!(k, TokenKind::LParen))
                {
                    self.bump();
                    accessors.push(self.parse_setter_accessor(attrs));
                } else if self.eat_kind(|k| matches!(k, TokenKind::Operator)) {
                    let mut method = self.parse_operator_method_decl(attrs);
                    if method.this_param == ThisParam::None {
                        method.this_param = ThisParam::Value;
                    }
                    methods.push(method);
                } else {
                    let mut method = self.parse_method_decl(attrs);
                    if method.this_param == ThisParam::None {
                        method.this_param = ThisParam::Value;
                    }
                    methods.push(method);
                }
            } else if self.eat_kind(|k| matches!(k, TokenKind::Get)) {
                accessors.push(self.parse_getter_accessor(attrs));
            } else if self.check_kind(|k| matches!(k, TokenKind::Set))
                && matches!(self.peek_at(1).kind, TokenKind::Ident(_))
                && matches!(self.peek_at(2).kind, TokenKind::LParen)
            {
                self.bump(); // consume 'set'
                accessors.push(self.parse_setter_accessor(attrs));
            } else {
                fields.push(self.parse_class_field(attrs));
                self.eat_kind(|k| matches!(k, TokenKind::Comma | TokenKind::Semicolon));
            }
        }

        (fields, methods, accessors)
    }

    fn parse_class_field(&mut self, attrs: Vec<AttributeNode>) -> ClassField {
        let start_span = self.curr_span();
        let is_pub = self.eat_kind(|k| matches!(k, TokenKind::Public));
        self.eat_kind(|k| matches!(k, TokenKind::Set));
        let name = self.parse_ident_node();
        self.expect_token(&TokenKind::Colon, "':' for class field type");
        let ty = self.parse_type();
        let default_val = if self.eat_kind(|k| matches!(k, TokenKind::Eq)) {
            Some(self.parse_expr())
        } else {
            None
        };
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        ClassField {
            name,
            attrs,
            ty,
            is_pub,
            default_val,
            span,
        }
    }

    fn parse_operator_name(&mut self) -> Ident {
        let span = self.curr_span();
        let tok = self.bump();
        let op_name = match tok.kind {
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Star => "*",
            TokenKind::Slash => "/",
            TokenKind::Percent => "%",
            TokenKind::StarStar => "**",
            TokenKind::Amp => "&",
            TokenKind::Pipe => "|",
            TokenKind::Caret => "^",
            TokenKind::Tilde => "~",
            TokenKind::LtLt => "<<",
            TokenKind::GtGt => ">>",
            TokenKind::EqEq => "==",
            TokenKind::BangEq => "!=",
            TokenKind::Lt => "<",
            TokenKind::LtEq => "<=",
            TokenKind::Gt => ">",
            TokenKind::GtEq => ">=",
            TokenKind::Bang => "!",
            TokenKind::LBracket => {
                self.expect_token(&TokenKind::RBracket, "']' after '[' in operator name");
                if self.eat_kind(|k| matches!(k, TokenKind::Eq)) {
                    "[]="
                } else {
                    "[]"
                }
            }
            TokenKind::Ident(s) => s,
            _ => {
                self.error(
                    ErrorCode::E0201,
                    format!("expected operator symbol or identifier, found {:?}", tok.kind),
                    span,
                );
                "<error>"
            }
        };
        Ident::new(op_name, span)
    }

    fn parse_method_decl_with_name(
        &mut self,
        attrs: Vec<AttributeNode>,
        name: Ident,
        is_operator: bool,
    ) -> MethodDecl {
        let start_span = self.prev_span();
        self.expect_token(&TokenKind::LParen, "'(' for method parameters");
        let mut this_param = ThisParam::None;
        if self.eat_kind(|k| matches!(k, TokenKind::This)) {
            this_param = ThisParam::Value;
            self.eat_kind(|k| matches!(k, TokenKind::Comma));
        } else if self.check_kind(|k| matches!(k, TokenKind::Ref))
            && self.check_kind_at(1, |k| matches!(k, TokenKind::This))
        {
            self.bump();
            self.expect_token(&TokenKind::This, "'this' after 'ref'");
            this_param = ThisParam::Ref;
            self.eat_kind(|k| matches!(k, TokenKind::Comma));
        }

        let params = self.parse_param_list();
        self.expect_token(&TokenKind::RParen, "')' to close method parameters");

        let return_type = if self.eat_kind(|k| matches!(k, TokenKind::Arrow)) {
            Some(self.parse_type())
        } else {
            None
        };

        let body = self.parse_block();
        let span = Span::new(start_span.file_id, start_span.start, body.span.end);
        MethodDecl {
            name,
            attrs,
            this_param,
            params,
            return_type,
            body,
            is_operator,
            is_override: false,
            span,
        }
    }

    fn parse_method_decl(&mut self, attrs: Vec<AttributeNode>) -> MethodDecl {
        let is_operator = self.eat_kind(|k| matches!(k, TokenKind::Operator));
        let name = if is_operator {
            self.parse_operator_name()
        } else {
            self.parse_ident_node()
        };
        self.parse_method_decl_with_name(attrs, name, is_operator)
    }

    fn parse_operator_method_decl(&mut self, attrs: Vec<AttributeNode>) -> MethodDecl {
        let name = self.parse_operator_name();
        self.parse_method_decl_with_name(attrs, name, true)
    }

    fn parse_getter_accessor(&mut self, attrs: Vec<AttributeNode>) -> PropertyAccessor {
        let start_span = self.prev_span();
        let name = self.parse_ident_node();
        if self.eat_kind(|k| matches!(k, TokenKind::LParen)) {
            self.expect_token(&TokenKind::RParen, "')' for getter accessor");
        }
        let ty = if self.eat_kind(|k| matches!(k, TokenKind::Arrow | TokenKind::Colon)) {
            self.parse_type()
        } else {
            self.parse_type()
        };
        let body = self.parse_block();
        let span = Span::new(start_span.file_id, start_span.start, body.span.end);
        PropertyAccessor {
            name,
            attrs,
            ty,
            kind: AccessorKind::Get,
            body,
            span,
        }
    }

    fn parse_setter_accessor(&mut self, attrs: Vec<AttributeNode>) -> PropertyAccessor {
        let start_span = self.prev_span();
        let name = self.parse_ident_node();
        self.expect_token(&TokenKind::LParen, "'(' for setter accessor parameter");
        let param_name = self.parse_ident_node();
        self.expect_token(&TokenKind::Colon, "':' for setter parameter type");
        let ty = self.parse_type();
        self.expect_token(&TokenKind::RParen, "')' to close setter accessor parameter");
        if self.eat_kind(|k| matches!(k, TokenKind::Arrow | TokenKind::Colon)) {
            let _ = self.parse_type();
        }
        let body = self.parse_block();
        let span = Span::new(start_span.file_id, start_span.start, body.span.end);
        PropertyAccessor {
            name,
            attrs,
            ty,
            kind: AccessorKind::Set(param_name),
            body,
            span,
        }
    }

    fn parse_struct_decl(&mut self) -> StructDecl {
        let start_span = self.curr_span();
        self.bump(); // consume 'struct'
        let name = self.parse_ident_node();
        let _generics = self.parse_generic_params();
        self.expect_token(&TokenKind::LBrace, "'{' to begin struct body");

        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut static_methods = Vec::new();

        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RBrace)) {
            if self.check_kind(|k| matches!(k, TokenKind::Cfun)) {
                self.error(
                    ErrorCode::E0209,
                    "cfun declaration is forbidden inside struct",
                    self.curr_span(),
                );
                self.parse_cfun_decl(Vec::new(), Vec::new());
                continue;
            }

            let attrs = self.parse_attributes();
            if self.eat_kind(|k| matches!(k, TokenKind::Static)) {
                self.expect_token(&TokenKind::Fun, "'fun' after static");
                static_methods.push(self.parse_static_method_decl(attrs));
            } else if self.eat_kind(|k| matches!(k, TokenKind::Operator)) {
                self.eat_kind(|k| matches!(k, TokenKind::Fun));
                methods.push(self.parse_operator_method_decl(attrs));
            } else if self.eat_kind(|k| matches!(k, TokenKind::Fun)) {
                if self.eat_kind(|k| matches!(k, TokenKind::Operator)) {
                    methods.push(self.parse_operator_method_decl(attrs));
                } else {
                    methods.push(self.parse_method_decl(attrs));
                }
            } else {
                fields.push(self.parse_field_def(attrs));
                self.eat_kind(|k| matches!(k, TokenKind::Comma | TokenKind::Semicolon));
            }
        }

        self.expect_token(&TokenKind::RBrace, "'}' to close struct body");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        StructDecl {
            name,
            fields,
            methods,
            static_methods,
            span,
        }
    }

    fn parse_static_method_decl(&mut self, attrs: Vec<AttributeNode>) -> StaticMethodDecl {
        let start_span = self.prev_span();
        let name = self.parse_ident_node();
        self.expect_token(&TokenKind::LParen, "'(' for static method parameters");
        let params = self.parse_param_list();
        self.expect_token(&TokenKind::RParen, "')' to close static method parameters");

        let return_type = if self.eat_kind(|k| matches!(k, TokenKind::Arrow)) {
            Some(self.parse_type())
        } else {
            None
        };

        let body = self.parse_block();
        let span = Span::new(start_span.file_id, start_span.start, body.span.end);
        StaticMethodDecl {
            name,
            attrs,
            params,
            return_type,
            body,
            span,
        }
    }

    fn parse_cstruct_decl(&mut self, c_attrs: Vec<CAttributeNode>) -> CstructDecl {
        let start_span = self.curr_span();
        self.bump(); // consume 'cstruct'
        let name = self.parse_ident_node();
        self.expect_token(&TokenKind::LBrace, "'{' to begin cstruct body");

        let mut fields = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RBrace)) {
            if self.check_kind(|k| matches!(k, TokenKind::Fun)) {
                self.error(
                    ErrorCode::E0211,
                    "method declaration is forbidden inside cstruct",
                    self.curr_span(),
                );
                self.bump();
                self.parse_method_decl(Vec::new());
                continue;
            }
            let attrs = self.parse_attributes();
            fields.push(self.parse_field_def(attrs));
            self.eat_kind(|k| matches!(k, TokenKind::Comma | TokenKind::Semicolon));
        }

        self.expect_token(&TokenKind::RBrace, "'}' to close cstruct body");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        CstructDecl {
            name,
            c_attrs,
            fields,
            span,
        }
    }

    fn parse_field_def(&mut self, attrs: Vec<AttributeNode>) -> FieldDef {
        let start_span = self.curr_span();
        let name = self.parse_ident_node();
        self.expect_token(&TokenKind::Colon, "':' for field type");
        let ty = self.parse_type();
        let default_val = if self.eat_kind(|k| matches!(k, TokenKind::Eq)) {
            Some(self.parse_expr())
        } else {
            None
        };
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        FieldDef {
            name,
            attrs,
            ty,
            default_val,
            span,
        }
    }

    fn parse_interface_decl(&mut self) -> InterfaceDecl {
        let start_span = self.curr_span();
        self.bump(); // consume 'interface'
        let name = self.parse_ident_node();
        let generics = self.parse_generic_params();
        self.expect_token(&TokenKind::LBrace, "'{' to begin interface body");

        let mut methods = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RBrace)) {
            if self.check_kind(|k| matches!(k, TokenKind::Cfun)) {
                self.error(
                    ErrorCode::E0209,
                    "cfun declaration is forbidden inside interface",
                    self.curr_span(),
                );
                self.parse_cfun_decl(Vec::new(), Vec::new());
                continue;
            }

            let attrs = self.parse_attributes();
            self.expect_token(&TokenKind::Fun, "'fun' in interface method");
            let m_span = self.prev_span();
            let m_name = self.parse_ident_node();
            self.expect_token(&TokenKind::LParen, "'(' for method parameters");
            let params = self.parse_param_list();
            self.expect_token(&TokenKind::RParen, "')' to close method parameters");

            let return_type = if self.eat_kind(|k| matches!(k, TokenKind::Arrow)) {
                Some(self.parse_type())
            } else {
                None
            };

            let default_body = if self.check_kind(|k| matches!(k, TokenKind::LBrace)) {
                Some(self.parse_block())
            } else {
                self.eat_kind(|k| matches!(k, TokenKind::Semicolon));
                None
            };

            let span = Span::new(m_span.file_id, m_span.start, self.prev_span().end);
            methods.push(InterfaceMethodDef {
                name: m_name,
                attrs,
                params,
                return_type,
                default_body,
                span,
            });
        }

        self.expect_token(&TokenKind::RBrace, "'}' to close interface body");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        InterfaceDecl {
            name,
            generics,
            methods,
            span,
        }
    }

    fn parse_enum_decl(&mut self) -> EnumDecl {
        let start_span = self.curr_span();
        self.bump(); // consume 'enum'
        let name = self.parse_ident_node();
        self.expect_token(&TokenKind::LBrace, "'{' to begin enum body");

        let mut variants = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RBrace)) {
            let v_name = self.parse_ident_node();
            if self.eat_kind(|k| matches!(k, TokenKind::Eq)) {
                let val = self.parse_expr();
                variants.push(EnumVariant::WithValue(v_name, val));
            } else if self.eat_kind(|k| matches!(k, TokenKind::LParen)) {
                let mut fields = Vec::new();
                while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RParen)) {
                    fields.push(self.parse_field_def(Vec::new()));
                    if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                        break;
                    }
                }
                self.expect_token(&TokenKind::RParen, "')' to close enum variant payload");
                variants.push(EnumVariant::Adt(v_name, fields));
            } else {
                variants.push(EnumVariant::Simple(v_name));
            }

            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }

        self.expect_token(&TokenKind::RBrace, "'}' to close enum body");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        EnumDecl {
            name,
            variants,
            span,
        }
    }

    fn parse_type_alias_decl(&mut self) -> TypeAliasDecl {
        let start_span = self.curr_span();
        self.bump(); // consume 'type'
        let name = self.parse_ident_node();
        let generics = self.parse_generic_params();
        self.expect_token(&TokenKind::Eq, "'=' in type alias declaration");
        let target = self.parse_type();
        let span = Span::new(start_span.file_id, start_span.start, target.span.end);
        TypeAliasDecl {
            name,
            generics,
            target,
            span,
        }
    }

    fn parse_static_decl(&mut self) -> StaticDecl {
        let start_span = self.curr_span();
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
        StaticDecl {
            name,
            ty,
            init,
            span,
        }
    }

    fn parse_externc_block(&mut self) -> ExterncBlock {
        let start_span = self.curr_span();
        self.bump(); // consume 'externc'

        let header = if let TokenKind::StringLiteral(ref s) = self.peek().kind {
            let h = s.clone();
            self.bump();
            Some(h)
        } else {
            None
        };

        self.expect_token(&TokenKind::LBrace, "'{' to begin externc block");
        let mut items = Vec::new();

        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RBrace)) {
            if self.eat_kind(|k| matches!(k, TokenKind::Semicolon)) {
                continue;
            }
            let attrs = self.parse_attributes();
            let c_attrs = self.parse_c_attributes();

            if self.check_kind(|k| matches!(k, TokenKind::Cfun)) {
                items.push(ExterncItem::Cfun(self.parse_cfun_decl(attrs, c_attrs)));
            } else if self.check_kind(|k| matches!(k, TokenKind::Cstruct)) {
                items.push(ExterncItem::Cstruct(self.parse_cstruct_decl(c_attrs)));
            } else {
                self.error(
                    ErrorCode::E0201,
                    "only 'cfun' and 'cstruct' are allowed inside externc block",
                    self.curr_span(),
                );
                self.bump();
            }
        }

        self.expect_token(&TokenKind::RBrace, "'}' to close externc block");
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        ExterncBlock {
            header,
            items,
            span,
        }
    }

    fn parse_load_decl(&mut self) -> ImportDecl {
        let start_span = self.curr_span();
        self.bump(); // consume 'load'

        let path = self.parse_import_path();
        let symbols = if self.eat_kind(|k| matches!(k, TokenKind::Colon)) {
            Some(self.parse_import_symbols_clause())
        } else {
            let sym_span = path.span;
            Some(ImportSymbolsClause {
                items: vec![ImportSymbolItem {
                    pattern: ImportPattern::All,
                    span: sym_span,
                }],
                aliases: Vec::new(),
                span: sym_span,
            })
        };

        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        ImportDecl {
            path,
            symbols,
            span,
        }
    }

    fn parse_leading_dots(&mut self) -> usize {
        let mut leading_dots = 0;
        while let Some(dots) = match self.peek().kind {
            TokenKind::Dot => Some(1),
            TokenKind::DotDot => Some(2),
            TokenKind::Ellipsis => Some(3),
            _ => None,
        } {
            leading_dots += dots;
            self.bump();
        }
        leading_dots
    }

    fn parse_import_pattern(&mut self) -> ImportPattern {
        if self.eat_kind(|k| matches!(k, TokenKind::Star)) {
            if self.check_kind(|k| matches!(k, TokenKind::Ident(_))) {
                let id = self.parse_ident_node();
                ImportPattern::Suffix(id)
            } else {
                ImportPattern::All
            }
        } else {
            let id = self.parse_ident_node();
            if self.eat_kind(|k| matches!(k, TokenKind::Star)) {
                ImportPattern::Prefix(id)
            } else {
                ImportPattern::Exact(id)
            }
        }
    }

    fn parse_import_path_segment(&mut self) -> ImportPathSegment {
        let start_span = self.curr_span();
        let pattern = self.parse_import_pattern();
        let force_folder = self.eat_kind(|k| matches!(k, TokenKind::Bang));
        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        ImportPathSegment {
            pattern,
            force_folder,
            span,
        }
    }

    fn parse_import_path(&mut self) -> ImportPath {
        let start_span = self.curr_span();
        let leading_dots = self.parse_leading_dots();
        let prefix = if leading_dots > 0 {
            PathPrefix::Relative(leading_dots - 1)
        } else {
            let first_id = self.parse_ident_node();
            if first_id.as_str() == "root" {
                PathPrefix::Root
            } else {
                PathPrefix::Package(first_id)
            }
        };

        let mut segments = Vec::new();
        if matches!(prefix, PathPrefix::Relative(_))
            && self.check_kind(|k| matches!(k, TokenKind::Ident(_) | TokenKind::Star))
        {
            segments.push(self.parse_import_path_segment());
        }
        while self.eat_kind(|k| matches!(k, TokenKind::Dot)) {
            segments.push(self.parse_import_path_segment());
        }

        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        ImportPath {
            prefix,
            segments,
            span,
        }
    }

    fn parse_import_symbols_clause(&mut self) -> ImportSymbolsClause {
        let start_span = self.curr_span();
        let mut items = Vec::new();

        while !self.is_eof() {
            let sym_start = self.curr_span();
            let pattern = self.parse_import_pattern();
            let sym_span = Span::new(sym_start.file_id, sym_start.start, self.prev_span().end);
            items.push(ImportSymbolItem {
                pattern,
                span: sym_span,
            });
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }

        let mut aliases = Vec::new();
        if self.eat_kind(|k| matches!(k, TokenKind::As)) {
            while !self.is_eof() {
                aliases.push(self.parse_ident_node());
                if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                    break;
                }
            }

            self.validate_import_aliases(&items, &aliases, start_span);
        }

        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        ImportSymbolsClause {
            items,
            aliases,
            span,
        }
    }

    fn validate_import_aliases(
        &mut self,
        items: &[ImportSymbolItem],
        aliases: &[Ident],
        span: Span,
    ) {
        let has_wildcard = items.iter().any(|it| it.pattern.is_wildcard());
        if has_wildcard {
            self.error(
                ErrorCode::E0304,
                "cannot use 'as' aliases with wildcard or pattern symbol imports",
                span,
            );
        } else if items.len() != aliases.len() {
            self.error(
                ErrorCode::E0304,
                format!(
                    "number of aliases ({}) must match number of symbols ({})",
                    aliases.len(),
                    items.len()
                ),
                span,
            );
        }
    }

    fn parse_alias_decl(&mut self) -> AliasDecl {
        let start_span = self.curr_span();
        self.bump(); // consume 'alias'

        let first = self.parse_ident_node();
        let mut namespace = vec![first];
        while self.eat_kind(|k| matches!(k, TokenKind::DoubleColon)) {
            namespace.push(self.parse_ident_node());
        }

        let kind = if self.eat_kind(|k| matches!(k, TokenKind::Hash)) {
            let symbol = self.parse_ident_node();
            AliasKind::Symbol {
                namespace,
                symbol,
            }
        } else if namespace.len() == 1 {
            AliasKind::Direct(namespace.remove(0))
        } else {
            AliasKind::Namespace(namespace)
        };

        self.expect_token(&TokenKind::As, "'as' in alias declaration");
        let alias = self.parse_ident_node();
        let span = Span::new(start_span.file_id, start_span.start, alias.span.end);
        AliasDecl {
            kind,
            alias,
            span,
        }
    }

    fn parse_macro_def_decl(&mut self) -> MacroDefDecl {
        let start_span = self.curr_span();
        self.bump(); // consume 'define'
        let name = self.parse_ident_node();
        self.eat_kind(|k| matches!(k, TokenKind::Bang)); // optional '!' in define name!

        self.expect_token(&TokenKind::LParen, "'(' for macro parameters");
        let mut params = Vec::new();
        while !self.is_eof() && !self.check_kind(|k| matches!(k, TokenKind::RParen)) {
            let p_span = self.curr_span();
            let p_name = self.parse_ident_node();
            let kind = if self.eat_kind(|k| matches!(k, TokenKind::Colon)) {
                let k_tok = self.parse_ident_node();
                match k_tok.name.as_str() {
                    "ident" => MacroParamKind::Ident,
                    "ty" => MacroParamKind::Ty,
                    "block" => MacroParamKind::Block,
                    "variadic" => MacroParamKind::Variadic,
                    _ => MacroParamKind::Expr,
                }
            } else {
                MacroParamKind::Expr
            };
            params.push(MacroParam {
                name: p_name,
                kind,
                span: Span::new(p_span.file_id, p_span.start, self.prev_span().end),
            });
            if !self.eat_kind(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }
        self.expect_token(&TokenKind::RParen, "')' to close macro parameters");

        let body = if self.eat_kind(|k| matches!(k, TokenKind::FatArrow)) {
            MacroBody::Expr(Box::new(self.parse_expr()))
        } else {
            MacroBody::Block(self.parse_block())
        };

        let span = Span::new(start_span.file_id, start_span.start, self.prev_span().end);
        MacroDefDecl {
            name,
            params,
            body,
            span,
        }
    }
}
