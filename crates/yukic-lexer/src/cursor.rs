#![deny(unsafe_code)]

pub struct Cursor<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
        }
    }

    pub fn pos(&self) -> usize { self.pos }
    pub fn is_eof(&self) -> bool { self.pos >= self.bytes.len() }
    pub fn peek(&self) -> Option<u8> { self.bytes.get(self.pos).copied() }
    pub fn peek_at(&self, offset: usize) -> Option<u8> { self.bytes.get(self.pos + offset).copied() }
    pub fn peek_char(&self) -> Option<char> { self.src[self.pos..].chars().next() }

    pub fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    pub fn bump_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    pub fn eat_if(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) { self.pos += 1; true} 
        else { false }
    }

    pub fn slice(&self, start: usize, end: usize) -> &'a str {
        &self.src[start..end]
    }
}
