#![deny(unsafe_code)]

use yukic_ast::Module;
use yukic_borrow::{BorrowChecker, EscapeKind};
use yukic_diagnostics::code::{ErrorCode, WarningCode};
use yukic_lexer::Lexer;
use yukic_parser::Parser;
use yukic_span::FileId;

fn parse_code(source: &str, file_id: FileId) -> Module {
    let lexer = Lexer::new(file_id, source);
    let (tokens, _) = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let (module, _) = parser.parse_module();
    module
}

#[test]
fn test_copy_types_no_move() {
    let code = r"
fun test_fn() {
    set a: int64 = 42
    set b = a
    set c = a
}
";
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    assert!(checker.check_module(&module).is_ok());
}

#[test]
fn test_string_is_copy_and_does_not_move() {
    let code = r#"
fun test_fn() {
    set s = "hello"
    set t = s
    set u = s
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    assert!(checker.check_module(&module).is_ok());
}

#[test]
fn test_affine_use_after_move() {
    let code = r#"
fun test_fn() {
    set s = [1, 2, 3]
    set t = s
    set u = s
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    let res = checker.check_module(&module);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert_eq!(errs[0].code.as_str(), ErrorCode::E0501.as_str());
}

#[test]
fn test_reinitialization_after_move() {
    let code = r#"
fun test_fn() {
    set s = [1, 2, 3]
    set t = s
    rewrite s = [4, 5, 6]
    set u = s
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    assert!(checker.check_module(&module).is_ok());
}

#[test]
fn test_cannot_move_while_borrowed() {
    let code = r#"
fun test_fn() {
    set s = [1, 2, 3]
    set r = ref s
    set t = s
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    let res = checker.check_module(&module);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert_eq!(errs[0].code.as_str(), ErrorCode::E0502.as_str());
}

#[test]
fn test_simultaneous_mut_and_shared_borrow() {
    let code1 = r#"
fun test_fn() {
    set s = "hello"
    set r1 = ref s
    set r2 = ref ~s
}
"#;
    let module1 = parse_code(code1, FileId(0));
    let mut checker1 = BorrowChecker::new();
    let res1 = checker1.check_module(&module1);
    assert!(res1.is_err());
    assert_eq!(
        res1.unwrap_err()[0].code.as_str(),
        ErrorCode::E0502.as_str()
    );

    let code2 = r#"
fun test_fn() {
    set s = "hello"
    set r1 = ref ~s
    set r2 = ref s
}
"#;
    let module2 = parse_code(code2, FileId(0));
    let mut checker2 = BorrowChecker::new();
    let res2 = checker2.check_module(&module2);
    assert!(res2.is_err());
    assert_eq!(
        res2.unwrap_err()[0].code.as_str(),
        ErrorCode::E0502.as_str()
    );
}

#[test]
fn test_multiple_shared_borrows_allowed() {
    let code = r#"
fun test_fn() {
    set s = "hello"
    set r1 = ref s
    set r2 = ref s
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    assert!(checker.check_module(&module).is_ok());
}

#[test]
fn test_mutate_while_borrowed() {
    let code = r#"
fun test_fn() {
    set s = "hello"
    set r = ref s
    rewrite s = "modified"
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    let res = checker.check_module(&module);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert_eq!(errs[0].code.as_str(), ErrorCode::E0503.as_str());
}

#[test]
fn test_borrow_outlives_owner() {
    let code = r#"
fun test_fn() {
    set outer = "temp"
    {
        set inner = "inner_value"
        rewrite outer = ref inner
    }
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    let res = checker.check_module(&module);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert_eq!(errs[0].code.as_str(), ErrorCode::E0504.as_str());
}

#[test]
fn test_move_in_loop() {
    let code = r#"
fun consume(x: int32[]) {}

fun test_fn() {
    set s = [1, 2]
    while true {
        consume(s)
    }
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    let res = checker.check_module(&module);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert_eq!(errs[0].code.as_str(), ErrorCode::E0505.as_str());
}

#[test]
fn test_conditional_move_in_branch() {
    let code = r#"
fun consume(x: int32[]) {}

fun test_fn() {
    set data = [1, 2]
    if true {
        consume(data)
    } else {
    }
    consume(data)
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    let res = checker.check_module(&module);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert_eq!(errs[0].code.as_str(), ErrorCode::E0501.as_str());
}

#[test]
fn test_balanced_reinit_in_both_branches() {
    let code = r#"
fun consume(x: int32[]) {}

fun test_fn() {
    set data = [1, 2]
    if true {
        consume(data)
        rewrite data = [3]
    } else {
        rewrite data = [4]
    }
    consume(data)
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    assert!(checker.check_module(&module).is_ok());
}

#[test]
fn test_borrow_ends_at_scope_exit() {
    let code = r#"
fun test_fn() {
    set s = [1, 2]
    {
        set r = ref s
    }
    set t = s
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    assert!(checker.check_module(&module).is_ok());
}

#[test]
fn test_escape_analysis_no_escape() {
    let code = r"
fun test_fn() {
    set arr = [1, 2, 3]
    set val = arr[0]
}
";
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    assert!(checker.check_module(&module).is_ok());
    assert_eq!(
        checker.escape_info.var_escapes.get("arr"),
        Some(&EscapeKind::NoEscape)
    );
}

#[test]
fn test_escape_analysis_escape_to_caller() {
    let code = r"
fun test_fn() -> int64[] {
    set arr = [1, 2, 3]
    return arr
}
";
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    assert!(checker.check_module(&module).is_ok());
    assert_eq!(
        checker.escape_info.var_escapes.get("arr"),
        Some(&EscapeKind::EscapeToCaller)
    );
}

#[test]
fn test_escape_analysis_loop_escape() {
    let code = r"
fun test_fn() {
    set list = []
    for i in 0..10 {
        set item = [i, 2]
        list.push(item)
    }
}
";
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    assert!(checker.check_module(&module).is_ok());
    assert_eq!(
        checker.escape_info.var_escapes.get("item"),
        Some(&EscapeKind::LoopEscape)
    );
}

#[test]
fn test_escape_analysis_global_escape() {
    let code = r"
fun test_fn() {
    set item = [1, 2, 3]
    set s = shared.new(item)
}
";
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    assert!(checker.check_module(&module).is_ok());
    assert_eq!(
        checker.escape_info.var_escapes.get("item"),
        Some(&EscapeKind::GlobalEscape)
    );
}

#[test]
fn test_return_local_reference_error_e0508() {
    let code = r"
fun test_fn() {
    set x = 42
    return ref x
}
";
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    let res = checker.check_module(&module);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert_eq!(errs[0].code.as_str(), ErrorCode::E0508.as_str());
}

#[test]
fn test_return_local_reference_alias_e0508() {
    let code = r"
fun test_fn() {
    set x = 42
    set r = ref x
    return r
}
";
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    let res = checker.check_module(&module);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert_eq!(errs[0].code.as_str(), ErrorCode::E0508.as_str());
}

#[test]
fn test_unpinned_affine_discard_warning_w0501() {
    let code = r"
fun test_fn() {
    [1, 2, 3]
}
";
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    assert!(checker.check_module(&module).is_ok());
    assert!(
        checker
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == WarningCode::W0501.as_str())
    );
}

#[test]
fn test_explicit_drop_no_warning_w0501() {
    let code = r"
fun test_fn() {
    set buf = [1, 2, 3]
    buf.drop()
}
";
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    assert!(checker.check_module(&module).is_ok());
    assert!(
        !checker
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == WarningCode::W0501.as_str())
    );
}

#[test]
fn test_unnecessary_mut_borrow_warning_w0505() {
    let code = r"
fun test_fn() {
    set x = 42
    set r = ref ~x
}
";
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    let res = checker.check_module(&module);
    assert!(res.is_ok());
    assert!(
        checker
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == WarningCode::W0505.as_str())
    );
}

#[test]
fn test_mut_borrow_used_no_warning_w0505() {
    let code = r"
fun test_fn() {
    set x = 42
    set r = ref ~x
    rewrite r = 100
}
";
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    assert!(checker.check_module(&module).is_ok());
    assert!(
        !checker
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == WarningCode::W0505.as_str())
    );
}

#[test]
fn test_unresolved_type_emits_e0402() {
    let code = r"
fun test_fn() {
    set x = unknown_func()
}
";
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    let res = checker.check_module(&module);
    assert!(res.is_err());
    let err = checker
        .diagnostics
        .iter()
        .find(|d| d.code.as_str() == "E0402")
        .expect("expected E0402 error");
    assert!(err.message.contains("cannot infer type for variable `x`"));
    assert!(
        err.notes
            .iter()
            .any(|n| n.contains("explicit type annotation"))
    );
}

#[test]
fn test_mutate_ref_mut_param() {
    let code = r#"
fun modify(ref ~p: int32) {
    rewrite p = 42
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    let res = checker.check_module(&module);
    assert!(res.is_ok(), "mutating ref ~ param should be allowed, got: {:?}", res);
}

#[test]
fn test_mutate_immutable_ref_param_rejected() {
    let code = r#"
fun read_only(ref p: int32) {
    rewrite p = 42
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    let res = checker.check_module(&module);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err()[0].code.as_str(), ErrorCode::E0503.as_str());
}

#[test]
fn test_cannot_move_ref_param() {
    let code = r#"
fun consume(s: string) {}

fun modify(ref ~p: string) {
    consume(p)
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    let res = checker.check_module(&module);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err()[0].code.as_str(), ErrorCode::E0502.as_str());
}

#[test]
fn test_mutate_ref_mut_while_reborrowed_rejected() {
    let code = r#"
fun modify(ref ~p: int32) {
    set r = ref p
    rewrite p = 42
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    let res = checker.check_module(&module);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err()[0].code.as_str(), ErrorCode::E0503.as_str());
}

#[test]
fn test_early_return_does_not_poison_subsequent_uses() {
    let code = r#"
fun choose(a: string, b: string) -> string {
    if a.len() == 0 {
        return b
    }
    if b.len() == 0 {
        return a
    }
    return a + b
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    assert!(checker.check_module(&module).is_ok());
}

#[test]
fn test_return_inside_loop_allowed() {
    let code = r#"
fun find_first(items: string) -> string {
    set i = 0
    while i < 10 {
        if i == 5 {
            return items
        }
        rewrite i = i + 1
    }
    return items
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    assert!(checker.check_module(&module).is_ok());
}

#[test]
fn test_call_ref_arg_does_not_persist_borrow() {
    let code = r#"
fun inspect(ref s: string) -> bool {
    return s.len() > 0
}

fun consume(s: string) -> string {
    if inspect(ref s) {
        return s
    }
    return ""
}
"#;
    let module = parse_code(code, FileId(0));
    let mut checker = BorrowChecker::new();
    assert!(checker.check_module(&module).is_ok());
}

