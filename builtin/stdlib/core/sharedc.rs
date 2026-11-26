use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{c_void, CString};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug)]
struct ExportRecord {
    name: String,
    mangled: String,
    signature: String,
    abi: String,
    kind: String,
}

#[derive(Clone, Debug)]
struct LibraryMeta {
    package_name: String,
    yukic_version: String,
    target: String,
    abi_hash: String,
    exports: Vec<ExportRecord>,
}

static META_CACHE: OnceLock<Mutex<HashMap<usize, LibraryMeta>>> = OnceLock::new();

fn meta_cache() -> &'static Mutex<HashMap<usize, LibraryMeta>> {
    META_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
    static ERR_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static STR_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn set_last_error(msg: String) {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = msg;
    });
}

fn clear_last_error() {
    LAST_ERROR.with(|cell| {
        cell.borrow_mut().clear();
    });
}

fn set_err_result(bytes: &[u8], out_len: *mut i64) -> *const u8 {
    ERR_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        buf.extend_from_slice(bytes);
        if !out_len.is_null() {
            unsafe { *out_len = buf.len() as i64; }
        }
        buf.as_ptr()
    })
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

unsafe fn str_from_raw<'a>(ptr: *const u8, len: i64) -> &'a str {
    if ptr.is_null() || len <= 0 {
        return "";
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    std::str::from_utf8(slice).unwrap_or("")
}

#[cfg(unix)]
mod sys {
    use std::ffi::{c_char, c_int, c_void};

    #[cfg(target_os = "macos")]
    const RTLD_GLOBAL: c_int = 0x8;
    #[cfg(not(target_os = "macos"))]
    const RTLD_GLOBAL: c_int = 0x100;

    #[cfg(target_os = "macos")]
    const RTLD_LOCAL: c_int = 0x4;
    #[cfg(not(target_os = "macos"))]
    const RTLD_LOCAL: c_int = 0x0;

    const RTLD_LAZY: c_int = 1;
    const RTLD_NOW: c_int = 2;

    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn dlclose(handle: *mut c_void) -> c_int;
        fn dlerror() -> *const c_char;
    }

    pub fn map_flags(flags: i32) -> c_int {
        let mut native = 0;
        if (flags & 1) != 0 {
            native |= RTLD_LAZY;
        } else {
            native |= RTLD_NOW;
        }

        if (flags & 8) != 0 {
            native |= RTLD_GLOBAL;
        } else {
            native |= RTLD_LOCAL;
        }
        native
    }

    pub unsafe fn dl_open(path: Option<&std::ffi::CStr>, flags: i32) -> Result<*mut c_void, String> {
        let native_flags = map_flags(flags);
        let path_ptr = path.map_or(std::ptr::null(), |p| p.as_ptr());
        let handle = unsafe { dlopen(path_ptr, native_flags) };
        if handle.is_null() {
            let err_ptr = unsafe { dlerror() };
            let msg = if !err_ptr.is_null() {
                unsafe { std::ffi::CStr::from_ptr(err_ptr) }
                    .to_string_lossy()
                    .to_string()
            } else {
                "dlopen failed".to_string()
            };
            Err(msg)
        } else {
            Ok(handle)
        }
    }

    pub unsafe fn dl_symbol(handle: *mut c_void, symbol: &std::ffi::CStr) -> Result<*mut c_void, String> {
        // clear previous error
        let _ = unsafe { dlerror() };
        let sym_ptr = unsafe { dlsym(handle, symbol.as_ptr()) };
        let err_ptr = unsafe { dlerror() };
        if !err_ptr.is_null() {
            let msg = unsafe { std::ffi::CStr::from_ptr(err_ptr) }
                .to_string_lossy()
                .to_string();
            Err(msg)
        } else {
            Ok(sym_ptr)
        }
    }

    pub unsafe fn dl_close(handle: *mut c_void) -> bool {
        if handle.is_null() {
            return true;
        }
        unsafe { dlclose(handle) == 0 }
    }
}

#[cfg(windows)]
mod sys {
    use std::ffi::{c_char, c_void};
    use std::os::windows::ffi::OsStrExt;

    unsafe extern "system" {
        fn LoadLibraryW(lpLibFileName: *const u16) -> *mut c_void;
        fn GetModuleHandleW(lpModuleName: *const u16) -> *mut c_void;
        fn GetProcAddress(hModule: *mut c_void, lpProcName: *const c_char) -> *mut c_void;
        fn FreeLibrary(hLibModule: *mut c_void) -> i32;
        fn GetLastError() -> u32;
        fn FormatMessageW(
            dwFlags: u32,
            lpSource: *const c_void,
            dwMessageId: u32,
            dwLanguageId: u32,
            lpBuffer: *mut u16,
            nSize: u32,
            Arguments: *mut c_void,
        ) -> u32;
    }

    fn win_error_string(code: u32) -> String {
        let mut buf = [0u16; 512];
        let flags = 0x00001000 | 0x00000200; // FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS
        let len = unsafe {
            FormatMessageW(
                flags,
                std::ptr::null(),
                code,
                0,
                buf.as_mut_ptr(),
                buf.len() as u32,
                std::ptr::null_mut(),
            )
        };
        if len > 0 {
            String::from_utf16_lossy(&buf[..len as usize])
                .trim_end()
                .to_string()
        } else {
            format!("Windows error code: {code}")
        }
    }

    pub unsafe fn dl_open(path: Option<&std::ffi::CStr>, _flags: i32) -> Result<*mut c_void, String> {
        let handle = match path {
            Some(p) => {
                let os_str = std::ffi::OsStr::new(p.to_str().unwrap_or(""));
                let wide: Vec<u16> = os_str.encode_wide().chain(std::iter::once(0)).collect();
                unsafe { LoadLibraryW(wide.as_ptr()) }
            }
            None => unsafe { GetModuleHandleW(std::ptr::null()) },
        };
        if handle.is_null() {
            let code = unsafe { GetLastError() };
            Err(win_error_string(code))
        } else {
            Ok(handle)
        }
    }

    pub unsafe fn dl_symbol(handle: *mut c_void, symbol: &std::ffi::CStr) -> Result<*mut c_void, String> {
        let sym_ptr = unsafe { GetProcAddress(handle, symbol.as_ptr()) };
        if sym_ptr.is_null() {
            let code = unsafe { GetLastError() };
            Err(win_error_string(code))
        } else {
            Ok(sym_ptr)
        }
    }

    pub unsafe fn dl_close(handle: *mut c_void) -> bool {
        if handle.is_null() {
            return true;
        }
        unsafe { FreeLibrary(handle) != 0 }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sharedc_open(path_p: *const u8, path_len: i64, flags: i32) -> *mut c_void {
    let path_str = unsafe { str_from_raw(path_p, path_len) };
    if path_str.is_empty() {
        set_last_error("library path cannot be empty".to_string());
        return std::ptr::null_mut();
    }
    let c_path = match CString::new(path_str) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(e.to_string());
            return std::ptr::null_mut();
        }
    };
    match unsafe { sys::dl_open(Some(&c_path), flags) } {
        Ok(handle) => {
            clear_last_error();
            handle
        }
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sharedc_open_self(flags: i32) -> *mut c_void {
    match unsafe { sys::dl_open(None, flags) } {
        Ok(handle) => {
            clear_last_error();
            handle
        }
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sharedc_symbol(
    handle: *mut c_void,
    name_p: *const u8,
    name_len: i64,
) -> *mut c_void {
    if handle.is_null() {
        set_last_error("invalid shared library handle (null)".to_string());
        return std::ptr::null_mut();
    }
    let name_str = unsafe { str_from_raw(name_p, name_len) };
    if name_str.is_empty() {
        set_last_error("symbol name cannot be empty".to_string());
        return std::ptr::null_mut();
    }
    let c_name = match CString::new(name_str) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(e.to_string());
            return std::ptr::null_mut();
        }
    };
    match unsafe { sys::dl_symbol(handle, &c_name) } {
        Ok(sym_ptr) => {
            clear_last_error();
            sym_ptr
        }
        Err(err) => {
            set_last_error(err);
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sharedc_close(handle: *mut c_void) -> bool {
    if !handle.is_null() {
        if let Ok(mut map) = meta_cache().lock() {
            map.remove(&(handle as usize));
        }
    }
    unsafe { sys::dl_close(handle) }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sharedc_yukino_has_meta(handle: *mut c_void) -> bool {
    ensure_meta_loaded(handle)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sharedc_yukino_info_field(
    handle: *mut c_void,
    field_idx: i32,
    out_len: *mut i64,
) -> *const u8 {
    if !ensure_meta_loaded(handle) {
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        return std::ptr::null();
    }
    let key = handle as usize;
    let val = match meta_cache().lock() {
        Ok(map) => map.get(&key).and_then(|meta| match field_idx {
            0 => Some(meta.package_name.clone()),
            1 => Some(meta.yukic_version.clone()),
            2 => Some(meta.target.clone()),
            3 => Some(meta.abi_hash.clone()),
            _ => None,
        }),
        Err(_) => None,
    };
    match val {
        Some(s) => set_str_result(s.as_bytes(), out_len),
        None => {
            if !out_len.is_null() {
                unsafe { *out_len = 0; }
            }
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sharedc_yukino_export_count(handle: *mut c_void) -> i64 {
    if !ensure_meta_loaded(handle) {
        return -1;
    }
    let key = handle as usize;
    match meta_cache().lock() {
        Ok(map) => map.get(&key).map_or(-1, |m| m.exports.len() as i64),
        Err(_) => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sharedc_yukino_export_field(
    handle: *mut c_void,
    idx: i64,
    field_idx: i32,
    out_len: *mut i64,
) -> *const u8 {
    if handle.is_null() || idx < 0 {
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        return std::ptr::null();
    }
    let key = handle as usize;
    let val = match meta_cache().lock() {
        Ok(map) => map.get(&key).and_then(|meta| {
            meta.exports.get(idx as usize).and_then(|exp| match field_idx {
                0 => Some(exp.name.clone()),
                1 => Some(exp.mangled.clone()),
                2 => Some(exp.signature.clone()),
                3 => Some(exp.abi.clone()),
                4 => Some(exp.kind.clone()),
                _ => None,
            })
        }),
        Err(_) => None,
    };
    match val {
        Some(s) => set_str_result(s.as_bytes(), out_len),
        None => {
            if !out_len.is_null() {
                unsafe { *out_len = 0; }
            }
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sharedc_yukino_load_ykni(
    handle: *mut c_void,
    path_p: *const u8,
    path_len: i64,
) -> bool {
    if handle.is_null() {
        set_last_error("invalid shared library handle (null)".to_string());
        return false;
    }
    let path_str = unsafe { str_from_raw(path_p, path_len) };
    if path_str.is_empty() {
        set_last_error("interface path cannot be empty".to_string());
        return false;
    }
    let text = match std::fs::read_to_string(path_str) {
        Ok(t) => t,
        Err(e) => {
            set_last_error(format!("failed to read interface '{path_str}': {e}"));
            return false;
        }
    };
    let meta = parse_ykni_meta(&text);
    if let Ok(mut map) = meta_cache().lock() {
        map.insert(handle as usize, meta);
    }
    clear_last_error();
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sharedc_yukino_find(
    handle: *mut c_void,
    name_p: *const u8,
    name_len: i64,
    sig_p: *const u8,
    sig_len: i64,
    out_status: *mut i32,
) -> *mut c_void {
    let write_status = |st: i32| {
        if !out_status.is_null() {
            unsafe { *out_status = st; }
        }
    };
    if handle.is_null() {
        write_status(3);
        set_last_error("invalid shared library handle (null)".to_string());
        return std::ptr::null_mut();
    }
    if !ensure_meta_loaded(handle) {
        write_status(3);
        set_last_error("shared library has no Yukino metadata (__yukino_metadata_v1) and no loaded .ykni interface".to_string());
        return std::ptr::null_mut();
    }
    let name_str = unsafe { str_from_raw(name_p, name_len) };
    let sig_str = unsafe { str_from_raw(sig_p, sig_len) };
    let norm_sig = normalize_sig(sig_str);
    let key = handle as usize;

    let matches: Vec<ExportRecord> = match meta_cache().lock() {
        Ok(map) => match map.get(&key) {
            Some(meta) => meta
                .exports
                .iter()
                .filter(|exp| exp.name == name_str)
                .filter(|exp| norm_sig.is_empty() || normalize_sig(&exp.signature) == norm_sig)
                .cloned()
                .collect(),
            None => Vec::new(),
        },
        Err(_) => Vec::new(),
    };

    if matches.is_empty() {
        write_status(1);
        if norm_sig.is_empty() {
            set_last_error(format!("Yukino symbol '{name_str}' not found in metadata"));
        } else {
            set_last_error(format!("Yukino symbol '{name_str}' with signature '{sig_str}' not found in metadata"));
        }
        return std::ptr::null_mut();
    }
    if matches.len() > 1 {
        let mut exact_mangled = None;
        for m in &matches {
            if norm_sig.is_empty() && m.signature.is_empty() {
                exact_mangled = Some(m.clone());
                break;
            }
        }
        if exact_mangled.is_none() {
            write_status(2);
            let candidates: Vec<&str> = matches.iter().map(|m| m.signature.as_str()).collect();
            set_last_error(format!(
                "ambiguous symbol '{name_str}'; matching signatures: {}",
                candidates.join(", ")
            ));
            return std::ptr::null_mut();
        }
    }

    let mangled = &matches[0].mangled;
    resolve_dlsym_with_status(handle, mangled, write_status)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sharedc_yukino_find_method(
    handle: *mut c_void,
    class_p: *const u8,
    class_len: i64,
    method_p: *const u8,
    method_len: i64,
    param_count: i32,
    out_status: *mut i32,
) -> *mut c_void {
    let write_status = |st: i32| {
        if !out_status.is_null() {
            unsafe { *out_status = st; }
        }
    };
    if handle.is_null() {
        write_status(3);
        set_last_error("invalid shared library handle (null)".to_string());
        return std::ptr::null_mut();
    }
    if !ensure_meta_loaded(handle) {
        write_status(3);
        set_last_error("shared library has no Yukino metadata (__yukino_metadata_v1) and no loaded .ykni interface".to_string());
        return std::ptr::null_mut();
    }
    let class_str = unsafe { str_from_raw(class_p, class_len) };
    let method_str = unsafe { str_from_raw(method_p, method_len) };
    let target_dot = format!("{class_str}.{method_str}");
    let target_colon = format!("{class_str}::{method_str}");
    let key = handle as usize;

    let matches: Vec<ExportRecord> = match meta_cache().lock() {
        Ok(map) => match map.get(&key) {
            Some(meta) => meta
                .exports
                .iter()
                .filter(|exp| exp.name == target_dot || exp.name == target_colon)
                .filter(|exp| param_count < 0 || count_sig_params(&exp.signature) == param_count)
                .cloned()
                .collect(),
            None => Vec::new(),
        },
        Err(_) => Vec::new(),
    };

    if matches.is_empty() {
        write_status(1);
        set_last_error(format!("Yukino method '{target_dot}' not found in metadata"));
        return std::ptr::null_mut();
    }
    if matches.len() > 1 {
        write_status(2);
        let candidates: Vec<&str> = matches.iter().map(|m| m.signature.as_str()).collect();
        set_last_error(format!(
            "ambiguous method '{target_dot}'; matching signatures: {}",
            candidates.join(", ")
        ));
        return std::ptr::null_mut();
    }

    let mangled = &matches[0].mangled;
    resolve_dlsym_with_status(handle, mangled, write_status)
}

fn resolve_dlsym_with_status<F: Fn(i32)>(
    handle: *mut c_void,
    mangled: &str,
    write_status: F,
) -> *mut c_void {
    let c_name = match CString::new(mangled) {
        Ok(s) => s,
        Err(e) => {
            write_status(4);
            set_last_error(e.to_string());
            return std::ptr::null_mut();
        }
    };
    match unsafe { sys::dl_symbol(handle, &c_name) } {
        Ok(sym_ptr) => {
            write_status(0);
            clear_last_error();
            sym_ptr
        }
        Err(err) => {
            write_status(4);
            set_last_error(format!("symbol '{mangled}' resolved in metadata but missing in library: {err}"));
            std::ptr::null_mut()
        }
    }
}

fn ensure_meta_loaded(handle: *mut c_void) -> bool {
    if handle.is_null() {
        return false;
    }
    let key = handle as usize;
    if let Ok(map) = meta_cache().lock() {
        if map.contains_key(&key) {
            return true;
        }
    }
    let sym = match unsafe { sys::dl_symbol(handle, c"__yukino_metadata_v1") } {
        Ok(p) if !p.is_null() => p,
        _ => return false,
    };
    let meta_fn: extern "C" fn(*mut i64) -> *const u8 = unsafe { std::mem::transmute(sym) };
    let mut len: i64 = 0;
    let ptr = meta_fn(&mut len);
    if ptr.is_null() || len <= 0 {
        return false;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    let Ok(json_str) = std::str::from_utf8(slice) else {
        return false;
    };
    let Some(parsed) = parse_shared_export_json(json_str) else {
        return false;
    };
    if let Ok(mut map) = meta_cache().lock() {
        map.insert(key, parsed);
    }
    true
}

fn normalize_sig(sig: &str) -> String {
    sig.chars().filter(|c| !c.is_whitespace()).collect()
}

fn count_sig_params(sig: &str) -> i32 {
    let Some(open_idx) = sig.find('(') else {
        return -1;
    };
    let rest = &sig[open_idx + 1..];
    let mut depth_paren = 1i32;
    let mut depth_angle = 0i32;
    let mut depth_bracket = 0i32;
    let mut has_content = false;
    let mut commas = 0i32;

    for ch in rest.chars() {
        match ch {
            '(' => {
                depth_paren += 1;
                has_content = true;
            }
            ')' => {
                depth_paren -= 1;
                if depth_paren == 0 {
                    break;
                }
                has_content = true;
            }
            '<' => {
                depth_angle += 1;
                has_content = true;
            }
            '>' => {
                depth_angle = (depth_angle - 1).max(0);
                has_content = true;
            }
            '[' => {
                depth_bracket += 1;
                has_content = true;
            }
            ']' => {
                depth_bracket = (depth_bracket - 1).max(0);
                has_content = true;
            }
            ',' if depth_paren == 1 && depth_angle == 0 && depth_bracket == 0 => {
                commas += 1;
            }
            c if !c.is_whitespace() => {
                has_content = true;
            }
            _ => {}
        }
    }
    if !has_content { 0 } else { commas + 1 }
}

enum JsonVal {
    Str(String),
    Arr(Vec<JsonVal>),
    Obj(Vec<(String, JsonVal)>),
    Other,
}

struct JsonReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonReader<'a> {
    fn new(input: &'a str) -> Self {
        Self { bytes: input.as_bytes(), pos: 0 }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.pos).copied()
    }

    fn parse_val(&mut self) -> Option<JsonVal> {
        match self.peek()? {
            b'"' => self.parse_str().map(JsonVal::Str),
            b'{' => self.parse_obj(),
            b'[' => self.parse_arr(),
            _ => {
                self.skip_primitive();
                Some(JsonVal::Other)
            }
        }
    }

    fn parse_str(&mut self) -> Option<String> {
        self.skip_ws();
        if self.bytes.get(self.pos) != Some(&b'"') {
            return None;
        }
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            self.pos += 1;
            if b == b'"' {
                return Some(out);
            }
            if b == b'\\' {
                let esc = *self.bytes.get(self.pos)?;
                self.pos += 1;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        if self.pos + 4 <= self.bytes.len() {
                            if let Ok(hex_str) = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4]) {
                                if let Ok(code) = u32::from_str_radix(hex_str, 16) {
                                    if let Some(ch) = char::from_u32(code) {
                                        out.push(ch);
                                    }
                                }
                            }
                            self.pos += 4;
                        }
                    }
                    other => out.push(other as char),
                }
            } else {
                // fast-path utf-8 multi-byte preservation by decoding chunks
                let start = self.pos - 1;
                while self.pos < self.bytes.len() && self.bytes[self.pos] >= 0x80 {
                    self.pos += 1;
                }
                if self.pos > start {
                    if let Ok(s) = std::str::from_utf8(&self.bytes[start..self.pos]) {
                        out.push_str(s);
                    }
                }
            }
        }
        None
    }

    fn parse_arr(&mut self) -> Option<JsonVal> {
        self.skip_ws();
        self.pos += 1;
        let mut items = Vec::new();
        loop {
            if self.peek()? == b']' {
                self.pos += 1;
                return Some(JsonVal::Arr(items));
            }
            items.push(self.parse_val()?);
            if self.peek() == Some(b',') {
                self.pos += 1;
            }
        }
    }

    fn parse_obj(&mut self) -> Option<JsonVal> {
        self.skip_ws();
        self.pos += 1;
        let mut fields = Vec::new();
        loop {
            if self.peek()? == b'}' {
                self.pos += 1;
                return Some(JsonVal::Obj(fields));
            }
            let key = self.parse_str()?;
            if self.peek()? != b':' {
                return None;
            }
            self.pos += 1;
            let val = self.parse_val()?;
            fields.push((key, val));
            if self.peek() == Some(b',') {
                self.pos += 1;
            }
        }
    }

    fn skip_primitive(&mut self) {
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == b',' || b == b'}' || b == b']' || b.is_ascii_whitespace() {
                break;
            }
            self.pos += 1;
        }
    }
}

fn parse_shared_export_json(input: &str) -> Option<LibraryMeta> {
    let mut reader = JsonReader::new(input);
    let JsonVal::Obj(root) = reader.parse_val()? else {
        return None;
    };
    let mut meta = LibraryMeta {
        package_name: String::new(),
        yukic_version: String::new(),
        target: String::new(),
        abi_hash: String::new(),
        exports: Vec::new(),
    };
    for (k, v) in root {
        match (k.as_str(), v) {
            ("package_name", JsonVal::Str(s)) => meta.package_name = s,
            ("yukic_version", JsonVal::Str(s)) => meta.yukic_version = s,
            ("target", JsonVal::Str(s)) => meta.target = s,
            ("abi_hash", JsonVal::Str(s)) => meta.abi_hash = s,
            ("exports", JsonVal::Arr(items)) => {
                for item in items {
                    if let JsonVal::Obj(fields) = item {
                        let mut rec = ExportRecord {
                            name: String::new(),
                            mangled: String::new(),
                            signature: String::new(),
                            abi: String::new(),
                            kind: String::new(),
                        };
                        for (fk, fv) in fields {
                            if let JsonVal::Str(fs) = fv {
                                match fk.as_str() {
                                    "name" => rec.name = fs,
                                    "mangled" => rec.mangled = fs,
                                    "signature" => rec.signature = fs,
                                    "abi" => rec.abi = fs,
                                    "kind" => rec.kind = fs,
                                    _ => {}
                                }
                            }
                        }
                        if !rec.name.is_empty() && !rec.mangled.is_empty() {
                            meta.exports.push(rec);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Some(meta)
}

fn extract_quoted_attr_arg(line: &str) -> Option<String> {
    let start = line.find('"')? + 1;
    let rest = &line[start..];
    let mut out = String::new();
    let mut escape = false;
    for ch in rest.chars() {
        if escape {
            match ch {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                other => out.push(other),
            }
            escape = false;
        } else if ch == '\\' {
            escape = true;
        } else if ch == '"' {
            return Some(out);
        } else {
            out.push(ch);
        }
    }
    None
}

fn extract_kv_str(line: &str, key: &str) -> Option<String> {
    let pat = format!("{key} = \"");
    let pos = line.find(&pat)? + pat.len();
    extract_quoted_attr_arg(&line[pos - 1..])
}

fn parse_ykni_meta(text: &str) -> LibraryMeta {
    let mut meta = LibraryMeta {
        package_name: String::new(),
        yukic_version: String::new(),
        target: String::new(),
        abi_hash: String::new(),
        exports: Vec::new(),
    };
    let mut pending_sym: Option<String> = None;
    let mut owner_stack: Vec<(String, i32)> = Vec::new();
    let mut brace_depth = 0i32;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if line.starts_with("#![ykni::abi(") {
            if let Some(v) = extract_kv_str(line, "yukic") { meta.yukic_version = v; }
            if let Some(v) = extract_kv_str(line, "target") { meta.target = v; }
            if let Some(v) = extract_kv_str(line, "abi_hash") { meta.abi_hash = v; }
            continue;
        }
        if line.starts_with("#![ykni::package(") {
            if let Some(v) = extract_kv_str(line, "name") { meta.package_name = v; }
            continue;
        }
        if line.starts_with("#[ykni::symbol(") {
            pending_sym = extract_quoted_attr_arg(line);
            continue;
        }
        if line.starts_with("#[") || line.starts_with("#![") {
            continue;
        }

        if let Some(rest) = line.strip_prefix("public class ")
            .or_else(|| line.strip_prefix("public locked class "))
        {
            let name = rest.split(|c: char| !c.is_alphanumeric() && c != '_').next().unwrap_or("");
            if !name.is_empty() && line.contains('{') {
                owner_stack.push((name.to_string(), brace_depth + 1));
            }
        } else if let Some(sym) = pending_sym.take() {
            let owner = owner_stack.last().map(|(n, _)| n.as_str());
            let (abi, kind, raw_name, sig) = if let Some(rest) = line.strip_prefix("public cfun ") {
                let fn_name = rest.split('(').next().unwrap_or("").trim();
                let sig_body = rest.strip_prefix(fn_name).unwrap_or("").split(" as ").next().unwrap_or("").trim();
                let full_sig = if sig_body.contains("->") {
                    format!("cfun{sig_body}")
                } else {
                    format!("cfun{sig_body} -> void")
                };
                ("cdecl", "cfun", fn_name.to_string(), full_sig)
            } else if let Some(rest) = line
                .strip_prefix("public fun ")
                .or_else(|| line.strip_prefix("public override fun "))
                .or_else(|| line.strip_prefix("static fun "))
            {
                let is_static = line.starts_with("static fun ");
                let (fn_name, is_acc) = if let Some(r) = rest.strip_prefix("get ") {
                    (format!("get_{}", r.split('(').next().unwrap_or("").trim()), true)
                } else if let Some(r) = rest.strip_prefix("set ") {
                    (format!("set_{}", r.split('(').next().unwrap_or("").trim()), true)
                } else {
                    (rest.split(|c| c == '(' || c == '<').next().unwrap_or("").trim().to_string(), false)
                };
                let after_open = rest.find('(').map_or("()", |i| &rest[i..]);
                let sig_body = after_open.split('{').next().unwrap_or(after_open).trim();
                let full_sig = if sig_body.contains("->") {
                    format!("fun{sig_body}")
                } else {
                    format!("fun{sig_body} -> void")
                };
                let (full_name, kind) = if let Some(cls) = owner {
                    let k = if is_acc {
                        "accessor"
                    } else if is_static {
                        "static_method"
                    } else if fn_name == "init" {
                        "ctor"
                    } else {
                        "method"
                    };
                    (format!("{cls}.{fn_name}"), k)
                } else {
                    (fn_name, "fun")
                };
                ("yukino_fastcall", kind, full_name, full_sig)
            } else {
                continue;
            };
            if !raw_name.is_empty() {
                meta.exports.push(ExportRecord {
                    name: raw_name,
                    mangled: sym,
                    signature: sig,
                    abi: abi.to_string(),
                    kind: kind.to_string(),
                });
            }
        }

        for ch in line.chars() {
            if ch == '{' {
                brace_depth += 1;
            } else if ch == '}' {
                brace_depth -= 1;
                while let Some((_, target_depth)) = owner_stack.last() {
                    if brace_depth < *target_depth {
                        owner_stack.pop();
                    } else {
                        break;
                    }
                }
            }
        }
    }
    meta
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sharedc_last_error(out_len: *mut i64) -> *const u8 {
    LAST_ERROR.with(|cell| {
        let msg = cell.borrow();
        if msg.is_empty() {
            if !out_len.is_null() {
                unsafe { *out_len = 0; }
            }
            std::ptr::null()
        } else {
            set_err_result(msg.as_bytes(), out_len)
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sharedc_filename(
    name_p: *const u8,
    name_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    let name = unsafe { str_from_raw(name_p, name_len) };
    #[cfg(target_os = "windows")]
    let full = format!("{name}.dll");
    #[cfg(target_os = "macos")]
    let full = format!("lib{name}.dylib");
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let full = format!("lib{name}.so");

    set_str_result(full.as_bytes(), out_len)
}
