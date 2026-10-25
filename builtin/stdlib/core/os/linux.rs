use std::cell::RefCell;
use std::ffi::CString;

thread_local! {
    static LINUX_STR_RING: RefCell<(usize, Vec<Vec<u8>>)> = const { RefCell::new((0, Vec::new())) };
}

fn set_str_result(bytes: &[u8], out_len: *mut i64) -> *const u8 {
    LINUX_STR_RING.with(|cell| {
        let mut ring = cell.borrow_mut();
        if ring.1.len() < 16 {
            ring.1.resize_with(16, Vec::new);
        }
        let idx = ring.0 % 16;
        ring.0 = ring.0.wrapping_add(1);
        let buf = &mut ring.1[idx];
        buf.clear();
        buf.extend_from_slice(bytes);
        if !out_len.is_null() {
            unsafe { *out_len = buf.len() as i64; }
        }
        buf.as_ptr()
    })
}

unsafe fn str_from_raw<'a>(ptr: *const u8, len: i64) -> &'a str {
    if ptr.is_null() || len <= 0 { return ""; }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    std::str::from_utf8(slice).unwrap_or("")
}

#[cfg(target_os = "linux")]
unsafe extern "C" {
    fn epoll_create1(flags: std::ffi::c_int) -> std::ffi::c_int;
    fn eventfd(initval: std::ffi::c_uint, flags: std::ffi::c_int) -> std::ffi::c_int;
    fn timerfd_create(clockid: std::ffi::c_int, flags: std::ffi::c_int) -> std::ffi::c_int;
    fn prctl(option: std::ffi::c_int, ...) -> std::ffi::c_int;
    fn syscall(number: std::ffi::c_long, ...) -> std::ffi::c_long;
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_linux_epoll_create1(flags: i32) -> i32 {
    #[cfg(target_os = "linux")]
    unsafe { epoll_create1(flags) }
    #[cfg(not(target_os = "linux"))]
    { -1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_linux_eventfd(initval: u32, flags: i32) -> i32 {
    #[cfg(target_os = "linux")]
    unsafe { eventfd(initval, flags) }
    #[cfg(not(target_os = "linux"))]
    { -1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_linux_timerfd_create(clockid: i32, flags: i32) -> i32 {
    #[cfg(target_os = "linux")]
    unsafe { timerfd_create(clockid, flags) }
    #[cfg(not(target_os = "linux"))]
    { -1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_linux_memfd_create(name_p: *const u8, name_len: i64, flags: u32) -> i32 {
    #[cfg(target_os = "linux")]
    {
        let name = unsafe { str_from_raw(name_p, name_len) };
        let c_name = match CString::new(name) {
            Ok(n) => n,
            Err(_) => return -1,
        };
        // SYS_memfd_create is 319 on x86_64
        #[cfg(target_arch = "x86_64")]
        const SYS_MEMFD_CREATE: std::ffi::c_long = 319;
        #[cfg(target_arch = "aarch64")]
        const SYS_MEMFD_CREATE: std::ffi::c_long = 279;
        #[cfg(target_arch = "arm")]
        const SYS_MEMFD_CREATE: std::ffi::c_long = 385;
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "arm")))]
        const SYS_MEMFD_CREATE: std::ffi::c_long = 319;

        unsafe { syscall(SYS_MEMFD_CREATE, c_name.as_ptr(), flags as std::ffi::c_uint) as i32 }
    }
    #[cfg(not(target_os = "linux"))]
    { -1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_linux_prctl_set_name(name_p: *const u8, name_len: i64) -> i32 {
    #[cfg(target_os = "linux")]
    {
        let name = unsafe { str_from_raw(name_p, name_len) };
        let c_name = match CString::new(name) {
            Ok(n) => n,
            Err(_) => return -1,
        };
        const PR_SET_NAME: std::ffi::c_int = 15;
        unsafe { prctl(PR_SET_NAME, c_name.as_ptr() as std::ffi::c_ulong, 0, 0, 0) }
    }
    #[cfg(not(target_os = "linux"))]
    { -1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_linux_prctl_get_name(out_len: *mut i64) -> *const u8 {
    #[cfg(target_os = "linux")]
    {
        let mut buf = [0u8; 16];
        const PR_GET_NAME: std::ffi::c_int = 16;
        let res = unsafe { prctl(PR_GET_NAME, buf.as_mut_ptr() as std::ffi::c_ulong, 0, 0, 0) };
        if res == 0 {
            let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
            return set_str_result(&buf[..len], out_len);
        }
    }
    if !out_len.is_null() { unsafe { *out_len = 0; } }
    std::ptr::null()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_linux_gettid() -> i32 {
    #[cfg(target_os = "linux")]
    {
        #[cfg(target_arch = "x86_64")]
        const SYS_GETTID: std::ffi::c_long = 186;
        #[cfg(target_arch = "aarch64")]
        const SYS_GETTID: std::ffi::c_long = 178;
        #[cfg(target_arch = "arm")]
        const SYS_GETTID: std::ffi::c_long = 224;
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "arm")))]
        const SYS_GETTID: std::ffi::c_long = 186;

        unsafe { syscall(SYS_GETTID) as i32 }
    }
    #[cfg(not(target_os = "linux"))]
    { -1 }
}
