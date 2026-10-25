#![deny(unsafe_code)]
#![allow(clippy::result_large_err)]

use indexmap::IndexMap;

use yukic_diagnostics::Diagnostic;
use yukic_diagnostics::code::ErrorCode;
use yukic_span::Span;

use crate::module_id::ModuleId;
use crate::symbol::SymbolId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeBinding {
    pub symbol_id: SymbolId,
    pub span: Span,
    pub is_imported: bool,
    pub is_point_import: bool,
    pub pack_namespace: Option<String>,
    pub is_reexported: bool,
}

#[derive(Debug, Default)]
pub struct ModuleScope {
    pub module_id: ModuleId,
    bindings: IndexMap<String, ScopeBinding>,
    namespaces: IndexMap<String, NamespaceEntry>,
    aliases: IndexMap<String, AliasEntry>,
    ambiguous_symbols: IndexMap<String, Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamespaceEntry {
    pub name: String,
    pub target_module: ModuleId,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AliasEntry {
    pub namespace: String,
    pub symbol_name: String,
    pub target_symbol: SymbolId,
    pub span: Span,
}

impl ModuleScope {
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            bindings: IndexMap::new(),
            namespaces: IndexMap::new(),
            aliases: IndexMap::new(),
            ambiguous_symbols: IndexMap::new(),
        }
    }

    pub fn insert_binding(
        &mut self,
        name: String,
        binding: ScopeBinding,
    ) -> Result<(), Diagnostic> {
        if let Some(existing) = self.bindings.get(&name) {
            return Err(build_conflict_diagnostic(
                &name,
                existing.span,
                binding.span,
            ));
        }
        self.bindings.insert(name, binding);
        Ok(())
    }

    pub fn insert_glob_binding(&mut self, name: String, binding: ScopeBinding) {
        if !self.bindings.contains_key(&name) {
            self.bindings.insert(name, binding);
        }
    }

    pub fn insert_pack_symbol(
        &mut self,
        name: String,
        mut binding: ScopeBinding,
        namespace: &str,
    ) {
        binding.pack_namespace = Some(namespace.to_string());
        if let Some(namespaces) = self.ambiguous_symbols.get_mut(&name) {
            if !namespaces.contains(&namespace.to_string()) {
                namespaces.push(namespace.to_string());
            }
            self.bindings.swap_remove(&name);
            return;
        }

        if let Some(existing) = self.bindings.get(&name) {
            let prev_ns = existing
                .pack_namespace
                .clone()
                .unwrap_or_else(|| "local".to_string());
            if prev_ns != namespace {
                let mut nss = vec![prev_ns, namespace.to_string()];
                nss.sort();
                nss.dedup();
                self.ambiguous_symbols.insert(name.clone(), nss);
                self.bindings.swap_remove(&name);
                return;
            }
        }

        self.bindings.insert(name, binding);
    }

    pub fn ambiguous_symbols(&self) -> &IndexMap<String, Vec<String>> {
        &self.ambiguous_symbols
    }

    pub fn get_ambiguous(&self, name: &str) -> Option<&[String]> {
        self.ambiguous_symbols.get(name).map(|v| v.as_slice())
    }

    pub fn get_binding(&self, name: &str) -> Option<&ScopeBinding> {
        self.bindings.get(name)
    }

    pub fn insert_namespace(&mut self, entry: NamespaceEntry) -> Result<(), Diagnostic> {
        if let Some(existing) = self.namespaces.get(&entry.name) {
            return Err(build_conflict_diagnostic(
                &entry.name,
                existing.span,
                entry.span,
            ));
        }
        self.namespaces.insert(entry.name.clone(), entry);
        Ok(())
    }

    pub fn get_namespace(&self, name: &str) -> Option<&NamespaceEntry> {
        self.namespaces.get(name)
    }

    pub fn insert_alias(&mut self, name: String, entry: AliasEntry) -> Result<(), Diagnostic> {
        for (_, existing) in &self.aliases {
            if existing.namespace == entry.namespace && existing.symbol_name == entry.symbol_name {
                return Err(Diagnostic::error(
                    ErrorCode::E0304,
                    "symbol already aliased",
                    entry.span,
                )
                .with_secondary_label(existing.span, "previously aliased here"));
            }
        }
        if self.bindings.contains_key(&name) {
            let existing_span = self.bindings[&name].span;
            return Err(build_conflict_diagnostic(&name, existing_span, entry.span));
        }
        self.aliases.insert(name, entry);
        Ok(())
    }

    pub fn get_alias(&self, name: &str) -> Option<&AliasEntry> {
        self.aliases.get(name)
    }

    pub fn resolve_name(&self, name: &str) -> Option<SymbolId> {
        if let Some(binding) = self.bindings.get(name) {
            return Some(binding.symbol_id);
        }
        if let Some(alias) = self.aliases.get(name) {
            return Some(alias.target_symbol);
        }
        None
    }

    pub fn bindings(&self) -> &IndexMap<String, ScopeBinding> {
        &self.bindings
    }

    pub fn namespaces(&self) -> &IndexMap<String, NamespaceEntry> {
        &self.namespaces
    }

    pub fn aliases(&self) -> &IndexMap<String, AliasEntry> {
        &self.aliases
    }
}

#[derive(Debug, Default)]
pub struct GlobalSymbolTable {
    scopes: IndexMap<ModuleId, ModuleScope>,
    builtins: IndexMap<String, SymbolId>,
}

impl GlobalSymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_scope(&mut self, scope: ModuleScope) {
        self.scopes.insert(scope.module_id, scope);
    }

    pub fn get_scope(&self, module_id: ModuleId) -> Option<&ModuleScope> {
        self.scopes.get(&module_id)
    }

    pub fn get_scope_mut(&mut self, module_id: ModuleId) -> Option<&mut ModuleScope> {
        self.scopes.get_mut(&module_id)
    }

    pub fn register_builtin(&mut self, name: String, id: SymbolId) {
        self.builtins.insert(name, id);
    }

    pub fn get_builtin(&self, name: &str) -> Option<SymbolId> {
        self.builtins.get(name).copied()
    }

    pub fn resolve_in_module(&self, module_id: ModuleId, name: &str) -> Option<SymbolId> {
        if let Some(scope) = self.scopes.get(&module_id)
            && let Some(id) = scope.resolve_name(name)
        {
            return Some(id);
        }
        self.builtins.get(name).copied()
    }

    pub fn scopes(&self) -> &IndexMap<ModuleId, ModuleScope> {
        &self.scopes
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LexicalScope {
    bindings: IndexMap<String, ScopeBinding>,
}

impl LexicalScope {
    pub fn new() -> Self { Self::default() }

    pub fn insert(&mut self, name: String, binding: ScopeBinding) -> Result<(), Diagnostic> {
        if let Some(existing) = self.bindings.get(&name) {
            return Err(build_conflict_diagnostic(
                &name,
                existing.span,
                binding.span,
            ));
        }
        self.bindings.insert(name, binding);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&ScopeBinding> {
        self.bindings.get(name)
    }

    pub fn bindings(&self) -> &IndexMap<String, ScopeBinding> {
        &self.bindings
    }
}

#[derive(Clone, Debug)]
pub struct ScopeStack {
    scopes: Vec<LexicalScope>,
}

impl Default for ScopeStack {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopeStack {
    pub fn new() -> Self {
        Self { scopes: vec![LexicalScope::new()], }
    }

    pub fn push(&mut self) {
        self.scopes.push(LexicalScope::new());
    }

    pub fn pop(&mut self) -> Option<LexicalScope> {
        if self.scopes.len() > 1 { self.scopes.pop() } 
        else { None }
    }

    pub fn depth(&self) -> usize { self.scopes.len() }

    pub fn insert_current(
        &mut self,
        name: String,
        binding: ScopeBinding,
    ) -> Result<(), Diagnostic> {
        let current = self.scopes.last_mut().expect("scope stack is never empty");
        current.insert(name, binding)
    }

    pub fn lookup(&self, name: &str) -> Option<&ScopeBinding> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }
}

fn build_conflict_diagnostic(name: &str, first_span: Span, second_span: Span) -> Diagnostic {
    Diagnostic::error(
        ErrorCode::E0304,
        format!("name `{name}` is already defined in this scope"),
        second_span,
    )
    .with_secondary_label(first_span, "previously defined here")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::SymbolId;
    use yukic_span::{BytePos, FileId, Span};

    fn dummy_span() -> Span {
        Span::new(FileId(0), BytePos(0), BytePos(0))
    }

    fn span_at(start: u32, end: u32) -> Span {
        Span::new(FileId(0), BytePos(start), BytePos(end))
    }

    #[test]
    fn test_scope_insert_and_resolve() {
        let mut scope = ModuleScope::new(ModuleId(0));
        let binding = ScopeBinding {
            symbol_id: SymbolId(1),
            span: dummy_span(),
            is_imported: false,
            is_point_import: false,
            pack_namespace: None,
            is_reexported: false,
        };
        scope.insert_binding("calc_plus".into(), binding).unwrap();
        assert_eq!(scope.resolve_name("calc_plus"), Some(SymbolId(1)));
        assert_eq!(scope.resolve_name("unknown"), None);
    }

    #[test]
    fn test_scope_conflict() {
        let mut scope = ModuleScope::new(ModuleId(0));
        let b1 = ScopeBinding {
            symbol_id: SymbolId(1),
            span: span_at(0, 5),
            is_imported: false,
            is_point_import: false,
            pack_namespace: None,
            is_reexported: false,
        };
        let b2 = ScopeBinding {
            symbol_id: SymbolId(2),
            span: span_at(10, 15),
            is_imported: false,
            is_point_import: false,
            pack_namespace: None,
            is_reexported: false,
        };
        scope.insert_binding("foo".into(), b1).unwrap();
        let result = scope.insert_binding("foo".into(), b2);
        assert!(result.is_err());
    }

    #[test]
    fn test_glob_binding_does_not_overwrite() {
        let mut scope = ModuleScope::new(ModuleId(0));
        let point = ScopeBinding {
            symbol_id: SymbolId(1),
            span: dummy_span(),
            is_imported: true,
            is_point_import: true,
            pack_namespace: None,
            is_reexported: false,
        };
        scope.insert_binding("calc".into(), point).unwrap();

        let glob = ScopeBinding {
            symbol_id: SymbolId(2),
            span: dummy_span(),
            is_imported: true,
            is_point_import: false,
            pack_namespace: None,
            is_reexported: false,
        };
        scope.insert_glob_binding("calc".into(), glob);

        // point-import should win
        assert_eq!(scope.resolve_name("calc"), Some(SymbolId(1)));
    }

    #[test]
    fn test_global_symbol_table() {
        let mut table = GlobalSymbolTable::new();
        let mut scope = ModuleScope::new(ModuleId(0));
        let binding = ScopeBinding {
            symbol_id: SymbolId(1),
            span: dummy_span(),
            is_imported: false,
            is_point_import: false,
            pack_namespace: None,
            is_reexported: false,
        };
        scope.insert_binding("main".into(), binding).unwrap();
        table.add_scope(scope);

        table.register_builtin("exit".into(), SymbolId(100));

        assert_eq!(
            table.resolve_in_module(ModuleId(0), "main"),
            Some(SymbolId(1))
        );
        assert_eq!(
            table.resolve_in_module(ModuleId(0), "exit"),
            Some(SymbolId(100))
        );
        assert_eq!(table.resolve_in_module(ModuleId(0), "unknown"), None);
    }

    #[test]
    fn test_alias() {
        let mut scope = ModuleScope::new(ModuleId(0));
        let entry = AliasEntry {
            namespace: "sqlite".into(),
            symbol_name: "open".into(),
            target_symbol: SymbolId(5),
            span: dummy_span(),
        };
        scope.insert_alias("open".into(), entry).unwrap();
        assert_eq!(scope.resolve_name("open"), Some(SymbolId(5)));
    }

    #[test]
    fn test_lexical_scope_shadowing_and_lookup() {
        let mut stack = ScopeStack::new();
        let b1 = ScopeBinding {
            symbol_id: SymbolId(1),
            span: dummy_span(),
            is_imported: false,
            is_point_import: false,
            pack_namespace: None,
            is_reexported: false,
        };
        stack.insert_current("x".into(), b1).unwrap();
        assert_eq!(stack.lookup("x").map(|b| b.symbol_id), Some(SymbolId(1)));

        stack.push();
        assert_eq!(stack.depth(), 2);
        assert_eq!(stack.lookup("x").map(|b| b.symbol_id), Some(SymbolId(1)));

        let b2 = ScopeBinding {
            symbol_id: SymbolId(2),
            span: dummy_span(),
            is_imported: false,
            is_point_import: false,
            pack_namespace: None,
            is_reexported: false,
        };
        stack.insert_current("x".into(), b2).unwrap();
        assert_eq!(stack.lookup("x").map(|b| b.symbol_id), Some(SymbolId(2)));

        let popped = stack.pop();
        assert!(popped.is_some());
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.lookup("x").map(|b| b.symbol_id), Some(SymbolId(1)));
    }

    #[test]
    fn test_pack_symbol_collision_makes_ambiguous() {
        let mut scope = ModuleScope::new(ModuleId(0));
        let b1 = ScopeBinding {
            symbol_id: SymbolId(1),
            span: dummy_span(),
            is_imported: true,
            is_point_import: false,
            pack_namespace: None,
            is_reexported: false,
        };
        scope.insert_pack_symbol("flush".into(), b1, "io");
        assert_eq!(scope.resolve_name("flush"), Some(SymbolId(1)));
        assert!(scope.get_ambiguous("flush").is_none());

        let b2 = ScopeBinding {
            symbol_id: SymbolId(2),
            span: dummy_span(),
            is_imported: true,
            is_point_import: false,
            pack_namespace: None,
            is_reexported: false,
        };
        scope.insert_pack_symbol("flush".into(), b2, "logger");
        assert_eq!(scope.resolve_name("flush"), None);
        assert_eq!(
            scope.get_ambiguous("flush"),
            Some(&["io".to_string(), "logger".to_string()][..])
        );
    }
}
