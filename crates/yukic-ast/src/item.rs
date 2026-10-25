#![deny(unsafe_code)]

use crate::attr::{AttributeNode, CAttributeNode};
use crate::expr::Expr;
use crate::ident::Ident;
use crate::stmt::Block;
use crate::types::{GenericParam, TypeNode};
use yukic_span::{FileId, Span};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ItemId(pub u32);

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub file_id: FileId,
    pub module_attrs: Vec<AttributeNode>,
    pub items: Vec<Item>,
}

impl Module {
    pub fn new(file_id: FileId) -> Self {
        Self {
            file_id,
            module_attrs: Vec::new(),
            items: Vec::new(),
        }
    }
}

impl Default for Module {
    fn default() -> Self {
        Self::new(FileId(0))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: ItemId,
    pub span: Span,
    pub visibility: Visibility,
    pub attrs: Vec<AttributeNode>,
    pub kind: ItemKind,
}

impl Item {
    pub fn new(id: ItemId, visibility: Visibility, kind: ItemKind, span: Span) -> Self {
        Self {
            id,
            span,
            visibility,
            attrs: Vec::new(),
            kind,
        }
    }

    pub fn with_attrs(
        id: ItemId,
        visibility: Visibility,
        attrs: Vec<AttributeNode>,
        kind: ItemKind,
        span: Span,
    ) -> Self {
        Self {
            id,
            span,
            visibility,
            attrs,
            kind,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Local,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ItemKind {
    Fun(FunDecl),
    Cfun(CfunDecl),
    Macro(MacroDefDecl),
    Class(ClassDecl),
    Struct(StructDecl),
    Cstruct(CstructDecl),
    Interface(InterfaceDecl),
    Enum(EnumDecl),
    TypeAlias(TypeAliasDecl),
    Import(ImportDecl),
    Alias(AliasDecl),
    Static(StaticDecl),
    Externc(ExterncBlock),
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunDecl {
    pub name: Ident,
    pub attrs: Vec<AttributeNode>,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeNode>,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CfunDecl {
    pub name: Ident,
    pub c_attrs: Vec<CAttributeNode>,
    pub yukino_attrs: Vec<AttributeNode>,
    pub params: Vec<CParam>,
    pub return_type: Option<TypeNode>,
    pub symbol_override: Option<Ident>,
    pub body: Option<Block>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: Ident,
    pub is_ref: bool,
    pub is_mut: bool,
    pub is_variadic: bool,
    pub ty: Option<TypeNode>,
    pub default_val: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CParam {
    pub name: Ident,
    pub ty: TypeNode,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MethodDecl {
    pub name: Ident,
    pub attrs: Vec<AttributeNode>,
    pub this_param: ThisParam,
    pub params: Vec<Param>,
    pub return_type: Option<TypeNode>,
    pub body: Block,
    pub is_operator: bool,
    pub is_override: bool,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThisParam {
    None,
    Value,
    Ref,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StaticMethodDecl {
    pub name: Ident,
    pub attrs: Vec<AttributeNode>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeNode>,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PropertyAccessor {
    pub name: Ident,
    pub attrs: Vec<AttributeNode>,
    pub ty: TypeNode,
    pub kind: AccessorKind,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AccessorKind {
    Get,
    Set(Ident),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExterncBlock {
    pub header: Option<String>,
    pub items: Vec<ExterncItem>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExterncItem {
    Cfun(CfunDecl),
    Cstruct(CstructDecl),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MacroDefDecl {
    pub name: Ident,
    pub params: Vec<MacroParam>,
    pub body: MacroBody,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MacroParam {
    pub name: Ident,
    pub kind: MacroParamKind,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MacroParamKind {
    Expr,
    Ident,
    Ty,
    Block,
    Variadic,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MacroBody {
    Block(Block),
    Expr(Box<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassDecl {
    pub name: Ident,
    pub is_locked: bool,
    pub base_class: Option<Ident>,
    pub interfaces: Vec<Ident>,
    pub fields: Vec<ClassField>,
    pub methods: Vec<MethodDecl>,
    pub accessors: Vec<PropertyAccessor>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassField {
    pub name: Ident,
    pub attrs: Vec<AttributeNode>,
    pub ty: TypeNode,
    pub is_pub: bool,
    pub default_val: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructDecl {
    pub name: Ident,
    pub fields: Vec<FieldDef>,
    pub methods: Vec<MethodDecl>,
    pub static_methods: Vec<StaticMethodDecl>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CstructDecl {
    pub name: Ident,
    pub c_attrs: Vec<CAttributeNode>,
    pub fields: Vec<FieldDef>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldDef {
    pub name: Ident,
    pub attrs: Vec<AttributeNode>,
    pub ty: TypeNode,
    pub default_val: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InterfaceDecl {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub methods: Vec<InterfaceMethodDef>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InterfaceMethodDef {
    pub name: Ident,
    pub attrs: Vec<AttributeNode>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeNode>,
    pub default_body: Option<Block>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumDecl {
    pub name: Ident,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EnumVariant {
    Simple(Ident),
    WithValue(Ident, Expr),
    Adt(Ident, Vec<FieldDef>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeAliasDecl {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub target: TypeNode,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StaticDecl {
    pub name: Ident,
    pub ty: Option<TypeNode>,
    pub init: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportDecl {
    pub path: ImportPath,
    pub symbols: Option<ImportSymbolsClause>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportSymbolsClause {
    pub items: Vec<ImportSymbolItem>,
    pub aliases: Vec<Ident>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportSymbolItem {
    pub pattern: ImportPattern,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImportPattern {
    Exact(Ident),
    All,
    Prefix(Ident),
    Suffix(Ident),
}

impl ImportPattern {
    pub fn matches(&self, name: &str) -> bool {
        match self {
            Self::All => true,
            Self::Exact(id) => id.as_str() == name,
            Self::Prefix(id) => name.starts_with(id.as_str()),
            Self::Suffix(id) => name.ends_with(id.as_str()),
        }
    }

    pub fn is_wildcard(&self) -> bool {
        !matches!(self, Self::Exact(_))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportPath {
    pub prefix: PathPrefix,
    pub segments: Vec<ImportPathSegment>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PathPrefix {
    // rel path: 0 = same dir, 1 = parent, 2 = grandparent, etc.
    Relative(usize),
    Root,
    Package(Ident),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportPathSegment {
    pub pattern: ImportPattern,
    // true if prefixed with ! to prior folder over file
    pub force_folder: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AliasDecl {
    pub kind: AliasKind,
    pub alias: Ident,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AliasKind {
    // alias std::io as io
    Namespace(Vec<Ident>),
    // alias std::io#println as println
    Symbol { namespace: Vec<Ident>, symbol: Ident, },
    // alias print as prt
    Direct(Ident),
}
