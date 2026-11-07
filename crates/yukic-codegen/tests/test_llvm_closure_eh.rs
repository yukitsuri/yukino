#![deny(unsafe_code)]

use inkwell::context::Context;
use std::fs;
use std::path::Path;
use yukic_codegen::{CapturedVar, CleanupAction, CodeGenerator, ScopeKind};
use yukic_target::TargetTriple;
use yukic_types::Type;

fn build_closure_and_eh_demo<'ctx>(codegen: &mut CodeGenerator<'ctx>, context: &'ctx Context) {
    let i64_type = context.i64_type();
    let ptr_type = codegen.ptr_type();

    // 1. Lambda definition: fastcc i64(ptr %env_ptr, i64 %delta)
    let lambda_fn_ty = i64_type.fn_type(&[ptr_type.into(), i64_type.into()], false);
    let lambda_fn = codegen
        .module
        .add_function("_Y_lambda_counter", lambda_fn_ty, None);
    lambda_fn.set_linkage(inkwell::module::Linkage::Internal);
    lambda_fn.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32);

    let lambda_bb = context.append_basic_block(lambda_fn, "entry");
    codegen.builder.position_at_end(lambda_bb);
    let env_param = lambda_fn.get_first_param().unwrap().into_pointer_value();
    let delta_param = lambda_fn.get_params()[1].into_int_value();

    let captures = vec![
        CapturedVar::new("count", Type::Int64),
        CapturedVar::new("tag", Type::String),
    ];
    let env_layout = codegen
        .create_closure_env_layout("counter", &captures)
        .expect("env layout");

    let unpacked = codegen
        .unpack_closure_environment(env_param, &env_layout)
        .expect("unpack");
    let count_val = unpacked[0].1.into_int_value();
    let sum = codegen
        .builder
        .build_int_add(count_val, delta_param, "new_count")
        .unwrap();
    codegen.builder.build_return(Some(&sum)).unwrap();

    // 2. Risky function that can panic/throw: fastcc i64(i64)
    let risky_ty = i64_type.fn_type(&[i64_type.into()], false);
    let risky_fn = codegen.module.add_function("risky_compute", risky_ty, None);
    risky_fn.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32);

    // 3. Worker function containing region, closure creation, invoke, and sus/catch
    let worker_ty = i64_type.fn_type(&[], false);
    let worker_fn = codegen.module.add_function("worker_task", worker_ty, None);
    worker_fn.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32);

    let personality = codegen.setup_personality_function(worker_fn);

    let worker_entry = context.append_basic_block(worker_fn, "entry");
    codegen.builder.position_at_end(worker_entry);

    // Region prologue
    let local_reg = codegen
        .emit_region_prologue(4096, "local_reg")
        .expect("prologue");

    // ScopeStack demo
    let mut scope_stack = yukic_codegen::ScopeStack::new();
    scope_stack.push_scope(ScopeKind::Function);
    scope_stack
        .add_cleanup(CleanupAction::RegionEpilogue(local_reg))
        .expect("cleanup reg");

    // Closure environment allocation in local region
    let count_init = i64_type.const_int(100, false).into();
    let tag_init = codegen
        .rodata
        .emit_string(&codegen.module, "task_tag")
        .into();
    let env_alloc = codegen
        .allocate_and_init_closure_env(worker_fn, local_reg, &env_layout, &[count_init, tag_init])
        .expect("alloc env");

    let closure_fat = codegen
        .make_closure_fat_pointer(
            lambda_fn.as_global_value().as_pointer_value(),
            env_alloc,
            "c_counter",
        )
        .expect("fat ptr");

    // Store in alloca to demonstrate dynamic indirect closure call
    let closure_slot = codegen
        .builder
        .build_alloca(codegen.closure_type(), "closure_slot")
        .unwrap();
    codegen
        .builder
        .build_store(closure_slot, closure_fat)
        .unwrap();

    let loaded_fat = codegen
        .build_load(codegen.closure_type(), closure_slot, "loaded_c")
        .unwrap()
        .into_struct_value();

    let delta_val = i64_type.const_int(25, false);
    let _closure_res = codegen
        .emit_closure_call(loaded_fat, lambda_fn_ty, &[delta_val.into()], None, "c_res")
        .expect("closure call");

    // Blocks for sus / catch / finally
    let sus_cont = context.append_basic_block(worker_fn, "sus_cont_ok");
    let sus_landing = context.append_basic_block(worker_fn, "sus_landingpad");
    let catch_net = context.append_basic_block(worker_fn, "catch_network_err");
    let catch_all = context.append_basic_block(worker_fn, "catch_any_err");
    let finally_bb = context.append_basic_block(worker_fn, "finally_block");
    let resume_bb = context.append_basic_block(worker_fn, "unwind_resume");

    // Invoke risky function inside sus block
    codegen
        .emit_invoke(
            risky_fn,
            &[delta_val.into()],
            sus_cont,
            sus_landing,
            "inv_risky",
        )
        .expect("invoke");

    // Sus normal continuation
    codegen.builder.position_at_end(sus_cont);
    codegen
        .builder
        .build_unconditional_branch(finally_bb)
        .unwrap();

    // Landing pad with typed catch and catch-all
    codegen.builder.position_at_end(sus_landing);
    let typeinfo_net = codegen.get_or_create_typeinfo("NetworkError");
    let eh_val = codegen
        .emit_landing_pad(personality, &[typeinfo_net], true, true, "eh_info")
        .expect("landing pad");

    let _exc_ptr = codegen
        .extract_exception_pointer(eh_val, "exc_ptr")
        .expect("exc ptr");
    let selector = codegen
        .extract_exception_selector(eh_val, "selector")
        .expect("selector");

    // Match selector against NetworkError
    let net_tid = codegen.emit_typeid_for(typeinfo_net, "net_tid").unwrap();
    let is_network_err = codegen
        .builder
        .build_int_compare(inkwell::IntPredicate::EQ, selector, net_tid, "is_net")
        .unwrap();

    let check_all_bb = context.append_basic_block(worker_fn, "check_all");
    codegen
        .builder
        .build_conditional_branch(is_network_err, catch_net, check_all_bb)
        .unwrap();

    // Check catch-all
    codegen.builder.position_at_end(check_all_bb);
    codegen
        .builder
        .build_unconditional_branch(catch_all)
        .unwrap();

    // Catch handlers
    codegen.builder.position_at_end(catch_net);
    codegen
        .builder
        .build_unconditional_branch(finally_bb)
        .unwrap();

    codegen.builder.position_at_end(catch_all);
    codegen
        .builder
        .build_unconditional_branch(finally_bb)
        .unwrap();

    // Finally block: executes cleanups and returns
    codegen.builder.position_at_end(finally_bb);
    codegen.emit_region_epilogue(local_reg).expect("epilogue");
    let ret_val = i64_type.const_int(0, false);
    codegen.builder.build_return(Some(&ret_val)).unwrap();

    // Resume block for unhandled exceptions
    codegen.builder.position_at_end(resume_bb);
    codegen.emit_resume(eh_val).expect("resume");
}

fn verify_emitted_ir(ir: &str) {
    // 1. Closures verification
    assert!(ir.contains("%struct.env_counter = type { i64, { ptr, i64 } }"));
    assert!(ir.contains("define internal fastcc i64 @_Y_lambda_counter(ptr %0, i64 %1)"));
    assert!(ir.contains("%count = load i64, ptr %count_gep"));
    assert!(ir.contains("%c_counter_fat = insertvalue { ptr, ptr } { ptr @_Y_lambda_counter, ptr undef }, ptr %alloc_ptr, 1"));
    assert!(ir.contains("%c_res_fn_ptr = extractvalue { ptr, ptr } %loaded_c, 0"));
    assert!(ir.contains("%c_res_env_ptr = extractvalue { ptr, ptr } %loaded_c, 1"));
    assert!(ir.contains("call fastcc i64 %c_res_fn_ptr(ptr %c_res_env_ptr, i64 25)"));

    // 2. Exception Handling verification
    assert!(ir.contains("@_Y_TYPEINFO_NetworkError = constant ptr null"));
    assert!(ir.contains("declare i32 @yukino_personality(...)"));
    assert!(ir.contains("personality ptr @yukino_personality"));
    assert!(ir.contains("invoke i64 @risky_compute(i64 25)"));
    assert!(ir.contains("to label %sus_cont_ok unwind label %sus_landingpad"));
    assert!(ir.contains("landingpad { ptr, i32 }"));
    assert!(ir.contains("cleanup"));
    assert!(ir.contains("catch ptr @_Y_TYPEINFO_NetworkError"));
    assert!(ir.contains("catch ptr null"));
    assert!(ir.contains("call i32 @llvm.eh.typeid.for.p0(ptr @_Y_TYPEINFO_NetworkError)"));
    assert!(ir.contains("icmp eq i32 %selector, %net_tid"));
    assert!(ir.contains("resume { ptr, i32 } %eh_info"));
}

#[test]
fn test_emit_llvm_closure_eh_ir_file() {
    let context = Context::create();
    let mut codegen = CodeGenerator::new(
        &context,
        "yukino_closure_eh_module",
        TargetTriple::X86_64UnknownLinuxGnu,
    );

    build_closure_and_eh_demo(&mut codegen, &context);

    codegen.module.verify().expect("verify module");

    let ir_content = codegen.emit_llvm_ir_to_string();
    verify_emitted_ir(&ir_content);
}
