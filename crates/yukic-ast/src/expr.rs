#![deny(unsafe_code)]

use crate::ident::Ident;
use crate::item::FunDecl;
use crate::stmt::Block;
use crate::types::TypeNode;
use yukic_span::Span;

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Literal(LiteralValue),
    Ident(Ident),
    This,
    Super,
    Path(Vec<Ident>),
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    MacroCall(MacroCallNode),
    MethodCall(Box<Expr>, Ident, Vec<Expr>),
    QualifiedMethodCall(Box<Expr>, Ident, Ident, Vec<Expr>),
    SafeCall(Box<Expr>, Ident, Vec<Expr>),
    FieldAccess(Box<Expr>, Ident),
    IndexAccess(Box<Expr>, Box<Expr>),
    SliceAccess(Box<Expr>, Option<Box<Expr>>, Option<Box<Expr>>, bool),
    StructInit(Ident, Vec<FieldInit>),
    ClassInit(Box<Expr>, Vec<Expr>),
    ArrayInit(Vec<Expr>),
    ArrayAlloc(Box<TypeNode>, usize),
    MapInit(Vec<(Expr, Expr)>),
    TupleInit(Vec<Expr>),
    Block(Block),
    RegionBlock(Block),
    If {
        cond: Box<Expr>,
        then_branch: Block,
        elif_branches: Vec<(Expr, Block)>,
        else_branch: Option<Block>,
    },
    Switch {
        target: Box<Expr>,
        arms: Vec<SwitchArm>,
    },
    While {
        label: Option<Ident>,
        cond: Box<Expr>,
        body: Block,
    },
    For {
        label: Option<Ident>,
        index_var: Option<Ident>,
        item_var: Ident,
        iter: Box<Expr>,
        body: Block,
    },
    Loop {
        label: Option<Ident>,
        body: Block,
    },
    Sus {
        body: Block,
        catches: Vec<CatchClause>,
        finally_branch: Option<Block>,
    },
    Pipe(Box<Expr>, Box<Expr>),
    Lambda(Vec<Ident>, Box<Expr>),
    AnonymousFun(Box<FunDecl>),
    PtrCast(Box<Expr>, TypeNode),
    PtrOf(Ident),
    Ref {
        mutable: bool,
        expr: Box<Expr>,
    },
    Is(Box<Expr>, TypeNode),
    Spread(Box<Expr>),
    NamedArg(Ident, Box<Expr>),
    InterpolatedString(Vec<StringPart>),
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StringPart {
    Lit(String),
    Expr(Expr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    NullCoalesce,
    Range,
    RangeInclusive,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SwitchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

impl Pattern {
    pub fn new(kind: PatternKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PatternKind {
    Wildcard,
    Literal(LiteralValue),
    Ident(Ident),
    Tuple(Vec<Pattern>),
    Struct(Ident, Vec<(Ident, Pattern)>),
    Variant(Option<Ident>, Ident, Vec<Pattern>),
    Range(Box<Expr>, Box<Expr>, bool),
    Or(Vec<Pattern>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CatchClause {
    pub error_var: Option<Ident>,
    pub error_type: Option<TypeNode>,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldInit {
    pub name: Ident,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LiteralValue {
    Int(i64),
    UInt(u64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    Bytes(Vec<u8>),
    Null,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MacroCallNode {
    pub path: Vec<Ident>,
    pub delimiter: MacroDelimiter,
    pub args: Vec<MacroArg>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MacroDelimiter {
    Paren,
    Brace,
    Bracket,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MacroArg {
    Expr(Expr),
    Block(Block),
    Tokens(Vec<TokenTree>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenTree {
    Token(String, Span),
    Group(MacroDelimiter, Vec<TokenTree>),
}
