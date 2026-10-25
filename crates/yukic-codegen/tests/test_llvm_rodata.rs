#![deny(unsafe_code)]

use inkwell::context::Context;
use std::fs;
use std::path::Path;
use yukic_ast::Ident;
use yukic_ast::expr::{Expr, ExprKind, LiteralValue};
use yukic_ast::item::{Item, ItemId, ItemKind, Module as AstModule, StaticDecl};
use yukic_codegen::CodeGenerator;
use yukic_span::{BytePos, FileId, Span};
use yukic_target::TargetTriple;
use yukic_types::Type;

#[test]
fn test_emit_llvm_ir_file() {
    let context = Context::create();
    let mut codegen = CodeGenerator::new(
        &context,
        "yukino_rodata_module",
        TargetTriple::X86_64UnknownLinuxGnu,
    );

    // 1. emit string literals (with deduplication)
    let _s1 = codegen.emit_string_literal("Hello, Yukino LLVM Backend!");
    let _s2 = codegen.emit_string_literal("Hello, Yukino LLVM Backend!");

    // 2. emit raw byte buffer
    let _b = codegen.emit_bytes_literal(&[0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01]);

    // 3. emit static typeof intrinsics
    let _t1 = codegen.emit_typeof(&Type::Int64);
    let _t2 = codegen.emit_typeof(&Type::String);
    let _t3 = codegen.emit_typeof(&Type::Array(Box::new(Type::Float64)));

    // 4. compile top-level static items
    let dummy_span = Span::new(FileId(0), BytePos(0), BytePos(0));
    let mut ast = AstModule::default();
    ast.items.push(Item::new(
        ItemId(1),
        yukic_ast::Visibility::Public,
        ItemKind::Static(StaticDecl {
            name: Ident::new("G_APP_NAME", dummy_span),
            ty: None,
            init: Expr::new(
                ExprKind::Literal(LiteralValue::String("yukino-app".to_string())),
                dummy_span,
            ),
            span: dummy_span,
        }),
        dummy_span,
    ));

    codegen.compile_module(&ast).expect("compile module");

    // ...
    let content = codegen.emit_llvm_ir_to_string();
    assert!(content.contains("@.str.anon.0 = private unnamed_addr constant [28 x i8] c\"Hello, Yukino LLVM Backend!\\00\", align 1"));
    assert!(content.contains("@.bytes.anon.0 = private unnamed_addr constant [7 x i8]"));
    assert!(
        content.contains(
            "@.str.type.0 = private unnamed_addr constant [6 x i8] c\"int64\\00\", align 1"
        )
    );
    assert!(content.contains(
        "@.str.type.1 = private unnamed_addr constant [7 x i8] c\"string\\00\", align 1"
    ));
    assert!(content.contains(
        "@.str.type.2 = private unnamed_addr constant [10 x i8] c\"float64[]\\00\", align 1"
    ));
    assert!(content.contains("@G_APP_NAME = constant { ptr, i64 } { ptr @.str.anon.1, i64 10 }"));
}
