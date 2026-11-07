#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct BytePos(pub u32);

pub type ByteOffset = BytePos;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LineCol { pub line: usize, pub col: usize, }

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    pub file_id: FileId,
    pub start: BytePos,
    pub end: BytePos,
}

impl Span {
    pub const DUMMY: Self = Self {
        file_id: FileId(u32::MAX),
        start: BytePos(0),
        end: BytePos(0),
    };

    pub fn new(file_id: FileId, start: BytePos, end: BytePos) -> Self { Self { file_id, start, end, } }

    pub fn is_dummy(&self) -> bool { self.file_id == FileId(u32::MAX) }

    pub fn len(&self) -> u32 {
        self.end.0.saturating_sub(self.start.0)
    }

    pub fn is_empty(&self) -> bool { self.start >= self.end }

    pub fn contains(&self, pos: BytePos) -> bool {
        self.start <= pos && pos <= self.end
    }

    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        if self.is_dummy() { return other; }
        if other.is_dummy() { return self; }
        debug_assert_eq!(
            self.file_id, other.file_id,
            "cannot merge spans from different files"
        );
        Self {
            file_id: self.file_id,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}
