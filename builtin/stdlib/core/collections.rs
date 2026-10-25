use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::strings::YukinoString;

#[derive(Default)]
pub struct YukinoMap {
    str_map: HashMap<String, String>,
    i64_map: HashMap<i64, i64>,
}

#[derive(Default)]
pub struct YukinoSet {
    str_set: HashSet<String>,
    i64_set: HashSet<i64>,
}

#[derive(Default)]
pub struct YukinoDeque {
    i64_deque: VecDeque<i64>,
}

thread_local! {
    static MAP_STR_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static MAP_KEYS_BUF: RefCell<Vec<YukinoString>> = const { RefCell::new(Vec::new()) };
    static MAP_VALS_BUF: RefCell<Vec<YukinoString>> = const { RefCell::new(Vec::new()) };
    static SET_ITEMS_BUF: RefCell<Vec<YukinoString>> = const { RefCell::new(Vec::new()) };
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_map_new() -> *mut YukinoMap {
    Box::into_raw(Box::new(YukinoMap::default()))
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_map_free(m: *mut YukinoMap) {
    if !m.is_null() {
        drop(unsafe { Box::from_raw(m) });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_map_set(
    m: *mut YukinoMap,
    k_ptr: *const u8,
    k_len: i64,
    v_ptr: *const u8,
    v_len: i64,
) {
    if m.is_null() || k_ptr.is_null() || k_len < 0 || v_ptr.is_null() || v_len < 0 {
        return;
    }
    let k_slice = unsafe { std::slice::from_raw_parts(k_ptr, k_len as usize) };
    let v_slice = unsafe { std::slice::from_raw_parts(v_ptr, v_len as usize) };
    let k = String::from_utf8_lossy(k_slice).into_owned();
    let v = String::from_utf8_lossy(v_slice).into_owned();
    let map = unsafe { &mut *m };
    map.str_map.insert(k, v);
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_map_get(
    m: *mut YukinoMap,
    k_ptr: *const u8,
    k_len: i64,
    out_len: *mut i64,
) -> *const u8 {
    if m.is_null() || k_ptr.is_null() || k_len < 0 {
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        return std::ptr::null();
    }
    let k_slice = unsafe { std::slice::from_raw_parts(k_ptr, k_len as usize) };
    let k = String::from_utf8_lossy(k_slice);
    let map = unsafe { &*m };
    match map.str_map.get(k.as_ref()) {
        Some(val) => {
            MAP_STR_BUF.with(|buf_cell| {
                let mut buf = buf_cell.borrow_mut();
                buf.clear();
                buf.extend_from_slice(val.as_bytes());
                if !out_len.is_null() {
                    unsafe { *out_len = buf.len() as i64; }
                }
                buf.as_ptr()
            })
        }
        None => {
            if !out_len.is_null() {
                unsafe { *out_len = 0; }
            }
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_map_has(m: *mut YukinoMap, k_ptr: *const u8, k_len: i64) -> bool {
    if m.is_null() || k_ptr.is_null() || k_len < 0 {
        return false;
    }
    let k_slice = unsafe { std::slice::from_raw_parts(k_ptr, k_len as usize) };
    let k = String::from_utf8_lossy(k_slice);
    let map = unsafe { &*m };
    map.str_map.contains_key(k.as_ref())
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_map_del(m: *mut YukinoMap, k_ptr: *const u8, k_len: i64) -> bool {
    if m.is_null() || k_ptr.is_null() || k_len < 0 {
        return false;
    }
    let k_slice = unsafe { std::slice::from_raw_parts(k_ptr, k_len as usize) };
    let k = String::from_utf8_lossy(k_slice);
    let map = unsafe { &mut *m };
    map.str_map.remove(k.as_ref()).is_some()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_map_len(m: *mut YukinoMap) -> i64 {
    if m.is_null() {
        return 0;
    }
    let map = unsafe { &*m };
    (map.str_map.len() + map.i64_map.len()) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_map_clear(m: *mut YukinoMap) {
    if m.is_null() {
        return;
    }
    let map = unsafe { &mut *m };
    map.str_map.clear();
    map.i64_map.clear();
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_map_set_i64(m: *mut YukinoMap, k: i64, v: i64) {
    if m.is_null() {
        return;
    }
    let map = unsafe { &mut *m };
    map.i64_map.insert(k, v);
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_map_get_i64(m: *mut YukinoMap, k: i64, out_found: *mut bool) -> i64 {
    if m.is_null() {
        if !out_found.is_null() {
            unsafe { *out_found = false; }
        }
        return 0;
    }
    let map = unsafe { &*m };
    match map.i64_map.get(&k) {
        Some(&v) => {
            if !out_found.is_null() {
                unsafe { *out_found = true; }
            }
            v
        }
        None => {
            if !out_found.is_null() {
                unsafe { *out_found = false; }
            }
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_map_has_i64(m: *mut YukinoMap, k: i64) -> bool {
    if m.is_null() {
        return false;
    }
    let map = unsafe { &*m };
    map.i64_map.contains_key(&k)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_map_del_i64(m: *mut YukinoMap, k: i64) -> bool {
    if m.is_null() {
        return false;
    }
    let map = unsafe { &mut *m };
    map.i64_map.remove(&k).is_some()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_set_new() -> *mut YukinoSet {
    Box::into_raw(Box::new(YukinoSet::default()))
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_set_free(s: *mut YukinoSet) {
    if !s.is_null() {
        drop(unsafe { Box::from_raw(s) });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_set_add(s: *mut YukinoSet, v_ptr: *const u8, v_len: i64) -> bool {
    if s.is_null() || v_ptr.is_null() || v_len < 0 {
        return false;
    }
    let v_slice = unsafe { std::slice::from_raw_parts(v_ptr, v_len as usize) };
    let v = String::from_utf8_lossy(v_slice).into_owned();
    let set = unsafe { &mut *s };
    set.str_set.insert(v)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_set_has(s: *mut YukinoSet, v_ptr: *const u8, v_len: i64) -> bool {
    if s.is_null() || v_ptr.is_null() || v_len < 0 {
        return false;
    }
    let v_slice = unsafe { std::slice::from_raw_parts(v_ptr, v_len as usize) };
    let v = String::from_utf8_lossy(v_slice);
    let set = unsafe { &*s };
    set.str_set.contains(v.as_ref())
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_set_del(s: *mut YukinoSet, v_ptr: *const u8, v_len: i64) -> bool {
    if s.is_null() || v_ptr.is_null() || v_len < 0 {
        return false;
    }
    let v_slice = unsafe { std::slice::from_raw_parts(v_ptr, v_len as usize) };
    let v = String::from_utf8_lossy(v_slice);
    let set = unsafe { &mut *s };
    set.str_set.remove(v.as_ref())
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_set_len(s: *mut YukinoSet) -> i64 {
    if s.is_null() {
        return 0;
    }
    let set = unsafe { &*s };
    (set.str_set.len() + set.i64_set.len()) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_set_clear(s: *mut YukinoSet) {
    if s.is_null() {
        return;
    }
    let set = unsafe { &mut *s };
    set.str_set.clear();
    set.i64_set.clear();
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_set_add_i64(s: *mut YukinoSet, val: i64) -> bool {
    if s.is_null() {
        return false;
    }
    let set = unsafe { &mut *s };
    set.i64_set.insert(val)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_set_has_i64(s: *mut YukinoSet, val: i64) -> bool {
    if s.is_null() {
        return false;
    }
    let set = unsafe { &*s };
    set.i64_set.contains(&val)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_set_del_i64(s: *mut YukinoSet, val: i64) -> bool {
    if s.is_null() {
        return false;
    }
    let set = unsafe { &mut *s };
    set.i64_set.remove(&val)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_deque_new() -> *mut YukinoDeque {
    Box::into_raw(Box::new(YukinoDeque::default()))
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_deque_free(d: *mut YukinoDeque) {
    if !d.is_null() {
        drop(unsafe { Box::from_raw(d) });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_deque_push_back(d: *mut YukinoDeque, val: i64) {
    if d.is_null() {
        return;
    }
    let deque = unsafe { &mut *d };
    deque.i64_deque.push_back(val);
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_deque_push_front(d: *mut YukinoDeque, val: i64) {
    if d.is_null() {
        return;
    }
    let deque = unsafe { &mut *d };
    deque.i64_deque.push_front(val);
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_deque_pop_front(d: *mut YukinoDeque, out_found: *mut bool) -> i64 {
    if d.is_null() {
        if !out_found.is_null() {
            unsafe { *out_found = false; }
        }
        return 0;
    }
    let deque = unsafe { &mut *d };
    match deque.i64_deque.pop_front() {
        Some(val) => {
            if !out_found.is_null() {
                unsafe { *out_found = true; }
            }
            val
        }
        None => {
            if !out_found.is_null() {
                unsafe { *out_found = false; }
            }
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_deque_pop_back(d: *mut YukinoDeque, out_found: *mut bool) -> i64 {
    if d.is_null() {
        if !out_found.is_null() {
            unsafe { *out_found = false; }
        }
        return 0;
    }
    let deque = unsafe { &mut *d };
    match deque.i64_deque.pop_back() {
        Some(val) => {
            if !out_found.is_null() {
                unsafe { *out_found = true; }
            }
            val
        }
        None => {
            if !out_found.is_null() {
                unsafe { *out_found = false; }
            }
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_deque_peek_front(d: *mut YukinoDeque, out_found: *mut bool) -> i64 {
    if d.is_null() {
        if !out_found.is_null() {
            unsafe { *out_found = false; }
        }
        return 0;
    }
    let deque = unsafe { &*d };
    match deque.i64_deque.front() {
        Some(&val) => {
            if !out_found.is_null() {
                unsafe { *out_found = true; }
            }
            val
        }
        None => {
            if !out_found.is_null() {
                unsafe { *out_found = false; }
            }
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_deque_peek_back(d: *mut YukinoDeque, out_found: *mut bool) -> i64 {
    if d.is_null() {
        if !out_found.is_null() {
            unsafe { *out_found = false; }
        }
        return 0;
    }
    let deque = unsafe { &*d };
    match deque.i64_deque.back() {
        Some(&val) => {
            if !out_found.is_null() {
                unsafe { *out_found = true; }
            }
            val
        }
        None => {
            if !out_found.is_null() {
                unsafe { *out_found = false; }
            }
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_deque_len(d: *mut YukinoDeque) -> i64 {
    if d.is_null() {
        return 0;
    }
    let deque = unsafe { &*d };
    deque.i64_deque.len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_deque_clear(d: *mut YukinoDeque) {
    if d.is_null() {
        return;
    }
    let deque = unsafe { &mut *d };
    deque.i64_deque.clear();
}
