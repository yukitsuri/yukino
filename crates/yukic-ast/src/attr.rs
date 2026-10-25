#![deny(unsafe_code)]

use crate::expr::Expr;
use crate::ident::Ident;
use yukic_span::Span;

#[derive(Clone, Debug, PartialEq)]
pub struct AttributePath {
    pub segments: Vec<Ident>,
    pub span: Span,
}

impl AttributePath {
    pub fn new(segments: Vec<Ident>, span: Span) -> Self {
        Self { segments, span }
    }

    pub fn from_ident(ident: Ident) -> Self {
        let span = ident.span;
        Self {
            segments: vec![ident],
            span,
        }
    }

    pub fn as_str(&self) -> String {
        self.segments
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>()
            .join("::")
    }

    pub fn is(&self, name: &str) -> bool {
        if !name.contains("::") {
            self.segments.len() == 1 && self.segments[0].as_str() == name
        } else {
            self.as_str() == name
        }
    }

    pub fn starts_with(&self, prefix: &str) -> bool {
        self.segments.first().map(|id| id.as_str()) == Some(prefix)
    }

    pub fn first(&self) -> Option<&Ident> {
        self.segments.first()
    }

    pub fn last(&self) -> Option<&Ident> {
        self.segments.last()
    }
}

impl From<Ident> for AttributePath {
    fn from(ident: Ident) -> Self {
        AttributePath::from_ident(ident)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttributeNode {
    pub path: AttributePath,
    pub negated: bool,
    pub args: AttributeArgs,
    pub span: Span,
}

impl AttributeNode {
    pub fn new(path: impl Into<AttributePath>, args: AttributeArgs, span: Span) -> Self {
        Self {
            path: path.into(),
            negated: false,
            args,
            span,
        }
    }

    pub fn new_negated(path: impl Into<AttributePath>, args: AttributeArgs, span: Span) -> Self {
        Self {
            path: path.into(),
            negated: true,
            args,
            span,
        }
    }

    pub fn is(&self, name: &str) -> bool {
        self.path.is(name)
    }

    pub fn path_str(&self) -> String {
        self.path.as_str()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AttributeArgs {
    None,
    Positional(Vec<Expr>),
    Named(Vec<(Ident, Expr)>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CAttributeNode {
    pub name: Ident,
    pub arg: Option<String>,
    pub span: Span,
}

impl CAttributeNode {
    pub fn new(name: Ident, arg: Option<String>, span: Span) -> Self {
        Self { name, arg, span }
    }
}
