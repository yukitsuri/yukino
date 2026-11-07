#![deny(unsafe_code)]

use std::sync::{Arc, Mutex};

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::context::Context;
use yukic_ast::attr::{AttributeArgs, AttributeNode};

use yukic_ast::expr::{Expr, ExprKind, LiteralValue};
use yukic_ast::ident::Ident;
use yukic_codegen::CompiledCgu;
use yukic_opt::*;
use yukic_span::{BytePos, FileId, Span};
use yukic_target::TargetTriple;

fn dummy_span() -> Span {
    Span::new(FileId(0), BytePos(0), BytePos(0))
}

fn ident(name: &str) -> Ident {
    Ident::new(name, dummy_span())
}

#[test]
fn test_opt_level_parsing_and_display() {
    assert_eq!(OptLevel::parse_str("0").unwrap(), OptLevel::O0);
    assert_eq!(OptLevel::parse_str("O0").unwrap(), OptLevel::O0);
    assert_eq!(OptLevel::parse_str("none").unwrap(), OptLevel::O0);
    assert_eq!(OptLevel::parse_str("1").unwrap(), OptLevel::O1);
    assert_eq!(OptLevel::parse_str("O1").unwrap(), OptLevel::O1);
    assert_eq!(OptLevel::parse_str("2").unwrap(), OptLevel::O2);
    assert_eq!(OptLevel::parse_str("O2").unwrap(), OptLevel::O2);
    assert_eq!(OptLevel::parse_str("default").unwrap(), OptLevel::O2);
    assert_eq!(OptLevel::parse_str("3").unwrap(), OptLevel::O3);
    assert_eq!(OptLevel::parse_str("O3").unwrap(), OptLevel::O3);
    assert_eq!(OptLevel::parse_str("aggressive").unwrap(), OptLevel::O3);
    assert_eq!(OptLevel::parse_str("s").unwrap(), OptLevel::Os);
    assert_eq!(OptLevel::parse_str("os").unwrap(), OptLevel::Os);
    assert_eq!(OptLevel::parse_str("size").unwrap(), OptLevel::Os);
    assert_eq!(OptLevel::parse_str("z").unwrap(), OptLevel::Oz);
    assert_eq!(OptLevel::parse_str("oz").unwrap(), OptLevel::Oz);
    assert_eq!(OptLevel::parse_str("minsize").unwrap(), OptLevel::Oz);

    assert_eq!(OptLevel::O0.as_str(), "O0");
    assert_eq!(OptLevel::O1.as_str(), "O1");
    assert_eq!(OptLevel::O2.as_str(), "O2");
    assert_eq!(OptLevel::O3.as_str(), "O3");
    assert_eq!(OptLevel::Os.as_str(), "Os");
    assert_eq!(OptLevel::Oz.as_str(), "Oz");

    assert!(OptLevel::parse_str("invalid").is_err());
    assert!(OptLevel::parse_str("4").is_err());
}

#[test]
fn test_lto_kind_parsing_and_display() {
    assert_eq!(LtoKind::parse_str("none").unwrap(), LtoKind::None);
    assert_eq!(LtoKind::parse_str("false").unwrap(), LtoKind::None);
    assert_eq!(LtoKind::parse_str("off").unwrap(), LtoKind::None);
    assert_eq!(LtoKind::parse_str("").unwrap(), LtoKind::None);
    assert_eq!(LtoKind::parse_str("thin").unwrap(), LtoKind::Thin);
    assert_eq!(LtoKind::parse_str("true").unwrap(), LtoKind::Thin);
    assert_eq!(LtoKind::parse_str("on").unwrap(), LtoKind::Thin);
    assert_eq!(LtoKind::parse_str("full").unwrap(), LtoKind::Full);
    assert_eq!(LtoKind::parse_str("fat").unwrap(), LtoKind::Full);

    assert!(!LtoKind::None.is_enabled());
    assert!(LtoKind::Thin.is_enabled());
    assert!(LtoKind::Full.is_enabled());

    assert_eq!(LtoKind::None.as_str(), "none");
    assert_eq!(LtoKind::Thin.as_str(), "thin");
    assert_eq!(LtoKind::Full.as_str(), "full");

    assert!(LtoKind::parse_str("unknown").is_err());
}

#[test]
fn test_target_machine_creation_all_targets() {
    for target in TargetTriple::all() {
        let machine = create_target_machine(*target, OptLevel::O2, None, None);
        assert!(
            machine.is_ok(),
            "TargetMachine creation failed for {}: {:?}",
            target.as_str(),
            machine.err()
        );
    }
}

#[test]
fn test_new_pass_manager_pipelines() {
    let context = Context::create();
    let levels = [
        OptLevel::O0,
        OptLevel::O1,
        OptLevel::O2,
        OptLevel::O3,
        OptLevel::Os,
        OptLevel::Oz,
    ];

    for level in levels {
        let module = context.create_module(&format!("test_mod_{}", level.as_str()));
        let i64_type = context.i64_type();
        let fn_type = i64_type.fn_type(&[i64_type.into()], false);
        let f = module.add_function("compute", fn_type, None);
        let bb = context.append_basic_block(f, "entry");
        let builder = context.create_builder();
        builder.position_at_end(bb);
        let param = f.get_first_param().unwrap().into_int_value();
        let added = builder.build_int_add(param, param, "x2").unwrap();
        builder.build_return(Some(&added)).unwrap();

        let opt = Optimizer::new(level, LtoKind::None);
        let res = opt.optimize(&module, TargetTriple::X86_64UnknownLinuxGnu);
        assert!(
            res.is_ok(),
            "pipeline for level {} failed: {:?}",
            level.as_str(),
            res.err()
        );
    }
}

#[test]
fn test_stable_function_cse_and_attributes() {
    let context = Context::create();
    let module = context.create_module("test_stable");
    let i64_type = context.i64_type();
    let fn_type = i64_type.fn_type(&[i64_type.into()], false);

    // declare pure function
    let pure_fn = module.add_function("pure_calc", fn_type, None);
    let attr_node = AttributeNode::new(ident("stable"), AttributeArgs::None, dummy_span());
    apply_ast_attributes(&context, pure_fn, &[attr_node]);

    // verify stable attributes: memory(none), nounwind, willreturn
    let mem_id = Attribute::get_named_enum_kind_id("memory");
    let nounwind_id = Attribute::get_named_enum_kind_id("nounwind");
    let willreturn_id = Attribute::get_named_enum_kind_id("willreturn");
    assert!(
        pure_fn
            .get_enum_attribute(AttributeLoc::Function, mem_id)
            .is_some()
    );
    assert!(
        pure_fn
            .get_enum_attribute(AttributeLoc::Function, nounwind_id)
            .is_some()
    );
    assert!(
        pure_fn
            .get_enum_attribute(AttributeLoc::Function, willreturn_id)
            .is_some()
    );

    // define caller calling pure_calc(x) twice
    let caller_fn = module.add_function("caller", fn_type, None);
    let bb = context.append_basic_block(caller_fn, "entry");
    let builder = context.create_builder();
    builder.position_at_end(bb);
    let param = caller_fn.get_first_param().unwrap();

    let c1 = builder
        .build_call(pure_fn, &[param.into()], "c1")
        .unwrap()
        .try_as_basic_value()
        .basic()
        .unwrap();
    let c2 = builder
        .build_call(pure_fn, &[param.into()], "c2")
        .unwrap()
        .try_as_basic_value()
        .basic()
        .unwrap();
    let sum = builder
        .build_int_add(c1.into_int_value(), c2.into_int_value(), "sum")
        .unwrap();
    builder.build_return(Some(&sum)).unwrap();

    let pre_ir = module.print_to_string().to_string();
    assert_eq!(pre_ir.matches("call i64 @pure_calc").count(), 2);

    let opt = Optimizer::new(OptLevel::O2, LtoKind::None);
    opt.optimize(&module, TargetTriple::X86_64UnknownLinuxGnu)
        .expect("O2 optimization failed");

    let post_ir = module.print_to_string().to_string();
    // LLVM CSE dedupes duplicate pure calls into a single call
    assert_eq!(post_ir.matches("call i64 @pure_calc").count(), 1);
}

#[test]
fn test_inlining_always_and_never() {
    let context = Context::create();
    let module = context.create_module("test_inline");
    let i64_type = context.i64_type();
    let fn_type = i64_type.fn_type(&[i64_type.into()], false);

    // callee with alwaysinline
    let callee_always = module.add_function("always_inlined", fn_type, None);
    let bb = context.append_basic_block(callee_always, "entry");
    let builder = context.create_builder();
    builder.position_at_end(bb);
    let p = callee_always.get_first_param().unwrap().into_int_value();
    let res = builder.build_int_mul(p, p, "sq").unwrap();
    builder.build_return(Some(&res)).unwrap();

    let arg_expr = Expr::new(
        ExprKind::Literal(LiteralValue::String("always".to_string())),
        dummy_span(),
    );
    let inline_always_attr = AttributeNode::new(
        ident("inline"),
        AttributeArgs::Positional(vec![arg_expr]),
        dummy_span(),
    );
    apply_ast_attributes(&context, callee_always, &[inline_always_attr]);

    // callee with noinline
    let callee_never = module.add_function("never_inlined", fn_type, None);
    let bb2 = context.append_basic_block(callee_never, "entry");
    builder.position_at_end(bb2);
    let p2 = callee_never.get_first_param().unwrap().into_int_value();
    let res2 = builder.build_int_add(p2, p2, "dub").unwrap();
    builder.build_return(Some(&res2)).unwrap();
    let noinline_attr = AttributeNode::new(ident("no_inline"), AttributeArgs::None, dummy_span());
    apply_ast_attributes(&context, callee_never, &[noinline_attr]);

    // caller invoking both
    let caller = module.add_function("caller_fn", fn_type, None);
    let bb3 = context.append_basic_block(caller, "entry");
    builder.position_at_end(bb3);
    let cp = caller.get_first_param().unwrap();
    let c1 = builder
        .build_call(callee_always, &[cp.into()], "c1")
        .unwrap()
        .try_as_basic_value()
        .basic()
        .unwrap();
    let c2 = builder
        .build_call(callee_never, &[c1.into()], "c2")
        .unwrap()
        .try_as_basic_value()
        .basic()
        .unwrap();
    builder.build_return(Some(&c2)).unwrap();

    let opt = Optimizer::new(OptLevel::O3, LtoKind::None);
    opt.optimize(&module, TargetTriple::X86_64UnknownLinuxGnu)
        .expect("O3 optimization failed");

    let post_ir = module.print_to_string().to_string();
    // caller_fn should inline always_inlined (eliminating the call), but keep never_inlined
    assert!(!post_ir.contains("call i64 @always_inlined"));
    assert!(post_ir.contains("call i64 @never_inlined"));
}

#[test]
fn test_panic_attributes() {
    let context = Context::create();
    let module = context.create_module("test_panic");
    let void_ty = context.void_type().fn_type(&[], false);
    let panic_fn = module.add_function("yukino_raise_panic", void_ty, None);
    annotate_panic(&context, panic_fn);

    let cold_id = Attribute::get_named_enum_kind_id("cold");
    let noreturn_id = Attribute::get_named_enum_kind_id("noreturn");
    assert!(
        panic_fn
            .get_enum_attribute(AttributeLoc::Function, cold_id)
            .is_some()
    );
    assert!(
        panic_fn
            .get_enum_attribute(AttributeLoc::Function, noreturn_id)
            .is_some()
    );
}

#[test]
fn test_tail_call_annotation() {
    let context = Context::create();
    let module = context.create_module("test_tail");
    let void_ty = context.void_type().fn_type(&[], false);
    let target_fn = module.add_function("recur_target", void_ty, None);
    let caller = module.add_function("recur_caller", void_ty, None);
    let bb = context.append_basic_block(caller, "entry");
    let builder = context.create_builder();
    builder.position_at_end(bb);
    let call = builder.build_call(target_fn, &[], "tail_call").unwrap();
    annotate_tail_call(call);
    builder.build_return(None).unwrap();

    assert!(call.is_tail_call());
    assert_eq!(
        call.get_tail_call_kind(),
        inkwell::llvm_sys::LLVMTailCallKind::LLVMTailCallKindMustTail
    );

    let ir = module.print_to_string().to_string();
    assert!(ir.contains("musttail call void @recur_target()"));
}

#[test]
fn test_lto_bitcode_generation() {
    let context = Context::create();
    let module = context.create_module("test_lto");
    let void_ty = context.void_type().fn_type(&[], false);
    let f = module.add_function("dummy", void_ty, None);
    let bb = context.append_basic_block(f, "entry");
    let builder = context.create_builder();
    builder.position_at_end(bb);
    builder.build_return(None).unwrap();

    let bytes = write_bitcode_to_memory(&module);
    assert!(bytes.len() >= 4);
    // LLVM bitcode magic bytes: 'B', 'C', 0xC0, 0xDE
    assert_eq!(&bytes[0..4], &[b'B', b'C', 0xC0, 0xDE]);

    let tmp_dir = std::env::temp_dir();
    let bc_path = tmp_dir.join("yukino_test_output.bc");
    let opt = Optimizer::new(OptLevel::O2, LtoKind::Thin);
    let res = opt.emit_bitcode(&module, &bc_path);
    assert!(res.is_ok());
    assert!(bc_path.exists());
    let read_back = std::fs::read(&bc_path).unwrap();
    assert_eq!(&read_back[0..4], &[b'B', b'C', 0xC0, 0xDE]);
    let _ = std::fs::remove_file(bc_path);
}

#[test]
fn test_parallel_cgu_optimization() {
    let cgu1 = CompiledCgu {
        name: "cgu_math".to_string(),
        ll_ir: r"
define i64 @add(i64 %a, i64 %b) {
entry:
  %res = add i64 %a, %b
  ret i64 %res
}
"
        .to_string(),
        item_count: 1,
        estimated_weight: 80,
    };

    let cgu2 = CompiledCgu {
        name: "cgu_string".to_string(),
        ll_ir: r"
define i64 @identity(i64 %x) {
entry:
  ret i64 %x
}
"
        .to_string(),
        item_count: 1,
        estimated_weight: 20,
    };

    let cgu3 = CompiledCgu {
        name: "cgu_io".to_string(),
        ll_ir: r"
define void @noop() {
entry:
  ret void
}
"
        .to_string(),
        item_count: 1,
        estimated_weight: 50,
    };

    let events = Arc::new(Mutex::new(Vec::new()));
    let events_cb = Arc::clone(&events);

    let opt = Optimizer::new(OptLevel::O2, LtoKind::Thin);
    let results = opt
        .optimize_cgus_parallel(
            vec![cgu1, cgu2, cgu3],
            TargetTriple::X86_64UnknownLinuxGnu,
            2,
            move |tid, ev| {
                let formatted = format_opt_progress(tid, &ev);
                events_cb.lock().unwrap().push(formatted);
            },
        )
        .expect("parallel cgu optimization must succeed");

    assert_eq!(results.len(), 3);
    for r in &results {
        assert!(!r.ll_ir.is_empty());
        assert!(r.bitcode.is_some());
    }

    let recorded = events.lock().unwrap();
    assert!(!recorded.is_empty());
    // verify format strings
    let has_opt_log = recorded.iter().any(|s| s.contains("optimizing cgu_"));
    let has_idle_log = recorded.iter().any(|s| s.contains("[waiting]: idle"));
    assert!(has_opt_log, "must contain optimizing logs: {recorded:?}");
    assert!(has_idle_log, "must contain idle logs: {recorded:?}");
}

#[test]
fn test_emit_compiler_test_artifacts() {
    let context = Context::create();
    let module = context.create_module("test_opt_module");
    let target = TargetTriple::X86_64UnknownLinuxGnu;
    let inkwell_triple = inkwell::targets::TargetTriple::create(target.spec().triple);
    module.set_triple(&inkwell_triple);
    let target_data = inkwell::targets::TargetData::create(target.spec().data_layout);
    module.set_data_layout(&target_data.get_data_layout());

    let i64_type = context.i64_type();
    let void_type = context.void_type();
    let fn_i64_i64 = i64_type.fn_type(&[i64_type.into()], false);
    let fn_void = void_type.fn_type(&[], false);

    // 1. Pure function with #[stable]
    let pure_hash = module.add_function("pure_hash", fn_i64_i64, None);
    annotate_stable(&context, pure_hash);

    // 2. Caller testing CSE
    let caller = module.add_function("calculate_hashes", fn_i64_i64, None);
    let bb1 = context.append_basic_block(caller, "entry");
    let builder = context.create_builder();
    builder.position_at_end(bb1);
    let p0 = caller.get_first_param().unwrap();
    let c1 = builder
        .build_call(pure_hash, &[p0.into()], "c1")
        .unwrap()
        .try_as_basic_value()
        .basic()
        .unwrap();
    let c2 = builder
        .build_call(pure_hash, &[p0.into()], "c2")
        .unwrap()
        .try_as_basic_value()
        .basic()
        .unwrap();
    let sum = builder
        .build_int_add(c1.into_int_value(), c2.into_int_value(), "sum")
        .unwrap();
    builder.build_return(Some(&sum)).unwrap();

    // 3. Always inline function
    let fast_double = module.add_function("fast_double", fn_i64_i64, None);
    annotate_inline(&context, fast_double, InlineKind::Always);
    let bb2 = context.append_basic_block(fast_double, "entry");
    builder.position_at_end(bb2);
    let p_fd = fast_double.get_first_param().unwrap().into_int_value();
    let fd_res = builder
        .build_int_mul(p_fd, context.i64_type().const_int(2, false), "d")
        .unwrap();
    builder.build_return(Some(&fd_res)).unwrap();

    // 4. No-inline function
    let slow_step = module.add_function("slow_step", fn_i64_i64, None);
    annotate_inline(&context, slow_step, InlineKind::Never);
    let bb3 = context.append_basic_block(slow_step, "entry");
    builder.position_at_end(bb3);
    let p_ss = slow_step.get_first_param().unwrap().into_int_value();
    let ss_res = builder
        .build_int_add(p_ss, context.i64_type().const_int(1, false), "inc")
        .unwrap();
    builder.build_return(Some(&ss_res)).unwrap();

    // 5. Inlining caller
    let pipeline_fn = module.add_function("pipeline_step", fn_i64_i64, None);
    let bb4 = context.append_basic_block(pipeline_fn, "entry");
    builder.position_at_end(bb4);
    let p_pipe = pipeline_fn.get_first_param().unwrap();
    let call_fast = builder
        .build_call(fast_double, &[p_pipe.into()], "fast")
        .unwrap()
        .try_as_basic_value()
        .basic()
        .unwrap();
    let call_slow = builder
        .build_call(slow_step, &[call_fast.into()], "slow")
        .unwrap()
        .try_as_basic_value()
        .basic()
        .unwrap();
    builder.build_return(Some(&call_slow)).unwrap();

    // 6. Tail call recur worker
    let ext_fn = module.add_function("recur_external", fn_i64_i64, None);
    let recur_fn = module.add_function("recur_worker", fn_i64_i64, None);
    let bb5 = context.append_basic_block(recur_fn, "entry");
    builder.position_at_end(bb5);
    let p_rec = recur_fn.get_first_param().unwrap().into_int_value();
    let rec_arg = builder
        .build_int_sub(p_rec, context.i64_type().const_int(1, false), "sub")
        .unwrap();
    let rec_call = builder
        .build_call(ext_fn, &[rec_arg.into()], "rec_call")
        .unwrap();
    annotate_tail_call(rec_call);
    builder
        .build_return(Some(&rec_call.try_as_basic_value().basic().unwrap()))
        .unwrap();

    // 7. Panic function
    let panic_fn = module.add_function("yukino_raise_panic", fn_void, None);
    annotate_panic(&context, panic_fn);
    let err_fn = module.add_function("trigger_err", fn_void, None);
    let bb6 = context.append_basic_block(err_fn, "entry");
    builder.position_at_end(bb6);
    builder.build_call(panic_fn, &[], "panic_call").unwrap();
    builder.build_unreachable().unwrap();

    // Optimize
    let opt = Optimizer::new(OptLevel::O2, LtoKind::Thin);
    opt.optimize(&module, target)
        .expect("module optimization failed");

    let opt_ir = module.print_to_string().to_string();
    assert!(opt_ir.contains("memory(none)"));
    assert!(opt_ir.contains("musttail call"));
    assert!(
        opt_ir.contains("cold noreturn")
            || (opt_ir.contains("cold") && opt_ir.contains("noreturn"))
    );
    assert!(!opt_ir.contains("call i64 @fast_double"));
    assert!(opt_ir.contains("call i64 @slow_step"));

    // ...
    let target_dir = std::env::temp_dir().join("yukic_opt_test");
    let _ = std::fs::create_dir_all(&target_dir);

    let ll_path = target_dir.join("test-opt.ll");
    let bc_path = target_dir.join("test-opt.bc");

    std::fs::write(&ll_path, &opt_ir).expect("write test-opt.ll");
    write_bitcode_to_path(&module, &bc_path).expect("write test-opt.bc");

    assert!(ll_path.exists());
    assert!(bc_path.exists());
    let _ = std::fs::remove_file(ll_path);
    let _ = std::fs::remove_file(bc_path);
}
