use crate::panic::yukino_raise_panic;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_is_valid_utf8(ptr: *const u8, len: usize) -> bool {
    if ptr.is_null() {
        return len == 0;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    std::str::from_utf8(slice).is_ok()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_utf8_validate(
    ptr: *const u8,
    len: usize,
    err_offset: *mut usize,
) -> bool {
    if ptr.is_null() {
        if len == 0 { return true; }
        if !err_offset.is_null() { unsafe { *err_offset = 0 }; }
        return false;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    match std::str::from_utf8(slice) {
        Ok(_) => true,
        Err(e) => {
            if !err_offset.is_null() { unsafe { *err_offset = e.valid_up_to() }; }
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_validate_utf8(ptr: *const u8, len: usize) -> i32 {
    if ptr.is_null() {
        if len == 0 { return 0; }
        let msg = b"R0402: invalid_utf8_conversion: null pointer with non-zero length";
        unsafe { yukino_raise_panic(msg.as_ptr(), msg.len()) };
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    match std::str::from_utf8(slice) {
        Ok(_) => 0,
        Err(e) => {
            let offset = e.valid_up_to();
            let msg = format!("R0402: invalid_utf8_conversion: invalid sequence at byte offset {offset}");
            unsafe { yukino_raise_panic(msg.as_ptr(), msg.len()) };
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_utf8_char_count(ptr: *const u8, len: usize) -> i64 {
    if ptr.is_null() { return if len == 0 { 0 } else { -1 }; }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    match std::str::from_utf8(slice) {
        Ok(s) => s.chars().count() as i64,
        Err(_) => -1,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_utf8_char_at(
    ptr: *const u8,
    len: usize,
    char_index: usize,
) -> u32 {
    if ptr.is_null() || len == 0 { return 0; }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    if let Ok(s) = std::str::from_utf8(slice) {
        s.chars().nth(char_index).map_or(0, |c| c as u32)
    } 



    else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_validate_char_code(code: u32) -> bool {
    char::from_u32(code).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_utf8() {
        let valid = "Привет, мир!".as_bytes();
        unsafe {
            assert!(yukino_rt_is_valid_utf8(valid.as_ptr(), valid.len()));
            let mut err = 0usize;
            assert!(yukino_rt_utf8_validate(valid.as_ptr(), valid.len(), &raw mut err));
            assert_eq!(yukino_rt_utf8_char_count(valid.as_ptr(), valid.len()), 12);
            assert_eq!(yukino_rt_utf8_char_at(valid.as_ptr(), valid.len(), 0), 'П' as u32);
            assert_eq!(yukino_rt_utf8_char_at(valid.as_ptr(), valid.len(), 11), '!' as u32);
            assert_eq!(yukino_rt_utf8_char_at(valid.as_ptr(), valid.len(), 12), 0);
        }
    }

    #[test]
    fn test_invalid_utf8() {
        let invalid = [0xFF, 0xFE, 0xFD];
        unsafe {
            assert!(!yukino_rt_is_valid_utf8(invalid.as_ptr(), invalid.len()));
            let mut err = 999usize;
            assert!(!yukino_rt_utf8_validate(invalid.as_ptr(), invalid.len(), &raw mut err));
            assert_eq!(err, 0);
            assert_eq!(yukino_rt_utf8_char_count(invalid.as_ptr(), invalid.len()), -1);
        }
    }

    #[test]
    fn test_validate_char_code() {
        unsafe {
            assert!(yukino_rt_validate_char_code('a' as u32));
            assert!(yukino_rt_validate_char_code('🦀' as u32));
            assert!(!yukino_rt_validate_char_code(0xD800)); // surrogate
            assert!(!yukino_rt_validate_char_code(0x110000)); // out of range
        }
    }
}
