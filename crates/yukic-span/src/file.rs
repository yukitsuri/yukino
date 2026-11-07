use std::path::PathBuf;

use crate::pos::{BytePos, FileId, LineCol, Span};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    pub id: FileId,
    pub path: PathBuf,
    pub src: String,
    pub line_starts: Vec<BytePos>,
}

impl SourceFile {
    pub fn new(id: FileId, path: PathBuf, src: String) -> Self {
        let line_starts = compute_line_starts(&src);
        Self {
            id,
            path,
            src,
            line_starts,
        }
    }

    pub fn lookup_line_col(&self, pos: BytePos) -> Option<LineCol> {
        let offset = pos.0 as usize;
        if offset > self.src.len() { return None; }
        let line_idx = match self.line_starts.binary_search(&pos) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };
        let line_start = self.line_starts[line_idx].0 as usize;
        let line_slice = self.src.get(line_start..offset)?;
        let col = line_slice.chars().count() + 1;
        Some(LineCol { line: line_idx + 1, col, })
    }

    pub fn line_content(&self, line_1_based: usize) -> Option<&str> {
        if line_1_based == 0 || line_1_based > self.line_starts.len() { return None; }
        let start = self.line_starts[line_1_based - 1].0 as usize;
        let end = if line_1_based < self.line_starts.len() {
            self.line_starts[line_1_based].0 as usize
        } else { self.src.len() };
        let line = self.src.get(start..end)?;
        Some(line.trim_end_matches(['\r', '\n']))
    }

    pub fn slice_span(&self, span: Span) -> Option<&str> {
        if span.file_id != self.id { return None; }
        let start = span.start.0 as usize;
        let end = span.end.0 as usize;
        if start > end || end > self.src.len() { return None; }
        self.src.get(start..end)
    }
}

fn compute_line_starts(src: &str) -> Vec<BytePos> {
    let mut starts = vec![BytePos(0)];
    for (i, byte) in src.bytes().enumerate() {
        if byte == b'\n' { starts.push(BytePos(u32::try_from(i + 1).unwrap_or(u32::MAX))); }
    }
    starts
}
