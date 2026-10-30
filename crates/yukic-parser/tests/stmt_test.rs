use yukic_ast::*;
use yukic_diagnostics::{Diagnostic, WarningCode};
use yukic_lexer::Lexer;
use yukic_parser::Parser;
use yukic_span::FileId;

fn parse_stmt(code: &str) -> (Stmt, Vec<Diagnostic>) {
    let lexer = Lexer::new(FileId(0), code);
    let (tokens, mut diags) = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let (stmt, mut parser_diags) = parser.parse_stmt();
    diags.append(&mut parser_diags);
    (stmt, diags)
}

fn parse_code(code: &str) -> (Module, Vec<Diagnostic>) {
    let lexer = Lexer::new(FileId(0), code);
    let (tokens, mut diags) = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let (module, mut parser_diags) = parser.parse_module();
    diags.append(&mut parser_diags);
    (module, diags)
}

#[test]
fn test_set_simple_binding() {
    let (stmt, diags) = parse_stmt("set x: i32 = 42");
    assert!(diags.is_empty());
    match stmt.kind {
        StmtKind::Set { target, ty, init } => {
            assert!(matches!(target, BindingTarget::Ident(id) if id.name == "x"));
            assert!(ty.is_some());
            assert!(matches!(
                init.kind,
                ExprKind::Literal(LiteralValue::Int(42))
            ));
        }
        other => panic!("expected Set, found {other:?}"),
    }
}

#[test]
fn test_set_tuple_destructuring() {
    let (stmt, diags) = parse_stmt("set (a, b) = get_pair()");
    assert!(diags.is_empty());
    match stmt.kind {
        StmtKind::Set {
            target,
            ty,
            init: _,
        } => {
            assert!(ty.is_none());
            match target {
                BindingTarget::Tuple(bindings) => {
                    assert_eq!(bindings.len(), 2);
                    assert!(matches!(&bindings[0], BindingTarget::Ident(id) if id.name == "a"));
                    assert!(matches!(&bindings[1], BindingTarget::Ident(id) if id.name == "b"));
                }
                other => panic!("expected Tuple binding target, found {other:?}"),
            }
        }
        other => panic!("expected Set, found {other:?}"),
    }
}

#[test]
fn test_set_struct_destructuring() {
    let (stmt, diags) = parse_stmt("set { x, y } = point");
    assert!(diags.is_empty());
    match stmt.kind {
        StmtKind::Set {
            target,
            ty,
            init: _,
        } => {
            assert!(ty.is_none());
            match target {
                BindingTarget::Map(bindings) => {
                    assert_eq!(bindings.len(), 2);
                    assert_eq!(bindings[0].field.name, "x");
                    assert_eq!(bindings[1].field.name, "y");
                }
                other => panic!("expected Map binding target, found {other:?}"),
            }
        }
        other => panic!("expected Set, found {other:?}"),
    }
}

#[test]
fn test_rewrite_and_compound_assign() {
    let (stmt, diags) = parse_stmt("rewrite count += 1");
    assert!(diags.is_empty());
    match stmt.kind {
        StmtKind::Rewrite { target, op, value } => {
            assert!(matches!(target, LValue::Ident(id) if id.name == "count"));
            assert_eq!(op, AssignOp::PlusEq);
            assert!(matches!(
                value.kind,
                ExprKind::Literal(LiteralValue::Int(1))
            ));
        }
        other => panic!("expected Rewrite, found {other:?}"),
    }
}

#[test]
fn test_return_and_recur() {
    let (ret_stmt, r_diags) = parse_stmt("return true");
    assert!(r_diags.is_empty());
    match ret_stmt.kind {
        StmtKind::Return(Some(val)) => {
            assert!(matches!(
                val.kind,
                ExprKind::Literal(LiteralValue::Bool(true))
            ));
        }
        other => panic!("expected Return, found {other:?}"),
    }

    let (recur_stmt, rec_diags) = parse_stmt("recur(10, 20)");
    assert!(rec_diags.is_empty());
    match recur_stmt.kind {
        StmtKind::Recur(args) => {
            assert_eq!(args.len(), 2);
        }
        other => panic!("expected Recur, found {other:?}"),
    }
}

#[test]
fn test_defer_statement() {
    let (stmt, diags) = parse_stmt("defer { cleanup(); }");
    assert!(diags.is_empty());
    match stmt.kind {
        StmtKind::Defer(block) => {
            assert_eq!(block.stmts.len(), 1);
        }
        other => panic!("expected Defer, found {other:?}"),
    }
}

#[test]
fn test_space_statement() {
    let (stmt, diags) = parse_stmt("space");
    assert!(diags.is_empty());
    assert_eq!(stmt.kind, StmtKind::Space);
}

#[test]
fn test_redundant_parentheses_warning_w0205() {
    let src = "fun test_paren() { while (true) { space } }";
    let (_, diags) = parse_code(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].code,
        yukic_diagnostics::DiagnosticCode::Warning(WarningCode::W0205)
    );
}

#[test]
fn test_redundant_semicolon_warning_w0202() {
    let src = "fun test_semi() { set x = 1;; }";
    let (_, diags) = parse_code(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].code,
        yukic_diagnostics::DiagnosticCode::Warning(WarningCode::W0202)
    );
}

#[test]
fn test_empty_defer_warning_w0203() {
    let src = "fun test_defer() { defer {} }";
    let (_, diags) = parse_code(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].code,
        yukic_diagnostics::DiagnosticCode::Warning(WarningCode::W0203)
    );
}

#[test]
fn test_set_dynamic_array_without_initializer() {
    let src = "fun test_arr() { set arr: int32[] }";
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "expected no diagnostics, found: {:?}", diags);
    assert_eq!(module.items.len(), 1);
    if let ItemKind::Fun(f) = &module.items[0].kind {
        assert_eq!(f.body.stmts.len(), 1);
        if let StmtKind::Set { ty, init, .. } = &f.body.stmts[0].kind {
            assert!(matches!(ty.as_ref().unwrap().kind, TypeNodeKind::Array(_, None)));
            assert!(matches!(init.kind, ExprKind::ArrayInit(ref elems) if elems.is_empty()));
        } else {
            panic!("expected StmtKind::Set");
        }
    } else {
        panic!("expected ItemKind::Fun");
    }
}
