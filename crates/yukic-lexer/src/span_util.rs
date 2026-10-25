#![deny(unsafe_code)]

use yukic_span::{BytePos, FileId, Span};

#[inline]
pub fn make_span(file_id: FileId, start: usize, end: usize) -> Span {
    Span::new(
        file_id,
        BytePos(u32::try_from(start).unwrap_or(u32::MAX)),
        BytePos(u32::try_from(end).unwrap_or(u32::MAX)),
    )
}
