#![deny(unsafe_code)]
#![allow(clippy::result_large_err)]

pub mod builtin;
pub mod expand;
pub mod hygiene;
pub mod runner;
pub mod subst;
pub mod table;

pub use builtin::{BuiltinMacroConfig, eval_builtin_macro, is_builtin_macro};
pub use expand::AstExpander;
pub use hygiene::{apply_hygiene, gensym};
pub use runner::MacroRunnerClient;
pub use subst::substitute_macro;
pub use table::{MacroScope, MacroTable};

use yukic_ast::{ItemKind, Module};
use yukic_diagnostics::Diagnostic;
use yukic_span::SourceMap;

/// main macro expnd for yuki src files
pub struct MacroExpander {
    pub recursion_limit: usize,
    pub config: BuiltinMacroConfig,
}

impl MacroExpander {
    pub fn new() -> Self {
        Self {
            recursion_limit: 128,
            config: BuiltinMacroConfig::default(),
        }
    }

    pub fn with_config(config: BuiltinMacroConfig) -> Self {
        Self {
            recursion_limit: 128,
            config,
        }
    }

    #[must_use]
    pub fn with_recursion_limit(mut self, limit: usize) -> Self {
        self.recursion_limit = limit;
        self
    }

    /// expands all macro defs and macro calls within a single module
    pub fn expand_module(
        &self,
        module: &mut Module,
        source_map: Option<&SourceMap>,
    ) -> Result<(), Vec<Diagnostic>> {
        let mut expander = AstExpander::new(source_map, self.config.clone(), self.recursion_limit);
        expander.expand_module(module);

        if expander.diagnostics.is_empty() {
            Ok(())
        } else {
            Err(expander.diagnostics)
        }
    }

    pub fn expand_module_tracked(
        &self,
        module: &mut Module,
        source_map: Option<&SourceMap>,
    ) -> (Result<(), Vec<Diagnostic>>, Vec<String>) {
        let mut expander = AstExpander::new(source_map, self.config.clone(), self.recursion_limit);
        expander.expand_module(module);
        let res = if expander.diagnostics.is_empty() {
            Ok(())
        } else {
            Err(expander.diagnostics)
        };
        (res, expander.tracked_files)
    }

    pub fn expand_project<I>(
        &self,
        modules: &mut [(I, Module)],
        module_names: &[String],
        source_map: Option<&SourceMap>,
    ) -> Result<(), Vec<Diagnostic>> {
        let mut expander = AstExpander::new(source_map, self.config.clone(), self.recursion_limit);

        for ((_, module), name) in modules.iter().zip(module_names) {
            for item in &module.items {
                if let ItemKind::Macro(decl) = &item.kind {
                    expander.table.register_qualified(
                        name.clone(),
                        decl.name.name.clone(),
                        decl.clone(),
                    );
                    expander
                        .table
                        .register(decl.name.name.clone(), decl.clone());
                }
            }
        }

        let mut all_diagnostics = Vec::new();
        for (_, module) in modules.iter_mut() {
            expander.expand_module(module);
        }
        all_diagnostics.extend(expander.diagnostics);

        if all_diagnostics.is_empty() { Ok(()) } 
        else { Err(all_diagnostics) }
    }
}

impl Default for MacroExpander {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_ast::*;
    use yukic_span::{BytePos, FileId, Span};

    fn dummy_span() -> Span {
        Span::new(FileId(0), BytePos(0), BytePos(10))
    }

    #[test]
    fn test_macro_expander_init() {
        let expander = MacroExpander::new();
        assert_eq!(expander.recursion_limit, 128);
    }

    #[test]
    fn test_expand_module_with_define_and_call() {
        let mut module = Module::new(FileId(0));
        let def = MacroDefDecl {
            name: Ident::new("double", dummy_span()),
            params: vec![MacroParam {
                name: Ident::new("n", dummy_span()),
                kind: MacroParamKind::Expr,
                span: dummy_span(),
            }],
            body: MacroBody::Expr(Box::new(Expr::new(
                ExprKind::Binary(
                    BinaryOp::Add,
                    Box::new(Expr::new(
                        ExprKind::Ident(Ident::new("n", dummy_span())),
                        dummy_span(),
                    )),
                    Box::new(Expr::new(
                        ExprKind::Ident(Ident::new("n", dummy_span())),
                        dummy_span(),
                    )),
                ),
                dummy_span(),
            ))),
            span: dummy_span(),
        };
        module.items.push(Item::new(
            ItemId(0),
            Visibility::Local,
            ItemKind::Macro(def),
            dummy_span(),
        ));

        let call = Expr::new(
            ExprKind::MacroCall(MacroCallNode {
                path: vec![Ident::new("double", dummy_span())],
                delimiter: MacroDelimiter::Paren,
                args: vec![MacroArg::Expr(Expr::new(
                    ExprKind::Literal(LiteralValue::Int(10)),
                    dummy_span(),
                ))],
                span: dummy_span(),
            }),
            dummy_span(),
        );

        module.items.push(Item::new(
            ItemId(1),
            Visibility::Public,
            ItemKind::Static(StaticDecl {
                name: Ident::new("X", dummy_span()),
                ty: None,
                init: call,
                span: dummy_span(),
            }),
            dummy_span(),
        ));

        let expander = MacroExpander::new();
        let res = expander.expand_module(&mut module, None);
        assert!(res.is_ok());

        if let ItemKind::Static(s) = &module.items[1].kind {
            if let ExprKind::Binary(BinaryOp::Add, lhs, rhs) = &s.init.kind {
                assert_eq!(lhs.kind, ExprKind::Literal(LiteralValue::Int(10)));
                assert_eq!(rhs.kind, ExprKind::Literal(LiteralValue::Int(10)));
            } else {
                panic!("expected binary add expression after expansion");
            }
        }
    }
}
