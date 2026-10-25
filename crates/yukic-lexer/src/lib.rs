#![deny(unsafe_code)]

pub mod cursor;
pub mod lexer;
pub mod number;
pub mod span_util;
pub mod string;
pub mod token;
pub mod words;

pub use cursor::Cursor;
pub use lexer::Lexer;
pub use number::*;
pub use span_util::make_span;
pub use string::StringMode;
pub use token::{FloatSuffix, IntSuffix, Radix, Token, TokenKind};
pub use words::lookup_keyword;
use yukic_diagnostics::Diagnostic;
use yukic_span::FileId;

pub fn tokenize(file_id: FileId, source: &str) -> (Vec<Token<'_>>, Vec<Diagnostic>) {
    Lexer::new(file_id, source).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_diagnostics::ErrorCode;

    #[test]
    fn test_keywords_and_declarations() {
        let src =
            "set x = 42\nrewrite y = 10\nstatic pi = 3.14\nfun add() { return 1 }\ncfun puts()";
        let (tokens, diags) = tokenize(FileId(0), src);
        assert!(diags.is_empty());

        let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(kinds[0], TokenKind::Set);
        assert_eq!(kinds[1], TokenKind::Ident("x"));
        assert_eq!(kinds[2], TokenKind::Eq);
        assert!(matches!(kinds[3], TokenKind::IntLiteral { value: 42, .. }));
        assert_eq!(kinds[4], TokenKind::Rewrite);
        assert_eq!(kinds[5], TokenKind::Ident("y"));
        assert_eq!(kinds[6], TokenKind::Eq);
        assert!(matches!(kinds[7], TokenKind::IntLiteral { value: 10, .. }));
        assert_eq!(kinds[8], TokenKind::Static);
        assert_eq!(kinds[9], TokenKind::Ident("pi"));
        assert_eq!(kinds[10], TokenKind::Eq);
        assert!(matches!(kinds[11], TokenKind::FloatLiteral { .. }));
        assert_eq!(kinds[12], TokenKind::Fun);
        assert_eq!(kinds[13], TokenKind::Ident("add"));
        assert_eq!(kinds[14], TokenKind::LParen);
        assert_eq!(kinds[15], TokenKind::RParen);
        assert_eq!(kinds[16], TokenKind::LBrace);
        assert_eq!(kinds[17], TokenKind::Return);
        assert!(matches!(kinds[18], TokenKind::IntLiteral { value: 1, .. }));
        assert_eq!(kinds[19], TokenKind::RBrace);
        assert_eq!(kinds[20], TokenKind::Cfun);
    }

    #[test]
    fn test_number_literals_and_suffixes() {
        let src = "100 1_000_000 0xFF 0b1010 0o755 42i8 10u64 3.14f32 1e-5 2.5E+10";
        let (tokens, diags) = tokenize(FileId(0), src);
        assert!(diags.is_empty());

        assert_eq!(
            tokens[0].kind,
            TokenKind::IntLiteral {
                value: 100,
                suffix: None,
                radix: Radix::Decimal
            }
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::IntLiteral {
                value: 1_000_000,
                suffix: None,
                radix: Radix::Decimal
            }
        );
        assert_eq!(
            tokens[2].kind,
            TokenKind::IntLiteral {
                value: 255,
                suffix: None,
                radix: Radix::Hex
            }
        );
        assert_eq!(
            tokens[3].kind,
            TokenKind::IntLiteral {
                value: 10,
                suffix: None,
                radix: Radix::Binary
            }
        );
        assert_eq!(
            tokens[4].kind,
            TokenKind::IntLiteral {
                value: 493,
                suffix: None,
                radix: Radix::Octal
            }
        );
        assert_eq!(
            tokens[5].kind,
            TokenKind::IntLiteral {
                value: 42,
                suffix: Some(IntSuffix::I8),
                radix: Radix::Decimal
            }
        );
        assert_eq!(
            tokens[6].kind,
            TokenKind::IntLiteral {
                value: 10,
                suffix: Some(IntSuffix::U64),
                radix: Radix::Decimal
            }
        );
        assert!(matches!(
            tokens[7].kind,
            TokenKind::FloatLiteral {
                suffix: Some(FloatSuffix::F32),
                ..
            }
        ));
        assert!(matches!(tokens[8].kind, TokenKind::FloatLiteral { .. }));
        assert!(matches!(tokens[9].kind, TokenKind::FloatLiteral { .. }));
    }

    #[test]
    fn test_dot_vs_float_vs_range() {
        let src = "10.to_string() 10.5 0..10 0..=5";
        let (tokens, diags) = tokenize(FileId(0), src);
        assert!(diags.is_empty());

        // 10.to_string() -> 10, Dot, to_string, (, )
        assert!(matches!(
            tokens[0].kind,
            TokenKind::IntLiteral { value: 10, .. }
        ));
        assert_eq!(tokens[1].kind, TokenKind::Dot);
        assert_eq!(tokens[2].kind, TokenKind::Ident("to_string"));
        assert_eq!(tokens[3].kind, TokenKind::LParen);
        assert_eq!(tokens[4].kind, TokenKind::RParen);

        // 10.5 -> FloatLiteral
        assert!(matches!(tokens[5].kind, TokenKind::FloatLiteral { .. }));

        // 0..10 -> 0, DotDot, 10
        assert!(matches!(
            tokens[6].kind,
            TokenKind::IntLiteral { value: 0, .. }
        ));
        assert_eq!(tokens[7].kind, TokenKind::DotDot);
        assert!(matches!(
            tokens[8].kind,
            TokenKind::IntLiteral { value: 10, .. }
        ));

        // 0..=5 -> 0, DotDotEq, 5
        assert!(matches!(
            tokens[9].kind,
            TokenKind::IntLiteral { value: 0, .. }
        ));
        assert_eq!(tokens[10].kind, TokenKind::DotDotEq);
        assert!(matches!(
            tokens[11].kind,
            TokenKind::IntLiteral { value: 5, .. }
        ));
    }

    #[test]
    fn test_compound_operators_and_delimiters() {
        let src = "#![ !#[ #[ |> ?. ?? -> => :: ... **= ** += -= *= /= %= &= |= ^= <<= >>=";
        let (tokens, diags) = tokenize(FileId(0), src);
        assert!(diags.is_empty());

        let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(kinds[0], TokenKind::ModuleAttrStart);
        assert_eq!(kinds[1], TokenKind::ModuleAttrStart);
        assert_eq!(kinds[2], TokenKind::AttrStart);
        assert_eq!(kinds[3], TokenKind::PipeOp);
        assert_eq!(kinds[4], TokenKind::QuestionDot);
        assert_eq!(kinds[5], TokenKind::QuestionQuestion);
        assert_eq!(kinds[6], TokenKind::Arrow);
        assert_eq!(kinds[7], TokenKind::FatArrow);
        assert_eq!(kinds[8], TokenKind::DoubleColon);
        assert_eq!(kinds[9], TokenKind::Ellipsis);
        assert_eq!(kinds[10], TokenKind::StarStarEq);
        assert_eq!(kinds[11], TokenKind::StarStar);
        assert_eq!(kinds[12], TokenKind::PlusEq);
        assert_eq!(kinds[13], TokenKind::MinusEq);
        assert_eq!(kinds[14], TokenKind::StarEq);
        assert_eq!(kinds[15], TokenKind::SlashEq);
        assert_eq!(kinds[16], TokenKind::PercentEq);
        assert_eq!(kinds[17], TokenKind::AmpEq);
        assert_eq!(kinds[18], TokenKind::PipeEq);
        assert_eq!(kinds[19], TokenKind::CaretEq);
        assert_eq!(kinds[20], TokenKind::LtLtEq);
        assert_eq!(kinds[21], TokenKind::GtGtEq);
    }

    #[test]
    fn test_utf8_bom_is_skipped() {
        let src = "\u{feff}fun main() { space }";
        let (tokens, diags) = tokenize(FileId(0), src);
        assert!(diags.is_empty());
        assert!(matches!(tokens[0].kind, TokenKind::Fun));
    }

    #[test]
    fn test_unrecognized_character_is_reported_and_consumed() {
        // non-alphanumeric multibyte char must be consumed with E0112, not loop
        let src = "fun \u{20ac} main() { space }";
        let (tokens, diags) = tokenize(FileId(0), src);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("unrecognized character")));
        let idents: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Ident(_)))
            .collect();
        assert!(idents.iter().any(|t| matches!(t.kind, TokenKind::Ident("main"))));
    }

    #[test]
    fn test_string_literal_and_multiline() {
        let src =
            "\"hello\\nworld\" \"\\u{1F600}\" \"\\$\" \"\"\"multi\nline\nstring\"\"\" 'x' '\\n'";
        let (tokens, diags) = tokenize(FileId(0), src);
        assert!(diags.is_empty());

        assert_eq!(
            tokens[0].kind,
            TokenKind::StringLiteral("hello\nworld".to_string())
        );
        assert_eq!(tokens[1].kind, TokenKind::StringLiteral("😀".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::StringLiteral("$".to_string()));
        assert_eq!(
            tokens[3].kind,
            TokenKind::StringLiteral("multi\nline\nstring".to_string())
        );
        assert_eq!(tokens[4].kind, TokenKind::CharLiteral('x'));
        assert_eq!(tokens[5].kind, TokenKind::CharLiteral('\n'));
    }

    #[test]
    fn test_string_interpolation_simple() {
        let src = "\"Hello, ${name}!\"";
        let (tokens, diags) = tokenize(FileId(0), src);
        assert!(diags.is_empty());

        assert_eq!(tokens[0].kind, TokenKind::StringPart("Hello, ".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::InterpolationStart);
        assert_eq!(tokens[2].kind, TokenKind::Ident("name"));
        assert_eq!(tokens[3].kind, TokenKind::InterpolationEnd);
        assert_eq!(tokens[4].kind, TokenKind::StringPart("!".to_string()));
    }

    #[test]
    fn test_string_interpolation_multiple() {
        let src = "\"X: ${x}, Y: ${y}\"";
        let (tokens, diags) = tokenize(FileId(0), src);
        assert!(diags.is_empty());

        assert_eq!(tokens[0].kind, TokenKind::StringPart("X: ".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::InterpolationStart);
        assert_eq!(tokens[2].kind, TokenKind::Ident("x"));
        assert_eq!(tokens[3].kind, TokenKind::InterpolationEnd);
        assert_eq!(tokens[4].kind, TokenKind::StringPart(", Y: ".to_string()));
        assert_eq!(tokens[5].kind, TokenKind::InterpolationStart);
        assert_eq!(tokens[6].kind, TokenKind::Ident("y"));
        assert_eq!(tokens[7].kind, TokenKind::InterpolationEnd);
    }

    #[test]
    fn test_string_interpolation_nested_braces() {
        let src = "\"val: ${if a { 1 } else { 2 }}\"";
        let (tokens, diags) = tokenize(FileId(0), src);
        assert!(diags.is_empty());

        assert_eq!(tokens[0].kind, TokenKind::StringPart("val: ".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::InterpolationStart);
        assert_eq!(tokens[2].kind, TokenKind::If);
        assert_eq!(tokens[3].kind, TokenKind::Ident("a"));
        assert_eq!(tokens[4].kind, TokenKind::LBrace);
        assert!(matches!(
            tokens[5].kind,
            TokenKind::IntLiteral { value: 1, .. }
        ));
        assert_eq!(tokens[6].kind, TokenKind::RBrace);
        assert_eq!(tokens[7].kind, TokenKind::Else);
        assert_eq!(tokens[8].kind, TokenKind::LBrace);
        assert!(matches!(
            tokens[9].kind,
            TokenKind::IntLiteral { value: 2, .. }
        ));
        assert_eq!(tokens[10].kind, TokenKind::RBrace);
        assert_eq!(tokens[11].kind, TokenKind::InterpolationEnd);
        assert_eq!(tokens[12].kind, TokenKind::StringPart(String::new()));
    }

    #[test]
    fn test_comments_single_and_nested_block() {
        let src =
            "// single line\nset a = 1\n// another line comment\n/* block /* nested */ comment */ set b = 2";
        let (tokens, diags) = tokenize(FileId(0), src);
        assert!(diags.is_empty());

        assert_eq!(tokens[0].kind, TokenKind::Set);
        assert_eq!(tokens[1].kind, TokenKind::Ident("a"));
        assert_eq!(tokens[2].kind, TokenKind::Eq);
        assert!(matches!(
            tokens[3].kind,
            TokenKind::IntLiteral { value: 1, .. }
        ));
        assert_eq!(tokens[4].kind, TokenKind::Set);
        assert_eq!(tokens[5].kind, TokenKind::Ident("b"));
        assert_eq!(tokens[6].kind, TokenKind::Eq);
        assert!(matches!(
            tokens[7].kind,
            TokenKind::IntLiteral { value: 2, .. }
        ));
    }

    #[test]
    fn test_error_recovery_and_diagnostics() {
        let src = "@ \"unterminated string\n 0x \"\\q\" 'x\n /* unclosed block";
        let (_, diags) = tokenize(FileId(0), src);
        assert!(!diags.is_empty());

        let codes: Vec<_> = diags.iter().map(|d| d.code.as_str()).collect();
        assert!(codes.contains(&ErrorCode::E0112.as_str())); // unrecognized char '@'
        assert!(codes.contains(&ErrorCode::E0102.as_str())); // unterminated string
        assert!(codes.contains(&ErrorCode::E0109.as_str())); // malformed int
        assert!(codes.contains(&ErrorCode::E0108.as_str())); // invalid escape '\q'
        assert!(codes.contains(&ErrorCode::E0105.as_str())); // unterminated char 'x\n
        assert!(codes.contains(&ErrorCode::E0104.as_str())); // unterminated block comment

        let src_multi = "\"\"\"unclosed multiline";
        let (_, diags_multi) = tokenize(FileId(1), src_multi);
        let codes_multi: Vec<_> = diags_multi.iter().map(|d| d.code.as_str()).collect();
        assert!(codes_multi.contains(&ErrorCode::E0103.as_str()));
    }

    #[test]
    fn test_load_and_hash_tokens() {
        let src = "load std.io: println as ptln\nalias std::io#println as prt";
        let (tokens, diags) = tokenize(FileId(0), src);
        assert!(diags.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::Load);
        assert_eq!(tokens[4].kind, TokenKind::Colon);
        assert_eq!(tokens[6].kind, TokenKind::As);
        assert_eq!(tokens[8].kind, TokenKind::Alias);
        assert_eq!(tokens[12].kind, TokenKind::Hash);
    }
}
