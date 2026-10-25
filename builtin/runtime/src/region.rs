use std::alloc::{Layout, alloc, dealloc};

pub const DEFAULT_CHUNK_CAPACITY: usize = 64 * 1024;
pub const REGION_ALIGNMENT: usize = 16;
pub const HEADER_PAYLOAD_OFFSET: usize = 32;

#[repr(C)]
pub struct YukinoChunk {
    pub next: *mut YukinoChunk,
    pub capacity: usize,
    pub used: usize,
}

#[repr(C)]
pub struct YukinoRegion {
    pub head: *mut YukinoChunk,
    pub current_ptr: *mut u8,
    pub end_ptr: *mut u8,
}

impl YukinoChunk {
    fn layout_for_capacity(capacity: usize) -> Layout {
        Layout::from_size_align(HEADER_PAYLOAD_OFFSET + capacity, REGION_ALIGNMENT)
            .expect("valid chunk layout")
    }

    unsafe fn allocate(capacity: usize, next: *mut YukinoChunk) -> *mut YukinoChunk {
        let layout = Self::layout_for_capacity(capacity);
        let raw = unsafe { alloc(layout) };
        if raw.is_null() { std::alloc::handle_alloc_error(layout); }
        let chunk_ptr = raw.cast::<YukinoChunk>();
        unsafe {
            (*chunk_ptr).next = next;
            (*chunk_ptr).capacity = capacity;
            (*chunk_ptr).used = 0;
        }
        chunk_ptr
    }

    unsafe fn payload_start(chunk_ptr: *mut YukinoChunk) -> *mut u8 {
        unsafe { chunk_ptr.cast::<u8>().add(HEADER_PAYLOAD_OFFSET) }
    }

    unsafe fn deallocate(chunk_ptr: *mut YukinoChunk) {
        unsafe {
            let layout = Self::layout_for_capacity((*chunk_ptr).capacity);
            dealloc(chunk_ptr.cast(), layout);
        }
    }
}

// the reg ptr must point to valid, allocated, aligned memory for YukinoRegion
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_region_init(region: *mut YukinoRegion, initial_capacity: usize) {
    if region.is_null() { return; }
    let cap = if initial_capacity == 0 { DEFAULT_CHUNK_CAPACITY
    } else { initial_capacity };
    unsafe {
        let chunk = YukinoChunk::allocate(cap, std::ptr::null_mut());
        let payload = YukinoChunk::payload_start(chunk);
        (*region).head = chunk;
        (*region).current_ptr = payload;
        (*region).end_ptr = payload.add(cap);
    }
}

// the reg pointer must be a valid initialized pointer to YukinoRegion
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_region_alloc(
    region: *mut YukinoRegion,
    size: usize,
    align: usize,
) -> *mut u8 {
    if region.is_null() { return std::ptr::null_mut(); }
    let effective_align = align.max(1).next_power_of_two();
    unsafe {
        let current = (*region).current_ptr;
        let end = (*region).end_ptr;
        let current_addr = current as usize;
        let aligned_addr = (current_addr + (effective_align - 1)) & !(effective_align - 1);
        let required = (aligned_addr - current_addr) + size;

        if current_addr + required <= end as usize {
            (*region).current_ptr = (aligned_addr + size) as *mut u8;
            return aligned_addr as *mut u8;
        }

        let new_cap = size.max(DEFAULT_CHUNK_CAPACITY);
        let new_chunk = YukinoChunk::allocate(new_cap, (*region).head);
        let payload = YukinoChunk::payload_start(new_chunk);
        let payload_addr = payload as usize;
        let aligned_payload = (payload_addr + (effective_align - 1)) & !(effective_align - 1);

        (*region).head = new_chunk;
        (*region).current_ptr = (aligned_payload + size) as *mut u8;
        (*region).end_ptr = payload.add(new_cap);
        aligned_payload as *mut u8
    }
}

// thereg ptr must be a valid inited pointer to YukinoRegion
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_region_save_mark(region: *mut YukinoRegion) -> *mut u8 {
    if region.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { (*region).current_ptr }
}


// the region ptr must be valid and "mark" must have been returned by yukino_region_save_mark
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_region_restore_mark(region: *mut YukinoRegion, mark: *mut u8) {
    if region.is_null() || mark.is_null() { return; }
    unsafe {
        let mut curr = (*region).head;
        while !curr.is_null() {
            let start = YukinoChunk::payload_start(curr);
            let end = start.add((*curr).capacity);
            if mark >= start && mark <= end {
                while (*region).head != curr {
                    let to_free = (*region).head;
                    (*region).head = (*to_free).next;
                    YukinoChunk::deallocate(to_free);
                }
                (*region).current_ptr = mark;
                (*region).end_ptr = end;
                return;
            }
            curr = (*curr).next;
        }
    }
}


// The `region` pointer must point to an initialized `YukinoRegion`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_region_reset(region: *mut YukinoRegion) {
    if region.is_null() { return; }
    unsafe {
        let head = (*region).head;
        if head.is_null() { return; }
        let mut curr = (*head).next;
        while !curr.is_null() {
            let next = (*curr).next;
            YukinoChunk::deallocate(curr);
            curr = next;
        }
        (*head).next = std::ptr::null_mut();
        (*head).used = 0;
        let payload = YukinoChunk::payload_start(head);
        (*region).current_ptr = payload;
        (*region).end_ptr = payload.add((*head).capacity);
    }
}


// the reg ptr must point to an inited YukinoRegion
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_region_used_bytes(region: *mut YukinoRegion) -> usize {
    if region.is_null() { return 0; }
    unsafe {
        let head = (*region).head;
        if head.is_null() || (*region).current_ptr.is_null() { return 0; }
        let head_payload = YukinoChunk::payload_start(head) as usize;
        let current = (*region).current_ptr as usize;
        let mut total = current.saturating_sub(head_payload);
        let mut curr = (*head).next;
        while !curr.is_null() {
            total = total.saturating_add((*curr).capacity);
            curr = (*curr).next;
        }
        total
    }
}


// the reg pointer must point to an inited YukinoRegion
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_region_destroy(region: *mut YukinoRegion) {
    if region.is_null() { return; }
    unsafe {
        let mut curr = (*region).head;
        while !curr.is_null() {
            let next = (*curr).next;
            YukinoChunk::deallocate(curr);
            curr = next;
        }
        (*region).head = std::ptr::null_mut();
        (*region).current_ptr = std::ptr::null_mut();
        (*region).end_ptr = std::ptr::null_mut();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_lifecycle() {
        unsafe {
            let mut reg = YukinoRegion {
                head: std::ptr::null_mut(),
                current_ptr: std::ptr::null_mut(),
                end_ptr: std::ptr::null_mut(),
            };
            yukino_region_init(&raw mut reg, 4096);
            assert!(!reg.head.is_null());
            assert_eq!((*reg.head).capacity, 4096);

            let ptr1 = yukino_region_alloc(&raw mut reg, 64, 8);
            assert!(!ptr1.is_null());
            assert_eq!((ptr1 as usize) % 8, 0);

            let ptr2 = yukino_region_alloc(&raw mut reg, 128, 16);
            assert!(!ptr2.is_null());
            assert_eq!((ptr2 as usize) % 16, 0);

            yukino_region_destroy(&raw mut reg);
            assert!(reg.head.is_null());
        }
    }

    #[test]
    fn test_region_save_restore_mark() {
        unsafe {
            let mut reg = YukinoRegion {
                head: std::ptr::null_mut(),
                current_ptr: std::ptr::null_mut(),
                end_ptr: std::ptr::null_mut(),
            };
            yukino_region_init(&raw mut reg, 1024);

            let _ = yukino_region_alloc(&raw mut reg, 32, 8);
            let mark = yukino_region_save_mark(&raw mut reg);

            let alloc_inside = yukino_region_alloc(&raw mut reg, 64, 8);
            assert!(alloc_inside >= mark);

            yukino_region_restore_mark(&raw mut reg, mark);
            assert_eq!(reg.current_ptr, mark);

            yukino_region_destroy(&raw mut reg);
        }
    }

    #[test]
    fn test_region_multi_chunk_overflow() {
        unsafe {
            let mut reg = YukinoRegion {
                head: std::ptr::null_mut(),
                current_ptr: std::ptr::null_mut(),
                end_ptr: std::ptr::null_mut(),
            };
            yukino_region_init(&raw mut reg, 512);

            let _ = yukino_region_alloc(&raw mut reg, 256, 8);
            let mark = yukino_region_save_mark(&raw mut reg);

            let big_alloc = yukino_region_alloc(&raw mut reg, 2048, 16);
            assert!(!big_alloc.is_null());
            assert_ne!((*reg.head).next, std::ptr::null_mut());

            yukino_region_restore_mark(&raw mut reg, mark);
            assert_eq!(reg.current_ptr, mark);

            yukino_region_destroy(&raw mut reg);
        }
    }

    #[test]
    fn test_region_reset() {
        unsafe {
            let mut reg = YukinoRegion {
                head: std::ptr::null_mut(),
                current_ptr: std::ptr::null_mut(),
                end_ptr: std::ptr::null_mut(),
            };
            yukino_region_init(&raw mut reg, 512);

            let _ = yukino_region_alloc(&raw mut reg, 256, 8);
            let big_alloc = yukino_region_alloc(&raw mut reg, 2048, 16);
            assert!(!big_alloc.is_null());
            assert!(!(*reg.head).next.is_null());

            yukino_region_reset(&raw mut reg);
            assert!((*reg.head).next.is_null());
            assert_eq!(reg.current_ptr, YukinoChunk::payload_start(reg.head));
            assert_eq!(yukino_region_used_bytes(&raw mut reg), 0);

            yukino_region_destroy(&raw mut reg);
        }
    }

    #[test]
    fn test_region_used_bytes() {
        unsafe {
            let mut reg = YukinoRegion {
                head: std::ptr::null_mut(),
                current_ptr: std::ptr::null_mut(),
                end_ptr: std::ptr::null_mut(),
            };
            yukino_region_init(&raw mut reg, 1024);
            assert_eq!(yukino_region_used_bytes(&raw mut reg), 0);

            let _ = yukino_region_alloc(&raw mut reg, 128, 8);
            assert!(yukino_region_used_bytes(&raw mut reg) >= 128);

            yukino_region_destroy(&raw mut reg);
        }
    }
}
