use std::cell::RefCell;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct YukinoString { pub ptr: *const u8, pub len: i64, }

thread_local! {
    static STR_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static SPLIT_BUF: RefCell<Vec<YukinoString>> = const { RefCell::new(Vec::new()) };
}

fn set_str_result(bytes: &[u8], out_len: *mut i64) -> *const u8 {
    STR_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        buf.extend_from_slice(bytes);
        if !out_len.is_null() {
            unsafe { *out_len = buf.len() as i64; }
        }
        buf.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_slice(
    ptr: *const u8,
    len: i64,
    start: i64,
    end: i64,
    out_len: *mut i64,
) -> *const u8 {
    if ptr.is_null() || len <= 0 {
        if !out_len.is_null() { unsafe { *out_len = 0; } }
        return std::ptr::null();
    }
    let s = start.max(0).min(len);
    let e = end.max(s).min(len);
    let sub_len = e - s;
    if !out_len.is_null() { unsafe { *out_len = sub_len; } }
    if sub_len == 0 { return std::ptr::null(); }
    unsafe { ptr.add(s as usize) }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_utflen(ptr: *const u8, len: i64) -> i64 {
    if ptr.is_null() || len <= 0 {
        return 0;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    match std::str::from_utf8(slice) {
        Ok(s) => s.chars().count() as i64,
        Err(_) => len,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_eq(p1: *const u8, l1: i64, p2: *const u8, l2: i64) -> bool {
    if l1 != l2 { return false; }
    if l1 <= 0 { return true; }
    if p1.is_null() || p2.is_null() { return p1 == p2; }
    let s1 = unsafe { std::slice::from_raw_parts(p1, l1 as usize) };
    let s2 = unsafe { std::slice::from_raw_parts(p2, l2 as usize) };
    s1 == s2
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_cmp(p1: *const u8, l1: i64, p2: *const u8, l2: i64) -> i32 {
    let s1 = if p1.is_null() || l1 <= 0 { &[] } else { unsafe { std::slice::from_raw_parts(p1, l1 as usize) } };
    let s2 = if p2.is_null() || l2 <= 0 { &[] } else { unsafe { std::slice::from_raw_parts(p2, l2 as usize) } };
    match s1.cmp(s2) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_starts_with(p1: *const u8, l1: i64, p2: *const u8, l2: i64) -> bool {
    if l2 == 0 { return true; }
    if l1 < l2 || p1.is_null() || p2.is_null() { return false; }
    let s1 = unsafe { std::slice::from_raw_parts(p1, l1 as usize) };
    let s2 = unsafe { std::slice::from_raw_parts(p2, l2 as usize) };
    s1.starts_with(s2)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_ends_with(p1: *const u8, l1: i64, p2: *const u8, l2: i64) -> bool {
    if l2 == 0 { return true; }
    if l1 < l2 || p1.is_null() || p2.is_null() { return false; }
    let s1 = unsafe { std::slice::from_raw_parts(p1, l1 as usize) };
    let s2 = unsafe { std::slice::from_raw_parts(p2, l2 as usize) };
    s1.ends_with(s2)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_contains(p1: *const u8, l1: i64, p2: *const u8, l2: i64) -> bool {
    yukistd_str_index_of(p1, l1, p2, l2, 0) >= 0
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_index_of(
    p1: *const u8,
    l1: i64,
    p2: *const u8,
    l2: i64,
    from_idx: i64,
) -> i64 {
    if l2 == 0 { return from_idx.max(0).min(l1); }
    if l1 <= 0 || p1.is_null() || p2.is_null() { return -1; }
    let from = from_idx.max(0) as usize;
    if from >= l1 as usize { return -1; }
    let s1 = unsafe { std::slice::from_raw_parts(p1, l1 as usize) };
    let s2 = unsafe { std::slice::from_raw_parts(p2, l2 as usize) };
    if let (Ok(str1), Ok(str2)) = (std::str::from_utf8(&s1[from..]), std::str::from_utf8(s2)) {
        return str1.find(str2).map_or(-1, |pos| (from + pos) as i64);
    }
    s1[from..]
        .windows(s2.len())
        .position(|window| window == s2)
        .map_or(-1, |pos| (from + pos) as i64)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_last_index_of(p1: *const u8, l1: i64, p2: *const u8, l2: i64) -> i64 {
    if l2 == 0 { return l1.max(0); }
    if l1 < l2 || p1.is_null() || p2.is_null() { return -1; }
    let s1 = unsafe { std::slice::from_raw_parts(p1, l1 as usize) };
    let s2 = unsafe { std::slice::from_raw_parts(p2, l2 as usize) };
    if let (Ok(str1), Ok(str2)) = (std::str::from_utf8(s1), std::str::from_utf8(s2)) {
        return str1.rfind(str2).map_or(-1, |pos| pos as i64);
    }
    s1.windows(s2.len())
        .rposition(|window| window == s2)
        .map_or(-1, |pos| pos as i64)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_count(p1: *const u8, l1: i64, p2: *const u8, l2: i64) -> i64 {
    if l1 <= 0 || l2 <= 0 || p1.is_null() || p2.is_null() || l1 < l2 { return 0; }
    let s1 = unsafe { std::slice::from_raw_parts(p1, l1 as usize) };
    let s2 = unsafe { std::slice::from_raw_parts(p2, l2 as usize) };
    if let (Ok(str1), Ok(str2)) = (std::str::from_utf8(s1), std::str::from_utf8(s2)) { return str1.matches(str2).count() as i64; }
    let mut count = 0i64;
    let mut i = 0usize;
    while i + s2.len() <= s1.len() {
        if &s1[i..i + s2.len()] == s2 {
            count += 1;
            i += s2.len();
        } else { i += 1; }
    }
    count
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_trim(
    p: *const u8,
    len: i64,
    mode: i32,
    out_len: *mut i64,
) -> *const u8 {
    if p.is_null() || len <= 0 {
        if !out_len.is_null() { unsafe { *out_len = 0; } }
        return std::ptr::null();
    }
    let s = unsafe { std::slice::from_raw_parts(p, len as usize) };
    let text = match std::str::from_utf8(s) {
        Ok(t) => t,
        Err(_) => {
            if !out_len.is_null() { unsafe { *out_len = len; } }
            return p;
        }
    };
    let trimmed = match mode {
        1 => text.trim_start(),
        2 => text.trim_end(),
        _ => text.trim(),
    };
    if !out_len.is_null() {
        unsafe { *out_len = trimmed.len() as i64; }
    }
    trimmed.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_upper(p: *const u8, len: i64, out_len: *mut i64) -> *const u8 {
    if p.is_null() || len <= 0 {
        if !out_len.is_null() { unsafe { *out_len = 0; } }
        return std::ptr::null();
    }
    let s = unsafe { std::slice::from_raw_parts(p, len as usize) };
    if let Ok(text) = std::str::from_utf8(s) {
        let res = text.to_uppercase();
        return set_str_result(res.as_bytes(), out_len);
    }
    let mut v = s.to_vec();
    v.make_ascii_uppercase();
    set_str_result(&v, out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_lower(p: *const u8, len: i64, out_len: *mut i64) -> *const u8 {
    if p.is_null() || len <= 0 {
        if !out_len.is_null() { unsafe { *out_len = 0; } }
        return std::ptr::null();
    }
    let s = unsafe { std::slice::from_raw_parts(p, len as usize) };
    if let Ok(text) = std::str::from_utf8(s) {
        let res = text.to_lowercase();
        return set_str_result(res.as_bytes(), out_len);
    }
    let mut v = s.to_vec();
    v.make_ascii_lowercase();
    set_str_result(&v, out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_replace(
    p: *const u8,
    len: i64,
    from_p: *const u8,
    from_len: i64,
    to_p: *const u8,
    to_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    if p.is_null() || len <= 0 {
        if !out_len.is_null() { unsafe { *out_len = 0; } }
        return std::ptr::null();
    }
    let s = unsafe { std::slice::from_raw_parts(p, len as usize) };
    let from_bytes = if from_p.is_null() || from_len <= 0 { &[][..] } else { unsafe { std::slice::from_raw_parts(from_p, from_len as usize) } };
    let to_bytes = if to_p.is_null() || to_len <= 0 { &[][..] } else { unsafe { std::slice::from_raw_parts(to_p, to_len as usize) } };

    if let (Ok(text), Ok(from_str), Ok(to_str)) = (
        std::str::from_utf8(s),
        std::str::from_utf8(from_bytes),
        std::str::from_utf8(to_bytes),
    ) {
        let replaced = text.replace(from_str, to_str);
        return set_str_result(replaced.as_bytes(), out_len);
    }
    set_str_result(s, out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_repeat(p: *const u8, len: i64, count: i64, out_len: *mut i64) -> *const u8 {
    if p.is_null() || len <= 0 || count <= 0 {
        if !out_len.is_null() { unsafe { *out_len = 0; } }
        return std::ptr::null();
    }
    let s = unsafe { std::slice::from_raw_parts(p, len as usize) };
    let n = count as usize;
    STR_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        buf.reserve(s.len() * n);
        for _ in 0..n { buf.extend_from_slice(s); }
        if !out_len.is_null() { unsafe { *out_len = buf.len() as i64; } }
        buf.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_reverse(p: *const u8, len: i64, out_len: *mut i64) -> *const u8 {
    if p.is_null() || len <= 0 {
        if !out_len.is_null() { unsafe { *out_len = 0; } }
        return std::ptr::null();
    }
    let s = unsafe { std::slice::from_raw_parts(p, len as usize) };
    if let Ok(text) = std::str::from_utf8(s) {
        let rev: String = text.chars().rev().collect();
        return set_str_result(rev.as_bytes(), out_len);
    }
    let mut v = s.to_vec();
    v.reverse();
    set_str_result(&v, out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_pad_start(
    p: *const u8,
    len: i64,
    target_len: i64,
    pad_p: *const u8,
    pad_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    if len >= target_len || pad_p.is_null() || pad_len <= 0 {
        if !out_len.is_null() { unsafe { *out_len = len.max(0); } }


        return p;
    }
    let missing = (target_len - len) as usize;
    let s = if p.is_null() || len <= 0 { &[][..] } else { unsafe { std::slice::from_raw_parts(p, len as usize) } };
    let pad = unsafe { std::slice::from_raw_parts(pad_p, pad_len as usize) };
    let mut res = Vec::with_capacity(target_len as usize);
    let full_pads = missing / pad.len();
    let rem = missing % pad.len();
    for _ in 0..full_pads { res.extend_from_slice(pad); }
    if rem > 0 { res.extend_from_slice(&pad[..rem]); }
    res.extend_from_slice(s);
    set_str_result(&res, out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_pad_end(
    p: *const u8,
    len: i64,
    target_len: i64,
    pad_p: *const u8,
    pad_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    if len >= target_len || pad_p.is_null() || pad_len <= 0 {
        if !out_len.is_null() { unsafe { *out_len = len.max(0); } } return p;
    }
    let missing = (target_len - len) as usize;
    let s = if p.is_null() || len <= 0 { &[][..] } else { unsafe { std::slice::from_raw_parts(p, len as usize) } };
    let pad = unsafe { std::slice::from_raw_parts(pad_p, pad_len as usize) };
    let mut res = Vec::with_capacity(target_len as usize);
    res.extend_from_slice(s);
    let full_pads = missing / pad.len();
    let rem = missing % pad.len();
    for _ in 0..full_pads { res.extend_from_slice(pad); }
    if rem > 0 { res.extend_from_slice(&pad[..rem]); }
    set_str_result(&res, out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_to_i64(p: *const u8, len: i64, default_val: i64) -> i64 {
    if p.is_null() || len <= 0 { return default_val; }
    let s = unsafe { std::slice::from_raw_parts(p, len as usize) };
    if let Ok(text) = std::str::from_utf8(s) {
        text.trim().parse::<i64>().unwrap_or(default_val)
    } 




    
    else { default_val }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_to_f64(p: *const u8, len: i64, default_val: f64) -> f64 {
    if p.is_null() || len <= 0 { return default_val; }
    let s = unsafe { std::slice::from_raw_parts(p, len as usize) };
    if let Ok(text) = std::str::from_utf8(s) {
        text.trim().parse::<f64>().unwrap_or(default_val)
    } 


    else { default_val }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_parse_i64(p: *const u8, len: i64, out_val: *mut i64) -> bool {
    if p.is_null() || len <= 0 || out_val.is_null() { return false; }
    let s = unsafe { std::slice::from_raw_parts(p, len as usize) };
    if let Ok(text) = std::str::from_utf8(s) {
        if let Ok(val) = text.trim().parse::<i64>() {
            unsafe { *out_val = val; }
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_parse_f64(p: *const u8, len: i64, out_val: *mut f64) -> bool {
    if p.is_null() || len <= 0 || out_val.is_null() { return false; }
    let s = unsafe { std::slice::from_raw_parts(p, len as usize) };
    if let Ok(text) = std::str::from_utf8(s) {
        if let Ok(val) = text.trim().parse::<f64>() {
            unsafe { *out_val = val; }
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_split(
    p: *const u8,
    len: i64,
    delim_p: *const u8,
    delim_len: i64,
    out_len: *mut i64,
) -> *const YukinoString {
    if p.is_null() || len <= 0 {
        if !out_len.is_null() { unsafe { *out_len = 0; } }
        return std::ptr::null();
    }
    let s = unsafe { std::slice::from_raw_parts(p, len as usize) };
    let delim = if delim_p.is_null() || delim_len <= 0 {
        &[][..]
    } else { unsafe { std::slice::from_raw_parts(delim_p, delim_len as usize) } };

    SPLIT_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();

        if delim.is_empty() { buf.push(YukinoString { ptr: p, len });
        } else if let (Ok(text), Ok(delim_str)) = (std::str::from_utf8(s), std::str::from_utf8(delim)) {
            for part in text.split(delim_str) {
                buf.push(YukinoString {
                    ptr: part.as_ptr(),
                    len: part.len() as i64,
                });
            }
        } else {
            let mut start = 0;
            while let Some(pos) = s[start..].windows(delim.len()).position(|w| w == delim) {
                let part_len = pos;
                buf.push(YukinoString {
                    ptr: unsafe { p.add(start) },
                    len: part_len as i64,
                });
                start += pos + delim.len();
            }
            buf.push(YukinoString {
                ptr: unsafe { p.add(start) },
                len: (s.len() - start) as i64,
            });
        }

        if !out_len.is_null() {
            unsafe { *out_len = buf.len() as i64; }
        }
        buf.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_lines(p: *const u8, len: i64, out_len: *mut i64) -> *const YukinoString {
    if p.is_null() || len <= 0 {
        if !out_len.is_null() { unsafe { *out_len = 0; } }
        return std::ptr::null();
    }
    let s = unsafe { std::slice::from_raw_parts(p, len as usize) };

    SPLIT_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();

        if let Ok(text) = std::str::from_utf8(s) {
            for line in text.lines() {
                buf.push(YukinoString {
                    ptr: line.as_ptr(),
                    len: line.len() as i64,
                });
            }
        } else {
            let mut start = 0;
            for i in 0..s.len() {
                if s[i] == b'\n' {
                    let mut end = i;
                    if end > start && s[end - 1] == b'\r' { end -= 1; }
                    buf.push(YukinoString {
                        ptr: unsafe { p.add(start) },
                        len: (end - start) as i64,
                    });
                    start = i + 1;
                }
            }
            if start < s.len() {
                let mut end = s.len();
                if end > start && s[end - 1] == b'\r' { end -= 1; }
                buf.push(YukinoString {
                    ptr: unsafe { p.add(start) },
                    len: (end - start) as i64,
                });
            }
        }

        if !out_len.is_null() {
            unsafe { *out_len = buf.len() as i64; }
        }
        buf.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_str_join(
    items_p: *const YukinoString,
    items_len: i64,
    delim_p: *const u8,
    delim_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    if items_p.is_null() || items_len <= 0 {
        if !out_len.is_null() { unsafe { *out_len = 0; } }
        return std::ptr::null();
    }
    let items = unsafe { std::slice::from_raw_parts(items_p, items_len as usize) };
    let delim = if delim_p.is_null() || delim_len <= 0 {
        &[][..]
    } else { unsafe { std::slice::from_raw_parts(delim_p, delim_len as usize) } };

    STR_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        for (i, item) in items.iter().enumerate() {
            if i > 0 && !delim.is_empty() { buf.extend_from_slice(delim); }
            if item.len > 0 && !item.ptr.is_null() {
                let slice = unsafe { std::slice::from_raw_parts(item.ptr, item.len as usize) };
                buf.extend_from_slice(slice);
            }
        }
        if !out_len.is_null() { unsafe { *out_len = buf.len() as i64; } }
        buf.as_ptr()
    })
}
