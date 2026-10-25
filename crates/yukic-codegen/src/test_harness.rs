#![deny(unsafe_code)]
#![allow(
    clippy::cast_possible_wrap,
    clippy::too_many_lines,
    clippy::module_name_repetitions
)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use inkwell::AddressSpace;
use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::values::{BasicValueEnum, PointerValue};
use yukic_ast::attr::{AttributeArgs, AttributeNode};
use yukic_ast::expr::{ExprKind, LiteralValue};
use yukic_ast::item::{FunDecl, ItemKind, Module as AstModule};
use yukic_target::TargetTriple;

use crate::cgu::CompiledCgu;
use crate::registry::TypeRegistry;

#[derive(Clone, Debug)]
pub struct TestFunctionInfo {
    pub display_name: String,
    pub symbol_name: String,
    pub should_panic: bool,
    pub expected_panic: Option<String>,
    pub ignore: bool,
    pub ignore_reason: Option<String>,
    pub timeout_ms: u64,
}

fn extract_attr_string(attr: &AttributeNode) -> Option<String> {
    match &attr.args {
        AttributeArgs::Positional(values) if !values.is_empty() => {
            if let ExprKind::Literal(LiteralValue::String(s)) = &values[0].kind {
                Some(s.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn extract_attr_u64(attr: &AttributeNode) -> Option<u64> {
    match &attr.args {
        AttributeArgs::Positional(values) if !values.is_empty() => match &values[0].kind {
            ExprKind::Literal(LiteralValue::Int(n)) if *n > 0 => Some((*n).cast_unsigned()),
            ExprKind::Literal(LiteralValue::UInt(n)) => Some(*n),
            _ => None,
        },
        _ => None,
    }
}

pub fn collect_test_functions(
    ast: &AstModule,
    item_sources: &HashMap<yukic_ast::ItemId, PathBuf>,
    registry: &TypeRegistry,
    project_root: Option<&Path>,
) -> Vec<TestFunctionInfo> {
    let mut tests = Vec::new();

    for item in &ast.items {
        if let ItemKind::Fun(fun) = &item.kind {
            if !fun.attrs.iter().any(|a| a.is("test")) {
                continue;
            }

            let display_name = derive_test_display_name(fun, item.id, item_sources, project_root);
            let symbol_name = crate::mangle::mangle_fun_decl(fun, registry);

            let should_panic_attr = fun.attrs.iter().find(|a| a.is("should_panic"));
            let should_panic = should_panic_attr.is_some();
            let expected_panic = should_panic_attr.and_then(extract_attr_string);

            let ignore_attr = fun.attrs.iter().find(|a| a.is("ignore"));
            let ignore = ignore_attr.is_some();
            let ignore_reason = ignore_attr.and_then(extract_attr_string);

            let timeout_attr = fun.attrs.iter().find(|a| a.is("timeout_ms"));
            let timeout_ms = timeout_attr.and_then(extract_attr_u64).unwrap_or(0);

            tests.push(TestFunctionInfo {
                display_name,
                symbol_name,
                should_panic,
                expected_panic,
                ignore,
                ignore_reason,
                timeout_ms,
            });
        }
    }

    tests
}

fn derive_test_display_name(
    fun: &FunDecl,
    item_id: yukic_ast::ItemId,
    item_sources: &HashMap<yukic_ast::ItemId, PathBuf>,
    project_root: Option<&Path>,
) -> String {
    let base_name = fun.name.name.as_str();
    let Some(src_path) = item_sources.get(&item_id) else {
        return base_name.to_string();
    };

    let suite = if let Some(root) = project_root {
        let rel = src_path.strip_prefix(root).unwrap_or(src_path);
        let stem = rel.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let parent = rel.parent().and_then(|p| p.to_str()).unwrap_or("");
        if parent.is_empty() || parent == "src" {
            stem.to_string()
        } else {
            let sanitized = parent.trim_start_matches("src/").replace('/', "::");
            if sanitized.is_empty() {
                stem.to_string()
            } else {
                format!("{sanitized}::{stem}")
            }
        }
    } else {
        src_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string()
    };

    if suite.is_empty() {
        format!("main::{base_name}")
    } else {
        format!("{suite}::{base_name}")
    }
}

pub fn generate_test_harness_cgu(
    tests: &[TestFunctionInfo],
    default_timeout_ms: u64,
    _target: TargetTriple,
) -> Result<CompiledCgu, String> {
    let context = Context::create();
    let module = context.create_module("cgu_test_harness");
    let builder = context.create_builder();

    let i32_type = context.i32_type();
    let i64_type = context.i64_type();
    let i1_type = context.bool_type();
    let ptr_type = context.ptr_type(AddressSpace::default());
    let void_type = context.void_type();

    let desc_type = context.struct_type(
        &[
            ptr_type.into(), // name_ptr
            i64_type.into(), // name_len
            ptr_type.into(), // test_fn
            i1_type.into(),  // should_panic
            ptr_type.into(), // expected_panic_ptr
            i64_type.into(), // expected_panic_len
            i1_type.into(),  // ignore
            ptr_type.into(), // ignore_reason_ptr
            i64_type.into(), // ignore_reason_len
            i64_type.into(), // timeout_ms
        ],
        false,
    );

    let mut desc_values: Vec<BasicValueEnum> = Vec::with_capacity(tests.len());

    for (idx, test) in tests.iter().enumerate() {
        let target_fn_type = void_type.fn_type(&[], false);
        let target_fn = module
            .get_function(&test.symbol_name)
            .unwrap_or_else(|| module.add_function(&test.symbol_name, target_fn_type, None));
        target_fn.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMFastCallConv as u32);

        let wrap_name = format!("__yukitest_wrap_{idx}");
        let wrap_fn_type = void_type.fn_type(&[], false);
        let wrap_fn = module.add_function(&wrap_name, wrap_fn_type, None);
        wrap_fn.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32);

        let entry = context.append_basic_block(wrap_fn, "entry");
        builder.position_at_end(entry);
        builder
            .build_call(target_fn, &[], "")
            .map_err(|e| e.to_string())?;
        builder.build_return(None).map_err(|e| e.to_string())?;

        let (name_ptr, name_len) =
            emit_static_str(&module, &context, &test.display_name, &format!("__tname_{idx}"));

        let (exp_panic_ptr, exp_panic_len) = if let Some(ref p) = test.expected_panic {
            emit_static_str(&module, &context, p, &format!("__tpanic_{idx}"))
        } else {
            (ptr_type.const_null(), i64_type.const_zero())
        };

        let (ign_reason_ptr, ign_reason_len) = if let Some(ref r) = test.ignore_reason {
            emit_static_str(&module, &context, r, &format!("__tignore_{idx}"))
        } else {
            (ptr_type.const_null(), i64_type.const_zero())
        };

        let wrap_fn_ptr = wrap_fn.as_global_value().as_pointer_value();

        let desc_val = context.const_struct(
            &[
                name_ptr.into(),
                name_len.into(),
                wrap_fn_ptr.into(),
                i1_type
                    .const_int(u64::from(test.should_panic), false)
                    .into(),
                exp_panic_ptr.into(),
                exp_panic_len.into(),
                i1_type
                    .const_int(u64::from(test.ignore), false)
                    .into(),
                ign_reason_ptr.into(),
                ign_reason_len.into(),
                i64_type.const_int(test.timeout_ms, false).into(),
            ],
            false,
        );
        desc_values.push(desc_val.into());
    }

    let descs_ptr = if desc_values.is_empty() {
        ptr_type.const_null()
    } else {
        let descs_array_type = desc_type.array_type(desc_values.len() as u32);
        let struct_values: Vec<inkwell::values::StructValue> = desc_values
            .iter()
            .map(|v| v.into_struct_value())
            .collect();
        let descs_array_val = desc_type.const_array(&struct_values);
        let descs_global = module.add_global(
            descs_array_type,
            Some(AddressSpace::default()),
            "__yukino_test_descs",
        );
        descs_global.set_initializer(&descs_array_val);
        descs_global.set_constant(true);
        descs_global.set_linkage(Linkage::Internal);
        descs_global.as_pointer_value()
    };

    let run_tests_fn_type = i32_type.fn_type(
        &[
            i64_type.into(), // count
            ptr_type.into(), // tests array ptr
            i32_type.into(), // argc
            ptr_type.into(), // argv
            i64_type.into(), // default_timeout_ms
        ],
        false,
    );
    let run_tests_fn = module.add_function("yukino_run_tests", run_tests_fn_type, None);
    run_tests_fn.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32);

    let main_fn_type = i32_type.fn_type(&[i32_type.into(), ptr_type.into()], false);
    let main_fn = module.add_function("main", main_fn_type, None);
    main_fn.set_call_conventions(inkwell::llvm_sys::LLVMCallConv::LLVMCCallConv as u32);
    let main_entry = context.append_basic_block(main_fn, "entry");
    builder.position_at_end(main_entry);

    let argc_val = main_fn
        .get_first_param()
        .unwrap_or_else(|| i32_type.const_zero().into());
    let argv_val = main_fn
        .get_params()
        .get(1)
        .copied()
        .unwrap_or_else(|| ptr_type.const_null().into());

    let count_val = i64_type.const_int(tests.len() as u64, false);
    let timeout_val = i64_type.const_int(default_timeout_ms, false);

    let call_site = builder
        .build_call(
            run_tests_fn,
            &[
                count_val.into(),
                descs_ptr.into(),
                argc_val.into(),
                argv_val.into(),
                timeout_val.into(),
            ],
            "test_exit_code",
        )
        .map_err(|e| e.to_string())?;

    let exit_code = call_site
        .try_as_basic_value()
        .basic()
        .unwrap_or_else(|| i32_type.const_zero().into());
    builder
        .build_return(Some(&exit_code))
        .map_err(|e| e.to_string())?;

    module.verify().map_err(|e| e.to_string())?;
    let ll_ir = module.print_to_string().to_string();

    Ok(CompiledCgu {
        name: "cgu_test_harness".to_string(),
        ll_ir,
        item_count: tests.len().max(1),
        estimated_weight: tests.len().max(1) * 2,
    })
}

fn emit_static_str<'ctx>(
    module: &inkwell::module::Module<'ctx>,
    context: &'ctx Context,
    text: &str,
    global_name: &str,
) -> (PointerValue<'ctx>, inkwell::values::IntValue<'ctx>) {
    let const_str = context.const_string(text.as_bytes(), false);
    let global = module.add_global(
        const_str.get_type(),
        Some(AddressSpace::default()),
        global_name,
    );
    global.set_initializer(&const_str);
    global.set_constant(true);
    global.set_linkage(Linkage::Internal);
    let len_val = context.i64_type().const_int(text.len() as u64, false);
    (global.as_pointer_value(), len_val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_harness_empty_tests() {
        let res = generate_test_harness_cgu(&[], 5000, TargetTriple::host());
        assert!(res.is_ok());
        let cgu = res.unwrap();
        assert_eq!(cgu.name, "cgu_test_harness");
        assert!(cgu.ll_ir.contains("@yukino_run_tests"));
        assert!(cgu.ll_ir.contains("define i32 @main("));
    }

    #[test]
    fn test_generate_harness_with_tests() {
        let tests = vec![
            TestFunctionInfo {
                display_name: "math::test_add".to_string(),
                symbol_name: "__yukifun_test_add___void".to_string(),
                should_panic: false,
                expected_panic: None,
                ignore: false,
                ignore_reason: None,
                timeout_ms: 1000,
            },
            TestFunctionInfo {
                display_name: "math::test_div_zero".to_string(),
                symbol_name: "__yukifun_test_div_zero___void".to_string(),
                should_panic: true,
                expected_panic: Some("division by zero".to_string()),
                ignore: true,
                ignore_reason: Some("needs fix".to_string()),
                timeout_ms: 0,
            },
        ];

        let res = generate_test_harness_cgu(&tests, 5000, TargetTriple::host());
        assert!(res.is_ok());
        let cgu = res.unwrap();
        assert!(cgu.ll_ir.contains("@__yukitest_wrap_0"));
        assert!(cgu.ll_ir.contains("@__yukitest_wrap_1"));
        assert!(cgu.ll_ir.contains("@__yukino_test_descs"));
        assert!(cgu.ll_ir.contains("@yukino_run_tests"));
        assert!(cgu.ll_ir.contains("define i32 @main("));
    }
}
