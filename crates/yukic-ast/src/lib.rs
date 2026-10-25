#![deny(unsafe_code)]

pub mod aliases;
pub mod attr;
pub mod defaults;
pub mod expr;
pub mod ident;
pub mod item;
pub mod module_attrs;
pub mod stmt;
pub mod types;

pub use aliases::{AliasMap, substitute_module_aliases};
pub use defaults::{collect_module_defaults, expand_default_args};
pub use module_attrs::propagate_module_attributes;

pub use attr::{AttributeArgs, AttributeNode, AttributePath, CAttributeNode};
pub use expr::{
    BinaryOp, CatchClause, Expr, ExprKind, FieldInit, LiteralValue, MacroArg, MacroCallNode,
    MacroDelimiter, Pattern, PatternKind, StringPart, SwitchArm, TokenTree, UnaryOp,
};
pub use ident::Ident;
pub use item::{
    AccessorKind, AliasDecl, AliasKind, CParam, CfunDecl, ClassDecl,
    ClassField, CstructDecl, EnumDecl, EnumVariant, ExterncBlock, ExterncItem, FieldDef, FunDecl,
    ImportDecl, ImportPath, ImportPathSegment, ImportPattern, ImportSymbolItem,
    ImportSymbolsClause, InterfaceDecl, InterfaceMethodDef, Item,
    ItemId, ItemKind, MacroBody, MacroDefDecl, MacroParam, MacroParamKind, MethodDecl, Module,
    Param, PathPrefix, PropertyAccessor, StaticDecl, StaticMethodDecl, StructDecl, ThisParam,
    TypeAliasDecl, Visibility,
};
pub use stmt::{
    ArrayDestructElement, AssignOp, BindingTarget, Block, LValue, MapDestructField, Stmt, StmtKind,
};
pub use types::{GenericParam, TypeNode, TypeNodeKind};

#[cfg(test)]
mod tests {
    use super::*;
    use yukic_span::{BytePos, FileId, Span};

    #[test]
    fn test_ast_node_creation() {
        let span = Span::new(FileId(0), BytePos(0), BytePos(5));
        let ident = Ident::new("hello", span);
        assert_eq!(ident.as_str(), "hello");

        let expr = Expr::new(ExprKind::Literal(LiteralValue::Int(42)), span);
        assert_eq!(expr.kind, ExprKind::Literal(LiteralValue::Int(42)));

        let stmt = Stmt::new(StmtKind::Expr(expr), span);
        let block = Block::new(vec![stmt], span);
        assert_eq!(block.stmts.len(), 1);

        let module = Module::new(FileId(0));
        assert!(module.items.is_empty());
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn test_ast_nodes_are_send_and_sync() {
        assert_send_sync::<Module>();
        assert_send_sync::<Item>();
        assert_send_sync::<FunDecl>();
        assert_send_sync::<CfunDecl>();
        assert_send_sync::<ClassDecl>();
        assert_send_sync::<StructDecl>();
        assert_send_sync::<Expr>();
        assert_send_sync::<Stmt>();
        assert_send_sync::<Block>();
        assert_send_sync::<Ident>();
    }
}
