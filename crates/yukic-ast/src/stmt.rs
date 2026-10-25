#![deny(unsafe_code)]

use crate::expr::{Expr, MacroCallNode};
use crate::ident::Ident;
use crate::item::{MacroDefDecl, StaticDecl};
use crate::types::TypeNode;
use yukic_span::Span;

#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

impl Stmt {
    pub fn new(kind: StmtKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    Set {
        target: BindingTarget,
        ty: Option<TypeNode>,
        init: Expr,
    },
    Rewrite {
        target: LValue,
        op: AssignOp,
        value: Expr,
    },
    Static(StaticDecl),
    Expr(Expr),
    MacroDef(MacroDefDecl),
    MacroCall(MacroCallNode),
    Return(Option<Expr>),
    Recur(Vec<Expr>),
    Break(Option<Ident>),
    Continue(Option<Ident>),
    Defer(Block),
    Space,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BindingTarget {
    Ident(Ident),
    Tuple(Vec<BindingTarget>),
    Array(Vec<ArrayDestructElement>),
    Map(Vec<MapDestructField>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayDestructElement {
    pub target: BindingTarget,
    pub is_rest: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapDestructField {
    pub field: Ident,
    pub alias: Option<BindingTarget>,
    pub default_val: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LValue {
    Ident(Ident),
    Field(Box<Expr>, Ident),
    Index(Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignOp {
    Eq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    StarStarEq,
    AmpEq,
    PipeEq,
    CaretEq,
    ShlEq,
    ShrEq,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

impl Block {
    pub fn new(stmts: Vec<Stmt>, span: Span) -> Self {
        Self { stmts, span }
    }
}
