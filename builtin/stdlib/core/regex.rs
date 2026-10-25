use std::cell::RefCell;
use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct YukinoString {
    pub ptr: *const u8,
    pub len: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct YukinoMatch {
    pub start: i64,
    pub end: i64,
    pub text_ptr: *const u8,
    pub text_len: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct YukinoCapture {
    pub index: i64,
    pub matched: i64,
    pub start: i64,
    pub end: i64,
    pub text_ptr: *const u8,
    pub text_len: i64,
    pub name_ptr: *const u8,
    pub name_len: i64,
}

thread_local! {
    static LAST_ERROR: RefCell<(i32, String)> = const { RefCell::new((0, String::new())) };
    static ERR_MSG_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static STR_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static SPLIT_BUF: RefCell<Vec<YukinoString>> = const { RefCell::new(Vec::new()) };
    static MATCHES_BUF: RefCell<Vec<YukinoMatch>> = const { RefCell::new(Vec::new()) };
    static CAPTURES_BUF: RefCell<Vec<YukinoCapture>> = const { RefCell::new(Vec::new()) };
}

fn set_last_error(code: i32, msg: String) {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = (code, msg);
    });
}

fn clear_last_error() {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = (0, String::new());
    });
}

fn set_str_result(bytes: &[u8], out_len: *mut i64) -> *const u8 {
    STR_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        buf.extend_from_slice(bytes);
        if !out_len.is_null() {
            unsafe {
                *out_len = buf.len() as i64;
            }
        }
        buf.as_ptr()
    })
}

unsafe fn slice_from_raw<'a>(ptr: *const u8, len: i64) -> Option<&'a [u8]> {
    if len < 0 {
        return None;
    }
    if len == 0 {
        return Some(&[]);
    }
    if ptr.is_null() {
        return None;
    }
    Some(unsafe { std::slice::from_raw_parts(ptr, len as usize) })
}

unsafe fn str_from_raw<'a>(ptr: *const u8, len: i64) -> Option<&'a str> {
    let slice = unsafe { slice_from_raw(ptr, len)? };
    std::str::from_utf8(slice).ok()
}

unsafe fn get_regex<'a>(handle: *mut c_void) -> Option<&'a regex::Regex> {
    if handle.is_null() {
        return None;
    }
    unsafe { (handle as *const regex::Regex).as_ref() }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_regex_compile(
    pattern_p: *const u8,
    pattern_len: i64,
    flags: i32,
) -> *mut c_void {
    let pattern_str = match unsafe { str_from_raw(pattern_p, pattern_len) } {
        Some(s) => s,
        None => {
            set_last_error(101, "pattern is not valid UTF-8".to_string());
            return std::ptr::null_mut();
        }
    };
    let mut builder = regex::RegexBuilder::new(pattern_str);
    if flags & 1 != 0 {
        builder.case_insensitive(true);
    }
    if flags & 2 != 0 {
        builder.multi_line(true);
    }
    if flags & 4 != 0 {
        builder.dot_matches_new_line(true);
    }
    if flags & 8 != 0 {
        builder.crlf(true);
    }
    if flags & 16 != 0 {
        builder.swap_greed(true);
    }
    if flags & 32 != 0 {
        builder.ignore_whitespace(true);
    }
    if flags & 64 != 0 {
        builder.unicode(true);
    }
    match builder.build() {
        Ok(re) => {
            clear_last_error();
            Box::into_raw(Box::new(re)) as *mut c_void
        }
        Err(err) => {
            set_last_error(101, err.to_string());
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_regex_free(handle: *mut c_void) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle as *mut regex::Regex));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_regex_last_error_code() -> i32 {
    LAST_ERROR.with(|cell| cell.borrow().0)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_regex_last_error_message(out_len: *mut i64) -> *const u8 {
    ERR_MSG_BUF.with(|buf_cell| {
        let mut buf = buf_cell.borrow_mut();
        buf.clear();
        LAST_ERROR.with(|err_cell| {
            buf.extend_from_slice(err_cell.borrow().1.as_bytes());
        });
        if !out_len.is_null() {
            unsafe {
                *out_len = buf.len() as i64;
            }
        }
        buf.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_regex_is_match(
    handle: *mut c_void,
    text_p: *const u8,
    text_len: i64,
) -> bool {
    let re = match unsafe { get_regex(handle) } {
        Some(r) => r,
        None => return false,
    };
    let text = match unsafe { str_from_raw(text_p, text_len) } {
        Some(t) => t,
        None => return false,
    };
    re.is_match(text)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_regex_find(
    handle: *mut c_void,
    text_p: *const u8,
    text_len: i64,
    start_offset: i64,
    out_start: *mut i64,
    out_end: *mut i64,
    out_len: *mut i64,
) -> *const u8 {
    let re = match unsafe { get_regex(handle) } {
        Some(r) => r,
        None => return std::ptr::null(),
    };
    let text = match unsafe { str_from_raw(text_p, text_len) } {
        Some(t) => t,
        None => return std::ptr::null(),
    };
    if start_offset < 0 || start_offset > text.len() as i64 {
        return std::ptr::null();
    }
    let offset = start_offset as usize;
    if !text.is_char_boundary(offset) {
        return std::ptr::null();
    }
    let sub = &text[offset..];
    match re.find(sub) {
        Some(m) => {
            let s = (offset + m.start()) as i64;
            let e = (offset + m.end()) as i64;
            let match_str = m.as_str();
            if !out_start.is_null() {
                unsafe { *out_start = s; }
            }
            if !out_end.is_null() {
                unsafe { *out_end = e; }
            }
            if !out_len.is_null() {
                unsafe { *out_len = match_str.len() as i64; }
            }
            match_str.as_ptr()
        }
        None => std::ptr::null(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_regex_find_all(
    handle: *mut c_void,
    text_p: *const u8,
    text_len: i64,
    out_count: *mut i64,
) -> *const YukinoMatch {
    if !out_count.is_null() {
        unsafe { *out_count = 0; }
    }
    let re = match unsafe { get_regex(handle) } {
        Some(r) => r,
        None => return std::ptr::null(),
    };
    let text = match unsafe { str_from_raw(text_p, text_len) } {
        Some(t) => t,
        None => return std::ptr::null(),
    };
    MATCHES_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        for m in re.find_iter(text) {
            buf.push(YukinoMatch {
                start: m.start() as i64,
                end: m.end() as i64,
                text_ptr: m.as_str().as_ptr(),
                text_len: m.as_str().len() as i64,
            });
        }
        if !out_count.is_null() {
            unsafe { *out_count = buf.len() as i64; }
        }
        buf.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_regex_captures(
    handle: *mut c_void,
    text_p: *const u8,
    text_len: i64,
    out_count: *mut i64,
) -> *const YukinoCapture {
    if !out_count.is_null() {
        unsafe { *out_count = 0; }
    }
    let re = match unsafe { get_regex(handle) } {
        Some(r) => r,
        None => return std::ptr::null(),
    };
    let text = match unsafe { str_from_raw(text_p, text_len) } {
        Some(t) => t,
        None => return std::ptr::null(),
    };
    let caps = match re.captures(text) {
        Some(c) => c,
        None => return std::ptr::null(),
    };
    let capture_names: Vec<Option<&str>> = re.capture_names().collect();
    CAPTURES_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        for i in 0..caps.len() {
            let name_opt = capture_names.get(i).copied().flatten();
            let (name_ptr, name_len) = match name_opt {
                Some(n) => (n.as_ptr(), n.len() as i64),
                None => (std::ptr::null(), 0),
            };
            if let Some(m) = caps.get(i) {
                buf.push(YukinoCapture {
                    index: i as i64,
                    matched: 1,
                    start: m.start() as i64,
                    end: m.end() as i64,
                    text_ptr: m.as_str().as_ptr(),
                    text_len: m.as_str().len() as i64,
                    name_ptr,
                    name_len,
                });
            } else {
                buf.push(YukinoCapture {
                    index: i as i64,
                    matched: 0,
                    start: -1,
                    end: -1,
                    text_ptr: std::ptr::null(),
                    text_len: 0,
                    name_ptr,
                    name_len,
                });
            }
        }
        if !out_count.is_null() {
            unsafe { *out_count = buf.len() as i64; }
        }
        buf.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_regex_replace(
    handle: *mut c_void,
    text_p: *const u8,
    text_len: i64,
    rep_p: *const u8,
    rep_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    let re = match unsafe { get_regex(handle) } {
        Some(r) => r,
        None => {
            if !out_len.is_null() {
                unsafe { *out_len = -1; }
            }
            return std::ptr::null();
        }
    };
    let text = match unsafe { str_from_raw(text_p, text_len) } {
        Some(t) => t,
        None => {
            if !out_len.is_null() {
                unsafe { *out_len = -1; }
            }
            return std::ptr::null();
        }
    };
    let rep = match unsafe { str_from_raw(rep_p, rep_len) } {
        Some(r) => r,
        None => {
            if !out_len.is_null() {
                unsafe { *out_len = -1; }
            }
            return std::ptr::null();
        }
    };
    let replaced = re.replace(text, rep);
    set_str_result(replaced.as_bytes(), out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_regex_replace_all(
    handle: *mut c_void,
    text_p: *const u8,
    text_len: i64,
    rep_p: *const u8,
    rep_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    let re = match unsafe { get_regex(handle) } {
        Some(r) => r,
        None => {
            if !out_len.is_null() {
                unsafe { *out_len = -1; }
            }
            return std::ptr::null();
        }
    };
    let text = match unsafe { str_from_raw(text_p, text_len) } {
        Some(t) => t,
        None => {
            if !out_len.is_null() {
                unsafe { *out_len = -1; }
            }
            return std::ptr::null();
        }
    };
    let rep = match unsafe { str_from_raw(rep_p, rep_len) } {
        Some(r) => r,
        None => {
            if !out_len.is_null() {
                unsafe { *out_len = -1; }
            }
            return std::ptr::null();
        }
    };
    let replaced = re.replace_all(text, rep);
    set_str_result(replaced.as_bytes(), out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_regex_split(
    handle: *mut c_void,
    text_p: *const u8,
    text_len: i64,
    out_count: *mut i64,
) -> *const YukinoString {
    let re = match unsafe { get_regex(handle) } {
        Some(r) => r,
        None => {
            if !out_count.is_null() {
                unsafe { *out_count = -1; }
            }
            return std::ptr::null();
        }
    };
    let text = match unsafe { str_from_raw(text_p, text_len) } {
        Some(t) => t,
        None => {
            if !out_count.is_null() {
                unsafe { *out_count = 0; }
            }
            return std::ptr::null();
        }
    };
    SPLIT_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        for part in re.split(text) {
            buf.push(YukinoString {
                ptr: part.as_ptr(),
                len: part.len() as i64,
            });
        }
        if !out_count.is_null() {
            unsafe { *out_count = buf.len() as i64; }
        }
        buf.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_regex_escape(
    text_p: *const u8,
    text_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    let text = match unsafe { str_from_raw(text_p, text_len) } {
        Some(t) => t,
        None => {
            if !out_len.is_null() {
                unsafe { *out_len = 0; }
            }
            return std::ptr::null();
        }
    };
    let escaped = regex::escape(text);
    set_str_result(escaped.as_bytes(), out_len)
}
