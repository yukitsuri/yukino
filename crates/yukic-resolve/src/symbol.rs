use std::fmt;

use yukic_ast::Visibility;
use yukic_span::Span;

use crate::module_id::ModuleId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymbolId(pub u32);

impl fmt::Display for SymbolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sym#{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Fun,
    Cfun,
    Class,
    Struct,
    Cstruct,
    Interface,
    Enum,
    TypeAlias,
    Static,
    Macro,
    Namespace,
    Builtin,
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Fun => "function",
            Self::Cfun => "cfun",
            Self::Class => "class",
            Self::Struct => "struct",
            Self::Cstruct => "cstruct",
            Self::Interface => "interface",
            Self::Enum => "enum",
            Self::TypeAlias => "type alias",
            Self::Static => "static",
            Self::Macro => "macro",
            Self::Namespace => "namespace",
            Self::Builtin => "built-in",
        };
        write!(f, "{name}")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolEntry {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub visibility: Visibility,
    pub module_id: ModuleId,
    pub span: Span,
}

impl SymbolEntry {
    pub fn is_public(&self) -> bool {
        self.visibility == Visibility::Public
    }
}

#[derive(Debug, Default)]
pub struct SymbolRegistry {
    entries: Vec<SymbolEntry>,
}

impl SymbolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        name: String,
        kind: SymbolKind,
        visibility: Visibility,
        module_id: ModuleId,
        span: Span,
    ) -> SymbolId {
        let id = SymbolId(u32::try_from(self.entries.len()).unwrap_or(u32::MAX));
        self.entries.push(SymbolEntry {
            id,
            name,
            kind,
            visibility,
            module_id,
            span,
        });
        id
    }

    pub fn get(&self, id: SymbolId) -> Option<&SymbolEntry> {
        self.entries.get(id.0 as usize)
    }

    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
}
