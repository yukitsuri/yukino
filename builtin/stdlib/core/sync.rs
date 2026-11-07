use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Barrier, Condvar, Mutex, Once};
use std::time::Duration;

static NEXT_SYNC_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> u64 {
    NEXT_SYNC_ID.fetch_add(1, Ordering::Relaxed)
}

// raw mutex

pub struct RawMutex {
    state: Mutex<bool>,
    cond: Condvar,
}

impl RawMutex {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(false),
            cond: Condvar::new(),
        }
    }

    pub fn lock(&self) {
        let mut locked = self.state.lock().unwrap();
        while *locked {
            locked = self.cond.wait(locked).unwrap();
        }
        *locked = true;
    }

    pub fn try_lock(&self) -> bool {
        let Ok(mut locked) = self.state.try_lock() else {
            return false;
        };
        if *locked {
            false
        } else {
            *locked = true;
            true
        }
    }

    pub fn unlock(&self) {
        let mut locked = self.state.lock().unwrap();
        *locked = false;
        self.cond.notify_one();
    }
}

// raw rwlock

pub struct RawRwLock {
    state: Mutex<(usize, bool)>,
    cond: Condvar,
}

impl RawRwLock {
    pub fn new() -> Self {
        Self {
            state: Mutex::new((0, false)),
            cond: Condvar::new(),
        }
    }

    pub fn read_lock(&self) {
        let mut s = self.state.lock().unwrap();
        while s.1 {
            s = self.cond.wait(s).unwrap();
        }
        s.0 = s.0.wrapping_add(1);
    }

    pub fn try_read_lock(&self) -> bool {
        let Ok(mut s) = self.state.try_lock() else {
            return false;
        };
        if s.1 {
            false
        } else {
            s.0 = s.0.wrapping_add(1);
            true
        }
    }

    pub fn read_unlock(&self) {
        let mut s = self.state.lock().unwrap();
        if s.0 > 0 {
            s.0 -= 1;
            if s.0 == 0 {
                self.cond.notify_all();
            }
        }
    }

    pub fn write_lock(&self) {
        let mut s = self.state.lock().unwrap();
        while s.1 || s.0 > 0 {
            s = self.cond.wait(s).unwrap();
        }
        s.1 = true;
    }

    pub fn try_write_lock(&self) -> bool {
        let Ok(mut s) = self.state.try_lock() else {
            return false;
        };
        if s.1 || s.0 > 0 {
            false
        } else {
            s.1 = true;
            true
        }
    }

    pub fn write_unlock(&self) {
        let mut s = self.state.lock().unwrap();
        s.1 = false;
        self.cond.notify_all();
    }
}

// raw condvar

pub struct RawCondvar {
    seq: Mutex<u64>,
    cond: Condvar,
}

impl RawCondvar {
    pub fn new() -> Self {
        Self {
            seq: Mutex::new(0),
            cond: Condvar::new(),
        }
    }

    pub fn wait(&self, mutex: &RawMutex) {
        let initial_seq = {
            let guard = self.seq.lock().unwrap();
            *guard
        };
        mutex.unlock();
        {
            let mut guard = self.seq.lock().unwrap();
            while *guard == initial_seq {
                guard = self.cond.wait(guard).unwrap();
            }
        }
        mutex.lock();
    }

    pub fn wait_timeout(&self, mutex: &RawMutex, ms: u64) -> bool {
        let initial_seq = {
            let guard = self.seq.lock().unwrap();
            *guard
        };
        mutex.unlock();
        let timeout = Duration::from_millis(ms);
        let start = std::time::Instant::now();
        let mut success = false;
        {
            let mut guard = self.seq.lock().unwrap();
            while *guard == initial_seq {
                let elapsed = start.elapsed();
                if elapsed >= timeout {
                    break;
                }
                let left = timeout - elapsed;
                let (g, res) = self.cond.wait_timeout(guard, left).unwrap();
                guard = g;
                if res.timed_out() {
                    break;
                }
                if *guard != initial_seq {
                    success = true;
                    break;
                }
            }
            if *guard != initial_seq {
                success = true;
            }
        }
        mutex.lock();
        success
    }

    pub fn notify_one(&self) {
        let mut guard = self.seq.lock().unwrap();
        *guard = guard.wrapping_add(1);
        self.cond.notify_one();
    }

    pub fn notify_all(&self) {
        let mut guard = self.seq.lock().unwrap();
        *guard = guard.wrapping_add(1);
        self.cond.notify_all();
    }
}

// raw once

pub struct RawOnce {
    once: Once,
    completed: AtomicBool,
}

impl RawOnce {
    pub fn new() -> Self {
        Self {
            once: Once::new(),
            completed: AtomicBool::new(false),
        }
    }
}

// raw channel

pub struct RawChannel {
    queue: Mutex<VecDeque<i64>>,
    bound: usize,
    not_empty: Condvar,
    not_full: Condvar,
    closed: AtomicBool,
}

impl RawChannel {
    pub fn new(bound: i64) -> Self {
        let cap = if bound > 0 { bound as usize } else { 0 };
        Self {
            queue: Mutex::new(VecDeque::new()),
            bound: cap,
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
            closed: AtomicBool::new(false),
        }
    }

    pub fn send(&self, val: i64) -> bool {
        if self.closed.load(Ordering::Acquire) {
            return false;
        }
        let mut q = self.queue.lock().unwrap();
        if self.bound > 0 {
            while q.len() >= self.bound && !self.closed.load(Ordering::Acquire) {
                q = self.not_full.wait(q).unwrap();
            }
            if self.closed.load(Ordering::Acquire) {
                return false;
            }
        }
        q.push_back(val);
        self.not_empty.notify_one();
        true
    }

    pub fn recv(&self, timeout_ms: i64, out_val: *mut i64) -> i32 {
        let mut q = self.queue.lock().unwrap();
        if timeout_ms == 0 {
            if let Some(v) = q.pop_front() {
                if !out_val.is_null() {
                    unsafe { *out_val = v; }
                }
                self.not_full.notify_one();
                return 0;
            }
            if self.closed.load(Ordering::Acquire) {
                return 1;
            }
            return 2;
        }

        if timeout_ms < 0 {
            while q.is_empty() && !self.closed.load(Ordering::Acquire) {
                q = self.not_empty.wait(q).unwrap();
            }
            if let Some(v) = q.pop_front() {
                if !out_val.is_null() {
                    unsafe { *out_val = v; }
                }
                self.not_full.notify_one();
                return 0;
            }
            return 1;
        }

        let timeout = Duration::from_millis(timeout_ms as u64);
        let start = std::time::Instant::now();
        while q.is_empty() && !self.closed.load(Ordering::Acquire) {
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                break;
            }
            let left = timeout - elapsed;
            let (next_q, res) = self.not_empty.wait_timeout(q, left).unwrap();
            q = next_q;
            if res.timed_out() {
                break;
            }
        }

        if let Some(v) = q.pop_front() {
            if !out_val.is_null() {
                unsafe { *out_val = v; }
            }
            self.not_full.notify_one();
            return 0;
        }
        if self.closed.load(Ordering::Acquire) {
            1
        } else {
            2
        }
    }

    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
        self.not_empty.notify_all();
        self.not_full.notify_all();
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    pub fn len(&self) -> i64 {
        let q = self.queue.lock().unwrap();
        q.len() as i64
    }
}

// raw semaphore

pub struct RawSemaphore {
    permits: Mutex<i64>,
    cond: Condvar,
}

impl RawSemaphore {
    pub fn new(permits: i64) -> Self {
        Self {
            permits: Mutex::new(permits),
            cond: Condvar::new(),
        }
    }

    pub fn acquire(&self) {
        let mut p = self.permits.lock().unwrap();
        while *p <= 0 {
            p = self.cond.wait(p).unwrap();
        }
        *p -= 1;
    }

    pub fn try_acquire(&self) -> bool {
        let Ok(mut p) = self.permits.try_lock() else {
            return false;
        };
        if *p <= 0 {
            false
        } else {
            *p -= 1;
            true
        }
    }

    pub fn release(&self) {
        let mut p = self.permits.lock().unwrap();
        *p += 1;
        self.cond.notify_one();
    }
}

// raw barrier

pub struct RawBarrier {
    barrier: Barrier,
}

impl RawBarrier {
    pub fn new(parties: u32) -> Self {
        let count = (parties as usize).max(1);
        Self {
            barrier: Barrier::new(count),
        }
    }

    pub fn wait(&self) -> bool {
        self.barrier.wait().is_leader()
    }
}

// global registry maps

static MUTEXES: Mutex<Option<HashMap<u64, Arc<RawMutex>>>> = Mutex::new(None);
static RWLOCKS: Mutex<Option<HashMap<u64, Arc<RawRwLock>>>> = Mutex::new(None);
static CONDVARS: Mutex<Option<HashMap<u64, Arc<RawCondvar>>>> = Mutex::new(None);
static ONCES: Mutex<Option<HashMap<u64, Arc<RawOnce>>>> = Mutex::new(None);
static CHANNELS: Mutex<Option<HashMap<u64, Arc<RawChannel>>>> = Mutex::new(None);
static SEMAPHORES: Mutex<Option<HashMap<u64, Arc<RawSemaphore>>>> = Mutex::new(None);
static BARRIERS: Mutex<Option<HashMap<u64, Arc<RawBarrier>>>> = Mutex::new(None);
static ATOMICS_I64: Mutex<Option<HashMap<u64, Arc<AtomicI64>>>> = Mutex::new(None);
static ATOMICS_BOOL: Mutex<Option<HashMap<u64, Arc<AtomicBool>>>> = Mutex::new(None);

// c abi exports: mutex

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_mutex_create() -> u64 {
    let id = next_id();
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
pub extern "C" fn yukistd_sync_mutex_lock(id: u64) {
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
pub extern "C" fn yukistd_sync_mutex_try_lock(id: u64) -> bool {
    let m = {
        let guard = match MUTEXES.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    m.is_some_and(|mutex| mutex.try_lock())
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_mutex_unlock(id: u64) {
    let m = {
        let guard = match MUTEXES.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    if let Some(mutex) = m {
        mutex.unlock();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_mutex_destroy(id: u64) {
    let mut guard = match MUTEXES.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(map) = guard.as_mut() {
        map.remove(&id);
    }
}

// c abi exports: rwlock

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_rwlock_create() -> u64 {
    let id = next_id();
    let mut guard = match RWLOCKS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard
        .get_or_insert_with(HashMap::new)
        .insert(id, Arc::new(RawRwLock::new()));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_rwlock_read_lock(id: u64) {
    let r = {
        let guard = match RWLOCKS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    if let Some(rw) = r {
        rw.read_lock();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_rwlock_try_read_lock(id: u64) -> bool {
    let r = {
        let guard = match RWLOCKS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    r.is_some_and(|rw| rw.try_read_lock())
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_rwlock_read_unlock(id: u64) {
    let r = {
        let guard = match RWLOCKS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    if let Some(rw) = r {
        rw.read_unlock();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_rwlock_write_lock(id: u64) {
    let r = {
        let guard = match RWLOCKS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    if let Some(rw) = r {
        rw.write_lock();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_rwlock_try_write_lock(id: u64) -> bool {
    let r = {
        let guard = match RWLOCKS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    r.is_some_and(|rw| rw.try_write_lock())
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_rwlock_write_unlock(id: u64) {
    let r = {
        let guard = match RWLOCKS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    if let Some(rw) = r {
        rw.write_unlock();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_rwlock_destroy(id: u64) {
    let mut guard = match RWLOCKS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(map) = guard.as_mut() {
        map.remove(&id);
    }
}

// c abi exports: condvar

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_condvar_create() -> u64 {
    let id = next_id();
    let mut guard = match CONDVARS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard
        .get_or_insert_with(HashMap::new)
        .insert(id, Arc::new(RawCondvar::new()));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_condvar_wait(cond_id: u64, mutex_id: u64) {
    let cond = {
        let guard = match CONDVARS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&cond_id).cloned())
    };
    let mutex = {
        let guard = match MUTEXES.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&mutex_id).cloned())
    };
    if let (Some(c), Some(m)) = (cond, mutex) {
        c.wait(&m);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_condvar_wait_timeout(
    cond_id: u64,
    mutex_id: u64,
    ms: u64,
) -> bool {
    let cond = {
        let guard = match CONDVARS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&cond_id).cloned())
    };
    let mutex = {
        let guard = match MUTEXES.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&mutex_id).cloned())
    };
    if let (Some(c), Some(m)) = (cond, mutex) {
        c.wait_timeout(&m, ms)
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_condvar_notify_one(cond_id: u64) {
    let cond = {
        let guard = match CONDVARS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&cond_id).cloned())
    };
    if let Some(c) = cond {
        c.notify_one();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_condvar_notify_all(cond_id: u64) {
    let cond = {
        let guard = match CONDVARS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&cond_id).cloned())
    };
    if let Some(c) = cond {
        c.notify_all();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_condvar_destroy(cond_id: u64) {
    let mut guard = match CONDVARS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(map) = guard.as_mut() {
        map.remove(&cond_id);
    }
}

// c abi exports: once

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_once_create() -> u64 {
    let id = next_id();
    let mut guard = match ONCES.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard
        .get_or_insert_with(HashMap::new)
        .insert(id, Arc::new(RawOnce::new()));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_once_call(id: u64, task: *const (), arg: *const ()) {
    let o = {
        let guard = match ONCES.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    if let Some(once_obj) = o {
        let task_addr = task as usize;
        let arg_addr = arg as usize;
        once_obj.once.call_once(|| {
            if task_addr != 0 {
                let f: unsafe extern "C" fn(*const ()) = unsafe { std::mem::transmute(task_addr) };
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
                    f(arg_addr as *const ());
                }));
            }
            once_obj.completed.store(true, Ordering::Release);
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_once_is_completed(id: u64) -> bool {
    let o = {
        let guard = match ONCES.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    o.is_some_and(|once_obj| once_obj.completed.load(Ordering::Acquire))
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_once_destroy(id: u64) {
    let mut guard = match ONCES.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(map) = guard.as_mut() {
        map.remove(&id);
    }
}

// c abi exports: channel

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_channel_create(bound: i64) -> u64 {
    let id = next_id();
    let mut guard = match CHANNELS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard
        .get_or_insert_with(HashMap::new)
        .insert(id, Arc::new(RawChannel::new(bound)));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_channel_send(id: u64, val: i64) -> bool {
    let ch = {
        let guard = match CHANNELS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    ch.is_some_and(|c| c.send(val))
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_channel_recv(id: u64, timeout_ms: i64, out_val: *mut i64) -> i32 {
    let ch = {
        let guard = match CHANNELS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    match ch {
        Some(c) => c.recv(timeout_ms, out_val),
        None => 1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_channel_close(id: u64) {
    let ch = {
        let guard = match CHANNELS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    if let Some(c) = ch {
        c.close();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_channel_is_closed(id: u64) -> bool {
    let ch = {
        let guard = match CHANNELS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    ch.is_some_and(|c| c.is_closed())
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_channel_len(id: u64) -> i64 {
    let ch = {
        let guard = match CHANNELS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    match ch {
        Some(c) => c.len(),
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_channel_destroy(id: u64) {
    let mut guard = match CHANNELS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(map) = guard.as_mut() {
        map.remove(&id);
    }
}

// c abi exports: semaphore

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_semaphore_create(permits: i64) -> u64 {
    let id = next_id();
    let mut guard = match SEMAPHORES.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard
        .get_or_insert_with(HashMap::new)
        .insert(id, Arc::new(RawSemaphore::new(permits)));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_semaphore_acquire(id: u64) {
    let sem = {
        let guard = match SEMAPHORES.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    if let Some(s) = sem {
        s.acquire();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_semaphore_try_acquire(id: u64) -> bool {
    let sem = {
        let guard = match SEMAPHORES.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    sem.is_some_and(|s| s.try_acquire())
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_semaphore_release(id: u64) {
    let sem = {
        let guard = match SEMAPHORES.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    if let Some(s) = sem {
        s.release();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_semaphore_destroy(id: u64) {
    let mut guard = match SEMAPHORES.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(map) = guard.as_mut() {
        map.remove(&id);
    }
}

// c abi exports: barrier

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_barrier_create(parties: u32) -> u64 {
    let id = next_id();
    let mut guard = match BARRIERS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard
        .get_or_insert_with(HashMap::new)
        .insert(id, Arc::new(RawBarrier::new(parties)));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_barrier_wait(id: u64) -> bool {
    let b = {
        let guard = match BARRIERS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    b.is_some_and(|barrier| barrier.wait())
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_barrier_destroy(id: u64) {
    let mut guard = match BARRIERS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(map) = guard.as_mut() {
        map.remove(&id);
    }
}

// c abi exports: atomic int64

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_atomic_i64_create(init: i64) -> u64 {
    let id = next_id();
    let mut guard = match ATOMICS_I64.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard
        .get_or_insert_with(HashMap::new)
        .insert(id, Arc::new(AtomicI64::new(init)));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_atomic_i64_load(id: u64) -> i64 {
    let a = {
        let guard = match ATOMICS_I64.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    match a {
        Some(atom) => atom.load(Ordering::SeqCst),
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_atomic_i64_store(id: u64, val: i64) {
    let a = {
        let guard = match ATOMICS_I64.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    if let Some(atom) = a {
        atom.store(val, Ordering::SeqCst);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_atomic_i64_fetch_add(id: u64, val: i64) -> i64 {
    let a = {
        let guard = match ATOMICS_I64.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    match a {
        Some(atom) => atom.fetch_add(val, Ordering::SeqCst),
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_atomic_i64_fetch_sub(id: u64, val: i64) -> i64 {
    let a = {
        let guard = match ATOMICS_I64.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    match a {
        Some(atom) => atom.fetch_sub(val, Ordering::SeqCst),
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_atomic_i64_swap(id: u64, val: i64) -> i64 {
    let a = {
        let guard = match ATOMICS_I64.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    match a {
        Some(atom) => atom.swap(val, Ordering::SeqCst),
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_atomic_i64_cas(id: u64, expected: i64, desired: i64) -> bool {
    let a = {
        let guard = match ATOMICS_I64.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    a.is_some_and(|atom| {
        atom.compare_exchange(expected, desired, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_atomic_i64_destroy(id: u64) {
    let mut guard = match ATOMICS_I64.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(map) = guard.as_mut() {
        map.remove(&id);
    }
}

// c abi exports: atomic bool

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_atomic_bool_create(init: bool) -> u64 {
    let id = next_id();
    let mut guard = match ATOMICS_BOOL.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard
        .get_or_insert_with(HashMap::new)
        .insert(id, Arc::new(AtomicBool::new(init)));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_atomic_bool_load(id: u64) -> bool {
    let a = {
        let guard = match ATOMICS_BOOL.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    a.is_some_and(|atom| atom.load(Ordering::SeqCst))
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_atomic_bool_store(id: u64, val: bool) {
    let a = {
        let guard = match ATOMICS_BOOL.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    if let Some(atom) = a {
        atom.store(val, Ordering::SeqCst);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_atomic_bool_swap(id: u64, val: bool) -> bool {
    let a = {
        let guard = match ATOMICS_BOOL.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    a.is_some_and(|atom| atom.swap(val, Ordering::SeqCst))
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_atomic_bool_cas(id: u64, expected: bool, desired: bool) -> bool {
    let a = {
        let guard = match ATOMICS_BOOL.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        guard.as_ref().and_then(|map| map.get(&id).cloned())
    };
    a.is_some_and(|atom| {
        atom.compare_exchange(expected, desired, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sync_atomic_bool_destroy(id: u64) {
    let mut guard = match ATOMICS_BOOL.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(map) = guard.as_mut() {
        map.remove(&id);
    }
}
