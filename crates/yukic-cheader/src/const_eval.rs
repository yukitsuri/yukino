#![deny(unsafe_code)]

use std::collections::HashMap;
use yukic_target::{DataModel, TargetTriple};

#[derive(Clone, Debug, PartialEq)]
enum CTok {
    Int(i128),
    Ident(String),
    LParen,
    RParen,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Tilde,
    Bang,
    LtLt,
    GtGt,
    Lt,
    LtEq,
    Gt,
    GtEq,
    EqEq,
    BangEq,
    Amp,
    Caret,
    Pipe,
    AmpAmp,
    PipePipe,
    Question,
    Colon,
    Comma,
}

pub fn eval_preproc_if_expr(
    raw: &str,
    defined_macros: &dyn Fn(&str) -> bool,
    target: TargetTriple,
) -> bool {
    let tokens = tokenize_c_expr(raw, defined_macros, None, true, target);
    let mut parser = CExprParser {
        tokens: &tokens,
        pos: 0,
        allow_unknown_as_zero: true,
        consts: None,
    };
    parser.parse_expr().is_some_and(|v| v != 0)
}

pub fn eval_c_int_expr(
    raw: &str,
    consts: &HashMap<String, i128>,
    target: TargetTriple,
) -> Option<i128> {
    let tokens = tokenize_c_expr(raw, &|_| false, Some(consts), false, target);
    if tokens.is_empty() {
        return None;
    }
    let mut parser = CExprParser {
        tokens: &tokens,
        pos: 0,
        allow_unknown_as_zero: false,
        consts: Some(consts),
    };
    let val = parser.parse_expr()?;
    if parser.pos < tokens.len() {
        return None;
    }
    Some(val)
}

pub fn c_sizeof_basic(type_str: &str, target: TargetTriple) -> Option<i128> {
    let norm: String = type_str.split_whitespace().collect::<Vec<_>>().join(" ");
    if norm.ends_with('*') {
        return Some(i128::from(target.spec().pointer_width / 8));
    }
    let spec = target.spec();
    let long_bytes = match spec.data_model {
        DataModel::Lp64 => 8,
        DataModel::Llp64 | DataModel::Ilp32 => 4,
    };
    let ptr_bytes = i128::from(spec.pointer_width / 8);
    match norm.as_str() {
        "char" | "signed char" | "unsigned char" | "int8_t" | "uint8_t" | "_Bool" | "bool" => Some(1),
        "short" | "short int" | "signed short" | "unsigned short" | "unsigned short int" | "int16_t" | "uint16_t" => Some(2),
        "int" | "signed" | "signed int" | "unsigned" | "unsigned int" | "int32_t" | "uint32_t" | "float" => Some(4),
        "long" | "long int" | "signed long" | "unsigned long" | "unsigned long int" => Some(long_bytes),
        "long long" | "long long int" | "signed long long" | "unsigned long long" | "int64_t" | "uint64_t" | "double" => Some(8),
        "size_t" | "ssize_t" | "intptr_t" | "uintptr_t" | "ptrdiff_t" | "void*" => Some(ptr_bytes),
        _ => None,
    }
}

fn tokenize_c_expr(
    raw: &str,
    defined_macros: &dyn Fn(&str) -> bool,
    consts: Option<&HashMap<String, i128>>,
    preproc_mode: bool,
    target: TargetTriple,
) -> Vec<CTok> {
    let bytes = raw.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if ch.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if ch == '\'' {
            let (val, next_i) = parse_char_literal(raw, i);
            if let Some(v) = val {
                out.push(CTok::Int(v));
                i = next_i;
                continue;
            }
            out.push(CTok::Ident("'".to_string()));
            i += 1;
            continue;
        }
        if ch.is_ascii_digit() {
            let start = i;
            if ch == '0' && i + 1 < bytes.len() && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X') {
                i += 2;
                while i < bytes.len() && (bytes[i] as char).is_ascii_hexdigit() {
                    i += 1;
                }
            } else if ch == '0' && i + 1 < bytes.len() && (bytes[i + 1] == b'b' || bytes[i + 1] == b'B') {
                i += 2;
                while i < bytes.len() && (bytes[i] == b'0' || bytes[i] == b'1') {
                    i += 1;
                }
            } else {
                while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                    i += 1;
                }
                if i < bytes.len() && (bytes[i] == b'.' || bytes[i] == b'e' || bytes[i] == b'E') {
                    // float literal is not a valid integer constant expression
                    out.push(CTok::Ident(raw[start..i].to_string()));
                    while i < bytes.len() && !(bytes[i] as char).is_ascii_whitespace() && bytes[i] != b')' {
                        i += 1;
                    }
                    continue;
                }
            }
            let num_slice = &raw[start..i];
            while i < bytes.len() && matches!(bytes[i] as char, 'u' | 'U' | 'l' | 'L') {
                i += 1;
            }
            if let Some(val) = parse_int_digits(num_slice) {
                out.push(CTok::Int(val));
            } else {
                out.push(CTok::Ident(num_slice.to_string()));
            }
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = i;
            i += 1;
            while i < bytes.len() && ((bytes[i] as char).is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let ident = &raw[start..i];
            if preproc_mode && ident == "defined" {
                while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
                    i += 1;
                }
                let has_paren = i < bytes.len() && bytes[i] == b'(';
                if has_paren {
                    i += 1;
                    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
                        i += 1;
                    }
                }
                let m_start = i;
                while i < bytes.len() && ((bytes[i] as char).is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let macro_name = &raw[m_start..i];
                if has_paren {
                    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
                        i += 1;
                    }
                    if i < bytes.len() && bytes[i] == b')' {
                        i += 1;
                    }
                }
                out.push(CTok::Int(i128::from(defined_macros(macro_name))));
                continue;
            }
            if ident == "sizeof" {
                while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
                    i += 1;
                }
                if i < bytes.len() && bytes[i] == b'(' {
                    let paren_start = i + 1;
                    let mut depth = 1;
                    i += 1;
                    while i < bytes.len() && depth > 0 {
                        if bytes[i] == b'(' {
                            depth += 1;
                        } else if bytes[i] == b')' {
                            depth -= 1;
                        }
                        i += 1;
                    }
                    let inner = &raw[paren_start..i.saturating_sub(1)];
                    if let Some(sz) = c_sizeof_basic(inner, target) {
                        out.push(CTok::Int(sz));
                        continue;
                    }
                }
                out.push(CTok::Ident(ident.to_string()));
                continue;
            }
            if ident == "true" {
                out.push(CTok::Int(1));
            } else if ident == "false" {
                out.push(CTok::Int(0));
            } else if let Some(v) = consts.and_then(|c| c.get(ident)) {
                out.push(CTok::Int(*v));
            } else {
                out.push(CTok::Ident(ident.to_string()));
            }
            continue;
        }
        let two = raw.get(i..i + 2).unwrap_or("");
        match two {
            "<<" => {
                out.push(CTok::LtLt);
                i += 2;
                continue;
            }
            ">>" => {
                out.push(CTok::GtGt);
                i += 2;
                continue;
            }
            "<=" => {
                out.push(CTok::LtEq);
                i += 2;
                continue;
            }
            ">=" => {
                out.push(CTok::GtEq);
                i += 2;
                continue;
            }
            "==" => {
                out.push(CTok::EqEq);
                i += 2;
                continue;
            }
            "!=" => {
                out.push(CTok::BangEq);
                i += 2;
                continue;
            }
            "&&" => {
                out.push(CTok::AmpAmp);
                i += 2;
                continue;
            }
            "||" => {
                out.push(CTok::PipePipe);
                i += 2;
                continue;
            }
            _ => {}
        }
        let tok = match ch {
            '(' => CTok::LParen,
            ')' => CTok::RParen,
            '+' => CTok::Plus,
            '-' => CTok::Minus,
            '*' => CTok::Star,
            '/' => CTok::Slash,
            '%' => CTok::Percent,
            '~' => CTok::Tilde,
            '!' => CTok::Bang,
            '<' => CTok::Lt,
            '>' => CTok::Gt,
            '&' => CTok::Amp,
            '^' => CTok::Caret,
            '|' => CTok::Pipe,
            '?' => CTok::Question,
            ':' => CTok::Colon,
            ',' => CTok::Comma,
            _ => {
                out.push(CTok::Ident(ch.to_string()));
                i += 1;
                continue;
            }
        };
        out.push(tok);
        i += 1;
    }
    out
}

fn parse_int_digits(s: &str) -> Option<i128> {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        i128::from_str_radix(hex, 16).ok()
    } else if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        i128::from_str_radix(bin, 2).ok()
    } else if s.len() > 1 && s.starts_with('0') {
        i128::from_str_radix(&s[1..], 8).ok()
    } else {
        s.parse::<i128>().ok()
    }
}

fn parse_char_literal(raw: &str, start: usize) -> (Option<i128>, usize) {
    let bytes = raw.as_bytes();
    let mut i = start + 1;
    if i >= bytes.len() {
        return (None, i);
    }
    let ch_val = if bytes[i] == b'\\' && i + 1 < bytes.len() {
        i += 1;
        let esc = bytes[i] as char;
        i += 1;
        match esc {
            'n' => b'\n' as i128,
            'r' => b'\r' as i128,
            't' => b'\t' as i128,
            '0' => 0,
            '\\' => b'\\' as i128,
            '\'' => b'\'' as i128,
            '\"' => b'\"' as i128,
            'x' => {
                let hex_start = i;
                while i < bytes.len() && (bytes[i] as char).is_ascii_hexdigit() && i - hex_start < 2 {
                    i += 1;
                }
                i128::from_str_radix(&raw[hex_start..i], 16).unwrap_or(0)
            }
            other => other as i128,
        }
    } else {
        let ch = raw[i..].chars().next().unwrap_or('\0');
        i += ch.len_utf8();
        ch as i128
    };
    if i < bytes.len() && bytes[i] == b'\'' {
        (Some(ch_val), i + 1)
    } else {
        (None, start + 1)
    }
}

struct CExprParser<'a> {
    tokens: &'a [CTok],
    pos: usize,
    allow_unknown_as_zero: bool,
    consts: Option<&'a HashMap<String, i128>>,
}

impl CExprParser<'_> {
    fn peek(&self) -> Option<&CTok> {
        self.tokens.get(self.pos)
    }

    fn eat(&mut self, expected: &CTok) -> bool {
        if self.peek() == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn parse_expr(&mut self) -> Option<i128> {
        let mut val = self.parse_ternary()?;
        while self.eat(&CTok::Comma) {
            val = self.parse_ternary()?;
        }
        Some(val)
    }

    fn parse_ternary(&mut self) -> Option<i128> {
        let cond = self.parse_or()?;
        if self.eat(&CTok::Question) {
            let then_v = self.parse_expr()?;
            if !self.eat(&CTok::Colon) {
                return None;
            }
            let else_v = self.parse_ternary()?;
            return Some(if cond != 0 { then_v } else { else_v });
        }
        Some(cond)
    }

    fn parse_or(&mut self) -> Option<i128> {
        let mut lhs = self.parse_and()?;
        while self.eat(&CTok::PipePipe) {
            let rhs = self.parse_and()?;
            lhs = i128::from(lhs != 0 || rhs != 0);
        }
        Some(lhs)
    }

    fn parse_and(&mut self) -> Option<i128> {
        let mut lhs = self.parse_bitor()?;
        while self.eat(&CTok::AmpAmp) {
            let rhs = self.parse_bitor()?;
            lhs = i128::from(lhs != 0 && rhs != 0);
        }
        Some(lhs)
    }

    fn parse_bitor(&mut self) -> Option<i128> {
        let mut lhs = self.parse_bitxor()?;
        while self.eat(&CTok::Pipe) {
            lhs |= self.parse_bitxor()?;
        }
        Some(lhs)
    }

    fn parse_bitxor(&mut self) -> Option<i128> {
        let mut lhs = self.parse_bitand()?;
        while self.eat(&CTok::Caret) {
            lhs ^= self.parse_bitand()?;
        }
        Some(lhs)
    }

    fn parse_bitand(&mut self) -> Option<i128> {
        let mut lhs = self.parse_eq()?;
        while self.eat(&CTok::Amp) {
            lhs &= self.parse_eq()?;
        }
        Some(lhs)
    }

    fn parse_eq(&mut self) -> Option<i128> {
        let mut lhs = self.parse_rel()?;
        loop {
            if self.eat(&CTok::EqEq) {
                lhs = i128::from(lhs == self.parse_rel()?);
            } else if self.eat(&CTok::BangEq) {
                lhs = i128::from(lhs != self.parse_rel()?);
            } else {
                break;
            }
        }
        Some(lhs)
    }

    fn parse_rel(&mut self) -> Option<i128> {
        let mut lhs = self.parse_shift()?;
        loop {
            if self.eat(&CTok::Lt) {
                lhs = i128::from(lhs < self.parse_shift()?);
            } else if self.eat(&CTok::LtEq) {
                lhs = i128::from(lhs <= self.parse_shift()?);
            } else if self.eat(&CTok::Gt) {
                lhs = i128::from(lhs > self.parse_shift()?);
            } else if self.eat(&CTok::GtEq) {
                lhs = i128::from(lhs >= self.parse_shift()?);
            } else {
                break;
            }
        }
        Some(lhs)
    }

    fn parse_shift(&mut self) -> Option<i128> {
        let mut lhs = self.parse_add()?;
        loop {
            if self.eat(&CTok::LtLt) {
                let rhs = u32::try_from(self.parse_add()?).ok()?;
                lhs = lhs.wrapping_shl(rhs);
            } else if self.eat(&CTok::GtGt) {
                let rhs = u32::try_from(self.parse_add()?).ok()?;
                lhs = lhs.wrapping_shr(rhs);
            } else {
                break;
            }
        }
        Some(lhs)
    }

    fn parse_add(&mut self) -> Option<i128> {
        let mut lhs = self.parse_mul()?;
        loop {
            if self.eat(&CTok::Plus) {
                lhs = lhs.wrapping_add(self.parse_mul()?);
            } else if self.eat(&CTok::Minus) {
                lhs = lhs.wrapping_sub(self.parse_mul()?);
            } else {
                break;
            }
        }
        Some(lhs)
    }

    fn parse_mul(&mut self) -> Option<i128> {
        let mut lhs = self.parse_unary()?;
        loop {
            if self.eat(&CTok::Star) {
                lhs = lhs.wrapping_mul(self.parse_unary()?);
            } else if self.eat(&CTok::Slash) {
                let rhs = self.parse_unary()?;
                if rhs == 0 {
                    return None;
                }
                lhs = lhs.wrapping_div(rhs);
            } else if self.eat(&CTok::Percent) {
                let rhs = self.parse_unary()?;
                if rhs == 0 {
                    return None;
                }
                lhs = lhs.wrapping_rem(rhs);
            } else {
                break;
            }
        }
        Some(lhs)
    }

    fn parse_unary(&mut self) -> Option<i128> {
        if self.eat(&CTok::Plus) {
            return self.parse_unary();
        }
        if self.eat(&CTok::Minus) {
            return Some(self.parse_unary()?.wrapping_neg());
        }
        if self.eat(&CTok::Tilde) {
            return Some(!self.parse_unary()?);
        }
        if self.eat(&CTok::Bang) {
            return Some(i128::from(self.parse_unary()? == 0));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Option<i128> {
        let tok = self.peek()?.clone();
        self.pos += 1;
        match tok {
            CTok::Int(v) => Some(v),
            CTok::Ident(name) => {
                if let Some(v) = self.consts.and_then(|c| c.get(&name)) {
                    return Some(*v);
                }
                if self.allow_unknown_as_zero {

                    if self.eat(&CTok::LParen) {
                        let mut depth = 1usize;
                        while depth > 0 {
                            match self.peek() {
                                Some(CTok::LParen) => {
                                    depth += 1;
                                    self.pos += 1;
                                }
                                Some(CTok::RParen) => {
                                    depth -= 1;
                                    self.pos += 1;
                                }
                                Some(_) => self.pos += 1,
                                None => break,
                            }
                        }
                    }
                    Some(0)
                } else {
                    None
                }
            }
            CTok::LParen => {

                let saved = self.pos;
                if let Some(val) = self.parse_expr()
                    && self.eat(&CTok::RParen)
                {
                    return Some(val);
                }
                self.pos = saved;
                None
            }
            _ => None,
        }
    }
}
