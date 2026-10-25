use std::cell::RefCell;

thread_local! {
    static LAST_NET_ERROR: RefCell<(i32, String)> = const { RefCell::new((0, String::new())) };
    static NET_STR_RING: RefCell<(usize, Vec<Vec<u8>>)> = const { RefCell::new((0, Vec::new())) };
}

pub fn set_last_error(code: i32, msg: String) {
    LAST_NET_ERROR.with(|cell| {
        *cell.borrow_mut() = (code, msg);
    });
}

pub fn clear_last_error() {
    LAST_NET_ERROR.with(|cell| {
        *cell.borrow_mut() = (0, String::new());
    });
}

pub fn record_io_error(e: std::io::Error) -> i32 {
    let code = match e.kind() {
        std::io::ErrorKind::NotFound => 1,
        std::io::ErrorKind::PermissionDenied => 2,
        std::io::ErrorKind::ConnectionRefused => 3,
        std::io::ErrorKind::ConnectionReset => 4,
        std::io::ErrorKind::ConnectionAborted => 5,
        std::io::ErrorKind::NotConnected => 6,
        std::io::ErrorKind::AddrInUse => 7,
        std::io::ErrorKind::AddrNotAvailable => 8,
        std::io::ErrorKind::TimedOut => 9,
        std::io::ErrorKind::WouldBlock => 10,
        std::io::ErrorKind::InvalidInput => 11,
        std::io::ErrorKind::InvalidData => 12,
        _ => 20,
    };
    let msg = e.to_string();
    set_last_error(code, msg);
    code
}

pub fn set_net_str_result(bytes: &[u8], out_len: *mut i64) -> *const u8 {
    NET_STR_RING.with(|cell| {
        let mut ring = cell.borrow_mut();
        if ring.1.len() < 256 {
            ring.1.resize_with(256, Vec::new);
        }
        let idx = ring.0 % 256;
        ring.0 = ring.0.wrapping_add(1);
        let slot = &mut ring.1[idx];
        slot.clear();
        slot.extend_from_slice(bytes);
        unsafe {
            if !out_len.is_null() {
                *out_len = slot.len() as i64;
            }
        }
        slot.as_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_net_last_error_code() -> i32 {
    LAST_NET_ERROR.with(|cell| cell.borrow().0)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_net_last_error_message(out_len: *mut i64) -> *const u8 {
    let s = LAST_NET_ERROR.with(|cell| cell.borrow().1.clone());
    set_net_str_result(s.as_bytes(), out_len)
}
