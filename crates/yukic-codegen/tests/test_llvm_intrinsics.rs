#![deny(unsafe_code)]

use inkwell::context::Context;
use yukic_codegen::CodeGenerator;
use yukic_target::TargetTriple;
use yukic_types::Type;

fn build_exit_demo<'ctx>(codegen: &mut CodeGenerator<'ctx>, context: &'ctx Context) {
    let void_ty = context.void_type().fn_type(&[], false);
    let abort_fn = codegen
        .module
        .add_function("demo_exit_caller", void_ty, None);
    let entry = context.append_basic_block(abort_fn, "entry");
    codegen.builder.position_at_end(entry);

    let code = context.i32_type().const_int(1, false);
    codegen.emit_exit(code).expect("emit exit");
}

fn build_typeof_demo<'ctx>(codegen: &mut CodeGenerator<'ctx>, context: &'ctx Context) {
    let str_ty = codegen.string_type();
    let fn_ty = str_ty.fn_type(&[], false);
    let typeof_fn = codegen
        .module
        .add_function("demo_typeof_caller", fn_ty, None);
    let entry = context.append_basic_block(typeof_fn, "entry");
    codegen.builder.position_at_end(entry);

    let ty_desc = codegen
        .rodata
        .emit_typeof(&codegen.module, &Type::Array(Box::new(Type::Int32)));
    codegen.builder.build_return(Some(&ty_desc)).unwrap();
}

fn build_numeric_casts<'ctx>(codegen: &mut CodeGenerator<'ctx>, context: &'ctx Context) {
    let p_i64 = codegen
        .builder
        .build_alloca(context.i64_type(), "p_i64")
        .unwrap();
    let v_i64 = codegen
        .builder
        .build_load(context.i64_type(), p_i64, "v_i64")
        .unwrap();
    codegen
        .emit_cast(v_i64, &Type::Int64, &Type::Int32, None)
        .expect("trunc");

    let p_u8 = codegen
        .builder
        .build_alloca(context.i8_type(), "p_u8")
        .unwrap();
    let v_u8 = codegen
        .builder
        .build_load(context.i8_type(), p_u8, "v_u8")
        .unwrap();
    codegen
        .emit_cast(v_u8, &Type::Uint8, &Type::Uint32, None)
        .expect("zext");

    let p_i8 = codegen
        .builder
        .build_alloca(context.i8_type(), "p_i8")
        .unwrap();
    let v_i8 = codegen
        .builder
        .build_load(context.i8_type(), p_i8, "v_i8")
        .unwrap();
    codegen
        .emit_cast(v_i8, &Type::Int8, &Type::Int32, None)
        .expect("sext");

    let p_f32 = codegen
        .builder
        .build_alloca(context.f32_type(), "p_f32")
        .unwrap();
    let v_f32 = codegen
        .builder
        .build_load(context.f32_type(), p_f32, "v_f32")
        .unwrap();
    codegen
        .emit_cast(v_f32, &Type::Float32, &Type::Float64, None)
        .expect("fpext");

    let p_f64 = codegen
        .builder
        .build_alloca(context.f64_type(), "p_f64")
        .unwrap();
    let v_f64 = codegen
        .builder
        .build_load(context.f64_type(), p_f64, "v_f64")
        .unwrap();
    codegen
        .emit_cast(v_f64, &Type::Float64, &Type::Float32, None)
        .expect("fptrunc");
    codegen
        .emit_cast(v_f64, &Type::Float64, &Type::Int32, None)
        .expect("fptosi");
    codegen
        .emit_cast(v_f64, &Type::Float64, &Type::Uint32, None)
        .expect("fptoui");
    codegen
        .emit_cast(v_i64, &Type::Int64, &Type::Float64, None)
        .expect("sitofp");
    codegen
        .emit_cast(v_u8, &Type::Uint8, &Type::Float64, None)
        .expect("uitofp");
}

fn build_ptr_and_bool_casts<'ctx>(codegen: &mut CodeGenerator<'ctx>, context: &'ctx Context) {
    let p_cell = codegen
        .builder
        .build_alloca(context.i64_type(), "p_cell")
        .unwrap();
    let ptr_to_int = codegen
        .emit_cast(
            p_cell.into(),
            &Type::Ptr(Box::new(Type::Int64)),
            &Type::Uint64,
            None,
        )
        .expect("ptrtoint");
    codegen
        .emit_cast(
            ptr_to_int,
            &Type::Uint64,
            &Type::Ptr(Box::new(Type::Int64)),
            None,
        )
        .expect("inttoptr");

    let ptr_to_f64 = codegen
        .emit_cast(
            p_cell.into(),
            &Type::Ptr(Box::new(Type::Int64)),
            &Type::Float64,
            None,
        )
        .expect("ptr_to_f64");
    codegen
        .emit_cast(
            ptr_to_f64,
            &Type::Float64,
            &Type::Ptr(Box::new(Type::Int64)),
            None,
        )
        .expect("f64_to_ptr");

    let p_bool = codegen
        .builder
        .build_alloca(context.bool_type(), "p_bool")
        .unwrap();
    let v_bool = codegen
        .builder
        .build_load(context.bool_type(), p_bool, "v_bool")
        .unwrap();
    let b_to_i = codegen
        .emit_cast(v_bool, &Type::Bool, &Type::Int32, None)
        .expect("b_to_i");
    codegen
        .emit_cast(b_to_i, &Type::Int32, &Type::Bool, None)
        .expect("i_to_b");
}

fn build_string_and_rt_casts<'ctx>(
    codegen: &mut CodeGenerator<'ctx>,
    context: &'ctx Context,
    reg_val: inkwell::values::PointerValue<'ctx>,
) {
    let dummy_bytes = codegen.slice_type().const_zero().into();
    let b_to_s = codegen
        .emit_cast(dummy_bytes, &Type::Bytes, &Type::String, None)
        .expect("b_to_s");
    codegen
        .emit_cast(b_to_s, &Type::String, &Type::Bytes, None)
        .expect("s_to_b");

    let num_val = context.i64_type().const_int(42, false).into();
    let num_str = codegen
        .emit_cast(num_val, &Type::Int64, &Type::String, Some(reg_val))
        .expect("i64_to_s");
    codegen
        .emit_cast(num_str, &Type::String, &Type::Int64, Some(reg_val))
        .expect("s_to_i64");

    let overflow_val = context.i64_type().const_int(9_999_999_999, false).into();
    let err = codegen
        .emit_cast(overflow_val, &Type::Int64, &Type::Int32, None)
        .unwrap_err();
    assert_eq!(
        err,
        "value `9999999999` exceeds 32 bits for target type `int32`"
    );
}

fn build_main_args_demo<'ctx>(codegen: &mut CodeGenerator<'ctx>, context: &'ctx Context) {
    let i32_type = context.i32_type();
    let ptr_type = codegen.ptr_type();
    let main_ty = i32_type.fn_type(&[i32_type.into(), ptr_type.into()], false);
    let main_fn = codegen.module.add_function("main", main_ty, None);
    let entry = context.append_basic_block(main_fn, "entry");
    codegen.builder.position_at_end(entry);

    let argc = main_fn.get_first_param().unwrap().into_int_value();
    let argv = main_fn.get_params()[1].into_pointer_value();
    codegen
        .intrinsic
        .setup_main_args(&codegen.module, &codegen.builder, argc, argv)
        .expect("setup");

    let app_reg = codegen.emit_region_prologue(16384, "app_reg").expect("reg");

    build_numeric_casts(codegen, context);
    build_ptr_and_bool_casts(codegen, context);
    build_string_and_rt_casts(codegen, context, app_reg);
    build_collection_intrinsics_demo(codegen, context, app_reg);

    codegen.emit_args(app_reg).expect("emit args");
    codegen.emit_raw_args(app_reg).expect("emit raw_args");

    codegen.emit_region_epilogue(app_reg).expect("epilogue");
    codegen
        .builder
        .build_return(Some(&i32_type.const_int(0, false)))
        .unwrap();
}

fn build_collection_intrinsics_demo<'ctx>(
    codegen: &mut CodeGenerator<'ctx>,
    context: &'ctx Context,
    region_ptr: inkwell::values::PointerValue<'ctx>,
) {
    let slice_ty = codegen.slice_type();
    let slice_var = codegen.builder.build_alloca(slice_ty, "slice_var").unwrap();

    let dummy_slice = slice_ty.const_zero();
    let _ = codegen
        .emit_slice_len(dummy_slice.into())
        .expect("slice len");
    let _ = codegen
        .emit_slice_cap(dummy_slice.into())
        .expect("slice cap");
    let _ = codegen
        .emit_slice_as_ptr(dummy_slice.into())
        .expect("slice as_ptr");

    let _ = codegen
        .emit_slice_len(slice_var.into())
        .expect("ptr slice len");
    let _ = codegen
        .emit_slice_cap(slice_var.into())
        .expect("ptr slice cap");
    let _ = codegen
        .emit_slice_as_ptr(slice_var.into())
        .expect("ptr slice as_ptr");

    let str_ty = codegen.string_type();
    let dummy_str = str_ty.const_zero();
    let _ = codegen.emit_slice_len(dummy_str.into()).expect("str len");
    let _ = codegen
        .emit_slice_as_ptr(dummy_str.into())
        .expect("str as_ptr");

    let elem_var = codegen
        .builder
        .build_alloca(context.i32_type(), "elem_var")
        .unwrap();
    let i64_ty = context.i64_type();
    let elem_size = i64_ty.const_int(4, false);
    let elem_align = i64_ty.const_int(4, false);
    let index = i64_ty.const_int(0, false);
    let additional = i64_ty.const_int(8, false);
    let layout = yukic_codegen::SliceElemLayout {
        size: elem_size,
        align: elem_align,
    };

    codegen
        .emit_slice_push(region_ptr, slice_var, elem_var, layout)
        .expect("push");
    codegen
        .emit_slice_del(slice_var, index, elem_size)
        .expect("del");
    codegen
        .emit_slice_reserve(region_ptr, slice_var, additional, layout)
        .expect("reserve");

    let raw_ptr = codegen
        .builder
        .build_alloca(context.i8_type(), "raw_bytes")
        .unwrap();
    let raw_len = i64_ty.const_int(12, false);
    let raw_cap = i64_ty.const_int(16, false);
    let _ = codegen
        .emit_string_from_raw_parts(raw_ptr, raw_len)
        .expect("str from raw parts");
    let _ = codegen
        .emit_slice_from_raw_parts(raw_ptr, raw_len, raw_cap)
        .expect("slice from raw parts");

    let _ = codegen
        .emit_slice_elem_ptr(dummy_slice.into(), index, i64_ty.into())
        .expect("elem ptr");

    let sub_start = i64_ty.const_int(2, false);
    let sub_end = i64_ty.const_int(5, false);
    let _ = codegen
        .emit_subslice(
            dummy_str.into(),
            sub_start,
            sub_end,
            i64_ty.const_int(1, false),
            true,
        )
        .expect("substr");
    let _ = codegen
        .emit_subslice(dummy_slice.into(), sub_start, sub_end, elem_size, false)
        .expect("subslice");
}

fn verify_intrinsics_ir(ir: &str) {
    // 1. Exit intrinsic
    assert!(ir.contains("declare void @exit(i32)"));
    assert!(ir.contains("call void @exit(i32 1)"));
    assert!(ir.contains("unreachable"));
    assert!(ir.contains("{ noreturn }"));

    // 2. Typeof intrinsic
    assert!(ir.contains("@.str.type."));
    assert!(ir.contains("int32[]"));

    // 3. Cast numeric instructions
    assert!(ir.contains("trunc i64 %v_i64 to i32"));
    assert!(ir.contains("zext i8 %v_u8 to i32"));
    assert!(ir.contains("sext i8 %v_i8 to i32"));
    assert!(ir.contains("fpext float %v_f32 to double"));
    assert!(ir.contains("fptrunc double %v_f64 to float"));
    assert!(ir.contains("fptosi double %v_f64 to i32"));
    assert!(ir.contains("fptoui double %v_f64 to i32"));
    assert!(ir.contains("sitofp i64 %v_i64 to double"));
    assert!(ir.contains("uitofp i8 %v_u8 to double"));

    // 4. Pointer and two-step float64 conversions
    assert!(ir.contains("ptrtoint ptr %p_cell to i64"));
    assert!(ir.contains("inttoptr i64 %cast_ptrtoint to ptr"));
    assert!(ir.contains("uitofp i64 %ptr_addr to double"));
    assert!(ir.contains("fptoui double %cast_ptr_to_f64 to i64"));
    assert!(ir.contains("inttoptr i64 %f64_to_addr to ptr"));

    // 5. Bool conversions
    assert!(ir.contains("zext i1 %v_bool to i32"));
    assert!(ir.contains("icmp ne i32 %bool_to_int, 0"));

    // 6. Runtime helpers for string & args
    assert!(ir.contains("call void @yukino_rt_int64_to_string"));
    assert!(ir.contains("call i64 @yukino_rt_string_to_int64"));
    assert!(ir.contains("call void @yukino_rt_get_args"));
    assert!(ir.contains("call void @yukino_rt_get_raw_args"));

    // 7. Runtime helpers for collections
    assert!(ir.contains("call void @yukino_rt_slice_push"));
    assert!(ir.contains("call void @yukino_rt_slice_del"));
    assert!(ir.contains("call void @yukino_rt_slice_reserve"));

    // 8. Raw parts declarations and construction
    assert!(ir.contains("declare void @yukino_rt_string_from_raw_parts"));
    assert!(ir.contains("declare { ptr, i64, i64 } @yukino_rt_slice_from_raw_parts"));
    assert!(ir.contains("insertvalue { ptr, i64 } zeroinitializer, ptr %raw_bytes, 0"));
    assert!(ir.contains("insertvalue { ptr, i64, i64 } zeroinitializer, ptr %raw_bytes, 0"));
}

#[test]
fn test_emit_llvm_intrinsics_ir_file() {
    let context = Context::create();
    let mut codegen = CodeGenerator::new(
        &context,
        "yukino_intrinsics_module",
        TargetTriple::X86_64UnknownLinuxGnu,
    );

    build_exit_demo(&mut codegen, &context);
    build_typeof_demo(&mut codegen, &context);
    build_main_args_demo(&mut codegen, &context);

    codegen.module.verify().expect("verify module");

    let ir_content = codegen.emit_llvm_ir_to_string();
    verify_intrinsics_ir(&ir_content);
}

#[test]
fn test_body_compiler_builtins_dispatch() {
    use yukic_ast::expr::{Expr, ExprKind, LiteralValue};
    use yukic_ast::ident::Ident;
    use yukic_span::{BytePos, FileId, Span};

    let context = Context::create();
    let mut codegen = CodeGenerator::new(
        &context,
        "test_body_builtins_mod",
        TargetTriple::X86_64UnknownLinuxGnu,
    );

    let void_ty = context.void_type().fn_type(&[], false);
    let fn_val = codegen.module.add_function("run_builtins", void_ty, None);
    let entry = context.append_basic_block(fn_val, "entry");
    codegen.builder.position_at_end(entry);

    let reg = codegen.emit_region_prologue(4096, "reg").expect("reg");
    let dummy_span = Span::new(FileId(0), BytePos(0), BytePos(1));

    let mut body = yukic_codegen::body::BodyCompiler::new(
        &context,
        &codegen.builder,
        &codegen.module,
        &mut codegen.rodata,
        &codegen.intrinsic,
        &codegen.region,
        &codegen.vtable,
        &codegen.cabi,
        &codegen.registry,
        codegen.target,
        Some(reg),
        None,
        fn_val,
        None,
        false,
    );

    let args_callee = Expr::new(ExprKind::Ident(Ident::new("args", dummy_span)), dummy_span);
    let args_val = body.compile_call(&args_callee, &[]).expect("args call");
    assert!(args_val.is_struct_value());

    let raw_callee = Expr::new(
        ExprKind::Ident(Ident::new("raw_args", dummy_span)),
        dummy_span,
    );
    let raw_val = body.compile_call(&raw_callee, &[]).expect("raw_args call");
    assert!(raw_val.is_struct_value());

    let typeof_callee = Expr::new(
        ExprKind::Ident(Ident::new("typeof", dummy_span)),
        dummy_span,
    );
    let typeof_arg = Expr::new(
        ExprKind::Literal(LiteralValue::Int(42)),
        dummy_span,
    );
    let typeof_val = body
        .compile_call(&typeof_callee, &[typeof_arg])
        .expect("typeof call");
    assert!(typeof_val.is_struct_value());

    let exit_callee = Expr::new(
        ExprKind::Ident(Ident::new("exit", dummy_span)),
        dummy_span,
    );
    let exit_arg = Expr::new(
        ExprKind::Literal(LiteralValue::Int(0)),
        dummy_span,
    );
    body.compile_call(&exit_callee, &[exit_arg]).expect("exit call");

    let ir = codegen.module.print_to_string().to_string();
    assert!(ir.contains("call void @yukino_rt_get_args"));
    assert!(ir.contains("call void @yukino_rt_get_raw_args"));
    assert!(ir.contains("call void @exit(i32 0)"));
    assert!(!ir.contains("declare void @args"));
}

#[test]
fn test_expr_switch_interpolation_sus() {
    use yukic_ast::expr::{
        Expr, ExprKind, LiteralValue, Pattern, PatternKind, StringPart, SwitchArm,
    };
    use yukic_ast::stmt::{Block, Stmt, StmtKind};
    use yukic_span::{BytePos, FileId, Span};

    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, "test_expr_mod", TargetTriple::host());
    let dummy_span = Span::new(FileId(0), BytePos(0), BytePos(0));

    let fn_ty = context.i64_type().fn_type(&[], false);
    let fn_val = codegen.module.add_function("test_fn", fn_ty, None);
    let entry = context.append_basic_block(fn_val, "entry");
    codegen.builder.position_at_end(entry);

    let reg = codegen
        .emit_region_prologue(4096, "_local_reg")
        .expect("region prologue");

    let mut body = yukic_codegen::body::BodyCompiler::new(
        &context,
        &codegen.builder,
        &codegen.module,
        &mut codegen.rodata,
        &codegen.intrinsic,
        &codegen.region,
        &codegen.vtable,
        &codegen.cabi,
        &codegen.registry,
        codegen.target,
        Some(reg),
        None,
        fn_val,
        Some(context.i64_type().into()),
        false,
    );

    // 1. Interpolated string
    let interp = Expr::new(
        ExprKind::InterpolatedString(vec![
            StringPart::Lit("Number: ".to_string()),
            StringPart::Expr(Expr::new(ExprKind::Literal(LiteralValue::Int(42)), dummy_span)),
        ]),
        dummy_span,
    );
    let s_val = body.compile_expr(&interp).expect("interpolated string");
    assert!(s_val.is_struct_value());

    // 2. Switch expression
    let sw = Expr::new(
        ExprKind::Switch {
            target: Box::new(Expr::new(ExprKind::Literal(LiteralValue::Char('+')), dummy_span)),
            arms: vec![
                SwitchArm {
                    pattern: Pattern::new(PatternKind::Literal(LiteralValue::Char('+')), dummy_span),
                    guard: None,
                    body: Block::new(
                        vec![Stmt::new(
                            StmtKind::Expr(Expr::new(ExprKind::Literal(LiteralValue::Int(100)), dummy_span)),
                            dummy_span,
                        )],
                        dummy_span,
                    ),
                    span: dummy_span,
                },
                SwitchArm {
                    pattern: Pattern::new(PatternKind::Wildcard, dummy_span),
                    guard: None,
                    body: Block::new(
                        vec![Stmt::new(
                            StmtKind::Expr(Expr::new(ExprKind::Literal(LiteralValue::Int(0)), dummy_span)),
                            dummy_span,
                        )],
                        dummy_span,
                    ),
                    span: dummy_span,
                },
            ],
        },
        dummy_span,
    );
    let sw_val = body.compile_expr(&sw).expect("switch expr");
    assert!(sw_val.is_int_value());

    // 3. Sus expression
    let sus = Expr::new(
        ExprKind::Sus {
            body: Block::new(
                vec![Stmt::new(
                    StmtKind::Expr(Expr::new(ExprKind::Literal(LiteralValue::Int(777)), dummy_span)),
                    dummy_span,
                )],
                dummy_span,
            ),
            catches: vec![],
            finally_branch: None,
        },
        dummy_span,
    );
    let sus_val = body.compile_expr(&sus).expect("sus expr");
    assert!(sus_val.is_int_value());

    let ir = codegen.module.print_to_string().to_string();
    assert!(ir.contains("call void @yukino_rt_int64_to_string"));
    assert!(ir.contains("sw.arm_0"));
}

#[test]
fn test_variadic_function_codegen() {
    use yukic_ast::expr::{Expr, ExprKind, LiteralValue};
    use yukic_ast::ident::Ident;
    use yukic_span::{BytePos, FileId, Span};

    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, "test_variadic_mod", TargetTriple::host());
    let dummy_span = Span::new(FileId(0), BytePos(0), BytePos(0));

    // Define variadic target: fun sum(numbers: i32[]): i32
    let slice_ty = codegen.slice_type();
    let sum_fn_ty = context.i32_type().fn_type(&[slice_ty.into()], false);
    codegen.module.add_function("sum", sum_fn_ty, None);

    // Caller
    let caller_fn_ty = context.i32_type().fn_type(&[], false);
    let caller_fn = codegen.module.add_function("caller", caller_fn_ty, None);
    let entry = context.append_basic_block(caller_fn, "entry");
    codegen.builder.position_at_end(entry);

    let reg = codegen
        .emit_region_prologue(4096, "_local_reg")
        .expect("region prologue");

    let mut body = yukic_codegen::body::BodyCompiler::new(
        &context,
        &codegen.builder,
        &codegen.module,
        &mut codegen.rodata,
        &codegen.intrinsic,
        &codegen.region,
        &codegen.vtable,
        &codegen.cabi,
        &codegen.registry,
        codegen.target,
        Some(reg),
        None,
        caller_fn,
        Some(context.i32_type().into()),
        false,
    );

    // ArrayInit: [10, 20, 30]
    let arr_init = Expr::new(
        ExprKind::ArrayInit(vec![
            Expr::new(ExprKind::Literal(LiteralValue::Int(10)), dummy_span),
            Expr::new(ExprKind::Literal(LiteralValue::Int(20)), dummy_span),
            Expr::new(ExprKind::Literal(LiteralValue::Int(30)), dummy_span),
        ]),
        dummy_span,
    );

    let call_expr = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(Ident::new("sum", dummy_span)), dummy_span)),
            vec![arr_init],
        ),
        dummy_span,
    );

    let res = body.compile_expr(&call_expr).expect("compile call");
    assert!(res.is_int_value());

    let ir = codegen.module.print_to_string().to_string();
    assert!(ir.contains("call i32 @sum("));
}

#[test]
fn test_ref_parameter_and_call_codegen() {
    use yukic_ast::Module as AstModule;
    use yukic_ast::expr::{BinaryOp, Expr, ExprKind, LiteralValue};
    use yukic_ast::ident::Ident;
    use yukic_ast::item::{FunDecl, Item, ItemKind, Param};
    use yukic_ast::stmt::{AssignOp, Block, LValue, Stmt, StmtKind};
    use yukic_ast::types::{TypeNode, TypeNodeKind};
    use yukic_span::{BytePos, FileId, Span};

    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, "test_ref_mod", TargetTriple::host());
    let dummy_span = Span::new(FileId(0), BytePos(0), BytePos(0));

    // 1. fun inc(ref ~x: int32) { rewrite x = x + 1 }
    let i32_node = TypeNode::new(TypeNodeKind::Ident(Ident::new("int32", dummy_span)), dummy_span);
    let inc_decl = FunDecl {
        name: Ident::new("inc", dummy_span),
        attrs: vec![],
        generics: vec![],
        params: vec![Param {
            name: Ident::new("x", dummy_span),
            ty: Some(i32_node.clone()),
            default_val: None,
            is_ref: true,
            is_mut: true,
            is_variadic: false,
            span: dummy_span,
        }],
        return_type: None,
        body: Block::new(
            vec![Stmt::new(
                StmtKind::Rewrite {
                    target: LValue::Ident(Ident::new("x", dummy_span)),
                    op: AssignOp::Eq,
                    value: Expr::new(
                        ExprKind::Binary(
                            BinaryOp::Add,
                            Box::new(Expr::new(ExprKind::Ident(Ident::new("x", dummy_span)), dummy_span)),
                            Box::new(Expr::new(ExprKind::Literal(LiteralValue::Int(1)), dummy_span)),
                        ),
                        dummy_span,
                    ),
                },
                dummy_span,
            )],
            dummy_span,
        ),
        span: dummy_span,
    };

    let mut ast_mod = AstModule::new(FileId(0));
    ast_mod.items.push(Item::new(
        yukic_ast::item::ItemId(0),
        yukic_ast::item::Visibility::Local,
        ItemKind::Fun(inc_decl),
        dummy_span,
    ));
    codegen.compile_module(&ast_mod).expect("compile inc module");

    // 2. Caller: fun test_caller() { set a = 10; inc(ref ~a); }
    let caller_fn_ty = context.void_type().fn_type(&[], false);
    let caller_fn = codegen.module.add_function("test_caller", caller_fn_ty, None);
    let entry = context.append_basic_block(caller_fn, "entry");
    codegen.builder.position_at_end(entry);

    let reg = codegen
        .emit_region_prologue(4096, "_local_reg")
        .expect("region prologue");

    let mut body = yukic_codegen::body::BodyCompiler::new(
        &context,
        &codegen.builder,
        &codegen.module,
        &mut codegen.rodata,
        &codegen.intrinsic,
        &codegen.region,
        &codegen.vtable,
        &codegen.cabi,
        &codegen.registry,
        codegen.target,
        Some(reg),
        None,
        caller_fn,
        None,
        false,
    );

    // set a = 10 (local var)
    let a_alloca = codegen.builder.build_alloca(context.i32_type(), "a").unwrap();
    codegen.builder.build_store(a_alloca, context.i32_type().const_int(10, false)).unwrap();
    body.add_local_with_ty("a".to_string(), a_alloca, context.i32_type().into(), Some(yukic_types::Type::Int32));

    // Call inc(ref ~a)
    let ref_a = Expr::new(
        ExprKind::Ref {
            mutable: true,
            expr: Box::new(Expr::new(ExprKind::Ident(Ident::new("a", dummy_span)), dummy_span)),
        },
        dummy_span,
    );
    let call_expr = Expr::new(
        ExprKind::Call(
            Box::new(Expr::new(ExprKind::Ident(Ident::new("inc", dummy_span)), dummy_span)),
            vec![ref_a],
        ),
        dummy_span,
    );

    body.compile_expr(&call_expr).expect("compile inc(ref ~a)");

    let ir = codegen.module.print_to_string().to_string();
    assert!(ir.contains("define fastcc void @__yukifun_inc_mutref_int32__void(ptr %0)"));
    assert!(ir.contains("call fastcc void @__yukifun_inc_mutref_int32__void(ptr %a)"));
}

#[test]
fn test_class_method_ref_param_signature_codegen() {
    use yukic_ast::Module as AstModule;
    use yukic_ast::expr::{Expr, ExprKind};
    use yukic_ast::ident::Ident;
    use yukic_ast::item::{ClassDecl, ClassField, Item, ItemKind, MethodDecl, Param, ThisParam};
    use yukic_ast::stmt::{AssignOp, Block, LValue, Stmt, StmtKind};
    use yukic_ast::types::{TypeNode, TypeNodeKind};
    use yukic_span::{BytePos, FileId, Span};

    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, "test_class_ref_mod", TargetTriple::host());
    let dummy_span = Span::new(FileId(0), BytePos(0), BytePos(0));

    let i32_node = TypeNode::new(TypeNodeKind::Ident(Ident::new("int32", dummy_span)), dummy_span);

    // class Worker { total: int32; fun bump(ref ~x: int32) { rewrite this.total = x } }
    let class_decl = ClassDecl {
        name: Ident::new("Worker", dummy_span),
        is_locked: false,
        base_class: None,
        interfaces: vec![],
        fields: vec![ClassField {
            name: Ident::new("total", dummy_span),
            attrs: vec![],
            ty: i32_node.clone(),
            is_pub: false,
            default_val: None,
            span: dummy_span,
        }],
        methods: vec![MethodDecl {
            name: Ident::new("bump", dummy_span),
            attrs: vec![],
            this_param: ThisParam::Value,
            params: vec![Param {
                name: Ident::new("x", dummy_span),
                ty: Some(i32_node.clone()),
                default_val: None,
                is_ref: true,
                is_mut: true,
                is_variadic: false,
                span: dummy_span,
            }],
            return_type: None,
            body: Block::new(
                vec![Stmt::new(
                    StmtKind::Rewrite {
                        target: LValue::Field(
                            Box::new(Expr::new(
                                ExprKind::Ident(Ident::new("this", dummy_span)),
                                dummy_span,
                            )),
                            Ident::new("total", dummy_span),
                        ),
                        op: AssignOp::Eq,
                        value: Expr::new(ExprKind::Ident(Ident::new("x", dummy_span)), dummy_span),
                    },
                    dummy_span,
                )],
                dummy_span,
            ),
            is_operator: false,
            is_override: false,
            span: dummy_span,
        }],
        accessors: vec![],
        span: dummy_span,
    };

    let mut ast_mod = AstModule::new(FileId(0));
    ast_mod.items.push(Item::new(
        yukic_ast::item::ItemId(0),
        yukic_ast::item::Visibility::Local,
        ItemKind::Class(class_decl),
        dummy_span,
    ));
    codegen.compile_module(&ast_mod).expect("compile class module");

    // pre-declare and compile signatures must agree: ref param -> ptr
    let ir = codegen.module.print_to_string().to_string();
    assert!(
        ir.contains("define fastcc void @_Y_Worker_bump(ptr %0, ptr %1)"),
        "ir: {ir}"
    );
}

#[test]
fn test_named_args_call_codegen() {
    use yukic_ast::Module as AstModule;
    use yukic_ast::defaults::expand_default_args;
    use yukic_ast::expr::{BinaryOp, Expr, ExprKind, LiteralValue};
    use yukic_ast::ident::Ident;
    use yukic_ast::item::{FunDecl, Item, ItemKind, Param};
    use yukic_ast::stmt::{Block, Stmt, StmtKind};
    use yukic_ast::types::{TypeNode, TypeNodeKind};
    use yukic_span::{BytePos, FileId, Span};

    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, "test_named_args_mod", TargetTriple::host());
    let dummy_span = Span::new(FileId(0), BytePos(0), BytePos(0));

    let i32_node = TypeNode::new(TypeNodeKind::Ident(Ident::new("int32", dummy_span)), dummy_span);
    let sub_decl = FunDecl {
        name: Ident::new("sub", dummy_span),
        attrs: vec![],
        generics: vec![],
        params: vec![
            Param {
                name: Ident::new("a", dummy_span),
                ty: Some(i32_node.clone()),
                default_val: None,
                is_ref: false,
                is_mut: false,
                is_variadic: false,
                span: dummy_span,
            },
            Param {
                name: Ident::new("b", dummy_span),
                ty: Some(i32_node.clone()),
                default_val: None,
                is_ref: false,
                is_mut: false,
                is_variadic: false,
                span: dummy_span,
            },
        ],
        return_type: Some(i32_node.clone()),
        body: Block::new(
            vec![Stmt::new(
                StmtKind::Return(Some(Expr::new(
                    ExprKind::Binary(
                        BinaryOp::Sub,
                        Box::new(Expr::new(ExprKind::Ident(Ident::new("a", dummy_span)), dummy_span)),
                        Box::new(Expr::new(ExprKind::Ident(Ident::new("b", dummy_span)), dummy_span)),
                    ),
                    dummy_span,
                ))),
                dummy_span,
            )],
            dummy_span,
        ),
        span: dummy_span,
    };

    let caller_decl = FunDecl {
        name: Ident::new("caller", dummy_span),
        attrs: vec![],
        generics: vec![],
        params: vec![],
        return_type: Some(i32_node),
        body: Block::new(
            vec![Stmt::new(
                StmtKind::Return(Some(Expr::new(
                    ExprKind::Call(
                        Box::new(Expr::new(ExprKind::Ident(Ident::new("sub", dummy_span)), dummy_span)),
                        vec![
                            Expr::new(
                                ExprKind::NamedArg(
                                    Ident::new("b", dummy_span),
                                    Box::new(Expr::new(ExprKind::Literal(LiteralValue::Int(20)), dummy_span)),
                                ),
                                dummy_span,
                            ),
                            Expr::new(
                                ExprKind::NamedArg(
                                    Ident::new("a", dummy_span),
                                    Box::new(Expr::new(ExprKind::Literal(LiteralValue::Int(100)), dummy_span)),
                                ),
                                dummy_span,
                            ),
                        ],
                    ),
                    dummy_span,
                ))),
                dummy_span,
            )],
            dummy_span,
        ),
        span: dummy_span,
    };

    let mut ast_mod = AstModule::new(FileId(0));
    ast_mod.items.push(Item::new(
        yukic_ast::item::ItemId(0),
        yukic_ast::item::Visibility::Local,
        ItemKind::Fun(sub_decl),
        dummy_span,
    ));
    ast_mod.items.push(Item::new(
        yukic_ast::item::ItemId(1),
        yukic_ast::item::Visibility::Local,
        ItemKind::Fun(caller_decl),
        dummy_span,
    ));

    expand_default_args(&mut ast_mod, &[]);
    codegen.compile_module(&ast_mod).expect("compile module");

    let ir = codegen.module.print_to_string().to_string();
    assert!(ir.contains("call fastcc i32 @__yukifun_sub_int32_int32__int32(i32 100, i32 20)"));
}

#[test]
fn test_string_and_array_cp_codegen() {
    use yukic_ast::Module as AstModule;
    use yukic_ast::expr::{Expr, ExprKind, LiteralValue};
    use yukic_ast::ident::Ident;
    use yukic_ast::item::{FunDecl, Item, ItemKind};
    use yukic_ast::stmt::{Block, Stmt, StmtKind, BindingTarget};
    use yukic_span::{BytePos, FileId, Span};

    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, "test_cp_mod", TargetTriple::host());
    let dummy_span = Span::new(FileId(0), BytePos(0), BytePos(0));

    let s_ident = Ident::new("s", dummy_span);
    let sc_ident = Ident::new("sc", dummy_span);
    let arr_ident = Ident::new("arr", dummy_span);
    let arrc_ident = Ident::new("arrc", dummy_span);

    let test_fn = FunDecl {
        name: Ident::new("test_cp", dummy_span),
        attrs: vec![],
        generics: vec![],
        params: vec![],
        return_type: None,
        body: Block::new(
            vec![
                Stmt::new(
                    StmtKind::Set {
                        target: BindingTarget::Ident(s_ident.clone()),
                        ty: None,
                        init: Expr::new(ExprKind::Literal(LiteralValue::String("hello".into())), dummy_span),
                    },
                    dummy_span,
                ),
                Stmt::new(
                    StmtKind::Set {
                        target: BindingTarget::Ident(sc_ident),
                        ty: None,
                        init: Expr::new(
                            ExprKind::MethodCall(
                                Box::new(Expr::new(ExprKind::Ident(s_ident), dummy_span)),
                                Ident::new("cp", dummy_span),
                                vec![],
                            ),
                            dummy_span,
                        ),
                    },
                    dummy_span,
                ),
                Stmt::new(
                    StmtKind::Set {
                        target: BindingTarget::Ident(arr_ident.clone()),
                        ty: None,
                        init: Expr::new(
                            ExprKind::ArrayInit(vec![
                                Expr::new(ExprKind::Literal(LiteralValue::Int(1)), dummy_span),
                                Expr::new(ExprKind::Literal(LiteralValue::Int(2)), dummy_span),
                                Expr::new(ExprKind::Literal(LiteralValue::Int(3)), dummy_span),
                            ]),
                            dummy_span,
                        ),
                    },
                    dummy_span,
                ),
                Stmt::new(
                    StmtKind::Set {
                        target: BindingTarget::Ident(arrc_ident),
                        ty: None,
                        init: Expr::new(
                            ExprKind::MethodCall(
                                Box::new(Expr::new(ExprKind::Ident(arr_ident), dummy_span)),
                                Ident::new("cp", dummy_span),
                                vec![],
                            ),
                            dummy_span,
                        ),
                    },
                    dummy_span,
                ),
            ],
            dummy_span,
        ),
        span: dummy_span,
    };

    let mut ast_mod = AstModule::new(FileId(0));
    ast_mod.items.push(Item::new(
        yukic_ast::item::ItemId(0),
        yukic_ast::item::Visibility::Local,
        ItemKind::Fun(test_fn),
        dummy_span,
    ));

    codegen.compile_module(&ast_mod).expect("compile module");
    let ir = codegen.module.print_to_string().to_string();
    assert!(ir.contains("define fastcc void @__yukifun_test_cp___void()"));
    assert!(ir.contains("call void @llvm.memcpy"));
}

#[test]
fn test_region_block_codegen() {
    use yukic_ast::Module as AstModule;
    use yukic_ast::expr::{BinaryOp, Expr, ExprKind, LiteralValue};
    use yukic_ast::ident::Ident;
    use yukic_ast::item::{FunDecl, Item, ItemKind};
    use yukic_ast::stmt::{Block, Stmt, StmtKind, BindingTarget};
    use yukic_span::{BytePos, FileId, Span};

    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, "test_region_mod", TargetTriple::host());
    let dummy_span = Span::new(FileId(0), BytePos(0), BytePos(0));

    let res_ident = Ident::new("res", dummy_span);
    let a_ident = Ident::new("a", dummy_span);
    let b_ident = Ident::new("b", dummy_span);

    let region_inner_block = Block::new(
        vec![
            Stmt::new(
                StmtKind::Set {
                    target: BindingTarget::Ident(a_ident.clone()),
                    ty: None,
                    init: Expr::new(ExprKind::Literal(LiteralValue::Int(10)), dummy_span),
                },
                dummy_span,
            ),
            Stmt::new(
                StmtKind::Set {
                    target: BindingTarget::Ident(b_ident.clone()),
                    ty: None,
                    init: Expr::new(ExprKind::Literal(LiteralValue::Int(20)), dummy_span),
                },
                dummy_span,
            ),
            Stmt::new(
                StmtKind::Expr(Expr::new(
                    ExprKind::Binary(
                        BinaryOp::Add,
                        Box::new(Expr::new(ExprKind::Ident(a_ident), dummy_span)),
                        Box::new(Expr::new(ExprKind::Ident(b_ident), dummy_span)),
                    ),
                    dummy_span,
                )),
                dummy_span,
            ),
        ],
        dummy_span,
    );

    let test_fn = FunDecl {
        name: Ident::new("test_region", dummy_span),
        attrs: vec![],
        generics: vec![],
        params: vec![],
        return_type: None,
        body: Block::new(
            vec![
                Stmt::new(
                    StmtKind::Set {
                        target: BindingTarget::Ident(res_ident),
                        ty: None,
                        init: Expr::new(ExprKind::RegionBlock(region_inner_block), dummy_span),
                    },
                    dummy_span,
                ),
            ],
            dummy_span,
        ),
        span: dummy_span,
    };

    let mut ast_mod = AstModule::new(FileId(0));
    ast_mod.items.push(Item::new(
        yukic_ast::item::ItemId(0),
        yukic_ast::item::Visibility::Local,
        ItemKind::Fun(test_fn),
        dummy_span,
    ));

    codegen.compile_module(&ast_mod).expect("compile module");
    let ir = codegen.module.print_to_string().to_string();
    assert!(ir.contains("define fastcc void @__yukifun_test_region___void()"));
    assert!(ir.contains("call ptr @yukino_region_save_mark(ptr %_local_region)"));
    assert!(ir.contains("call void @yukino_region_restore_mark(ptr %_local_region, ptr %loop_mark)"));
}

fn make_pow_test_fn(span: yukic_span::Span) -> yukic_ast::item::FunDecl {
    use yukic_ast::expr::{BinaryOp, Expr, ExprKind, LiteralValue};
    use yukic_ast::ident::Ident;
    use yukic_ast::stmt::{AssignOp, Block, BindingTarget, LValue, Stmt, StmtKind};

    let x = Ident::new("x", span);
    let f = Ident::new("f", span);
    let int_pow = Expr::new(
        ExprKind::Binary(
            BinaryOp::Pow,
            Box::new(Expr::new(ExprKind::Literal(LiteralValue::Int(2)), span)),
            Box::new(Expr::new(ExprKind::Literal(LiteralValue::Int(10)), span)),
        ),
        span,
    );
    let flt_pow = Expr::new(
        ExprKind::Binary(
            BinaryOp::Pow,
            Box::new(Expr::new(ExprKind::Literal(LiteralValue::Float(2.5)), span)),
            Box::new(Expr::new(ExprKind::Literal(LiteralValue::Float(3.0)), span)),
        ),
        span,
    );

    let stmts = vec![
        Stmt::new(StmtKind::Set { target: BindingTarget::Ident(x.clone()), ty: None, init: int_pow }, span),
        Stmt::new(StmtKind::Rewrite {
            op: AssignOp::StarStarEq,
            target: LValue::Ident(x),
            value: Expr::new(ExprKind::Literal(LiteralValue::Int(3)), span),
        }, span),
        Stmt::new(StmtKind::Set { target: BindingTarget::Ident(f), ty: None, init: flt_pow }, span),
    ];

    yukic_ast::item::FunDecl {
        name: Ident::new("test_pow", span),
        attrs: vec![],
        generics: vec![],
        params: vec![],
        return_type: None,
        body: Block::new(stmts, span),
        span,
    }
}

#[test]
fn test_pow_operator_codegen() {
    use yukic_ast::Module as AstModule;
    use yukic_ast::item::{Item, ItemKind};
    use yukic_span::{BytePos, FileId, Span};

    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, "test_pow_mod", TargetTriple::host());
    let dummy_span = Span::new(FileId(0), BytePos(0), BytePos(0));

    let mut ast_mod = AstModule::new(FileId(0));
    ast_mod.items.push(Item::new(
        yukic_ast::item::ItemId(0),
        yukic_ast::item::Visibility::Local,
        ItemKind::Fun(make_pow_test_fn(dummy_span)),
        dummy_span,
    ));

    codegen.compile_module(&ast_mod).expect("compile module");
    let ir = codegen.module.print_to_string().to_string();
    assert!(ir.contains("call i64 @_Y_ipow64("));
    assert!(ir.contains("call double @llvm.pow.f64("));
}

