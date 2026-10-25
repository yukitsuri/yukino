#![deny(unsafe_code)]

use inkwell::context::Context;
use std::fs;
use std::path::Path;
use yukic_ast::Ident;
use yukic_ast::item::{
    CParam, CfunDecl, FunDecl, Item, ItemId, ItemKind, Module as AstModule, Param,
};
use yukic_ast::stmt::Block;
use yukic_ast::types::{TypeNode, TypeNodeKind};
use yukic_codegen::CodeGenerator;
use yukic_span::{BytePos, FileId, Span};
use yukic_target::TargetTriple;

#[test]
fn test_emit_llvm_dps_ir_file() {
    let context = Context::create();
    let mut codegen = CodeGenerator::new(
        &context,
        "yukino_dps_module",
        TargetTriple::X86_64UnknownLinuxGnu,
    );
    let dummy_span = Span::new(FileId(0), BytePos(0), BytePos(0));

    let mut ast = AstModule::default();

    // 1. fun main() -> i32 (C ABI entry point, app region)
    ast.items.push(Item::new(
        ItemId(1),
        yukic_ast::Visibility::Public,
        ItemKind::Fun(FunDecl {
            name: Ident::new("main", dummy_span),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: Vec::new(),
            return_type: None,
            body: Block::new(Vec::new(), dummy_span),
            span: dummy_span,
        }),
        dummy_span,
    ));

    // 2. fun compute_message(code: int64) -> string (DPS transformed, %_dest_region)
    ast.items.push(Item::new(
        ItemId(2),
        yukic_ast::Visibility::Public,
        ItemKind::Fun(FunDecl {
            name: Ident::new("compute_message", dummy_span),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: vec![Param {
                name: Ident::new("code", dummy_span),
                is_ref: false,
                is_mut: false,
                is_variadic: false,
                ty: Some(TypeNode::new(
                    TypeNodeKind::Ident(Ident::new("int64", dummy_span)),
                    dummy_span,
                )),
                default_val: None,
                span: dummy_span,
            }],
            return_type: Some(TypeNode::new(
                TypeNodeKind::Ident(Ident::new("string", dummy_span)),
                dummy_span,
            )),
            body: Block::new(Vec::new(), dummy_span),
            span: dummy_span,
        }),
        dummy_span,
    ));

    // 3. cfun c_helper(val: int32) -> int32 (C ABI, no DPS)
    ast.items.push(Item::new(
        ItemId(3),
        yukic_ast::Visibility::Public,
        ItemKind::Cfun(CfunDecl {
            name: Ident::new("c_helper", dummy_span),
            c_attrs: Vec::new(),
            yukino_attrs: Vec::new(),
            params: vec![CParam {
                name: Ident::new("val", dummy_span),
                ty: TypeNode::new(
                    TypeNodeKind::Ident(Ident::new("int32", dummy_span)),
                    dummy_span,
                ),
                span: dummy_span,
            }],
            return_type: Some(TypeNode::new(
                TypeNodeKind::Ident(Ident::new("int32", dummy_span)),
                dummy_span,
            )),
            symbol_override: None,
            body: None,
            span: dummy_span,
        }),
        dummy_span,
    ));

    codegen.compile_module(&ast).expect("compile module");

    // 4. emit test function with fast-path bump alloc and loop marks
    let void_ty = context.void_type().fn_type(&[], false);
    let fn_val = codegen
        .module
        .add_function("test_loop_and_bump_pipeline", void_ty, None);
    let entry = context.append_basic_block(fn_val, "entry");
    codegen.builder.position_at_end(entry);

    let local_reg = codegen
        .emit_region_prologue(4096, "_pipeline_reg")
        .expect("prologue");

    let mark = codegen.emit_loop_save_mark(local_reg).expect("save mark");
    let alloc_ptr = codegen
        .emit_fast_path_alloc(fn_val, local_reg, 64, 16)
        .expect("bump alloc");
    assert_eq!(alloc_ptr.get_type(), codegen.ptr_type());

    codegen
        .emit_loop_restore_mark(local_reg, mark)
        .expect("restore mark");
    codegen.emit_region_epilogue(local_reg).expect("epilogue");
    codegen.builder.build_return(None).unwrap();

    codegen.module.verify().expect("verify module");

    // ...
    let ir_content = codegen.emit_llvm_ir_to_string();

    // 6. verify expected symbols in generated IR
    assert!(ir_content.contains("define i32 @main(i32 %0, ptr %1)"));
    assert!(ir_content.contains("call void @yukino_region_init(ptr %_app_region, i64 65536)"));
    assert!(ir_content.contains("call void @yukino_region_destroy(ptr %_app_region)"));
    assert!(
        ir_content
            .contains("define fastcc { ptr, i64 } @__yukifun_compute_message_int64__string(ptr %_dest_region, i64 %0)")
    );
    assert!(ir_content.contains("call void @yukino_region_init(ptr %_local_region, i64 4096)"));
    assert!(ir_content.contains("call void @yukino_region_destroy(ptr %_local_region)"));
    assert!(ir_content.contains("declare i32 @c_helper(i32)"));
    assert!(ir_content.contains("call ptr @yukino_region_save_mark(ptr %_pipeline_reg)"));
    assert!(
        ir_content
            .contains("call void @yukino_region_restore_mark(ptr %_pipeline_reg, ptr %loop_mark)")
    );
    assert!(ir_content.contains("bump_fast:"));
    assert!(ir_content.contains("bump_slow:"));
    assert!(ir_content.contains("bump_merge:"));
}
