#![deny(unsafe_code)]

use crate::token::TokenKind;

pub fn lookup_keyword(ident: &str) -> Option<TokenKind<'static>> {
    lookup_decl_keyword(ident)
        .or_else(|| lookup_control_keyword(ident))
        .or_else(|| lookup_other_keyword(ident))
}

// decl kw
fn lookup_decl_keyword(ident: &str) -> Option<TokenKind<'static>> {
    match ident {
        "set" => Some(TokenKind::Set),
        "rewrite" => Some(TokenKind::Rewrite),
        "static" => Some(TokenKind::Static),
        "fun" => Some(TokenKind::Fun),
        "cfun" => Some(TokenKind::Cfun),
        "define" => Some(TokenKind::Define),
        "class" => Some(TokenKind::Class),
        "struct" => Some(TokenKind::Struct),
        "cstruct" => Some(TokenKind::Cstruct),
        "interface" => Some(TokenKind::Interface),
        "enum" => Some(TokenKind::Enum),
        "type" => Some(TokenKind::Type),
        "public" => Some(TokenKind::Public),
        "locked" => Some(TokenKind::Locked),
        "operator" => Some(TokenKind::Operator),
        "externc" => Some(TokenKind::Externc),
        "get" => Some(TokenKind::Get),
        _ => None,
    }
}

// cf kw
fn lookup_control_keyword(ident: &str) -> Option<TokenKind<'static>> {
    match ident {
        "if" => Some(TokenKind::If),
        "elif" => Some(TokenKind::Elif),
        "else" => Some(TokenKind::Else),
        "switch" => Some(TokenKind::Switch),
        "while" => Some(TokenKind::While),
        "for" => Some(TokenKind::For),
        "in" => Some(TokenKind::In),
        "loop" => Some(TokenKind::Loop),
        "break" => Some(TokenKind::Break),
        "continue" => Some(TokenKind::Continue),
        "return" => Some(TokenKind::Return),
        "recur" => Some(TokenKind::Recur),
        "region" => Some(TokenKind::Region),
        "sus" => Some(TokenKind::Sus),
        "catch" => Some(TokenKind::Catch),
        "finally" => Some(TokenKind::Finally),
        "panic" => Some(TokenKind::Panic),
        "defer" => Some(TokenKind::Defer),
        "space" => Some(TokenKind::Space),
        _ => None,
    }
}

// misc
fn lookup_other_keyword(ident: &str) -> Option<TokenKind<'static>> {
    match ident {
        "this" => Some(TokenKind::This),
        "super" => Some(TokenKind::Super),
        "init" => Some(TokenKind::Init),
        "ref" => Some(TokenKind::Ref),
        "with" => Some(TokenKind::With),
        "extends" => Some(TokenKind::Extends),
        "is" => Some(TokenKind::Is),
        "load" => Some(TokenKind::Load),
        "as" => Some(TokenKind::As),
        "alias" => Some(TokenKind::Alias),
        "true" => Some(TokenKind::BoolLiteral(true)),
        "false" => Some(TokenKind::BoolLiteral(false)),
        "null" => Some(TokenKind::NullLiteral),
        _ => None,
    }
}
