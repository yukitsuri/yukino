use crate::region::{YukinoRegion, yukino_region_alloc};

// TODO: err to yukino null

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct YukinoString {
    pub ptr: *const u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct YukinoSlice {
    pub ptr: *mut u8,
    pub len: usize,
    pub cap: usize,
}

unsafe fn copy_to_region(region: *mut YukinoRegion, bytes: &[u8]) -> YukinoString {
    if region.is_null() || bytes.is_empty() {
        return YukinoString {
            ptr: std::ptr::null(),
            len: 0,
        };
    }
    let dest = unsafe { yukino_region_alloc(region, bytes.len(), 1) };
    if dest.is_null() {
        return YukinoString {
            ptr: std::ptr::null(),
            len: 0,
        };
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), dest, bytes.len());
    }
    YukinoString {
        ptr: dest,
        len: bytes.len(),
    }
}

// string results use an out pointer: a 16-byte struct return diverges on Win64 (hidden sret)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_char_to_string(
    out: *mut YukinoString,
    region: *mut YukinoRegion,
    ch: u32,
) {
    let mut buf = [0u8; 4];
    let result = match char::from_u32(ch) {
        Some(c) => {
            let s = c.encode_utf8(&mut buf);
            unsafe { copy_to_region(region, s.as_bytes()) }
        }
        None => YukinoString {
            ptr: std::ptr::null(),
            len: 0,
        },
    };
    unsafe { out.write(result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_string_to_char(ptr: *const u8, len: usize) -> u32 {
    if ptr.is_null() || len == 0 {
        let msg = b"cannot cast empty or null string to char";
        unsafe { crate::panic::yukino_raise_panic(msg.as_ptr(), msg.len()); }
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    match std::str::from_utf8(slice).ok().and_then(|s| s.chars().next()) {
        Some(c) => c as u32,
        None => {
            let msg = b"invalid utf-8 in string cast to char";
            unsafe { crate::panic::yukino_raise_panic(msg.as_ptr(), msg.len()); }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_int64_to_string(
    out: *mut YukinoString,
    region: *mut YukinoRegion,
    val: i64,
) {
    let s = val.to_string();
    let result = unsafe { copy_to_region(region, s.as_bytes()) };
    unsafe { out.write(result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_uint64_to_string(
    out: *mut YukinoString,
    region: *mut YukinoRegion,
    val: u64,
) {
    let s = val.to_string();
    let result = unsafe { copy_to_region(region, s.as_bytes()) };
    unsafe { out.write(result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_float64_to_string(
    out: *mut YukinoString,
    region: *mut YukinoRegion,
    val: f64,
) {
    let s = val.to_string();
    let result = unsafe { copy_to_region(region, s.as_bytes()) };
    unsafe { out.write(result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_bool_to_string(
    out: *mut YukinoString,
    region: *mut YukinoRegion,
    val: bool,
) {
    let s = if val { "true" } else { "false" };
    let result = unsafe { copy_to_region(region, s.as_bytes()) };
    unsafe { out.write(result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_string_to_bool(ptr: *const u8, len: usize) -> bool {
    if ptr.is_null() || len == 0 {
        return false;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    match std::str::from_utf8(slice).map(|s| s.trim()) {
        Ok("true") | Ok("1") => true,
        Ok("false") | Ok("0") => false,
        _ => {
            let msg = b"invalid boolean syntax in string cast to bool";
            unsafe { crate::panic::yukino_raise_panic(msg.as_ptr(), msg.len()); }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_string_to_int64(ptr: *const u8, len: usize) -> i64 {
    if ptr.is_null() || len == 0 {
        let msg = b"cannot cast empty or null string to int64";
        unsafe { crate::panic::yukino_raise_panic(msg.as_ptr(), msg.len()); }
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    match std::str::from_utf8(slice).ok().and_then(|s| s.trim().parse::<i64>().ok()) {
        Some(val) => val,
        None => {
            let msg = b"invalid integer syntax in string cast to int64";
            unsafe { crate::panic::yukino_raise_panic(msg.as_ptr(), msg.len()); }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_string_to_uint64(ptr: *const u8, len: usize) -> u64 {
    if ptr.is_null() || len == 0 {
        let msg = b"cannot cast empty or null string to uint64";
        unsafe { crate::panic::yukino_raise_panic(msg.as_ptr(), msg.len()); }
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    match std::str::from_utf8(slice).ok().and_then(|s| s.trim().parse::<u64>().ok()) {
        Some(val) => val,
        None => {
            let msg = b"invalid integer syntax in string cast to uint64";
            unsafe { crate::panic::yukino_raise_panic(msg.as_ptr(), msg.len()); }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_string_to_float64(ptr: *const u8, len: usize) -> f64 {
    if ptr.is_null() || len == 0 {
        let msg = b"cannot cast empty or null string to float64";
        unsafe { crate::panic::yukino_raise_panic(msg.as_ptr(), msg.len()); }
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    match std::str::from_utf8(slice).ok().and_then(|s| s.trim().parse::<f64>().ok()) {
        Some(val) => val,
        None => {
            let msg = b"invalid float syntax in string cast to float64";
            unsafe { crate::panic::yukino_raise_panic(msg.as_ptr(), msg.len()); }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_ptr_to_string(
    out: *mut YukinoString,
    region: *mut YukinoRegion,
    ptr: *const u8,
) {
    let s = format!("0x{:x}", ptr as usize);
    let result = unsafe { copy_to_region(region, s.as_bytes()) };
    unsafe { out.write(result) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_string_to_ptr(ptr: *const u8, len: usize) -> *const u8 {
    if ptr.is_null() || len == 0 { return std::ptr::null(); }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    let text = match std::str::from_utf8(slice) {
        Ok(t) => t.trim(),
        Err(_) => return std::ptr::null(),
    };
    let hex_str = text.strip_prefix("0x").unwrap_or(text);
    usize::from_str_radix(hex_str, 16).map_or(std::ptr::null(), |addr| addr as *const u8)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_string_from_raw_parts(
    out: *mut YukinoString,
    ptr: *const u8,
    len: usize,
) {
    unsafe { out.write(YukinoString { ptr, len }) };
}

// Out pointer: 24-byte struct return diverges between Win64 and SysV.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_slice_from_raw_parts(
    out: *mut YukinoSlice,
    ptr: *mut u8,
    len: usize,
    cap: usize,
) {
    unsafe { out.write(YukinoSlice { ptr, len, cap }) };
}

// Out pointer: 24-byte struct return diverges between Win64 and SysV.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_string_to_chars(
    out: *mut YukinoSlice,
    region: *mut YukinoRegion,
    ptr: *const u8,
    len: usize,
) {
    if region.is_null() || ptr.is_null() || len == 0 {
        unsafe { out.write(YukinoSlice {
            ptr: std::ptr::null_mut(),
            len: 0,
            cap: 0,
        }) };
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    let Ok(s) = std::str::from_utf8(slice) else {
        unsafe { out.write(YukinoSlice {
            ptr: std::ptr::null_mut(),
            len: 0,
            cap: 0,
        }) };
        return;
    };
    let count = s.chars().count();
    let bytes_needed = count.saturating_mul(std::mem::size_of::<u32>());
    let dest = unsafe { yukino_region_alloc(region, bytes_needed, std::mem::align_of::<u32>()) };
    if dest.is_null() {
        unsafe { out.write(YukinoSlice {
            ptr: std::ptr::null_mut(),
            len: 0,
            cap: 0,
        }) };
        return;
    }
    let u32_buf = dest.cast::<u32>();
    for (i, ch) in s.chars().enumerate() {
        unsafe { *u32_buf.add(i) = ch as u32 };
    }
    unsafe { out.write(YukinoSlice {
        ptr: dest,
        len: count,
        cap: count,
    }) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_chars_to_string(
    out: *mut YukinoString,
    region: *mut YukinoRegion,
    ptr: *const u32,
    len: usize,
) {
    if region.is_null() || ptr.is_null() || len == 0 {
        unsafe { out.write(YukinoString {
            ptr: std::ptr::null(),
            len: 0,
        }) };
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    let mut total_bytes = 0usize;
    for &code in slice {
        if let Some(ch) = char::from_u32(code) {
            total_bytes += ch.len_utf8();
        }
    }
    if total_bytes == 0 {
        unsafe { out.write(YukinoString {
            ptr: std::ptr::null(),
            len: 0,
        }) };
        return;
    }
    let dest = unsafe { yukino_region_alloc(region, total_bytes, 1) };
    if dest.is_null() {
        unsafe { out.write(YukinoString {
            ptr: std::ptr::null(),
            len: 0,
        }) };
        return;
    }
    let mut offset = 0usize;
    let mut buf = [0u8; 4];
    for &code in slice {
        if let Some(ch) = char::from_u32(code) {
            let encoded = ch.encode_utf8(&mut buf);
            unsafe {
                std::ptr::copy_nonoverlapping(encoded.as_ptr(), dest.add(offset), encoded.len());
            }
            offset += encoded.len();
        }
    }
    unsafe { out.write(YukinoString {
        ptr: dest,
        len: total_bytes,
    }) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_cstr_to_string(
    out: *mut YukinoString,
    region: *mut YukinoRegion,
    cstr: *const u8,
) {
    if region.is_null() || cstr.is_null() {
        unsafe { out.write(YukinoString {
            ptr: std::ptr::null(),
            len: 0,
        }) };
        return;
    }
    let mut len = 0usize;
    unsafe {
        while *cstr.add(len) != 0 { len += 1; }
        let slice = std::slice::from_raw_parts(cstr, len);
        let result = copy_to_region(region, slice);
        out.write(result);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_string_to_cstr(
    region: *mut YukinoRegion,
    ptr: *const u8,
    len: usize,
) -> *const u8 {
    if region.is_null() { return std::ptr::null(); }
    let dest = unsafe { yukino_region_alloc(region, len.saturating_add(1), 1) };
    if dest.is_null() { return std::ptr::null(); }
    unsafe {
        if !ptr.is_null() && len > 0 { std::ptr::copy_nonoverlapping(ptr, dest, len); }
        *dest.add(len) = 0;
    }
    dest
}
