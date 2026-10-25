#![deny(unsafe_code)]

use indexmap::IndexMap;
use yukic_ast::{Ident, MacroDefDecl};

/// lex scope for macros inside a block
#[derive(Clone, Debug, Default)]
pub struct MacroScope {
    macros: IndexMap<String, MacroDefDecl>,
}

impl MacroScope {
    pub fn new() -> Self { Self::default() }

    pub fn insert(&mut self, name: String, decl: MacroDefDecl) {
        self.macros.insert(name, decl);
    }

    pub fn get(&self, name: &str) -> Option<&MacroDefDecl> {
        self.macros.get(name)
    }
}

#[derive(Clone, Debug, Default)]
pub struct MacroTable {
    module_macros: IndexMap<String, MacroDefDecl>,
    qualified_macros: IndexMap<String, IndexMap<String, MacroDefDecl>>,
    scopes: Vec<MacroScope>,
}

impl MacroTable {
    pub fn new() -> Self {
        Self {
            module_macros: IndexMap::new(),
            qualified_macros: IndexMap::new(),
            scopes: vec![MacroScope::new()],
        }
    }

    pub fn register(&mut self, name: String, decl: MacroDefDecl) {
        self.module_macros.insert(name, decl);
    }

    pub fn register_qualified(&mut self, module_path: String, name: String, decl: MacroDefDecl) {
        self.qualified_macros
            .entry(module_path)
            .or_default()
            .insert(name, decl);
    }

    pub fn push_scope(&mut self) { self.scopes.push(MacroScope::new()); }
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 { self.scopes.pop(); }
    }

    pub fn register_local(&mut self, name: String, decl: MacroDefDecl) {
        let current = self.scopes.last_mut().expect("scope stack is never empty");
        current.insert(name, decl);
    }

    pub fn lookup(&self, path: &[Ident]) -> Option<MacroDefDecl> {


        if path.is_empty() { return None; }

        if path.len() == 1 {
            let name = &path[0].name;
            for scope in self.scopes.iter().rev() {
                if let Some(decl) = scope.get(name) {
                    return Some(decl.clone());
                }
            }
            return self.module_macros.get(name).cloned();
        }

        let mod_prefix = path[..path.len() - 1]
            .iter()
            .map(|id| id.name.as_str())
            .collect::<Vec<_>>()
            .join("::");
        let macro_name = &path[path.len() - 1].name;

        self.qualified_macros
            .get(&mod_prefix)
            .and_then(|m| m.get(macro_name))
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_ast::MacroBody;
    use yukic_span::{BytePos, FileId, Span};

    fn dummy_span() -> Span {
        Span::new(FileId(0), BytePos(0), BytePos(10))
    }

    fn make_macro(name: &str) -> MacroDefDecl {
        MacroDefDecl {
            name: Ident::new(name, dummy_span()),
            params: Vec::new(),
            body: MacroBody::Block(yukic_ast::Block::new(Vec::new(), dummy_span())),
            span: dummy_span(),
        }
    }

    #[test]
    fn test_macro_table_local_shadowing() {
        let mut table = MacroTable::new();
        table.register("m".into(), make_macro("global_m"));

        let lookup1 = table.lookup(&[Ident::new("m", dummy_span())]).unwrap();
        assert_eq!(lookup1.name.name, "global_m");

        table.push_scope();
        table.register_local("m".into(), make_macro("local_m"));

        let lookup2 = table.lookup(&[Ident::new("m", dummy_span())]).unwrap();
        assert_eq!(lookup2.name.name, "local_m");

        table.pop_scope();
        let lookup3 = table.lookup(&[Ident::new("m", dummy_span())]).unwrap();
        assert_eq!(lookup3.name.name, "global_m");
    }

    #[test]
    fn test_macro_table_qualified() {
        let mut table = MacroTable::new();
        table.register_qualified("math".into(), "square".into(), make_macro("square"));

        let path = vec![
            Ident::new("math", dummy_span()),
            Ident::new("square", dummy_span()),
        ];
        let found = table.lookup(&path);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name.name, "square");
    }
}
