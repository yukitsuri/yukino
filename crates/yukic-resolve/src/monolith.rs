#![deny(unsafe_code)]

use indexmap::IndexMap;
use yukic_ast::{ItemKind, Module};
use yukic_span::FileId;

use crate::module_id::ModuleId;

pub fn build_monolithic_module(
    modules: &[(ModuleId, Module)],
    order: &[ModuleId],
    root_file_id: FileId,
) -> Module {
    let mod_map: IndexMap<ModuleId, &Module> = modules.iter().map(|(id, m)| (*id, m)).collect();
    let mut combined_items = Vec::new();

    for id in order {
        if let Some(module) = mod_map.get(id) {
            for item in &module.items {
                if !matches!(item.kind, ItemKind::Import(_)) {
                    combined_items.push(item.clone());
                }
            }
        }
    }

    let mut result = Module::new(root_file_id);
    result.items = combined_items;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_ast::{Block, FunDecl, Ident, Item, ItemId, Visibility};
    use yukic_span::{BytePos, Span};

    fn dummy_span() -> Span {
        Span::new(FileId(0), BytePos(0), BytePos(10))
    }

    fn make_item(name: &str) -> Item {
        Item::new(
            ItemId(0),
            Visibility::Public,
            ItemKind::Fun(FunDecl {
                name: Ident::new(name, dummy_span()),
                attrs: Vec::new(),
                generics: Vec::new(),
                params: Vec::new(),
                return_type: None,
                body: Block::new(Vec::new(), dummy_span()),
                span: dummy_span(),
            }),
            dummy_span(),
        )
    }

    #[test]
    fn test_build_monolithic_module() {
        let mut mod_a = Module::new(FileId(1));
        mod_a.items.push(make_item("helper"));

        let mut mod_b = Module::new(FileId(0));
        mod_b.items.push(make_item("main"));

        let modules = vec![(ModuleId(1), mod_a), (ModuleId(0), mod_b)];
        let order = vec![ModuleId(1), ModuleId(0)];

        let mono = build_monolithic_module(&modules, &order, FileId(0));
        assert_eq!(mono.file_id, FileId(0));
        assert_eq!(mono.items.len(), 2);
    }
}
