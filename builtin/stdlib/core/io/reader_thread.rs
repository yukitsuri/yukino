use std::collections::VecDeque;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

static BG_RUNNING: AtomicBool = AtomicBool::new(false);
static BG_QUEUE: Mutex<VecDeque<u8>> = Mutex::new(VecDeque::new());
static BG_CONDVAR: Condvar = Condvar::new();

fn start_bg_worker() {
    let mut byte_buf = [0u8; 1];
    let stdin = std::io::stdin();
    loop {
        let mut handle = stdin.lock();
        match handle.read_exact(&mut byte_buf) {
            Ok(()) => {
                let b = byte_buf[0];
                drop(handle);
                let mut q = match BG_QUEUE.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                q.push_back(b);
                BG_CONDVAR.notify_one();
            }
            Err(_) => { break; }
        }
    }
    BG_RUNNING.store(false, Ordering::SeqCst);
}

fn ensure_bg_reader_started() {
    if BG_RUNNING.swap(true, Ordering::SeqCst) { return; }
    let _ = std::thread::Builder::new()
        .name("yukino-stdin-reader".to_string())
        .spawn(start_bg_worker);
}

pub fn yukistd_io_bg_try_pop() -> Option<u8> {
    if !BG_RUNNING.load(Ordering::SeqCst) { return None; }
    let mut q = match BG_QUEUE.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    q.pop_front()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_bg_enable() { ensure_bg_reader_started(); }

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_bg_poll(timeout_ms: i64) -> bool {
    if !BG_RUNNING.load(Ordering::SeqCst) { return false; }
    let q = match BG_QUEUE.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if !q.is_empty() { return true; }
    if timeout_ms == 0 { return false; }
    if timeout_ms < 0 {
        let res = match BG_CONDVAR.wait(q) {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        return !res.is_empty();
    }
    let dur = Duration::from_millis(timeout_ms as u64);
    let (res, _) = match BG_CONDVAR.wait_timeout(q, dur) {
        Ok(r) => r,
        Err(p) => p.into_inner(),
    };
    !res.is_empty()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_bg_read_byte() -> i32 {
    ensure_bg_reader_started();
    let mut q = match BG_QUEUE.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    match q.pop_front() {
        Some(b) => b as i32,
        None => -1,
    }
}
