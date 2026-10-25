#![deny(unsafe_code)]

use yukic_ast::{ItemKind, Module};
use yukic_dce::{DceConfig, DeadCodeEliminator};
use yukic_lexer::Lexer;
use yukic_parser::Parser;
use yukic_span::FileId;

fn parse_code(source: &str) -> Module {
    let lexer = Lexer::new(FileId(0), source);
    let (tokens, _) = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let (module, _) = parser.parse_module();
    module
}

fn has_function(module: &Module, name: &str) -> bool {
    module.items.iter().any(|item| {
        if let ItemKind::Fun(f) = &item.kind {
            f.name.as_str() == name
        } else {
            false
        }
    })
}

fn has_class(module: &Module, name: &str) -> bool {
    module.items.iter().any(|item| {
        if let ItemKind::Class(c) = &item.kind {
            c.name.as_str() == name
        } else {
            false
        }
    })
}

fn has_struct(module: &Module, name: &str) -> bool {
    module.items.iter().any(|item| {
        if let ItemKind::Struct(s) = &item.kind {
            s.name.as_str() == name
        } else {
            false
        }
    })
}

fn has_static(module: &Module, name: &str) -> bool {
    module.items.iter().any(|item| {
        if let ItemKind::Static(s) = &item.kind {
            s.name.as_str() == name
        } else {
            false
        }
    })
}

#[test]
fn test_app_basic_dce() {
    let code = r"
fun used_helper() {
}

fun unused_function() {
}

fun main() {
    used_helper()
}
";
    let mut module = parse_code(code);
    let dce = DeadCodeEliminator::new(DceConfig::app());
    let metrics = dce.eliminate(&mut module);

    assert_eq!(metrics.functions_removed, 1);
    assert!(has_function(&module, "main"));
    assert!(has_function(&module, "used_helper"));
    assert!(!has_function(&module, "unused_function"));
}

#[test]
fn test_app_transitive_call_chain() {
    let code = r"
fun f1() { f2() }
fun f2() { f3() }
fun f3() {}
fun dead1() { dead2() }
fun dead2() {}

fun main() {
    f1()
}
";
    let mut module = parse_code(code);
    let dce = DeadCodeEliminator::new(DceConfig::app());
    let metrics = dce.eliminate(&mut module);

    assert_eq!(metrics.functions_removed, 2);
    assert!(has_function(&module, "main"));
    assert!(has_function(&module, "f1"));
    assert!(has_function(&module, "f2"));
    assert!(has_function(&module, "f3"));
    assert!(!has_function(&module, "dead1"));
    assert!(!has_function(&module, "dead2"));
}

#[test]
fn test_unused_classes_and_structs() {
    let code = r"
class UsedClass {
    set x: int64 = 0
}

class DeadClass {
    set y: int64 = 1
}

struct DeadStruct {
    set z: int64
}

fun main() {
    set c = init UsedClass()
}
";
    let mut module = parse_code(code);
    let dce = DeadCodeEliminator::new(DceConfig::app());
    let metrics = dce.eliminate(&mut module);

    assert_eq!(metrics.classes_removed, 1);
    assert_eq!(metrics.structs_removed, 1);
    assert!(has_class(&module, "UsedClass"));
    assert!(!has_class(&module, "DeadClass"));
    assert!(!has_struct(&module, "DeadStruct"));
}

#[test]
fn test_library_roots() {
    let code = r"
public fun api_func() {
    internal_helper()
}

fun internal_helper() {
}

fun dead_private() {
}
";
    let mut module = parse_code(code);
    let dce = DeadCodeEliminator::new(DceConfig::static_lib());
    let metrics = dce.eliminate(&mut module);

    assert_eq!(metrics.functions_removed, 1);
    assert!(has_function(&module, "api_func"));
    assert!(has_function(&module, "internal_helper"));
    assert!(!has_function(&module, "dead_private"));
}

#[test]
fn test_prune_tests_in_normal_build() {
    let code = r"
#[test]
fun test_something() {
    only_in_test()
}

fun only_in_test() {
}

fun main() {
}
";
    let mut module = parse_code(code);
    let config = DceConfig::app().with_test(false);
    let dce = DeadCodeEliminator::new(config);
    let metrics = dce.eliminate(&mut module);

    assert_eq!(metrics.tests_pruned, 1);
    assert_eq!(metrics.functions_removed, 1);
    assert!(has_function(&module, "main"));
    assert!(!has_function(&module, "test_something"));
    assert!(!has_function(&module, "only_in_test"));
}

#[test]
fn test_preserve_tests_in_test_build() {
    let code = r"
#[test]
fun test_feature() {
    helper()
}

fun helper() {
}

fun unused_dead() {
}

fun main() {
}
";
    let mut module = parse_code(code);
    let config = DceConfig::app().with_test(true);
    let dce = DeadCodeEliminator::new(config);
    let metrics = dce.eliminate(&mut module);

    assert_eq!(metrics.tests_pruned, 0);
    assert_eq!(metrics.functions_removed, 1);
    assert!(has_function(&module, "test_feature"));
    assert!(has_function(&module, "helper"));
    assert!(!has_function(&module, "unused_dead"));
}

#[test]
fn test_debug_mode_no_strip() {
    let code = r"
fun dead_fn() {}
fun main() {}
";
    let mut module = parse_code(code);
    let config = DceConfig::app().with_strip_unused(false);
    let dce = DeadCodeEliminator::new(config);
    let metrics = dce.eliminate(&mut module);

    assert_eq!(metrics.total_removed(), 0);
    assert!(has_function(&module, "dead_fn"));
    assert!(has_function(&module, "main"));
}

#[test]
fn test_static_constants() {
    let code = r"
static ALIVE_VAL = 42
static DEAD_VAL = 100

fun main() {
    set a = ALIVE_VAL
}
";
    let mut module = parse_code(code);
    let dce = DeadCodeEliminator::new(DceConfig::app());
    let metrics = dce.eliminate(&mut module);

    assert_eq!(metrics.statics_removed, 1);
    assert!(has_static(&module, "ALIVE_VAL"));
    assert!(!has_static(&module, "DEAD_VAL"));
}

#[test]
fn test_class_hierarchy_base_preserved() {
    let code = r"
class Base {
}

class Derived extends Base {
}

fun main() {
    set d = Derived()
}
";
    let mut module = parse_code(code);
    let dce = DeadCodeEliminator::new(DceConfig::app());
    let metrics = dce.eliminate(&mut module);

    assert_eq!(metrics.classes_removed, 0);
    assert!(has_class(&module, "Base"));
    assert!(has_class(&module, "Derived"));
}

#[test]
fn test_inspect_attributes_dump() {
    let code = r"
#[stable]
fun calc() -> int64 {
    return 42
}

fun main() {
    calc()
}
";
    let mut module = parse_code(code);
    let tmp_dir = std::env::temp_dir().join("yukic_dce_test_inspect");
    let attr_file = tmp_dir.join("attributes.json");

    let config = DceConfig::app()
        .with_inspect_attributes(Some(attr_file.clone()))
        .with_project_name("test_proj");

    let dce = DeadCodeEliminator::new(config);
    dce.eliminate(&mut module);

    assert!(attr_file.exists());
    let content = std::fs::read_to_string(&attr_file).unwrap();
    assert!(content.contains("\"project\": \"test_proj\""));
    assert!(content.contains("\"name\": \"calc\""));
    assert!(content.contains("\"name\": \"stable\""));

    let _ = std::fs::remove_file(attr_file);
}

#[test]
fn test_metrics_report_format() {
    let code = r"
fun dead1() {}
fun dead2() {}
fun main() {}
";
    let mut module = parse_code(code);
    let dce = DeadCodeEliminator::new(DceConfig::app().with_verbose(true));
    let metrics = dce.eliminate(&mut module);

    assert_eq!(metrics.functions_removed, 2);
    let report = metrics.format_report();
    assert!(report.contains("removed=2"));
    assert!(report.contains("fn: 2"));
}
