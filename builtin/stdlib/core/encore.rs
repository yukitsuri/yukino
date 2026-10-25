use std::cell::RefCell;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct YukinoString {
    pub ptr: *const u8,
    pub len: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct YukinoCodepointEntry {
    pub codepoint: u32,
    pub byte_offset: i64,
    pub byte_len: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct YukinoGraphemeCluster {
    pub text: YukinoString,
    pub byte_offset: i64,
    pub byte_len: i32,
}

thread_local! {
    static ENCORE_STR_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static ENCORE_CP_ENTRIES: RefCell<Vec<YukinoCodepointEntry>> = const { RefCell::new(Vec::new()) };
    static ENCORE_CP_VALS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static ENCORE_GRAPH_BUF: RefCell<Vec<YukinoGraphemeCluster>> = const { RefCell::new(Vec::new()) };
    static ENCORE_U16_BUF: RefCell<Vec<u16>> = const { RefCell::new(Vec::new()) };
}

unsafe fn str_from_raw<'a>(ptr: *const u8, len: i64) -> &'a str {
    if ptr.is_null() || len <= 0 {
        return "";
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    std::str::from_utf8(slice).unwrap_or("")
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum GraphemeCategory {
    Other,
    CR,
    LF,
    Control,
    Extend,
    ZWJ,
    RegionalIndicator,
    ExtendedPictographic,
}

fn categorize(cp: u32) -> GraphemeCategory {
    if cp == 0x0D {
        GraphemeCategory::CR
    } else if cp == 0x0A {
        GraphemeCategory::LF
    } else if cp == 0x200D {
        GraphemeCategory::ZWJ
    } else if (0x1F1E6..=0x1F1FF).contains(&cp) {
        GraphemeCategory::RegionalIndicator
    } else if (0x1F3FB..=0x1F3FF).contains(&cp)
        || (0x0300..=0x036F).contains(&cp)
        || (0x1AB0..=0x1AFF).contains(&cp)
        || (0x1DC0..=0x1DFF).contains(&cp)
        || (0x20D0..=0x20FF).contains(&cp)
        || (0xFE20..=0xFE2F).contains(&cp)
        || (0xFE00..=0xFE0F).contains(&cp)
        || (0xE0100..=0xE01EF).contains(&cp)
    {
        GraphemeCategory::Extend
    } else if (cp <= 0x1F) || (0x7F..=0x9F).contains(&cp) {
        GraphemeCategory::Control
    } else if is_extended_pictographic(cp) {
        GraphemeCategory::ExtendedPictographic
    } else {
        GraphemeCategory::Other
    }
}

fn is_extended_pictographic(cp: u32) -> bool {
    (0x1F300..=0x1F9FF).contains(&cp)
        || (0x1FA00..=0x1FAFF).contains(&cp)
        || (0x2600..=0x27BF).contains(&cp)
        || (0x2300..=0x23FF).contains(&cp)
        || (0x2B50..=0x2B55).contains(&cp)
}

fn should_break_grapheme(
    prev_cat: GraphemeCategory,
    curr_cat: GraphemeCategory,
    ri_counter: usize,
    saw_pict_zwj: bool,
) -> bool {
    if prev_cat == GraphemeCategory::CR && curr_cat == GraphemeCategory::LF {
        return false;
    }
    if prev_cat == GraphemeCategory::Control
        || prev_cat == GraphemeCategory::CR
        || prev_cat == GraphemeCategory::LF
    {
        return true;
    }
    if curr_cat == GraphemeCategory::Control
        || curr_cat == GraphemeCategory::CR
        || curr_cat == GraphemeCategory::LF
    {
        return true;
    }
    if curr_cat == GraphemeCategory::Extend || curr_cat == GraphemeCategory::ZWJ {
        return false;
    }
    if prev_cat == GraphemeCategory::ZWJ
        && saw_pict_zwj
        && curr_cat == GraphemeCategory::ExtendedPictographic
    {
        return false;
    }
    if prev_cat == GraphemeCategory::RegionalIndicator
        && curr_cat == GraphemeCategory::RegionalIndicator
        && (ri_counter % 2 != 0)
    {
        return false;
    }
    true
}

fn find_grapheme_boundaries(s: &str) -> Vec<(usize, usize)> {
    if s.is_empty() {
        return Vec::new();
    }
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let mut bounds = Vec::new();
    let mut cluster_start = 0usize;
    let mut prev_cat = categorize(chars[0].1 as u32);
    let mut ri_counter = if prev_cat == GraphemeCategory::RegionalIndicator { 1 } else { 0 };
    let mut in_pict = prev_cat == GraphemeCategory::ExtendedPictographic;

    for i in 1..chars.len() {
        let (byte_idx, ch) = chars[i];
        let curr_cat = categorize(ch as u32);
        let saw_pict_zwj = in_pict && prev_cat == GraphemeCategory::ZWJ;

        if should_break_grapheme(prev_cat, curr_cat, ri_counter, saw_pict_zwj) {
            bounds.push((cluster_start, byte_idx));
            cluster_start = byte_idx;
            ri_counter = 0;
            in_pict = curr_cat == GraphemeCategory::ExtendedPictographic;
        }

        if curr_cat == GraphemeCategory::RegionalIndicator {
            ri_counter = ri_counter.wrapping_add(1);
        } else {
            ri_counter = 0;
        }

        if curr_cat == GraphemeCategory::ExtendedPictographic {
            in_pict = true;
        } else if curr_cat != GraphemeCategory::Extend && curr_cat != GraphemeCategory::ZWJ {
            in_pict = false;
        }

        prev_cat = curr_cat;
    }

    bounds.push((cluster_start, s.len()));
    bounds
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_encore_is_valid_utf8(p: *const u8, len: i64) -> bool {
    if p.is_null() {
        return len == 0;
    }
    if len < 0 {
        return false;
    }
    let slice = unsafe { std::slice::from_raw_parts(p, len as usize) };
    std::str::from_utf8(slice).is_ok()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_encore_from_utf8_lossy(
    p: *const u8,
    len: i64,
    out_len: *mut i64,
) -> *const u8 {
    if p.is_null() || len <= 0 {
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        return b"".as_ptr();
    }
    let slice = unsafe { std::slice::from_raw_parts(p, len as usize) };
    ENCORE_STR_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        let decoded = String::from_utf8_lossy(slice);
        buf.extend_from_slice(decoded.as_bytes());
        if !out_len.is_null() {
            unsafe { *out_len = buf.len() as i64; }
        }
        buf.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_encore_count_codepoints(p: *const u8, len: i64) -> i64 {
    let s = unsafe { str_from_raw(p, len) };
    s.chars().count() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_encore_codepoints(
    p: *const u8,
    len: i64,
    out_count: *mut i64,
) -> *const YukinoCodepointEntry {
    let s = unsafe { str_from_raw(p, len) };
    ENCORE_CP_ENTRIES.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        for (offset, ch) in s.char_indices() {
            buf.push(YukinoCodepointEntry {
                codepoint: ch as u32,
                byte_offset: offset as i64,
                byte_len: ch.len_utf8() as i32,
            });
        }
        if !out_count.is_null() {
            unsafe { *out_count = buf.len() as i64; }
        }
        if buf.is_empty() {
            std::ptr::null()
        } else {
            buf.as_ptr()
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_encore_decode_codepoints(
    p: *const u8,
    len: i64,
    out_count: *mut i64,
) -> *const u32 {
    let s = unsafe { str_from_raw(p, len) };
    ENCORE_CP_VALS.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        for ch in s.chars() {
            buf.push(ch as u32);
        }
        if !out_count.is_null() {
            unsafe { *out_count = buf.len() as i64; }
        }
        if buf.is_empty() {
            std::ptr::null()
        } else {
            buf.as_ptr()
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_encore_encode_codepoint(cp: u32, out_len: *mut i64) -> *const u8 {
    let Some(ch) = char::from_u32(cp) else {
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        return std::ptr::null();
    };
    ENCORE_STR_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        let mut temp = [0u8; 4];
        let encoded = ch.encode_utf8(&mut temp);
        buf.extend_from_slice(encoded.as_bytes());
        if !out_len.is_null() {
            unsafe { *out_len = buf.len() as i64; }
        }
        buf.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_encore_encode_codepoints(
    cps: *const u32,
    count: i64,
    out_len: *mut i64,
) -> *const u8 {
    if cps.is_null() || count <= 0 {
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        return b"".as_ptr();
    }
    let slice = unsafe { std::slice::from_raw_parts(cps, count as usize) };
    ENCORE_STR_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        let mut temp = [0u8; 4];
        for &val in slice {
            if let Some(ch) = char::from_u32(val) {
                let encoded = ch.encode_utf8(&mut temp);
                buf.extend_from_slice(encoded.as_bytes());
            } else {
                let encoded = '\u{FFFD}'.encode_utf8(&mut temp);
                buf.extend_from_slice(encoded.as_bytes());
            }
        }
        if !out_len.is_null() {
            unsafe { *out_len = buf.len() as i64; }
        }
        buf.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_encore_count_graphemes(p: *const u8, len: i64) -> i64 {
    let s = unsafe { str_from_raw(p, len) };
    find_grapheme_boundaries(s).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_encore_graphemes(
    p: *const u8,
    len: i64,
    out_count: *mut i64,
) -> *const YukinoGraphemeCluster {
    let s = unsafe { str_from_raw(p, len) };
    let bounds = find_grapheme_boundaries(s);
    ENCORE_GRAPH_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        for (start, end) in bounds {
            let slice = &s[start..end];
            buf.push(YukinoGraphemeCluster {
                text: YukinoString {
                    ptr: slice.as_ptr(),
                    len: slice.len() as i64,
                },
                byte_offset: start as i64,
                byte_len: (end - start) as i32,
            });
        }
        if !out_count.is_null() {
            unsafe { *out_count = buf.len() as i64; }
        }
        if buf.is_empty() {
            std::ptr::null()
        } else {
            buf.as_ptr()
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_encore_to_utf16(
    p: *const u8,
    len: i64,
    null_term: bool,
    out_count: *mut i64,
) -> *const u16 {
    let s = unsafe { str_from_raw(p, len) };
    ENCORE_U16_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        buf.extend(s.encode_utf16());
        if null_term {
            buf.push(0);
        }
        if !out_count.is_null() {
            unsafe { *out_count = buf.len() as i64; }
        }
        if buf.is_empty() {
            std::ptr::null()
        } else {
            buf.as_ptr()
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_encore_from_utf16(
    units: *const u16,
    len: i64,
    lossy: bool,
    out_len: *mut i64,
) -> *const u8 {
    if units.is_null() || len <= 0 {
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        return b"".as_ptr();
    }
    let slice = unsafe { std::slice::from_raw_parts(units, len as usize) };
    ENCORE_STR_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        if lossy {
            let decoded = String::from_utf16_lossy(slice);
            buf.extend_from_slice(decoded.as_bytes());
        } else {
            match String::from_utf16(slice) {
                Ok(decoded) => buf.extend_from_slice(decoded.as_bytes()),
                Err(_) => {
                    if !out_len.is_null() {
                        unsafe { *out_len = -1; }
                    }
                    return std::ptr::null();
                }
            }
        }
        if !out_len.is_null() {
            unsafe { *out_len = buf.len() as i64; }
        }
        buf.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_encore_from_utf16_null_terminated(
    p: *const u16,
    lossy: bool,
    out_len: *mut i64,
) -> *const u8 {
    if p.is_null() {
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        return b"".as_ptr();
    }
    let mut len = 0usize;
    unsafe {
        while *p.add(len) != 0 {
            len = len.wrapping_add(1);
        }
    }
    yukistd_encore_from_utf16(p, len as i64, lossy, out_len)
}
