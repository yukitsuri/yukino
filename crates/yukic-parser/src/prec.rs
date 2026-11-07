#![deny(unsafe_code)]

use yukic_ast::BinaryOp;
use yukic_lexer::TokenKind;

pub const PREFIX_BP: u8 = 32;
pub const POSTFIX_BP: u8 = 34;

pub fn infix_binding_power(kind: &TokenKind<'_>) -> Option<(u8, u8)> {
    match kind {
        TokenKind::Question => Some((3, 2)),
        TokenKind::PipeOp => Some((4, 5)),
        TokenKind::QuestionQuestion => Some((6, 7)),
        TokenKind::PipePipe => Some((8, 9)),
        TokenKind::AmpAmp => Some((10, 11)),
        TokenKind::EqEq | TokenKind::BangEq | TokenKind::Is => Some((12, 13)),
        TokenKind::Lt | TokenKind::LtEq | TokenKind::Gt | TokenKind::GtEq => Some((14, 15)),
        TokenKind::Pipe => Some((16, 17)),
        TokenKind::Caret => Some((18, 19)),
        TokenKind::Amp => Some((20, 21)),
        TokenKind::DotDot | TokenKind::DotDotEq => Some((22, 23)),
        TokenKind::LtLt | TokenKind::GtGt => Some((24, 25)),
        TokenKind::Plus | TokenKind::Minus => Some((26, 27)),
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Some((28, 29)),
        TokenKind::StarStar => Some((31, 30)),
        _ => None,
    }
}

pub fn token_to_binary_op(kind: &TokenKind<'_>) -> Option<BinaryOp> {
    match kind {
        TokenKind::Plus => Some(BinaryOp::Add),
        TokenKind::Minus => Some(BinaryOp::Sub),
        TokenKind::Star => Some(BinaryOp::Mul),
        TokenKind::Slash => Some(BinaryOp::Div),
        TokenKind::Percent => Some(BinaryOp::Rem),
        TokenKind::StarStar => Some(BinaryOp::Pow),
        TokenKind::EqEq => Some(BinaryOp::Eq),
        TokenKind::BangEq => Some(BinaryOp::NotEq),
        TokenKind::Lt => Some(BinaryOp::Lt),
        TokenKind::LtEq => Some(BinaryOp::LtEq),
        TokenKind::Gt => Some(BinaryOp::Gt),
        TokenKind::GtEq => Some(BinaryOp::GtEq),
        TokenKind::AmpAmp => Some(BinaryOp::And),
        TokenKind::PipePipe => Some(BinaryOp::Or),
        TokenKind::Amp => Some(BinaryOp::BitAnd),
        TokenKind::Pipe => Some(BinaryOp::BitOr),
        TokenKind::Caret => Some(BinaryOp::BitXor),
        TokenKind::LtLt => Some(BinaryOp::Shl),
        TokenKind::GtGt => Some(BinaryOp::Shr),
        TokenKind::QuestionQuestion => Some(BinaryOp::NullCoalesce),
        TokenKind::DotDot => Some(BinaryOp::Range),
        TokenKind::DotDotEq => Some(BinaryOp::RangeInclusive),
        _ => None,
    }
}
