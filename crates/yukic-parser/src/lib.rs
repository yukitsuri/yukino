#![deny(unsafe_code)]

pub mod expr;
pub mod item;
pub mod pattern;
pub mod prec;
pub mod recovery;
pub mod stmt;
pub mod stream;
pub mod types;

pub use stream::TokenStream;
use yukic_ast::{Expr, Module, Stmt};
use yukic_diagnostics::Diagnostic;
use yukic_lexer::Token;
use yukic_span::FileId;

pub struct Parser<'a> {
    stream: TokenStream<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token<'a>>) -> Self { Self::with_file_id(tokens, FileId(0)) }

    pub fn with_file_id(tokens: Vec<Token<'a>>, file_id: FileId) -> Self {
        Self { stream: TokenStream::new(tokens, file_id), }
    }

    pub fn parse_module(&mut self) -> (Module, Vec<Diagnostic>) {
        let module = self.stream.parse_module();
        let diags = std::mem::take(&mut self.stream.diagnostics);
        (module, diags)
    }

    pub fn parse_expr(&mut self) -> (Expr, Vec<Diagnostic>) {
        let expr = self.stream.parse_expr();
        let diags = std::mem::take(&mut self.stream.diagnostics);
        (expr, diags)
    }

    pub fn parse_stmt(&mut self) -> (Stmt, Vec<Diagnostic>) {
        let stmt = self.stream.parse_stmt();
        let diags = std::mem::take(&mut self.stream.diagnostics);
        (stmt, diags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_ast::*;
    use yukic_lexer::Lexer;

    fn parse_code(code: &str) -> (Module, Vec<Diagnostic>) {
        let lexer = Lexer::new(FileId(0), code);
        let (tokens, mut diags) = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let (module, mut parser_diags) = parser.parse_module();
        diags.append(&mut parser_diags);
        (module, diags)
    }

    fn parse_single_expr(code: &str) -> (Expr, Vec<Diagnostic>) {
        let lexer = Lexer::new(FileId(0), code);
        let (tokens, mut diags) = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let (expr, mut parser_diags) = parser.parse_expr();
        diags.append(&mut parser_diags);
        (expr, diags)
    }

    #[test]
    fn test_empty_source() {
        let (module, diags) = parse_code("");
        assert!(module.items.is_empty());
        assert!(diags.is_empty());
    }

    #[test]
    fn test_arithmetic_precedence() {
        let (expr, diags) = parse_single_expr("1 + 2 * 3 ** 4");
        assert!(diags.is_empty());
        // Should parse as: 1 + (2 * (3 ** 4))
        match expr.kind {
            ExprKind::Binary(BinaryOp::Add, lhs, rhs) => {
                assert_eq!(lhs.kind, ExprKind::Literal(LiteralValue::Int(1)));
                match rhs.kind {
                    ExprKind::Binary(BinaryOp::Mul, m_lhs, m_rhs) => {
                        assert_eq!(m_lhs.kind, ExprKind::Literal(LiteralValue::Int(2)));
                        match m_rhs.kind {
                            ExprKind::Binary(BinaryOp::Pow, p_lhs, p_rhs) => {
                                assert_eq!(p_lhs.kind, ExprKind::Literal(LiteralValue::Int(3)));
                                assert_eq!(p_rhs.kind, ExprKind::Literal(LiteralValue::Int(4)));
                            }
                            other => panic!("expected Pow, found {other:?}"),
                        }
                    }
                    other => panic!("expected Mul, found {other:?}"),
                }
            }
            other => panic!("expected Add, found {other:?}"),
        }
    }
}
