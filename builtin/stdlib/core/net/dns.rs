use std::net::ToSocketAddrs;
use super::error::{record_io_error, set_last_error, set_net_str_result};

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_dns_resolve(
    host_p: *const u8,
    host_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    if host_p.is_null() || host_len <= 0 {
        set_last_error(11, "invalid host pointer or length".to_string());
        return std::ptr::null();
    }
    let host_bytes = unsafe { std::slice::from_raw_parts(host_p, host_len as usize) };
    let host = match std::str::from_utf8(host_bytes) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(12, format!("invalid utf-8 host: {e}"));
            return std::ptr::null();
        }
    };
    let lookup_target = format!("{host}:0");
    match lookup_target.to_socket_addrs() {
        Ok(addrs) => {
            let mut list = Vec::new();
            for addr in addrs {
                let ip_str = addr.ip().to_string();
                if !list.contains(&ip_str) {
                    list.push(ip_str);
                }
            }
            if list.is_empty() {
                set_last_error(1, "host not found".to_string());
                return std::ptr::null();
            }
            let joined = list.join(";");
            set_net_str_result(joined.as_bytes(), out_len)
        }
        Err(e) => {
            record_io_error(e);
            std::ptr::null()
        }
    }
}
