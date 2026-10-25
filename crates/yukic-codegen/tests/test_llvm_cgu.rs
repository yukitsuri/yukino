#![deny(unsafe_code)]

use std::fs;
use std::path::Path;
use yukic_ast::Ident;
use yukic_ast::item::{
    CParam, CfunDecl, ClassDecl, FunDecl, Item, ItemId, ItemKind, MethodDecl, Module as AstModule,
    Param, ThisParam,
};
use yukic_ast::stmt::{Block, Stmt, StmtKind};
use yukic_ast::types::{TypeNode, TypeNodeKind};
use yukic_codegen::compile_modules_parallel;
use yukic_span::Span;
use yukic_target::TargetTriple;

fn dummy_ident(name: &str) -> Ident {
    Ident {
        name: name.to_string(),
        span: Span::DUMMY,
    }
}

fn dummy_method(name: &str) -> MethodDecl {
    MethodDecl {
        name: dummy_ident(name),
        attrs: Vec::new(),
        this_param: ThisParam::Value,
        params: Vec::new(),
        return_type: None,
        body: Block {
            stmts: vec![Stmt::new(StmtKind::Space, Span::DUMMY)],
            span: Span::DUMMY,
        },
        is_operator: false,
        is_override: false,
        span: Span::DUMMY,
    }
}

fn int32_type_node() -> TypeNode {
    TypeNode::new(TypeNodeKind::Ident(dummy_ident("int32")), Span::DUMMY)
}

fn make_param(name: &str) -> Param {
    Param {
        name: dummy_ident(name),
        is_ref: false,
        is_mut: false,
        is_variadic: false,
        ty: Some(int32_type_node()),
        default_val: None,
        span: Span::DUMMY,
    }
}

fn build_multi_item_ast() -> AstModule {
    let mut ast = AstModule::default();
    let dummy_span = Span::DUMMY;

    // 1. class Animal { speak() }
    ast.items.push(Item::new(
        ItemId(1),
        yukic_ast::Visibility::Public,
        ItemKind::Class(ClassDecl {
            name: dummy_ident("Animal"),
            is_locked: false,
            base_class: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: vec![dummy_method("speak")],
            accessors: Vec::new(),
            span: dummy_span,
        }),
        dummy_span,
    ));

    // 2. class Dog: Animal { speak(), bark() }
    ast.items.push(Item::new(
        ItemId(2),
        yukic_ast::Visibility::Public,
        ItemKind::Class(ClassDecl {
            name: dummy_ident("Dog"),
            is_locked: false,
            base_class: Some(dummy_ident("Animal")),
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: vec![dummy_method("speak"), dummy_method("bark")],
            accessors: Vec::new(),
            span: dummy_span,
        }),
        dummy_span,
    ));

    // 3. fun compute_sum(a: int32, b: int32) -> int32
    ast.items.push(Item::new(
        ItemId(3),
        yukic_ast::Visibility::Public,
        ItemKind::Fun(FunDecl {
            name: dummy_ident("compute_sum"),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: vec![make_param("a"), make_param("b")],
            return_type: Some(int32_type_node()),
            body: Block::new(
                vec![
                    Stmt::new(StmtKind::Space, dummy_span),
                    Stmt::new(StmtKind::Space, dummy_span),
                ],
                dummy_span,
            ),
            span: dummy_span,
        }),
        dummy_span,
    ));

    // 4. fun main() -> int32
    ast.items.push(Item::new(
        ItemId(4),
        yukic_ast::Visibility::Public,
        ItemKind::Fun(FunDecl {
            name: dummy_ident("main"),
            attrs: Vec::new(),
            generics: Vec::new(),
            params: Vec::new(),
            return_type: Some(int32_type_node()),
            body: Block::new(
                vec![
                    Stmt::new(StmtKind::Space, dummy_span),
                    Stmt::new(StmtKind::Space, dummy_span),
                ],
                dummy_span,
            ),
            span: dummy_span,
        }),
        dummy_span,
    ));

    // 5. public cfun c_api_calc(val: int32) -> int32
    ast.items.push(Item::new(
        ItemId(5),
        yukic_ast::Visibility::Public,
        ItemKind::Cfun(CfunDecl {
            name: dummy_ident("c_api_calc"),
            c_attrs: Vec::new(),
            yukino_attrs: Vec::new(),
            params: vec![CParam {
                name: dummy_ident("val"),
                ty: int32_type_node(),
                span: dummy_span,
            }],
            return_type: Some(int32_type_node()),
            symbol_override: None,
            body: Some(Block::new(
                vec![Stmt::new(StmtKind::Space, dummy_span)],
                dummy_span,
            )),
            span: dummy_span,
        }),
        dummy_span,
    ));

    ast
}

#[test]
fn test_parallel_cgu_emission_and_cross_linkage() {
    let ast = build_multi_item_ast();
    let compiled_units = compile_modules_parallel(&ast, &[], TargetTriple::X86_64UnknownLinuxGnu, 2, "unwind")
        .expect("parallel compilation");

    assert_eq!(compiled_units.len(), 2);
    let cgu0 = &compiled_units[0];
    let cgu1 = &compiled_units[1];

    // Combine both IRs for cross-check
    let combined_ir = format!("{}\n{}", cgu0.ll_ir, cgu1.ll_ir);

    // Verify definitions exist across units
    assert!(combined_ir.contains("define fastcc void @_Y_Animal_speak"));
    assert!(combined_ir.contains("define fastcc void @_Y_Dog_bark"));
    assert!(combined_ir.contains("define fastcc i32 @__yukifun_compute_sum_int32_int32__int32"));
    assert!(combined_ir.contains("define i32 @main(i32"));
    assert!(combined_ir.contains("define i32 @c_api_calc(i32"));

    // Verify external vtable symbols exist
    assert!(combined_ir.contains("@_Y_VTABLE_Animal = external constant"));
    assert!(combined_ir.contains("@_Y_VTABLE_Dog = external constant"));

    // Verify foreign declarations exist in the counterpart module
    let has_foreign_decl = (cgu0.ll_ir.contains("declare fastcc void @_Y_Animal_speak")
        && cgu1.ll_ir.contains("define fastcc void @_Y_Animal_speak"))
        || (cgu1.ll_ir.contains("declare fastcc void @_Y_Animal_speak")
            && cgu0.ll_ir.contains("define fastcc void @_Y_Animal_speak"));
    assert!(has_foreign_decl, "expected cross-cgu function declaration");
}
