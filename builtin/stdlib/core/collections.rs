use std::cell::RefCell;
use std::collections::{HashMap, HashSet, LinkedList, VecDeque};

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
            if !out_len.is_null() {
                unsafe {
                    *out_len = val.len() as i64;
                }
            }
            val.as_ptr()
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

#[derive(Default)]
pub struct YukinoLinkedList {
    i64_list: LinkedList<i64>,
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_linked_list_new() -> *mut YukinoLinkedList {
    Box::into_raw(Box::new(YukinoLinkedList::default()))
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_linked_list_free(l: *mut YukinoLinkedList) {
    if !l.is_null() {
        drop(unsafe { Box::from_raw(l) });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_linked_list_push_back(l: *mut YukinoLinkedList, val: i64) {
    if l.is_null() {
        return;
    }
    let list = unsafe { &mut *l };
    list.i64_list.push_back(val);
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_linked_list_push_front(l: *mut YukinoLinkedList, val: i64) {
    if l.is_null() {
        return;
    }
    let list = unsafe { &mut *l };
    list.i64_list.push_front(val);
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_linked_list_pop_front(
    l: *mut YukinoLinkedList,
    out_found: *mut bool,
) -> i64 {
    if l.is_null() {
        if !out_found.is_null() {
            unsafe { *out_found = false; }
        }
        return 0;
    }
    let list = unsafe { &mut *l };
    match list.i64_list.pop_front() {
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
pub extern "C" fn yukistd_linked_list_pop_back(
    l: *mut YukinoLinkedList,
    out_found: *mut bool,
) -> i64 {
    if l.is_null() {
        if !out_found.is_null() {
            unsafe { *out_found = false; }
        }
        return 0;
    }
    let list = unsafe { &mut *l };
    match list.i64_list.pop_back() {
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
pub extern "C" fn yukistd_linked_list_peek_front(
    l: *mut YukinoLinkedList,
    out_found: *mut bool,
) -> i64 {
    if l.is_null() {
        if !out_found.is_null() {
            unsafe { *out_found = false; }
        }
        return 0;
    }
    let list = unsafe { &*l };
    match list.i64_list.front() {
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
pub extern "C" fn yukistd_linked_list_peek_back(
    l: *mut YukinoLinkedList,
    out_found: *mut bool,
) -> i64 {
    if l.is_null() {
        if !out_found.is_null() {
            unsafe { *out_found = false; }
        }
        return 0;
    }
    let list = unsafe { &*l };
    match list.i64_list.back() {
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
pub extern "C" fn yukistd_linked_list_len(l: *mut YukinoLinkedList) -> i64 {
    if l.is_null() {
        return 0;
    }
    let list = unsafe { &*l };
    list.i64_list.len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_linked_list_clear(l: *mut YukinoLinkedList) {
    if l.is_null() {
        return;
    }
    let list = unsafe { &mut *l };
    list.i64_list.clear();
}

#[repr(C)]
pub struct YukinoIntrusiveNode {
    pub prev: *mut YukinoIntrusiveNode,
    pub next: *mut YukinoIntrusiveNode,
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_intrusive_node_init(node: *mut YukinoIntrusiveNode) {
    if node.is_null() {
        return;
    }
    unsafe {
        (*node).prev = node;
        (*node).next = node;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_intrusive_list_new() -> *mut YukinoIntrusiveNode {
    let node = Box::into_raw(Box::new(YukinoIntrusiveNode {
        prev: std::ptr::null_mut(),
        next: std::ptr::null_mut(),
    }));
    unsafe {
        (*node).prev = node;
        (*node).next = node;
    }
    node
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_intrusive_list_free(head: *mut YukinoIntrusiveNode) {
    if !head.is_null() {
        drop(unsafe { Box::from_raw(head) });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_intrusive_node_is_linked(node: *mut YukinoIntrusiveNode) -> bool {
    if node.is_null() {
        return false;
    }
    unsafe { (*node).next != node && !(*node).next.is_null() }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_intrusive_list_is_empty(head: *mut YukinoIntrusiveNode) -> bool {
    if head.is_null() {
        return true;
    }
    unsafe { (*head).next == head }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_intrusive_list_len(head: *mut YukinoIntrusiveNode) -> i64 {
    if head.is_null() {
        return 0;
    }
    let mut count = 0i64;
    unsafe {
        let mut curr = (*head).next;
        while curr != head && !curr.is_null() {
            count += 1;
            curr = (*curr).next;
        }
    }
    count
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_intrusive_list_push_back(
    head: *mut YukinoIntrusiveNode,
    node: *mut YukinoIntrusiveNode,
) {
    if head.is_null() || node.is_null() {
        return;
    }
    unsafe {
        let prev = (*head).prev;
        (*node).next = head;
        (*node).prev = prev;
        (*prev).next = node;
        (*head).prev = node;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_intrusive_list_push_front(
    head: *mut YukinoIntrusiveNode,
    node: *mut YukinoIntrusiveNode,
) {
    if head.is_null() || node.is_null() {
        return;
    }
    unsafe {
        let next = (*head).next;
        (*node).prev = head;
        (*node).next = next;
        (*next).prev = node;
        (*head).next = node;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_intrusive_list_pop_front(
    head: *mut YukinoIntrusiveNode,
) -> *mut YukinoIntrusiveNode {
    if head.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let first = (*head).next;
        if first == head || first.is_null() {
            return std::ptr::null_mut();
        }
        let next = (*first).next;
        (*head).next = next;
        (*next).prev = head;
        (*first).prev = first;
        (*first).next = first;
        first
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_intrusive_list_pop_back(
    head: *mut YukinoIntrusiveNode,
) -> *mut YukinoIntrusiveNode {
    if head.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let last = (*head).prev;
        if last == head || last.is_null() {
            return std::ptr::null_mut();
        }
        let prev = (*last).prev;
        (*head).prev = prev;
        (*prev).next = head;
        (*last).prev = last;
        (*last).next = last;
        last
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_intrusive_list_remove(node: *mut YukinoIntrusiveNode) {
    if node.is_null() {
        return;
    }
    unsafe {
        let prev = (*node).prev;
        let next = (*node).next;
        if !prev.is_null() {
            (*prev).next = next;
        }
        if !next.is_null() {
            (*next).prev = prev;
        }
        (*node).prev = node;
        (*node).next = node;
    }
}
