#![deny(unsafe_code)]

use inkwell::context::Context;
use std::fs;
use std::path::Path;
use yukic_codegen::{CodeGenerator, DebugFormat};
use yukic_target::TargetTriple;
use yukic_types::Type;

fn build_debug_demo<'ctx>(codegen: &mut CodeGenerator<'ctx>, context: &'ctx Context) {
    let compile_dir = ".";
    let file = codegen
        .enable_debug_info("main.yu", compile_dir, false)
        .expect("enable debug info");

    let int32_di = codegen
        .debug
        .get_or_create_type(&Type::Int32, file)
        .unwrap();
    let int64_di = codegen
        .debug
        .get_or_create_type(&Type::Int64, file)
        .unwrap();
    let bool_di = codegen.debug.get_or_create_type(&Type::Bool, file).unwrap();
    let ptr_di = codegen
        .debug
        .create_pointer_type("ptr_i32", int32_di)
        .unwrap();
    let arr_di = codegen
        .debug
        .create_array_type(int32_di, 128, 32, 4)
        .unwrap();

    let root_scope = codegen.debug.current_scope().unwrap();
    let m0 = codegen
        .debug
        .create_member_type(root_scope, "x", file, 5, 32, 32, 0, int32_di)
        .unwrap();
    let m1 = codegen
        .debug
        .create_member_type(root_scope, "y", file, 6, 32, 32, 32, int32_di)
        .unwrap();
    let m2 = codegen
        .debug
        .create_member_type(root_scope, "id", file, 7, 64, 64, 64, int64_di)
        .unwrap();
    let m3 = codegen
        .debug
        .create_member_type(root_scope, "valid", file, 8, 8, 8, 128, bool_di)
        .unwrap();
    let m4 = codegen
        .debug
        .create_member_type(
            root_scope,
            "raw_ptr",
            file,
            9,
            64,
            64,
            192,
            ptr_di.as_type(),
        )
        .unwrap();
    let m5 = codegen
        .debug
        .create_member_type(
            root_scope,
            "history",
            file,
            10,
            128,
            32,
            256,
            arr_di.as_type(),
        )
        .unwrap();
    let point_di = codegen
        .debug
        .create_struct_type(
            root_scope,
            "Point",
            file,
            4,
            384,
            64,
            &[
                m0.as_type(),
                m1.as_type(),
                m2.as_type(),
                m3.as_type(),
                m4.as_type(),
                m5.as_type(),
            ],
        )
        .unwrap();

    let i32_llvm = context.i32_type();
    let fn_sig = i32_llvm.fn_type(&[i32_llvm.into(), i32_llvm.into()], false);
    let compute_fn = codegen.module.add_function("compute", fn_sig, None);

    let sub_ty = codegen
        .debug
        .create_subroutine_type(file, Some(int32_di), &[int32_di, int32_di])
        .unwrap();
    codegen
        .debug
        .emit_function(compute_fn, "compute", None, file, 10, sub_ty, true, false);

    let entry = context.append_basic_block(compute_fn, "entry");
    codegen.builder.position_at_end(entry);

    let _param_x = codegen
        .debug
        .create_parameter_variable("x", 1, file, 10, int32_di)
        .unwrap();
    let _param_y = codegen
        .debug
        .create_parameter_variable("y", 2, file, 10, int32_di)
        .unwrap();

    codegen.set_debug_location_coords(11, 5);
    let arg0 = compute_fn.get_nth_param(0).unwrap().into_int_value();
    let arg1 = compute_fn.get_nth_param(1).unwrap().into_int_value();
    let sum = codegen.builder.build_int_add(arg0, arg1, "sum").unwrap();

    let _block_scope = codegen.debug.push_lexical_block(file, 12, 9).unwrap();
    let _diff_var = codegen
        .debug
        .create_auto_variable("diff", file, 13, int32_di)
        .unwrap();
    codegen.set_debug_location_coords(13, 9);
    let diff = codegen.builder.build_int_sub(arg0, arg1, "diff").unwrap();
    let total = codegen.builder.build_int_add(sum, diff, "total").unwrap();
    codegen.debug.pop_scope();

    codegen.set_debug_location_coords(15, 5);
    codegen.builder.build_return(Some(&total)).unwrap();

    let main_sig = i32_llvm.fn_type(&[], false);
    let main_fn = codegen.module.add_function("main", main_sig, None);
    let main_sub_ty = codegen
        .debug
        .create_subroutine_type(file, Some(int32_di), &[])
        .unwrap();
    codegen
        .debug
        .emit_function(main_fn, "main", None, file, 20, main_sub_ty, true, false);

    let main_bb = context.append_basic_block(main_fn, "entry");
    codegen.builder.position_at_end(main_bb);

    let _pt_var = codegen
        .debug
        .create_auto_variable("pt", file, 21, point_di.as_type())
        .unwrap();

    codegen.set_debug_location_coords(21, 5);
    let c10 = i32_llvm.const_int(10, false);
    let c20 = i32_llvm.const_int(20, false);
    let call_res = codegen
        .builder
        .build_call(compute_fn, &[c10.into(), c20.into()], "call_compute")
        .unwrap()
        .try_as_basic_value()
        .unwrap_basic();

    codegen.set_debug_location_coords(22, 5);
    codegen.builder.build_return(Some(&call_res)).unwrap();

    codegen.clear_debug_location();
}

fn verify_dwarf_ir(ir: &str) {
    assert!(ir.contains("!llvm.dbg.cu"));
    assert!(ir.contains("!{i32 2, !\"Debug Info Version\", i32 3}"));
    assert!(ir.contains("!{i32 2, !\"Dwarf Version\", i32 4}"));
    assert!(ir.contains("!DISubprogram(name: \"compute\""));
    assert!(ir.contains("!DISubprogram(name: \"main\""));
    assert!(ir.contains("!DILexicalBlock("));
    assert!(ir.contains("!DILocation(line: 11, column: 5"));
    assert!(ir.contains("!DILocation(line: 13, column: 9"));
    assert!(ir.contains("!DILocation(line: 15, column: 5"));
    assert!(ir.contains("!DILocation(line: 21, column: 5"));
    assert!(ir.contains("!DIBasicType(name: \"int32\""));
    assert!(ir.contains("!DIBasicType(name: \"int64\""));
    assert!(ir.contains("!DIBasicType(name: \"bool\""));
    assert!(ir.contains("!DIDerivedType(tag: DW_TAG_pointer_type"));
    assert!(ir.contains("!DICompositeType(tag: DW_TAG_array_type"));
    assert!(ir.contains("!DICompositeType(tag: DW_TAG_structure_type, name: \"Point\""));
    assert!(ir.contains("!DIDerivedType(tag: DW_TAG_member, name: \"x\""));
    assert!(ir.contains("!DIDerivedType(tag: DW_TAG_member, name: \"y\""));
    assert!(ir.contains("!DIDerivedType(tag: DW_TAG_member, name: \"id\""));
    assert!(ir.contains("!DIDerivedType(tag: DW_TAG_member, name: \"valid\""));
    assert!(ir.contains("!DIDerivedType(tag: DW_TAG_member, name: \"raw_ptr\""));
    assert!(ir.contains("!DIDerivedType(tag: DW_TAG_member, name: \"history\""));
    assert!(ir.contains("!DILocalVariable(name: \"x\", arg: 1"));
    assert!(ir.contains("!DILocalVariable(name: \"y\", arg: 2"));
    assert!(ir.contains("!DILocalVariable(name: \"diff\""));
    assert!(ir.contains("!DILocalVariable(name: \"pt\""));
}

#[test]
fn test_debug_info_emission_and_verification() {
    let context = Context::create();
    let mut codegen = CodeGenerator::new(
        &context,
        "yukino_debug_module",
        TargetTriple::X86_64UnknownLinuxGnu,
    );

    build_debug_demo(&mut codegen, &context);

    codegen.module.verify().expect("verify module");

    let ir_content = codegen.emit_llvm_ir_to_string();
    verify_dwarf_ir(&ir_content);
}

#[test]
fn test_codeview_debug_format_flags() {
    let context = Context::create();
    let mut codegen = CodeGenerator::new(
        &context,
        "yukino_codeview_module",
        TargetTriple::X86_64PcWindowsGnu,
    );

    assert_eq!(codegen.debug.format(), DebugFormat::CodeView);
    codegen
        .enable_debug_info("win.yu", "C:\\project", false)
        .expect("enable win debug");

    let ir = codegen.emit_llvm_ir_to_string();
    assert!(ir.contains("!{i32 2, !\"Debug Info Version\", i32 3}"));
    assert!(ir.contains("!{i32 2, !\"CodeView\", i32 1}"));
    assert!(!ir.contains("Dwarf Version"));
}

#[test]
fn test_emit_llvm_debug_ir_file() {
    let context = Context::create();
    let mut codegen = CodeGenerator::new(
        &context,
        "yukino_debug_module",
        TargetTriple::X86_64UnknownLinuxGnu,
    );

    build_debug_demo(&mut codegen, &context);

    codegen.module.verify().expect("verify module");

    let ir_content = codegen.emit_llvm_ir_to_string();
    verify_dwarf_ir(&ir_content);
}
