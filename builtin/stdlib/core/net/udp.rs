use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::error::{clear_last_error, record_io_error, set_last_error, set_net_str_result};

#[derive(Clone)]
pub struct UdpEntry {
    pub socket: Arc<UdpSocket>,
    pub last_sender: Arc<Mutex<String>>,
}

static NEXT_UDP_ID: AtomicU64 = AtomicU64::new(1);
static UDP_SOCKETS: Mutex<Option<HashMap<u64, UdpEntry>>> = Mutex::new(None);

pub fn get_udp_entry(id: u64) -> Option<UdpEntry> {
    let guard = match UDP_SOCKETS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.as_ref().and_then(|map| map.get(&id).cloned())
}

pub fn get_udp_socket(id: u64) -> Option<Arc<UdpSocket>> {
    get_udp_entry(id).map(|e| e.socket)
}

fn parse_str_arg<'a>(p: *const u8, len: i64) -> Result<&'a str, i32> {
    if p.is_null() || len <= 0 {
        set_last_error(11, "null or empty string pointer".to_string());
        return Err(11);
    }
    let slice = unsafe { std::slice::from_raw_parts(p, len as usize) };
    std::str::from_utf8(slice).map_err(|e| {
        set_last_error(12, format!("invalid utf-8 argument: {e}"));
        12
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_udp_bind(addr_p: *const u8, addr_len: i64) -> u64 {
    clear_last_error();
    let addr_str = match parse_str_arg(addr_p, addr_len) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let socket = match UdpSocket::bind(addr_str) {
        Ok(s) => s,
        Err(e) => {
            record_io_error(e);
            return 0;
        }
    };
    let id = NEXT_UDP_ID.fetch_add(1, Ordering::Relaxed);
    let mut guard = match UDP_SOCKETS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.get_or_insert_with(HashMap::new).insert(
        id,
        UdpEntry {
            socket: Arc::new(socket),
            last_sender: Arc::new(Mutex::new(String::new())),
        },
    );
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_udp_connect(socket_id: u64, addr_p: *const u8, addr_len: i64) -> bool {
    clear_last_error();
    let addr_str = match parse_str_arg(addr_p, addr_len) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let socket = match get_udp_socket(socket_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown udp socket id {socket_id}"));
            return false;
        }
    };
    match socket.connect(addr_str) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_udp_send(socket_id: u64, buf_p: *const u8, len: i64) -> i64 {
    clear_last_error();
    if buf_p.is_null() || len < 0 {
        set_last_error(11, "invalid buffer pointer or len < 0".to_string());
        return -1;
    }
    let socket = match get_udp_socket(socket_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown udp socket id {socket_id}"));
            return -1;
        }
    };
    let slice = unsafe { std::slice::from_raw_parts(buf_p, len as usize) };
    match socket.send(slice) {
        Ok(n) => n as i64,
        Err(e) => {
            record_io_error(e);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_udp_recv_text(
    socket_id: u64,
    max_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    clear_last_error();
    let socket = match get_udp_socket(socket_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown udp socket id {socket_id}"));
            return std::ptr::null();
        }
    };
    let cap = if max_len <= 0 { 65536 } else { max_len as usize };
    let mut buf = vec![0u8; cap];
    match socket.recv(&mut buf) {
        Ok(0) => {
            unsafe {
                if !out_len.is_null() {
                    *out_len = 0;
                }
            }
            b"".as_ptr()
        }
        Ok(n) => {
            buf.truncate(n);
            set_net_str_result(&buf, out_len)
        }
        Err(e) => {
            record_io_error(e);
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_udp_recv_from_text(
    socket_id: u64,
    max_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    clear_last_error();
    let entry = match get_udp_entry(socket_id) {
        Some(e) => e,
        None => {
            set_last_error(11, format!("unknown udp socket id {socket_id}"));
            return std::ptr::null();
        }
    };
    let cap = if max_len <= 0 { 65536 } else { max_len as usize };
    let mut buf = vec![0u8; cap];
    match entry.socket.recv_from(&mut buf) {
        Ok((n, sender)) => {
            if let Ok(mut g) = entry.last_sender.lock() {
                *g = sender.to_string();
            }
            buf.truncate(n);
            set_net_str_result(&buf, out_len)
        }
        Err(e) => {
            record_io_error(e);
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_udp_last_sender(
    socket_id: u64,
    out_len: *mut i64,
) -> *const u8 {
    clear_last_error();
    let entry = match get_udp_entry(socket_id) {
        Some(e) => e,
        None => {
            set_last_error(11, format!("unknown udp socket id {socket_id}"));
            return std::ptr::null();
        }
    };
    let sender = match entry.last_sender.lock() {
        Ok(g) => g.clone(),
        Err(p) => p.into_inner().clone(),
    };
    set_net_str_result(sender.as_bytes(), out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_udp_recv(socket_id: u64, buf_p: *mut u8, max_len: i64) -> i64 {
    clear_last_error();
    if buf_p.is_null() || max_len <= 0 {
        set_last_error(11, "invalid buffer pointer or max_len <= 0".to_string());
        return -1;
    }
    let socket = match get_udp_socket(socket_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown udp socket id {socket_id}"));
            return -1;
        }
    };
    let slice = unsafe { std::slice::from_raw_parts_mut(buf_p, max_len as usize) };
    match socket.recv(slice) {
        Ok(n) => n as i64,
        Err(e) => {
            record_io_error(e);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_udp_send_to(
    socket_id: u64,
    buf_p: *const u8,
    len: i64,
    addr_p: *const u8,
    addr_len: i64,
) -> i64 {
    clear_last_error();
    if buf_p.is_null() || len < 0 {
        set_last_error(11, "invalid buffer pointer or len < 0".to_string());
        return -1;
    }
    let addr_str = match parse_str_arg(addr_p, addr_len) {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let socket = match get_udp_socket(socket_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown udp socket id {socket_id}"));
            return -1;
        }
    };
    let slice = unsafe { std::slice::from_raw_parts(buf_p, len as usize) };
    match socket.send_to(slice, addr_str) {
        Ok(n) => n as i64,
        Err(e) => {
            record_io_error(e);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_udp_recv_from(
    socket_id: u64,
    buf_p: *mut u8,
    max_len: i64,
) -> i64 {
    clear_last_error();
    if buf_p.is_null() || max_len <= 0 {
        set_last_error(11, "invalid buffer pointer or max_len <= 0".to_string());
        return -1;
    }
    let entry = match get_udp_entry(socket_id) {
        Some(e) => e,
        None => {
            set_last_error(11, format!("unknown udp socket id {socket_id}"));
            return -1;
        }
    };
    let slice = unsafe { std::slice::from_raw_parts_mut(buf_p, max_len as usize) };
    match entry.socket.recv_from(slice) {
        Ok((n, sender_addr)) => {
            if let Ok(mut g) = entry.last_sender.lock() {
                *g = sender_addr.to_string();
            }
            n as i64
        }
        Err(e) => {
            record_io_error(e);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_udp_set_broadcast(socket_id: u64, broadcast: bool) -> bool {
    clear_last_error();
    let socket = match get_udp_socket(socket_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown udp socket id {socket_id}"));
            return false;
        }
    };
    match socket.set_broadcast(broadcast) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_udp_set_nonblocking(socket_id: u64, nonblocking: bool) -> bool {
    clear_last_error();
    let socket = match get_udp_socket(socket_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown udp socket id {socket_id}"));
            return false;
        }
    };
    match socket.set_nonblocking(nonblocking) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_udp_set_timeouts(
    socket_id: u64,
    read_ms: u64,
    write_ms: u64,
) -> bool {
    clear_last_error();
    let socket = match get_udp_socket(socket_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown udp socket id {socket_id}"));
            return false;
        }
    };
    let r_dur = if read_ms > 0 { Some(Duration::from_millis(read_ms)) } else { None };
    let w_dur = if write_ms > 0 { Some(Duration::from_millis(write_ms)) } else { None };
    if let Err(e) = socket.set_read_timeout(r_dur) {
        record_io_error(e);
        return false;
    }
    if let Err(e) = socket.set_write_timeout(w_dur) {
        record_io_error(e);
        return false;
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_udp_close(socket_id: u64) -> bool {
    clear_last_error();
    let mut guard = match UDP_SOCKETS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(map) = guard.as_mut() {
        map.remove(&socket_id).is_some()
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_udp_local_addr(socket_id: u64, out_len: *mut i64) -> *const u8 {
    clear_last_error();
    let socket = match get_udp_socket(socket_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown udp socket id {socket_id}"));
            return std::ptr::null();
        }
    };
    match socket.local_addr() {
        Ok(addr) => {
            let s = addr.to_string();
            set_net_str_result(s.as_bytes(), out_len)
        }
        Err(e) => {
            record_io_error(e);
            std::ptr::null()
        }
    }
}
