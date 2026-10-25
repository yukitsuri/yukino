use yukic_ast::*;
use yukic_diagnostics::Diagnostic;
use yukic_lexer::Lexer;
use yukic_parser::Parser;
use yukic_span::FileId;

fn parse_expr(code: &str) -> (Expr, Vec<Diagnostic>) {
    let lexer = Lexer::new(FileId(0), code);
    let (tokens, mut diags) = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let (expr, mut parser_diags) = parser.parse_expr();
    diags.append(&mut parser_diags);
    (expr, diags)
}

#[test]
fn test_pratt_precedence_mult_over_add() {
    let (expr, diags) = parse_expr("1 + 2 * 3");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::Binary(BinaryOp::Add, lhs, rhs) => {
            assert_eq!(lhs.kind, ExprKind::Literal(LiteralValue::Int(1)));
            match rhs.kind {
                ExprKind::Binary(BinaryOp::Mul, m_lhs, m_rhs) => {
                    assert_eq!(m_lhs.kind, ExprKind::Literal(LiteralValue::Int(2)));
                    assert_eq!(m_rhs.kind, ExprKind::Literal(LiteralValue::Int(3)));
                }
                other => panic!("expected Mul, found {other:?}"),
            }
        }
        other => panic!("expected Add, found {other:?}"),
    }
}

#[test]
fn test_power_is_right_associative() {
    let (expr, diags) = parse_expr("2 ** 3 ** 4");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::Binary(BinaryOp::Pow, lhs, rhs) => {
            assert_eq!(lhs.kind, ExprKind::Literal(LiteralValue::Int(2)));
            match rhs.kind {
                ExprKind::Binary(BinaryOp::Pow, p_lhs, p_rhs) => {
                    assert_eq!(p_lhs.kind, ExprKind::Literal(LiteralValue::Int(3)));
                    assert_eq!(p_rhs.kind, ExprKind::Literal(LiteralValue::Int(4)));
                }
                other => panic!("expected Pow on rhs, found {other:?}"),
            }
        }
        other => panic!("expected Pow at root, found {other:?}"),
    }
}

#[test]
fn test_pipeline_operator() {
    let (expr, diags) = parse_expr("val |> transform |> safe_transform");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::Pipe(lhs, rhs) => {
            assert_eq!(
                rhs.kind,
                ExprKind::Ident(Ident::new("safe_transform", rhs.span))
            );
            match lhs.kind {
                ExprKind::Pipe(p_lhs, p_rhs) => {
                    assert_eq!(p_lhs.kind, ExprKind::Ident(Ident::new("val", p_lhs.span)));
                    assert_eq!(
                        p_rhs.kind,
                        ExprKind::Ident(Ident::new("transform", p_rhs.span))
                    );
                }
                other => panic!("expected Pipe on lhs, found {other:?}"),
            }
        }
        other => panic!("expected Pipe, found {other:?}"),
    }
}

#[test]
fn test_coalesce_and_ternary() {
    let (expr, diags) = parse_expr("a ?? b ? x : y");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::Ternary(cond, then_expr, else_expr) => {
            assert_eq!(
                then_expr.kind,
                ExprKind::Ident(Ident::new("x", then_expr.span))
            );
            assert_eq!(
                else_expr.kind,
                ExprKind::Ident(Ident::new("y", else_expr.span))
            );
            match cond.kind {
                ExprKind::Binary(BinaryOp::NullCoalesce, c_lhs, c_rhs) => {
                    assert_eq!(c_lhs.kind, ExprKind::Ident(Ident::new("a", c_lhs.span)));
                    assert_eq!(c_rhs.kind, ExprKind::Ident(Ident::new("b", c_rhs.span)));
                }
                other => panic!("expected NullCoalesce, found {other:?}"),
            }
        }
        other => panic!("expected Ternary, found {other:?}"),
    }
}

#[test]
fn test_unary_precedence() {
    let (expr, diags) = parse_expr("-a * !b");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::Binary(BinaryOp::Mul, lhs, rhs) => {
            assert!(matches!(lhs.kind, ExprKind::Unary(UnaryOp::Neg, _)));
            assert!(matches!(rhs.kind, ExprKind::Unary(UnaryOp::Not, _)));
        }
        other => panic!("expected Mul, found {other:?}"),
    }
}

#[test]
fn test_safe_call_and_method_chain() {
    let (expr, diags) = parse_expr("user?.get_profile().name");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::FieldAccess(target, field) => {
            assert_eq!(field.name, "name");
            match target.kind {
                ExprKind::SafeCall(s_target, s_field, s_args) => {
                    assert!(s_args.is_empty());
                    assert_eq!(s_field.name, "get_profile");
                    assert!(matches!(s_target.kind, ExprKind::Ident(_)));
                }
                other => panic!("expected SafeCall, found {other:?}"),
            }
        }
        other => panic!("expected FieldAccess, found {other:?}"),
    }
}

#[test]
fn test_struct_literal_parsing() {
    let (expr, diags) = parse_expr("Point { x: 10, y: 20 }");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::StructInit(name, fields) => {
            assert_eq!(name.name, "Point");
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name.name, "x");
            assert_eq!(
                fields[0].value.kind,
                ExprKind::Literal(LiteralValue::Int(10))
            );
            assert_eq!(fields[1].name.name, "y");
            assert_eq!(
                fields[1].value.kind,
                ExprKind::Literal(LiteralValue::Int(20))
            );
        }
        other => panic!("expected StructInit, found {other:?}"),
    }
}

#[test]
fn test_interpolated_string_parsing() {
    let (expr, diags) = parse_expr("\"hello ${name}!\"");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::InterpolatedString(parts) => {
            assert_eq!(parts.len(), 3);
            assert_eq!(parts[0], StringPart::Lit("hello ".to_string()));
            if let StringPart::Expr(e) = &parts[1] {
                assert_eq!(e.kind, ExprKind::Ident(Ident::new("name", e.span)));
            } else {
                panic!("expected expr part, found {:?}", parts[1]);
            }
            assert_eq!(parts[2], StringPart::Lit("!".to_string()));
        }
        other => panic!("expected InterpolatedString, found {other:?}"),
    }
}

#[test]
fn test_multiple_interpolated_string_parsing() {
    let (expr, diags) = parse_expr("\"X: ${x}, Y: ${y}\"");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::InterpolatedString(parts) => {
            assert_eq!(parts.len(), 4);
            assert_eq!(parts[0], StringPart::Lit("X: ".to_string()));
            if let StringPart::Expr(e) = &parts[1] {
                assert_eq!(e.kind, ExprKind::Ident(Ident::new("x", e.span)));
            } else {
                panic!("expected expr part, found {:?}", parts[1]);
            }
            assert_eq!(parts[2], StringPart::Lit(", Y: ".to_string()));
            if let StringPart::Expr(e) = &parts[3] {
                assert_eq!(e.kind, ExprKind::Ident(Ident::new("y", e.span)));
            } else {
                panic!("expected expr part, found {:?}", parts[3]);
            }
        }
        other => panic!("expected InterpolatedString, found {other:?}"),
    }
}

#[test]
fn test_arrow_function_parsing() {
    let (expr, diags) = parse_expr("(x) => x + 1");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::Lambda(params, body) => {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "x");
            assert!(matches!(body.kind, ExprKind::Binary(BinaryOp::Add, _, _)));
        }
        other => panic!("expected Lambda, found {other:?}"),
    }
}

#[test]
fn test_fun_anonymous_closure_parsing() {
    let (expr, diags) = parse_expr("fun(x) => x + 1");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::AnonymousFun(decl) => {
            assert_eq!(decl.params.len(), 1);
            assert_eq!(decl.params[0].name.name, "x");
            assert_eq!(decl.body.stmts.len(), 1);
            assert!(matches!(
                decl.body.stmts[0].kind,
                yukic_ast::StmtKind::Return(Some(_))
            ));
        }
        other => panic!("expected AnonymousFun, found {other:?}"),
    }

    let (expr2, diags2) = parse_expr("fun(a: int32, b: int32) -> int32 => a + b");
    assert!(diags2.is_empty());
    match expr2.kind {
        ExprKind::AnonymousFun(decl) => {
            assert_eq!(decl.params.len(), 2);
            assert!(decl.return_type.is_some());
        }
        other => panic!("expected AnonymousFun, found {other:?}"),
    }
}

#[test]
fn test_switch_expression_parsing() {
    let src = "switch val { 0 -> { 10 }, 1..=5 -> { 20 }, else -> { 30 } }";
    let (expr, diags) = parse_expr(src);
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::Switch { target, arms } => {
            assert_eq!(target.kind, ExprKind::Ident(Ident::new("val", target.span)));
            assert_eq!(arms.len(), 3);
            assert!(matches!(
                arms[0].pattern.kind,
                PatternKind::Literal(LiteralValue::Int(0))
            ));
            assert!(matches!(
                arms[1].pattern.kind,
                PatternKind::Range(_, _, true)
            ));
            assert!(matches!(arms[2].pattern.kind, PatternKind::Wildcard));
        }
        other => panic!("expected Switch, found {other:?}"),
    }
}

#[test]
fn test_switch_with_else_arm() {
    let src = "switch val { 1 -> 10, else -> 20 }";
    let (expr, diags) = parse_expr(src);
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::Switch { arms, .. } => {
            assert_eq!(arms.len(), 2);
            assert!(matches!(arms[1].pattern.kind, PatternKind::Wildcard));
        }
        other => panic!("expected Switch, found {other:?}"),
    }
}

#[test]
fn test_switch_underscore_wildcard_rejected() {
    let src = "switch val { 0 -> { 10 }, _ -> { 20 } }";
    let (_expr, diags) = parse_expr(src);
    assert!(!diags.is_empty());
    assert!(diags.iter().any(|d| d.message.contains("use 'else' instead")));
}

#[test]
fn test_switch_fat_arrow_rejected() {
    let src = "switch val { 0 => { 10 }, else => { 20 } }";
    let (_expr, diags) = parse_expr(src);
    assert!(!diags.is_empty());
    assert!(diags.iter().any(|d| d.message.contains("expected '->' after switch pattern")));
}

#[test]
fn test_sus_catch_without_parens() {
    let src = "sus { risky() } catch e: NetworkError { handle(e) } catch { fallback() } finally { cleanup() }";
    let (expr, diags) = parse_expr(src);
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::Sus {
            catches,
            finally_branch,
            ..
        } => {
            assert_eq!(catches.len(), 2);
            assert_eq!(catches[0].error_var.as_ref().unwrap().name, "e");
            assert!(catches[0].error_type.is_some());
            assert!(catches[1].error_var.is_none());
            assert!(catches[1].error_type.is_none());
            assert!(finally_branch.is_some());
        }
        other => panic!("expected Sus, found {other:?}"),
    }
}

#[test]
fn test_sus_catch_with_parens_error() {
    let src = "sus { risky() } catch (e) { handle(e) }";
    let (_, diags) = parse_expr(src);
    assert!(diags.iter().any(|d| d.message.contains("parentheses around catch parameter are not allowed")));
}

#[test]
fn test_map_init_parsing() {
    let src = "{\"name\": \"yukino\", \"version\": 1}";
    let (expr, diags) = parse_expr(src);
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::MapInit(entries) => {
            assert_eq!(entries.len(), 2);
            assert!(matches!(
                entries[0].0.kind,
                ExprKind::Literal(LiteralValue::String(_))
            ));
            assert!(matches!(
                entries[0].1.kind,
                ExprKind::Literal(LiteralValue::String(_))
            ));
            assert!(matches!(
                entries[1].0.kind,
                ExprKind::Literal(LiteralValue::String(_))
            ));
            assert!(matches!(
                entries[1].1.kind,
                ExprKind::Literal(LiteralValue::Int(1))
            ));
        }
        other => panic!("expected MapInit, found {other:?}"),
    }
}

#[test]
fn test_anonymous_fun_parsing() {
    let src = "fun(x: int64, y) -> int64 { return x + y }";
    let (expr, diags) = parse_expr(src);
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::AnonymousFun(f) => {
            assert_eq!(f.params.len(), 2);
            assert_eq!(f.params[0].name.name, "x");
            assert!(f.params[0].ty.is_some());
            assert_eq!(f.params[1].name.name, "y");
            assert!(f.params[1].ty.is_none());
            assert!(f.return_type.is_some());
            assert_eq!(f.body.stmts.len(), 1);
        }
        other => panic!("expected AnonymousFun, found {other:?}"),
    }
}

#[test]
fn test_region_block_parsing() {
    let src = "region { set temp = compute(); temp.process() }";
    let (expr, diags) = parse_expr(src);
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::RegionBlock(b) => {
            assert_eq!(b.stmts.len(), 2);
        }
        other => panic!("expected RegionBlock, found {other:?}"),
    }
}

#[test]
fn test_spread_expression_parsing() {
    let src = "...args";
    let (expr, diags) = parse_expr(src);
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::Spread(inner) => {
            assert!(matches!(inner.kind, ExprKind::Ident(_)));
        }
        other => panic!("expected Spread, found {other:?}"),
    }
}

#[test]
fn test_ptr_of_and_cast_parsing() {
    let (p_expr, diags1) = parse_expr("ptr_of(my_var)");
    assert!(diags1.is_empty());
    match p_expr.kind {
        ExprKind::PtrOf(id) => {
            assert_eq!(id.name, "my_var");
        }
        other => panic!("expected PtrOf, found {other:?}"),
    }

    let (c_expr, diags2) = parse_expr("cast<ptr<int32>>(raw_ptr)");
    assert!(diags2.is_empty());
    match c_expr.kind {
        ExprKind::PtrCast(target, ty) => {
            assert!(matches!(target.kind, ExprKind::Ident(_)));
            assert!(matches!(ty.kind, TypeNodeKind::Pointer(_)));
        }
        other => panic!("expected PtrCast, found {other:?}"),
    }

    let (rt_p_expr, diags3) = parse_expr("rt::ptr_of(my_var)");
    assert!(diags3.is_empty());
    match rt_p_expr.kind {
        ExprKind::PtrOf(id) => {
            assert_eq!(id.name, "my_var");
        }
        other => panic!("expected PtrOf, found {other:?}"),
    }

    let (rt_c_expr, diags4) = parse_expr("rt::cast<ptr<int32>>(raw_ptr)");
    assert!(diags4.is_empty());
    match rt_c_expr.kind {
        ExprKind::PtrCast(target, ty) => {
            assert!(matches!(target.kind, ExprKind::Ident(_)));
            assert!(matches!(ty.kind, TypeNodeKind::Pointer(_)));
        }
        other => panic!("expected PtrCast, found {other:?}"),
    }
}

#[test]
fn test_labeled_for_loop() {
    let src = ":outer for i in 0..10 { break :outer }";
    let (expr, diags) = parse_expr(src);
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::For { label, body, .. } => {
            assert_eq!(label.as_ref().unwrap().name, "outer");
            assert_eq!(body.stmts.len(), 1);
            match &body.stmts[0].kind {
                StmtKind::Break(l) => {
                    assert_eq!(l.as_ref().unwrap().name, "outer");
                }
                other => panic!("expected Break, found {other:?}"),
            }
        }
        other => panic!("expected For, found {other:?}"),
    }

    let src2 = "outer: for i in 0..10 { break outer }";
    let (expr2, diags2) = parse_expr(src2);
    assert!(diags2.is_empty());
    match expr2.kind {
        ExprKind::For { label, .. } => {
            assert_eq!(label.as_ref().unwrap().name, "outer");
        }
        other => panic!("expected For, found {other:?}"),
    }
}

#[test]
fn test_labeled_while_and_loop() {
    let src1 = "outer: while true { break outer }";
    let (expr1, diags1) = parse_expr(src1);
    assert!(diags1.is_empty());
    match expr1.kind {
        ExprKind::While { label, .. } => {
            assert_eq!(label.as_ref().unwrap().name, "outer");
        }
        other => panic!("expected While, found {other:?}"),
    }

    let src2 = "outer: loop { break outer }";
    let (expr2, diags2) = parse_expr(src2);
    assert!(diags2.is_empty());
    match expr2.kind {
        ExprKind::Loop { label, .. } => {
            assert_eq!(label.as_ref().unwrap().name, "outer");
        }
        other => panic!("expected Loop, found {other:?}"),
    }

    let src3 = "while outer: true { continue outer }";
    let (expr3, diags3) = parse_expr(src3);
    assert!(diags3.is_empty());
    match expr3.kind {
        ExprKind::While { label, .. } => {
            assert_eq!(label.as_ref().unwrap().name, "outer");
        }
        other => panic!("expected While, found {other:?}"),
    }

    let src4 = "loop outer: { break outer }";
    let (expr4, diags4) = parse_expr(src4);
    assert!(diags4.is_empty());
    match expr4.kind {
        ExprKind::Loop { label, .. } => {
            assert_eq!(label.as_ref().unwrap().name, "outer");
        }
        other => panic!("expected Loop, found {other:?}"),
    }
}

#[test]
fn test_panic_expr_parsing() {
    let (expr, diags) = parse_expr("panic(\"something failed\")");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::Call(callee, args) => {
            match callee.kind {
                ExprKind::Ident(id) => assert_eq!(id.name, "panic"),
                other => panic!("expected panic ident, found {other:?}"),
            }
            assert_eq!(args.len(), 1);
            assert!(matches!(
                args[0].kind,
                ExprKind::Literal(LiteralValue::String(_))
            ));
        }
        other => panic!("expected Call, found {other:?}"),
    }
}

#[test]
fn test_typed_sus_catch_parsing() {
    let src = "sus { risky() } catch e: MathError { e.message } catch MathError { 0 }";
    let (expr, diags) = parse_expr(src);
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::Sus { catches, .. } => {
            assert_eq!(catches.len(), 2);
            assert_eq!(catches[0].error_var.as_ref().unwrap().name, "e");
            assert!(catches[0].error_type.is_some());
            assert!(catches[1].error_var.is_none());
            assert!(catches[1].error_type.is_some());
        }
        other => panic!("expected Sus, found {other:?}"),
    }
}

#[test]
fn test_named_arguments_parsing() {
    let src = "connect(host = \"localhost\", port = 8080)";
    let (expr, diags) = parse_expr(src);
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::Call(callee, args) => {
            assert!(matches!(callee.kind, ExprKind::Ident(ref id) if id.name == "connect"));
            assert_eq!(args.len(), 2);
            match &args[0].kind {
                ExprKind::NamedArg(name, val) => {
                    assert_eq!(name.name, "host");
                    assert!(matches!(val.kind, ExprKind::Literal(LiteralValue::String(_))));
                }
                other => panic!("expected NamedArg, found {other:?}"),
            }
            match &args[1].kind {
                ExprKind::NamedArg(name, val) => {
                    assert_eq!(name.name, "port");
                    assert_eq!(val.kind, ExprKind::Literal(LiteralValue::Int(8080)));
                }
                other => panic!("expected NamedArg, found {other:?}"),
            }
        }
        other => panic!("expected Call, found {other:?}"),
    }
}

#[test]
fn test_method_named_arguments_parsing() {
    let src = "client.request(url = \"/api\", timeout = 30)";
    let (expr, diags) = parse_expr(src);
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::MethodCall(target, method, args) => {
            assert!(matches!(target.kind, ExprKind::Ident(ref id) if id.name == "client"));
            assert_eq!(method.name, "request");
            assert_eq!(args.len(), 2);
            match &args[0].kind {
                ExprKind::NamedArg(name, _) => assert_eq!(name.name, "url"),
                other => panic!("expected NamedArg, found {other:?}"),
            }
            match &args[1].kind {
                ExprKind::NamedArg(name, _) => assert_eq!(name.name, "timeout"),
                other => panic!("expected NamedArg, found {other:?}"),
            }
        }
        other => panic!("expected MethodCall, found {other:?}"),
    }
}

#[test]
fn test_slice_and_index_access_parsing() {
    // 1. Regular index: s[0]
    let (expr, diags) = parse_expr("s[0]");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::IndexAccess(target, index) => {
            assert!(matches!(target.kind, ExprKind::Ident(ref id) if id.name == "s"));
            assert_eq!(index.kind, ExprKind::Literal(LiteralValue::Int(0)));
        }
        other => panic!("expected IndexAccess, found {other:?}"),
    }

    // 2. Full slice: s[0..5]
    let (expr, diags) = parse_expr("s[0..5]");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::SliceAccess(target, Some(start), Some(end), false) => {
            assert!(matches!(target.kind, ExprKind::Ident(ref id) if id.name == "s"));
            assert_eq!(start.kind, ExprKind::Literal(LiteralValue::Int(0)));
            assert_eq!(end.kind, ExprKind::Literal(LiteralValue::Int(5)));
        }
        other => panic!("expected SliceAccess [0..5], found {other:?}"),
    }

    // 3. Inclusive slice: s[0..=5]
    let (expr, diags) = parse_expr("s[0..=5]");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::SliceAccess(target, Some(start), Some(end), true) => {
            assert!(matches!(target.kind, ExprKind::Ident(ref id) if id.name == "s"));
            assert_eq!(start.kind, ExprKind::Literal(LiteralValue::Int(0)));
            assert_eq!(end.kind, ExprKind::Literal(LiteralValue::Int(5)));
        }
        other => panic!("expected SliceAccess [0..=5], found {other:?}"),
    }

    // 4. Open-ended slice: s[2..]
    let (expr, diags) = parse_expr("s[2..]");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::SliceAccess(target, Some(start), None, false) => {
            assert!(matches!(target.kind, ExprKind::Ident(ref id) if id.name == "s"));
            assert_eq!(start.kind, ExprKind::Literal(LiteralValue::Int(2)));
        }
        other => panic!("expected SliceAccess [2..], found {other:?}"),
    }

    // 5. Open-start slice: s[..5]
    let (expr, diags) = parse_expr("s[..5]");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::SliceAccess(target, None, Some(end), false) => {
            assert!(matches!(target.kind, ExprKind::Ident(ref id) if id.name == "s"));
            assert_eq!(end.kind, ExprKind::Literal(LiteralValue::Int(5)));
        }
        other => panic!("expected SliceAccess [..5], found {other:?}"),
    }

    // 6. Open-start inclusive slice: s[..=5]
    let (expr, diags) = parse_expr("s[..=5]");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::SliceAccess(target, None, Some(end), true) => {
            assert!(matches!(target.kind, ExprKind::Ident(ref id) if id.name == "s"));
            assert_eq!(end.kind, ExprKind::Literal(LiteralValue::Int(5)));
        }
        other => panic!("expected SliceAccess [..=5], found {other:?}"),
    }

    // 7. Full range slice: s[..]
    let (expr, diags) = parse_expr("s[..]");
    assert!(diags.is_empty());
    match expr.kind {
        ExprKind::SliceAccess(target, None, None, false) => {
            assert!(matches!(target.kind, ExprKind::Ident(ref id) if id.name == "s"));
        }
        other => panic!("expected SliceAccess [..], found {other:?}"),
    }
}

