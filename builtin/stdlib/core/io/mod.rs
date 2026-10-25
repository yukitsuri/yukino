pub mod linux;
pub mod reader_thread;
pub mod unix;
pub mod windows;

use std::sync::Mutex;

static LINE_BUF: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static ALL_BUF: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static COLOR_MODE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_set_color_mode(mode: i32) {
    COLOR_MODE.store(mode, std::sync::atomic::Ordering::Relaxed);
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_get_color_mode() -> i32 {
    COLOR_MODE.load(std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_write_stdout(ptr: *const u8, len: usize, flush: bool) -> i64 {
    use std::io::Write;
    if ptr.is_null() || len == 0 {
        if flush { let _ = std::io::stdout().flush(); }
        return 0;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    let mut stdout = std::io::stdout().lock();
    let res = match stdout.write_all(slice) {
        Ok(()) => len as i64,
        Err(_) => -1,
    };
    if flush { let _ = stdout.flush(); }
    res
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_write_stderr(ptr: *const u8, len: usize, flush: bool) -> i64 {
    use std::io::Write;
    if ptr.is_null() || len == 0 {
        if flush { let _ = std::io::stderr().flush(); }
        return 0;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    let mut stderr = std::io::stderr().lock();
    let res = match stderr.write_all(slice) {
        Ok(()) => len as i64,
        Err(_) => -1,
    };
    if flush { let _ = stderr.flush(); }
    res
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_flush_stdout() -> i32 {
    use std::io::Write;
    match std::io::stdout().flush() {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_flush_stderr() -> i32 {
    use std::io::Write;
    match std::io::stderr().flush() {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_read_stdin(buf: *mut u8, max_len: usize) -> i64 {
    use std::io::Read;
    if buf.is_null() || max_len == 0 { return 0; }
    let slice = unsafe { std::slice::from_raw_parts_mut(buf, max_len) };
    let mut stdin = std::io::stdin().lock();
    match stdin.read(slice) {
        Ok(n) => n as i64,
        Err(_) => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_read_line(out_len: *mut i64) -> *const u8 {
    use std::io::BufRead;
    let mut guard = match LINE_BUF.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.clear();

    let mut line = String::new();
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    match handle.read_line(&mut line) {
        Ok(0) => {
            if !out_len.is_null() { unsafe { *out_len = 0; } }
            std::ptr::null()
        }
        Ok(_) => {
            if line.ends_with('\n') { line.pop(); if line.ends_with('\r') { line.pop(); } }
            guard.extend_from_slice(line.as_bytes());
            if !out_len.is_null() { unsafe { *out_len = guard.len() as i64; } }
            guard.as_ptr()
        }
        Err(_) => {
            if !out_len.is_null() {
                unsafe { *out_len = -1; }
            }
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_read_all(out_len: *mut i64) -> *const u8 {
    use std::io::Read;
    let mut guard = match ALL_BUF.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.clear();

    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    match handle.read_to_end(&mut guard) {
        Ok(n) => {
            if !out_len.is_null() { unsafe { *out_len = n as i64; } }
            guard.as_ptr()
        }
        Err(_) => {
            if !out_len.is_null() { unsafe { *out_len = -1; } }
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_read_char() -> i32 {
    if let Some(b) = reader_thread::yukistd_io_bg_try_pop() { return b as i32; }
    self::unix::yukistd_io_read_stdin_byte()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_is_terminal(stream_id: i32) -> bool {
    use std::io::IsTerminal;
    match stream_id {
        0 => std::io::stdin().is_terminal(),
        1 => std::io::stdout().is_terminal(),
        2 => std::io::stderr().is_terminal(),
        _ => false,
    }
}

static STDOUT_LOCK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static STDIN_LOCK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_lock_stdout() {
    use std::sync::atomic::Ordering;
    while STDOUT_LOCK
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    { std::thread::yield_now(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_unlock_stdout() {
    use std::sync::atomic::Ordering;
    STDOUT_LOCK.store(false, Ordering::Release);
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_lock_stdin() {
    use std::sync::atomic::Ordering;
    while STDIN_LOCK
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    { std::thread::yield_now(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_unlock_stdin() {
    use std::sync::atomic::Ordering;
    STDIN_LOCK.store(false, Ordering::Release);
}
