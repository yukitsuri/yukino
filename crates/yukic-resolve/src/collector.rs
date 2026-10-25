#![deny(unsafe_code)]

use yukic_ast::{Item, ItemKind, Module, Visibility};
use yukic_span::Span;

use crate::module_id::ModuleId;
use crate::scope::{ModuleScope, ScopeBinding};
use crate::symbol::{SymbolId, SymbolKind, SymbolRegistry};

pub fn collect_module_symbols(
    module: &Module,
    module_id: ModuleId,
    registry: &mut SymbolRegistry,
) -> (ModuleScope, Vec<yukic_diagnostics::Diagnostic>) {
    let mut scope = ModuleScope::new(module_id);
    let mut diagnostics = Vec::new();

    for item in &module.items {
        for (name, kind, span) in extract_symbols(item) {
            let sym_id =
                registry.register(name.clone(), kind, item.visibility, module_id, span);
            let binding = ScopeBinding {
                symbol_id: sym_id,
                span,
                is_imported: false,
                is_point_import: false,
                pack_namespace: None,
                is_reexported: false,
            };
            if let Err(diag) = scope.insert_binding(name, binding) {
                diagnostics.push(diag);
            }
        }
    }

    (scope, diagnostics)
}

fn extract_symbols(item: &Item) -> Vec<(String, SymbolKind, Span)> {
    match &item.kind {
        ItemKind::Fun(decl) => vec![(decl.name.name.clone(), SymbolKind::Fun, item.span)],
        ItemKind::Cfun(decl) => vec![(decl.name.name.clone(), SymbolKind::Cfun, item.span)],
        ItemKind::Class(decl) => vec![(decl.name.name.clone(), SymbolKind::Class, item.span)],
        ItemKind::Struct(decl) => vec![(decl.name.name.clone(), SymbolKind::Struct, item.span)],
        ItemKind::Cstruct(decl) => vec![(decl.name.name.clone(), SymbolKind::Cstruct, item.span)],
        ItemKind::Interface(decl) => vec![(decl.name.name.clone(), SymbolKind::Interface, item.span)],
        ItemKind::Enum(decl) => vec![(decl.name.name.clone(), SymbolKind::Enum, item.span)],
        ItemKind::TypeAlias(decl) => vec![(decl.name.name.clone(), SymbolKind::TypeAlias, item.span)],
        ItemKind::Static(decl) => vec![(decl.name.name.clone(), SymbolKind::Static, item.span)],
        ItemKind::Macro(decl) => vec![(decl.name.name.clone(), SymbolKind::Macro, item.span)],
        ItemKind::Externc(block) => {
            let mut symbols = Vec::new();
            for ext_item in &block.items {
                match ext_item {
                    yukic_ast::ExterncItem::Cfun(decl) => {
                        symbols.push((decl.name.name.clone(), SymbolKind::Cfun, decl.span));
                    }
                    yukic_ast::ExterncItem::Cstruct(decl) => {
                        symbols.push((decl.name.name.clone(), SymbolKind::Cstruct, decl.span));
                    }
                }
            }
            symbols
        }
        ItemKind::Import(_) | ItemKind::Alias(_) | ItemKind::Error => Vec::new(),
    }
}

pub fn register_builtins(registry: &mut SymbolRegistry) -> Vec<(String, SymbolId)> {
    let builtins = [
        "exit",
        "typeof",
        "__typeof_intrinsic__",
        "cast",
        "args",
        "raw_args",
        "panic",
        "assert",
        "assert_eq",
        "assert_ne",
        "from_raw_parts",
        "ptr_of",
    ];
    builtins
        .iter()
        .map(|name| {
            let id = registry.register(
                (*name).to_string(),
                SymbolKind::Builtin,
                Visibility::Public,
                ModuleId::ROOT,
                Span::DUMMY,
            );
            ((*name).to_string(), id)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_ast::{Block, Ident};
    use yukic_ast::{FunDecl, Item, ItemId, ItemKind, Module, Visibility};
    use yukic_span::{BytePos, FileId, Span};

    fn dummy_span() -> Span {
        Span::new(FileId(0), BytePos(0), BytePos(10))
    }

    fn make_fun_item(name: &str, vis: Visibility) -> Item {
        let body_span = dummy_span();
        Item::new(
            ItemId(0),
            vis,
            ItemKind::Fun(FunDecl {
                name: Ident::new(name, dummy_span()),
                attrs: Vec::new(),
                generics: Vec::new(),
                params: Vec::new(),
                return_type: None,
                body: Block::new(Vec::new(), body_span),
                span: dummy_span(),
            }),
            dummy_span(),
        )
    }

    #[test]
    fn test_collect_symbols() {
        let mut module = Module::new(FileId(0));
        module.items.push(make_fun_item("main", Visibility::Public));
        module
            .items
            .push(make_fun_item("helper", Visibility::Local));

        let mut registry = SymbolRegistry::new();
        let (scope, diagnostics) = collect_module_symbols(&module, ModuleId(0), &mut registry);

        assert!(diagnostics.is_empty());
        assert!(scope.resolve_name("main").is_some());
        assert!(scope.resolve_name("helper").is_some());
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_duplicate_symbol_detected() {
        let mut module = Module::new(FileId(0));
        module.items.push(make_fun_item("foo", Visibility::Public));
        module.items.push(make_fun_item("foo", Visibility::Public));

        let mut registry = SymbolRegistry::new();
        let (_, diagnostics) = collect_module_symbols(&module, ModuleId(0), &mut registry);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("already defined"));
    }

    #[test]
    fn test_register_builtins() {
        let mut registry = SymbolRegistry::new();
        let builtins = register_builtins(&mut registry);

        assert_eq!(builtins.len(), 12);
        let names: Vec<&str> = builtins.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"exit"));
        assert!(names.contains(&"typeof"));
        assert!(names.contains(&"__typeof_intrinsic__"));
        assert!(names.contains(&"cast"));
        assert!(names.contains(&"args"));
        assert!(names.contains(&"raw_args"));
        assert!(names.contains(&"panic"));
        assert!(names.contains(&"assert"));
        assert!(names.contains(&"assert_eq"));
        assert!(names.contains(&"assert_ne"));
        assert!(names.contains(&"from_raw_parts"));
        assert!(names.contains(&"ptr_of"));
    }
}
