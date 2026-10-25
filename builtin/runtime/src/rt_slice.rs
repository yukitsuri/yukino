use crate::region::{YukinoRegion, yukino_region_alloc};
use crate::rt_cast::YukinoSlice;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_slice_push(
    region: *mut YukinoRegion,
    slice: *mut YukinoSlice,
    elem_ptr: *const u8,
    elem_size: usize,
    elem_align: usize,
) {
    if region.is_null() || slice.is_null() || elem_ptr.is_null() || elem_size == 0 { return; }

    let mut s = unsafe { std::ptr::read_unaligned(slice) };
    if s.len >= s.cap {
        let new_cap = if s.cap == 0 {
            4
        } else {
            s.cap.saturating_mul(2)
        };
        let new_bytes = new_cap.saturating_mul(elem_size);
        let new_buf = unsafe { yukino_region_alloc(region, new_bytes, elem_align.max(1)) };
        if new_buf.is_null() {
            return;
        }

        if s.len > 0 && !s.ptr.is_null() && s.ptr != new_buf {
            unsafe {
                std::ptr::copy(s.ptr, new_buf, s.len * elem_size);
            }
        }
        s.ptr = new_buf;
        s.cap = new_cap;
    }

    unsafe {
        std::ptr::copy(elem_ptr, s.ptr.add(s.len * elem_size), elem_size);
    }
    s.len += 1;
    unsafe {
        std::ptr::write_unaligned(slice, s);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_slice_del(
    slice: *mut YukinoSlice,
    index: usize,
    elem_size: usize,
) {
    if slice.is_null() {
        let msg = b"slice pointer is null in slice.del";
        unsafe {
            crate::panic::yukino_raise_panic(msg.as_ptr(), msg.len());
        }
    }
    let mut s = unsafe { std::ptr::read_unaligned(slice) };
    if index >= s.len {
        let msg = b"slice index out of bounds in slice.del";
        unsafe {
            crate::panic::yukino_raise_panic(msg.as_ptr(), msg.len());
        }
    }
    if elem_size == 0 || s.ptr.is_null() {
        return;
    }

    let items_to_move = s.len - 1 - index;
    if items_to_move > 0 {
        unsafe {
            let dst = s.ptr.add(index * elem_size);
            let src = s.ptr.add((index + 1) * elem_size);
            std::ptr::copy(src, dst, items_to_move * elem_size);
        }
    }
    s.len -= 1;
    unsafe {
        std::ptr::write_unaligned(slice, s);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_rt_slice_reserve(
    region: *mut YukinoRegion,
    slice: *mut YukinoSlice,
    additional: usize,
    elem_size: usize,
    elem_align: usize,
) {
    if region.is_null() || slice.is_null() || elem_size == 0 {
        return;
    }
    let mut s = unsafe { std::ptr::read_unaligned(slice) };
    let required_cap = match s.len.checked_add(additional) {
        Some(c) => c,
        None => return,
    };
    if required_cap <= s.cap {
        return;
    }

    let new_cap = required_cap.max(s.cap.saturating_mul(2));
    let new_bytes = new_cap.saturating_mul(elem_size);
    let new_buf = unsafe { yukino_region_alloc(region, new_bytes, elem_align.max(1)) };
    if new_buf.is_null() {
        return;
    }

    if s.len > 0 && !s.ptr.is_null() && s.ptr != new_buf {
        unsafe {
            std::ptr::copy(s.ptr, new_buf, s.len * elem_size);
        }
    }
    s.ptr = new_buf;
    s.cap = new_cap;
    unsafe {
        std::ptr::write_unaligned(slice, s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::region::{yukino_region_destroy, yukino_region_init};

    #[test]
    fn test_slice_push_and_del() {
        let mut reg = std::mem::MaybeUninit::<YukinoRegion>::uninit();
        unsafe {
            yukino_region_init(reg.as_mut_ptr(), 4096);
            let reg_ptr = reg.as_mut_ptr();

            let mut slice = YukinoSlice {
                ptr: std::ptr::null_mut(),
                len: 0,
                cap: 0,
            };

            // push els 10, 20, 30
            let val1: i32 = 10;
            yukino_rt_slice_push(reg_ptr, &mut slice, std::ptr::addr_of!(val1).cast(), 4, 4);
            assert_eq!(slice.len, 1);
            assert!(slice.cap >= 1);

            let val2: i32 = 20;
            yukino_rt_slice_push(reg_ptr, &mut slice, std::ptr::addr_of!(val2).cast(), 4, 4);
            assert_eq!(slice.len, 2);

            let val3: i32 = 30;
            yukino_rt_slice_push(reg_ptr, &mut slice, std::ptr::addr_of!(val3).cast(), 4, 4);
            assert_eq!(slice.len, 3);

            let items = std::slice::from_raw_parts(slice.ptr.cast::<i32>(), slice.len);
            assert_eq!(items, &[10, 20, 30]);

            // del el at indx 1 (val 20)
            yukino_rt_slice_del(&mut slice, 1, 4);
            assert_eq!(slice.len, 2);
            let remaining = std::slice::from_raw_parts(slice.ptr.cast::<i32>(), slice.len);
            assert_eq!(remaining, &[10, 30]);

            // res add..l
            yukino_rt_slice_reserve(reg_ptr, &mut slice, 50, 4, 4);
            assert!(slice.cap >= 52);

            yukino_region_destroy(reg_ptr);
        }
    }
}
