#![deny(unsafe_code)]

use crate::attr::AttributeNode;
use crate::item::{Item, ItemKind, Module};

/// Module-level attributes propagated to every function-like declaration of the file.
const PROPAGATED_ATTRIBUTES: [&str; 8] = [
    "deprecated",
    "stable",
    "inline",
    "no_inline",
    "inline_never",
    "inline_always",
    "cold",
    "noreturn",
];

/// Mutually exclusive inlining hints; any of them on a declaration overrides the file-level one.
const INLINE_ATTRIBUTES: [&str; 4] = [
    "inline",
    "no_inline",
    "inline_never",
    "inline_always",
];

/// Appends file-level attributes to all fun/method/accessor declarations; item-level attrs win.
pub fn propagate_module_attributes(module: &mut Module) {
    let inherited: Vec<AttributeNode> = module
        .module_attrs
        .iter()
        .filter(|attr| {
            PROPAGATED_ATTRIBUTES.contains(&attr.path_str().as_str())
        })
        .cloned()
        .collect();
    if inherited.is_empty() {
        return;
    }

    for item in &mut module.items {
        propagate_item(item, &inherited);
    }
}

fn propagate_item(item: &mut Item, inherited: &[AttributeNode]) {
    match &mut item.kind {
        ItemKind::Fun(decl) => append_inherited(&mut decl.attrs, inherited),
        ItemKind::Cfun(decl) if decl.body.is_some() => {
            append_inherited(&mut decl.yukino_attrs, inherited)
        }
        ItemKind::Class(decl) => {
            for method in &mut decl.methods {
                append_inherited(&mut method.attrs, inherited);
            }
            for accessor in &mut decl.accessors {
                append_inherited(&mut accessor.attrs, inherited);
            }
        }
        ItemKind::Struct(decl) => {
            for method in &mut decl.methods {
                append_inherited(&mut method.attrs, inherited);
            }
            for method in &mut decl.static_methods {
                append_inherited(&mut method.attrs, inherited);
            }
        }
        _ => {}
    }
}

fn append_inherited(target: &mut Vec<AttributeNode>, inherited: &[AttributeNode]) {
    for attr in inherited {
        let name = attr.path_str();
        let overridden = target.iter().any(|existing| {
            let existing_name = existing.path_str();
            existing_name == name
                || (INLINE_ATTRIBUTES.contains(&name.as_str())
                    && INLINE_ATTRIBUTES.contains(&existing_name.as_str()))
        });
        if !overridden {
            target.push(attr.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attr::AttributeArgs;
    use crate::expr::{Expr, ExprKind, LiteralValue};
    use crate::ident::Ident;
    use crate::item::{FunDecl, MethodDecl, StaticMethodDecl, StructDecl, ThisParam};
    use crate::stmt::Block;
    use crate::{Item, ItemId, Visibility};
    use yukic_span::{BytePos, FileId, Span};

    fn span() -> Span {
        Span::new(FileId(0), BytePos(0), BytePos(1))
    }

    fn module_attr(name: &str) -> AttributeNode {
        AttributeNode::new(Ident::new(name, span()), AttributeArgs::None, span())
    }

    fn fun(name: &str, attrs: Vec<AttributeNode>) -> Item {
        Item::new(
            ItemId(0),
            Visibility::Local,
            ItemKind::Fun(FunDecl {
                name: Ident::new(name, span()),
                attrs,
                generics: Vec::new(),
                params: Vec::new(),
                return_type: None,
                body: Block::new(Vec::new(), span()),
                span: span(),
            }),
            span(),
        )
    }

    #[test]
    fn propagates_codegen_attributes_to_functions_and_methods() {
        let mut module = Module::new(FileId(0));
        module
            .module_attrs
            .extend([module_attr("cold"), module_attr("deprecated")]);
        module.items.push(fun("alpha", Vec::new()));

        let struct_decl = StructDecl {
            name: Ident::new("S", span()),
            fields: Vec::new(),
            methods: vec![MethodDecl {
                name: Ident::new("m", span()),
                attrs: Vec::new(),
                this_param: ThisParam::Value,
                params: Vec::new(),
                return_type: None,
                body: Block::new(Vec::new(), span()),
                is_operator: false,
                is_override: false,
                span: span(),
            }],
            static_methods: vec![StaticMethodDecl {
                name: Ident::new("sm", span()),
                attrs: Vec::new(),
                params: Vec::new(),
                return_type: None,
                body: Block::new(Vec::new(), span()),
                span: span(),
            }],
            span: span(),
        };
        let struct_item = Item::new(
            ItemId(1),
            Visibility::Local,
            ItemKind::Struct(struct_decl),
            span(),
        );
        module.items.push(struct_item);

        propagate_module_attributes(&mut module);

        let ItemKind::Fun(fun) = &module.items[0].kind else {
            panic!("first item must be a fun");
        };
        let names: Vec<String> = fun.attrs.iter().map(|a| a.path_str()).collect();
        assert!(names.iter().any(|n| n == "cold"));
        assert!(names.iter().any(|n| n == "deprecated"));

        let ItemKind::Struct(decl) = &module.items[1].kind else {
            panic!("second item must be a struct");
        };
        assert!(decl.methods[0].attrs.iter().any(|a| a.is("cold")));
        assert!(decl.static_methods[0]
            .attrs
            .iter()
            .any(|a| a.is("cold")));
    }

    #[test]
    fn item_level_attribute_wins_over_module_level() {
        let mut module = Module::new(FileId(0));
        module.module_attrs.push(module_attr("deprecated"));
        let msg = Expr::new(
            ExprKind::Literal(LiteralValue::String("use beta".to_string())),
            span(),
        );
        let mut own = module_attr("deprecated");
        own.args = AttributeArgs::Positional(vec![msg]);
        module.items.push(fun("alpha", vec![own]));

        propagate_module_attributes(&mut module);

        let ItemKind::Fun(fun) = &module.items[0].kind else {
            panic!("item must be a fun");
        };
        assert_eq!(fun.attrs.len(), 1);
        assert!(matches!(&fun.attrs[0].args, AttributeArgs::Positional(_)));
    }

    #[test]
    fn does_not_propagate_module_scoped_attributes() {
        let mut module = Module::new(FileId(0));
        module
            .module_attrs
            .extend([module_attr("target"), module_attr("warns_ignore")]);
        module.items.push(fun("alpha", Vec::new()));

        propagate_module_attributes(&mut module);

        let ItemKind::Fun(fun) = &module.items[0].kind else {
            panic!("item must be a fun");
        };
        assert!(fun.attrs.is_empty());
    }

    #[test]
    fn statements_stay_untouched() {
        let mut module = Module::new(FileId(0));
        module.module_attrs.push(module_attr("cold"));
        module.items.push(Item::new(
            ItemId(0),
            Visibility::Local,
            ItemKind::Static(crate::item::StaticDecl {
                name: Ident::new("N", span()),
                ty: None,
                init: Expr::new(ExprKind::Literal(LiteralValue::Int(1)), span()),
                span: span(),
            }),
            span(),
        ));

        propagate_module_attributes(&mut module);

        let ItemKind::Static(decl) = &module.items[0].kind else {
            panic!("item must be a static");
        };
        assert!(decl.name.name == "N");
    }
    #[test]
    fn propagates_to_cfun_with_body() {
        let mut module = Module::new(FileId(0));
        module.module_attrs.push(module_attr("cold"));
        module.items.push(Item::new(
            ItemId(0),
            Visibility::Local,
            ItemKind::Cfun(crate::item::CfunDecl {
                name: Ident::new("native_worker", span()),
                c_attrs: Vec::new(),
                yukino_attrs: Vec::new(),
                params: Vec::new(),
                return_type: None,
                symbol_override: None,
                body: Some(Block::new(Vec::new(), span())),
                span: span(),
            }),
            span(),
        ));

        propagate_module_attributes(&mut module);

        let ItemKind::Cfun(decl) = &module.items[0].kind else {
            panic!("item must be a cfun");
        };
        assert!(decl.yukino_attrs.iter().any(|a| a.is("cold")));
    }
}