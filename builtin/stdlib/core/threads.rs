use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

static NEXT_THREAD_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_MUTEX_ID: AtomicU64 = AtomicU64::new(1);

static THREADS: Mutex<Option<HashMap<u64, std::thread::JoinHandle<()>>>> = Mutex::new(None);
static MUTEXES: Mutex<Option<HashMap<u64, Arc<RawMutex>>>> = Mutex::new(None);

thread_local! {
    static CURRENT_TID: u64 = NEXT_THREAD_ID.fetch_add(1, Ordering::Relaxed);
}

pub struct RawMutex { state: AtomicBool, }

impl RawMutex {
    pub fn new() -> Self { Self { state: AtomicBool::new(false), } }

    pub fn lock(&self) {
        while self
            .state
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        { std::thread::yield_now(); } // while
    }

    pub fn unlock(&self) { self.state.store(false, Ordering::Release); }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_thread_spawn(task: *const (), arg: *const ()) -> u64 {
    if task.is_null() { return 0; }
    let id = NEXT_THREAD_ID.fetch_add(1, Ordering::Relaxed);
    let task_addr = task as usize;
    let arg_addr = arg as usize;

    let handle = std::thread::spawn(move || {
        let f: unsafe extern "C" fn(*const ()) = unsafe { std::mem::transmute(task_addr) };
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            f(arg_addr as *const ());
        }));
    });

    let mut guard = match THREADS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.get_or_insert_with(HashMap::new).insert(id, handle);
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_thread_join(id: u64) -> i32 {
    let handle = {
        let mut guard = match THREADS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_mut().and_then(|map| map.remove(&id))
    };
    match handle {
        Some(h) => match h.join() {
            Ok(()) => 0,
            Err(_) => -1,
        },
        None => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_thread_is_finished(id: u64) -> bool {
    let guard = match THREADS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(map) = guard.as_ref()
        && let Some(h) = map.get(&id)
    { return h.is_finished(); }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_thread_id(id: u64) -> u64 { id }

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_thread_sleep(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_thread_yield() { std::thread::yield_now(); }

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_thread_current_id() -> u64 { CURRENT_TID.with(|&id| id) }

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_thread_hardware_concurrency() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_mutex_create() -> u64 {
    let id = NEXT_MUTEX_ID.fetch_add(1, Ordering::Relaxed);
    let mut guard = match MUTEXES.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard
        .get_or_insert_with(HashMap::new)
        .insert(id, Arc::new(RawMutex::new()));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_mutex_lock(id: u64) {
    let m = {
        let guard = match MUTEXES.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    if let Some(mutex) = m {
        mutex.lock();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_mutex_unlock(id: u64) {
    let m = {
        let guard = match MUTEXES.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    if let Some(mutex) = m { mutex.unlock(); }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_mutex_destroy(id: u64) {
    let mut guard = match MUTEXES.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(map) = guard.as_mut() {
        map.remove(&id);
    }
}
