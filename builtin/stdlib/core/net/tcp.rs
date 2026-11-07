use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::error::{clear_last_error, record_io_error, set_last_error, set_net_str_result};

static NEXT_TCP_ID: AtomicU64 = AtomicU64::new(1);
static TCP_LISTENERS: Mutex<Option<HashMap<u64, Arc<TcpListener>>>> = Mutex::new(None);
static TCP_STREAMS: Mutex<Option<HashMap<u64, Arc<TcpStream>>>> = Mutex::new(None);

pub fn get_tcp_stream(id: u64) -> Option<Arc<TcpStream>> {
    let guard = match TCP_STREAMS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.as_ref().and_then(|map| map.get(&id).cloned())
}

pub fn get_tcp_listener(id: u64) -> Option<Arc<TcpListener>> {
    let guard = match TCP_LISTENERS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.as_ref().and_then(|map| map.get(&id).cloned())
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
pub extern "C" fn yukistd_tcp_listen(addr_p: *const u8, addr_len: i64) -> u64 {
    clear_last_error();
    let addr_str = match parse_str_arg(addr_p, addr_len) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let listener = match TcpListener::bind(addr_str) {
        Ok(l) => l,
        Err(e) => {
            record_io_error(e);
            return 0;
        }
    };
    let id = NEXT_TCP_ID.fetch_add(1, Ordering::Relaxed);
    let mut guard = match TCP_LISTENERS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.get_or_insert_with(HashMap::new).insert(id, Arc::new(listener));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tcp_accept(
    listener_id: u64,
    out_peer_len: *mut i64,
) -> u64 {
    clear_last_error();
    let listener = match get_tcp_listener(listener_id) {
        Some(l) => l,
        None => {
            set_last_error(11, format!("unknown tcp listener id {listener_id}"));
            return 0;
        }
    };
    let (stream, peer_addr) = match listener.accept() {
        Ok(res) => res,
        Err(e) => {
            record_io_error(e);
            return 0;
        }
    };
    if !out_peer_len.is_null() {
        let peer_str = peer_addr.to_string();
        set_net_str_result(peer_str.as_bytes(), out_peer_len);
    }
    let id = NEXT_TCP_ID.fetch_add(1, Ordering::Relaxed);
    let mut guard = match TCP_STREAMS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.get_or_insert_with(HashMap::new).insert(id, Arc::new(stream));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tcp_connect(
    addr_p: *const u8,
    addr_len: i64,
    timeout_ms: u64,
) -> u64 {
    clear_last_error();
    let addr_str = match parse_str_arg(addr_p, addr_len) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let stream_res = if timeout_ms == 0 {
        TcpStream::connect(addr_str)
    } else {
        match addr_str.to_socket_addrs() {
            Ok(mut addrs) => match addrs.next() {
                Some(addr) => TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)),
                None => {
                    set_last_error(8, format!("could not resolve address {addr_str}"));
                    return 0;
                }
            },
            Err(e) => {
                record_io_error(e);
                return 0;
            }
        }
    };
    let stream = match stream_res {
        Ok(s) => s,
        Err(e) => {
            record_io_error(e);
            return 0;
        }
    };
    let id = NEXT_TCP_ID.fetch_add(1, Ordering::Relaxed);
    let mut guard = match TCP_STREAMS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.get_or_insert_with(HashMap::new).insert(id, Arc::new(stream));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tcp_read_text(
    stream_id: u64,
    max_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    clear_last_error();
    let stream = match get_tcp_stream(stream_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tcp stream id {stream_id}"));
            return std::ptr::null();
        }
    };
    let cap = if max_len <= 0 { 65536 } else { max_len as usize };
    let mut buf = vec![0u8; cap];
    match (&*stream).read(&mut buf) {
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
pub extern "C" fn yukistd_tcp_read(
    stream_id: u64,
    buf_p: *mut u8,
    max_len: i64,
) -> i64 {
    clear_last_error();
    if buf_p.is_null() || max_len <= 0 {
        set_last_error(11, "invalid buffer pointer or max_len <= 0".to_string());
        return -1;
    }
    let stream = match get_tcp_stream(stream_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tcp stream id {stream_id}"));
            return -1;
        }
    };
    let slice = unsafe { std::slice::from_raw_parts_mut(buf_p, max_len as usize) };
    match (&*stream).read(slice) {
        Ok(n) => n as i64,
        Err(e) => {
            record_io_error(e);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tcp_write(
    stream_id: u64,
    buf_p: *const u8,
    len: i64,
) -> i64 {
    clear_last_error();
    if buf_p.is_null() || len < 0 {
        set_last_error(11, "invalid buffer pointer or len < 0".to_string());
        return -1;
    }
    if len == 0 {
        return 0;
    }
    let stream = match get_tcp_stream(stream_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tcp stream id {stream_id}"));
            return -1;
        }
    };
    let slice = unsafe { std::slice::from_raw_parts(buf_p, len as usize) };
    match (&*stream).write(slice) {
        Ok(n) => n as i64,
        Err(e) => {
            record_io_error(e);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tcp_flush(stream_id: u64) -> bool {
    clear_last_error();
    let stream = match get_tcp_stream(stream_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tcp stream id {stream_id}"));
            return false;
        }
    };
    match (&*stream).flush() {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tcp_set_nonblocking(stream_id: u64, nonblocking: bool) -> bool {
    clear_last_error();
    let stream = match get_tcp_stream(stream_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tcp stream id {stream_id}"));
            return false;
        }
    };
    match stream.set_nonblocking(nonblocking) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tcp_listener_set_nonblocking(
    listener_id: u64,
    nonblocking: bool,
) -> bool {
    clear_last_error();
    let listener = match get_tcp_listener(listener_id) {
        Some(l) => l,
        None => {
            set_last_error(11, format!("unknown tcp listener id {listener_id}"));
            return false;
        }
    };
    match listener.set_nonblocking(nonblocking) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tcp_set_nodelay(stream_id: u64, nodelay: bool) -> bool {
    clear_last_error();
    let stream = match get_tcp_stream(stream_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tcp stream id {stream_id}"));
            return false;
        }
    };
    match stream.set_nodelay(nodelay) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tcp_set_timeouts(
    stream_id: u64,
    read_ms: u64,
    write_ms: u64,
) -> bool {
    clear_last_error();
    let stream = match get_tcp_stream(stream_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tcp stream id {stream_id}"));
            return false;
        }
    };
    let r_dur = if read_ms > 0 { Some(Duration::from_millis(read_ms)) } else { None };
    let w_dur = if write_ms > 0 { Some(Duration::from_millis(write_ms)) } else { None };
    if let Err(e) = stream.set_read_timeout(r_dur) {
        record_io_error(e);
        return false;
    }
    if let Err(e) = stream.set_write_timeout(w_dur) {
        record_io_error(e);
        return false;
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tcp_shutdown(stream_id: u64, how: i32) -> bool {
    clear_last_error();
    let stream = match get_tcp_stream(stream_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tcp stream id {stream_id}"));
            return false;
        }
    };
    let mode = match how {
        0 => Shutdown::Read,
        1 => Shutdown::Write,
        _ => Shutdown::Both,
    };
    match stream.shutdown(mode) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tcp_close(stream_id: u64) -> bool {
    clear_last_error();
    let mut guard = match TCP_STREAMS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(map) = guard.as_mut() {
        map.remove(&stream_id).is_some()
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tcp_listener_close(listener_id: u64) -> bool {
    clear_last_error();
    let mut guard = match TCP_LISTENERS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(map) = guard.as_mut() {
        map.remove(&listener_id).is_some()
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tcp_peer_addr(stream_id: u64, out_len: *mut i64) -> *const u8 {
    clear_last_error();
    let stream = match get_tcp_stream(stream_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tcp stream id {stream_id}"));
            return std::ptr::null();
        }
    };
    match stream.peer_addr() {
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

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tcp_local_addr(stream_id: u64, out_len: *mut i64) -> *const u8 {
    clear_last_error();
    let stream = match get_tcp_stream(stream_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tcp stream id {stream_id}"));
            return std::ptr::null();
        }
    };
    match stream.local_addr() {
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

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_tcp_listener_local_addr(
    listener_id: u64,
    out_len: *mut i64,
) -> *const u8 {
    clear_last_error();
    let listener = match get_tcp_listener(listener_id) {
        Some(l) => l,
        None => {
            set_last_error(11, format!("unknown tcp listener id {listener_id}"));
            return std::ptr::null();
        }
    };
    match listener.local_addr() {
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
