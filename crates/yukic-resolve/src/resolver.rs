#![deny(unsafe_code)]
#![allow(clippy::result_large_err)]

use yukic_ast::{
    AliasDecl, Ident, ImportDecl, ImportPattern, ItemKind, Module,
};
use yukic_diagnostics::Diagnostic;
use yukic_diagnostics::code::ErrorCode;
use yukic_span::Span;

use crate::module_id::ModuleId;
use crate::module_path::build_target_name;
use crate::scope::{AliasEntry, GlobalSymbolTable, ModuleScope, NamespaceEntry, ScopeBinding};
use crate::symbol::SymbolRegistry;

pub fn resolve_imports(
    module: &Module,
    module_id: ModuleId,
    symbol_table: &mut GlobalSymbolTable,
    registry: &SymbolRegistry,
    module_scopes: &[(ModuleId, String)],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for item in &module.items {
        match &item.kind {
            ItemKind::Import(import_decl) => {
                let is_reexported = item.visibility == yukic_ast::Visibility::Public;
                let errs = resolve_single_import(
                    import_decl,
                    is_reexported,
                    module_id,
                    symbol_table,
                    registry,
                    module_scopes,
                );
                diagnostics.extend(errs);
            }
            ItemKind::Alias(alias_decl) => {
                let errs = resolve_alias(
                    alias_decl,
                    module_id,
                    symbol_table,
                    registry,
                    module_scopes,
                );
                diagnostics.extend(errs);
            }
            _ => {}
        }
    }

    diagnostics
}

fn resolve_single_import(
    import: &ImportDecl,
    is_reexported: bool,
    current_module: ModuleId,
    symbol_table: &mut GlobalSymbolTable,
    registry: &SymbolRegistry,
    module_scopes: &[(ModuleId, String)],
) -> Vec<Diagnostic> {
    if let Some(clause) = &import.symbols {
        resolve_symbol_import(
            import,
            is_reexported,
            clause,
            current_module,
            symbol_table,
            registry,
            module_scopes,
        )
    } else {
        resolve_namespace_import(
            import,
            current_module,
            symbol_table,
            module_scopes,
        )
    }
}

fn resolve_namespace_import(
    import: &ImportDecl,
    current_module: ModuleId,
    symbol_table: &mut GlobalSymbolTable,
    module_scopes: &[(ModuleId, String)],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let has_wildcard = import.path.segments.iter().any(|s| s.pattern.is_wildcard());

    if has_wildcard {
        let mut matched = false;
        for (mod_id, mod_name) in module_scopes {
            if module_matches_path(mod_name, &import.path) {
                matched = true;
                let ns_name = mod_name.replace('.', "::");
                let entry = NamespaceEntry {
                    name: ns_name,
                    target_module: *mod_id,
                    span: import.span,
                };
                if let Some(scope) = symbol_table.get_scope_mut(current_module)
                    && let Err(d) = scope.insert_namespace(entry)
                {
                    diagnostics.push(d);
                }
            }
        }
        if !matched {
            diagnostics.push(Diagnostic::error(
                ErrorCode::E0301,
                "no modules matched the import pattern",
                import.span,
            ));
        }
        return diagnostics;
    }

    let (target_name, target) =
        resolve_import_target_module(&import.path, current_module, module_scopes);
    let Some(target_module_id) = target else {
        return vec![Diagnostic::error(
            ErrorCode::E0301,
            format!("module `{target_name}` not found"),
            import.span,
        )];
    };

    let ns_name = target_name.replace('.', "::");
    let entry = NamespaceEntry {
        name: ns_name,
        target_module: target_module_id,
        span: import.span,
    };

    let Some(scope) = symbol_table.get_scope_mut(current_module) else {
        return diagnostics;
    };
    if let Err(d) = scope.insert_namespace(entry) {
        diagnostics.push(d);
    }

    diagnostics
}

fn resolve_symbol_import(
    import: &ImportDecl,
    is_reexported: bool,
    clause: &yukic_ast::ImportSymbolsClause,
    current_module: ModuleId,
    symbol_table: &mut GlobalSymbolTable,
    registry: &SymbolRegistry,
    module_scopes: &[(ModuleId, String)],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let (target_name, target) =
        resolve_import_target_module(&import.path, current_module, module_scopes);

    let Some(target_module_id) = target else {
        return vec![Diagnostic::error(
            ErrorCode::E0301,
            format!("module `{target_name}` not found"),
            import.span,
        )];
    };

    let bindings_to_insert = {
        let Some(target_scope) = symbol_table.get_scope(target_module_id) else {
            return diagnostics;
        };
        collect_clause_bindings(clause, is_reexported, target_scope, registry, &mut diagnostics)
    };

    let Some(scope) = symbol_table.get_scope_mut(current_module) else {
        return diagnostics;
    };

    for (name, b, is_exact) in bindings_to_insert {
        if is_exact {
            if let Err(diag) = scope.insert_binding(name, b) {
                diagnostics.push(diag);
            }
        } else {
            scope.insert_glob_binding(name, b);
        }
    }

    diagnostics
}

fn collect_clause_bindings(
    clause: &yukic_ast::ImportSymbolsClause,
    is_reexported: bool,
    target_scope: &ModuleScope,
    registry: &SymbolRegistry,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<(String, ScopeBinding, bool)> {
    let mut bindings = Vec::new();
    for (idx, item) in clause.items.iter().enumerate() {
        let alias = clause.aliases.get(idx).map(|id| id.as_str());
        match &item.pattern {
            ImportPattern::Exact(id) => {
                let sym_name = id.as_str();
                let binding = target_scope.get_binding(sym_name);
                let Some(target_binding) = binding else {
                    diagnostics.push(Diagnostic::error(
                        ErrorCode::E0305,
                        format!("symbol `{sym_name}` not found in target module"),
                        item.span,
                    ));
                    continue;
                };

                if target_binding.is_imported && !target_binding.is_reexported {
                    diagnostics.push(
                        Diagnostic::error(
                            ErrorCode::E0303,
                            format!("`{sym_name}` is not public"),
                            item.span,
                        )
                        .with_secondary_label(target_binding.span, "imported here as private"),
                    );
                    continue;
                }

                if let Some(entry) = registry.get(target_binding.symbol_id)
                    && !entry.is_public()
                {
                    diagnostics.push(
                        Diagnostic::error(
                            ErrorCode::E0303,
                            format!("`{sym_name}` is not public"),
                            item.span,
                        )
                        .with_secondary_label(entry.span, "defined here as local"),
                    );
                    continue;
                }

                let import_name = alias.unwrap_or(sym_name).to_string();
                let b = ScopeBinding {
                    symbol_id: target_binding.symbol_id,
                    span: item.span,
                    is_imported: true,
                    is_point_import: true,
                    pack_namespace: None,
                    is_reexported,
                };
                bindings.push((import_name, b, true));
            }
            wildcard => {
                for (name, b) in
                    collect_matching_public_bindings(target_scope, wildcard, registry, item.span, is_reexported)
                {
                    bindings.push((name, b, false));
                }
            }
        }
    }
    bindings
}

fn resolve_alias(
    alias: &AliasDecl,
    current_module: ModuleId,
    symbol_table: &mut GlobalSymbolTable,
    registry: &SymbolRegistry,
    module_scopes: &[(ModuleId, String)],
) -> Vec<Diagnostic> {
    match &alias.kind {
        yukic_ast::AliasKind::Namespace(ns_idents) => {
            resolve_alias_namespace(alias, ns_idents, current_module, symbol_table, module_scopes)
        }
        yukic_ast::AliasKind::Symbol { namespace, symbol } => {
            resolve_alias_symbol(
                alias,
                namespace,
                symbol,
                current_module,
                symbol_table,
                registry,
                module_scopes,
            )
        }
        yukic_ast::AliasKind::Direct(orig) => {
            resolve_alias_direct(alias, orig, current_module, symbol_table)
        }
    }
}

fn resolve_alias_namespace(
    alias: &AliasDecl,
    ns_idents: &[Ident],
    current_module: ModuleId,
    symbol_table: &mut GlobalSymbolTable,
    module_scopes: &[(ModuleId, String)],
) -> Vec<Diagnostic> {
    let ns_path = ns_idents.iter().map(|id| id.as_str()).collect::<Vec<_>>().join("::");
    let mod_dot_path = ns_idents.iter().map(|id| id.as_str()).collect::<Vec<_>>().join(".");

    let target_module = if let Some(scope) = symbol_table.get_scope(current_module)
        && let Some(ns_entry) = scope.get_namespace(&ns_path)
    {
        Some(ns_entry.target_module)
    } else {
        find_target_module(&mod_dot_path, module_scopes)
    };

    let Some(target_module_id) = target_module else {
        return vec![Diagnostic::error(
            ErrorCode::E0305,
            format!("namespace `{ns_path}` not found"),
            alias.span,
        )];
    };

    let entry = NamespaceEntry {
        name: alias.alias.name.clone(),
        target_module: target_module_id,
        span: alias.span,
    };

    let Some(scope) = symbol_table.get_scope_mut(current_module) else {
        return Vec::new();
    };
    match scope.insert_namespace(entry) {
        Ok(()) => Vec::new(),
        Err(diag) => vec![diag],
    }
}

fn resolve_alias_symbol(
    alias: &AliasDecl,
    namespace: &[Ident],
    symbol: &Ident,
    current_module: ModuleId,
    symbol_table: &mut GlobalSymbolTable,
    registry: &SymbolRegistry,
    module_scopes: &[(ModuleId, String)],
) -> Vec<Diagnostic> {
    let ns_path = namespace.iter().map(|id| id.as_str()).collect::<Vec<_>>().join("::");
    let mod_dot_path = namespace.iter().map(|id| id.as_str()).collect::<Vec<_>>().join(".");

    let target_module = if let Some(scope) = symbol_table.get_scope(current_module)
        && let Some(ns_entry) = scope.get_namespace(&ns_path)
    {
        Some(ns_entry.target_module)
    } else {
        find_target_module(&mod_dot_path, module_scopes)
    };

    let Some(target_module_id) = target_module else {
        return vec![Diagnostic::error(
            ErrorCode::E0305,
            format!("namespace `{ns_path}` not found"),
            alias.span,
        )];
    };

    let (target_symbol_id, sym_name) = {
        let Some(target_scope) = symbol_table.get_scope(target_module_id) else {
            return Vec::new();
        };

        let sym_name = symbol.as_str();
        let Some(target_binding) = target_scope.get_binding(sym_name) else {
            return vec![Diagnostic::error(
                ErrorCode::E0305,
                format!("symbol `{sym_name}` not found in `{ns_path}`"),
                symbol.span,
            )];
        };

        if let Some(entry) = registry.get(target_binding.symbol_id)
            && !entry.is_public()
        {
            return vec![
                Diagnostic::error(
                    ErrorCode::E0303,
                    format!("`{sym_name}` is not public in `{ns_path}`"),
                    alias.span,
                )
                .with_secondary_label(entry.span, "defined here as local"),
            ];
        }

        (target_binding.symbol_id, sym_name.to_string())
    };

    let Some(scope) = symbol_table.get_scope_mut(current_module) else {
        return Vec::new();
    };

    let b = ScopeBinding {
        symbol_id: target_symbol_id,
        span: alias.span,
        is_imported: true,
        is_point_import: true,
        pack_namespace: None,
        is_reexported: false,
    };
    let alias_entry = AliasEntry {
        namespace: ns_path,
        symbol_name: sym_name,
        target_symbol: target_symbol_id,
        span: alias.span,
    };

    let _ = scope.insert_alias(alias.alias.name.clone(), alias_entry);
    match scope.insert_binding(alias.alias.name.clone(), b) {
        Ok(()) => Vec::new(),
        Err(diag) => vec![diag],
    }
}

fn resolve_alias_direct(
    alias: &AliasDecl,
    orig: &Ident,
    current_module: ModuleId,
    symbol_table: &mut GlobalSymbolTable,
) -> Vec<Diagnostic> {
    let orig_name = orig.as_str();
    let target_binding = {
        let Some(scope) = symbol_table.get_scope(current_module) else {
            return Vec::new();
        };

        let Some(binding) = scope.get_binding(orig_name) else {
            return vec![Diagnostic::error(
                ErrorCode::E0305,
                format!("symbol `{orig_name}` not found in current scope"),
                orig.span,
            )];
        };

        (binding.symbol_id, binding.is_imported)
    };

    let b = ScopeBinding {
        symbol_id: target_binding.0,
        span: alias.span,
        is_imported: target_binding.1,
        is_point_import: true,
        pack_namespace: None,
        is_reexported: false,
    };

    let Some(scope_mut) = symbol_table.get_scope_mut(current_module) else {
        return Vec::new();
    };
    match scope_mut.insert_binding(alias.alias.name.clone(), b) {
        Ok(()) => Vec::new(),
        Err(diag) => vec![diag],
    }
}

fn module_matches_path(mod_name: &str, path: &yukic_ast::ImportPath) -> bool {
    let (last, lead) = match path.segments.split_last() {
        Some(p) => p,
        None => return false,
    };
    let base_name = crate::module_path::build_target_name_from_segments(&path.prefix, lead);
    if base_name.is_empty() {
        last.pattern.matches(mod_name)
    } else if let Some(rest) = mod_name.strip_prefix(&format!("{base_name}.")) {
        last.pattern.matches(rest)
    } else {
        false
    }
}

fn find_target_module(name: &str, module_scopes: &[(ModuleId, String)]) -> Option<ModuleId> {
    if let Some((id, _)) = module_scopes.iter().find(|(_, n)| n == name) {
        return Some(*id);
    }
    let lib_name = format!("{name}.lib");
    if let Some((id, _)) = module_scopes.iter().find(|(_, n)| n == &lib_name) {
        return Some(*id);
    }
    let mod_name = format!("{name}.mod");
    module_scopes
        .iter()
        .find(|(_, n)| n == &mod_name)
        .map(|(id, _)| *id)
}

fn resolve_import_target_module(
    path: &yukic_ast::ImportPath,
    current_module: ModuleId,
    module_scopes: &[(ModuleId, String)],
) -> (String, Option<ModuleId>) {
    let raw_target_name = build_target_name(path);
    let current_name = module_scopes
        .iter()
        .find(|(id, _)| *id == current_module)
        .map(|(_, n)| n.as_str());

    if let yukic_ast::PathPrefix::Relative(levels) = &path.prefix {
        if let Some(curr) = current_name {
            let parts: Vec<&str> = curr.split('.').collect();
            // Candidate 1: treat current module as package (e.g. mod.ykn / lib.ykn)
            if parts.len() >= *levels {
                let keep = parts.len() - *levels;
                let base = parts[..keep].join(".");
                let cand = if base.is_empty() {
                    raw_target_name.clone()
                } else if raw_target_name.is_empty() {
                    base
                } else {
                    format!("{base}.{raw_target_name}")
                };
                if let Some(id) = find_target_module(&cand, module_scopes) {
                    return (cand, Some(id));
                }
            }
            // Candidate 2: treat current module as leaf file (sibling lookup)
            if parts.len() > *levels {
                let keep = parts.len() - 1 - *levels;
                let base = parts[..keep].join(".");
                let cand = if base.is_empty() {
                    raw_target_name.clone()
                } else if raw_target_name.is_empty() {
                    base
                } else {
                    format!("{base}.{raw_target_name}")
                };
                if let Some(id) = find_target_module(&cand, module_scopes) {
                    return (cand, Some(id));
                }
            }
        }
    }

    let id = find_target_module(&raw_target_name, module_scopes);
    (raw_target_name, id)
}

fn collect_public_bindings(
    scope: &ModuleScope,
    registry: &SymbolRegistry,
    import_span: Span,
    is_reexported: bool,
) -> Vec<(String, ScopeBinding)> {
    let mut result = Vec::new();
    for (name, binding) in scope.bindings() {
        let is_pub = if binding.is_imported {
            binding.is_reexported
        } else if let Some(entry) = registry.get(binding.symbol_id) {
            entry.is_public()
        } else {
            false
        };

        if is_pub {
            result.push((
                name.clone(),
                ScopeBinding {
                    symbol_id: binding.symbol_id,
                    span: import_span,
                    is_imported: true,
                    is_point_import: false,
                    pack_namespace: None,
                    is_reexported,
                },
            ));
        }
    }
    result
}

fn collect_matching_public_bindings(
    scope: &ModuleScope,
    pattern: &ImportPattern,
    registry: &SymbolRegistry,
    import_span: Span,
    is_reexported: bool,
) -> Vec<(String, ScopeBinding)> {
    collect_public_bindings(scope, registry, import_span, is_reexported)
        .into_iter()
        .filter(|(name, _)| pattern.matches(name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collector::collect_module_symbols;
    use crate::symbol::SymbolRegistry;
    use yukic_ast::*;
    use yukic_span::{BytePos, FileId, Span};

    fn dummy_span() -> Span {
        Span::new(FileId(0), BytePos(0), BytePos(10))
    }

    fn make_fun_item(name: &str, vis: Visibility) -> Item {
        Item::new(
            ItemId(0),
            vis,
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

    fn make_segment(name: &str) -> ImportPathSegment {
        ImportPathSegment {
            pattern: ImportPattern::Exact(Ident::new(name, dummy_span())),
            force_folder: false,
            span: dummy_span(),
        }
    }

    fn make_load_symbols_item(
        target: &str,
        symbols: Vec<ImportSymbolItem>,
        aliases: Vec<Ident>,
    ) -> Item {
        Item::new(
            ItemId(0),
            Visibility::Local,
            ItemKind::Import(ImportDecl {
                path: ImportPath {
                    prefix: PathPrefix::Package(Ident::new(target, dummy_span())),
                    segments: Vec::new(),
                    span: dummy_span(),
                },
                symbols: Some(ImportSymbolsClause {
                    items: symbols,
                    aliases,
                    span: dummy_span(),
                }),
                span: dummy_span(),
            }),
            dummy_span(),
        )
    }

    fn make_load_namespace_item(target: &str) -> Item {
        Item::new(
            ItemId(0),
            Visibility::Local,
            ItemKind::Import(ImportDecl {
                path: ImportPath {
                    prefix: PathPrefix::Package(Ident::new(target, dummy_span())),
                    segments: Vec::new(),
                    span: dummy_span(),
                },
                symbols: None,
                span: dummy_span(),
            }),
            dummy_span(),
        )
    }

    #[test]
    fn test_resolve_point_import() {
        let mut registry = SymbolRegistry::new();
        let mut symbol_table = GlobalSymbolTable::new();

        let mut target = Module::new(FileId(1));
        target
            .items
            .push(make_fun_item("calc_plus", Visibility::Public));
        let (target_scope, _) = collect_module_symbols(&target, ModuleId(1), &mut registry);
        symbol_table.add_scope(target_scope);

        let import_item = ImportSymbolItem {
            pattern: ImportPattern::Exact(Ident::new("calc_plus", dummy_span())),
            span: dummy_span(),
        };
        let mut source = Module::new(FileId(0));
        source.items.push(make_load_symbols_item("utils", vec![import_item], Vec::new()));
        let source_scope = crate::scope::ModuleScope::new(ModuleId(0));
        symbol_table.add_scope(source_scope);

        let module_scopes = vec![(ModuleId(1), "utils".to_string())];
        let diags = resolve_imports(
            &source,
            ModuleId(0),
            &mut symbol_table,
            &registry,
            &module_scopes,
        );

        assert!(diags.is_empty(), "diags: {diags:?}");
        assert!(
            symbol_table
                .resolve_in_module(ModuleId(0), "calc_plus")
                .is_some()
        );
    }

    #[test]
    fn test_private_symbol_access_rejected() {
        let mut registry = SymbolRegistry::new();
        let mut symbol_table = GlobalSymbolTable::new();

        let mut target = Module::new(FileId(1));
        target
            .items
            .push(make_fun_item("secret", Visibility::Local));
        let (target_scope, _) = collect_module_symbols(&target, ModuleId(1), &mut registry);
        symbol_table.add_scope(target_scope);

        let import_item = ImportSymbolItem {
            pattern: ImportPattern::Exact(Ident::new("secret", dummy_span())),
            span: dummy_span(),
        };
        let mut source = Module::new(FileId(0));
        source.items.push(make_load_symbols_item("utils", vec![import_item], Vec::new()));
        let source_scope = crate::scope::ModuleScope::new(ModuleId(0));
        symbol_table.add_scope(source_scope);

        let module_scopes = vec![(ModuleId(1), "utils".to_string())];
        let diags = resolve_imports(
            &source,
            ModuleId(0),
            &mut symbol_table,
            &registry,
            &module_scopes,
        );

        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("not public"));
    }

    #[test]
    fn test_module_not_found() {
        let registry = SymbolRegistry::new();
        let mut symbol_table = GlobalSymbolTable::new();

        let import_item = ImportSymbolItem {
            pattern: ImportPattern::Exact(Ident::new("something", dummy_span())),
            span: dummy_span(),
        };
        let mut source = Module::new(FileId(0));
        source.items.push(make_load_symbols_item("missing", vec![import_item], Vec::new()));
        let source_scope = crate::scope::ModuleScope::new(ModuleId(0));
        symbol_table.add_scope(source_scope);

        let module_scopes: Vec<(ModuleId, String)> = Vec::new();
        let diags = resolve_imports(
            &source,
            ModuleId(0),
            &mut symbol_table,
            &registry,
            &module_scopes,
        );

        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("not found"));
    }

    #[test]
    fn test_import_with_alias() {
        let mut registry = SymbolRegistry::new();
        let mut symbol_table = GlobalSymbolTable::new();

        let mut target = Module::new(FileId(1));
        target
            .items
            .push(make_fun_item("calc_minus", Visibility::Public));
        let (target_scope, _) = collect_module_symbols(&target, ModuleId(1), &mut registry);
        symbol_table.add_scope(target_scope);

        let import_item = ImportSymbolItem {
            pattern: ImportPattern::Exact(Ident::new("calc_minus", dummy_span())),
            span: dummy_span(),
        };
        let alias = Ident::new("cm", dummy_span());
        let mut source = Module::new(FileId(0));
        source.items.push(make_load_symbols_item("utils", vec![import_item], vec![alias]));
        let source_scope = crate::scope::ModuleScope::new(ModuleId(0));
        symbol_table.add_scope(source_scope);

        let module_scopes = vec![(ModuleId(1), "utils".to_string())];
        let diags = resolve_imports(
            &source,
            ModuleId(0),
            &mut symbol_table,
            &registry,
            &module_scopes,
        );

        assert!(diags.is_empty(), "diags: {diags:?}");
        assert!(symbol_table.resolve_in_module(ModuleId(0), "cm").is_some());
        assert!(
            symbol_table
                .resolve_in_module(ModuleId(0), "calc_minus")
                .is_none()
        );
    }

    #[test]
    fn test_load_namespace_import_registers_namespace() {
        let mut registry = SymbolRegistry::new();
        let mut symbol_table = GlobalSymbolTable::new();

        let mut target = Module::new(FileId(1));
        target
            .items
            .push(make_fun_item("print", Visibility::Public));
        let (target_scope, _) = collect_module_symbols(&target, ModuleId(1), &mut registry);
        symbol_table.add_scope(target_scope);

        let mut source = Module::new(FileId(0));
        source.items.push(make_load_namespace_item("std.io"));
        let source_scope = crate::scope::ModuleScope::new(ModuleId(0));
        symbol_table.add_scope(source_scope);

        let module_scopes = vec![(ModuleId(1), "std.io".to_string())];
        let diags = resolve_imports(
            &source,
            ModuleId(0),
            &mut symbol_table,
            &registry,
            &module_scopes,
        );

        assert!(diags.is_empty(), "diags: {diags:?}");
        let scope = symbol_table.get_scope(ModuleId(0)).unwrap();
        assert!(scope.get_namespace("std::io").is_some());
    }

    #[test]
    fn test_load_segmented_namespace_import() {
        let mut registry = SymbolRegistry::new();
        let mut symbol_table = GlobalSymbolTable::new();

        let mut target = Module::new(FileId(1));
        target
            .items
            .push(make_fun_item("print", Visibility::Public));
        let (target_scope, _) = collect_module_symbols(&target, ModuleId(1), &mut registry);
        symbol_table.add_scope(target_scope);

        let mut source = Module::new(FileId(0));
        let seg_item = Item::new(
            ItemId(0),
            Visibility::Local,
            ItemKind::Import(ImportDecl {
                path: ImportPath {
                    prefix: PathPrefix::Package(Ident::new("std", dummy_span())),
                    segments: vec![make_segment("io")],
                    span: dummy_span(),
                },
                symbols: None,
                span: dummy_span(),
            }),
            dummy_span(),
        );
        source.items.push(seg_item);
        let source_scope = crate::scope::ModuleScope::new(ModuleId(0));
        symbol_table.add_scope(source_scope);

        let module_scopes = vec![(ModuleId(1), "std.io".to_string())];
        let diags = resolve_imports(
            &source,
            ModuleId(0),
            &mut symbol_table,
            &registry,
            &module_scopes,
        );

        assert!(diags.is_empty(), "diags: {diags:?}");
        let scope = symbol_table.get_scope(ModuleId(0)).unwrap();
        assert!(scope.get_namespace("std::io").is_some());
    }

    #[test]
    fn test_alias_syntax_features() {
        let mut registry = SymbolRegistry::new();
        let mut symbol_table = GlobalSymbolTable::new();

        let mut target = Module::new(FileId(1));
        target
            .items
            .push(make_fun_item("println", Visibility::Public));
        let (target_scope, _) = collect_module_symbols(&target, ModuleId(1), &mut registry);
        symbol_table.add_scope(target_scope);

        let mut source = Module::new(FileId(0));
        // alias std::io as io
        source.items.push(Item::new(
            ItemId(0),
            Visibility::Local,
            ItemKind::Alias(AliasDecl {
                kind: AliasKind::Namespace(vec![Ident::new("std", dummy_span()), Ident::new("io", dummy_span())]),
                alias: Ident::new("io", dummy_span()),
                span: dummy_span(),
            }),
            dummy_span(),
        ));
        // alias std::io#println as prt
        source.items.push(Item::new(
            ItemId(1),
            Visibility::Local,
            ItemKind::Alias(AliasDecl {
                kind: AliasKind::Symbol {
                    namespace: vec![Ident::new("std", dummy_span()), Ident::new("io", dummy_span())],
                    symbol: Ident::new("println", dummy_span()),
                },
                alias: Ident::new("prt", dummy_span()),
                span: dummy_span(),
            }),
            dummy_span(),
        ));

        let source_scope = crate::scope::ModuleScope::new(ModuleId(0));
        symbol_table.add_scope(source_scope);

        let module_scopes = vec![(ModuleId(1), "std.io".to_string())];
        let diags = resolve_imports(
            &source,
            ModuleId(0),
            &mut symbol_table,
            &registry,
            &module_scopes,
        );

        assert!(diags.is_empty(), "diags: {diags:?}");
        let scope = symbol_table.get_scope(ModuleId(0)).unwrap();
        assert!(scope.get_namespace("io").is_some());
        assert!(symbol_table.resolve_in_module(ModuleId(0), "prt").is_some());
    }

    #[test]
    fn test_public_load_reexport() {
        let mut registry = SymbolRegistry::new();
        let mut symbol_table = GlobalSymbolTable::new();

        // Module 2: defines public fun foo()
        let mut mod2 = Module::new(FileId(2));
        mod2.items.push(make_fun_item("foo", Visibility::Public));
        let (scope2, _) = collect_module_symbols(&mod2, ModuleId(2), &mut registry);
        symbol_table.add_scope(scope2);

        // Module 1: public load mod2: *
        let mut mod1 = Module::new(FileId(1));
        let mut load_item = make_load_symbols_item("mod2", vec![ImportSymbolItem {
            pattern: ImportPattern::All,
            span: dummy_span(),
        }], Vec::new());
        load_item.visibility = Visibility::Public;
        mod1.items.push(load_item);
        let scope1 = crate::scope::ModuleScope::new(ModuleId(1));
        symbol_table.add_scope(scope1);

        // Module 0: load mod1: *
        let mut mod0 = Module::new(FileId(0));
        mod0.items.push(make_load_symbols_item("mod1", vec![ImportSymbolItem {
            pattern: ImportPattern::All,
            span: dummy_span(),
        }], Vec::new()));
        let scope0 = crate::scope::ModuleScope::new(ModuleId(0));
        symbol_table.add_scope(scope0);

        let module_scopes = vec![
            (ModuleId(0), "mod0".to_string()),
            (ModuleId(1), "mod1".to_string()),
            (ModuleId(2), "mod2".to_string()),
        ];

        let d1 = resolve_imports(&mod1, ModuleId(1), &mut symbol_table, &registry, &module_scopes);
        assert!(d1.is_empty(), "mod1 diags: {d1:?}");

        let d0 = resolve_imports(&mod0, ModuleId(0), &mut symbol_table, &registry, &module_scopes);
        assert!(d0.is_empty(), "mod0 diags: {d0:?}");

        assert!(symbol_table.resolve_in_module(ModuleId(0), "foo").is_some());
    }

    #[test]
    fn test_private_load_not_reexported() {
        let mut registry = SymbolRegistry::new();
        let mut symbol_table = GlobalSymbolTable::new();

        // Module 2: defines public fun secret()
        let mut mod2 = Module::new(FileId(2));
        mod2.items.push(make_fun_item("secret", Visibility::Public));
        let (scope2, _) = collect_module_symbols(&mod2, ModuleId(2), &mut registry);
        symbol_table.add_scope(scope2);

        // Module 1: load mod2: * (PRIVATE, default Visibility::Local)
        let mut mod1 = Module::new(FileId(1));
        let load_item = make_load_symbols_item("mod2", vec![ImportSymbolItem {
            pattern: ImportPattern::All,
            span: dummy_span(),
        }], Vec::new());
        mod1.items.push(load_item);
        let scope1 = crate::scope::ModuleScope::new(ModuleId(1));
        symbol_table.add_scope(scope1);

        // Module 0: load mod1: *
        let mut mod0 = Module::new(FileId(0));
        mod0.items.push(make_load_symbols_item("mod1", vec![ImportSymbolItem {
            pattern: ImportPattern::All,
            span: dummy_span(),
        }], Vec::new()));
        let scope0 = crate::scope::ModuleScope::new(ModuleId(0));
        symbol_table.add_scope(scope0);

        let module_scopes = vec![
            (ModuleId(0), "mod0".to_string()),
            (ModuleId(1), "mod1".to_string()),
            (ModuleId(2), "mod2".to_string()),
        ];

        let d1 = resolve_imports(&mod1, ModuleId(1), &mut symbol_table, &registry, &module_scopes);
        assert!(d1.is_empty(), "mod1 diags: {d1:?}");

        let d0 = resolve_imports(&mod0, ModuleId(0), &mut symbol_table, &registry, &module_scopes);
        assert!(d0.is_empty(), "mod0 diags: {d0:?}");

        // mod0 should NOT have secret because mod1 loaded it privately
        assert!(symbol_table.resolve_in_module(ModuleId(0), "secret").is_none());
    }
}
