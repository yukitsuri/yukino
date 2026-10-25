use std::path::Path;

use crate::file::SourceFile;
use crate::pos::{FileId, LineCol, Span};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self { Self::default() }

    pub fn add_file(&mut self, path: impl AsRef<Path>, src: String) -> FileId {
        let id = FileId(u32::try_from(self.files.len()).unwrap_or(u32::MAX));
        self.files.push(SourceFile::new(id, path.as_ref().to_path_buf(), src));
        id
    }

    pub fn get_file(&self, id: FileId) -> Option<&SourceFile> { self.files.get(id.0 as usize) }

    pub fn lookup_line_col(&self, span: Span) -> Option<LineCol> {
        self.get_file(span.file_id)
            .and_then(|f| f.lookup_line_col(span.start))
    }

    pub fn lookup_line_column(&self, span: Span) -> Option<(usize, usize)> {
        self.lookup_line_col(span).map(|lc| (lc.line, lc.col))
    }

    pub fn slice_span(&self, span: Span) -> Option<&str> {
        self.get_file(span.file_id).and_then(|f| f.slice_span(span))
    }

    pub fn line_content(&self, file_id: FileId, line: usize) -> Option<&str> {
        self.get_file(file_id).and_then(|f| f.line_content(line))
    }

    pub fn files_count(&self) -> usize {
        self.files.len()
    }
}
