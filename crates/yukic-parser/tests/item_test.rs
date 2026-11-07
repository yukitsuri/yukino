use yukic_ast::*;
use yukic_diagnostics::Diagnostic;
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
fn test_fun_declaration_with_generics_and_defaults() {
    let src = "fun greet<T>(val: T, count: i32 = 1) -> void { space }";
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty());
    assert_eq!(module.items.len(), 1);
    match &module.items[0].kind {
        ItemKind::Fun(f) => {
            assert_eq!(f.name.name, "greet");
            assert_eq!(f.generics.len(), 1);
            assert_eq!(f.generics[0].name.name, "T");
            assert_eq!(f.params.len(), 2);
            assert_eq!(f.params[0].name.name, "val");
            assert!(f.params[0].default_val.is_none());
            assert_eq!(f.params[1].name.name, "count");
            assert!(f.params[1].default_val.is_some());
            assert!(f.return_type.is_some());
        }
        other => panic!("expected Fun, found {other:?}"),
    }
}

#[test]
fn test_class_inheritance_and_with() {
    let src = "class Dog extends Animal with IRunner, IDomestic { set name: string; }";
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty());
    assert_eq!(module.items.len(), 1);
    match &module.items[0].kind {
        ItemKind::Class(c) => {
            assert_eq!(c.name.name, "Dog");
            assert_eq!(c.base_class.as_ref().unwrap().name, "Animal");
            assert_eq!(c.interfaces.len(), 2);
            assert_eq!(c.interfaces[0].name, "IRunner");
            assert_eq!(c.interfaces[1].name, "IDomestic");
            assert_eq!(c.fields.len(), 1);
            assert_eq!(c.fields[0].name.name, "name");
        }
        other => panic!("expected Class, found {other:?}"),
    }
}

#[test]
fn test_class_with_interfaces() {
    let src = "class Robot with Speaker { fun speak() { space } }";
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty());
    assert_eq!(module.items.len(), 1);
    match &module.items[0].kind {
        ItemKind::Class(c) => {
            assert_eq!(c.name.name, "Robot");
            assert!(c.base_class.is_none());
            assert_eq!(c.interfaces.len(), 1);
            assert_eq!(c.interfaces[0].name, "Speaker");
            assert_eq!(c.methods.len(), 1);
        }
        other => panic!("expected Class, found {other:?}"),
    }

    let src2 = "class Dog extends Animal with Speaker, Mover { fun speak() { space } }";
    let (module2, diags2) = parse_code(src2);
    assert!(diags2.is_empty());
    match &module2.items[0].kind {
        ItemKind::Class(c) => {
            assert_eq!(c.name.name, "Dog");
            assert_eq!(c.base_class.as_ref().unwrap().name, "Animal");
            assert_eq!(c.interfaces.len(), 2);
            assert_eq!(c.interfaces[0].name, "Speaker");
            assert_eq!(c.interfaces[1].name, "Mover");
        }
        other => panic!("expected Class, found {other:?}"),
    }
}

#[test]
fn test_member_attributes_are_preserved() {
    let src = r#"
class User {
    #[feature = "db"] id: int64;
    #[target = "x86_64-unknown-linux-gnu"] fun fetch(this) { space }
}
"#;
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "diags: {diags:?}");
    match &module.items[0].kind {
        ItemKind::Class(class) => {
            assert_eq!(class.fields[0].attrs.len(), 1);
            assert_eq!(class.methods[0].attrs.len(), 1);
        }
        other => panic!("expected Class, found {other:?}"),
    }
}

#[test]
fn test_struct_declaration() {
    let src = "struct Point { x: f64, y: f64 }";
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty());
    assert_eq!(module.items.len(), 1);
    match &module.items[0].kind {
        ItemKind::Struct(s) => {
            assert_eq!(s.name.name, "Point");
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].name.name, "x");
            assert_eq!(s.fields[1].name.name, "y");
        }
        other => panic!("expected Struct, found {other:?}"),
    }
}

#[test]
fn test_interface_declaration() {
    let src = "interface Printable { fun print_to() -> void; }";
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty());
    assert_eq!(module.items.len(), 1);
    match &module.items[0].kind {
        ItemKind::Interface(i) => {
            assert_eq!(i.name.name, "Printable");
            assert_eq!(i.methods.len(), 1);
            assert_eq!(i.methods[0].name.name, "print_to");
        }
        other => panic!("expected Interface, found {other:?}"),
    }
}

#[test]
fn test_enum_adt_variants() {
    let src = "enum Event { Click, Move(x: i32, y: i32), Key = 13 }";
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty());
    assert_eq!(module.items.len(), 1);
    match &module.items[0].kind {
        ItemKind::Enum(e) => {
            assert_eq!(e.name.name, "Event");
            assert_eq!(e.variants.len(), 3);
            assert!(matches!(&e.variants[0], EnumVariant::Simple(id) if id.name == "Click"));
            assert!(
                matches!(&e.variants[1], EnumVariant::Adt(id, fields) if id.name == "Move" && fields.len() == 2)
            );
            assert!(matches!(&e.variants[2], EnumVariant::WithValue(id, _) if id.name == "Key"));
        }
        other => panic!("expected Enum, found {other:?}"),
    }
}

#[test]
fn test_type_alias() {
    let src = "type Number = i32 | f64; type IntList = i32[];";
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty());
    assert_eq!(module.items.len(), 2);
    match &module.items[0].kind {
        ItemKind::TypeAlias(t) => {
            assert_eq!(t.name.name, "Number");
        }
        other => panic!("expected TypeAlias, found {other:?}"),
    }
    match &module.items[1].kind {
        ItemKind::TypeAlias(t) => {
            assert_eq!(t.name.name, "IntList");
        }
        other => panic!("expected TypeAlias, found {other:?}"),
    }
}

#[test]
fn test_externc_block() {
    let src = "externc \"sqlite3.h\" { cfun sqlite3_open(path: string) -> i32; cstruct sqlite3 { handle: i64 } }";
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty());
    assert_eq!(module.items.len(), 1);
    match &module.items[0].kind {
        ItemKind::Externc(ext) => {
            assert_eq!(ext.header.as_deref(), Some("sqlite3.h"));
            assert_eq!(ext.items.len(), 2);
            assert!(
                matches!(&ext.items[0], ExterncItem::Cfun(cf) if cf.name.name == "sqlite3_open")
            );
            assert!(matches!(&ext.items[1], ExterncItem::Cstruct(cs) if cs.name.name == "sqlite3"));
        }
        other => panic!("expected Externc, found {other:?}"),
    }
}

#[test]
fn test_externc_c_attributes_without_semicolon() {
    let src = r#"
externc {
    cfun abs(x: int32) -> int32
    |packed, align="4"|
    cstruct NativePoint {
        x: int32,
        y: int32
    }
}
"#;
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "diags should be empty: {diags:?}");
    assert_eq!(module.items.len(), 1);
    match &module.items[0].kind {
        ItemKind::Externc(ext) => {
            assert_eq!(ext.items.len(), 2);
            match &ext.items[0] {
                ExterncItem::Cfun(cf) => {
                    assert_eq!(cf.name.name, "abs");
                    assert!(cf.return_type.is_some());
                }
                other => panic!("expected Cfun, found {other:?}"),
            }
            match &ext.items[1] {
                ExterncItem::Cstruct(cs) => {
                    assert_eq!(cs.name.name, "NativePoint");
                    assert_eq!(cs.c_attrs.len(), 2);
                    assert_eq!(cs.c_attrs[0].name.name, "packed");
                    assert_eq!(cs.c_attrs[1].name.name, "align");
                    assert_eq!(cs.c_attrs[1].arg.as_deref(), Some("4"));
                    assert_eq!(cs.fields.len(), 2);
                }
                other => panic!("expected Cstruct, found {other:?}"),
            }
        }
        other => panic!("expected Externc, found {other:?}"),
    }
}

#[test]
fn test_externc_cfun_followed_by_c_attribute_cfun() {
    let src = r#"
externc {
    cfun foo(x: int32) -> int32
    |cdecl|
    cfun bar(y: int32) -> int32
}
"#;
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "diags should be empty: {diags:?}");
    assert_eq!(module.items.len(), 1);
    match &module.items[0].kind {
        ItemKind::Externc(ext) => {
            assert_eq!(ext.items.len(), 2);
            match &ext.items[1] {
                ExterncItem::Cfun(cf) => {
                    assert_eq!(cf.name.name, "bar");
                    assert_eq!(cf.c_attrs.len(), 1);
                    assert_eq!(cf.c_attrs[0].name.name, "cdecl");
                }
                other => panic!("expected Cfun with cdecl, found {other:?}"),
            }
        }
        other => panic!("expected Externc, found {other:?}"),
    }
}

#[test]
fn test_import_and_alias_declarations() {
    let src = "load std.io: print, println as p1, p2; alias std::collections as coll; alias std::io#println as prt;";
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "diags: {diags:?}");
    assert_eq!(module.items.len(), 3);
    match &module.items[0].kind {
        ItemKind::Import(imp) => {
            let symbols = imp.symbols.as_ref().expect("symbols clause");
            assert_eq!(symbols.items.len(), 2);
            assert_eq!(symbols.aliases.len(), 2);
            assert_eq!(symbols.aliases[0].name, "p1");
            assert_eq!(symbols.aliases[1].name, "p2");
        }
        other => panic!("expected Import, found {other:?}"),
    }
    match &module.items[1].kind {
        ItemKind::Alias(al) => {
            assert_eq!(al.alias.name, "coll");
            assert!(matches!(&al.kind, yukic_ast::AliasKind::Namespace(ns) if ns.len() == 2));
        }
        other => panic!("expected Alias, found {other:?}"),
    }
    match &module.items[2].kind {
        ItemKind::Alias(al) => {
            assert_eq!(al.alias.name, "prt");
            assert!(matches!(&al.kind, yukic_ast::AliasKind::Symbol { namespace, symbol } if namespace.len() == 2 && symbol.name == "println"));
        }
        other => panic!("expected Alias, found {other:?}"),
    }
}

#[test]
fn test_macro_definition() {
    let src = "define log!(msg: expr) => print(msg);";
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty());
    assert_eq!(module.items.len(), 1);
    match &module.items[0].kind {
        ItemKind::Macro(m) => {
            assert_eq!(m.name.name, "log");
            assert_eq!(m.params.len(), 1);
            assert_eq!(m.params[0].name.name, "msg");
        }
        other => panic!("expected MacroDef, found {other:?}"),
    }
}

#[test]
fn test_locked_class_declaration() {
    let src =
        "locked class ImmutablePoint { set x: f64; set y: f64; } public locked class Sealed {}";
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "diags: {diags:?}");
    assert_eq!(module.items.len(), 2);
    match &module.items[0].kind {
        ItemKind::Class(c) => {
            assert_eq!(c.name.name, "ImmutablePoint");
            assert!(c.is_locked);
            assert_eq!(module.items[0].visibility, Visibility::Local);
        }
        other => panic!("expected Class, found {other:?}"),
    }
    match &module.items[1].kind {
        ItemKind::Class(c) => {
            assert_eq!(c.name.name, "Sealed");
            assert!(c.is_locked);
            assert_eq!(module.items[1].visibility, Visibility::Public);
        }
        other => panic!("expected Class, found {other:?}"),
    }
}

#[test]
fn test_class_method_ref_params_parsing() {
    // ref / ref ~ first params must not be confused with `ref this`.
    let src = r#"
class Worker {
    total: int64
    fun bump(ref ~c: Counter, delta: int64) { rewrite c.value = c.value + delta }
    fun peek(ref c: Counter) -> int64 { return c.value }
    fun mix(ref ~a: Counter, ref b: Counter) -> int64 { return b.value }
}
"#;
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "diags: {diags:?}");
    assert_eq!(module.items.len(), 1);
    let ItemKind::Class(c) = &module.items[0].kind else {
        panic!("expected Class");
    };
    assert_eq!(c.methods.len(), 3);

    let bump = &c.methods[0];
    assert_eq!(bump.name.name, "bump");
    assert!(bump.this_param == ThisParam::Value);
    assert_eq!(bump.params.len(), 2);
    assert!(bump.params[0].is_ref);
    assert!(bump.params[0].is_mut);
    assert_eq!(bump.params[0].name.name, "c");
    assert!(!bump.params[1].is_ref);
    assert!(!bump.params[1].is_mut);

    let peek = &c.methods[1];
    assert_eq!(peek.name.name, "peek");
    assert!(peek.params[0].is_ref);
    assert!(!peek.params[0].is_mut);

    let mix = &c.methods[2];
    assert_eq!(mix.name.name, "mix");
    assert!(mix.params[0].is_ref && mix.params[0].is_mut);
    assert!(mix.params[1].is_ref && !mix.params[1].is_mut);
}

#[test]
fn test_class_method_ref_this_receiver_parsing() {
    let src = "class Probe { fun scan(ref this, ref data: Counter) -> int64 { return data.value } }";
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "diags: {diags:?}");
    let ItemKind::Class(c) = &module.items[0].kind else {
        panic!("expected Class");
    };
    assert_eq!(c.methods.len(), 1);
    let scan = &c.methods[0];
    assert!(scan.this_param == ThisParam::Ref);
    assert_eq!(scan.params.len(), 1);
    assert!(scan.params[0].is_ref);
    assert_eq!(scan.params[0].name.name, "data");
}

#[test]
fn test_class_method_plain_this_and_named_params() {
    let src = "class Probe { fun scan(this, first: int64) -> int64 { return first } }";
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "diags: {diags:?}");
    let ItemKind::Class(c) = &module.items[0].kind else {
        panic!("expected Class");
    };
    let scan = &c.methods[0];
    assert!(scan.this_param == ThisParam::Value);
    assert_eq!(scan.params.len(), 1);
    assert!(!scan.params[0].is_ref);
    assert_eq!(scan.params[0].name.name, "first");
}

#[test]
fn test_property_accessors_parsing() {
    let src = r#"
class User {
    set _name: string;
    get name: string { return this._name; }
    set name(val: string) { rewrite this._name = val; }
    fun get age() -> int32 { return 42; }
    fun set age(a: int32) { space }
}
"#;
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "diags: {diags:?}");
    match &module.items[0].kind {
        ItemKind::Class(c) => {
            assert_eq!(c.name.name, "User");
            assert_eq!(c.fields.len(), 1);
            assert_eq!(c.fields[0].name.name, "_name");
            assert_eq!(c.accessors.len(), 4);
            assert_eq!(c.accessors[0].name.name, "name");
            assert_eq!(c.accessors[0].kind, AccessorKind::Get);
            assert_eq!(c.accessors[1].name.name, "name");
            assert!(matches!(&c.accessors[1].kind, AccessorKind::Set(p) if p.name == "val"));
            assert_eq!(c.accessors[2].name.name, "age");
            assert_eq!(c.accessors[2].kind, AccessorKind::Get);
            assert_eq!(c.accessors[3].name.name, "age");
            assert!(matches!(&c.accessors[3].kind, AccessorKind::Set(p) if p.name == "a"));
        }
        other => panic!("expected Class, found {other:?}"),
    }
}

#[test]
fn test_main_ykn_structure_parsing() {
    let src = r#"
load std.io: print

struct MathError {
    message: string
}

fun main() {
    set input: string[] = args()

    if input.len() < 3 {
        print("Please enter 3 arguments: <num1> <op> <num2>")
        exit(1)
    }

    set input_a: int32 = cast(input[0], "int32")
    set input_op: char = cast(input[1], "char")
    set input_b: int32 = cast(input[2], "int32")

    sus {
        set result: int32 = calc(input_a, input_op, input_b)
        print("Your result: ${result}")
    } catch e: MathError {
        print("Math error: ${e.message}")
        exit(1)
    } catch {
        print("Error: unknown failure")
        exit(1)
    }
}

fun calc(a: int32, op: char, b: int32) -> int32 {
    return switch op {
        '*' -> a * b,
        '+' -> a + b,
        '-' -> a - b,
        '/' -> if b == 0 {
            panic(MathError { message: "division by zero" })
        } else {
            a / b
        },
        else -> {
            print("Invalid operator: ${op}")
            exit(1)
        }
    }
}
"#;
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "diags: {diags:?}");
    assert_eq!(module.items.len(), 4);
}

#[test]
fn test_relative_import_declarations() {
    let src = "load .io: print, println; load .io.unix: pipe, PipeFds; load ..utils: helper; load ...grandparent; load root.app: start; load path.to.test!;";
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "diags: {diags:?}");
    assert_eq!(module.items.len(), 6);
    match &module.items[0].kind {
        ItemKind::Import(imp) => {
            assert_eq!(imp.path.prefix, PathPrefix::Relative(0));
            assert_eq!(imp.path.segments.len(), 1);
            assert!(matches!(&imp.path.segments[0].pattern, yukic_ast::ImportPattern::Exact(id) if id.name == "io"));
        }
        other => panic!("expected Import, found {other:?}"),
    }
    match &module.items[1].kind {
        ItemKind::Import(imp) => {
            assert_eq!(imp.path.prefix, PathPrefix::Relative(0));
            assert_eq!(imp.path.segments.len(), 2);
            assert!(matches!(&imp.path.segments[0].pattern, yukic_ast::ImportPattern::Exact(id) if id.name == "io"));
            assert!(matches!(&imp.path.segments[1].pattern, yukic_ast::ImportPattern::Exact(id) if id.name == "unix"));
        }
        other => panic!("expected Import, found {other:?}"),
    }
    match &module.items[2].kind {
        ItemKind::Import(imp) => {
            assert_eq!(imp.path.prefix, PathPrefix::Relative(1));
            assert_eq!(imp.path.segments.len(), 1);
            assert!(matches!(&imp.path.segments[0].pattern, yukic_ast::ImportPattern::Exact(id) if id.name == "utils"));
        }
        other => panic!("expected Import, found {other:?}"),
    }
    match &module.items[3].kind {
        ItemKind::Import(imp) => {
            assert_eq!(imp.path.prefix, PathPrefix::Relative(2));
            assert_eq!(imp.path.segments.len(), 1);
            assert!(matches!(&imp.path.segments[0].pattern, yukic_ast::ImportPattern::Exact(id) if id.name == "grandparent"));
        }
        other => panic!("expected Import, found {other:?}"),
    }
    match &module.items[4].kind {
        ItemKind::Import(imp) => {
            assert_eq!(imp.path.prefix, PathPrefix::Root);
            assert_eq!(imp.path.segments.len(), 1);
            assert!(matches!(&imp.path.segments[0].pattern, yukic_ast::ImportPattern::Exact(id) if id.name == "app"));
        }
        other => panic!("expected Import, found {other:?}"),
    }
    match &module.items[5].kind {
        ItemKind::Import(imp) => {
            assert!(matches!(&imp.path.prefix, PathPrefix::Package(id) if id.name == "path"));
            assert_eq!(imp.path.segments.len(), 2);
            assert_eq!(imp.path.segments[1].force_folder, true);
        }
        other => panic!("expected Import, found {other:?}"),
    }
}

#[test]
fn test_load_without_symbols_treats_as_wildcard() {
    let src = "load std.platformio; load std.platformio!;";
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "diags: {diags:?}");
    assert_eq!(module.items.len(), 2);
    match &module.items[0].kind {
        ItemKind::Import(imp) => {
            assert_eq!(imp.path.segments.len(), 1);
            assert_eq!(imp.path.segments[0].force_folder, false);
            let symbols = imp.symbols.as_ref().expect("symbols clause present");
            assert_eq!(symbols.items.len(), 1);
            assert!(matches!(symbols.items[0].pattern, yukic_ast::ImportPattern::All));
        }
        other => panic!("expected Import, found {other:?}"),
    }
    match &module.items[1].kind {
        ItemKind::Import(imp) => {
            assert_eq!(imp.path.segments.len(), 1);
            assert_eq!(imp.path.segments[0].force_folder, true);
            let symbols = imp.symbols.as_ref().expect("symbols clause present");
            assert_eq!(symbols.items.len(), 1);
            assert!(matches!(symbols.items[0].pattern, yukic_ast::ImportPattern::All));
        }
        other => panic!("expected Import, found {other:?}"),
    }
}

#[test]
fn test_cfun_type_annotation_parsing() {
    let src = "fun main() { set cb: cfun(int32, int32) -> int32 = my_callback; }";
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "{diags:?}");
    match &module.items[0].kind {
        ItemKind::Fun(f) => match &f.body.stmts[0].kind {
            StmtKind::Set { ty: Some(type_node), .. } => match &type_node.kind {
                TypeNodeKind::CFun(params, ret) => {
                    assert_eq!(params.len(), 2);
                    assert!(matches!(&ret.kind, TypeNodeKind::Ident(id) if id.name == "int32"));
                }
                other => panic!("expected CFun, found {other:?}"),
            },
            other => panic!("expected Set, found {other:?}"),
        },
        other => panic!("expected Fun, found {other:?}"),
    }
}

#[test]
fn test_attribute_syntax_variants() {
    let src = r#"
!#[target("x86_64-unknown-linux-gnu")]
!#[warns_ignore("unused_variable")]

#[arch("x86_64")]
public fun fast_simd() -> int32 {
    return 1
}

#[!arch("arm")]
public fun non_arm() {
    space
}

#[target("x86_64-unknown-linux-gnu")]
public fun linux_only() {
    space
}

#[target = "x86_64-unknown-linux-gnu"]
public fun linux_eq() {
    space
}

#[target = ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"]]
public fun multi_target() {
    space
}

#[cold, noreturn]
public fun abort_critical() {
    space
}

#[test]
#[should_panic("fail")]
#[timeout_ms(5000)]
fun my_test() {
    space
}
"#;
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "diags: {diags:?}");
    assert_eq!(module.module_attrs.len(), 2);
    assert_eq!(module.module_attrs[0].path_str(), "target");
    assert!(!module.module_attrs[0].negated);
    assert_eq!(module.module_attrs[1].path_str(), "warns_ignore");

    assert_eq!(module.items.len(), 7);

    // fast_simd: #[arch("x86_64")]
    match &module.items[0].kind {
        ItemKind::Fun(f) => {
            assert_eq!(f.attrs.len(), 1);
            assert_eq!(f.attrs[0].path_str(), "arch");
            assert!(!f.attrs[0].negated);
            match &f.attrs[0].args {
                AttributeArgs::Positional(pos) => {
                    assert_eq!(pos.len(), 1);
                    assert!(matches!(&pos[0].kind, ExprKind::Literal(LiteralValue::String(s)) if s == "x86_64"));
                }
                other => panic!("expected positional args, got {other:?}"),
            }
        }
        other => panic!("expected Fun, got {other:?}"),
    }

    // non_arm: #[!arch("arm")]
    match &module.items[1].kind {
        ItemKind::Fun(f) => {
            assert_eq!(f.attrs.len(), 1);
            assert_eq!(f.attrs[0].path_str(), "arch");
            assert!(f.attrs[0].negated);
        }
        other => panic!("expected Fun, got {other:?}"),
    }

    // linux_only: #[target("x86_64-unknown-linux-gnu")]
    match &module.items[2].kind {
        ItemKind::Fun(f) => {
            assert_eq!(f.attrs.len(), 1);
            assert_eq!(f.attrs[0].path_str(), "target");
            assert!(!f.attrs[0].negated);
        }
        other => panic!("expected Fun, got {other:?}"),
    }

    // linux_eq: #[target = "x86_64-unknown-linux-gnu"]
    match &module.items[3].kind {
        ItemKind::Fun(f) => {
            assert_eq!(f.attrs.len(), 1);
            assert_eq!(f.attrs[0].path_str(), "target");
            assert!(!f.attrs[0].negated);
        }
        other => panic!("expected Fun, got {other:?}"),
    }

    // abort_critical: #[cold, noreturn]
    match &module.items[5].kind {
        ItemKind::Fun(f) => {
            assert_eq!(f.attrs.len(), 2);
            assert_eq!(f.attrs[0].path_str(), "cold");
            assert_eq!(f.attrs[1].path_str(), "noreturn");
        }
        other => panic!("expected Fun, got {other:?}"),
    }

    // my_test: #[test], #[should_panic("fail")], #[timeout_ms(5000)]
    match &module.items[6].kind {
        ItemKind::Fun(f) => {
            assert_eq!(f.attrs.len(), 3);
            assert_eq!(f.attrs[0].path_str(), "test");
            assert_eq!(f.attrs[1].path_str(), "should_panic");
            assert_eq!(f.attrs[2].path_str(), "timeout_ms");
        }
        other => panic!("expected Fun, got {other:?}"),
    }
}

#[test]
fn test_attribute_paths_and_module_attrs() {
    let src = r#"
#![lint::deny("unused_variable")]
#![target = "linux"]

#[lint::ignore("dead_code")]
#[json::rename = "user_name"]
#[custom::api::endpoint("POST", "/login")]
fun handle_login() {
    space
}
"#;
    let (module, diags) = parse_code(src);
    assert!(diags.is_empty(), "diags: {diags:?}");
    assert_eq!(module.module_attrs.len(), 2);
    assert_eq!(module.module_attrs[0].path_str(), "lint::deny");
    assert_eq!(module.module_attrs[0].path.segments.len(), 2);
    assert_eq!(module.module_attrs[0].path.segments[0].as_str(), "lint");
    assert_eq!(module.module_attrs[0].path.segments[1].as_str(), "deny");
    assert_eq!(module.module_attrs[1].path_str(), "target");

    assert_eq!(module.items.len(), 1);
    match &module.items[0].kind {
        ItemKind::Fun(f) => {
            assert_eq!(f.attrs.len(), 3);
            assert_eq!(f.attrs[0].path_str(), "lint::ignore");
            assert_eq!(f.attrs[1].path_str(), "json::rename");
            assert_eq!(f.attrs[2].path_str(), "custom::api::endpoint");
            assert_eq!(f.attrs[2].path.segments.len(), 3);
            assert_eq!(f.attrs[2].path.segments[0].as_str(), "custom");
            assert_eq!(f.attrs[2].path.segments[1].as_str(), "api");
            assert_eq!(f.attrs[2].path.segments[2].as_str(), "endpoint");
        }
        other => panic!("expected Fun, got {other:?}"),
    }
}
