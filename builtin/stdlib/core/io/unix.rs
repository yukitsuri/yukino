use std::sync::Mutex;

#[cfg(unix)]
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct WinSize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

#[cfg(unix)]
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Termios {
    c_iflag: u32,
    c_oflag: u32,
    c_cflag: u32,
    c_lflag: u32,
    c_line: u8,
    c_cc: [u8; 32],
    c_ispeed: u32,
    c_ospeed: u32,
}

#[cfg(unix)]
#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

#[cfg(unix)]
unsafe extern "C" {
    fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
    fn ioctl(fd: i32, request: u64, ...) -> i32;
    fn tcgetattr(fd: i32, termios_p: *mut Termios) -> i32;
    fn tcsetattr(fd: i32, optional_actions: i32, termios_p: *const Termios) -> i32;
    fn poll(fds: *mut PollFd, nfds: usize, timeout: i32) -> i32;
    fn isatty(fd: i32) -> i32;
    fn pipe(pipefd: *mut i32) -> i32;
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn fsync(fd: i32) -> i32;
    fn fdatasync(fd: i32) -> i32;
}

#[cfg(unix)]
static SAVED_TERMIOS: Mutex<Option<Termios>> = Mutex::new(None);

#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_read_stdin_byte() -> i32 {
    let mut b = 0u8;
    let n = unsafe { read(0, &mut b, 1) };
    if n == 1 { b as i32} else { -1 }
}

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_read_stdin_byte() -> i32 {
    #[cfg(windows)]
    {
        super::windows::win_read_stdin_byte()
    }
    #[cfg(not(windows))]
    {
        -1
    }
}

#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_poll_stdin(timeout_ms: i64) -> bool {
    let mut pfd = PollFd {
        fd: 0,
        events: 0x0001,
        revents: 0,
    };
    let t = if timeout_ms < 0 { -1 } else { timeout_ms as i32 };
    let res = unsafe { poll(&mut pfd, 1, t) };
    res > 0 && (pfd.revents & 0x0001 != 0)
}

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_poll_stdin(timeout_ms: i64) -> bool {
    #[cfg(windows)]
    {
        super::windows::win_poll_stdin(timeout_ms)
    }
    #[cfg(not(windows))]
    {
        let _ = timeout_ms;
        false
    }
}

#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_terminal_size(out_cols: *mut u32, out_rows: *mut u32) -> bool {
    let mut ws = WinSize::default();
    const TIOCGWINSZ: u64 = 0x5413;
    let mut ret = unsafe { ioctl(1, TIOCGWINSZ, &mut ws) };
    if ret != 0 || ws.ws_col == 0 || ws.ws_row == 0 {
        ret = unsafe { ioctl(0, TIOCGWINSZ, &mut ws) };
    }
    if ret != 0 || ws.ws_col == 0 || ws.ws_row == 0 {
        ret = unsafe { ioctl(2, TIOCGWINSZ, &mut ws) };
    }
    if ret == 0 && ws.ws_col > 0 && ws.ws_row > 0 {
        if !out_cols.is_null() { unsafe { *out_cols = ws.ws_col as u32; } }
        if !out_rows.is_null() { unsafe { *out_rows = ws.ws_row as u32; } }
        return true;
    }
    if !out_cols.is_null() { unsafe { *out_cols = 80; } }
    if !out_rows.is_null() { unsafe { *out_rows = 24; } }
    false
}

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_terminal_size(out_cols: *mut u32, out_rows: *mut u32) -> bool {
    #[cfg(windows)]
    {
        super::windows::win_terminal_size(out_cols, out_rows)
    }
    #[cfg(not(windows))]
    {
        if !out_cols.is_null() { unsafe { *out_cols = 80; } }
        if !out_rows.is_null() { unsafe { *out_rows = 24; } }
        false
    }
}

#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_set_raw_mode(enable: bool) -> bool {
    let mut saved = match SAVED_TERMIOS.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if enable {
        let mut orig = Termios::default();
        if unsafe { tcgetattr(0, &mut orig) } != 0 { return false; }
        *saved = Some(orig);
        let mut raw = orig;
        raw.c_lflag &= !(2 | 8 | 1);
        raw.c_iflag &= !0x0400;
        raw.c_cc[6] = 1;
        raw.c_cc[5] = 0;
        unsafe { tcsetattr(0, 0, &raw) == 0 }
    } else if let Some(orig) = saved.take() {
        unsafe { tcsetattr(0, 0, &orig) == 0 }
    } else { true }
}

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_set_raw_mode(enable: bool) -> bool {
    #[cfg(windows)]
    {
        super::windows::win_set_raw_mode(enable)
    }
    #[cfg(not(windows))]
    {
        let _ = enable;
        false
    }
}

static MOUSE_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_is_mouse_enabled() -> bool {
    MOUSE_ENABLED.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_set_mouse_mode(enable: bool) -> bool {
    MOUSE_ENABLED.store(enable, std::sync::atomic::Ordering::Relaxed);
    true
}

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_set_mouse_mode(enable: bool) -> bool {
    MOUSE_ENABLED.store(enable, std::sync::atomic::Ordering::Relaxed);
    #[cfg(windows)]
    {
        super::windows::win_set_mouse_mode(enable)
    }
    #[cfg(not(windows))]
    {
        let _ = enable;
        false
    }
}

#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_posix_isatty(fd: i32) -> bool { unsafe { isatty(fd) == 1 } }

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_posix_isatty(_fd: i32) -> bool { false }

#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_posix_pipe(read_fd: *mut i32, write_fd: *mut i32) -> i32 {
    let mut fds = [0i32; 2];
    let res = unsafe { pipe(fds.as_mut_ptr()) };
    if res == 0 {
        if !read_fd.is_null() { unsafe { *read_fd = fds[0]; } }
        if !write_fd.is_null() { unsafe { *write_fd = fds[1]; } }
    }
    res
}

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_posix_pipe(_read_fd: *mut i32, _write_fd: *mut i32) -> i32 { -1 }

#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_posix_dup(oldfd: i32) -> i32 { unsafe { dup(oldfd) } }

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_posix_dup(_oldfd: i32) -> i32 { -1 }

#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_posix_dup2(oldfd: i32, newfd: i32) -> i32 { unsafe { dup2(oldfd, newfd) } }

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_posix_dup2(_oldfd: i32, _newfd: i32) -> i32 { -1 }

#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_posix_fsync(fd: i32) -> i32 { unsafe { fsync(fd) } }

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_posix_fsync(_fd: i32) -> i32 { -1 }

#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_posix_fdatasync(fd: i32) -> i32 { unsafe { fdatasync(fd) } }

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_posix_fdatasync(_fd: i32) -> i32 { -1 }
