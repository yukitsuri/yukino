use std::cell::RefCell;

thread_local! {
    static NUMF_STR_RING: RefCell<(usize, Vec<Vec<u8>>)> = const { RefCell::new((0, Vec::new())) };
}

fn set_str_result(bytes: &[u8], out_len: *mut i64) -> *const u8 {
    NUMF_STR_RING.with(|cell| {
        let mut ring = cell.borrow_mut();
        if ring.1.len() < 64 {
            ring.1.resize_with(64, Vec::new);
        }
        let idx = ring.0 % 64;
        ring.0 = ring.0.wrapping_add(1);
        let buf = &mut ring.1[idx];
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

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_numf_to_fixed(
    val: f64,
    precision: i32,
    out_len: *mut i64,
) -> *const u8 {
    if val.is_nan() {
        return set_str_result(b"NaN", out_len);
    }
    if val.is_infinite() {
        if val < 0.0 {
            return set_str_result(b"-Infinity", out_len);
        } else {
            return set_str_result(b"Infinity", out_len);
        }
    }
    let p = precision.clamp(0, 100) as usize;
    let s = format!("{:.*}", p, val);
    set_str_result(s.as_bytes(), out_len)
}
