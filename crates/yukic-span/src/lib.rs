#![deny(unsafe_code)]

pub mod file;
pub mod map;
pub mod pos;

pub use file::SourceFile;
pub use map::SourceMap;
pub use pos::{ByteOffset, BytePos, FileId, LineCol, Span};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_merge() {
        let s1 = Span::new(FileId(1), BytePos(10), BytePos(20));
        let s2 = Span::new(FileId(1), BytePos(15), BytePos(30));
        let merged = s1.merge(s2);
        assert_eq!(merged.start, BytePos(10));
        assert_eq!(merged.end, BytePos(30));
    }

    #[test]
    fn test_span_dummy() {
        let s1 = Span::DUMMY;
        assert!(s1.is_dummy());
        let s2 = Span::new(FileId(0), BytePos(5), BytePos(10));
        assert_eq!(s1.merge(s2), s2);
        assert_eq!(s2.merge(s1), s2);
    }

    #[test]
    fn test_span_properties() {
        let s = Span::new(FileId(0), BytePos(5), BytePos(12));
        assert_eq!(s.len(), 7);
        assert!(!s.is_empty());
        assert!(s.contains(BytePos(5)));
        assert!(s.contains(BytePos(8)));
        assert!(s.contains(BytePos(12)));
        assert!(!s.contains(BytePos(4)));
        assert!(!s.contains(BytePos(13)));
    }

    #[test]
    fn test_source_file_ascii() {
        let src = "set x = 42\nset y = 100\nset z = x + y".to_string();
        let file = SourceFile::new(FileId(0), "test.ykn".into(), src);

        // Line 1: 'set x = 42' -> pos 4 is 'x' -> line 1, col 5
        let pos_x = file.lookup_line_col(BytePos(4)).unwrap();
        assert_eq!(pos_x.line, 1);
        assert_eq!(pos_x.col, 5);

        // Line 2: starts at byte 11 ('s' in 'set y = 100') -> line 2, col 1
        let pos_l2 = file.lookup_line_col(BytePos(11)).unwrap();
        assert_eq!(pos_l2.line, 2);
        assert_eq!(pos_l2.col, 1);

        // Line content checks
        assert_eq!(file.line_content(1), Some("set x = 42"));
        assert_eq!(file.line_content(2), Some("set y = 100"));
        assert_eq!(file.line_content(3), Some("set z = x + y"));
        assert_eq!(file.line_content(4), None);
    }

    #[test]
    fn test_source_file_utf8_multibyte() {
        // "set сообщение = \"привет\";"
        // 'сообщение' has 9 cyrillic chars = 18 bytes
        let src = "set сообщение = \"привет\";\nвторая строка".to_string();
        let file = SourceFile::new(FileId(0), "utf8.ykn".into(), src);

        // First char of 'сообщение' is at byte offset 4
        let pos_c = file.lookup_line_col(BytePos(4)).unwrap();
        assert_eq!(pos_c.line, 1);
        assert_eq!(pos_c.col, 5);

        // Next char 'о' is at byte offset 6 -> col 6
        let pos_o = file.lookup_line_col(BytePos(6)).unwrap();
        assert_eq!(pos_o.line, 1);
        assert_eq!(pos_o.col, 6);

        // Second line check
        let line2_start = file.line_starts[1];
        let pos_l2 = file.lookup_line_col(line2_start).unwrap();
        assert_eq!(pos_l2.line, 2);
        assert_eq!(pos_l2.col, 1);
        assert_eq!(file.line_content(2), Some("вторая строка"));
    }

    #[test]
    fn test_source_file_crlf() {
        let src = "line1\r\nline2\r\nline3".to_string();
        let file = SourceFile::new(FileId(0), "crlf.ykn".into(), src);

        assert_eq!(file.line_content(1), Some("line1"));
        assert_eq!(file.line_content(2), Some("line2"));
        assert_eq!(file.line_content(3), Some("line3"));

        let pos_l2 = file.lookup_line_col(BytePos(7)).unwrap();
        assert_eq!(pos_l2.line, 2);
        assert_eq!(pos_l2.col, 1);
    }

    #[test]
    fn test_source_map_integration() {
        let mut map = SourceMap::new();
        let id0 = map.add_file("main.ykn", "fun main() {\n    exit(0)\n}".to_string());
        assert_eq!(id0, FileId(0));
        assert_eq!(map.files_count(), 1);

        let span = Span::new(id0, BytePos(17), BytePos(24));
        let (line, col) = map.lookup_line_column(span).unwrap();
        assert_eq!(line, 2);
        assert_eq!(col, 5);

        assert_eq!(map.slice_span(span), Some("exit(0)"));
        assert_eq!(map.line_content(id0, 2), Some("    exit(0)"));
    }

    #[test]
    fn test_out_of_bounds_safety() {
        let file = SourceFile::new(FileId(0), "empty.ykn".into(), String::new());
        assert_eq!(
            file.lookup_line_col(BytePos(0)),
            Some(LineCol { line: 1, col: 1 })
        );
        assert_eq!(file.lookup_line_col(BytePos(1)), None);
        assert_eq!(file.line_content(0), None);
        assert_eq!(file.line_content(2), None);
    }
}
