#![deny(unsafe_code)]

pub mod body;
pub mod cabi;
pub mod cgu;
pub mod closure;
pub mod context;
pub mod debug;
pub mod exception;
pub mod generator;
pub mod intrinsic;
pub mod mangle;
pub mod region;
pub mod region_elision;
pub mod registry;
pub mod rodata;
pub mod test_harness;
pub mod types;
pub mod vtable;

pub use body::*;
pub use cabi::*;
pub use cgu::{
    CodegenUnit, CompiledCgu, compile_modules_parallel, compile_units_parallel,
    compile_units_parallel_with_overrides, estimate_item_weight, partition_module,
    partition_module_with_custom,
};
pub use closure::*;
pub use context::init_llvm_targets;
pub use debug::*;
pub use exception::*;
pub use generator::{CodeGenerator, embed_artifact_version_metadata};
pub use inkwell;
pub use intrinsic::*;
pub use mangle::*;
pub use region::*;
pub use region_elision::*;
pub use registry::*;
pub use rodata::*;
pub use test_harness::*;
pub use types::*;
pub use vtable::*;

#[cfg(test)]
mod tests {
    use super::*;
    use inkwell::context::Context;
    use yukic_ast::{
        FieldDef, Ident, Item, ItemId, ItemKind, Module as AstModule, StructDecl, TypeNode,
        TypeNodeKind, Visibility,
    };
    use yukic_span::{BytePos, FileId, Span};
    use yukic_target::TargetTriple;
    use yukic_types::{CIntKind, Type};

    #[test]
    fn test_codegen_init() {
        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, "test_mod", TargetTriple::host());
        let ast = AstModule::default();
        assert!(codegen.compile_module(&ast).is_ok());
    }

    #[test]
    fn test_target_triple_and_datalayout_configured() {
        let context = Context::create();
        let target = TargetTriple::X86_64UnknownLinuxGnu;
        let codegen = CodeGenerator::new(&context, "test_target", target);

        assert_eq!(
            codegen.module.get_triple().as_str().to_str().unwrap(),
            target.spec().triple
        );
        assert_eq!(
            codegen.module.get_data_layout().as_str().to_str().unwrap(),
            target.spec().data_layout
        );
    }

    #[test]
    fn test_opaque_pointer_load_and_struct_gep() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_ptr", TargetTriple::host());

        let i64_type = context.i64_type();
        let ptr_type = codegen.ptr_type();
        assert_eq!(ptr_type.to_string(), "\"ptr\"");

        let fn_type = context.void_type().fn_type(&[], false);
        let function = codegen.module.add_function("test_fn", fn_type, None);
        let basic_block = context.append_basic_block(function, "entry");
        codegen.builder.position_at_end(basic_block);

        let struct_type = context.struct_type(&[i64_type.into(), i64_type.into()], false);
        let alloca = codegen.builder.build_alloca(struct_type, "pt").unwrap();

        let field0_gep = codegen
            .build_struct_gep(struct_type, alloca, 0, "f0")
            .unwrap();
        assert_eq!(field0_gep.get_type().to_string(), "\"ptr\"");

        let store_val = i64_type.const_int(42, false);
        codegen.builder.build_store(field0_gep, store_val).unwrap();

        let load_val = codegen.build_load(i64_type, field0_gep, "v0").unwrap();
        assert!(load_val.is_int_value());
        assert_eq!(load_val.into_int_value().get_type(), i64_type);
    }

    #[test]
    fn test_opaque_pointer_indirect_call() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_icall", TargetTriple::host());

        let i64_type = context.i64_type();
        let target_fn_type = i64_type.fn_type(&[i64_type.into()], false);
        let caller_fn_type = context.void_type().fn_type(&[], false);
        let caller = codegen.module.add_function("caller", caller_fn_type, None);
        let entry = context.append_basic_block(caller, "entry");
        codegen.builder.position_at_end(entry);

        let dummy_fn_ptr = codegen.ptr_type().const_null();
        let arg = i64_type.const_int(10, false);
        let call_res =
            codegen.build_indirect_call(target_fn_type, dummy_fn_ptr, &[arg.into()], "vcall_res");
        assert!(call_res.is_ok());
    }

    #[test]
    fn test_direct_call_builder() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_call", TargetTriple::host());

        let i32_type = context.i32_type();
        let callee_type = i32_type.fn_type(&[i32_type.into()], false);
        let callee = codegen.module.add_function("target_add", callee_type, None);

        let caller_type = context.void_type().fn_type(&[], false);
        let caller = codegen
            .module
            .add_function("caller_direct", caller_type, None);
        let entry = context.append_basic_block(caller, "entry");
        codegen.builder.position_at_end(entry);

        let arg = i32_type.const_int(5, false);
        let call_site = codegen.build_call(callee, &[arg.into()], "c_site");
        assert!(call_site.is_ok());
    }

    #[test]
    fn test_scalar_types_mapping() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_scalars", TargetTriple::host());

        assert_eq!(
            codegen.llvm_type(&Type::Int8).unwrap().to_string(),
            "\"i8\""
        );
        assert_eq!(
            codegen.llvm_type(&Type::Uint8).unwrap().to_string(),
            "\"i8\""
        );
        assert_eq!(
            codegen.llvm_type(&Type::Int16).unwrap().to_string(),
            "\"i16\""
        );
        assert_eq!(
            codegen.llvm_type(&Type::Uint16).unwrap().to_string(),
            "\"i16\""
        );
        assert_eq!(
            codegen.llvm_type(&Type::Int32).unwrap().to_string(),
            "\"i32\""
        );
        assert_eq!(
            codegen.llvm_type(&Type::Uint32).unwrap().to_string(),
            "\"i32\""
        );
        assert_eq!(
            codegen.llvm_type(&Type::Int64).unwrap().to_string(),
            "\"i64\""
        );
        assert_eq!(
            codegen.llvm_type(&Type::Uint64).unwrap().to_string(),
            "\"i64\""
        );
        assert_eq!(
            codegen.llvm_type(&Type::Float32).unwrap().to_string(),
            "\"float\""
        );
        assert_eq!(
            codegen.llvm_type(&Type::Float64).unwrap().to_string(),
            "\"double\""
        );
        assert_eq!(
            codegen.llvm_type(&Type::Bool).unwrap().to_string(),
            "\"i1\""
        );
        assert_eq!(
            codegen.llvm_type(&Type::Char).unwrap().to_string(),
            "\"i32\""
        );
    }

    #[test]
    fn test_c_int_types_by_target_model() {
        let context = Context::create();

        // LP64 (x86_64 linux): long is 64-bit, size_t is 64-bit
        let linux_gen = CodeGenerator::new(&context, "lp64", TargetTriple::X86_64UnknownLinuxGnu);
        assert_eq!(
            linux_gen
                .llvm_type(&Type::CInt(CIntKind::Long))
                .unwrap()
                .to_string(),
            "\"i64\""
        );
        assert_eq!(
            linux_gen
                .llvm_type(&Type::CInt(CIntKind::SizeT))
                .unwrap()
                .to_string(),
            "\"i64\""
        );

        // LLP64 (x86_64 windows): long is 32-bit, long long is 64-bit, size_t is 64-bit
        let win_gen = CodeGenerator::new(&context, "llp64", TargetTriple::X86_64PcWindowsGnu);
        assert_eq!(
            win_gen
                .llvm_type(&Type::CInt(CIntKind::Long))
                .unwrap()
                .to_string(),
            "\"i32\""
        );
        assert_eq!(
            win_gen
                .llvm_type(&Type::CInt(CIntKind::LongLong))
                .unwrap()
                .to_string(),
            "\"i64\""
        );
    }

    #[test]
    fn test_composite_strings_and_slices() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_composite", TargetTriple::host());

        let str_ty = codegen.llvm_type(&Type::String).unwrap();
        assert_eq!(str_ty.to_string(), "\"{ ptr, i64 }\"");

        let bytes_ty = codegen.llvm_type(&Type::Bytes).unwrap();
        assert_eq!(bytes_ty.to_string(), "\"{ ptr, i64, i64 }\"");

        let arr_ty = codegen
            .llvm_type(&Type::Array(Box::new(Type::Int32)))
            .unwrap();
        assert_eq!(arr_ty.to_string(), "\"{ ptr, i64, i64 }\"");
    }

    #[test]
    fn test_composite_tuples_and_interfaces() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_tuples", TargetTriple::host());

        let tuple_ty = codegen
            .llvm_type(&Type::Tuple(vec![Type::Int32, Type::Float64, Type::String]))
            .unwrap();
        assert_eq!(tuple_ty.to_string(), "\"{ i32, double, { ptr, i64 } }\"");

        let iface_ty = codegen
            .llvm_type(&Type::Interface("Reader".into()))
            .unwrap();
        assert_eq!(iface_ty.to_string(), "\"{ ptr, ptr }\"");

        let weak_ty = codegen
            .llvm_type(&Type::Weak(Box::new(Type::Int32)))
            .unwrap();
        assert_eq!(weak_ty.to_string(), "\"{ ptr, ptr }\"");

        let closure_ty = codegen
            .llvm_type(&Type::Fn {
                params: vec![],
                ret: Box::new(Type::Void),
                is_cfun: false,
            })
            .unwrap();
        assert_eq!(closure_ty.to_string(), "\"{ ptr, ptr }\"");

        let cfun_ty = codegen
            .llvm_type(&Type::Fn {
                params: vec![],
                ret: Box::new(Type::Void),
                is_cfun: true,
            })
            .unwrap();
        assert_eq!(cfun_ty.to_string(), "\"ptr\"");
    }

    #[test]
    fn test_struct_registry_and_layout() {
        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, "test_struct", TargetTriple::host());

        codegen.registry.add_struct(
            "Point",
            vec![
                ("x".to_string(), Type::Float64),
                ("y".to_string(), Type::Float64),
            ],
        );

        let pt_ty = codegen.llvm_type(&Type::Struct("Point".into())).unwrap();
        assert_eq!(
            pt_ty.to_string(),
            "\"%struct.Point = type { double, double }\""
        );
        assert_eq!(
            codegen.abi_size_of(&Type::Struct("Point".into())).unwrap(),
            16
        );
    }

    #[test]
    fn test_class_instance_and_reference_type() {
        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, "test_class", TargetTriple::host());

        codegen.registry.add_class(
            "Animal",
            None,
            vec![
                ("age".to_string(), Type::Int32),
                ("name".to_string(), Type::String),
            ],
        );

        // Class variables are always opaque pointers in Yukino
        let class_var_ty = codegen.llvm_type(&Type::Class("Animal".into())).unwrap();
        assert_eq!(class_var_ty.to_string(), "\"ptr\"");

        // Class instance has vtable at index 0 followed by fields
        let inst_ty = codegen.lowerer().class_instance_type("Animal").unwrap();
        assert_eq!(
            inst_ty.to_string(),
            "\"%class.Animal = type { ptr, i32, { ptr, i64 } }\""
        );
    }

    #[test]
    fn test_adt_enum_with_payload_sizing() {
        let context = Context::create();
        let mut codegen =
            CodeGenerator::new(&context, "test_enum", TargetTriple::X86_64UnknownLinuxGnu);

        // variant 0: Empty (0 bytes payload)
        // variant 1: Point { x: f64, y: f64 } (16 bytes payload)
        codegen.registry.add_enum(
            "Shape",
            vec![
                EnumVariantLayout {
                    name: "None".to_string(),
                    payload: vec![],
                    tag: 0,
                },
                EnumVariantLayout {
                    name: "Point".to_string(),
                    payload: vec![Type::Float64, Type::Float64],
                    tag: 1,
                },
            ],
        );

        let shape_ty = codegen.llvm_type(&Type::Enum("Shape".into())).unwrap();
        assert!(shape_ty.to_string().contains("{ i32, [16 x i8] }"));
        // Size on x86_64: 4 bytes tag + 16 bytes byte-array payload = 20 bytes (aligned to 4)
        assert_eq!(
            codegen.abi_size_of(&Type::Enum("Shape".into())).unwrap(),
            20
        );
    }

    #[test]
    fn test_nullable_null_pointer_optimization() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_npo", TargetTriple::host());

        // Pointer/reference types qualify for NPO -> raw ptr
        let null_class = codegen
            .nullable_type(&Type::Class("Person".into()))
            .unwrap();
        assert_eq!(null_class.to_string(), "\"ptr\"");

        let null_ptr = codegen
            .nullable_type(&Type::Ptr(Box::new(Type::Int32)))
            .unwrap();
        assert_eq!(null_ptr.to_string(), "\"ptr\"");

        let null_shared = codegen
            .nullable_type(&Type::Shared(Box::new(Type::Int32)))
            .unwrap();
        assert_eq!(null_shared.to_string(), "\"ptr\"");

        // Value types do not qualify for NPO -> { i1 is_some, T value }
        let null_int = codegen.nullable_type(&Type::Int64).unwrap();
        assert_eq!(null_int.to_string(), "\"{ i1, i64 }\"");

        let null_str = codegen.nullable_type(&Type::String).unwrap();
        assert_eq!(null_str.to_string(), "\"{ i1, { ptr, i64 } }\"");
    }

    #[test]
    fn test_general_tagged_union() {
        let context = Context::create();
        let codegen =
            CodeGenerator::new(&context, "test_union", TargetTriple::X86_64UnknownLinuxGnu);

        // Union of int32 (4 bytes) and string (16 bytes)
        let union_ty = codegen
            .llvm_type(&Type::Union(
                [Type::Int32, Type::String].into_iter().collect(),
            ))
            .unwrap();
        assert_eq!(union_ty.to_string(), "\"{ i32, [16 x i8] }\"");
    }

    #[test]
    fn test_ast_driven_module_type_registration() {
        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, "test_ast_reg", TargetTriple::host());
        let dummy_span = Span::new(FileId(0), BytePos(0), BytePos(0));

        let mut ast = AstModule::default();
        ast.items.push(Item::new(
            ItemId(1),
            Visibility::Public,
            ItemKind::Struct(StructDecl {
                name: Ident::new("Vector2".to_string(), dummy_span),
                fields: vec![
                    FieldDef {
                        name: Ident::new("x", dummy_span),
                        attrs: Vec::new(),
                        ty: TypeNode::new(
                            TypeNodeKind::Ident(Ident::new("float32", dummy_span)),
                            dummy_span,
                        ),
                        default_val: None,
                        span: dummy_span,
                    },
                    FieldDef {
                        name: Ident::new("y", dummy_span),
                        attrs: Vec::new(),
                        ty: TypeNode::new(
                            TypeNodeKind::Ident(Ident::new("float32", dummy_span)),
                            dummy_span,
                        ),
                        default_val: None,
                        span: dummy_span,
                    },
                ],
                methods: Vec::new(),
                static_methods: Vec::new(),
                span: dummy_span,
            }),
            dummy_span,
        ));

        codegen.register_ast_types(&ast);
        let vec2_ty = codegen.llvm_type(&Type::Struct("Vector2".into())).unwrap();
        assert_eq!(
            vec2_ty.to_string(),
            "\"%struct.Vector2 = type { float, float }\""
        );
    }

    fn make_test_fun(id: u32, stmt_count: usize) -> Item {
        let dummy_span = Span::new(FileId(0), BytePos(0), BytePos(0));
        let stmts = (0..stmt_count)
            .map(|_| yukic_ast::Stmt::new(yukic_ast::StmtKind::Space, dummy_span))
            .collect();
        Item::new(
            ItemId(id),
            Visibility::Public,
            ItemKind::Fun(yukic_ast::FunDecl {
                name: yukic_ast::Ident::new(format!("fn_{id}"), dummy_span),
                attrs: Vec::new(),
                generics: Vec::new(),
                params: Vec::new(),
                return_type: None,
                body: yukic_ast::Block::new(stmts, dummy_span),
                span: dummy_span,
            }),
            dummy_span,
        )
    }

    #[test]
    fn test_estimate_item_weight() {
        let f = make_test_fun(1, 5);
        assert_eq!(estimate_item_weight(&f), 6);

        let dummy_span = Span::new(FileId(0), BytePos(0), BytePos(0));
        let err_item = Item::new(ItemId(2), Visibility::Public, ItemKind::Error, dummy_span);
        assert_eq!(estimate_item_weight(&err_item), 0);
    }

    #[test]
    fn test_partition_greedy_lpt_balancing() {
        let mut ast = AstModule::default();
        ast.items.push(make_test_fun(1, 9));
        ast.items.push(make_test_fun(2, 7));
        ast.items.push(make_test_fun(3, 5));
        ast.items.push(make_test_fun(4, 3));

        let units = partition_module(&ast, 2);
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].estimated_weight, 14);
        assert_eq!(units[1].estimated_weight, 14);
    }

    #[test]
    fn test_partition_module_clamps_empty_units() {
        let mut ast = AstModule::default();
        ast.items.push(make_test_fun(1, 2));
        ast.items.push(make_test_fun(2, 2));

        let units = partition_module(&ast, 8);
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].item_ids.len(), 1);
        assert_eq!(units[1].item_ids.len(), 1);
    }

    #[test]
    fn test_partition_module_empty_ast() {
        let ast = AstModule::default();
        let units = partition_module(&ast, 4);
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].name, "cgu_0");
        assert!(units[0].item_ids.is_empty());
    }

    #[test]
    fn test_partition_module_with_custom_mapping() {
        use std::collections::HashMap;
        use std::path::PathBuf;

        let mut ast = AstModule::default();
        ast.items.push(make_test_fun(1, 5));
        ast.items.push(make_test_fun(2, 5));
        ast.items.push(make_test_fun(3, 5));

        let mut sources = HashMap::new();
        sources.insert(ItemId(1), PathBuf::from("src/io.ykn"));
        sources.insert(ItemId(2), PathBuf::from("src/fs/mod.ykn"));
        sources.insert(ItemId(3), PathBuf::from("src/other.ykn"));

        let mut custom = HashMap::new();
        custom.insert("io".to_string(), vec!["src/io".to_string()]);
        custom.insert("fs".to_string(), vec!["src/fs".to_string()]);

        let units = partition_module_with_custom(&ast, 4, Some(&custom), Some(&sources));
        assert_eq!(units.len(), 3);
        assert_eq!(units[0].name, "cgu_fs");
        assert_eq!(units[0].item_ids, vec![ItemId(2)]);
        assert_eq!(units[1].name, "cgu_io");
        assert_eq!(units[1].item_ids, vec![ItemId(1)]);
        assert_eq!(units[2].name, "cgu_misc_0");
        assert_eq!(units[2].item_ids, vec![ItemId(3)]);
    }

    #[test]
    fn test_rodata_string_literal_and_deduplication() {
        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, "test_str", TargetTriple::host());

        let s1 = codegen.emit_string_literal("hello world");
        let s2 = codegen.emit_string_literal("hello world");
        assert_eq!(s1, s2);
        assert_eq!(codegen.rodata.string_count(), 1);

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains(
            "@.str.anon.0 = private unnamed_addr constant [12 x i8] c\"hello world\\00\", align 1"
        ));
    }

    #[test]
    fn test_rodata_bytes_literal_and_deduplication() {
        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, "test_bytes", TargetTriple::host());

        let bytes_data = [0x7f, 0x45, 0x4c, 0x46]; // ELF magic
        let b1 = codegen.emit_bytes_literal(&bytes_data);
        let b2 = codegen.emit_bytes_literal(&bytes_data);
        assert_eq!(b1, b2);
        assert_eq!(codegen.rodata.bytes_count(), 1);

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("@.bytes.anon.0 = private unnamed_addr constant [4 x i8]"));
    }

    #[test]
    fn test_rodata_typeof_intrinsic_emission() {
        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, "test_typeof", TargetTriple::host());

        let t1 = codegen.emit_typeof(&Type::Int64);
        let t2 = codegen.emit_typeof(&Type::Int64);
        assert_eq!(t1, t2);
        assert_eq!(codegen.rodata.typeof_count(), 1);

        let t3 = codegen.emit_typeof(&Type::Array(Box::new(Type::String)));
        assert_eq!(codegen.rodata.typeof_count(), 2);
        assert_ne!(t1, t3);

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains(
            "@.str.type.0 = private unnamed_addr constant [6 x i8] c\"int64\\00\", align 1"
        ));
        assert!(ir.contains(
            "@.str.type.1 = private unnamed_addr constant [9 x i8] c\"string[]\\00\", align 1"
        ));
    }

    #[test]
    fn test_compile_module_static_and_ll_emission() {
        use yukic_ast::expr::{Expr, ExprKind, LiteralValue};
        use yukic_ast::item::StaticDecl;

        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, "test_static", TargetTriple::host());
        let dummy_span = Span::new(FileId(0), BytePos(0), BytePos(0));

        let mut ast = AstModule::default();
        ast.items.push(Item::new(
            ItemId(1),
            Visibility::Public,
            ItemKind::Static(StaticDecl {
                name: Ident::new("G_MESSAGE", dummy_span),
                ty: None,
                init: Expr::new(
                    ExprKind::Literal(LiteralValue::String("hello from yuki".to_string())),
                    dummy_span,
                ),
                span: dummy_span,
            }),
            dummy_span,
        ));

        assert!(codegen.compile_module(&ast).is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("@G_MESSAGE = constant { ptr, i64 } { ptr @.str.anon.0, i64 15 }"));
    }

    #[test]
    fn test_region_type_and_intrinsics_declaration() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_reg_type", TargetTriple::host());

        let reg_type = codegen.region_type();
        assert_eq!(reg_type.count_fields(), 3);
        assert!(codegen.module.get_function("yukino_region_init").is_some());
        assert!(codegen.module.get_function("yukino_region_alloc").is_some());
        assert!(
            codegen
                .module
                .get_function("yukino_region_destroy")
                .is_some()
        );
        assert!(
            codegen
                .module
                .get_function("yukino_region_save_mark")
                .is_some()
        );
        assert!(
            codegen
                .module
                .get_function("yukino_region_restore_mark")
                .is_some()
        );
    }

    #[test]
    fn test_is_regional_type_classification() {
        assert!(CodeGenerator::is_regional_type(&Type::String));
        assert!(CodeGenerator::is_regional_type(&Type::Bytes));
        assert!(CodeGenerator::is_regional_type(&Type::Array(Box::new(
            Type::Int64
        ))));
        assert!(CodeGenerator::is_regional_type(&Type::Class(
            "User".to_string()
        )));
        assert!(CodeGenerator::is_regional_type(&Type::Interface(
            "Printable".to_string()
        )));
        assert!(CodeGenerator::is_regional_type(&Type::nullable(
            Type::String
        )));

        assert!(!CodeGenerator::is_regional_type(&Type::Int64));
        assert!(!CodeGenerator::is_regional_type(&Type::Float64));
        assert!(!CodeGenerator::is_regional_type(&Type::Bool));
        assert!(!CodeGenerator::is_regional_type(&Type::Char));
        assert!(!CodeGenerator::is_regional_type(&Type::Ptr(Box::new(
            Type::Int32
        ))));
        assert!(!CodeGenerator::is_regional_type(&Type::Void));
    }

    #[test]
    fn test_dps_signature_transformation() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_dps_sig", TargetTriple::host());

        let (fn_ty_str, has_dest) = codegen.transform_function_type(
            &[context.i64_type().into()],
            &Type::String,
            Some(codegen.string_type().into()),
            false,
        );
        assert!(has_dest);
        assert_eq!(fn_ty_str.count_param_types(), 2);

        let (fn_ty_int, has_dest_int) = codegen.transform_function_type(
            &[context.i64_type().into()],
            &Type::Int64,
            Some(context.i64_type().into()),
            false,
        );
        assert!(!has_dest_int);
        assert_eq!(fn_ty_int.count_param_types(), 1);

        let (cfun_ty, has_dest_c) = codegen.transform_function_type(
            &[context.i64_type().into()],
            &Type::String,
            Some(codegen.string_type().into()),
            true,
        );
        assert!(!has_dest_c);
        assert_eq!(cfun_ty.count_param_types(), 1);
    }

    #[test]
    fn test_fast_path_bump_allocation_ir() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_bump", TargetTriple::host());

        let void_ty = context.void_type().fn_type(&[], false);
        let fn_val = codegen.module.add_function("test_fn", void_ty, None);
        let entry = context.append_basic_block(fn_val, "entry");
        codegen.builder.position_at_end(entry);

        let reg = codegen
            .emit_region_prologue(4096, "test_reg")
            .expect("prologue");
        let alloc_ptr = codegen
            .emit_fast_path_alloc(fn_val, reg, 128, 16)
            .expect("alloc");
        assert_eq!(alloc_ptr.get_type(), codegen.ptr_type());

        codegen.emit_region_epilogue(reg).expect("epilogue");
        codegen.builder.build_return(None).unwrap();

        assert!(codegen.module.verify().is_ok());
        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("bump_fast:"));
        assert!(ir.contains("bump_slow:"));
        assert!(ir.contains("bump_merge:"));
        assert!(ir.contains("phi ptr [ %aligned_cur_ptr, %bump_fast ], [ %slow_ptr, %bump_slow ]"));
    }

    #[test]
    fn test_loop_mark_ir_emission() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_loop", TargetTriple::host());

        let void_ty = context.void_type().fn_type(&[], false);
        let fn_val = codegen.module.add_function("test_loop_fn", void_ty, None);
        let entry = context.append_basic_block(fn_val, "entry");
        codegen.builder.position_at_end(entry);

        let reg = codegen
            .emit_region_prologue(4096, "local_reg")
            .expect("prologue");
        let mark = codegen.emit_loop_save_mark(reg).expect("save_mark");
        codegen
            .emit_loop_restore_mark(reg, mark)
            .expect("restore_mark");
        codegen.emit_region_epilogue(reg).expect("epilogue");
        codegen.builder.build_return(None).unwrap();

        assert!(codegen.module.verify().is_ok());
        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("call ptr @yukino_region_save_mark(ptr %local_reg)"));
        assert!(
            ir.contains("call void @yukino_region_restore_mark(ptr %local_reg, ptr %loop_mark)")
        );
    }

    #[test]
    fn test_select_region_for_escape() {
        use yukic_borrow::EscapeKind;

        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_esc", TargetTriple::host());

        let local_reg = codegen.ptr_type().const_null();
        let dest_reg = context
            .i64_type()
            .const_int(0x1000, false)
            .const_to_pointer(codegen.ptr_type());

        let r1 = codegen.select_region_for_escape(EscapeKind::NoEscape, local_reg, Some(dest_reg));
        assert_eq!(r1, local_reg);

        let r2 =
            codegen.select_region_for_escape(EscapeKind::LoopEscape, local_reg, Some(dest_reg));
        assert_eq!(r2, dest_reg);

        let r3 =
            codegen.select_region_for_escape(EscapeKind::EscapeToCaller, local_reg, Some(dest_reg));
        assert_eq!(r3, dest_reg);
    }

    fn dummy_ident(name: &str) -> yukic_ast::Ident {
        yukic_ast::Ident {
            name: name.to_string(),
            span: yukic_span::Span::DUMMY,
        }
    }

    fn dummy_method(name: &str) -> yukic_ast::item::MethodDecl {
        yukic_ast::item::MethodDecl {
            name: dummy_ident(name),
            attrs: Vec::new(),
            this_param: yukic_ast::item::ThisParam::Value,
            params: Vec::new(),
            return_type: None,
            body: yukic_ast::Block {
                stmts: Vec::new(),
                span: yukic_span::Span::DUMMY,
            },
            is_operator: false,
            is_override: false,
            span: yukic_span::Span::DUMMY,
        }
    }

    fn dummy_class(name: &str, base: Option<&str>, methods: &[&str]) -> yukic_ast::item::ClassDecl {
        yukic_ast::item::ClassDecl {
            name: dummy_ident(name),
            is_locked: false,
            base_class: base.map(dummy_ident),
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: methods.iter().map(|m| dummy_method(m)).collect(),
            accessors: Vec::new(),
            span: yukic_span::Span::DUMMY,
        }
    }

    fn dummy_interface(name: &str, methods: &[&str]) -> yukic_ast::item::InterfaceDecl {
        yukic_ast::item::InterfaceDecl {
            name: dummy_ident(name),
            generics: Vec::new(),
            methods: methods
                .iter()
                .map(|m| yukic_ast::item::InterfaceMethodDef {
                    name: dummy_ident(m),
                    attrs: Vec::new(),
                    params: Vec::new(),
                    return_type: None,
                    default_body: None,
                    span: yukic_span::Span::DUMMY,
                })
                .collect(),
            span: yukic_span::Span::DUMMY,
        }
    }

    #[test]
    fn test_vtable_layout_inheritance() {
        let context = Context::create();
        let mut vm = VTableManager::new(&context);

        let animal = dummy_class("Animal", None, &["speak"]);
        let dog = dummy_class("Dog", Some("Animal"), &["speak", "bark"]);
        let cat = dummy_class("Cat", Some("Animal"), &["meow"]);

        vm.register_class(&animal, 101);
        vm.register_class(&dog, 102);
        vm.register_class(&cat, 103);

        let a_layout = vm.get_layout("Animal").unwrap();
        assert_eq!(a_layout.slots.len(), 1);
        assert_eq!(a_layout.slots[0].method_name, "speak");
        assert_eq!(a_layout.slots[0].mangled_symbol, "_Y_Animal_speak");

        let d_layout = vm.get_layout("Dog").unwrap();
        assert_eq!(d_layout.slots.len(), 2);
        assert_eq!(d_layout.slots[0].method_name, "speak");
        assert_eq!(d_layout.slots[0].mangled_symbol, "_Y_Dog_speak");
        assert_eq!(d_layout.slots[1].method_name, "bark");
        assert_eq!(d_layout.slots[1].mangled_symbol, "_Y_Dog_bark");

        let c_layout = vm.get_layout("Cat").unwrap();
        assert_eq!(c_layout.slots.len(), 2);
        assert_eq!(c_layout.slots[0].method_name, "speak");
        assert_eq!(c_layout.slots[0].mangled_symbol, "_Y_Animal_speak");
        assert_eq!(c_layout.slots[1].method_name, "meow");
        assert_eq!(c_layout.slots[1].mangled_symbol, "_Y_Cat_meow");
    }

    #[test]
    fn test_vtable_emission_and_dispatch() {
        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, "test_vt", TargetTriple::host());

        let animal = dummy_class("Animal", None, &["speak"]);
        let dog = dummy_class("Dog", Some("Animal"), &["speak", "bark"]);

        codegen.vtable.register_class(&animal, 201);
        codegen.vtable.register_class(&dog, 202);

        let dog_vt = codegen.emit_class_vtable("Dog").expect("dog vtable");
        let animal_vt = codegen.emit_class_vtable("Animal").expect("animal vtable");

        assert!(dog_vt.is_constant());
        assert!(animal_vt.is_constant());

        let fn_ty = context
            .void_type()
            .fn_type(&[codegen.ptr_type().into()], false);
        let test_fn = codegen.module.add_function("test_run", fn_ty, None);
        let entry = context.append_basic_block(test_fn, "entry");
        codegen.builder.position_at_end(entry);

        let inst = test_fn.get_first_param().unwrap().into_pointer_value();
        codegen.init_instance_vtable(inst, dog_vt).expect("init vt");

        let vcall1 = codegen.emit_virtual_call("Dog", "speak", inst, fn_ty, &[]);
        assert!(vcall1.is_ok());

        let vcall2 = codegen.emit_virtual_call("Dog", "bark", inst, fn_ty, &[]);
        assert!(vcall2.is_ok());

        let base_fn = codegen.module.get_function("_Y_Animal_speak").unwrap();
        let scall = codegen.emit_super_call(base_fn, inst, &[]);
        assert!(scall.is_ok());

        codegen.builder.build_return(None).unwrap();
        assert!(codegen.module.verify().is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("@_Y_VTABLE_Dog"));
        assert!(ir.contains("@_Y_VTABLE_Animal"));
        assert!(ir.contains("call void @_Y_Animal_speak(ptr %0)"));
    }

    #[test]
    fn test_itable_emission_and_dispatch() {
        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, "test_it", TargetTriple::host());

        let dog = dummy_class("Dog", None, &["speak"]);
        let speaker = dummy_interface("Speaker", &["speak"]);

        codegen.vtable.register_class(&dog, 301);
        codegen
            .vtable
            .register_interface_implementation("Dog", &speaker);

        let itable_g = codegen
            .emit_interface_itable("Dog", "Speaker")
            .expect("itable");
        assert!(itable_g.is_constant());

        let fn_ty = context
            .void_type()
            .fn_type(&[codegen.ptr_type().into()], false);
        let test_fn = codegen.module.add_function("test_iface", fn_ty, None);
        let entry = context.append_basic_block(test_fn, "entry");
        codegen.builder.position_at_end(entry);

        let inst = test_fn.get_first_param().unwrap().into_pointer_value();
        let fat_val = codegen
            .make_interface_fat_pointer(inst, itable_g)
            .expect("fat ptr");

        let slot = codegen.get_interface_slot("Speaker", "speak").unwrap();
        let icall = codegen.emit_interface_call(fat_val, slot, fn_ty, &[]);
        assert!(icall.is_ok());

        codegen.builder.build_return(None).unwrap();
        assert!(codegen.module.verify().is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("@_Y_ITABLE_Dog_Speaker"));
        assert!(ir.contains("extractvalue { ptr, ptr }"));
    }

    #[test]
    fn test_closure_env_layout_and_fat_pointer() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_closure_layout", TargetTriple::host());

        let captures = vec![
            CapturedVar::new("count", Type::Int64),
            CapturedVar::new("msg", Type::String),
        ];

        let layout = codegen
            .create_closure_env_layout("greeter", &captures)
            .expect("env layout");

        assert_eq!(layout.field_names, vec!["count", "msg"]);
        assert_eq!(layout.field_types.len(), 2);
        assert_eq!(layout.field_types[0].to_string(), "\"i64\"");
        assert_eq!(layout.field_types[1].to_string(), "\"{ ptr, i64 }\"");
        assert!(layout.size_bytes >= 24);
        assert!(layout.align_bytes >= 8);

        let ptr_ty = codegen.ptr_type();
        let fn_ty = context
            .void_type()
            .fn_type(&[ptr_ty.into(), ptr_ty.into()], false);
        let test_fn = codegen.module.add_function("test_fat_fn", fn_ty, None);
        let entry = context.append_basic_block(test_fn, "entry");
        codegen.builder.position_at_end(entry);

        let fn_arg = test_fn.get_first_param().unwrap().into_pointer_value();
        let env_arg = test_fn.get_params()[1].into_pointer_value();

        let fat_ptr = codegen
            .make_closure_fat_pointer(fn_arg, env_arg, "c_val")
            .expect("make fat ptr");

        let fn_field = codegen
            .builder
            .build_extract_value(fat_ptr, 0, "fn_part")
            .unwrap();
        let env_field = codegen
            .builder
            .build_extract_value(fat_ptr, 1, "env_part")
            .unwrap();

        assert_eq!(fn_field.get_type(), codegen.ptr_type().into());
        assert_eq!(env_field.get_type(), codegen.ptr_type().into());

        codegen.builder.build_return(None).unwrap();
        assert!(codegen.module.verify().is_ok());

        assert_eq!(
            layout.struct_type.to_string(),
            "\"%struct.env_greeter = type { i64, { ptr, i64 } }\""
        );
        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("insertvalue { ptr, ptr } undef, ptr %0, 0"));
        assert!(ir.contains("insertvalue { ptr, ptr } %c_val_fn, ptr %1, 1"));
    }

    #[test]
    fn test_closure_allocation_and_unpacking() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_closure_alloc", TargetTriple::host());

        let void_ty = context.void_type().fn_type(&[], false);
        let test_fn = codegen.module.add_function("test_alloc_fn", void_ty, None);
        let entry = context.append_basic_block(test_fn, "entry");
        codegen.builder.position_at_end(entry);

        let reg = codegen.emit_region_prologue(4096, "reg").expect("prologue");

        let captures = vec![
            CapturedVar::new("count", Type::Int64),
            CapturedVar::new("active", Type::Bool),
        ];
        let layout = codegen
            .create_closure_env_layout("worker", &captures)
            .expect("layout");

        let val_count = context.i64_type().const_int(42, false).into();
        let val_active = context.bool_type().const_int(1, false).into();

        let env_ptr = codegen
            .allocate_and_init_closure_env(test_fn, reg, &layout, &[val_count, val_active])
            .expect("alloc env");

        let unpacked = codegen
            .unpack_closure_environment(env_ptr, &layout)
            .expect("unpack");

        assert_eq!(unpacked.len(), 2);
        assert_eq!(unpacked[0].0, "count");
        assert_eq!(unpacked[1].0, "active");

        codegen.emit_region_epilogue(reg).expect("epilogue");
        codegen.builder.build_return(None).unwrap();

        assert!(codegen.module.verify().is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("%struct.env_worker = type { i64, i1 }"));
        assert!(ir.contains("store i64 42, ptr %cap_gep_0"));
        assert!(ir.contains("store i1 true, ptr %cap_gep_1"));
        assert!(ir.contains("%count = load i64, ptr %count_gep"));
        assert!(ir.contains("%active = load i1, ptr %active_gep"));
    }

    #[test]
    fn test_closure_indirect_call_and_invoke() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_closure_call", TargetTriple::host());

        // Target lambda signature: fastcc i64(ptr %env_ptr, i64 %y)
        let i64_type = context.i64_type();
        let lambda_ty = i64_type.fn_type(&[codegen.ptr_type().into(), i64_type.into()], false);
        let lambda_fn = codegen.module.add_function("_Y_lambda_0", lambda_ty, None);
        lambda_fn.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32);

        // Populate lambda body: load from env_ptr and add
        let lambda_entry = context.append_basic_block(lambda_fn, "entry");
        codegen.builder.position_at_end(lambda_entry);
        let env_arg = lambda_fn.get_first_param().unwrap().into_pointer_value();
        let y_arg = lambda_fn.get_params()[1].into_int_value();
        let load_val = codegen
            .build_load(i64_type, env_arg, "env_val")
            .unwrap()
            .into_int_value();
        let sum = codegen
            .builder
            .build_int_add(load_val, y_arg, "sum")
            .unwrap();
        codegen.builder.build_return(Some(&sum)).unwrap();

        // Caller function
        let caller_ty = context.void_type().fn_type(&[], false);
        let caller_fn = codegen.module.add_function("caller", caller_ty, None);
        let caller_entry = context.append_basic_block(caller_fn, "entry");
        codegen.builder.position_at_end(caller_entry);

        let env_ptr = codegen.ptr_type().const_null();
        let fat_val = codegen
            .make_closure_fat_pointer(
                lambda_fn.as_global_value().as_pointer_value(),
                env_ptr,
                "c1",
            )
            .expect("fat");

        // Dynamic indirect call: store in alloca and reload to prevent constant folding
        let alloca = codegen
            .builder
            .build_alloca(codegen.closure_type(), "c_alloca")
            .unwrap();
        codegen.builder.build_store(alloca, fat_val).unwrap();
        let dyn_fat = codegen
            .build_load(codegen.closure_type(), alloca, "dyn_fat")
            .unwrap()
            .into_struct_value();

        let y_val = i64_type.const_int(15, false);
        let dyn_call = codegen.emit_closure_call(dyn_fat, lambda_ty, &[y_val.into()], None, "res1");
        assert!(dyn_call.is_ok());

        // Indirect Invoke with personality
        let personality = codegen.setup_personality_function(caller_fn);
        assert!(caller_fn.has_personality_function());
        assert_eq!(
            personality.get_name().to_str().unwrap(),
            "yukino_personality"
        );

        let then_block = context.append_basic_block(caller_fn, "then_blk");
        let unwind_block = context.append_basic_block(caller_fn, "unwind_blk");

        let invoke_res = codegen.emit_closure_invoke(
            dyn_fat,
            lambda_ty,
            &[y_val.into()],
            then_block,
            unwind_block,
            None,
            "res2",
        );
        assert!(invoke_res.is_ok());

        // Complete blocks
        codegen.builder.position_at_end(then_block);
        codegen.builder.build_return(None).unwrap();

        codegen.builder.position_at_end(unwind_block);
        let eh_val = codegen
            .emit_landing_pad(personality, &[], true, true, "eh_res")
            .expect("landing pad");
        codegen.emit_resume(eh_val).expect("resume");

        assert!(codegen.module.verify().is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("%res1_fn_ptr = extractvalue { ptr, ptr } %dyn_fat, 0"));
        assert!(ir.contains("%res1_env_ptr = extractvalue { ptr, ptr } %dyn_fat, 1"));
        assert!(ir.contains("call fastcc i64 %res1_fn_ptr(ptr %res1_env_ptr, i64 15)"));
        assert!(ir.contains("%res2_fn_ptr = extractvalue { ptr, ptr } %dyn_fat, 0"));
        assert!(ir.contains("%res2_env_ptr = extractvalue { ptr, ptr } %dyn_fat, 1"));
        assert!(ir.contains("invoke fastcc i64 %res2_fn_ptr(ptr %res2_env_ptr, i64 15)"));
        assert!(ir.contains("to label %then_blk unwind label %unwind_blk"));
        assert!(ir.contains("landingpad { ptr, i32 }"));
        assert!(ir.contains("cleanup"));
        assert!(ir.contains("catch ptr null"));
        assert!(ir.contains("resume { ptr, i32 } %eh_res"));
    }

    #[test]
    fn test_scope_stack_return_and_loop_cleanups() {
        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, "test_scopes", TargetTriple::host());

        let dummy_span = yukic_span::Span::DUMMY;
        let fn_ty = context.void_type().fn_type(&[], false);
        let test_fn = codegen.module.add_function("test_fn", fn_ty, None);
        let break_bb = context.append_basic_block(test_fn, "break_bb");
        let continue_bb = context.append_basic_block(test_fn, "cont_bb");

        // Function Scope with region epilogue
        codegen.push_scope(ScopeKind::Function);
        let dummy_reg = codegen.ptr_type().const_null();
        codegen
            .add_cleanup(CleanupAction::RegionEpilogue(dummy_reg))
            .expect("add cleanup");

        // Outer Block Scope with defer 1
        codegen.push_scope(ScopeKind::Block);
        let defer1 = yukic_ast::Block::new(vec![], dummy_span);
        codegen
            .add_cleanup(CleanupAction::Defer(defer1))
            .expect("add defer 1");

        // Loop Scope
        codegen.push_scope(ScopeKind::Loop {
            label: None,
            break_target: break_bb,
            continue_target: continue_bb,
        });

        // Inner Block Scope inside loop with defer 2
        codegen.push_scope(ScopeKind::Block);
        let defer2 = yukic_ast::Block::new(vec![], dummy_span);
        codegen
            .add_cleanup(CleanupAction::Defer(defer2))
            .expect("add defer 2");

        // Break cleanups: only cleanups inside loop scope!
        let (loop_cleanups, target_bb) = codegen
            .collect_loop_cleanups(true)
            .expect("collect loop cleanups");
        assert_eq!(loop_cleanups.len(), 1);
        assert!(matches!(loop_cleanups[0], CleanupAction::Defer(_)));
        assert_eq!(target_bb, break_bb);

        // Continue cleanups
        let (_, cont_target) = codegen
            .collect_loop_cleanups(false)
            .expect("collect continue cleanups");
        assert_eq!(cont_target, continue_bb);

        // Return cleanups: all cleanups across all scopes in LIFO order!
        let ret_cleanups = codegen.collect_return_cleanups();
        assert_eq!(ret_cleanups.len(), 3);
        assert!(matches!(ret_cleanups[0], CleanupAction::Defer(_)));
        assert!(matches!(ret_cleanups[1], CleanupAction::Defer(_)));
        assert!(matches!(ret_cleanups[2], CleanupAction::RegionEpilogue(_)));

        codegen.pop_scope();
        codegen.pop_scope();
        codegen.pop_scope();
        codegen.pop_scope();
        assert!(codegen.scopes.is_empty());
    }

    #[test]
    fn test_sus_catch_full_dispatch_with_typeinfo() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_sus_catch", TargetTriple::host());

        let i32_type = context.i32_type();
        let fn_ty = i32_type.fn_type(&[], false);
        let test_fn = codegen.module.add_function("test_sus_fn", fn_ty, None);
        let personality = codegen.setup_personality_function(test_fn);

        let entry = context.append_basic_block(test_fn, "entry");
        let then_block = context.append_basic_block(test_fn, "sus_cont_ok");
        let landing_block = context.append_basic_block(test_fn, "sus_landing");
        let catch_net = context.append_basic_block(test_fn, "catch_network");
        let catch_all = context.append_basic_block(test_fn, "catch_all");
        let cleanup_resume = context.append_basic_block(test_fn, "cleanup_resume");
        let normal_exit = context.append_basic_block(test_fn, "normal_exit");

        // Entry block: invoke a risky callee
        codegen.builder.position_at_end(entry);
        let risky_fn = codegen.module.add_function("risky_call", fn_ty, None);
        codegen
            .emit_invoke(risky_fn, &[], then_block, landing_block, "invoke_call")
            .unwrap();

        // Normal continuation block
        codegen.builder.position_at_end(then_block);
        codegen
            .builder
            .build_unconditional_branch(normal_exit)
            .unwrap();

        // Landing pad block
        codegen.builder.position_at_end(landing_block);
        let typeinfo_net = codegen.get_or_create_typeinfo("NetworkError");
        let eh_val = codegen
            .emit_landing_pad(personality, &[typeinfo_net], true, true, "eh")
            .unwrap();

        let _exc_ptr = codegen
            .extract_exception_pointer(eh_val, "exc_ptr")
            .unwrap();
        let selector = codegen
            .extract_exception_selector(eh_val, "selector")
            .unwrap();

        // Check if selector matches NetworkError typeid
        let net_tid = codegen.emit_typeid_for(typeinfo_net, "net_tid").unwrap();
        let is_net = codegen
            .builder
            .build_int_compare(inkwell::IntPredicate::EQ, selector, net_tid, "is_net")
            .unwrap();

        let next_check = context.append_basic_block(test_fn, "check_catch_all");
        codegen
            .builder
            .build_conditional_branch(is_net, catch_net, next_check)
            .unwrap();

        // Catch-all branch
        codegen.builder.position_at_end(next_check);
        codegen
            .builder
            .build_unconditional_branch(catch_all)
            .unwrap();

        // Catch network handler
        codegen.builder.position_at_end(catch_net);
        codegen
            .builder
            .build_unconditional_branch(normal_exit)
            .unwrap();

        // Catch all handler
        codegen.builder.position_at_end(catch_all);
        codegen
            .builder
            .build_unconditional_branch(normal_exit)
            .unwrap();

        // Normal exit returns 0
        codegen.builder.position_at_end(normal_exit);
        codegen
            .builder
            .build_return(Some(&i32_type.const_int(0, false)))
            .unwrap();

        // Cleanup and resume block
        codegen.builder.position_at_end(cleanup_resume);
        codegen.emit_resume(eh_val).unwrap();

        assert!(codegen.module.verify().is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("@_Y_TYPEINFO_NetworkError = constant ptr null"));
        assert!(ir.contains("personality ptr @yukino_personality"));
        assert!(ir.contains("invoke i32 @risky_call()"));
        assert!(ir.contains("to label %sus_cont_ok unwind label %sus_landing"));
        assert!(ir.contains("landingpad { ptr, i32 }"));
        assert!(ir.contains("cleanup"));
        assert!(ir.contains("catch ptr @_Y_TYPEINFO_NetworkError"));
        assert!(ir.contains("catch ptr null"));
        assert!(ir.contains("call i32 @llvm.eh.typeid.for.p0(ptr @_Y_TYPEINFO_NetworkError)"));
        assert!(ir.contains("icmp eq i32 %selector, %net_tid"));
    }

    #[test]
    fn test_panic_emission() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_panic_mod", TargetTriple::host());

        let void_ty = context.void_type().fn_type(&[], false);
        let test_fn = codegen.module.add_function("trigger_panic", void_ty, None);
        let entry = context.append_basic_block(test_fn, "entry");
        codegen.builder.position_at_end(entry);

        let msg_ptr = codegen.ptr_type().const_null();
        let msg_len = context.i64_type().const_int(18, false);

        codegen.emit_panic(msg_ptr, msg_len).expect("panic");

        assert!(codegen.module.verify().is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("declare void @yukino_raise_panic(ptr, i64)"));
        assert!(ir.contains("call void @yukino_raise_panic(ptr null, i64 18)"));
        assert!(ir.contains("unreachable"));
    }

    #[test]
    fn test_cfun_calling_conventions_and_attributes() {
        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, "test_cfun_mod", TargetTriple::host());

        // cfun with stdcall and noreturn
        let dummy_span = yukic_span::Span::new(
            yukic_span::FileId(0),
            yukic_span::BytePos(0),
            yukic_span::BytePos(0),
        );
        let cfun_decl = yukic_ast::item::CfunDecl {
            name: yukic_ast::Ident::new("win32_exit", dummy_span),
            c_attrs: vec![
                yukic_ast::attr::CAttributeNode::new(
                    yukic_ast::Ident::new("stdcall", dummy_span),
                    None,
                    dummy_span,
                ),
                yukic_ast::attr::CAttributeNode::new(
                    yukic_ast::Ident::new("noreturn", dummy_span),
                    None,
                    dummy_span,
                ),
            ],
            yukino_attrs: Vec::new(),
            params: vec![yukic_ast::item::CParam {
                name: yukic_ast::Ident::new("code", dummy_span),
                ty: yukic_ast::TypeNode::new(
                    yukic_ast::TypeNodeKind::Ident(yukic_ast::Ident::new("int32", dummy_span)),
                    dummy_span,
                ),
                span: dummy_span,
            }],
            return_type: None,
            symbol_override: None,
            body: None,
            span: dummy_span,
        };

        let fn_val = codegen
            .compile_cfun_decl(&cfun_decl, true)
            .expect("cfun compile");
        assert_eq!(fn_val.get_call_conventions(), 64);

        // public cfun with fastcall and body
        let fast_cfun = yukic_ast::item::CfunDecl {
            name: yukic_ast::Ident::new("fast_add", dummy_span),
            c_attrs: vec![yukic_ast::attr::CAttributeNode::new(
                yukic_ast::Ident::new("fastcall", dummy_span),
                None,
                dummy_span,
            )],
            yukino_attrs: Vec::new(),
            params: vec![
                yukic_ast::item::CParam {
                    name: yukic_ast::Ident::new("a", dummy_span),
                    ty: yukic_ast::TypeNode::new(
                        yukic_ast::TypeNodeKind::Ident(yukic_ast::Ident::new("int32", dummy_span)),
                        dummy_span,
                    ),
                    span: dummy_span,
                },
                yukic_ast::item::CParam {
                    name: yukic_ast::Ident::new("b", dummy_span),
                    ty: yukic_ast::TypeNode::new(
                        yukic_ast::TypeNodeKind::Ident(yukic_ast::Ident::new("int32", dummy_span)),
                        dummy_span,
                    ),
                    span: dummy_span,
                },
            ],
            return_type: Some(yukic_ast::TypeNode::new(
                yukic_ast::TypeNodeKind::Ident(yukic_ast::Ident::new("int32", dummy_span)),
                dummy_span,
            )),
            symbol_override: Some(yukic_ast::Ident::new("custom_fast_add", dummy_span)),
            body: Some(yukic_ast::Block {
                stmts: Vec::new(),
                span: dummy_span,
            }),
            span: dummy_span,
        };

        let fast_fn = codegen
            .compile_cfun_decl(&fast_cfun, true)
            .expect("fast cfun");
        assert_eq!(fast_fn.get_call_conventions(), 65);
        assert_eq!(fast_fn.get_name().to_str().unwrap(), "custom_fast_add");

        assert!(codegen.module.verify().is_ok());
        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("declare x86_stdcallcc void @win32_exit(i32)"));
        assert!(ir.contains("define x86_fastcallcc i32 @custom_fast_add(i32 %0, i32 %1)"));
        assert!(ir.contains("noreturn"));
    }

    #[test]
    fn test_cstruct_layout_packed_and_natural() {
        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, "test_cstruct_mod", TargetTriple::host());

        // 1. Natural struct layout
        codegen.registry.add_cstruct(
            "Point",
            vec![
                ("x".to_string(), Type::Int32),
                ("y".to_string(), Type::Int64),
            ],
            false,
            None,
        );

        // 2. Packed struct layout
        codegen.registry.add_cstruct(
            "PackedPixel",
            vec![
                ("r".to_string(), Type::Uint8),
                ("g".to_string(), Type::Uint8),
                ("b".to_string(), Type::Uint8),
            ],
            true,
            None,
        );

        let point_ty = codegen.lower_named_cstruct("Point").expect("point");
        let pixel_ty = codegen.lower_named_cstruct("PackedPixel").expect("pixel");

        assert!(point_ty.to_string().contains("cstruct.Point"));
        assert!(pixel_ty.to_string().contains("cstruct.PackedPixel"));

        // Field operations
        let void_ty = context.void_type().fn_type(&[], false);
        let test_fn = codegen
            .module
            .add_function("test_cstruct_fields", void_ty, None);
        let entry = context.append_basic_block(test_fn, "entry");
        codegen.builder.position_at_end(entry);

        let pt_alloca = codegen.emit_cstruct_alloca(point_ty, "pt").expect("alloca");
        let _px_alloca = codegen.emit_cstruct_alloca(pixel_ty, "px").expect("alloca");
        let val_42 = context.i32_type().const_int(42, false).into();
        codegen
            .emit_cstruct_store_field(point_ty, pt_alloca, 0, val_42)
            .expect("store");

        let loaded = codegen
            .emit_cstruct_load_field(point_ty, pt_alloca, 0, context.i32_type().into(), "x_val")
            .expect("load");
        assert!(loaded.is_int_value());

        codegen.builder.build_return(None).unwrap();
        assert!(codegen.module.verify().is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("%cstruct.Point = type { i32, i64 }"));
        assert!(ir.contains("%cstruct.PackedPixel = type <{ i8, i8, i8 }>"));
        assert!(ir.contains("store i32 42, ptr %f_gep"));
        assert!(ir.contains("%x_val = load i32, ptr %x_val_gep"));
    }

    #[test]
    fn test_ptr_raw_pointer_operations() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_ptr_mod", TargetTriple::host());

        let void_ty = context.void_type().fn_type(&[], false);
        let test_fn = codegen.module.add_function("test_ptr_ops", void_ty, None);
        let entry = context.append_basic_block(test_fn, "entry");
        codegen.builder.position_at_end(entry);

        let i64_type = context.i64_type();
        let alloca = codegen.builder.build_alloca(i64_type, "cell").unwrap();

        // deref_set & deref
        let val = i64_type.const_int(999, false).into();
        codegen.emit_ptr_deref_set(alloca, val).expect("deref_set");
        let read_back = codegen
            .emit_ptr_deref(i64_type.into(), alloca, "v_read")
            .expect("deref");
        assert!(read_back.is_int_value());

        // offset
        let offset_bytes = i64_type.const_int(8, false);
        let off_ptr = codegen
            .emit_ptr_offset(alloca, offset_bytes, "off")
            .expect("offset");
        assert_eq!(off_ptr.get_type().to_string(), "\"ptr\"");

        // is_null
        let is_null_res = codegen.emit_ptr_is_null(alloca, "is_n").expect("is_null");
        assert_eq!(is_null_res.get_type().get_bit_width(), 1);

        // int_to_ptr & ptr_to_int
        let addr = codegen.emit_ptr_to_int(alloca, "addr").expect("to_int");
        let restored_ptr = codegen.emit_int_to_ptr(addr, "restored").expect("to_ptr");
        assert_eq!(restored_ptr.get_type().to_string(), "\"ptr\"");

        // as_fn_ptr callback conversion
        let callback_ptr = codegen.get_cfun_pointer(test_fn);
        assert_eq!(callback_ptr.get_type().to_string(), "\"ptr\"");

        codegen.builder.build_return(None).unwrap();
        assert!(codegen.module.verify().is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("store i64 999, ptr %cell"));
        assert!(ir.contains("%v_read = load i64, ptr %cell"));
        assert!(ir.contains("%off_int = ptrtoint ptr %cell to i64"));
        assert!(ir.contains("%off_add = add i64 %off_int, 8"));
        assert!(ir.contains("%off = inttoptr i64 %off_add to ptr"));
        assert!(ir.contains("%is_n = icmp eq ptr %cell, null"));
    }

    #[test]
    fn test_cstr_conversions() {
        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, "test_cstr_mod", TargetTriple::host());

        let void_ty = context.void_type().fn_type(&[], false);
        let worker_fn = codegen.module.add_function("worker", void_ty, None);
        let entry = context.append_basic_block(worker_fn, "entry");
        codegen.builder.position_at_end(entry);

        let local_reg = codegen
            .emit_region_prologue(4096, "local_reg")
            .expect("reg");

        // 1. to_cstr
        let test_str = codegen.rodata.emit_string(&codegen.module, "yukino_c");
        let c_str = codegen
            .emit_to_cstr(worker_fn, local_reg, test_str, "my_cstr")
            .expect("to_cstr");
        assert_eq!(c_str.get_type().to_string(), "\"ptr\"");

        // 2. from_cstr_n
        let len_10 = context.i64_type().const_int(10, false);
        let from_n = codegen
            .emit_from_cstr_n(c_str, len_10, "s_fixed")
            .expect("from_cstr_n");
        assert!(from_n.get_type().to_string().contains("{ ptr, i64 }"));

        // 3. from_cstr
        let from_dynamic = codegen
            .emit_from_cstr(c_str, "s_dynamic")
            .expect("from_cstr");
        assert!(from_dynamic.get_type().to_string().contains("{ ptr, i64 }"));

        codegen.emit_region_epilogue(local_reg).expect("epilogue");
        codegen.builder.build_return(None).unwrap();

        assert!(codegen.module.verify().is_ok());
        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("call void @llvm.memcpy"));
        assert!(ir.contains("store i8 0, ptr %null_ptr"));
        assert!(ir.contains("declare i64 @strlen(ptr)"));
        assert!(ir.contains("call i64 @strlen(ptr"));
    }

    #[test]
    fn test_intrinsic_exit() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_exit_mod", TargetTriple::host());

        let void_ty = context.void_type().fn_type(&[], false);
        let test_fn = codegen.module.add_function("abort_caller", void_ty, None);
        let entry = context.append_basic_block(test_fn, "entry");
        codegen.builder.position_at_end(entry);

        let code = context.i32_type().const_int(42, false);
        let unreachable_inst = codegen.emit_exit(code).expect("exit");
        assert_eq!(
            unreachable_inst.get_opcode(),
            inkwell::values::InstructionOpcode::Unreachable
        );

        assert!(codegen.module.verify().is_ok());
        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("declare void @exit(i32) #0"));
        assert!(ir.contains("call void @exit(i32 42) #0"));
        assert!(ir.contains("unreachable"));
        assert!(ir.contains("attributes #0 = { noreturn }"));
    }

    #[test]
    fn test_intrinsic_cast_primitives_and_overflow() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_cast_prim", TargetTriple::host());

        let void_ty = context.void_type().fn_type(&[], false);
        let test_fn = codegen.module.add_function("test_casts", void_ty, None);
        let entry = context.append_basic_block(test_fn, "entry");
        codegen.builder.position_at_end(entry);

        // 1. Narrowing and widening
        let p_i64 = codegen
            .builder
            .build_alloca(context.i64_type(), "p_i64")
            .unwrap();
        let i64_val = codegen
            .builder
            .build_load(context.i64_type(), p_i64, "v_i64")
            .unwrap();
        let trunc_res = codegen
            .emit_cast(i64_val, &Type::Int64, &Type::Int32, None)
            .expect("trunc");
        assert_eq!(trunc_res.into_int_value().get_type().get_bit_width(), 32);

        let p_i8 = codegen
            .builder
            .build_alloca(context.i8_type(), "p_i8")
            .unwrap();
        let i8_val = codegen
            .builder
            .build_load(context.i8_type(), p_i8, "v_i8")
            .unwrap();
        let sext_res = codegen
            .emit_cast(i8_val, &Type::Int8, &Type::Int32, None)
            .expect("sext");
        assert_eq!(sext_res.into_int_value().get_type().get_bit_width(), 32);

        // 2. Float and Int conversions
        let p_f64 = codegen
            .builder
            .build_alloca(context.f64_type(), "p_f64")
            .unwrap();
        let f64_val = codegen
            .builder
            .build_load(context.f64_type(), p_f64, "v_f64")
            .unwrap();
        let fptosi_res = codegen
            .emit_cast(f64_val, &Type::Float64, &Type::Int32, None)
            .expect("fptosi");
        assert_eq!(fptosi_res.into_int_value().get_type().get_bit_width(), 32);

        let sitofp_res = codegen
            .emit_cast(trunc_res, &Type::Int32, &Type::Float64, None)
            .expect("sitofp");
        assert!(sitofp_res.is_float_value());

        // 3. Pointer and Int
        let alloca = codegen
            .builder
            .build_alloca(context.i64_type(), "p_cell")
            .unwrap();
        let ptr_to_int_res = codegen
            .emit_cast(
                alloca.into(),
                &Type::Ptr(Box::new(Type::Int64)),
                &Type::Uint64,
                None,
            )
            .expect("ptrtoint");
        assert_eq!(
            ptr_to_int_res.into_int_value().get_type().get_bit_width(),
            64
        );

        let int_to_ptr_res = codegen
            .emit_cast(
                ptr_to_int_res,
                &Type::Uint64,
                &Type::Ptr(Box::new(Type::Int64)),
                None,
            )
            .expect("inttoptr");
        assert!(int_to_ptr_res.is_pointer_value());

        // 4. Two-step Pointer <-> Float64
        let ptr_to_f64 = codegen
            .emit_cast(
                alloca.into(),
                &Type::Ptr(Box::new(Type::Int64)),
                &Type::Float64,
                None,
            )
            .expect("ptr_to_f64");
        assert!(ptr_to_f64.is_float_value());

        let f64_to_ptr = codegen
            .emit_cast(
                ptr_to_f64,
                &Type::Float64,
                &Type::Ptr(Box::new(Type::Int64)),
                None,
            )
            .expect("f64_to_ptr");
        assert!(f64_to_ptr.is_pointer_value());

        // Char casts
        let i32_to_char = codegen
            .emit_cast(sext_res, &Type::Int32, &Type::Char, None)
            .expect("i32_to_char");
        assert!(i32_to_char.is_int_value());
        let char_to_i64 = codegen
            .emit_cast(i32_to_char, &Type::Char, &Type::Int64, None)
            .expect("char_to_i64");
        assert!(char_to_i64.is_int_value());

        // 5. Constant overflow detection with clear message
        let huge_val = context.i64_type().const_int(5_000_000_000, false).into();
        let overflow_err = codegen
            .emit_cast(huge_val, &Type::Int64, &Type::Int32, None)
            .unwrap_err();
        assert_eq!(
            overflow_err,
            "value `5000000000` exceeds 32 bits for target type `int32`"
        );

        codegen.builder.build_return(None).unwrap();
        assert!(codegen.module.verify().is_ok());

        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("trunc i64 %v_i64 to i32"));
        assert!(ir.contains("sext i8 %v_i8 to i32"));
        assert!(ir.contains("fptosi double %v_f64 to i32"));
        assert!(ir.contains("sitofp i32 %cast_trunc to double"));
        assert!(ir.contains("ptrtoint ptr %p_cell to i64"));
        assert!(ir.contains("uitofp i64 %ptr_addr to double"));
        assert!(ir.contains("fptoui double %cast_ptr_to_f64 to i64"));
        assert!(ir.contains("inttoptr i64 %f64_to_addr to ptr"));
    }

    #[test]
    fn test_intrinsic_cast_strings_and_bools() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_cast_str", TargetTriple::host());

        let void_ty = context.void_type().fn_type(&[], false);
        let test_fn = codegen.module.add_function("test_str_casts", void_ty, None);
        let entry = context.append_basic_block(test_fn, "entry");
        codegen.builder.position_at_end(entry);

        let local_reg = codegen.emit_region_prologue(4096, "cast_reg").expect("reg");

        // 1. int64 -> string & string -> int64
        let num_42 = context.i64_type().const_int(42, false).into();
        let str_res = codegen
            .emit_cast(num_42, &Type::Int64, &Type::String, Some(local_reg))
            .expect("to_s");
        assert!(str_res.is_struct_value());

        let back_int = codegen
            .emit_cast(str_res, &Type::String, &Type::Int64, Some(local_reg))
            .expect("from_s");
        assert!(back_int.is_int_value());

        // 2. bytes <-> string
        let dummy_bytes = codegen.slice_type().const_zero().into();
        let b_to_s = codegen
            .emit_cast(dummy_bytes, &Type::Bytes, &Type::String, None)
            .expect("b_to_s");
        assert!(b_to_s.is_struct_value());

        let s_to_b = codegen
            .emit_cast(b_to_s, &Type::String, &Type::Bytes, None)
            .expect("s_to_b");
        assert!(s_to_b.is_struct_value());

        // 3. bool <-> int
        let p_bool = codegen
            .builder
            .build_alloca(context.bool_type(), "p_bool")
            .unwrap();
        let bool_val = codegen
            .builder
            .build_load(context.bool_type(), p_bool, "v_bool")
            .unwrap();
        let b_to_i = codegen
            .emit_cast(bool_val, &Type::Bool, &Type::Int32, None)
            .expect("b_to_i");
        assert_eq!(b_to_i.into_int_value().get_type().get_bit_width(), 32);

        let i_to_b = codegen
            .emit_cast(b_to_i, &Type::Int32, &Type::Bool, None)
            .expect("i_to_b");
        assert_eq!(i_to_b.into_int_value().get_type().get_bit_width(), 1);

        // 4. bool <-> string
        let bool_to_str = codegen
            .emit_cast(bool_val, &Type::Bool, &Type::String, Some(local_reg))
            .expect("b_to_s");
        assert!(bool_to_str.is_struct_value());

        let str_to_bool = codegen
            .emit_cast(bool_to_str, &Type::String, &Type::Bool, None)
            .expect("s_to_b_parsed");
        assert_eq!(str_to_bool.into_int_value().get_type().get_bit_width(), 1);

        // 5. string <-> char[]
        let str_to_chars = codegen
            .emit_cast(
                bool_to_str,
                &Type::String,
                &Type::Array(Box::new(Type::Char)),
                Some(local_reg),
            )
            .expect("s_to_chars");
        assert!(str_to_chars.is_struct_value());

        let chars_to_str = codegen
            .emit_cast(
                str_to_chars,
                &Type::Array(Box::new(Type::Char)),
                &Type::String,
                Some(local_reg),
            )
            .expect("chars_to_s");
        assert!(chars_to_str.is_struct_value());

        codegen.emit_region_epilogue(local_reg).expect("epilogue");
        codegen.builder.build_return(None).unwrap();

        assert!(codegen.module.verify().is_ok());
        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("call void @yukino_rt_int64_to_string"));
        assert!(ir.contains("call i64 @yukino_rt_string_to_int64"));
        assert!(ir.contains("zext i1 %v_bool to i32"));
        assert!(ir.contains("icmp ne i32 %bool_to_int, 0"));
        assert!(ir.contains("call void @yukino_rt_bool_to_string"));
        assert!(ir.contains("call i1 @yukino_rt_string_to_bool"));
        assert!(ir.contains("call { ptr, i64, i64 } @yukino_rt_string_to_chars"));
        assert!(ir.contains("call void @yukino_rt_chars_to_string"));
    }

    #[test]
    fn test_intrinsic_args_and_raw_args() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_args_mod", TargetTriple::host());

        let i32_type = context.i32_type();
        let ptr_type = codegen.ptr_type();
        let main_fn_type = i32_type.fn_type(&[i32_type.into(), ptr_type.into()], false);

        let main_fn = codegen.module.add_function("main", main_fn_type, None);
        let entry = context.append_basic_block(main_fn, "entry");
        codegen.builder.position_at_end(entry);

        let argc = main_fn.get_first_param().unwrap().into_int_value();
        let argv = main_fn.get_params()[1].into_pointer_value();
        codegen
            .intrinsic
            .setup_main_args(&codegen.module, &codegen.builder, argc, argv)
            .expect("setup");

        let app_reg = codegen.emit_region_prologue(8192, "app_reg").expect("reg");

        let args_val = codegen.emit_args(app_reg).expect("args");
        assert!(args_val.is_struct_value());

        let raw_val = codegen.emit_raw_args(app_reg).expect("raw_args");
        assert!(raw_val.is_struct_value());

        codegen.emit_region_epilogue(app_reg).expect("epilogue");
        codegen
            .builder
            .build_return(Some(&i32_type.const_int(0, false)))
            .unwrap();

        assert!(codegen.module.verify().is_ok());
        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("@_Y_argc = internal global i32 0"));
        assert!(ir.contains("@_Y_argv = internal global ptr null"));
        assert!(ir.contains("store i32 %0, ptr @_Y_argc"));
        assert!(ir.contains("store ptr %1, ptr @_Y_argv"));
        assert!(ir.contains("call void @yukino_rt_get_args"));
        assert!(ir.contains("call void @yukino_rt_get_raw_args"));
    }

    #[test]
    fn test_debug_info_dwarf_and_codeview_init() {
        let context = Context::create();

        // 1. DWARF for Linux host
        let mut codegen_linux =
            CodeGenerator::new(&context, "test_dwarf", TargetTriple::X86_64UnknownLinuxGnu);
        codegen_linux.enable_debug_info("main.ykn", "/app", false);
        assert_eq!(codegen_linux.debug.format(), DebugFormat::Dwarf);
        assert!(codegen_linux.debug.is_enabled());
        let ir_linux = codegen_linux.emit_llvm_ir_to_string();
        assert!(ir_linux.contains("!\"Debug Info Version\", i32 3"));
        assert!(ir_linux.contains("!\"Dwarf Version\", i32 4"));
        assert!(ir_linux.contains("!DICompileUnit("));

        // 2. CodeView for Windows target
        let mut codegen_win =
            CodeGenerator::new(&context, "test_codeview", TargetTriple::X86_64PcWindowsGnu);
        codegen_win.debug.set_format(DebugFormat::CodeView);
        codegen_win.enable_debug_info("main.ykn", "C:\\app", false);
        assert_eq!(codegen_win.debug.format(), DebugFormat::CodeView);
        assert!(codegen_win.debug.is_enabled());
        let ir_win = codegen_win.emit_llvm_ir_to_string();
        assert!(ir_win.contains("!\"Debug Info Version\", i32 3"));
        assert!(ir_win.contains("!\"CodeView\", i32 1"));
        assert!(ir_win.contains("!DICompileUnit("));
    }

    #[test]
    fn test_debug_info_function_and_locations() {
        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, "test_dbg_fn", TargetTriple::host());
        codegen.enable_debug_info("calc.ykn", "/src", false);

        let i32_type = context.i32_type();
        let fn_type = i32_type.fn_type(&[i32_type.into()], false);
        let test_fn = codegen.module.add_function("compute", fn_type, None);

        let file = codegen
            .debug
            .get_or_create_file(std::path::Path::new("/src/calc.ykn"))
            .unwrap();
        let i32_di = codegen
            .debug
            .get_or_create_type(&Type::Int32, file)
            .unwrap();
        let sub_ty = codegen
            .debug
            .create_subroutine_type(file, Some(i32_di), &[i32_di])
            .unwrap();

        codegen
            .debug
            .emit_function(test_fn, "compute", None, file, 10, sub_ty, true, false);

        let entry = context.append_basic_block(test_fn, "entry");
        codegen.builder.position_at_end(entry);

        codegen.debug.set_location(&codegen.builder, 11, 4);
        let cell = codegen.builder.build_alloca(i32_type, "cell").unwrap();
        codegen
            .builder
            .build_store(cell, i32_type.const_int(42, false))
            .unwrap();

        // Push nested lexical block
        codegen.debug.push_lexical_block(file, 12, 8);
        codegen.debug.set_location(&codegen.builder, 13, 12);
        let loaded = codegen.builder.build_load(i32_type, cell, "val").unwrap();
        codegen.debug.pop_scope();

        codegen.debug.set_location(&codegen.builder, 14, 4);
        codegen.builder.build_return(Some(&loaded)).unwrap();

        assert!(codegen.module.verify().is_ok());
        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("!DISubprogram(name: \"compute\""));
        assert!(ir.contains("!DILocation(line: 11, column: 4"));
        assert!(ir.contains("!DILexicalBlock("));
        assert!(ir.contains("!DILocation(line: 13, column: 12"));
    }

    #[test]
    fn test_debug_info_types_and_variables() {
        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, "test_dbg_types", TargetTriple::host());
        codegen.enable_debug_info("model.ykn", "/src", false);

        let file = codegen
            .debug
            .get_or_create_file(std::path::Path::new("/src/model.ykn"))
            .unwrap();

        // 1. Basic types
        let bool_di = codegen.debug.get_or_create_type(&Type::Bool, file).unwrap();
        let int64_di = codegen
            .debug
            .get_or_create_type(&Type::Int64, file)
            .unwrap();
        let f64_di = codegen
            .debug
            .get_or_create_type(&Type::Float64, file)
            .unwrap();
        assert!(!bool_di.as_mut_ptr().is_null());
        assert!(!int64_di.as_mut_ptr().is_null());
        assert!(!f64_di.as_mut_ptr().is_null());

        // 2. Pointer & String types
        let ptr_di = codegen
            .debug
            .get_or_create_type(&Type::Ptr(Box::new(Type::Int32)), file)
            .unwrap();
        let str_di = codegen
            .debug
            .get_or_create_type(&Type::String, file)
            .unwrap();
        assert!(!ptr_di.as_mut_ptr().is_null());
        assert!(!str_di.as_mut_ptr().is_null());

        // 3. Composite struct type
        let scope = codegen.debug.current_scope().unwrap();
        let m0 = codegen
            .debug
            .create_member_type(scope, "id", file, 5, 32, 32, 0, bool_di)
            .unwrap();
        let m1 = codegen
            .debug
            .create_member_type(scope, "val", file, 6, 64, 64, 64, int64_di)
            .unwrap();
        let struct_di = codegen
            .debug
            .create_struct_type(
                scope,
                "Record",
                file,
                4,
                128,
                64,
                &[m0.as_type(), m1.as_type()],
            )
            .unwrap();

        // 4. Function with auto variable
        let void_ty = context.void_type().fn_type(&[], false);
        let test_fn = codegen.module.add_function("setup_record", void_ty, None);
        let sub_ty = codegen
            .debug
            .create_subroutine_type(file, None, &[])
            .unwrap();
        codegen
            .debug
            .emit_function(test_fn, "setup_record", None, file, 10, sub_ty, true, false);

        let entry = context.append_basic_block(test_fn, "entry");
        codegen.builder.position_at_end(entry);

        let _rec_alloca = codegen
            .builder
            .build_alloca(context.i64_type(), "rec_slot")
            .unwrap();
        let _var = codegen
            .debug
            .create_auto_variable("my_rec", file, 11, struct_di.as_type())
            .unwrap();

        codegen.builder.build_return(None).unwrap();

        assert!(codegen.module.verify().is_ok());
        let ir = codegen.emit_llvm_ir_to_string();
        assert!(ir.contains("!DIBasicType(name: \"bool\""));
        assert!(ir.contains("!DIBasicType(name: \"int64\""));
        assert!(ir.contains("!DICompositeType(tag: DW_TAG_structure_type, name: \"Record\""));
        assert!(ir.contains("!DIDerivedType(tag: DW_TAG_member, name: \"id\""));
        assert!(ir.contains("!DIDerivedType(tag: DW_TAG_member, name: \"val\""));
        assert!(ir.contains("!DILocalVariable(name: \"my_rec\""));
    }

    #[test]
    fn test_compile_cgu_definitions_and_foreign_declarations() {
        let mut ast = AstModule::default();
        ast.items.push(make_test_fun(1, 5));
        ast.items.push(make_test_fun(2, 5));

        let units = partition_module(&ast, 2);
        assert_eq!(units.len(), 2);

        let context0 = Context::create();
        let mut codegen0 =
            CodeGenerator::new(&context0, "cgu_0", TargetTriple::X86_64UnknownLinuxGnu);
        codegen0
            .compile_cgu(&ast, &units[0])
            .expect("compile cgu 0");
        let ir0 = codegen0.emit_llvm_ir_to_string();

        let context1 = Context::create();
        let mut codegen1 =
            CodeGenerator::new(&context1, "cgu_1", TargetTriple::X86_64UnknownLinuxGnu);
        codegen1
            .compile_cgu(&ast, &units[1])
            .expect("compile cgu 1");
        let ir1 = codegen1.emit_llvm_ir_to_string();

        assert!(
            ir0.contains("define fastcc void @__yukifun_fn_1___void()")
                || ir0.contains("define fastcc void @__yukifun_fn_2___void()")
        );
        assert!(
            ir0.contains("declare fastcc void @__yukifun_fn_1___void()")
                || ir0.contains("declare fastcc void @__yukifun_fn_2___void()")
        );
        assert!(
            ir1.contains("define fastcc void @__yukifun_fn_1___void()")
                || ir1.contains("define fastcc void @__yukifun_fn_2___void()")
        );
        assert!(
            ir1.contains("declare fastcc void @__yukifun_fn_1___void()")
                || ir1.contains("declare fastcc void @__yukifun_fn_2___void()")
        );
    }

    #[test]
    fn test_compile_modules_parallel_execution() {
        let mut ast = AstModule::default();
        ast.items.push(make_test_fun(1, 8));
        ast.items.push(make_test_fun(2, 6));
        ast.items.push(make_test_fun(3, 4));
        ast.items.push(make_test_fun(4, 2));

        let compiled = compile_modules_parallel(&ast, &[], TargetTriple::X86_64UnknownLinuxGnu, 2, "unwind")
            .expect("compile parallel");
        assert_eq!(compiled.len(), 2);
        for cgu in &compiled {
            assert!(
                cgu.ll_ir
                    .contains("target triple = \"x86_64-unknown-linux-gnu\"")
            );
            assert!(cgu.item_count > 0);
        }
    }

    #[test]
    fn test_uwtable_attr() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_uwtable", TargetTriple::host());
        let fn_ty = context.void_type().fn_type(&[], false);
        let fn_val = codegen.module.add_function("foo", fn_ty, None);
        let kind_id = inkwell::attributes::Attribute::get_named_enum_kind_id("uwtable");
        if kind_id != 0 {
            let attr = context.create_enum_attribute(kind_id, 2);
            fn_val.add_attribute(inkwell::attributes::AttributeLoc::Function, attr);
        }
        let ir = codegen.module.print_to_string().to_string();
        assert!(ir.contains("uwtable"));
    }

    #[test]
    fn test_thunk_attribute_codegen() {
        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, "test_thunk", TargetTriple::host());
        let dspan = yukic_span::Span::new(yukic_span::FileId(0), yukic_span::BytePos(0), yukic_span::BytePos(0));
        let thunk_attr = yukic_ast::AttributeNode::new(
            yukic_ast::Ident::new("thunk", dspan),
            yukic_ast::AttributeArgs::None,
            dspan,
        );
        let fun_decl = yukic_ast::FunDecl {
            name: yukic_ast::Ident::new("thread_worker", dspan),
            attrs: vec![thunk_attr],
            generics: vec![],
            params: vec![],
            return_type: None,
            body: yukic_ast::Block { stmts: vec![], span: dspan },
            span: dspan,
        };
        codegen.compile_fun_decl(&fun_decl).unwrap();
        let sym_name = codegen.symbol_for_fun(&fun_decl);
        let fn_val = codegen.module.get_function(&sym_name).expect("function not found");
        assert_eq!(
            fn_val.get_call_conventions(),
            inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32
        );
    }

    #[test]
    fn test_compiler_ident_version_metadata() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_ident", TargetTriple::host());
        let ir = codegen.module.print_to_string().to_string();
        let expected = format!("yukic v{}", env!("CARGO_PKG_VERSION"));
        assert!(ir.contains("!llvm.ident"));
        assert!(ir.contains(&expected));
    }
}

