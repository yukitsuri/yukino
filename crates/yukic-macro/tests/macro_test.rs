use std::fs;
use yukic_ast::*;
use yukic_lexer::Lexer;
use yukic_macro::MacroExpander;
use yukic_parser::Parser;
use yukic_resolve::{Resolver, build_monolithic_module};
use yukic_span::{FileId, SourceMap};

fn parse_code(source: &str, file_id: FileId) -> Module {
    let lexer = Lexer::new(file_id, source);
    let (tokens, _) = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let (module, _) = parser.parse_module();
    module
}

#[test]
fn test_macro_expand_expression_e2e() {
    let code = r"
define square(x) => x * x

fun test_fn() {
    set res = square!(5)
}
";
    let mut module = parse_code(code, FileId(0));
    let expander = MacroExpander::new();
    let result = expander.expand_module(&mut module, None);
    assert!(result.is_ok());

    if let ItemKind::Fun(f) = &module.items[1].kind {
        if let StmtKind::Set { init, .. } = &f.body.stmts[0].kind {
            if let ExprKind::Binary(BinaryOp::Mul, lhs, rhs) = &init.kind {
                assert_eq!(lhs.kind, ExprKind::Literal(LiteralValue::Int(5)));
                assert_eq!(rhs.kind, ExprKind::Literal(LiteralValue::Int(5)));
            } else {
                panic!("expected binary multiplication expression");
            }
        } else {
            panic!("expected set statement");
        }
    } else {
        panic!("expected fun item");
    }
}

#[test]
fn test_macro_builtin_rt_typeof() {
    let code = r#"
fun run() {
    set t = rt::typeof!(42)
}
"#;
    let mut module = parse_code(code, FileId(0));
    let expander = MacroExpander::new();
    let result = expander.expand_module(&mut module, None);
    assert!(result.is_ok());

    if let ItemKind::Fun(f) = &module.items[0].kind {
        if let StmtKind::Set { init, .. } = &f.body.stmts[0].kind {
            if let ExprKind::Call(callee, args) = &init.kind {
                if let ExprKind::Ident(id) = &callee.kind {
                    assert_eq!(id.name, "__typeof_intrinsic__");
                    assert_eq!(args.len(), 1);
                } else {
                    panic!("expected intrinsic ident");
                }
            } else {
                panic!("expected intrinsic call");
            }
        } else {
            panic!("expected set statement");
        }
    } else {
        panic!("expected fun item");
    }
}

#[test]
fn test_macro_expand_block_splicing_e2e() {
    let code = r"
define setup_two_vars() {
    set x = 10
    set y = 20
}

fun run() {
    setup_two_vars!()
}
";
    let mut module = parse_code(code, FileId(0));
    let expander = MacroExpander::new();
    let result = expander.expand_module(&mut module, None);
    assert!(result.is_ok());

    if let ItemKind::Fun(f) = &module.items[1].kind {
        assert_eq!(f.body.stmts.len(), 2);
        if let StmtKind::Set { target, init, .. } = &f.body.stmts[0].kind {
            if let BindingTarget::Ident(id) = target {
                assert!(id.name.starts_with('x'));
            }
            assert_eq!(init.kind, ExprKind::Literal(LiteralValue::Int(10)));
        }
        if let StmtKind::Set { target, init, .. } = &f.body.stmts[1].kind {
            if let BindingTarget::Ident(id) = target {
                assert!(id.name.starts_with('y'));
            }
            assert_eq!(init.kind, ExprKind::Literal(LiteralValue::Int(20)));
        }
    } else {
        panic!("expected fun item");
    }
}

#[test]
fn test_macro_expand_hygiene_e2e() {
    let code = r#"
define run_task(action) {
    set secret = 42
    action
}

fun main() {
    set secret = "outer"
    run_task!(secret)
}
"#;
    let mut module = parse_code(code, FileId(0));
    let expander = MacroExpander::new();
    let result = expander.expand_module(&mut module, None);
    assert!(result.is_ok());

    if let ItemKind::Fun(f) = &module.items[1].kind {
        // statement 0: set secret = "outer"
        if let StmtKind::Set {
            target: BindingTarget::Ident(id),
            ..
        } = &f.body.stmts[0].kind
        {
            assert_eq!(id.name, "secret");
        }
        // statement 1: set secret_g* = 42 (hygienic internal variable)
        if let StmtKind::Set {
            target: BindingTarget::Ident(id),
            ..
        } = &f.body.stmts[1].kind
        {
            assert_ne!(id.name, "secret");
            assert!(id.name.starts_with("secret_g"));
        }
        // statement 2: secret (passed from caller, remains "secret")
        if let StmtKind::Expr(e) = &f.body.stmts[2].kind
            && let ExprKind::Ident(id) = &e.kind
        {
            assert_eq!(id.name, "secret");
        }
    }
}

#[test]
fn test_macro_builtin_include_strings_and_bytes() {
    let dir = std::env::temp_dir().join("yukino_macro_test");
    let _ = fs::create_dir_all(&dir);
    let txt_path = dir.join("hello.txt");
    fs::write(&txt_path, "yukino compiled resource").unwrap();

    let code = format!(
        r#"
static text_data = include_strings!("{path}")
"#,
        // forward slashes: backslashes would act as escapes in the string literal
        path = txt_path.display().to_string().replace('\\', "/")
    );

    let mut module = parse_code(&code, FileId(0));
    let expander = MacroExpander::new();
    let res = expander.expand_module(&mut module, None);
    assert!(res.is_ok());

    if let ItemKind::Static(s) = &module.items[0].kind {
        assert_eq!(
            s.init.kind,
            ExprKind::Literal(LiteralValue::String("yukino compiled resource".into()))
        );
    }

    let _ = fs::remove_file(txt_path);
    let _ = fs::remove_dir(dir);
}

#[test]
fn test_macro_include_relative_to_config_dir() {
    let tmp = std::env::temp_dir().join(format!("yukino_inc_rel_{}", std::process::id()));
    let _ = fs::create_dir_all(&tmp);
    let src_dir = tmp.join("src");
    let assets_dir = tmp.join("assets");
    let _ = fs::create_dir_all(&src_dir);
    let _ = fs::create_dir_all(&assets_dir);

    fs::write(tmp.join("yuki.toml"), "[bin]\nname = \"rel_test\"\n").unwrap();
    fs::write(assets_dir.join("data.txt"), "config-relative content").unwrap();

    let code = "static content = include_str!(\"assets/data.txt\")\n";
    let src_file = src_dir.join("main.ykn");
    fs::write(&src_file, code).unwrap();

    let mut sm = SourceMap::new();
    let fid = sm.add_file(&src_file, code.to_string());

    let mut module = parse_code(code, fid);
    let expander = MacroExpander::new();
    let res = expander.expand_module(&mut module, Some(&sm));
    assert!(res.is_ok());

    if let ItemKind::Static(s) = &module.items[0].kind {
        assert_eq!(
            s.init.kind,
            ExprKind::Literal(LiteralValue::String("config-relative content".into()))
        );
    } else {
        panic!("expected static item");
    }

    let _ = fs::remove_dir_all(tmp);
}

#[test]
fn test_macro_builtin_file_line_col() {
    let code = "fun main() {\n    set f = file!()\n    set l = line!()\n    set c = col!()\n}";
    let mut sm = SourceMap::new();
    let fid = sm.add_file("src/main.ykn", code.to_string());

    let mut module = parse_code(code, fid);
    let expander = MacroExpander::new();
    let res = expander.expand_module(&mut module, Some(&sm));
    assert!(res.is_ok());

    if let ItemKind::Fun(f) = &module.items[0].kind {
        // file!()
        if let StmtKind::Set { init, .. } = &f.body.stmts[0].kind {
            assert_eq!(
                init.kind,
                ExprKind::Literal(LiteralValue::String("src/main.ykn".into()))
            );
        }
        // line!()
        if let StmtKind::Set { init, .. } = &f.body.stmts[1].kind
            && let ExprKind::Literal(LiteralValue::Int(line)) = init.kind
        {
            assert_eq!(line, 3);
        }
        // col!()
        if let StmtKind::Set { init, .. } = &f.body.stmts[2].kind
            && let ExprKind::Literal(LiteralValue::Int(col)) = init.kind
        {
            assert!(col > 1);
        }
    }
}

#[test]
fn test_phase3_full_pipeline_multi_module_monolith() {
    let utils_code = r"
public define add(a, b) => a + b
public fun helper() -> int64 {
    return 100
}
";

    let main_code = r"
load .utils: helper

fun main() {
    set sum = add!(10, 20)
    set h = helper()
}
";

    let mut sm = SourceMap::new();
    let fid_utils = sm.add_file("utils.ykn", utils_code.to_string());
    let fid_main = sm.add_file("main.ykn", main_code.to_string());

    let mod_utils = parse_code(utils_code, fid_utils);
    let mod_main = parse_code(main_code, fid_main);

    // 1. Resolve symbols & dependencies
    let mut resolver = Resolver::new();
    let (id_utils, diags_u) = resolver.add_module("utils".into(), &mod_utils);
    assert!(diags_u.is_empty());
    let (id_main, diags_m) = resolver.add_module("main".into(), &mod_main);
    assert!(diags_m.is_empty());

    resolver.add_dependency(id_main, id_utils, yukic_span::Span::DUMMY);

    let cycle_diags = resolver.check_cycles();
    assert!(cycle_diags.is_empty());

    // 2. Expand macros across project
    let mut modules = vec![(id_utils, mod_utils), (id_main, mod_main)];
    let mod_names = vec!["utils".to_string(), "main".to_string()];

    let expander = MacroExpander::new();
    let exp_res = expander.expand_project(&mut modules, &mod_names, Some(&sm));
    assert!(exp_res.is_ok());

    // 3. Monolithic AST module creation in topological order
    let topo_order = resolver.graph.topological_order().expect("valid DAG");
    assert_eq!(topo_order, vec![id_utils, id_main]);

    let monolith = build_monolithic_module(&modules, &topo_order, fid_main);
    assert_eq!(monolith.file_id, fid_main);

    // items should contain: macro def, helper fun, main fun (internal imports filtered out)
    assert!(monolith.items.len() >= 3);
}

#[test]
fn test_macro_builtin_compile_print() {
    let code = r#"
fun run() {
    compile_print!("hello from compile_print\n")
    set msg = compile_print!("comptime message")
}
"#;
    let mut module = parse_code(code, FileId(0));
    let expander = MacroExpander::new();
    let result = expander.expand_module(&mut module, None);
    assert!(result.is_ok());

    if let ItemKind::Fun(f) = &module.items[0].kind {
        if let StmtKind::Expr(expr) = &f.body.stmts[0].kind {
            assert_eq!(expr.kind, ExprKind::Literal(LiteralValue::String("hello from compile_print\n".into())));
        } else {
            panic!("expected expr statement from compile_print!");
        }

        if let StmtKind::Set { init, .. } = &f.body.stmts[1].kind {
            assert_eq!(init.kind, ExprKind::Literal(LiteralValue::String("comptime message".into())));
        } else {
            panic!("expected set statement initialized with compile_print!");
        }
    } else {
        panic!("expected fun item");
    }
}
