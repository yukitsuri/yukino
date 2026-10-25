#![deny(unsafe_code)]

use crate::ident::Ident;
use yukic_span::Span;

#[derive(Clone, Debug, PartialEq)]
pub struct GenericParam {
    pub name: Ident,
    pub bounds: Vec<TypeNode>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeNode {
    pub kind: TypeNodeKind,
    pub span: Span,
}

impl TypeNode {
    pub fn new(kind: TypeNodeKind, span: Span) -> Self { Self { kind, span } }
}

impl std::fmt::Display for TypeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            TypeNodeKind::Ident(id) => write!(f, "{}", id.name),
            TypeNodeKind::Path(path) => {
                let parts: Vec<&str> = path.iter().map(|i| i.name.as_str()).collect();
                write!(f, "{}", parts.join("::"))
            }
            TypeNodeKind::Pointer(inner) => write!(f, "ptr<{inner}>"),
            TypeNodeKind::Ref(inner) => write!(f, "ref {inner}"),
            TypeNodeKind::Array(inner, None) => write!(f, "{inner}[]"),
            TypeNodeKind::Array(inner, Some(sz)) => write!(f, "{inner}[{sz}]"),
            TypeNodeKind::Map(k, v) => write!(f, "map<{k}, {v}>"),
            TypeNodeKind::Tuple(elems) => {
                let inner: Vec<String> = elems.iter().map(|e| e.to_string()).collect();
                write!(f, "({})", inner.join(", "))
            }
            TypeNodeKind::Fun(params, ret) => {
                let p: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "fun({}) -> {ret}", p.join(", "))
            }
            TypeNodeKind::CFun(params, ret) => {
                let p: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "cfun({}) -> {ret}", p.join(", "))
            }
            TypeNodeKind::Optional(inner) => write!(f, "{inner}?"),
            TypeNodeKind::Result(ok, err) => write!(f, "result<{ok}, {err}>"),
            TypeNodeKind::Union(members) => {
                let m: Vec<String> = members.iter().map(|m| m.to_string()).collect();
                write!(f, "{}", m.join(" | "))
            }
            TypeNodeKind::Generic(name, args) => {
                let a: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                write!(f, "{}<{}>", name.name, a.join(", "))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeNodeKind {
    Ident(Ident),
    Path(Vec<Ident>),
    Pointer(Box<TypeNode>),
    Ref(Box<TypeNode>),
    Array(Box<TypeNode>, Option<usize>),
    Map(Box<TypeNode>, Box<TypeNode>),
    Tuple(Vec<TypeNode>),
    Fun(Vec<TypeNode>, Box<TypeNode>),
    CFun(Vec<TypeNode>, Box<TypeNode>),
    Optional(Box<TypeNode>),
    Result(Box<TypeNode>, Box<TypeNode>),
    Union(Vec<TypeNode>),
    Generic(Ident, Vec<TypeNode>),
}
