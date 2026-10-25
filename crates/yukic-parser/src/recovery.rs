#![deny(unsafe_code)]

use crate::stream::TokenStream;
use yukic_lexer::TokenKind;

impl TokenStream<'_> {
    pub fn synchronize_stmt(&mut self) {
        while !self.is_eof() {
            if self.eat_kind(|k| matches!(k, TokenKind::Semicolon)) { return; }
            if self.check_kind(|k| matches!(k, TokenKind::RBrace)) { return; }
            if self.check_kind(|k| {
                matches!(
                    k,
                    TokenKind::Set
                        | TokenKind::Rewrite
                        | TokenKind::Static
                        | TokenKind::Return
                        | TokenKind::Recur
                        | TokenKind::Break
                        | TokenKind::Continue
                        | TokenKind::Defer
                        | TokenKind::Space
                        | TokenKind::If
                        | TokenKind::While
                        | TokenKind::For
                        | TokenKind::Loop
                        | TokenKind::Sus
                        | TokenKind::Region
                )
            }) 

            { return; }


            self.bump();
        }
    }

    pub fn synchronize_item(&mut self) {
        while !self.is_eof() {
            if self.check_kind(|k| {
                matches!(
                    k,
                    TokenKind::Fun
                        | TokenKind::Cfun
                        | TokenKind::Class
                        | TokenKind::Struct
                        | TokenKind::Cstruct
                        | TokenKind::Interface
                        | TokenKind::Enum
                        | TokenKind::Type
                        | TokenKind::Static
                        | TokenKind::Externc
                        | TokenKind::Load
                        | TokenKind::Alias
                        | TokenKind::Define
                        | TokenKind::Public
                        | TokenKind::Locked
                        | TokenKind::AttrStart
                        | TokenKind::ModuleAttrStart
                )
            }) 


            { return; }
            
            self.bump();
        }
    }
}
