#![deny(unsafe_code)]

use yukic_span::Span;

// int rad

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Radix {
    Binary = 2,
    Octal = 8,
    Decimal = 10,
    Hex = 16,
}

impl Radix {
    pub fn as_u32(self) -> u32 {
        match self {
            Self::Binary => 2,
            Self::Octal => 8,
            Self::Decimal => 10,
            Self::Hex => 16,
        }
    }
}

// int
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntSuffix {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
}

// float
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatSuffix {
    F32,
    F64,
}


// kws
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind<'a> {
    // ids and literals
    Ident(&'a str),
    IntLiteral {
        value: u64,
        suffix: Option<IntSuffix>,
        radix: Radix,
    },
    FloatLiteral {
        value: f64,
        suffix: Option<FloatSuffix>,
    },
    StringLiteral(String),
    CharLiteral(char),
    BoolLiteral(bool),
    NullLiteral,

    // str interpolation parts
    StringPart(String),
    InterpolationStart,
    InterpolationEnd,

    // decl kw
    Set,
    Rewrite,
    Static,
    Fun,
    Cfun,
    Define,
    Class,
    Struct,
    Cstruct,
    Interface,
    Enum,
    Type,
    Public,
    Locked,
    Operator,
    Externc,
    Get,

    // control flow kw
    If,
    Elif,
    Else,
    Switch,
    While,
    For,
    In,
    Loop,
    Break,
    Continue,
    Return,
    Recur,
    Region,
    Sus,
    Catch,
    Finally,
    Panic,
    Defer,
    Space,

    // obj and memory kw
    This,
    Super,
    Init,
    Ref,
    With,
    Extends,
    Is,

    // module kw
    Load,
    As,
    Alias,

    // arithmetic ops
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    StarStar,

    // assignment and compound assignment
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
    LtLtEq,
    GtGtEq,

    // comparison ops
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,

    // logical and bitwise ops
    AmpAmp,
    PipePipe,
    Bang,
    Amp,
    Pipe,
    Caret,
    Tilde,
    LtLt,
    GtGt,

    // special nav & punctuation
    PipeOp,
    QuestionDot,
    QuestionQuestion,
    Question,
    Arrow,
    FatArrow,
    DoubleColon,
    Colon,
    Hash,
    Semicolon,
    Comma,
    Dot,
    DotDot,
    DotDotEq,
    Ellipsis,

    // attribute start markers
    AttrStart,
    ModuleAttrStart,

    // delimiters
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,


    
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token<'a> { pub kind: TokenKind<'a>, pub span: Span, }
impl<'a> Token<'a> {
    pub fn new(kind: TokenKind<'a>, span: Span) -> Self { Self { kind, span } }
    pub fn is_eof(&self) -> bool { matches!(self.kind, TokenKind::Eof) }
}
