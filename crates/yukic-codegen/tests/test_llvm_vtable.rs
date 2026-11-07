#![deny(unsafe_code)]

use inkwell::context::Context;
use std::fs;
use std::path::Path;
use yukic_ast::Ident;
use yukic_ast::item::{
    ClassDecl, FunDecl, InterfaceDecl, InterfaceMethodDef, Item, ItemId, ItemKind, MethodDecl,
    Module as AstModule, ThisParam,
};
use yukic_ast::stmt::Block;
use yukic_codegen::CodeGenerator;
use yukic_span::{BytePos, FileId, Span};
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
            stmts: Vec::new(),
            span: Span::DUMMY,
        },
        is_operator: false,
        is_override: false,
        span: Span::DUMMY,
    }
}

fn build_test_ast(dummy_span: Span) -> AstModule {
    let mut ast = AstModule::default();

    // 1. interface Speaker { speak() }
    ast.items.push(Item::new(
        ItemId(1),
        yukic_ast::Visibility::Public,
        ItemKind::Interface(InterfaceDecl {
            name: dummy_ident("Speaker"),
            generics: Vec::new(),
            methods: vec![InterfaceMethodDef {
                name: dummy_ident("speak"),
                attrs: Vec::new(),
                params: Vec::new(),
                return_type: None,
                default_body: None,
                span: dummy_span,
            }],
            span: dummy_span,
        }),
        dummy_span,
    ));

    // 2. class Animal { speak() }
    ast.items.push(Item::new(
        ItemId(2),
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

    // 3. class Dog: Animal, Speaker { speak(), bark() }
    ast.items.push(Item::new(
        ItemId(3),
        yukic_ast::Visibility::Public,
        ItemKind::Class(ClassDecl {
            name: dummy_ident("Dog"),
            is_locked: false,
            base_class: Some(dummy_ident("Animal")),
            interfaces: vec![dummy_ident("Speaker")],
            fields: Vec::new(),
            methods: vec![dummy_method("speak"), dummy_method("bark")],
            accessors: Vec::new(),
            span: dummy_span,
        }),
        dummy_span,
    ));

    // 4. fun main() -> i32
    ast.items.push(Item::new(
        ItemId(4),
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

    ast
}

fn build_dispatch_demo<'ctx>(codegen: &CodeGenerator<'ctx>, context: &'ctx Context) {
    let ptr_ty = codegen.ptr_type();
    let void_ty = context.void_type().fn_type(&[ptr_ty.into()], false);
    let demo_fn = codegen.module.add_function("demo_dispatch", void_ty, None);
    let entry = context.append_basic_block(demo_fn, "entry");
    codegen.builder.position_at_end(entry);

    let dog_inst = demo_fn.get_first_param().unwrap().into_pointer_value();

    // a. initialize instance vtable
    let dog_vt = codegen.module.get_global("_Y_VTABLE_Dog").expect("dog vt");
    codegen
        .init_instance_vtable(dog_inst, dog_vt)
        .expect("init vtable");

    // b. virtual call Dog.speak(dog_inst)
    codegen
        .emit_virtual_call("Dog", "speak", dog_inst, void_ty, &[])
        .expect("vcall speak");

    // c. virtual call Dog.bark(dog_inst)
    codegen
        .emit_virtual_call("Dog", "bark", dog_inst, void_ty, &[])
        .expect("vcall bark");

    // d. super call to Animal.speak(dog_inst)
    let animal_speak = codegen
        .module
        .get_function("_Y_Animal_speak")
        .expect("animal speak fn");
    codegen
        .emit_super_call(animal_speak, dog_inst, &[])
        .expect("super call");

    // e. interface fat pointer and call Speaker.speak(fat_ptr)
    let dog_speaker_itable = codegen
        .module
        .get_global("_Y_ITABLE_Dog_Speaker")
        .expect("dog itable");
    let fat_speaker = codegen
        .make_interface_fat_pointer(dog_inst, dog_speaker_itable)
        .expect("fat ptr");
    let slot = codegen
        .get_interface_slot("Speaker", "speak")
        .expect("slot");
    codegen
        .emit_interface_call(fat_speaker, slot, void_ty, &[])
        .expect("icall");

    codegen.builder.build_return(None).unwrap();
}

fn verify_emitted_ir(ir_content: &str) {
    assert!(ir_content.contains("@_Y_VTABLE_Animal = internal constant"));
    assert!(ir_content.contains("@_Y_VTABLE_Dog = internal constant"));
    assert!(ir_content.contains("@_Y_ITABLE_Dog_Speaker = internal constant"));
    assert!(ir_content.contains("define fastcc void @_Y_Animal_speak(ptr %0)"));
    assert!(ir_content.contains("define fastcc void @_Y_Dog_speak(ptr %0)"));
    assert!(ir_content.contains("define fastcc void @_Y_Dog_bark(ptr %0)"));
    assert!(ir_content.contains("call fastcc void @_Y_Animal_speak(ptr %0)"));
    assert!(ir_content.contains("islot_gep"));
    assert!(ir_content.contains("imethod_ptr"));
}

#[test]
fn test_emit_llvm_vtable_ir_file() {
    let context = Context::create();
    let mut codegen = CodeGenerator::new(
        &context,
        "yukino_vtable_module",
        TargetTriple::X86_64UnknownLinuxGnu,
    );
    let dummy_span = Span::new(FileId(0), BytePos(0), BytePos(0));

    let ast = build_test_ast(dummy_span);
    codegen.compile_module(&ast).expect("compile module");

    build_dispatch_demo(&codegen, &context);

    codegen.module.verify().expect("verify module");

    let ir_content = codegen.emit_llvm_ir_to_string();
    verify_emitted_ir(&ir_content);
}
