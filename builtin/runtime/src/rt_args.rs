use crate::region::{YukinoRegion, yukino_region_alloc};
use crate::rt_cast::{YukinoSlice, YukinoString};

unsafe fn c_str_len(ptr: *const u8) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let mut len = 0;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    len
}


// yuki reg or argv to an array of argc cstr ptrs
// Out pointer: 24-byte struct return diverges between Win64 and SysV.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_get_args(
    out: *mut YukinoSlice,
    region: *mut YukinoRegion,
    argc: i32,
    argv: *const *const u8,
) {
    if argc <= 0 || argv.is_null() || region.is_null() {
        unsafe { out.write(YukinoSlice {
            ptr: std::ptr::null_mut(),
            len: 0,
            cap: 0,
        }) };
        return;
    }

    let count = argc as usize;
    let elem_size = std::mem::size_of::<YukinoString>();
    let elem_align = std::mem::align_of::<YukinoString>();
    let total_bytes = count.saturating_mul(elem_size);

    let raw_buf = unsafe { yukino_region_alloc(region, total_bytes, elem_align) };
    if raw_buf.is_null() {
        unsafe { out.write(YukinoSlice {
            ptr: std::ptr::null_mut(),
            len: 0,
            cap: 0,
        }) };
        return;
    }

    let slice_ptr = raw_buf.cast::<YukinoString>();
    for i in 0..count {
        let arg_ptr = unsafe { *argv.add(i) };
        let len = unsafe { c_str_len(arg_ptr) };

    
        unsafe { slice_ptr.add(i).write(YukinoString { ptr: arg_ptr, len }); }
    }

    unsafe { out.write(YukinoSlice {
        ptr: raw_buf,
        len: count,
        cap: count,
    }) };
}

// out pointer result: 16-byte struct return diverges between Win64 and codegen
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_get_raw_args(
    out: *mut YukinoString,
    region: *mut YukinoRegion,
    argc: i32,
    argv: *const *const u8,
) {
    if argc <= 0 || argv.is_null() || region.is_null() {
        unsafe { out.write(YukinoString {
            ptr: std::ptr::null(),
            len: 0,
        }) };
        return;
    }

    let count = argc as usize;
    let mut total_len = 0;
    for i in 0..count {
        let arg_ptr = unsafe { *argv.add(i) };
        total_len += unsafe { c_str_len(arg_ptr) };
    }
    if count > 1 { total_len += count - 1; }

    let raw_buf = unsafe { yukino_region_alloc(region, total_len, 1) };
    if raw_buf.is_null() {
        unsafe { out.write(YukinoString {
            ptr: std::ptr::null(),
            len: 0,
        }) };
        return;
    }

    let mut offset = 0;
    for i in 0..count {
        let arg_ptr = unsafe { *argv.add(i) };
        let len = unsafe { c_str_len(arg_ptr) };
        if len > 0 {
            unsafe { std::ptr::copy_nonoverlapping(arg_ptr, raw_buf.add(offset), len); }
            offset += len;
        }
        if i + 1 < count {
            unsafe { *raw_buf.add(offset) = b' '; }
            offset += 1;
        }
    }

    unsafe { out.write(YukinoString {
        ptr: raw_buf,
        len: total_len,
    }) };
}
