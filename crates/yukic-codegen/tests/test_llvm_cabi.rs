#![deny(unsafe_code)]

use inkwell::context::Context;
use inkwell::values::{FunctionValue, PointerValue};
use std::fs;
use std::path::Path;
use yukic_ast::Ident;
use yukic_ast::attr::CAttributeNode;
use yukic_codegen::CodeGenerator;
use yukic_span::{BytePos, FileId, Span};
use yukic_target::TargetTriple;
use yukic_types::Type;

fn c_attr(name: &str) -> CAttributeNode {
    let dummy_span = Span::new(FileId(0), BytePos(0), BytePos(0));
    CAttributeNode::new(Ident::new(name, dummy_span), None, dummy_span)
}

fn build_cstruct_types_and_ops<'ctx>(codegen: &mut CodeGenerator<'ctx>, context: &'ctx Context) {
    codegen.registry.add_cstruct(
        "HardwareInfo",
        vec![
            ("device_id".to_string(), Type::Int32),
            ("memory_bytes".to_string(), Type::Int64),
        ],
        false,
        None,
    );
    codegen.registry.add_cstruct(
        "PackedRgb",
        vec![
            ("r".to_string(), Type::Uint8),
            ("g".to_string(), Type::Uint8),
            ("b".to_string(), Type::Uint8),
        ],
        true,
        None,
    );

    let hw_ty = codegen.lower_named_cstruct("HardwareInfo").expect("hw");
    let rgb_ty = codegen.lower_named_cstruct("PackedRgb").expect("rgb");

    let hw_alloca = codegen.emit_cstruct_alloca(hw_ty, "hw").expect("alloca");
    let _rgb_alloca = codegen.emit_cstruct_alloca(rgb_ty, "rgb").expect("alloca");

    let id_val = context.i32_type().const_int(101, false).into();
    let mem_val = context
        .i64_type()
        .const_int(1024 * 1024 * 1024, false)
        .into();

    codegen
        .emit_cstruct_store_field(hw_ty, hw_alloca, 0, id_val)
        .expect("store id");
    codegen
        .emit_cstruct_store_field(hw_ty, hw_alloca, 1, mem_val)
        .expect("store mem");

    let _loaded_id = codegen
        .emit_cstruct_load_field(hw_ty, hw_alloca, 0, context.i32_type().into(), "dev_id")
        .expect("load id");
}

fn build_raw_pointer_ops<'ctx>(
    codegen: &mut CodeGenerator<'ctx>,
    context: &'ctx Context,
    sample_cfun: FunctionValue<'ctx>,
) {
    let i64_type = context.i64_type();
    let cell_ptr = codegen.builder.build_alloca(i64_type, "raw_cell").unwrap();

    let val = i64_type.const_int(777, false).into();
    codegen.emit_ptr_deref_set(cell_ptr, val).expect("set");
    let _val_read = codegen
        .emit_ptr_deref(i64_type.into(), cell_ptr, "v_read")
        .expect("read");

    let byte_offset = i64_type.const_int(16, false);
    let _off_ptr = codegen
        .emit_ptr_offset(cell_ptr, byte_offset, "off_p")
        .expect("offset");

    let _is_null = codegen
        .emit_ptr_is_null(cell_ptr, "is_p_null")
        .expect("null_check");

    let addr = codegen.emit_ptr_to_int(cell_ptr, "p_addr").expect("to_int");
    let _restored = codegen.emit_int_to_ptr(addr, "p_restored").expect("to_ptr");

    let _cb_ptr = codegen.get_cfun_pointer(sample_cfun);
}

fn build_cstr_ops<'ctx>(
    codegen: &mut CodeGenerator<'ctx>,
    context: &'ctx Context,
    worker_fn: FunctionValue<'ctx>,
    local_reg: PointerValue<'ctx>,
) {
    let msg_str = codegen.rodata.emit_string(&codegen.module, "interop_test");
    let c_str = codegen
        .emit_to_cstr(worker_fn, local_reg, msg_str, "c_msg")
        .expect("to_cstr");

    let fixed_len = context.i64_type().const_int(12, false);
    let _from_n = codegen
        .emit_from_cstr_n(c_str, fixed_len, "str_slice")
        .expect("from_cstr_n");

    let _from_dyn = codegen.emit_from_cstr(c_str, "str_dyn").expect("from_cstr");
}

fn build_cfun_declarations_and_exports<'ctx>(
    codegen: &mut CodeGenerator<'ctx>,
    context: &'ctx Context,
) -> FunctionValue<'ctx> {
    let i32_type = context.i32_type();
    let i64_type = context.i64_type();
    let ptr_type = codegen.ptr_type();
    let void_type = context.void_type();

    // 1. External cdecl function: puts(ptr) -> i32
    let puts_ty = i32_type.fn_type(&[ptr_type.into()], false);
    let puts_fn = codegen.module.add_function("puts", puts_ty, None);
    codegen
        .cabi
        .configure_cfun(puts_fn, &[c_attr("cdecl")], false, false)
        .unwrap();

    // 2. External stdcall function: MessageBoxA(ptr, ptr, ptr, i32) -> i32
    let msgbox_ty = i32_type.fn_type(
        &[
            ptr_type.into(),
            ptr_type.into(),
            ptr_type.into(),
            i32_type.into(),
        ],
        false,
    );
    let msgbox_fn = codegen.module.add_function("MessageBoxA", msgbox_ty, None);
    codegen
        .cabi
        .configure_cfun(msgbox_fn, &[c_attr("stdcall")], false, false)
        .unwrap();

    // 3. External fastcall function: fast_compute(i64, i64) -> i64
    let fast_ty = i64_type.fn_type(&[i64_type.into(), i64_type.into()], false);
    let fast_fn = codegen.module.add_function("fast_compute", fast_ty, None);
    codegen
        .cabi
        .configure_cfun(fast_fn, &[c_attr("fastcall")], false, false)
        .unwrap();

    // 4. External cold noreturn function: c_abort() -> void
    let abort_ty = void_type.fn_type(&[], false);
    let abort_fn = codegen.module.add_function("c_abort", abort_ty, None);
    codegen
        .cabi
        .configure_cfun(
            abort_fn,
            &[c_attr("cdecl"), c_attr("noreturn"), c_attr("cold")],
            false,
            false,
        )
        .unwrap();

    // 5. Public exported cfun: yuki_plugin_entry(ptr) -> ptr
    let entry_ty = ptr_type.fn_type(&[ptr_type.into()], false);
    let entry_fn = codegen
        .module
        .add_function("yuki_plugin_entry", entry_ty, None);
    codegen
        .cabi
        .configure_cfun(
            entry_fn,
            &[c_attr("inline_always"), c_attr("returns_nonnull")],
            true,
            true,
        )
        .unwrap();

    let entry_bb = context.append_basic_block(entry_fn, "entry");
    codegen.builder.position_at_end(entry_bb);
    let p_arg = entry_fn.get_first_param().unwrap();
    codegen.builder.build_return(Some(&p_arg)).unwrap();

    entry_fn
}

fn build_cabi_demo<'ctx>(codegen: &mut CodeGenerator<'ctx>, context: &'ctx Context) {
    let sample_cfun = build_cfun_declarations_and_exports(codegen, context);

    let i32_type = context.i32_type();
    let main_ty = i32_type.fn_type(&[], false);
    let main_fn = codegen
        .module
        .add_function("main_cabi_worker", main_ty, None);
    let main_bb = context.append_basic_block(main_fn, "entry");
    codegen.builder.position_at_end(main_bb);

    let local_reg = codegen
        .emit_region_prologue(8192, "interop_region")
        .expect("reg");

    build_cstruct_types_and_ops(codegen, context);
    build_raw_pointer_ops(codegen, context, sample_cfun);
    build_cstr_ops(codegen, context, main_fn, local_reg);

    codegen.emit_region_epilogue(local_reg).expect("epilogue");
    let zero = i32_type.const_int(0, false);
    codegen.builder.build_return(Some(&zero)).unwrap();
}

fn verify_emitted_ir(ir: &str) {
    // 1. Calling conventions and attributes
    assert!(ir.contains("@puts(ptr)"));
    assert!(ir.contains("x86_stdcallcc i32 @MessageBoxA(ptr, ptr, ptr, i32)"));
    assert!(ir.contains("@c_abort()"));
    assert!(ir.contains("define nonnull ptr @yuki_plugin_entry(ptr %0)"));
    assert!(ir.contains("{ cold noreturn }"));
    assert!(ir.contains("{ alwaysinline }"));

    // 2. Struct layouts (natural vs packed)
    assert!(ir.contains("%cstruct.HardwareInfo = type { i32, i64 }"));
    assert!(ir.contains("%cstruct.PackedRgb = type <{ i8, i8, i8 }>"));

    // 3. Raw pointer operations
    assert!(ir.contains("store i64 777, ptr %raw_cell"));
    assert!(ir.contains("%v_read = load i64, ptr %raw_cell"));
    assert!(ir.contains("%off_p_int = ptrtoint ptr %raw_cell to i64"));
    assert!(ir.contains("%off_p_add = add i64 %off_p_int, 16"));
    assert!(ir.contains("%off_p = inttoptr i64 %off_p_add to ptr"));
    assert!(ir.contains("%is_p_null = icmp eq ptr %raw_cell, null"));

    // 4. C-string conversions and region alloc
    assert!(ir.contains("call void @llvm.memcpy.p0.p0.i64"));
    assert!(ir.contains("store i8 0, ptr %null_ptr"));
    assert!(ir.contains("declare i64 @strlen(ptr)"));
    assert!(ir.contains("%c_len = call i64 @strlen(ptr %alloc_ptr)"));
}

#[test]
fn test_emit_llvm_cabi_ir_file() {
    let context = Context::create();
    let mut codegen = CodeGenerator::new(
        &context,
        "yukino_cabi_module",
        TargetTriple::X86_64UnknownLinuxGnu,
    );

    build_cabi_demo(&mut codegen, &context);

    codegen.module.verify().expect("verify module");

    let ir_content = codegen.emit_llvm_ir_to_string();
    verify_emitted_ir(&ir_content);
}
