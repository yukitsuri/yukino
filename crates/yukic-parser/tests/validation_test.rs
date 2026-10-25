use yukic_ast::*;
use yukic_diagnostics::{Diagnostic, DiagnosticCode, ErrorCode, WarningCode};
use yukic_lexer::Lexer;
use yukic_parser::Parser;
use yukic_span::FileId;

fn parse_code(code: &str) -> (Module, Vec<Diagnostic>) {
    let lexer = Lexer::new(FileId(0), code);
    let (tokens, mut diags) = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let (module, mut parser_diags) = parser.parse_module();
    diags.append(&mut parser_diags);
    (module, diags)
}

#[test]
fn test_valid_test_function() {
    let src = "#[test] fun test_something() { space }";
    let (_, diags) = parse_code(src);
    assert!(diags.is_empty(), "expected no diags, got: {diags:?}");

    let src_void = "#[test] fun test_void_ok() -> void { space }";
    let (_, diags_void) = parse_code(src_void);
    assert!(
        diags_void.is_empty(),
        "expected no diags for void return, got: {diags_void:?}"
    );
}

#[test]
fn test_conditional_attribute_negation_and_target_list() {
    let src = r#"
#[!target = "x86_64-pc-windows-gnu"]
fun unix_only() { space }
#[target = ["x86_64-unknown-linux-gnu", "x86_64-unknown-linux-musl"]]
fun linux_only() { space }
"#;
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "expected no diags, got: {diags:?}");
    assert_eq!(module.items.len(), 2);
    match &module.items[0].kind {
        ItemKind::Fun(fun) => assert!(fun.attrs[0].negated),
        other => panic!("expected function, found {other:?}"),
    }
    match &module.items[1].kind {
        ItemKind::Fun(fun) => assert!(!fun.attrs[0].negated),
        other => panic!("expected function, found {other:?}"),
    }
}

#[test]
fn test_multiple_attributes_in_one_group() {
    let src = r#"
!#[target = "x86_64-unknown-linux-gnu", feature = "ssl"]
#[target = "x86_64-unknown-linux-gnu", feature = "ssl"]
fun secure_linux() { space }
"#;
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "expected no diags, got: {diags:?}");
    match &module.items[0].kind {
        ItemKind::Fun(fun) => assert_eq!(fun.attrs.len(), 2),
        other => panic!("expected function, found {other:?}"),
    }
    assert_eq!(module.module_attrs.len(), 2);
}

#[test]
fn test_e0219_test_function_has_parameters() {
    let src = "#[test] fun test_with_params(x: i32) { space }";
    let (_, diags) = parse_code(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, DiagnosticCode::Error(ErrorCode::E0219));
}

#[test]
fn test_e0220_test_function_returns_value() {
    let src = "#[test] fun test_with_ret() -> i32 { return 1; }";
    let (_, diags) = parse_code(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, DiagnosticCode::Error(ErrorCode::E0220));
}

#[test]
fn test_e0221_test_function_has_generics() {
    let src = "#[test] fun test_generic<T>() { space }";
    let (_, diags) = parse_code(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, DiagnosticCode::Error(ErrorCode::E0221));
}

#[test]
fn test_e0206_cfun_with_generics() {
    let src = "cfun c_gen<T>()";
    let (_, diags) = parse_code(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, DiagnosticCode::Error(ErrorCode::E0206));
}

#[test]
fn test_e0207_cfun_with_defaults() {
    let src = "cfun c_def(x: i32 = 10)";
    let (_, diags) = parse_code(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, DiagnosticCode::Error(ErrorCode::E0207));
}

#[test]
fn test_e0209_cfun_inside_class() {
    let src = "class BadClass { cfun nested() }";
    let (_, diags) = parse_code(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, DiagnosticCode::Error(ErrorCode::E0209));
}

#[test]
fn test_e0210_ffi_attr_on_pure_yukino() {
    let src = "|stdcall| fun pure_fn() { space }";
    let (_, diags) = parse_code(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, DiagnosticCode::Error(ErrorCode::E0210));
}

#[test]
fn test_e0211_cstruct_method_forbidden() {
    let src = "cstruct RawData { fun compute() -> i32 { return 1; } }";
    let (_, diags) = parse_code(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, DiagnosticCode::Error(ErrorCode::E0211));
}

#[test]
fn test_e0212_arrow_function_block_body() {
    let src = "fun wrapper() { set f = () => { space }; }";
    let (_, diags) = parse_code(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, DiagnosticCode::Error(ErrorCode::E0212));
}

#[test]
fn test_e0215_conflicting_calling_conventions() {
    let src = "|stdcall, fastcall| cfun c_call()";
    let (_, diags) = parse_code(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, DiagnosticCode::Error(ErrorCode::E0215));
}

#[test]
fn test_e0216_conflicting_optimization_hints() {
    let src = "|cold, hot| cfun c_opt()";
    let (_, diags) = parse_code(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, DiagnosticCode::Error(ErrorCode::E0216));
}

#[test]
fn test_e0217_invalid_packed_target() {
    let src = "|packed| class MyClass {}";
    let (_, diags) = parse_code(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, DiagnosticCode::Error(ErrorCode::E0217));
}

#[test]
fn test_w0201_empty_function_body() {
    let src = "fun empty() {}";
    let (_, diags) = parse_code(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, DiagnosticCode::Warning(WarningCode::W0201));
}

#[test]
fn test_error_recovery_recovers_to_next_item() {
    let src = "set x = 1; fun valid_after() { space }";
    let (module, diags) = parse_code(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, DiagnosticCode::Error(ErrorCode::E0204));
    assert_eq!(module.items.len(), 1);
    match &module.items[0].kind {
        ItemKind::Fun(f) => assert_eq!(f.name.name, "valid_after"),
        other => panic!("expected valid_after fun, found {other:?}"),
    }
}
