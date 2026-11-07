#![deny(unsafe_code)]

use std::collections::HashSet;
use yukic_ast::{Item, ItemId, ItemKind, Module, Visibility};

use crate::config::{DceConfig, OutputType};

pub fn prune_tests(module: &mut Module) -> usize {
    let original_len = module.items.len();
    module.items.retain(|item| !is_test_item(item));
    original_len - module.items.len()
}

pub fn is_test_item(item: &Item) -> bool {
    if let ItemKind::Fun(decl) = &item.kind {
        decl.attrs.iter().any(|attr| attr.is("test"))
    } 

    else { false }
}

pub fn discover_roots(module: &Module, config: &DceConfig) -> HashSet<ItemId> {
    let mut roots = HashSet::new();

    for item in &module.items {
        if is_root_item(item, config) {
            roots.insert(item.id);
        }
    }

    roots
}

fn is_root_item(item: &Item, config: &DceConfig) -> bool {
    if config.is_test && is_test_item(item) { return true; }

    match config.output_type {
        OutputType::App => is_app_root(item, config.is_test),
        OutputType::StaticLib | OutputType::SharedLib => is_lib_root(item),
    }
}

fn is_app_root(item: &Item, is_test: bool) -> bool {
    match &item.kind {
        ItemKind::Fun(decl) => {
            let name = decl.name.as_str();
            name == "main" || (is_test && name == "__yukino_test_main")
        }
        ItemKind::Cfun(_) | ItemKind::Externc(_) => true,
        _ => false,
    }
}

fn is_lib_root(item: &Item) -> bool {
    if item.visibility == Visibility::Public { return true; }

    matches!(
        &item.kind,
        ItemKind::Cfun(_) | ItemKind::Cstruct(_) | ItemKind::Externc(_)
    )
}
