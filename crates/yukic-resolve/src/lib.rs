#![deny(unsafe_code)]

pub mod collector;
pub mod conditional;
pub mod graph;
pub mod module_id;
pub mod module_path;
pub mod monolith;
pub mod resolver;
pub mod scope;
pub mod symbol;

pub use collector::{collect_module_symbols, register_builtins};
pub use conditional::{ConditionalConfig, filter_module};
pub use graph::ModuleGraph;
pub use module_id::{ModuleId, ModulePath};
pub use module_path::{
    LibRoot, ResolvedPath, SearchPaths, build_target_name, find_matching_modules,
    find_module_file, find_module_file_for_mod, find_module_file_for_path,
    find_module_in_package, find_package_dir, resolve_import_path,
};
pub use monolith::build_monolithic_module;
pub use resolver::resolve_imports;
pub use scope::{GlobalSymbolTable, LexicalScope, ModuleScope, ScopeBinding, ScopeStack};
pub use symbol::{SymbolEntry, SymbolId, SymbolKind, SymbolRegistry};

use yukic_ast::Module;
use yukic_diagnostics::Diagnostic;

pub struct Resolver {
    pub registry: SymbolRegistry,
    pub symbol_table: GlobalSymbolTable,
    pub graph: ModuleGraph,
    module_names: Vec<String>,
    module_ids: Vec<(ModuleId, String)>,
}

impl Resolver {
    pub fn new() -> Self {
        let mut registry = SymbolRegistry::new();
        let mut symbol_table = GlobalSymbolTable::new();

        let builtins = register_builtins(&mut registry);
        for (name, id) in builtins {
            symbol_table.register_builtin(name, id);
        }

        Self {
            registry,
            symbol_table,
            graph: ModuleGraph::new(),
            module_names: Vec::new(),
            module_ids: Vec::new(),
        }
    }

    pub fn add_module(&mut self, name: String, module: &Module) -> (ModuleId, Vec<Diagnostic>) {
        let module_id = self.graph.add_node();
        self.module_names.push(name.clone());
        self.module_ids.push((module_id, name));

        let (scope, diagnostics) = collect_module_symbols(module, module_id, &mut self.registry);
        self.symbol_table.add_scope(scope);

        (module_id, diagnostics)
    }

    // adds an import edge in the dependency graph
    pub fn add_dependency(&mut self, from: ModuleId, to: ModuleId, span: yukic_span::Span) {
        self.graph.add_edge(from, to, span);
    }

    // runs cycle detection on the module graph
    pub fn check_cycles(&self) -> Vec<Diagnostic> {
        self.graph.detect_cycles(&self.module_names)
    }

    // resolves all imports in a module against the symbol table
    pub fn resolve_module_imports(
        &mut self,
        module: &Module,
        module_id: ModuleId,
    ) -> Vec<Diagnostic> {
        resolve_imports(
            module,
            module_id,
            &mut self.symbol_table,
            &self.registry,
            &self.module_ids,
        )
    }

    // full resolution pass: check cycles + resolve all module imports in topological order
    pub fn resolve_all(&mut self, modules: &[(ModuleId, Module)]) -> Vec<Diagnostic> {
        let mut diagnostics = self.check_cycles();

        let order = self.graph.topological_order().unwrap_or_else(|| {
            modules.iter().map(|(id, _)| *id).collect()
        });

        let mut mod_map: std::collections::HashMap<ModuleId, &Module> =
            modules.iter().map(|(id, m)| (*id, m)).collect();

        for module_id in order {
            if let Some(module) = mod_map.remove(&module_id) {
                let import_diags = self.resolve_module_imports(module, module_id);
                diagnostics.extend(import_diags);
            }
        }

        // any remaining modules not in topological sort (e.g. in case of cycles)
        for (module_id, module) in modules {
            if mod_map.contains_key(module_id) {
                let import_diags = self.resolve_module_imports(module, *module_id);
                diagnostics.extend(import_diags);
            }
        }

        diagnostics
    }

    pub fn module_ids(&self) -> &[(ModuleId, String)] {
        &self.module_ids
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn test_resolver_builtins_available() {
        let resolver = Resolver::new();
        assert!(resolver.symbol_table.get_builtin("exit").is_some());
        assert!(resolver.symbol_table.get_builtin("typeof").is_some());
        assert!(resolver.symbol_table.get_builtin("cast").is_some());
        assert!(resolver.symbol_table.get_builtin("args").is_some());
        assert!(resolver.symbol_table.get_builtin("raw_args").is_some());
        assert!(resolver.symbol_table.get_builtin("panic").is_some());
        assert!(resolver.symbol_table.get_builtin("assert").is_some());
        assert!(resolver.symbol_table.get_builtin("assert_eq").is_some());
        assert!(resolver.symbol_table.get_builtin("assert_ne").is_some());
    }

    #[test]
    fn test_resolver_args_and_raw_args_resolution() {
        let resolver = Resolver::new();

        let args_sym = resolver
            .symbol_table
            .resolve_in_module(ModuleId(0), "args")
            .expect("args should resolve in module");
        let args_entry = resolver
            .registry
            .get(args_sym)
            .expect("symbol entry for args must exist");
        assert_eq!(args_entry.name, "args");
        assert_eq!(args_entry.kind, SymbolKind::Builtin);
        assert_eq!(args_entry.visibility, Visibility::Public);

        let raw_args_sym = resolver
            .symbol_table
            .resolve_in_module(ModuleId(0), "raw_args")
            .expect("raw_args should resolve in module");
        let raw_args_entry = resolver
            .registry
            .get(raw_args_sym)
            .expect("symbol entry for raw_args must exist");
        assert_eq!(raw_args_entry.name, "raw_args");
        assert_eq!(raw_args_entry.kind, SymbolKind::Builtin);
        assert_eq!(raw_args_entry.visibility, Visibility::Public);
    }

    #[test]
    fn test_resolver_builtin_shadowing() {
        let mut resolver = Resolver::new();
        let mut module = Module::new(FileId(0));
        module.items.push(make_fun_item("args", Visibility::Local));

        let (mod_id, diags) = resolver.add_module("test_mod".into(), &module);
        assert!(diags.is_empty());

        let resolved_id = resolver
            .symbol_table
            .resolve_in_module(mod_id, "args")
            .expect("args should resolve to local symbol");
        let entry = resolver.registry.get(resolved_id).unwrap();
        assert_eq!(entry.kind, SymbolKind::Fun);
        assert_ne!(
            resolved_id,
            resolver.symbol_table.get_builtin("args").unwrap()
        );
    }

    #[test]
    fn test_resolver_add_module() {
        let mut resolver = Resolver::new();
        let mut module = Module::new(FileId(0));
        module.items.push(make_fun_item("main", Visibility::Public));

        let (id, diags) = resolver.add_module("main".into(), &module);
        assert!(diags.is_empty());
        assert!(
            resolver
                .symbol_table
                .resolve_in_module(id, "main")
                .is_some()
        );
    }

    #[test]
    fn test_resolver_cross_module_import() {
        let mut resolver = Resolver::new();

        // module "utils" with public fun calc_plus
        let mut utils = Module::new(FileId(1));
        utils
            .items
            .push(make_fun_item("calc_plus", Visibility::Public));
        let (utils_id, _) = resolver.add_module("utils".into(), &utils);

        // module "main" that imports calc_plus from utils
        let import = Item::new(
            ItemId(0),
            Visibility::Local,
            ItemKind::Import(ImportDecl {
                path: ImportPath {
                    prefix: PathPrefix::Package(Ident::new("utils", dummy_span())),
                    segments: Vec::new(),
                    span: dummy_span(),
                },
                symbols: Some(yukic_ast::ImportSymbolsClause {
                    items: vec![yukic_ast::ImportSymbolItem {
                        pattern: yukic_ast::ImportPattern::Exact(Ident::new("calc_plus", dummy_span())),
                        span: dummy_span(),
                    }],
                    aliases: Vec::new(),
                    span: dummy_span(),
                }),
                span: dummy_span(),
            }),
            dummy_span(),
        );
        let mut main_mod = Module::new(FileId(0));
        main_mod.items.push(import);
        let (main_id, _) = resolver.add_module("main".into(), &main_mod);
        resolver.add_dependency(main_id, utils_id, dummy_span());

        let diags = resolver.resolve_module_imports(&main_mod, main_id);
        assert!(diags.is_empty());
        assert!(
            resolver
                .symbol_table
                .resolve_in_module(main_id, "calc_plus")
                .is_some()
        );
    }

    #[test]
    fn test_resolver_cycle_detection() {
        let mut resolver = Resolver::new();

        let mod_a = Module::new(FileId(0));
        let mod_b = Module::new(FileId(1));
        let (id_a, _) = resolver.add_module("a".into(), &mod_a);
        let (id_b, _) = resolver.add_module("b".into(), &mod_b);

        resolver.add_dependency(id_a, id_b, dummy_span());
        resolver.add_dependency(id_b, id_a, dummy_span());

        let diags = resolver.check_cycles();
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("circular"));
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn test_resolve_types_are_send_and_sync() {
        assert_send_sync::<GlobalSymbolTable>();
        assert_send_sync::<ModuleScope>();
        assert_send_sync::<SymbolRegistry>();
        assert_send_sync::<SymbolEntry>();
        assert_send_sync::<ModuleGraph>();
        assert_send_sync::<Resolver>();
    }
}
