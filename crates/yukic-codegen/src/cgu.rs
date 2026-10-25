#![deny(unsafe_code)]

use std::collections::HashMap;
use std::path::PathBuf;

use yukic_ast::{Item, ItemId, ItemKind, Module as AstModule};
use yukic_target::TargetTriple;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenUnit {
    pub name: String,
    pub item_ids: Vec<ItemId>,
    pub estimated_weight: usize,
}

impl CodegenUnit {
    pub fn new(name: String) -> Self {
        Self {
            name,
            item_ids: Vec::new(),
            estimated_weight: 0,
        }
    }
}

pub fn estimate_item_weight(item: &Item) -> usize {
    match &item.kind {
        ItemKind::Fun(f) => 1 + f.body.stmts.len(),
        ItemKind::Cfun(cf) => 1 + cf.body.as_ref().map_or(0, |b| b.stmts.len()),
        ItemKind::Class(c) => {
            let methods_weight: usize = c.methods.iter().map(|m| 1 + m.body.stmts.len()).sum();
            let accessors_weight: usize = c.accessors.iter().map(|a| 1 + a.body.stmts.len()).sum();
            1 + methods_weight + accessors_weight
        }
        ItemKind::Struct(s) => {
            let methods_weight: usize = s.methods.iter().map(|m| 1 + m.body.stmts.len()).sum();
            let statics_weight: usize = s
                .static_methods
                .iter()
                .map(|m| 1 + m.body.stmts.len())
                .sum();
            1 + methods_weight + statics_weight
        }
        ItemKind::Static(_) | ItemKind::Enum(_) | ItemKind::Cstruct(_) => 1,
        ItemKind::Externc(_)
        | ItemKind::Import(_)
        | ItemKind::Alias(_)
        | ItemKind::TypeAlias(_)
        | ItemKind::Interface(_)
        | ItemKind::Macro(_)
        | ItemKind::Error => 0,
    }
}

pub fn partition_module(ast: &AstModule, num_units: usize) -> Vec<CodegenUnit> {
    let num_units = num_units.max(1);
    if ast.items.is_empty() {
        return vec![CodegenUnit::new("cgu_0".to_string())];
    }

    let weighted_items: Vec<(ItemId, usize)> = ast
        .items
        .iter()
        .map(|item| (item.id, estimate_item_weight(item)))
        .collect();

    let has_positive_weight = weighted_items.iter().any(|(_, w)| *w > 0);
    if !has_positive_weight {
        let count = num_units.min(ast.items.len()).max(1);
        let mut units: Vec<CodegenUnit> = (0..count)
            .map(|i| CodegenUnit::new(format!("cgu_{i}")))
            .collect();
        for (i, item) in ast.items.iter().enumerate() {
            let idx = i % count;
            units[idx].item_ids.push(item.id);
            units[idx].estimated_weight += 1;
        }
        return units;
    }

    let mut code_items: Vec<(ItemId, usize)> = weighted_items
        .iter()
        .filter(|(_, w)| *w > 0)
        .copied()
        .collect();

    // sort
    code_items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.0.cmp(&b.0.0)));

    let actual_units = num_units.min(code_items.len()).max(1);
    let mut units: Vec<CodegenUnit> = (0..actual_units)
        .map(|i| CodegenUnit::new(format!("cgu_{i}")))
        .collect();

    // assign
    for (id, weight) in code_items {
        let min_idx = units
            .iter()
            .enumerate()
            .min_by_key(|(_, u)| u.estimated_weight)
            .map_or(0, |(idx, _)| idx);
        units[min_idx].item_ids.push(id);
        units[min_idx].estimated_weight += weight;
    }

    // assign remaining
    for (id, weight) in weighted_items {
        if weight == 0 {
            units[0].item_ids.push(id);
        }
    }

    units
}

fn path_matches_pattern(path_str: &str, pattern: &str) -> bool {
    let pat = pattern.replace('\\', "/");
    let pat = pat.trim();
    if pat.ends_with('*') {
        let prefix = pat.trim_end_matches('*');
        path_str.contains(prefix)
    } else if path_str == pat || path_str.ends_with(&format!("/{pat}")) || path_str.ends_with(pat) {
        true
    } else {
        path_str.contains(&format!("/{pat}/"))
            || path_str.starts_with(&format!("{pat}/"))
            || path_str.contains(&format!("/{pat}."))
            || path_str.starts_with(&format!("{pat}."))
            || path_str.contains(pat)
    }
}

pub fn partition_module_with_custom(
    ast: &AstModule,
    num_units: usize,
    custom_cgus: Option<&HashMap<String, Vec<String>>>,
    item_sources: Option<&HashMap<ItemId, PathBuf>>,
) -> Vec<CodegenUnit> {
    let Some(custom_map) = custom_cgus.filter(|m| !m.is_empty()) else {
        return partition_module(ast, num_units);
    };

    let Some(sources) = item_sources else {
        return partition_module(ast, num_units);
    };

    let mut cgu_names: Vec<&String> = custom_map.keys().collect();
    cgu_names.sort();

    let mut units: Vec<CodegenUnit> = cgu_names
        .iter()
        .map(|name| {
            let cgu_name = if name.starts_with("cgu_") {
                (*name).clone()
            } else {
                format!("cgu_{name}")
            };
            CodegenUnit::new(cgu_name)
        })
        .collect();

    let mut unassigned_items = Vec::new();

    for item in &ast.items {
        let weight = estimate_item_weight(item);
        let src_path = sources.get(&item.id);
        let mut matched = false;

        if let Some(src) = src_path {
            let path_str = src.to_string_lossy().replace('\\', "/");
            for (idx, name) in cgu_names.iter().enumerate() {
                if let Some(patterns) = custom_map.get(*name) {
                    if patterns.iter().any(|pat| path_matches_pattern(&path_str, pat)) {
                        units[idx].item_ids.push(item.id);
                        units[idx].estimated_weight += weight;
                        matched = true;
                        break;
                    }
                }
            }
        }

        if !matched {
            unassigned_items.push((item.id, weight));
        }
    }

    if !unassigned_items.is_empty() {
        let fallback_count = num_units.saturating_sub(units.len()).max(1);
        let mut fallback_units: Vec<CodegenUnit> = (0..fallback_count)
            .map(|i| CodegenUnit::new(format!("cgu_misc_{i}")))
            .collect();

        for (id, weight) in unassigned_items {
            let min_idx = fallback_units
                .iter()
                .enumerate()
                .min_by_key(|(_, u)| u.estimated_weight)
                .map_or(0, |(idx, _)| idx);
            fallback_units[min_idx].item_ids.push(id);
            fallback_units[min_idx].estimated_weight += weight;
        }

        units.extend(fallback_units);
    }

    units.retain(|u| !u.item_ids.is_empty());
    if units.is_empty() {
        units.push(CodegenUnit::new("cgu_0".to_string()));
    }

    units
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledCgu {
    pub name: String,
    pub ll_ir: String,
    pub item_count: usize,
    pub estimated_weight: usize,
}

pub fn compile_modules_parallel(
    ast: &AstModule,
    foreign_modules: &[AstModule],
    target: TargetTriple,
    num_units: usize,
    panic_strategy: &str,
) -> Result<Vec<CompiledCgu>, String> {
    let units = partition_module(ast, num_units);
    compile_units_parallel(ast, foreign_modules, target, units, panic_strategy, false)
}

pub fn compile_units_parallel(
    ast: &AstModule,
    foreign_modules: &[AstModule],
    target: TargetTriple,
    units: Vec<CodegenUnit>,
    panic_strategy: &str,
    function_sections: bool,
) -> Result<Vec<CompiledCgu>, String> {
    compile_units_parallel_with_overrides(
        ast,
        foreign_modules,
        target,
        units,
        panic_strategy,
        function_sections,
        &HashMap::new(),
    )
}

pub fn compile_units_parallel_with_overrides(
    ast: &AstModule,
    foreign_modules: &[AstModule],
    target: TargetTriple,
    units: Vec<CodegenUnit>,
    panic_strategy: &str,
    function_sections: bool,
    imported_overrides: &HashMap<String, String>,
) -> Result<Vec<CompiledCgu>, String> {
    std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(units.len());
        for unit in units {
            handles.push(s.spawn(move || {
                let context = inkwell::context::Context::create();
                let mut codegen =
                    crate::generator::CodeGenerator::new(&context, &unit.name, target)
                        .with_panic_strategy(panic_strategy)
                        .with_function_sections(function_sections);
                codegen.register_ast_types(ast);
                codegen.register_vtable_layouts(ast);
                for fmod in foreign_modules {
                    codegen.register_ast_types(fmod);
                    codegen.register_vtable_layouts(fmod);
                }
                codegen.registry.canonicalize_registered_types();
                for fmod in foreign_modules {
                    for item in &fmod.items {
                        if item.visibility == yukic_ast::item::Visibility::Public {
                            match &item.kind {
                                yukic_ast::ItemKind::Fun(fun) => codegen.declare_foreign_fun(fun)?,
                                yukic_ast::ItemKind::Cfun(cfun) => codegen.declare_foreign_cfun(cfun, true)?,
                                yukic_ast::ItemKind::Class(decl) => codegen.declare_foreign_class(decl)?,
                                yukic_ast::ItemKind::Externc(block) => codegen.compile_externc_block(block, true)?,
                                yukic_ast::ItemKind::Static(decl) => codegen.declare_foreign_static(decl),
                                _ => {}
                            }
                        }
                    }
                }
                for (k, v) in imported_overrides {
                    codegen.registry.symbol_overrides.insert(k.clone(), v.clone());
                }
                codegen.compile_cgu(ast, &unit)?;
                let ll_ir = codegen.emit_llvm_ir_to_string();
                Ok(CompiledCgu {
                    name: unit.name,
                    ll_ir,
                    item_count: unit.item_ids.len(),
                    estimated_weight: unit.estimated_weight,
                })
            }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            let res: Result<CompiledCgu, String> = handle
                .join()
                .map_err(|_| "worker thread panicked during CGU compilation".to_string())?;
            results.push(res?);
        }
        Ok(results)
    })
}
