#[cfg(target_os = "android")]
use std::ffi::CString;

#[allow(dead_code)]
unsafe fn str_from_raw<'a>(ptr: *const u8, len: i64) -> &'a str {
    if ptr.is_null() || len <= 0 {
        return "";
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    std::str::from_utf8(slice).unwrap_or("")
}

#[cfg(target_os = "android")]
unsafe extern "C" {
    fn __android_log_print(
        prio: std::ffi::c_int,
        tag: *const std::ffi::c_char,
        fmt: *const std::ffi::c_char,
        ...
    ) -> std::ffi::c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_android_log_print(
    prio: i32,
    tag_p: *const u8,
    tag_len: i64,
    text_p: *const u8,
    text_len: i64,
) -> i32 {
    #[cfg(target_os = "android")]
    {
        let tag = unsafe { str_from_raw(tag_p, tag_len) };
        let text = unsafe { str_from_raw(text_p, text_len) };
        let c_tag = CString::new(tag).unwrap_or_default();
        let c_text = CString::new(text).unwrap_or_default();
        let c_fmt = CString::new("%s").unwrap_or_default();
        unsafe { __android_log_print(prio, c_tag.as_ptr(), c_fmt.as_ptr(), c_text.as_ptr()) }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (prio, tag_p, tag_len, text_p, text_len);
        -1
    }
}
