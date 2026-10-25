use std::cell::RefCell;
use std::ffi::{c_void, CString};

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
    unsafe { sys::dl_close(handle) }
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
