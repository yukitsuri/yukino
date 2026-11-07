use std::fmt;

use yukic_span::FileId;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleId(pub u32);

impl ModuleId { pub const ROOT: Self = Self(0); }

impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mod#{}", self.0)
    }
}

impl From<FileId> for ModuleId { fn from(id: FileId) -> Self { Self(id.0) } }

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModulePath { pub segments: Vec<String>, }

impl ModulePath {
    pub fn new(segments: Vec<String>) -> Self {
        Self { segments }
    }

    pub fn root() -> Self {
        Self { segments: Vec::new(), }
    }

    pub fn push(&mut self, segment: impl Into<String>) {
        self.segments.push(segment.into());
    }

    pub fn is_empty(&self) -> bool { self.segments.is_empty() }
    pub fn as_dotted(&self) -> String { self.segments.join(".") }
}

impl fmt::Display for ModulePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.segments.is_empty() {
            write!(f, "<root>")
        } else {
            write!(f, "{}", self.segments.join("."))
        }
    }
}
