use std::ffi::CString;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct YukinoString {
    pub ptr: *const u8,
    pub len: i64,
}

unsafe fn str_from_raw<'a>(ptr: *const u8, len: i64) -> &'a str {
    if ptr.is_null() || len <= 0 {
        return "";
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    std::str::from_utf8(slice).unwrap_or("")
}

unsafe fn parse_c_string_array(items_p: *const YukinoString, items_len: i64) -> (Vec<CString>, Vec<*const libc_types::c_char>) {
    if items_p.is_null() || items_len <= 0 {
        return (Vec::new(), vec![std::ptr::null()]);
    }
    let items = unsafe { std::slice::from_raw_parts(items_p, items_len as usize) };
    let mut cstrings = Vec::with_capacity(items.len());
    let mut ptrs = Vec::with_capacity(items.len() + 1);
    for item in items {
        let s = if item.ptr.is_null() || item.len <= 0 {
            ""
        } else {
            let slice = unsafe { std::slice::from_raw_parts(item.ptr, item.len as usize) };
            std::str::from_utf8(slice).unwrap_or("")
        };
        let cs = CString::new(s).unwrap_or_default();
        ptrs.push(cs.as_ptr());
        cstrings.push(cs);
    }
    ptrs.push(std::ptr::null());
    (cstrings, ptrs)
}

#[allow(non_camel_case_types)]
mod libc_types {
    pub type c_char = std::ffi::c_char;
    pub type c_int = std::ffi::c_int;
    pub type c_uint = std::ffi::c_uint;
}

#[cfg(unix)]
unsafe extern "C" {
    fn fork() -> libc_types::c_int;
    fn execve(
        pathname: *const libc_types::c_char,
        argv: *const *const libc_types::c_char,
        envp: *const *const libc_types::c_char,
    ) -> libc_types::c_int;
    fn waitpid(pid: libc_types::c_int, status: *mut libc_types::c_int, options: libc_types::c_int) -> libc_types::c_int;
    fn getpid() -> libc_types::c_int;
    fn getppid() -> libc_types::c_int;
    fn kill(pid: libc_types::c_int, sig: libc_types::c_int) -> libc_types::c_int;
    fn chmod(pathname: *const libc_types::c_char, mode: libc_types::c_uint) -> libc_types::c_int;
    fn chown(pathname: *const libc_types::c_char, owner: libc_types::c_uint, group: libc_types::c_uint) -> libc_types::c_int;
    fn umask(mask: libc_types::c_uint) -> libc_types::c_uint;
    fn getuid() -> libc_types::c_uint;
    fn geteuid() -> libc_types::c_uint;
    fn getgid() -> libc_types::c_uint;
    fn getegid() -> libc_types::c_uint;
    fn fcntl(fd: libc_types::c_int, cmd: libc_types::c_int, arg: libc_types::c_int) -> libc_types::c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_unix_fork() -> i32 {
    #[cfg(unix)]
    unsafe { fork() }
    #[cfg(not(unix))]
    { -1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_unix_execve(
    path_p: *const u8,
    path_len: i64,
    args_p: *const YukinoString,
    args_len: i64,
    envs_p: *const YukinoString,
    envs_len: i64,
) -> i32 {
    #[cfg(unix)]
    {
        let path = unsafe { str_from_raw(path_p, path_len) };
        let c_path = match CString::new(path) {
            Ok(p) => p,
            Err(_) => return -1,
        };
        let (_args_storage, args_ptrs) = unsafe { parse_c_string_array(args_p, args_len) };
        let (_envs_storage, envs_ptrs) = unsafe { parse_c_string_array(envs_p, envs_len) };

        unsafe { execve(c_path.as_ptr(), args_ptrs.as_ptr(), envs_ptrs.as_ptr()) }
    }
    #[cfg(not(unix))]
    { -1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_unix_waitpid(
    pid: i32,
    out_status: *mut i32,
    options: i32,
) -> i32 {
    #[cfg(unix)]
    {
        let mut st: libc_types::c_int = 0;
        let res = unsafe { waitpid(pid, &mut st, options) };
        if !out_status.is_null() {
            unsafe { *out_status = st; }
        }
        res
    }
    #[cfg(not(unix))]
    { -1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_unix_getpid() -> i32 {
    #[cfg(unix)]
    unsafe { getpid() }
    #[cfg(not(unix))]
    { -1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_unix_getppid() -> i32 {
    #[cfg(unix)]
    unsafe { getppid() }
    #[cfg(not(unix))]
    { -1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_unix_kill(pid: i32, sig: i32) -> i32 {
    #[cfg(unix)]
    unsafe { kill(pid, sig) }
    #[cfg(not(unix))]
    { -1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_unix_chmod(path_p: *const u8, path_len: i64, mode: u32) -> i32 {
    #[cfg(unix)]
    {
        let path = unsafe { str_from_raw(path_p, path_len) };
        let c_path = match CString::new(path) {
            Ok(p) => p,
            Err(_) => return -1,
        };
        unsafe { chmod(c_path.as_ptr(), mode) }
    }
    #[cfg(not(unix))]
    { -1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_unix_chown(path_p: *const u8, path_len: i64, uid: u32, gid: u32) -> i32 {
    #[cfg(unix)]
    {
        let path = unsafe { str_from_raw(path_p, path_len) };
        let c_path = match CString::new(path) {
            Ok(p) => p,
            Err(_) => return -1,
        };
        unsafe { chown(c_path.as_ptr(), uid, gid) }
    }
    #[cfg(not(unix))]
    { -1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_unix_umask(mask: u32) -> u32 {
    #[cfg(unix)]
    unsafe { umask(mask) }
    #[cfg(not(unix))]
    { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_unix_getuid() -> u32 {
    #[cfg(unix)]
    unsafe { getuid() }
    #[cfg(not(unix))]
    { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_unix_geteuid() -> u32 {
    #[cfg(unix)]
    unsafe { geteuid() }
    #[cfg(not(unix))]
    { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_unix_getgid() -> u32 {
    #[cfg(unix)]
    unsafe { getgid() }
    #[cfg(not(unix))]
    { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_unix_getegid() -> u32 {
    #[cfg(unix)]
    unsafe { getegid() }
    #[cfg(not(unix))]
    { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_unix_fcntl(fd: i32, cmd: i32, arg: i32) -> i32 {
    #[cfg(unix)]
    unsafe { fcntl(fd, cmd, arg) }
    #[cfg(not(unix))]
    { -1 }
}
