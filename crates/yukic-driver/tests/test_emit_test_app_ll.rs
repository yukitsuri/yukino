#![deny(unsafe_code)]

use std::fs;
use std::path::Path;
use yukic_codegen::CodeGenerator;
use yukic_codegen::inkwell;
use yukic_codegen::inkwell::IntPredicate;
use yukic_codegen::inkwell::context::Context;
use yukic_opt::{OptLevel, Optimizer};
use yukic_target::TargetTriple;
use yukic_types::Type;

#[test]
fn test_generate_test_app_llvm_ir() {
    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, "test-app", TargetTriple::host());

    // 1. Register struct MathError { message: string }
    codegen
        .registry
        .add_struct("MathError", vec![("message".to_string(), Type::String)]);
    let math_error_ty = codegen
        .llvm_type(&Type::Struct("MathError".into()))
        .expect("MathError type");
    assert!(math_error_ty.is_struct_type());

    let i32_type = context.i32_type();
    let i64_type = context.i64_type();
    let ptr_type = codegen.ptr_type();
    let str_ty = codegen.string_type();

    // 2. Setup typeinfo for MathError
    let math_typeinfo = codegen.get_or_create_typeinfo("MathError");

    // 3. String constant for "division by zero"
    let div_zero_str = codegen.emit_string_literal("division by zero");

    // 4. Function: fastcc i32 @calc(i32 %a, i32 %op, i32 %b)
    let calc_fn_ty = i32_type.fn_type(&[i32_type.into(), i32_type.into(), i32_type.into()], false);
    let calc_fn = codegen.module.add_function("calc", calc_fn_ty, None);
    calc_fn.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32);
    let personality = codegen.setup_personality_function(calc_fn);

    let calc_entry = context.append_basic_block(calc_fn, "entry");
    codegen.builder.position_at_end(calc_entry);

    let local_reg = codegen
        .emit_region_prologue(4096, "_local_region")
        .expect("local region");

    let a_val = calc_fn.get_first_param().unwrap().into_int_value();
    let op_val = calc_fn.get_params()[1].into_int_value();
    let b_val = calc_fn.get_params()[2].into_int_value();

    // switch op
    let op_mul = context.append_basic_block(calc_fn, "op_mul");
    let op_add = context.append_basic_block(calc_fn, "op_add");
    let op_sub = context.append_basic_block(calc_fn, "op_sub");
    let op_div = context.append_basic_block(calc_fn, "op_div");
    let op_else = context.append_basic_block(calc_fn, "op_else");
    let calc_exit = context.append_basic_block(calc_fn, "calc_exit");

    codegen
        .builder
        .build_switch(
            op_val,
            op_else,
            &[
                (i32_type.const_int(42, false), op_mul), // '*'
                (i32_type.const_int(43, false), op_add), // '+'
                (i32_type.const_int(45, false), op_sub), // '-'
                (i32_type.const_int(47, false), op_div), // '/'
            ],
        )
        .unwrap();

    // op_mul: a * b
    codegen.builder.position_at_end(op_mul);
    let res_mul = codegen
        .builder
        .build_int_mul(a_val, b_val, "res_mul")
        .unwrap();
    codegen
        .builder
        .build_unconditional_branch(calc_exit)
        .unwrap();

    // op_add: a + b
    codegen.builder.position_at_end(op_add);
    let res_add = codegen
        .builder
        .build_int_add(a_val, b_val, "res_add")
        .unwrap();
    codegen
        .builder
        .build_unconditional_branch(calc_exit)
        .unwrap();

    // op_sub: a - b
    codegen.builder.position_at_end(op_sub);
    let res_sub = codegen
        .builder
        .build_int_sub(a_val, b_val, "res_sub")
        .unwrap();
    codegen
        .builder
        .build_unconditional_branch(calc_exit)
        .unwrap();

    // op_div: if b == 0 panic(MathError) else a / b
    codegen.builder.position_at_end(op_div);
    let div_zero = context.append_basic_block(calc_fn, "div_zero");
    let div_ok = context.append_basic_block(calc_fn, "div_ok");
    let is_zero = codegen
        .builder
        .build_int_compare(IntPredicate::EQ, b_val, i32_type.const_zero(), "is_zero")
        .unwrap();
    codegen
        .builder
        .build_conditional_branch(is_zero, div_zero, div_ok)
        .unwrap();

    // div_ok
    codegen.builder.position_at_end(div_ok);
    let res_div = codegen
        .builder
        .build_int_signed_div(a_val, b_val, "res_div")
        .unwrap();
    codegen
        .builder
        .build_unconditional_branch(calc_exit)
        .unwrap();

    // div_zero
    codegen.builder.position_at_end(div_zero);
    let err_alloc = codegen
        .emit_fast_path_alloc(calc_fn, local_reg, 16, 8)
        .expect("alloc MathError");
    codegen
        .builder
        .build_store(err_alloc, div_zero_str)
        .unwrap();
    let msg_ptr = codegen
        .builder
        .build_extract_value(div_zero_str, 0, "msg_ptr")
        .unwrap()
        .into_pointer_value();
    let msg_len = codegen
        .builder
        .build_extract_value(div_zero_str, 1, "msg_len")
        .unwrap()
        .into_int_value();
    codegen.emit_panic(msg_ptr, msg_len).expect("panic call");

    // op_else
    codegen.builder.position_at_end(op_else);
    codegen.emit_region_epilogue(local_reg).expect("epilogue");
    codegen
        .emit_exit(i32_type.const_int(1, false))
        .expect("exit call");

    // calc_exit
    codegen.builder.position_at_end(calc_exit);
    let phi = codegen.builder.build_phi(i32_type, "result").unwrap();
    phi.add_incoming(&[
        (&res_mul, op_mul),
        (&res_add, op_add),
        (&res_sub, op_sub),
        (&res_div, div_ok),
    ]);
    codegen.emit_region_epilogue(local_reg).expect("epilogue");
    codegen
        .builder
        .build_return(Some(&phi.as_basic_value()))
        .unwrap();

    // 5. Function: i32 @main(i32 %argc, ptr %argv)
    let main_fn_ty = i32_type.fn_type(&[i32_type.into(), ptr_type.into()], false);
    let main_fn = codegen.module.add_function("main", main_fn_ty, None);
    main_fn.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32);
    codegen.setup_personality_function(main_fn);

    let main_entry = context.append_basic_block(main_fn, "entry");
    codegen.builder.position_at_end(main_entry);

    let argc_arg = main_fn.get_first_param().unwrap().into_int_value();
    let argv_arg = main_fn.get_params()[1].into_pointer_value();
    codegen
        .intrinsic
        .setup_main_args(&codegen.module, &codegen.builder, argc_arg, argv_arg)
        .expect("setup main args");

    let app_reg = codegen
        .emit_region_prologue(65536, "_app_region")
        .expect("app region");

    // set input: string[] = args()
    let args_slice = codegen
        .emit_args(app_reg)
        .expect("emit args")
        .into_struct_value();
    let input_ptr = codegen
        .builder
        .build_extract_value(args_slice, 0, "input_ptr")
        .unwrap()
        .into_pointer_value();
    let input_len = codegen
        .builder
        .build_extract_value(args_slice, 1, "input_len")
        .unwrap()
        .into_int_value();

    // if input.len() != 3 { exit(1) }
    let is_not_3 = codegen
        .builder
        .build_int_compare(
            IntPredicate::NE,
            input_len,
            i64_type.const_int(3, false),
            "is_not_3",
        )
        .unwrap();

    let arg_err = context.append_basic_block(main_fn, "arg_err");
    let parse_args = context.append_basic_block(main_fn, "parse_args");
    codegen
        .builder
        .build_conditional_branch(is_not_3, arg_err, parse_args)
        .unwrap();

    // arg_err: exit(1)
    codegen.builder.position_at_end(arg_err);
    codegen.emit_region_epilogue(app_reg).expect("epilogue");
    codegen
        .emit_exit(i32_type.const_int(1, false))
        .expect("exit call");

    // parse_args:
    codegen.builder.position_at_end(parse_args);

    // input_a = cast(input[0], "int32")
    let elem0_ptr = codegen
        .emit_ptr_offset(input_ptr, i64_type.const_zero(), "elem0_ptr")
        .unwrap();
    let elem0_val = codegen
        .builder
        .build_load(str_ty, elem0_ptr, "elem0")
        .unwrap();
    let cast_a = codegen
        .emit_cast(elem0_val, &Type::String, &Type::Int32, Some(app_reg))
        .expect("cast input[0]");
    let input_a = cast_a.into_int_value();

    // input_op = cast(input[1], "char")
    let elem1_ptr = codegen
        .emit_ptr_offset(input_ptr, i64_type.const_int(16, false), "elem1_ptr")
        .unwrap();
    let elem1_val = codegen
        .builder
        .build_load(str_ty, elem1_ptr, "elem1")
        .unwrap();
    let cast_op = codegen
        .emit_cast(elem1_val, &Type::String, &Type::Char, Some(app_reg))
        .expect("cast input[1]");
    let input_op = cast_op.into_int_value();

    // input_b = cast(input[2], "int32")
    let elem2_ptr = codegen
        .emit_ptr_offset(input_ptr, i64_type.const_int(32, false), "elem2_ptr")
        .unwrap();
    let elem2_val = codegen
        .builder
        .build_load(str_ty, elem2_ptr, "elem2")
        .unwrap();
    let cast_b = codegen
        .emit_cast(elem2_val, &Type::String, &Type::Int32, Some(app_reg))
        .expect("cast input[2]");
    let input_b = cast_b.into_int_value();

    // sus { calc(...) } catch e: MathError { exit(1) } catch e { exit(1) }
    let sus_ok = context.append_basic_block(main_fn, "sus_ok");
    let sus_landingpad = context.append_basic_block(main_fn, "sus_landingpad");

    let calc_call = codegen
        .emit_invoke(
            calc_fn,
            &[input_a.into(), input_op.into(), input_b.into()],
            sus_ok,
            sus_landingpad,
            "calc_res",
        )
        .expect("invoke calc");
    calc_call.set_call_convention(inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32);

    // sus_ok
    codegen.builder.position_at_end(sus_ok);
    codegen.emit_region_epilogue(app_reg).expect("epilogue");
    codegen
        .builder
        .build_return(Some(&i32_type.const_zero()))
        .unwrap();

    // sus_landingpad
    codegen.builder.position_at_end(sus_landingpad);
    let eh_val = codegen
        .emit_landing_pad(personality, &[math_typeinfo], true, true, "eh_info")
        .expect("landing pad");
    let selector = codegen
        .extract_exception_selector(eh_val, "sel")
        .expect("sel");
    let math_tid = codegen
        .emit_typeid_for(math_typeinfo, "math_tid")
        .expect("typeid");

    let is_math = codegen
        .builder
        .build_int_compare(IntPredicate::EQ, selector, math_tid, "is_math")
        .unwrap();

    let catch_math = context.append_basic_block(main_fn, "catch_math");
    let catch_any = context.append_basic_block(main_fn, "catch_any");
    codegen
        .builder
        .build_conditional_branch(is_math, catch_math, catch_any)
        .unwrap();

    // catch_math
    codegen.builder.position_at_end(catch_math);
    codegen.emit_region_epilogue(app_reg).expect("epilogue");
    codegen
        .emit_exit(i32_type.const_int(1, false))
        .expect("exit call");

    // catch_any
    codegen.builder.position_at_end(catch_any);
    codegen.emit_region_epilogue(app_reg).expect("epilogue");
    codegen
        .emit_exit(i32_type.const_int(1, false))
        .expect("exit call");

    // Verify unoptimized module
    assert!(
        codegen.module.verify().is_ok(),
        "module verification failed: {:?}",
        codegen.module.verify().err()
    );

    let raw_ir = codegen.emit_llvm_ir_to_string();
    assert!(!raw_ir.is_empty());

    // Now run Optimizer at O2
    let optimizer = Optimizer::new(OptLevel::O2, yukic_opt::LtoKind::None);
    optimizer
        .optimize(&codegen.module, TargetTriple::host())
        .expect("O2 optimization failed");

    let opt_ir = codegen.emit_llvm_ir_to_string();
    assert!(!opt_ir.is_empty());
}
