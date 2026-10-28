#![deny(unsafe_code)]

use yukic_ast::*;
use yukic_diagnostics::code::{ErrorCode, WarningCode};
use yukic_span::{BytePos, FileId, Span};
use yukic_types::*;

fn dummy_span() -> Span {
    Span::new(FileId(0), BytePos(0), BytePos(0))
}

fn ident(name: &str) -> Ident {
    Ident::new(name, dummy_span())
}

#[test]
fn test_primitive_and_composite_types_display() {
    assert_eq!(Type::Int8.display_name(), "int8");
    assert_eq!(Type::Int32.display_name(), "int32");
    assert_eq!(Type::Int64.display_name(), "int64");
    assert_eq!(Type::Uint8.display_name(), "uint8");
    assert_eq!(Type::Uint64.display_name(), "uint64");
    assert_eq!(Type::Float64.display_name(), "float64");
    assert_eq!(Type::Float32.display_name(), "float32");
    assert_eq!(Type::Bool.display_name(), "bool");
    assert_eq!(Type::Char.display_name(), "char");
    assert_eq!(Type::String.display_name(), "string");
    assert_eq!(Type::Bytes.display_name(), "bytes");
    assert_eq!(Type::Void.display_name(), "void");
    assert_eq!(Type::Never.display_name(), "never");
    assert_eq!(Type::Null.display_name(), "null");

    let arr = Type::Array(Box::new(Type::Int64));
    assert_eq!(arr.display_name(), "int64[]");

    let map = Type::Map(Box::new(Type::String), Box::new(Type::Int32));
    assert_eq!(map.display_name(), "map<string, int32>");

    let tuple = Type::Tuple(vec![Type::Int64, Type::String]);
    assert_eq!(tuple.display_name(), "(int64, string)");

    let ptr = Type::Ptr(Box::new(Type::Uint8));
    assert_eq!(ptr.display_name(), "ptr<uint8>");

    let shared = Type::Shared(Box::new(Type::Int64));
    assert_eq!(shared.display_name(), "shared<int64>");

    let weak = Type::Weak(Box::new(Type::Int64));
    assert_eq!(weak.display_name(), "weak<int64>");
}

#[test]
fn test_union_flattening_and_deduplication() {
    let u1 = Type::union([Type::Int32, Type::Int64]);
    let u2 = Type::union([u1, Type::String, Type::Int32]);
    let expected = Type::union([Type::Int32, Type::Int64, Type::String]);
    assert_eq!(u2, expected);

    let single = Type::union([Type::Int64, Type::Int64]);
    assert_eq!(single, Type::Int64);

    let with_never = Type::union([Type::Int32, Type::Never]);
    assert_eq!(with_never, Type::Int32);
}

#[test]
fn test_nullable_canonicalization() {
    let n1 = Type::nullable(Type::Int64);
    assert_eq!(n1.display_name(), "int64?");
    assert!(n1.is_nullable());
    assert_eq!(n1.unwrap_nullable(), Type::Int64);

    let n2 = Type::nullable(n1.clone());
    assert_eq!(n2, n1);
}

#[test]
fn test_subtyping() {
    let env = TypeEnv::new();
    let is_impl = |c: &str, i: &str| env.is_interface_impl(c, i);

    assert!(Type::Int64.is_subtype_of(&Type::Int64, &is_impl));
    assert!(Type::Never.is_subtype_of(&Type::Int64, &is_impl));

    let nullable_int = Type::nullable(Type::Int64);
    assert!(Type::Int64.is_subtype_of(&nullable_int, &is_impl));
    assert!(Type::Null.is_subtype_of(&nullable_int, &is_impl));

    let union_type = Type::union([Type::Int64, Type::String]);
    assert!(Type::Int64.is_subtype_of(&union_type, &is_impl));
    assert!(Type::String.is_subtype_of(&union_type, &is_impl));
    assert!(!Type::Bool.is_subtype_of(&union_type, &is_impl));
}

#[test]
fn test_class_interface_subtyping() {
    let mut env = TypeEnv::new();
    let mut class_def = ClassDef::default();
    class_def.interfaces.insert("Printable".to_string());
    env.register_class("User".to_string(), class_def);

    let user_ty = Type::Class("User".to_string());
    let printable_ty = Type::Interface("Printable".to_string());

    let is_impl = |c: &str, i: &str| env.is_interface_impl(c, i);
    assert!(user_ty.is_subtype_of(&printable_ty, &is_impl));
    assert!(!Type::Int64.is_subtype_of(&printable_ty, &is_impl));
}

#[test]
fn test_check_integer_literals_range() {
    let mut checker = TypeChecker::new();

    let lit42 = Expr::new(ExprKind::Literal(LiteralValue::Int(42)), dummy_span());
    assert_eq!(checker.check_expr(&lit42, &Type::Int8).unwrap(), Type::Int8);
    assert_eq!(
        checker.check_expr(&lit42, &Type::Int16).unwrap(),
        Type::Int16
    );
    assert_eq!(
        checker.check_expr(&lit42, &Type::Int32).unwrap(),
        Type::Int32
    );
    assert_eq!(
        checker.check_expr(&lit42, &Type::Uint8).unwrap(),
        Type::Uint8
    );

    let lit300 = Expr::new(ExprKind::Literal(LiteralValue::Int(300)), dummy_span());
    let err = checker.check_expr(&lit300, &Type::Int8).unwrap_err();
    assert_eq!(err.code.as_str(), ErrorCode::E0401.as_str());

    let lit_neg = Expr::new(ExprKind::Literal(LiteralValue::Int(-10)), dummy_span());
    let err_neg = checker.check_expr(&lit_neg, &Type::Uint8).unwrap_err();
    assert_eq!(err_neg.code.as_str(), ErrorCode::E0401.as_str());
}

#[test]
fn test_infer_unannotated_literals() {
    let mut checker = TypeChecker::new();

    let int_e = Expr::new(ExprKind::Literal(LiteralValue::Int(42)), dummy_span());
    assert_eq!(checker.infer_expr(&int_e).unwrap(), Type::Int64);

    let float_e = Expr::new(ExprKind::Literal(LiteralValue::Float(3.5)), dummy_span());
    assert_eq!(checker.infer_expr(&float_e).unwrap(), Type::Float64);

    let str_e = Expr::new(
        ExprKind::Literal(LiteralValue::String("hello".to_string())),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&str_e).unwrap(), Type::String);

    let bool_e = Expr::new(ExprKind::Literal(LiteralValue::Bool(true)), dummy_span());
    assert_eq!(checker.infer_expr(&bool_e).unwrap(), Type::Bool);

    let char_e = Expr::new(ExprKind::Literal(LiteralValue::Char('x')), dummy_span());
    assert_eq!(checker.infer_expr(&char_e).unwrap(), Type::Char);

    let null_e = Expr::new(ExprKind::Literal(LiteralValue::Null), dummy_span());
    assert_eq!(checker.infer_expr(&null_e).unwrap(), Type::Null);
}

#[test]
fn test_infer_binary_operations() {
    let mut checker = TypeChecker::new();

    let a = Expr::new(ExprKind::Literal(LiteralValue::Int(10)), dummy_span());
    let b = Expr::new(ExprKind::Literal(LiteralValue::Int(20)), dummy_span());
    let add = Expr::new(
        ExprKind::Binary(BinaryOp::Add, Box::new(a), Box::new(b)),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&add).unwrap(), Type::Int64);

    let s1 = Expr::new(
        ExprKind::Literal(LiteralValue::String("a".to_string())),
        dummy_span(),
    );
    let s2 = Expr::new(
        ExprKind::Literal(LiteralValue::String("b".to_string())),
        dummy_span(),
    );
    let concat = Expr::new(
        ExprKind::Binary(BinaryOp::Add, Box::new(s1), Box::new(s2)),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&concat).unwrap(), Type::String);

    let eq_e = Expr::new(
        ExprKind::Binary(
            BinaryOp::Eq,
            Box::new(Expr::new(
                ExprKind::Literal(LiteralValue::Int(1)),
                dummy_span(),
            )),
            Box::new(Expr::new(
                ExprKind::Literal(LiteralValue::Int(2)),
                dummy_span(),
            )),
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&eq_e).unwrap(), Type::Bool);
}

#[test]
fn test_variable_type_fixation() {
    let mut checker = TypeChecker::new();

    let set_stmt = Stmt::new(
        StmtKind::Set {
            target: BindingTarget::Ident(ident("x")),
            ty: None,
            init: Expr::new(ExprKind::Literal(LiteralValue::Int(42)), dummy_span()),
        },
        dummy_span(),
    );
    assert!(checker.check_stmt(&set_stmt).is_ok());

    let rewrite_ok = Stmt::new(
        StmtKind::Rewrite {
            target: LValue::Ident(ident("x")),
            op: AssignOp::Eq,
            value: Expr::new(ExprKind::Literal(LiteralValue::Int(100)), dummy_span()),
        },
        dummy_span(),
    );
    assert!(checker.check_stmt(&rewrite_ok).is_ok());

    let rewrite_err = Stmt::new(
        StmtKind::Rewrite {
            target: LValue::Ident(ident("x")),
            op: AssignOp::Eq,
            value: Expr::new(
                ExprKind::Literal(LiteralValue::String("mismatch".to_string())),
                dummy_span(),
            ),
        },
        dummy_span(),
    );
    let err = checker.check_stmt(&rewrite_err).unwrap_err();
    assert_eq!(err.code.as_str(), ErrorCode::E0401.as_str());
}

#[test]
fn test_compiler_builtins() {
    let mut checker = TypeChecker::new();

    let call_typeof = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("__typeof_intrinsic__")), dummy_span())),
            vec![Expr::new(
                ExprKind::Literal(LiteralValue::Int(42)),
                dummy_span(),
            )],
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&call_typeof).unwrap(), Type::String);

    let direct_typeof = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("typeof")), dummy_span())),
            vec![Expr::new(
                ExprKind::Literal(LiteralValue::Int(42)),
                dummy_span(),
            )],
        ),
        dummy_span(),
    );
    let err = checker.infer_expr(&direct_typeof).unwrap_err();
    assert!(err.message.contains("typeof!"));

    let call_args = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("args")), dummy_span())),
            vec![],
        ),
        dummy_span(),
    );
    assert_eq!(
        checker.infer_expr(&call_args).unwrap(),
        Type::Array(Box::new(Type::String))
    );

    let call_raw_args = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("raw_args")), dummy_span())),
            vec![],
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&call_raw_args).unwrap(), Type::String);

    let call_exit = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("exit")), dummy_span())),
            vec![Expr::new(
                ExprKind::Literal(LiteralValue::Int(0)),
                dummy_span(),
            )],
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&call_exit).unwrap(), Type::Never);
}

#[test]
fn test_mro_inherent_and_inheritance() {
    let mut checker = TypeChecker::new();

    let mut animal_def = ClassDef::default();
    animal_def.methods.insert(
        "speak".to_string(),
        FnSig {
            params: vec![TypeParam::value(Type::Class("Animal".to_string()))],
            ret: Type::String,
            is_cfun: false,
        },
    );
    checker.env.register_class("Animal".to_string(), animal_def);

    let mut dog_def = ClassDef {
        superclass: Some("Animal".to_string()),
        ..Default::default()
    };
    dog_def.methods.insert(
        "bark".to_string(),
        FnSig {
            params: vec![TypeParam::value(Type::Class("Dog".to_string()))],
            ret: Type::String,
            is_cfun: false,
        },
    );
    checker.env.register_class("Dog".to_string(), dog_def);

    checker.env.insert_var(
        "my_dog".to_string(),
        Type::Class("Dog".to_string()),
        false,
        dummy_span(),
    );

    let dog_expr = Expr::new(ExprKind::Ident(ident("my_dog")), dummy_span());
    let call_bark = Expr::new(
        ExprKind::MethodCall(Box::new(dog_expr.clone()), ident("bark"), vec![]),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&call_bark).unwrap(), Type::String);

    let call_speak = Expr::new(
        ExprKind::MethodCall(Box::new(dog_expr.clone()), ident("speak"), vec![]),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&call_speak).unwrap(), Type::String);

    let call_fly = Expr::new(
        ExprKind::MethodCall(Box::new(dog_expr), ident("fly"), vec![]),
        dummy_span(),
    );
    let err = checker.infer_expr(&call_fly).unwrap_err();
    assert_eq!(err.code.as_str(), ErrorCode::E0403.as_str());
}

#[test]
fn test_mro_ufcs_and_qualified_calls() {
    let mut checker = TypeChecker::new();

    checker.env.register_fn(
        "is_positive".to_string(),
        FnSig {
            params: vec![TypeParam::value(Type::Int64)],
            ret: Type::Bool,
            is_cfun: false,
        },
    );

    let num_expr = Expr::new(ExprKind::Literal(LiteralValue::Int(10)), dummy_span());
    let ufcs_call = Expr::new(
        ExprKind::MethodCall(Box::new(num_expr), ident("is_positive"), vec![]),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&ufcs_call).unwrap(), Type::Bool);

    checker.env.register_fn(
        "strings::trim".to_string(),
        FnSig {
            params: vec![TypeParam::value(Type::String)],
            ret: Type::String,
            is_cfun: false,
        },
    );

    let str_expr = Expr::new(
        ExprKind::Literal(LiteralValue::String(" hello ".to_string())),
        dummy_span(),
    );
    let q_call = Expr::new(
        ExprKind::QualifiedMethodCall(Box::new(str_expr), ident("strings"), ident("trim"), vec![]),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&q_call).unwrap(), Type::String);
}

#[test]
fn test_safe_call_nullable() {
    let mut checker = TypeChecker::new();

    let mut user_def = ClassDef::default();
    user_def.methods.insert(
        "name".to_string(),
        FnSig {
            params: vec![TypeParam::value(Type::Class("User".to_string()))],
            ret: Type::String,
            is_cfun: false,
        },
    );
    checker.env.register_class("User".to_string(), user_def);

    checker.env.insert_var(
        "opt_user".to_string(),
        Type::nullable(Type::Class("User".to_string())),
        false,
        dummy_span(),
    );

    let opt_expr = Expr::new(ExprKind::Ident(ident("opt_user")), dummy_span());
    let safe_call = Expr::new(
        ExprKind::SafeCall(Box::new(opt_expr), ident("name"), vec![]),
        dummy_span(),
    );
    assert_eq!(
        checker.infer_expr(&safe_call).unwrap(),
        Type::nullable(Type::String)
    );
}

#[test]
fn test_switch_exhaustiveness_enum() {
    let mut checker = TypeChecker::new();

    let mut color_def = EnumDef::default();
    color_def.variants.insert("Red".to_string(), None);
    color_def.variants.insert("Green".to_string(), None);
    color_def.variants.insert("Blue".to_string(), None);
    checker.env.register_enum("Color".to_string(), color_def);

    checker.env.insert_var(
        "c".to_string(),
        Type::Enum("Color".to_string()),
        false,
        dummy_span(),
    );

    let target = Expr::new(ExprKind::Ident(ident("c")), dummy_span());
    let make_arm = |name: &str| SwitchArm {
        pattern: Pattern::new(
            PatternKind::Variant(None, ident(name), vec![]),
            dummy_span(),
        ),
        guard: None,
        body: Block::new(vec![], dummy_span()),
        span: dummy_span(),
    };

    let switch_exhaustive = Expr::new(
        ExprKind::Switch {
            target: Box::new(target.clone()),
            arms: vec![make_arm("Red"), make_arm("Green"), make_arm("Blue")],
        },
        dummy_span(),
    );
    assert!(checker.infer_expr(&switch_exhaustive).is_ok());

    let switch_non_exhaustive = Expr::new(
        ExprKind::Switch {
            target: Box::new(target.clone()),
            arms: vec![make_arm("Red"), make_arm("Green")],
        },
        dummy_span(),
    );
    let err = checker.infer_expr(&switch_non_exhaustive).unwrap_err();
    assert_eq!(err.code.as_str(), ErrorCode::E0405.as_str());

    let switch_unreachable = Expr::new(
        ExprKind::Switch {
            target: Box::new(target),
            arms: vec![
                SwitchArm {
                    pattern: Pattern::new(PatternKind::Wildcard, dummy_span()),
                    guard: None,
                    body: Block::new(vec![], dummy_span()),
                    span: dummy_span(),
                },
                make_arm("Red"),
            ],
        },
        dummy_span(),
    );
    let err_unreachable = checker.infer_expr(&switch_unreachable).unwrap_err();
    assert_eq!(err_unreachable.code.as_str(), ErrorCode::E0406.as_str());
}

#[test]
fn test_switch_exhaustiveness_bool() {
    let mut checker = TypeChecker::new();

    let target = Expr::new(ExprKind::Literal(LiteralValue::Bool(true)), dummy_span());
    let make_arm = |b: bool| SwitchArm {
        pattern: Pattern::new(PatternKind::Literal(LiteralValue::Bool(b)), dummy_span()),
        guard: None,
        body: Block::new(vec![], dummy_span()),
        span: dummy_span(),
    };

    let switch_ok = Expr::new(
        ExprKind::Switch {
            target: Box::new(target.clone()),
            arms: vec![make_arm(true), make_arm(false)],
        },
        dummy_span(),
    );
    assert!(checker.infer_expr(&switch_ok).is_ok());

    let switch_err = Expr::new(
        ExprKind::Switch {
            target: Box::new(target),
            arms: vec![make_arm(true)],
        },
        dummy_span(),
    );
    let err = checker.infer_expr(&switch_err).unwrap_err();
    assert_eq!(err.code.as_str(), ErrorCode::E0405.as_str());
}

#[test]
fn test_cfun_c_abi_validation() {
    let cfun_valid = CfunDecl {
        name: ident("add"),
        c_attrs: Vec::new(),
        yukino_attrs: Vec::new(),
        params: vec![CParam {
            name: ident("x"),
            ty: TypeNode::new(
                yukic_ast::types::TypeNodeKind::Ident(ident("int32")),
                dummy_span(),
            ),
            span: dummy_span(),
        }],
        return_type: Some(TypeNode::new(
            yukic_ast::types::TypeNodeKind::Ident(ident("int32")),
            dummy_span(),
        )),
        symbol_override: None,
        body: None,
        span: dummy_span(),
    };
    assert!(SemanticContracts::validate_cfun(&cfun_valid).is_ok());

    let cfun_invalid = CfunDecl {
        name: ident("invalid_cfun"),
        c_attrs: Vec::new(),
        yukino_attrs: Vec::new(),
        params: vec![CParam {
            name: ident("user"),
            ty: TypeNode::new(
                yukic_ast::types::TypeNodeKind::Ident(ident("User")),
                dummy_span(),
            ),
            span: dummy_span(),
        }],
        return_type: None,
        symbol_override: None,
        body: None,
        span: dummy_span(),
    };
    let err = SemanticContracts::validate_cfun(&cfun_invalid).unwrap_err();
    assert_eq!(err.code.as_str(), ErrorCode::E0701.as_str());
}

#[test]
fn test_recursive_type_aliases() {
    let alias_a = TypeAliasDecl {
        name: ident("A"),
        generics: vec![],
        target: TypeNode::new(
            yukic_ast::types::TypeNodeKind::Ident(ident("B")),
            dummy_span(),
        ),
        span: dummy_span(),
    };
    let alias_b = TypeAliasDecl {
        name: ident("B"),
        generics: vec![],
        target: TypeNode::new(
            yukic_ast::types::TypeNodeKind::Ident(ident("A")),
            dummy_span(),
        ),
        span: dummy_span(),
    };

    let err = SemanticContracts::check_alias_cycles(&[alias_a, alias_b]).unwrap_err();
    assert_eq!(err.code.as_str(), ErrorCode::E0407.as_str());
}

#[test]
fn test_cast_semantics() {
    let mut checker = TypeChecker::new();

    let cast_num = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("cast")), dummy_span())),
            vec![
                Expr::new(ExprKind::Literal(LiteralValue::Int(42)), dummy_span()),
                Expr::new(
                    ExprKind::Literal(LiteralValue::String("int32".to_string())),
                    dummy_span(),
                ),
            ],
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&cast_num).unwrap(), Type::Int32);

    let cast_str = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("cast")), dummy_span())),
            vec![
                Expr::new(ExprKind::Literal(LiteralValue::Int(123)), dummy_span()),
                Expr::new(
                    ExprKind::Literal(LiteralValue::String("string".to_string())),
                    dummy_span(),
                ),
            ],
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&cast_str).unwrap(), Type::String);

    let cast_char = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("cast")), dummy_span())),
            vec![
                Expr::new(ExprKind::Literal(LiteralValue::Int(116)), dummy_span()),
                Expr::new(
                    ExprKind::Literal(LiteralValue::String("char".to_string())),
                    dummy_span(),
                ),
            ],
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&cast_char).unwrap(), Type::Char);

    let cast_str_to_chars = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("cast")), dummy_span())),
            vec![
                Expr::new(
                    ExprKind::Literal(LiteralValue::String("hello".to_string())),
                    dummy_span(),
                ),
                Expr::new(
                    ExprKind::Literal(LiteralValue::String("char[]".to_string())),
                    dummy_span(),
                ),
            ],
        ),
        dummy_span(),
    );
    assert_eq!(
        checker.infer_expr(&cast_str_to_chars).unwrap(),
        Type::Array(Box::new(Type::Char))
    );

    let cast_bool_to_str = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("cast")), dummy_span())),
            vec![
                Expr::new(ExprKind::Literal(LiteralValue::Bool(true)), dummy_span()),
                Expr::new(
                    ExprKind::Literal(LiteralValue::String("string".to_string())),
                    dummy_span(),
                ),
            ],
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&cast_bool_to_str).unwrap(), Type::String);

    let cast_str_to_bool = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("cast")), dummy_span())),
            vec![
                Expr::new(
                    ExprKind::Literal(LiteralValue::String("true".to_string())),
                    dummy_span(),
                ),
                Expr::new(
                    ExprKind::Literal(LiteralValue::String("bool".to_string())),
                    dummy_span(),
                ),
            ],
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&cast_str_to_bool).unwrap(), Type::Bool);

    let cast_not_const = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("cast")), dummy_span())),
            vec![
                Expr::new(ExprKind::Literal(LiteralValue::Int(42)), dummy_span()),
                Expr::new(ExprKind::Ident(ident("dynamic_target")), dummy_span()),
            ],
        ),
        dummy_span(),
    );
    let err_const = checker.infer_expr(&cast_not_const).unwrap_err();
    assert_eq!(err_const.code.as_str(), ErrorCode::E0431.as_str());

    let cast_overflow = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("cast")), dummy_span())),
            vec![
                Expr::new(
                    ExprKind::Literal(LiteralValue::Int(5_000_000_000)),
                    dummy_span(),
                ),
                Expr::new(
                    ExprKind::Literal(LiteralValue::String("int32".to_string())),
                    dummy_span(),
                ),
            ],
        ),
        dummy_span(),
    );
    let err_overflow = checker.infer_expr(&cast_overflow).unwrap_err();
    assert_eq!(err_overflow.code.as_str(), ErrorCode::E0430.as_str());
}

#[test]
fn test_test_modifiers_and_stable_contracts() {
    let fun_without_test = FunDecl {
        name: ident("panic_helper"),
        attrs: vec![AttributeNode::new(
            ident("should_panic"),
            AttributeArgs::None,
            dummy_span(),
        )],
        generics: vec![],
        params: vec![],
        return_type: None,
        body: Block::new(vec![], dummy_span()),
        span: dummy_span(),
    };
    let err = SemanticContracts::validate_test_modifiers(&fun_without_test).unwrap_err();
    assert_eq!(err.code.as_str(), ErrorCode::E0429.as_str());

    let fun_with_test = FunDecl {
        name: ident("test_panic"),
        attrs: vec![
            AttributeNode::new(ident("test"), AttributeArgs::None, dummy_span()),
            AttributeNode::new(ident("should_panic"), AttributeArgs::None, dummy_span()),
        ],
        generics: vec![],
        params: vec![],
        return_type: None,
        body: Block::new(vec![], dummy_span()),
        span: dummy_span(),
    };
    assert!(SemanticContracts::validate_test_modifiers(&fun_with_test).is_ok());

    let stable_fun = FunDecl {
        name: ident("pure_fn"),
        attrs: vec![AttributeNode::new(
            ident("stable"),
            AttributeArgs::None,
            dummy_span(),
        )],
        generics: vec![],
        params: vec![],
        return_type: None,
        body: Block::new(vec![], dummy_span()),
        span: dummy_span(),
    };
    let stable_err = SemanticContracts::validate_stable_decl(&stable_fun, true).unwrap_err();
    assert_eq!(stable_err.code.as_str(), ErrorCode::E0417.as_str());
}

#[test]
fn test_suggest_switch_warning() {
    let mut checker = TypeChecker::new();
    checker
        .env
        .insert_var("status".to_string(), Type::Int64, false, dummy_span());

    let if_cond = Expr::new(
        ExprKind::Binary(
            BinaryOp::Eq,
            Box::new(Expr::new(ExprKind::Ident(ident("status")), dummy_span())),
            Box::new(Expr::new(
                ExprKind::Literal(LiteralValue::Int(1)),
                dummy_span(),
            )),
        ),
        dummy_span(),
    );
    let elif1 = (
        Expr::new(
            ExprKind::Binary(
                BinaryOp::Eq,
                Box::new(Expr::new(ExprKind::Ident(ident("status")), dummy_span())),
                Box::new(Expr::new(
                    ExprKind::Literal(LiteralValue::Int(2)),
                    dummy_span(),
                )),
            ),
            dummy_span(),
        ),
        Block::new(vec![], dummy_span()),
    );
    let elif2 = (
        Expr::new(
            ExprKind::Binary(
                BinaryOp::Eq,
                Box::new(Expr::new(ExprKind::Ident(ident("status")), dummy_span())),
                Box::new(Expr::new(
                    ExprKind::Literal(LiteralValue::Int(3)),
                    dummy_span(),
                )),
            ),
            dummy_span(),
        ),
        Block::new(vec![], dummy_span()),
    );

    let if_expr = Expr::new(
        ExprKind::If {
            cond: Box::new(if_cond),
            then_branch: Block::new(vec![], dummy_span()),
            elif_branches: vec![elif1, elif2],
            else_branch: Some(Block::new(vec![], dummy_span())),
        },
        dummy_span(),
    );

    let res = checker.infer_expr(&if_expr);
    assert!(res.is_ok());
    assert!(
        checker
            .diagnostics
            .iter()
            .any(|d| d.code.as_str() == WarningCode::W0415.as_str())
    );
}

#[test]
fn test_panic_type_inference() {
    let mut checker = TypeChecker::new();

    let mut math_err = StructDef::default();
    math_err.fields.insert("detail".to_string(), Type::String);
    checker
        .env
        .register_struct("MathError".to_string(), math_err);

    let call_struct = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("panic")), dummy_span())),
            vec![Expr::new(
                ExprKind::StructInit(ident("MathError"), vec![]),
                dummy_span(),
            )],
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&call_struct).unwrap(), Type::Never);

    let call_str = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("panic")), dummy_span())),
            vec![Expr::new(
                ExprKind::Literal(LiteralValue::String("failed".to_string())),
                dummy_span(),
            )],
        ),
        dummy_span(),
    );
    assert!(checker.infer_expr(&call_str).is_err());

    let call_empty = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("panic")), dummy_span())),
            vec![],
        ),
        dummy_span(),
    );
    assert!(checker.infer_expr(&call_empty).is_err());
}

#[test]
fn test_field_access_inference() {
    let mut checker = TypeChecker::new();

    let mut struct_def = StructDef::default();
    struct_def.fields.insert("code".to_string(), Type::Int32);
    struct_def
        .fields
        .insert("message".to_string(), Type::String);
    checker
        .env
        .register_struct("CustomError".to_string(), struct_def);

    checker.env.insert_var(
        "err".to_string(),
        Type::Struct("CustomError".to_string()),
        false,
        dummy_span(),
    );

    let access_code = Expr::new(
        ExprKind::FieldAccess(
            Box::new(Expr::new(ExprKind::Ident(ident("err")), dummy_span())),
            ident("code"),
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&access_code).unwrap(), Type::Int32);

    let access_missing = Expr::new(
        ExprKind::FieldAccess(
            Box::new(Expr::new(ExprKind::Ident(ident("err")), dummy_span())),
            ident("non_existent"),
        ),
        dummy_span(),
    );
    assert!(checker.infer_expr(&access_missing).is_err());
}

#[test]
fn test_sus_catch_inference() {
    let mut checker = TypeChecker::new();

    let mut math_err = StructDef::default();
    math_err.fields.insert("detail".to_string(), Type::String);
    checker
        .env
        .register_struct("MathError".to_string(), math_err);

    let body_expr = Expr::new(ExprKind::Literal(LiteralValue::Int(42)), dummy_span());
    let body = Block::new(
        vec![Stmt::new(StmtKind::Expr(body_expr), dummy_span())],
        dummy_span(),
    );

    let catch1_body_expr = Expr::new(ExprKind::Literal(LiteralValue::Int(0)), dummy_span());
    let catch1 = CatchClause {
        error_var: Some(ident("e")),
        error_type: Some(TypeNode::new(
            TypeNodeKind::Ident(ident("MathError")),
            dummy_span(),
        )),
        body: Block::new(
            vec![Stmt::new(StmtKind::Expr(catch1_body_expr), dummy_span())],
            dummy_span(),
        ),
        span: dummy_span(),
    };

    let catch2_body_expr = Expr::new(ExprKind::Literal(LiteralValue::Int(1)), dummy_span());
    let catch2 = CatchClause {
        error_var: None,
        error_type: None,
        body: Block::new(
            vec![Stmt::new(StmtKind::Expr(catch2_body_expr), dummy_span())],
            dummy_span(),
        ),
        span: dummy_span(),
    };

    let sus_expr = Expr::new(
        ExprKind::Sus {
            body,
            catches: vec![catch1, catch2],
            finally_branch: None,
        },
        dummy_span(),
    );

    assert_eq!(checker.infer_expr(&sus_expr).unwrap(), Type::Int64);
}

#[test]
fn test_sus_catch_field_access() {
    let mut checker = TypeChecker::new();

    let mut math_err = StructDef::default();
    math_err.fields.insert("detail".to_string(), Type::String);
    checker
        .env
        .register_struct("MathError".to_string(), math_err);

    let body_expr = Expr::new(
        ExprKind::Literal(LiteralValue::String("ok".to_string())),
        dummy_span(),
    );
    let body = Block::new(
        vec![Stmt::new(StmtKind::Expr(body_expr), dummy_span())],
        dummy_span(),
    );

    let catch1_field = Expr::new(
        ExprKind::FieldAccess(
            Box::new(Expr::new(ExprKind::Ident(ident("e")), dummy_span())),
            ident("detail"),
        ),
        dummy_span(),
    );
    let catch1 = CatchClause {
        error_var: Some(ident("e")),
        error_type: Some(TypeNode::new(
            TypeNodeKind::Ident(ident("MathError")),
            dummy_span(),
        )),
        body: Block::new(
            vec![Stmt::new(StmtKind::Expr(catch1_field), dummy_span())],
            dummy_span(),
        ),
        span: dummy_span(),
    };

    let catch2_body = Expr::new(
        ExprKind::Literal(LiteralValue::String("fallback".to_string())),
        dummy_span(),
    );
    let catch2 = CatchClause {
        error_var: None,
        error_type: None,
        body: Block::new(
            vec![Stmt::new(StmtKind::Expr(catch2_body), dummy_span())],
            dummy_span(),
        ),
        span: dummy_span(),
    };

    let sus_expr = Expr::new(
        ExprKind::Sus {
            body,
            catches: vec![catch1, catch2],
            finally_branch: None,
        },
        dummy_span(),
    );

    assert_eq!(checker.infer_expr(&sus_expr).unwrap(), Type::String);
}

#[test]
fn test_collection_intrinsic_methods_and_literal_coercion() {
    let mut checker = TypeChecker::new();

    // 1. Array len, cap, as_ptr
    checker.env.insert_var(
        "arr".to_string(),
        Type::Array(Box::new(Type::Int32)),
        false,
        dummy_span(),
    );
    let arr_expr = Expr::new(ExprKind::Ident(ident("arr")), dummy_span());

    let arr_len = Expr::new(
        ExprKind::MethodCall(Box::new(arr_expr.clone()), ident("len"), vec![]),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&arr_len).unwrap(), Type::Int64);

    let arr_cap = Expr::new(
        ExprKind::MethodCall(Box::new(arr_expr.clone()), ident("cap"), vec![]),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&arr_cap).unwrap(), Type::Int64);

    let arr_ptr = Expr::new(
        ExprKind::MethodCall(Box::new(arr_expr), ident("as_ptr"), vec![]),
        dummy_span(),
    );
    assert_eq!(
        checker.infer_expr(&arr_ptr).unwrap(),
        Type::Ptr(Box::new(Type::Int32))
    );

    // 2. String len and as_ptr
    let str_expr = Expr::new(
        ExprKind::Literal(LiteralValue::String("hello".to_string())),
        dummy_span(),
    );
    let str_len = Expr::new(
        ExprKind::MethodCall(Box::new(str_expr.clone()), ident("len"), vec![]),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&str_len).unwrap(), Type::Int64);

    let str_ptr = Expr::new(
        ExprKind::MethodCall(Box::new(str_expr), ident("as_ptr"), vec![]),
        dummy_span(),
    );
    assert_eq!(
        checker.infer_expr(&str_ptr).unwrap(),
        Type::Ptr(Box::new(Type::Uint8))
    );

    // 3. Literal coercion in binary operations: b (int32) == 0 (int literal)
    checker
        .env
        .insert_var("b".to_string(), Type::Int32, false, dummy_span());
    let b_expr = Expr::new(ExprKind::Ident(ident("b")), dummy_span());
    let zero_expr = Expr::new(ExprKind::Literal(LiteralValue::Int(0)), dummy_span());
    let eq_expr = Expr::new(
        ExprKind::Binary(BinaryOp::Eq, Box::new(b_expr), Box::new(zero_expr)),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&eq_expr).unwrap(), Type::Bool);
}

#[test]
fn test_ptr_of_and_indexing_inference() {
    let mut checker = TypeChecker::new();

    // 1. ptr_of(x)
    checker
        .env
        .insert_var("num".to_string(), Type::Int32, false, dummy_span());
    let ptr_of_expr = Expr::new(ExprKind::PtrOf(ident("num")), dummy_span());
    assert_eq!(
        checker.infer_expr(&ptr_of_expr).unwrap(),
        Type::Ptr(Box::new(Type::Int32))
    );

    // 2. IndexAccess: array[i] -> T, string[i] -> uint8
    checker.env.insert_var(
        "arr".to_string(),
        Type::Array(Box::new(Type::Int32)),
        false,
        dummy_span(),
    );
    let idx_expr = Expr::new(ExprKind::Literal(LiteralValue::Int(0)), dummy_span());
    let arr_idx = Expr::new(
        ExprKind::IndexAccess(
            Box::new(Expr::new(ExprKind::Ident(ident("arr")), dummy_span())),
            Box::new(idx_expr.clone()),
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&arr_idx).unwrap(), Type::Int32);

    let str_expr = Expr::new(
        ExprKind::Literal(LiteralValue::String("abc".to_string())),
        dummy_span(),
    );
    let str_idx = Expr::new(
        ExprKind::IndexAccess(Box::new(str_expr.clone()), Box::new(idx_expr.clone())),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&str_idx).unwrap(), Type::Uint8);

    // 3. SliceAccess: arr[0..2] -> int32[], str[0..2] -> string
    let slice_end = Expr::new(ExprKind::Literal(LiteralValue::Int(2)), dummy_span());
    let arr_slice = Expr::new(
        ExprKind::SliceAccess(
            Box::new(Expr::new(ExprKind::Ident(ident("arr")), dummy_span())),
            Some(Box::new(idx_expr.clone())),
            Some(Box::new(slice_end.clone())),
            false,
        ),
        dummy_span(),
    );
    assert_eq!(
        checker.infer_expr(&arr_slice).unwrap(),
        Type::Array(Box::new(Type::Int32))
    );

    let str_slice = Expr::new(
        ExprKind::SliceAccess(
            Box::new(str_expr),
            Some(Box::new(idx_expr)),
            Some(Box::new(slice_end)),
            false,
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&str_slice).unwrap(), Type::String);
}

#[test]
fn test_raw_parts_and_assert_inference() {
    let mut checker = TypeChecker::new();

    // 1. string::from_raw_parts(ptr, len)
    checker.env.insert_var(
        "p_u8".to_string(),
        Type::Ptr(Box::new(Type::Uint8)),
        false,
        dummy_span(),
    );
    checker
        .env
        .insert_var("len_val".to_string(), Type::Int64, false, dummy_span());
    let from_raw_expr = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(
                ExprKind::Path(vec![ident("string"), ident("from_raw_parts")]),
                dummy_span(),
            )),
            vec![
                Expr::new(ExprKind::Ident(ident("p_u8")), dummy_span()),
                Expr::new(ExprKind::Ident(ident("len_val")), dummy_span()),
            ],
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&from_raw_expr).unwrap(), Type::String);

    // 2. assert_eq(a, b) -> Void
    let assert_call = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("assert_eq")), dummy_span())),
            vec![
                Expr::new(ExprKind::Literal(LiteralValue::Int(1)), dummy_span()),
                Expr::new(ExprKind::Literal(LiteralValue::Int(1)), dummy_span()),
            ],
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&assert_call).unwrap(), Type::Void);
}

#[test]
fn test_static_module_and_local_registration() {
    let mut checker = TypeChecker::new();
    let static_item = Item {
        id: ItemId(1),
        visibility: Visibility::Public,
        attrs: Vec::new(),
        kind: ItemKind::Static(StaticDecl {
            name: ident("GLOBAL_NUM"),
            ty: Some(TypeNode::new(
                yukic_ast::types::TypeNodeKind::Ident(ident("int32")),
                dummy_span(),
            )),
            init: Expr::new(
                ExprKind::Unary(
                    UnaryOp::Neg,
                    Box::new(Expr::new(ExprKind::Literal(LiteralValue::Int(10)), dummy_span())),
                ),
                dummy_span(),
            ),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    let mut module = Module::new(FileId(0));
    module.items.push(static_item);
    assert!(checker.check_module(&module).is_ok());
    let var = checker.env.lookup_var("GLOBAL_NUM").unwrap();
    assert_eq!(var.ty, Type::Int32);
    assert!(var.is_static);
}

#[test]
fn test_static_reassignment_error() {
    let mut checker = TypeChecker::new();
    let static_stmt = Stmt {
        kind: StmtKind::Static(StaticDecl {
            name: ident("MAX_VAL"),
            ty: None,
            init: Expr::new(ExprKind::Literal(LiteralValue::Int(100)), dummy_span()),
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    assert!(checker.check_stmt(&static_stmt).is_ok());

    let rewrite_stmt = Stmt {
        kind: StmtKind::Rewrite {
            target: LValue::Ident(ident("MAX_VAL")),
            op: AssignOp::Eq,
            value: Expr::new(ExprKind::Literal(LiteralValue::Int(200)), dummy_span()),
        },
        span: dummy_span(),
    };
    let err = checker.check_stmt(&rewrite_stmt).unwrap_err();
    assert_eq!(
        err.code,
        yukic_diagnostics::DiagnosticCode::Error(ErrorCode::E0503)
    );
}

#[test]
fn test_nullable_operations_and_safe_field_access() {
    let mut checker = TypeChecker::new();

    // Register User struct
    let mut struct_def = StructDef::default();
    struct_def.fields.insert("name".to_string(), Type::String);
    struct_def.fields.insert("age".to_string(), Type::Int64);
    checker
        .env
        .register_struct("User".to_string(), struct_def);

    checker.env.insert_var(
        "opt_int".to_string(),
        Type::nullable(Type::Int64),
        false,
        dummy_span(),
    );
    checker.env.insert_var(
        "opt_user".to_string(),
        Type::nullable(Type::Struct("User".to_string())),
        false,
        dummy_span(),
    );
    checker.env.insert_var(
        "raw_ptr".to_string(),
        Type::Ptr(Box::new(Type::Int32)),
        false,
        dummy_span(),
    );

    // 1. opt_int == null
    let eq_null = Expr::new(
        ExprKind::Binary(
            BinaryOp::Eq,
            Box::new(Expr::new(ExprKind::Ident(ident("opt_int")), dummy_span())),
            Box::new(Expr::new(
                ExprKind::Literal(LiteralValue::Null),
                dummy_span(),
            )),
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&eq_null).unwrap(), Type::Bool);

    // 2. raw_ptr != null
    let ne_null = Expr::new(
        ExprKind::Binary(
            BinaryOp::NotEq,
            Box::new(Expr::new(ExprKind::Ident(ident("raw_ptr")), dummy_span())),
            Box::new(Expr::new(
                ExprKind::Literal(LiteralValue::Null),
                dummy_span(),
            )),
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&ne_null).unwrap(), Type::Bool);

    // 3. opt_int ?? 0
    let coalesce = Expr::new(
        ExprKind::Binary(
            BinaryOp::NullCoalesce,
            Box::new(Expr::new(ExprKind::Ident(ident("opt_int")), dummy_span())),
            Box::new(Expr::new(ExprKind::Literal(LiteralValue::Int(0)), dummy_span())),
        ),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&coalesce).unwrap(), Type::Int64);

    // 4. opt_user?.name (safe field access)
    let safe_field = Expr::new(
        ExprKind::SafeCall(
            Box::new(Expr::new(ExprKind::Ident(ident("opt_user")), dummy_span())),
            ident("name"),
            vec![],
        ),
        dummy_span(),
    );
    assert_eq!(
        checker.infer_expr(&safe_field).unwrap(),
        Type::nullable(Type::String)
    );
}

#[test]
fn test_defer_type_checking() {
    let mut checker = TypeChecker::new();
    let dspan = dummy_span();

    // defer { set x = 10; }
    let defer_stmt = Stmt::new(
        StmtKind::Defer(Block::new(
            vec![Stmt::new(
                StmtKind::Set {
                    target: BindingTarget::Ident(ident("x")),
                    ty: None,
                    init: Expr::new(ExprKind::Literal(LiteralValue::Int(10)), dspan),
                },
                dspan,
            )],
            dspan,
        )),
        dspan,
    );
    assert!(checker.check_stmt(&defer_stmt).is_ok());
}

#[test]
fn test_super_type_checking() {
    let mut checker = TypeChecker::new();
    let dspan = dummy_span();

    let mut animal_def = ClassDef::default();
    animal_def.fields.insert("age".to_string(), Type::Int64);
    animal_def.methods.insert(
        "init".to_string(),
        FnSig {
            params: vec![
                TypeParam::value(Type::Class("Animal".to_string())),
                TypeParam::value(Type::Int64),
            ],
            ret: Type::Void,
            is_cfun: false,
        },
    );
    animal_def.methods.insert(
        "speak".to_string(),
        FnSig {
            params: vec![TypeParam::value(Type::Class("Animal".to_string()))],
            ret: Type::String,
            is_cfun: false,
        },
    );
    checker.env.register_class("Animal".to_string(), animal_def);

    let mut dog_def = ClassDef::default();
    dog_def.superclass = Some("Animal".to_string());
    dog_def.methods.insert(
        "speak".to_string(),
        FnSig {
            params: vec![TypeParam::value(Type::Class("Dog".to_string()))],
            ret: Type::String,
            is_cfun: false,
        },
    );
    checker.env.register_class("Dog".to_string(), dog_def);

    checker.current_class = Some("Dog".to_string());

    let super_expr = Expr::new(ExprKind::Super, dspan);
    assert_eq!(
        checker.infer_expr(&super_expr).unwrap(),
        Type::Class("Animal".to_string())
    );

    let super_call = Expr::new(
        ExprKind::MethodCall(Box::new(super_expr.clone()), ident("speak"), vec![]),
        dspan,
    );
    assert_eq!(checker.infer_expr(&super_call).unwrap(), Type::String);

    let super_field = Expr::new(
        ExprKind::FieldAccess(Box::new(super_expr.clone()), ident("age")),
        dspan,
    );
    assert_eq!(checker.infer_expr(&super_field).unwrap(), Type::Int64);

    let super_init = Expr::new(
        ExprKind::Call(
            Box::new(super_expr.clone()),
            vec![Expr::new(ExprKind::Literal(LiteralValue::Int(42)), dspan)],
        ),
        dspan,
    );
    assert_eq!(checker.infer_expr(&super_init).unwrap(), Type::Void);

    let bad_method = Expr::new(
        ExprKind::MethodCall(Box::new(super_expr.clone()), ident("fly"), vec![]),
        dspan,
    );
    let err = checker.infer_expr(&bad_method).unwrap_err();
    assert_eq!(err.code.as_str(), ErrorCode::E0416.as_str());

    let bad_field = Expr::new(
        ExprKind::FieldAccess(Box::new(super_expr.clone()), ident("wings")),
        dspan,
    );
    let err = checker.infer_expr(&bad_field).unwrap_err();
    assert_eq!(err.code.as_str(), ErrorCode::E0416.as_str());

    checker.current_class = Some("Animal".to_string());
    let err = checker.infer_expr(&super_expr).unwrap_err();
    assert_eq!(err.code.as_str(), ErrorCode::E0416.as_str());

    checker.current_class = None;
    let err = checker.infer_expr(&super_expr).unwrap_err();
    assert_eq!(err.code.as_str(), ErrorCode::E0416.as_str());
}

#[test]
fn test_class_with_interface_validation() {
    let dspan = dummy_span();

    let speaker_decl = Item {
        id: ItemId(1),
        attrs: vec![],
        visibility: item::Visibility::Public,
        kind: ItemKind::Interface(item::InterfaceDecl {
            name: ident("Speaker"),
            generics: vec![],
            methods: vec![item::InterfaceMethodDef {
                name: ident("speak"),
                attrs: vec![],
                params: vec![],
                return_type: Some(yukic_ast::types::TypeNode::new(
                    yukic_ast::types::TypeNodeKind::Ident(ident("string")),
                    dspan,
                )),
                default_body: None,
                span: dspan,
            }],
            span: dspan,
        }),
        span: dspan,
    };

    let robot_decl = Item {
        id: ItemId(2),
        attrs: vec![],
        visibility: item::Visibility::Public,
        kind: ItemKind::Class(item::ClassDecl {
            name: ident("Robot"),
            is_locked: false,
            base_class: None,
            interfaces: vec![ident("Speaker")],
            fields: vec![],
            methods: vec![item::MethodDecl {
                name: ident("speak"),
                attrs: vec![],
                this_param: item::ThisParam::None,
                params: vec![],
                return_type: Some(yukic_ast::types::TypeNode::new(
                    yukic_ast::types::TypeNodeKind::Ident(ident("string")),
                    dspan,
                )),
                body: Block {
                    stmts: vec![Stmt {
                        kind: StmtKind::Return(Some(Expr::new(
                            ExprKind::Literal(LiteralValue::String("beep".to_string())),
                            dspan,
                        ))),
                        span: dspan,
                    }],
                    span: dspan,
                },
                is_operator: false,
                is_override: false,
                span: dspan,
            }],
            accessors: vec![],
            span: dspan,
        }),
        span: dspan,
    };

    let mut module = Module::new(FileId(0));
    module.items.push(speaker_decl.clone());
    module.items.push(robot_decl);

    let mut checker = TypeChecker::new();
    assert!(checker.check_module(&module).is_ok());

    let broken_robot = Item {
        id: ItemId(3),
        attrs: vec![],
        visibility: item::Visibility::Public,
        kind: ItemKind::Class(item::ClassDecl {
            name: ident("BrokenRobot"),
            is_locked: false,
            base_class: None,
            interfaces: vec![ident("Speaker")],
            fields: vec![],
            methods: vec![],
            accessors: vec![],
            span: dspan,
        }),
        span: dspan,
    };

    let mut bad_module = Module::new(FileId(0));
    bad_module.items.push(speaker_decl);
    bad_module.items.push(broken_robot);

    let mut bad_checker = TypeChecker::new();
    let errs = bad_checker.check_module(&bad_module).unwrap_err();
    assert!(errs.iter().any(|d| d.code.as_str() == "E0408"));

    let ghost_robot = Item {
        id: ItemId(4),
        attrs: vec![],
        visibility: item::Visibility::Public,
        kind: ItemKind::Class(item::ClassDecl {
            name: ident("GhostRobot"),
            is_locked: false,
            base_class: None,
            interfaces: vec![ident("NonExistent")],
            fields: vec![],
            methods: vec![],
            accessors: vec![],
            span: dspan,
        }),
        span: dspan,
    };

    let mut ghost_module = Module::new(FileId(0));
    ghost_module.items.push(ghost_robot);

    let mut ghost_checker = TypeChecker::new();
    let errs = ghost_checker.check_module(&ghost_module).unwrap_err();
    assert!(errs.iter().any(|d| d.code.as_str() == "E0305"));
}

#[test]
fn test_generic_interface_validation() {
    let dspan = dummy_span();

    let writer_decl = Item {
        id: ItemId(1),
        attrs: vec![],
        visibility: item::Visibility::Public,
        kind: ItemKind::Interface(item::InterfaceDecl {
            name: ident("Writer"),
            generics: vec![GenericParam {
                name: ident("T"),
                bounds: vec![],
                span: dspan,
            }],
            methods: vec![item::InterfaceMethodDef {
                name: ident("write"),
                attrs: vec![],
                params: vec![item::Param {
                    name: ident("data"),
                    ty: Some(yukic_ast::types::TypeNode::new(
                        yukic_ast::types::TypeNodeKind::Ident(ident("T")),
                        dspan,
                    )),
                    is_ref: false,
                    is_mut: false,
                    is_variadic: false,
                    default_val: None,
                    span: dspan,
                }],
                return_type: Some(yukic_ast::types::TypeNode::new(
                    yukic_ast::types::TypeNodeKind::Ident(ident("int64")),
                    dspan,
                )),
                default_body: None,
                span: dspan,
            }],
            span: dspan,
        }),
        span: dspan,
    };

    let string_writer = Item {
        id: ItemId(2),
        attrs: vec![],
        visibility: item::Visibility::Public,
        kind: ItemKind::Class(item::ClassDecl {
            name: ident("StringWriter"),
            is_locked: false,
            base_class: None,
            interfaces: vec![ident("Writer<string>")],
            fields: vec![],
            methods: vec![item::MethodDecl {
                name: ident("write"),
                attrs: vec![],
                this_param: item::ThisParam::None,
                params: vec![item::Param {
                    name: ident("data"),
                    ty: Some(yukic_ast::types::TypeNode::new(
                        yukic_ast::types::TypeNodeKind::Ident(ident("string")),
                        dspan,
                    )),
                    is_ref: false,
                    is_mut: false,
                    is_variadic: false,
                    default_val: None,
                    span: dspan,
                }],
                return_type: Some(yukic_ast::types::TypeNode::new(
                    yukic_ast::types::TypeNodeKind::Ident(ident("int64")),
                    dspan,
                )),
                body: Block {
                    stmts: vec![Stmt {
                        kind: StmtKind::Return(Some(Expr::new(
                            ExprKind::Literal(LiteralValue::Int(0)),
                            dspan,
                        ))),
                        span: dspan,
                    }],
                    span: dspan,
                },
                is_operator: false,
                is_override: false,
                span: dspan,
            }],
            accessors: vec![],
            span: dspan,
        }),
        span: dspan,
    };

    let mut module = Module::new(FileId(0));
    module.items.push(writer_decl.clone());
    module.items.push(string_writer);

    let mut checker = TypeChecker::new();
    assert!(checker.check_module(&module).is_ok());

    let bad_writer = Item {
        id: ItemId(3),
        attrs: vec![],
        visibility: item::Visibility::Public,
        kind: ItemKind::Class(item::ClassDecl {
            name: ident("BadWriter"),
            is_locked: false,
            base_class: None,
            interfaces: vec![ident("Writer<string>")],
            fields: vec![],
            methods: vec![item::MethodDecl {
                name: ident("write"),
                attrs: vec![],
                this_param: item::ThisParam::None,
                params: vec![item::Param {
                    name: ident("data"),
                    ty: Some(yukic_ast::types::TypeNode::new(
                        yukic_ast::types::TypeNodeKind::Ident(ident("int32")),
                        dspan,
                    )),
                    is_ref: false,
                    is_mut: false,
                    is_variadic: false,
                    default_val: None,
                    span: dspan,
                }],
                return_type: Some(yukic_ast::types::TypeNode::new(
                    yukic_ast::types::TypeNodeKind::Ident(ident("int64")),
                    dspan,
                )),
                body: Block { stmts: vec![], span: dspan },
                is_operator: false,
                is_override: false,
                span: dspan,
            }],
            accessors: vec![],
            span: dspan,
        }),
        span: dspan,
    };

    let mut bad_module = Module::new(FileId(0));
    bad_module.items.push(writer_decl);
    bad_module.items.push(bad_writer);

    let mut bad_checker = TypeChecker::new();
    let errs = bad_checker.check_module(&bad_module).unwrap_err();
    assert!(errs.iter().any(|d| d.code.as_str() == "E0408"));
}

#[test]
fn test_locked_class_inheritance_error() {
    let dspan = Span::new(FileId(0), BytePos(0), BytePos(0));
    let base_locked = Item {
        id: ItemId(1),
        visibility: Visibility::Public,
        attrs: vec![],
        kind: ItemKind::Class(ClassDecl {
            name: ident("Base"),
            is_locked: true,
            base_class: None,
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            accessors: vec![],
            span: dspan,
        }),
        span: dspan,
    };
    let child_class = Item {
        id: ItemId(2),
        visibility: Visibility::Public,
        attrs: vec![],
        kind: ItemKind::Class(ClassDecl {
            name: ident("Child"),
            is_locked: false,
            base_class: Some(ident("Base")),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            accessors: vec![],
            span: dspan,
        }),
        span: dspan,
    };

    let mut module = Module::new(FileId(0));
    module.items.push(base_locked);
    module.items.push(child_class);

    let mut checker = TypeChecker::new();
    let errs = checker.check_module(&module).unwrap_err();
    assert!(errs.iter().any(|d| d.code.as_str() == "E0409"));
}

#[test]
fn test_property_accessor_types() {
    let dspan = Span::new(FileId(0), BytePos(0), BytePos(0));
    let user_class = Item {
        id: ItemId(1),
        visibility: Visibility::Public,
        attrs: vec![],
        kind: ItemKind::Class(ClassDecl {
            name: ident("User"),
            is_locked: false,
            base_class: None,
            interfaces: vec![],
            fields: vec![ClassField {
                name: ident("_name"),
                attrs: vec![],
                ty: TypeNode::new(yukic_ast::types::TypeNodeKind::Ident(ident("string")), dspan),
                is_pub: false,
                default_val: None,
                span: dspan,
            }],
            methods: vec![],
            accessors: vec![
                PropertyAccessor {
                    name: ident("name"),
                    attrs: vec![],
                    ty: TypeNode::new(yukic_ast::types::TypeNodeKind::Ident(ident("string")), dspan),
                    kind: AccessorKind::Get,
                    body: Block {
                        stmts: vec![Stmt::new(
                            StmtKind::Return(Some(Expr::new(
                                ExprKind::FieldAccess(
                                    Box::new(Expr::new(ExprKind::This, dspan)),
                                    ident("_name"),
                                ),
                                dspan,
                            ))),
                            dspan,
                        )],
                        span: dspan,
                    },
                    span: dspan,
                },
                PropertyAccessor {
                    name: ident("name"),
                    attrs: vec![],
                    ty: TypeNode::new(yukic_ast::types::TypeNodeKind::Ident(ident("string")), dspan),
                    kind: AccessorKind::Set(ident("v")),
                    body: Block {
                        stmts: vec![Stmt::new(
                            StmtKind::Rewrite {
                                target: LValue::Field(
                                    Box::new(Expr::new(ExprKind::This, dspan)),
                                    ident("_name"),
                                ),
                                op: AssignOp::Eq,
                                value: Expr::new(ExprKind::Ident(ident("v")), dspan),
                            },
                            dspan,
                        )],
                        span: dspan,
                    },
                    span: dspan,
                },
            ],
            span: dspan,
        }),
        span: dspan,
    };

    let mut module = Module::new(FileId(0));
    module.items.push(user_class);

    let mut checker = TypeChecker::new();
    assert!(checker.check_module(&module).is_ok());

    let u_expr = Expr::new(ExprKind::Ident(ident("u")), dspan);
    checker.env.insert_var("u".to_string(), Type::Class("User".to_string()), false, dspan);
    let access_expr = Expr::new(ExprKind::FieldAccess(Box::new(u_expr.clone()), ident("name")), dspan);
    let res_ty = checker.infer_expr(&access_expr).unwrap();
    assert_eq!(res_ty, Type::String);
}

#[test]
fn test_cp_intrinsic_types() {
    let mut checker = TypeChecker::new();
    let dspan = dummy_span();

    checker.env.insert_var("s".to_string(), Type::String, false, dspan);
    let s_cp = Expr::new(
        ExprKind::MethodCall(
            Box::new(Expr::new(ExprKind::Ident(ident("s")), dspan)),
            ident("cp"),
            vec![],
        ),
        dspan,
    );
    assert_eq!(checker.infer_expr(&s_cp).unwrap(), Type::String);

    checker.env.insert_var("b".to_string(), Type::Bytes, false, dspan);
    let b_cp = Expr::new(
        ExprKind::MethodCall(
            Box::new(Expr::new(ExprKind::Ident(ident("b")), dspan)),
            ident("cp"),
            vec![],
        ),
        dspan,
    );
    assert_eq!(checker.infer_expr(&b_cp).unwrap(), Type::Bytes);

    checker.env.insert_var("arr".to_string(), Type::Array(Box::new(Type::Int32)), false, dspan);
    let arr_cp = Expr::new(
        ExprKind::MethodCall(
            Box::new(Expr::new(ExprKind::Ident(ident("arr")), dspan)),
            ident("cp"),
            vec![],
        ),
        dspan,
    );
    assert_eq!(checker.infer_expr(&arr_cp).unwrap(), Type::Array(Box::new(Type::Int32)));

    checker.env.insert_var("n".to_string(), Type::Int32, false, dspan);
    let n_cp = Expr::new(
        ExprKind::MethodCall(
            Box::new(Expr::new(ExprKind::Ident(ident("n")), dspan)),
            ident("cp"),
            vec![],
        ),
        dspan,
    );
    let err = checker.infer_expr(&n_cp).unwrap_err();
    assert_eq!(err.code.as_str(), ErrorCode::E0403.as_str());
}

#[test]
fn test_region_block_typing() {
    let mut checker = TypeChecker::new();
    let dspan = dummy_span();

    let block = Block::new(
        vec![
            Stmt::new(
                StmtKind::Set {
                    target: BindingTarget::Ident(ident("temp")),
                    ty: None,
                    init: Expr::new(ExprKind::Literal(LiteralValue::Int(100)), dspan),
                },
                dspan,
            ),
            Stmt::new(
                StmtKind::Expr(Expr::new(
                    ExprKind::Binary(
                        BinaryOp::Add,
                        Box::new(Expr::new(ExprKind::Ident(ident("temp")), dspan)),
                        Box::new(Expr::new(ExprKind::Literal(LiteralValue::Int(50)), dspan)),
                    ),
                    dspan,
                )),
                dspan,
            ),
        ],
        dspan,
    );
    let region_expr = Expr::new(ExprKind::RegionBlock(block), dspan);
    assert_eq!(checker.infer_expr(&region_expr).unwrap(), Type::Int64);
}

#[test]
fn test_operator_overload_typing() {
    let mut checker = TypeChecker::new();
    let dspan = dummy_span();

    let vec_ty_node = TypeNode {
        kind: yukic_ast::types::TypeNodeKind::Ident(ident("Vec2")),
        span: dspan,
    };

    let class_decl = ClassDecl {
        name: ident("Vec2"),
        base_class: None,
        interfaces: vec![],
        fields: vec![],
        methods: vec![
            item::MethodDecl {
                name: ident("+"),
                attrs: vec![],
                this_param: item::ThisParam::Value,
                params: vec![item::Param {
                    name: ident("other"),
                    is_ref: false,
                    is_mut: false,
                    is_variadic: false,
                    ty: Some(vec_ty_node.clone()),
                    default_val: None,
                    span: dspan,
                }],
                return_type: Some(vec_ty_node.clone()),
                body: Block::new(vec![return_this_stmt(dspan)], dspan),
                is_operator: true,
                is_override: false,
                span: dspan,
            },
            item::MethodDecl {
                name: ident("-"),
                attrs: vec![],
                this_param: item::ThisParam::Value,
                params: vec![],
                return_type: Some(vec_ty_node.clone()),
                body: Block::new(vec![return_this_stmt(dspan)], dspan),
                is_operator: true,
                is_override: false,
                span: dspan,
            },
        ],
        accessors: vec![],
        is_locked: false,
        span: dspan,
    };

    let mut mod_ast = Module::new(FileId(0));
    mod_ast.items.push(Item::new(
        ItemId(0),
        yukic_ast::item::Visibility::Local,
        ItemKind::Class(class_decl),
        dspan,
    ));

    checker.check_module(&mod_ast).unwrap();

    let v1 = Expr::new(ExprKind::ClassInit(Box::new(Expr::new(ExprKind::Ident(ident("Vec2")), dspan)), vec![]), dspan);
    let v2 = v1.clone();
    let add_expr = Expr::new(ExprKind::Binary(BinaryOp::Add, Box::new(v1.clone()), Box::new(v2)), dspan);
    let neg_expr = Expr::new(ExprKind::Unary(UnaryOp::Neg, Box::new(v1)), dspan);

    assert_eq!(checker.infer_expr(&add_expr).unwrap(), Type::Class("Vec2".to_string()));
    assert_eq!(checker.infer_expr(&neg_expr).unwrap(), Type::Class("Vec2".to_string()));
}

#[test]
fn test_cfun_pointer_typechecking_and_casting() {
    let dspan = dummy_span();
    let mut checker = TypeChecker::new();

    let cfun_decl = CfunDecl {
        name: ident("c_add"),
        c_attrs: vec![],
        yukino_attrs: vec![],
        params: vec![
            CParam { name: ident("a"), ty: TypeNode::new(TypeNodeKind::Ident(ident("int32")), dspan), span: dspan },
            CParam { name: ident("b"), ty: TypeNode::new(TypeNodeKind::Ident(ident("int32")), dspan), span: dspan },
        ],
        return_type: Some(TypeNode::new(TypeNodeKind::Ident(ident("int32")), dspan)),
        symbol_override: None,
        body: None,
        span: dspan,
    };

    let mut mod_ast = Module::new(FileId(0));
    mod_ast.items.push(Item::new(
        ItemId(0),
        yukic_ast::item::Visibility::Local,
        ItemKind::Cfun(cfun_decl),
        dspan,
    ));

    checker.check_module(&mod_ast).unwrap();

    let cfun_ref = Expr::new(ExprKind::Ident(ident("c_add")), dspan);
    let cfun_ty = checker.infer_expr(&cfun_ref).unwrap();
    assert!(matches!(cfun_ty, Type::Fn { is_cfun: true, .. }));

    let cast_to_ptr = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("cast")), dspan)),
            vec![
                cfun_ref.clone(),
                Expr::new(ExprKind::Literal(LiteralValue::String("ptr<void>".to_string())), dspan),
            ],
        ),
        dspan,
    );
    assert_eq!(checker.infer_expr(&cast_to_ptr).unwrap(), Type::Ptr(Box::new(Type::Void)));

    let ptr_var = Expr::new(ExprKind::Ident(ident("my_ptr")), dspan);
    let let_ptr = Stmt::new(
        StmtKind::Set {
            target: BindingTarget::Ident(ident("my_ptr")),
            ty: Some(TypeNode::new(TypeNodeKind::Pointer(Box::new(TypeNode::new(TypeNodeKind::Ident(ident("void")), dspan))), dspan)),
            init: cast_to_ptr,
        },
        dspan,
    );
    checker.check_stmt(&let_ptr).unwrap();

    let typeof_expr = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("typeof")), dspan)),
            vec![cfun_ref.clone()],
        ),
        dspan,
    );
    let cast_back = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("cast")), dspan)),
            vec![ptr_var.clone(), typeof_expr],
        ),
        dspan,
    );
    let restored_fn_ty = checker.infer_expr(&cast_back).unwrap();
    assert!(matches!(restored_fn_ty, Type::Fn { is_cfun: true, .. }));

    let call_restored = Expr::new(
        ExprKind::Call(
            Box::new(cast_back),
            vec![
                Expr::new(ExprKind::Literal(LiteralValue::Int(10)), dspan),
                Expr::new(ExprKind::Literal(LiteralValue::Int(20)), dspan),
            ],
        ),
        dspan,
    );
    assert_eq!(checker.infer_expr(&call_restored).unwrap(), Type::Int32);

    let call_raw_ptr = Expr::new(
        ExprKind::Call(
            Box::new(ptr_var),
            vec![
                Expr::new(ExprKind::Literal(LiteralValue::Int(10)), dspan),
                Expr::new(ExprKind::Literal(LiteralValue::Int(20)), dspan),
            ],
        ),
        dspan,
    );
    assert_eq!(checker.infer_expr(&call_raw_ptr).unwrap(), Type::Int64);

    let fn_var = Expr::new(ExprKind::Ident(ident("f1")), dspan);
    let let_fn = Stmt::new(
        StmtKind::Set {
            target: BindingTarget::Ident(ident("f1")),
            ty: Some(TypeNode::new(
                TypeNodeKind::CFun(
                    vec![
                        TypeNode::new(TypeNodeKind::Ident(ident("int32")), dspan),
                        TypeNode::new(TypeNodeKind::Ident(ident("int32")), dspan),
                    ],
                    Box::new(TypeNode::new(TypeNodeKind::Ident(ident("int32")), dspan)),
                ),
                dspan,
            )),
            init: cfun_ref.clone(),
        },
        dspan,
    );
    checker.check_stmt(&let_fn).unwrap();

    let call_fn = Expr::new(
        ExprKind::Call(
            Box::new(fn_var.clone()),
            vec![
                Expr::new(ExprKind::Literal(LiteralValue::Int(10)), dspan),
                Expr::new(ExprKind::Literal(LiteralValue::Int(20)), dspan),
            ],
        ),
        dspan,
    );
    let call_ty = checker.infer_expr(&call_fn).unwrap();
    assert_eq!(call_ty, Type::Int32);

    let let_res = Stmt::new(
        StmtKind::Set {
            target: BindingTarget::Ident(ident("res1")),
            ty: None,
            init: call_fn,
        },
        dspan,
    );
    checker.check_stmt(&let_res).unwrap();
}

#[test]
fn test_c_integer_predicates_and_properties() {
    let c_int = Type::CInt(CIntKind::Int);
    let c_uint = Type::CInt(CIntKind::UInt);
    let c_char = Type::CInt(CIntKind::Char);
    let c_uchar = Type::CInt(CIntKind::UChar);
    let c_long = Type::CInt(CIntKind::Long);
    let c_size_t = Type::CInt(CIntKind::SizeT);

    assert!(c_int.is_integer());
    assert!(c_int.is_numeric());
    assert!(c_int.is_signed_integer());
    assert!(!c_int.is_unsigned_integer());

    assert!(c_uint.is_integer());
    assert!(c_uint.is_numeric());
    assert!(!c_uint.is_signed_integer());
    assert!(c_uint.is_unsigned_integer());

    assert!(c_char.is_signed_integer());
    assert!(c_uchar.is_unsigned_integer());
    assert!(c_long.is_signed_integer());
    assert!(c_size_t.is_unsigned_integer());

    assert_eq!(c_int.bit_width(), Some(32));
    assert_eq!(c_char.bit_width(), Some(8));
    assert_eq!(c_uchar.bit_width(), Some(8));
    assert_eq!(Type::CInt(CIntKind::Short).bit_width(), Some(16));
    assert_eq!(Type::CInt(CIntKind::LongLong).bit_width(), Some(64));
}

#[test]
fn test_c_integer_literal_ranges_and_target_spec() {
    let dspan = dummy_span();

    // Host checker (LP64 Linux / 64-bit)
    let mut checker = TypeChecker::new();

    let lit42 = Expr::new(ExprKind::Literal(LiteralValue::Int(42)), dspan);
    assert_eq!(
        checker.check_expr(&lit42, &Type::CInt(CIntKind::Int)).unwrap(),
        Type::CInt(CIntKind::Int)
    );
    assert_eq!(
        checker.check_expr(&lit42, &Type::CInt(CIntKind::Short)).unwrap(),
        Type::CInt(CIntKind::Short)
    );
    assert_eq!(
        checker.check_expr(&lit42, &Type::CInt(CIntKind::Char)).unwrap(),
        Type::CInt(CIntKind::Char)
    );
    assert_eq!(
        checker.check_expr(&lit42, &Type::CInt(CIntKind::SizeT)).unwrap(),
        Type::CInt(CIntKind::SizeT)
    );

    // Overflow check for c_schar ([-128, 127])
    let lit200 = Expr::new(ExprKind::Literal(LiteralValue::Int(200)), dspan);
    let err_char = checker
        .check_expr(&lit200, &Type::CInt(CIntKind::SChar))
        .unwrap_err();
    assert_eq!(err_char.code.as_str(), ErrorCode::E0401.as_str());

    // Negative literal into unsigned C int
    let lit_neg = Expr::new(ExprKind::Literal(LiteralValue::Int(-1)), dspan);
    let err_uint = checker
        .check_expr(&lit_neg, &Type::CInt(CIntKind::UInt))
        .unwrap_err();
    assert_eq!(err_uint.code.as_str(), ErrorCode::E0401.as_str());

    // Target-specific: Windows (LLP64, long is 32-bit) vs Linux (LP64, long is 64-bit)
    let mut win_checker = TypeChecker::with_target(yukic_target::TargetTriple::X86_64PcWindowsGnu);
    let mut linux_checker =
        TypeChecker::with_target(yukic_target::TargetTriple::X86_64UnknownLinuxGnu);

    let large_num = 5_000_000_000_i64; // exceeds 32 bits
    let lit_large = Expr::new(ExprKind::Literal(LiteralValue::Int(large_num)), dspan);

    // On Windows, c_long is 32-bit: should fail with E0401
    let win_res = win_checker.check_expr(&lit_large, &Type::CInt(CIntKind::Long));
    assert!(win_res.is_err());
    assert_eq!(win_res.unwrap_err().code.as_str(), ErrorCode::E0401.as_str());

    // On Linux, c_long is 64-bit: should succeed
    let linux_res = linux_checker.check_expr(&lit_large, &Type::CInt(CIntKind::Long));
    assert!(linux_res.is_ok());
    assert_eq!(linux_res.unwrap(), Type::CInt(CIntKind::Long));
}

#[test]
fn test_c_integer_binary_and_unary_inference() {
    let dspan = dummy_span();
    let mut checker = TypeChecker::new();

    // Register local c_int variables
    checker.env.insert_var("x".to_string(), Type::CInt(CIntKind::Int), false, dspan);
    checker.env.insert_var("y".to_string(), Type::CInt(CIntKind::Int), false, dspan);

    let x_expr = Expr::new(ExprKind::Ident(ident("x")), dspan);
    let y_expr = Expr::new(ExprKind::Ident(ident("y")), dspan);
    let one_expr = Expr::new(ExprKind::Literal(LiteralValue::Int(1)), dspan);

    // x + y -> c_int
    let add_xy = Expr::new(
        ExprKind::Binary(BinaryOp::Add, Box::new(x_expr.clone()), Box::new(y_expr.clone())),
        dspan,
    );
    assert_eq!(checker.infer_expr(&add_xy).unwrap(), Type::CInt(CIntKind::Int));

    // x - y -> c_int
    let sub_xy = Expr::new(
        ExprKind::Binary(BinaryOp::Sub, Box::new(x_expr.clone()), Box::new(y_expr.clone())),
        dspan,
    );
    assert_eq!(checker.infer_expr(&sub_xy).unwrap(), Type::CInt(CIntKind::Int));

    // x + 1 -> c_int (coerced literal)
    let add_x1 = Expr::new(
        ExprKind::Binary(BinaryOp::Add, Box::new(x_expr.clone()), Box::new(one_expr.clone())),
        dspan,
    );
    assert_eq!(checker.infer_expr(&add_x1).unwrap(), Type::CInt(CIntKind::Int));

    // 1 + x -> c_int (coerced literal)
    let add_1x = Expr::new(
        ExprKind::Binary(BinaryOp::Add, Box::new(one_expr.clone()), Box::new(x_expr.clone())),
        dspan,
    );
    assert_eq!(checker.infer_expr(&add_1x).unwrap(), Type::CInt(CIntKind::Int));

    // x == y -> bool
    let eq_xy = Expr::new(
        ExprKind::Binary(BinaryOp::Eq, Box::new(x_expr.clone()), Box::new(y_expr.clone())),
        dspan,
    );
    assert_eq!(checker.infer_expr(&eq_xy).unwrap(), Type::Bool);

    // x < 10 -> bool
    let lt_x10 = Expr::new(
        ExprKind::Binary(
            BinaryOp::Lt,
            Box::new(x_expr.clone()),
            Box::new(Expr::new(ExprKind::Literal(LiteralValue::Int(10)), dspan)),
        ),
        dspan,
    );
    assert_eq!(checker.infer_expr(&lt_x10).unwrap(), Type::Bool);

    // -x -> c_int
    let neg_x = Expr::new(ExprKind::Unary(UnaryOp::Neg, Box::new(x_expr.clone())), dspan);
    assert_eq!(checker.infer_expr(&neg_x).unwrap(), Type::CInt(CIntKind::Int));

    // ~x -> c_int
    let bitnot_x = Expr::new(
        ExprKind::Unary(UnaryOp::BitNot, Box::new(x_expr.clone())),
        dspan,
    );
    assert_eq!(checker.infer_expr(&bitnot_x).unwrap(), Type::CInt(CIntKind::Int));
}

#[test]
fn test_sus_catch_implicit_variables() {
    let mut checker = TypeChecker::new();
    let dspan = dummy_span();

    let body_expr = Expr::new(ExprKind::Literal(LiteralValue::String("ok".to_string())), dspan);
    let body = Block::new(vec![Stmt::new(StmtKind::Expr(body_expr), dspan)], dspan);

    let errkind_expr = Expr::new(ExprKind::Ident(ident("errkind")), dspan);
    let catch = CatchClause {
        error_var: None,
        error_type: None,
        body: Block::new(vec![Stmt::new(StmtKind::Expr(errkind_expr), dspan)], dspan),
        span: dspan,
    };

    let sus_expr = Expr::new(
        ExprKind::Sus {
            body,
            catches: vec![catch],
            finally_branch: None,
        },
        dspan,
    );

    assert_eq!(checker.infer_expr(&sus_expr).unwrap(), Type::String);
}

#[test]
fn test_type_alias_equivalence() {
    let mut checker = TypeChecker::new();
    let dspan = dummy_span();

    let mut mod_ast = Module::new(FileId(0));
    // type ID = int64
    mod_ast.items.push(Item::new(
        ItemId(0),
        yukic_ast::item::Visibility::Local,
        ItemKind::TypeAlias(TypeAliasDecl {
            name: ident("ID"),
            generics: vec![],
            target: TypeNode::new(yukic_ast::types::TypeNodeKind::Ident(ident("int64")), dspan),
            span: dspan,
        }),
        dspan,
    ));
    // type UserId = ID
    mod_ast.items.push(Item::new(
        ItemId(1),
        yukic_ast::item::Visibility::Local,
        ItemKind::TypeAlias(TypeAliasDecl {
            name: ident("UserId"),
            generics: vec![],
            target: TypeNode::new(yukic_ast::types::TypeNodeKind::Ident(ident("ID")), dspan),
            span: dspan,
        }),
        dspan,
    ));
    // fun get_user(id: UserId) -> int64 { set x: UserId = 42; return id }
    let set_stmt = Stmt::new(
        StmtKind::Set {
            target: BindingTarget::Ident(ident("x")),
            ty: Some(TypeNode::new(yukic_ast::types::TypeNodeKind::Ident(ident("UserId")), dspan)),
            init: Expr::new(ExprKind::Literal(LiteralValue::Int(42)), dspan),
        },
        dspan,
    );
    let ret_stmt = Stmt::new(
        StmtKind::Return(Some(Expr::new(ExprKind::Ident(ident("id")), dspan))),
        dspan,
    );
    let fun_decl = FunDecl {
        name: ident("get_user"),
        attrs: vec![],
        generics: vec![],
        params: vec![Param {
            name: ident("id"),
            ty: Some(TypeNode::new(yukic_ast::types::TypeNodeKind::Ident(ident("UserId")), dspan)),
            default_val: None,
            is_ref: false,
            is_mut: false,
            is_variadic: false,
            span: dspan,
        }],
        return_type: Some(TypeNode::new(yukic_ast::types::TypeNodeKind::Ident(ident("int64")), dspan)),
        body: Block::new(vec![set_stmt, ret_stmt], dspan),
        span: dspan,
    };
    mod_ast.items.push(Item::new(
        ItemId(2),
        yukic_ast::item::Visibility::Local,
        ItemKind::Fun(fun_decl),
        dspan,
    ));

    // fun test_call(raw: int64) -> UserId { return get_user(raw) }
    let call_ret_stmt = Stmt::new(
        StmtKind::Return(Some(Expr::new(
            ExprKind::Call(
                Box::new(Expr::new(ExprKind::Ident(ident("get_user")), dspan)),
                vec![Expr::new(ExprKind::Ident(ident("raw")), dspan)],
            ),
            dspan,
        ))),
        dspan,
    );
    let test_call_decl = FunDecl {
        name: ident("test_call"),
        attrs: vec![],
        generics: vec![],
        params: vec![Param {
            name: ident("raw"),
            ty: Some(TypeNode::new(yukic_ast::types::TypeNodeKind::Ident(ident("int64")), dspan)),
            default_val: None,
            is_ref: false,
            is_mut: false,
            is_variadic: false,
            span: dspan,
        }],
        return_type: Some(TypeNode::new(yukic_ast::types::TypeNodeKind::Ident(ident("UserId")), dspan)),
        body: Block::new(vec![call_ret_stmt], dspan),
        span: dspan,
    };
    mod_ast.items.push(Item::new(
        ItemId(3),
        yukic_ast::item::Visibility::Local,
        ItemKind::Fun(test_call_decl),
        dspan,
    ));

    assert!(checker.check_module(&mod_ast).is_ok());
}

#[test]
fn test_class_init_syntax_and_ban_call() {
    let dspan = dummy_span();
    let mut checker = TypeChecker::new();

    let class_decl = ClassDecl {
        name: ident("Player"),
        base_class: None,
        interfaces: vec![],
        fields: vec![],
        methods: vec![],
        accessors: vec![],
        is_locked: false,
        span: dspan,
    };

    let mut mod_ast = Module::new(FileId(0));
    mod_ast.items.push(Item::new(
        ItemId(0),
        yukic_ast::item::Visibility::Local,
        ItemKind::Class(class_decl),
        dspan,
    ));

    checker.check_module(&mod_ast).unwrap();

    let init_expr = Expr::new(
        ExprKind::ClassInit(Box::new(Expr::new(ExprKind::Ident(ident("Player")), dspan)), vec![]),
        dspan,
    );
    assert_eq!(checker.infer_expr(&init_expr).unwrap(), Type::Class("Player".to_string()));

    let bare_call = Expr::new(
        ExprKind::Call(Box::new(Expr::new(ExprKind::Ident(ident("Player")), dspan)), vec![]),
        dspan,
    );
    assert!(checker.infer_expr(&bare_call).is_err());
}

#[test]
fn test_rt_and_extern_namespaces() {
    let mut checker = TypeChecker::new();
    let dspan = dummy_span();

    // 1. rt::exit
    let rt_exit = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Path(vec![ident("rt"), ident("exit")]), dspan)),
            vec![Expr::new(ExprKind::Literal(LiteralValue::Int(0)), dspan)],
        ),
        dspan,
    );
    assert_eq!(checker.infer_expr(&rt_exit).unwrap(), Type::Never);

    // 2. rt::args
    let rt_args = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Path(vec![ident("rt"), ident("args")]), dspan)),
            vec![],
        ),
        dspan,
    );
    assert_eq!(checker.infer_expr(&rt_args).unwrap(), Type::Array(Box::new(Type::String)));

    // 3. rt::cast
    let rt_cast = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Path(vec![ident("rt"), ident("cast")]), dspan)),
            vec![
                Expr::new(ExprKind::Literal(LiteralValue::Int(42)), dspan),
                Expr::new(ExprKind::Literal(LiteralValue::String("string".into())), dspan),
            ],
        ),
        dspan,
    );
    assert_eq!(checker.infer_expr(&rt_cast).unwrap(), Type::String);

    // 4. extern:: puts (cfun)
    checker.env.register_cfun(
        "puts".into(),
        FnSig {
            params: vec![TypeParam::value(Type::Ptr(Box::new(Type::Char)))],
            ret: Type::Int32,
            is_cfun: true,
        },
    );
    checker.env.register_fn(
        "my_yukino_fun".into(),
        FnSig {
            params: vec![],
            ret: Type::Void,
            is_cfun: false,
        },
    );

    let extern_puts = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Path(vec![ident("extern"), ident("puts")]), dspan)),
            vec![Expr::new(ExprKind::Literal(LiteralValue::Null), dspan)],
        ),
        dspan,
    );
    assert_eq!(checker.infer_expr(&extern_puts).unwrap(), Type::Int32);

    // 5. extern:: on a pure yukino fun is forbidden
    let extern_yukino = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Path(vec![ident("extern"), ident("my_yukino_fun")]), dspan)),
            vec![],
        ),
        dspan,
    );
    assert!(checker.infer_expr(&extern_yukino).is_err());

    // 6. extern:: on non-existent symbol is an error
    let extern_missing = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Path(vec![ident("extern"), ident("not_found")]), dspan)),
            vec![],
        ),
        dspan,
    );
    assert!(checker.infer_expr(&extern_missing).is_err());
}

#[test]
fn test_anonymous_fun_and_closure_types() {
    let mut checker = TypeChecker::new();
    let dspan = dummy_span();

    // fun(x: int32) => x * 2
    let param = yukic_ast::Param {
        name: ident("x"),
        is_ref: false,
        is_mut: false,
        is_variadic: false,
        ty: Some(yukic_ast::types::TypeNode::new(
            yukic_ast::types::TypeNodeKind::Ident(ident("int32")),
            dspan,
        )),
        default_val: None,
        span: dspan,
    };
    let x_expr = Expr::new(ExprKind::Ident(ident("x")), dspan);
    let two_expr = Expr::new(ExprKind::Literal(LiteralValue::Int(2)), dspan);
    let body_expr = Expr::new(
        ExprKind::Binary(BinaryOp::Mul, Box::new(x_expr), Box::new(two_expr)),
        dspan,
    );
    let ret_stmt = yukic_ast::Stmt::new(
        yukic_ast::StmtKind::Return(Some(body_expr)),
        dspan,
    );
    let block = yukic_ast::Block {
        stmts: vec![ret_stmt],
        span: dspan,
    };
    let decl = yukic_ast::FunDecl {
        name: ident("<anonymous>"),
        attrs: Vec::new(),
        generics: Vec::new(),
        params: vec![param],
        return_type: None,
        body: block,
        span: dspan,
    };
    let anon_fun = Expr::new(ExprKind::AnonymousFun(Box::new(decl)), dspan);
    let inferred = checker.infer_expr(&anon_fun).unwrap();
    match inferred {
        Type::Fn { params, ret, is_cfun } => {
            assert!(!is_cfun);
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].ty, Type::Int32);
            assert_eq!(*ret, Type::Int32);
        }
        other => panic!("expected Type::Fn, got {other:?}"),
    }
}

#[test]
fn test_thunk_function_attribute() {
    let dspan = dummy_span();
    let mut checker = TypeChecker::new();

    let thunk_attr = AttributeNode::new(
        ident("thunk"),
        yukic_ast::attr::AttributeArgs::None,
        dspan,
    );

    let worker_decl = FunDecl {
        name: ident("worker"),
        attrs: vec![thunk_attr],
        generics: vec![],
        params: vec![
            Param {
                name: ident("arg"),
                ty: Some(TypeNode::new(TypeNodeKind::Pointer(Box::new(TypeNode::new(TypeNodeKind::Ident(ident("void")), dspan))), dspan)),
                is_ref: false,
                is_mut: false,
                is_variadic: false,
                default_val: None,
                span: dspan,
            },
        ],
        return_type: None,
        body: Block { stmts: vec![], span: dspan },
        span: dspan,
    };

    let mut mod_ast = Module::new(FileId(0));
    mod_ast.items.push(Item::new(
        ItemId(0),
        yukic_ast::item::Visibility::Local,
        ItemKind::Fun(worker_decl),
        dspan,
    ));

    checker.check_module(&mod_ast).unwrap();

    let worker_ref = Expr::new(ExprKind::Ident(ident("worker")), dspan);
    let worker_ty = checker.infer_expr(&worker_ref).unwrap();
    assert!(matches!(worker_ty, Type::Fn { is_cfun: true, .. }));

    let cast_to_ptr = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(ident("cast")), dspan)),
            vec![
                worker_ref,
                Expr::new(ExprKind::Literal(LiteralValue::String("ptr<void>".to_string())), dspan),
            ],
        ),
        dspan,
    );
    assert_eq!(checker.infer_expr(&cast_to_ptr).unwrap(), Type::Ptr(Box::new(Type::Void)));
}

#[test]
fn test_interpolated_string_parts_are_type_checked() {
    let mut checker = TypeChecker::new();

    // Defined variable inside string interpolation.
    checker
        .env
        .insert_var("count".to_string(), Type::Int64, false, dummy_span());
    let ok_interp = Expr::new(
        ExprKind::InterpolatedString(vec![
            StringPart::Lit("total: ".to_string()),
            StringPart::Expr(Expr::new(ExprKind::Ident(ident("count")), dummy_span())),
        ]),
        dummy_span(),
    );
    assert_eq!(checker.infer_expr(&ok_interp).unwrap(), Type::String);

    // Undefined variable inside string interpolation reports E0402.
    let bad_interp = Expr::new(
        ExprKind::InterpolatedString(vec![
            StringPart::Lit("total: ".to_string()),
            StringPart::Expr(Expr::new(ExprKind::Ident(ident("missing")), dummy_span())),
        ]),
        dummy_span(),
    );
    let err = checker.infer_expr(&bad_interp).unwrap_err();
    assert_eq!(err.code.as_str(), ErrorCode::E0402.as_str());
    assert!(err.message.contains("missing"));
}

fn check_fun_module(ret: Option<&str>, body: Vec<Stmt>) -> Result<(), Vec<yukic_diagnostics::Diagnostic>> {
    let return_type = ret.map(|t| TypeNode::new(TypeNodeKind::Ident(ident(t)), dummy_span()));
    let decl = FunDecl {
        name: ident("compute"),
        attrs: vec![],
        generics: vec![],
        params: vec![],
        return_type,
        body: Block::new(body, dummy_span()),
        span: dummy_span(),
    };
    let mut module = Module::new(FileId(0));
    module.items.push(Item::new(
        ItemId(0),
        Visibility::Local,
        ItemKind::Fun(decl),
        dummy_span(),
    ));
    let mut checker = TypeChecker::new();
    checker.check_module(&module)
}

fn int_lit_stmt(v: i64) -> Stmt {
    Stmt {
        kind: StmtKind::Return(Some(Expr::new(
            ExprKind::Literal(LiteralValue::Int(v)),
            dummy_span(),
        ))),
        span: dummy_span(),
    }
}

fn return_this_stmt(span: Span) -> Stmt {
    Stmt {
        kind: StmtKind::Return(Some(Expr::new(ExprKind::Ident(ident("this")), span))),
        span,
    }
}

#[test]
fn test_catch_error_type_validation() {
    // Unknown struct in typed catch reports E0402.
    let mut checker = TypeChecker::new();
    let sus_unknown = Expr::new(
        ExprKind::Sus {
            body: Block::new(vec![], dummy_span()),
            catches: vec![CatchClause {
                error_var: Some(ident("e")),
                error_type: Some(TypeNode::new(
                    TypeNodeKind::Ident(ident("NoSuchError")),
                    dummy_span(),
                )),
                body: Block::new(vec![], dummy_span()),
                span: dummy_span(),
            }],
            finally_branch: None,
        },
        dummy_span(),
    );
    let err = checker.infer_expr(&sus_unknown).unwrap_err();
    assert_eq!(err.code.as_str(), ErrorCode::E0402.as_str());
    assert!(err.message.contains("NoSuchError"));

    // Structs registered from loaded modules validate without explicit import.
    let mut def = StructDef::default();
    def.fields.insert("message".to_string(), Type::String);
    checker.env.register_struct("MyError".to_string(), def);
    let sus_ok = Expr::new(
        ExprKind::Sus {
            body: Block::new(vec![], dummy_span()),
            catches: vec![CatchClause {
                error_var: Some(ident("e")),
                error_type: Some(TypeNode::new(
                    TypeNodeKind::Ident(ident("MyError")),
                    dummy_span(),
                )),
                body: Block::new(vec![], dummy_span()),
                span: dummy_span(),
            }],
            finally_branch: None,
        },
        dummy_span(),
    );
    assert!(checker.infer_expr(&sus_ok).is_ok());

    // Builtin Error struct is always available.
    let sus_error = Expr::new(
        ExprKind::Sus {
            body: Block::new(vec![], dummy_span()),
            catches: vec![CatchClause {
                error_var: None,
                error_type: Some(TypeNode::new(TypeNodeKind::Ident(ident("Error")), dummy_span())),
                body: Block::new(vec![], dummy_span()),
                span: dummy_span(),
            }],
            finally_branch: None,
        },
        dummy_span(),
    );
    assert!(checker.infer_expr(&sus_error).is_ok());
}

#[test]
fn test_missing_return_empty_body() {
    // Missing return reports dedicated E0410.
    let errs = check_fun_module(Some("int64"), vec![]).unwrap_err();
    assert!(errs.iter().any(|d| d.code.as_str() == ErrorCode::E0410.as_str()));
    assert!(errs.iter().all(|d| d.code.as_str() != ErrorCode::E0401.as_str()));
}

#[test]
fn test_missing_return_after_non_expr_stmt() {
    // Non-expression statement ending reports E0410.
    let body = vec![Stmt {
        kind: StmtKind::Set {
            target: BindingTarget::Ident(ident("x")),
            ty: None,
            init: Expr::new(ExprKind::Literal(LiteralValue::Int(1)), dummy_span()),
        },
        span: dummy_span(),
    }];
    let errs = check_fun_module(Some("int64"), body).unwrap_err();
    assert!(errs.iter().any(|d| d.code.as_str() == ErrorCode::E0410.as_str()));
}

#[test]
fn test_missing_return_trailing_void_call() {
    // Trailing void expression reports E0410.
    let body = vec![Stmt {
        kind: StmtKind::Expr(Expr::new(
            ExprKind::Call(
                Box::new(Expr::new(ExprKind::Ident(ident("assert_eq")), dummy_span())),
                vec![
                    Expr::new(ExprKind::Literal(LiteralValue::Int(1)), dummy_span()),
                    Expr::new(ExprKind::Literal(LiteralValue::Int(1)), dummy_span()),
                ],
            ),
            dummy_span(),
        )),
        span: dummy_span(),
    }];
    let errs = check_fun_module(Some("int64"), body).unwrap_err();
    assert!(errs.iter().any(|d| d.code.as_str() == ErrorCode::E0410.as_str()));
    assert!(errs.iter().all(|d| d.code.as_str() != ErrorCode::E0401.as_str()));
}

#[test]
fn test_trailing_if_with_returns_in_all_branches_is_ok() {
    // Divergent if branches satisfy return requirement.
    let ret_one = int_lit_stmt(1);
    let ret_two = int_lit_stmt(2);
    let if_expr = Expr::new(
        ExprKind::If {
            cond: Box::new(Expr::new(
                ExprKind::Literal(LiteralValue::Bool(true)),
                dummy_span(),
            )),
            then_branch: Block::new(vec![ret_one], dummy_span()),
            elif_branches: vec![],
            else_branch: Some(Block::new(vec![ret_two], dummy_span())),
        },
        dummy_span(),
    );
    let body = vec![Stmt {
        kind: StmtKind::Expr(if_expr),
        span: dummy_span(),
    }];
    assert!(check_fun_module(Some("int64"), body).is_ok());
}

#[test]
fn test_trailing_wrong_type_is_still_type_mismatch() {
    // Type mismatch remains E0401.
    let body = vec![Stmt {
        kind: StmtKind::Expr(Expr::new(
            ExprKind::Literal(LiteralValue::String("text".to_string())),
            dummy_span(),
        )),
        span: dummy_span(),
    }];
    let errs = check_fun_module(Some("int64"), body).unwrap_err();
    assert!(errs.iter().any(|d| d.code.as_str() == ErrorCode::E0401.as_str()));

    // Tail expression matching return type is valid.
    let ok_body = vec![Stmt {
        kind: StmtKind::Expr(Expr::new(
            ExprKind::Literal(LiteralValue::Int(5)),
            dummy_span(),
        )),
        span: dummy_span(),
    }];
    assert!(check_fun_module(Some("int64"), ok_body).is_ok());
    assert!(check_fun_module(None, vec![]).is_ok());
}
