use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use polling::{Event, Events, Poller};

use super::error::{clear_last_error, record_io_error, set_last_error};
use super::tcp::{get_tcp_listener, get_tcp_stream};

static NEXT_POLLER_ID: AtomicU64 = AtomicU64::new(1);
static POLLERS: Mutex<Option<HashMap<u64, Arc<Poller>>>> = Mutex::new(None);

fn get_poller(id: u64) -> Option<Arc<Poller>> {
    let guard = match POLLERS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.as_ref().and_then(|map| map.get(&id).cloned())
}

fn to_polling_event(interest: u32, key: u64) -> Event {
    let k = key as usize;
    match interest {
        1 => Event::readable(k),
        2 => Event::writable(k),
        _ => Event::all(k),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_poller_create() -> u64 {
    clear_last_error();
    let poller = match Poller::new() {
        Ok(p) => p,
        Err(e) => {
            record_io_error(e);
            return 0;
        }
    };
    let id = NEXT_POLLER_ID.fetch_add(1, Ordering::Relaxed);
    let mut guard = match POLLERS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.get_or_insert_with(HashMap::new).insert(id, Arc::new(poller));
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_poller_add_tcp_stream(
    poller_id: u64,
    stream_id: u64,
    interest: u32,
    key: u64,
) -> bool {
    clear_last_error();
    let poller = match get_poller(poller_id) {
        Some(p) => p,
        None => {
            set_last_error(11, format!("unknown poller id {poller_id}"));
            return false;
        }
    };
    let stream = match get_tcp_stream(stream_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tcp stream id {stream_id}"));
            return false;
        }
    };
    let ev = to_polling_event(interest, key);
    match unsafe { poller.add(&*stream, ev) } {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_poller_add_tcp_listener(
    poller_id: u64,
    listener_id: u64,
    interest: u32,
    key: u64,
) -> bool {
    clear_last_error();
    let poller = match get_poller(poller_id) {
        Some(p) => p,
        None => {
            set_last_error(11, format!("unknown poller id {poller_id}"));
            return false;
        }
    };
    let listener = match get_tcp_listener(listener_id) {
        Some(l) => l,
        None => {
            set_last_error(11, format!("unknown tcp listener id {listener_id}"));
            return false;
        }
    };
    let ev = to_polling_event(interest, key);
    match unsafe { poller.add(&*listener, ev) } {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_poller_modify_tcp_stream(
    poller_id: u64,
    stream_id: u64,
    interest: u32,
    key: u64,
) -> bool {
    clear_last_error();
    let poller = match get_poller(poller_id) {
        Some(p) => p,
        None => {
            set_last_error(11, format!("unknown poller id {poller_id}"));
            return false;
        }
    };
    let stream = match get_tcp_stream(stream_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tcp stream id {stream_id}"));
            return false;
        }
    };
    let ev = to_polling_event(interest, key);
    match poller.modify(&*stream, ev) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_poller_delete_tcp_stream(poller_id: u64, stream_id: u64) -> bool {
    clear_last_error();
    let poller = match get_poller(poller_id) {
        Some(p) => p,
        None => {
            set_last_error(11, format!("unknown poller id {poller_id}"));
            return false;
        }
    };
    let stream = match get_tcp_stream(stream_id) {
        Some(s) => s,
        None => {
            set_last_error(11, format!("unknown tcp stream id {stream_id}"));
            return false;
        }
    };
    match poller.delete(&*stream) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_poller_delete_tcp_listener(poller_id: u64, listener_id: u64) -> bool {
    clear_last_error();
    let poller = match get_poller(poller_id) {
        Some(p) => p,
        None => {
            set_last_error(11, format!("unknown poller id {poller_id}"));
            return false;
        }
    };
    let listener = match get_tcp_listener(listener_id) {
        Some(l) => l,
        None => {
            set_last_error(11, format!("unknown tcp listener id {listener_id}"));
            return false;
        }
    };
    match poller.delete(&*listener) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_poller_wait(
    poller_id: u64,
    out_keys: *mut u64,
    out_events: *mut u32,
    max_events: i32,
    timeout_ms: i32,
) -> i32 {
    clear_last_error();
    if out_keys.is_null() || out_events.is_null() || max_events <= 0 {
        set_last_error(11, "invalid buffer pointers or max_events <= 0".to_string());
        return -1;
    }
    let poller = match get_poller(poller_id) {
        Some(p) => p,
        None => {
            set_last_error(11, format!("unknown poller id {poller_id}"));
            return -1;
        }
    };
    let mut events = Events::with_capacity(std::num::NonZeroUsize::new(max_events as usize).unwrap());
    let timeout = if timeout_ms < 0 {
        None
    } else {
        Some(Duration::from_millis(timeout_ms as u64))
    };
    if let Err(e) = poller.wait(&mut events, timeout) {
        record_io_error(e);
        return -1;
    }
    let mut count = 0;
    for (i, ev) in events.iter().enumerate() {
        if i >= max_events as usize {
            break;
        }
        unsafe {
            *out_keys.add(i) = ev.key as u64;
            let mut mask: u32 = 0;
            if ev.readable {
                mask |= 1;
            }
            if ev.writable {
                mask |= 2;
            }
            *out_events.add(i) = mask;
        }
        count += 1;
    }
    count
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_poller_destroy(poller_id: u64) -> bool {
    clear_last_error();
    let mut guard = match POLLERS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(map) = guard.as_mut() {
        map.remove(&poller_id).is_some()
    } else {
        false
    }
}
