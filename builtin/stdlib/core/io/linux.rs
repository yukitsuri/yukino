#[cfg(target_os = "linux")]
unsafe extern "C" {
    fn pipe2(pipefd: *mut i32, flags: i32) -> i32;
    fn dup3(oldfd: i32, newfd: i32, flags: i32) -> i32;
    fn splice(
        fd_in: i32,
        off_in: *mut i64,
        fd_out: i32,
        off_out: *mut i64,
        len: usize,
        flags: u32,
    ) -> isize;
    fn tee(fd_in: i32, fd_out: i32, len: usize, flags: u32) -> isize;
}

#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_linux_pipe2(read_fd: *mut i32, write_fd: *mut i32, flags: i32) -> i32 {
    let mut fds = [0i32; 2];
    let res = unsafe { pipe2(fds.as_mut_ptr(), flags) };
    if res == 0 {
        if !read_fd.is_null() { unsafe { *read_fd = fds[0]; } }
        if !write_fd.is_null() { unsafe { *write_fd = fds[1]; } }
    }
    res
}

#[cfg(not(target_os = "linux"))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_linux_pipe2(
    _read_fd: *mut i32,
    _write_fd: *mut i32,
    _flags: i32,
) -> i32 { -1 }

#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_linux_dup3(oldfd: i32, newfd: i32, flags: i32) -> i32 {
    unsafe { dup3(oldfd, newfd, flags) }
}

#[cfg(not(target_os = "linux"))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_linux_dup3(_oldfd: i32, _newfd: i32, _flags: i32) -> i32 { -1 }

#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_linux_splice(
    fd_in: i32,
    off_in: *mut i64,
    fd_out: i32,
    off_out: *mut i64,
    len: usize,
    flags: u32,
) -> i64 { unsafe { splice(fd_in, off_in, fd_out, off_out, len, flags) as i64 } }

#[cfg(not(target_os = "linux"))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_linux_splice(
    _fd_in: i32,
    _off_in: *mut i64,
    _fd_out: i32,
    _off_out: *mut i64,
    _len: usize,
    _flags: u32,
) -> i64 { -1 }

#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_linux_tee(fd_in: i32, fd_out: i32, len: usize, flags: u32) -> i64 {
    unsafe { tee(fd_in, fd_out, len, flags) as i64 }
}

#[cfg(not(target_os = "linux"))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_linux_tee(
    _fd_in: i32,
    _fd_out: i32,
    _len: usize,
    _flags: u32,
) -> i64 { -1 }
