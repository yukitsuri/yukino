#[cfg(windows)]
unsafe extern "system" {
    fn GetStdHandle(nStdHandle: i32) -> *mut std::ffi::c_void;
    fn GetConsoleMode(hConsoleHandle: *mut std::ffi::c_void, lpMode: *mut u32) -> i32;
    fn SetConsoleMode(hConsoleHandle: *mut std::ffi::c_void, dwMode: u32) -> i32;
    fn CreatePipe(
        hReadPipe: *mut *mut std::ffi::c_void,
        hWritePipe: *mut *mut std::ffi::c_void,
        lpPipeAttributes: *mut std::ffi::c_void,
        nSize: u32,
    ) -> i32;
    fn GetConsoleScreenBufferInfo(
        hConsoleOutput: *mut std::ffi::c_void,
        lpConsoleScreenBufferInfo: *mut ConsoleScreenBufferInfo,
    ) -> i32;
    fn WaitForSingleObject(hHandle: *mut std::ffi::c_void, dwMilliseconds: u32) -> u32;
    fn ReadFile(
        hFile: *mut std::ffi::c_void,
        lpBuffer: *mut u8,
        nNumberOfBytesToRead: u32,
        lpNumberOfBytesRead: *mut u32,
        lpOverlapped: *mut std::ffi::c_void,
    ) -> i32;
}

#[cfg(windows)]
#[repr(C)]
struct Coord {
    x: i16,
    y: i16,
}

#[cfg(windows)]
#[repr(C)]
struct SmallRect {
    left: i16,
    top: i16,
    right: i16,
    bottom: i16,
}

#[cfg(windows)]
#[repr(C)]
struct ConsoleScreenBufferInfo {
    dw_size: Coord,
    dw_cursor_position: Coord,
    w_attributes: u16,
    sr_window: SmallRect,
    dw_maximum_window_size: Coord,
}

#[cfg(windows)]
static SAVED_WIN_MODES: std::sync::Mutex<Option<(u32, u32)>> = std::sync::Mutex::new(None);

#[cfg(windows)]
pub fn win_set_raw_mode(enable: bool) -> bool {
    unsafe {
        let h_in = GetStdHandle(-10);
        let h_out = GetStdHandle(-11);
        if h_in.is_null() || h_out.is_null() {
            return false;
        }

        let mut in_mode: u32 = 0;
        let mut out_mode: u32 = 0;
        if GetConsoleMode(h_in, &mut in_mode) == 0 || GetConsoleMode(h_out, &mut out_mode) == 0 {
            return false;
        }

        let mut saved = match SAVED_WIN_MODES.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if enable {
            *saved = Some((in_mode, out_mode));

            const ENABLE_PROCESSED_INPUT: u32 = 0x0001;
            const ENABLE_LINE_INPUT: u32 = 0x0002;
            const ENABLE_ECHO_INPUT: u32 = 0x0004;
            const ENABLE_VIRTUAL_TERMINAL_INPUT: u32 = 0x0200;

            let raw_in = (in_mode & !(ENABLE_PROCESSED_INPUT | ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT))
                | ENABLE_VIRTUAL_TERMINAL_INPUT;

            const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
            const DISABLE_NEWLINE_AUTO_RETURN: u32 = 0x0008;

            let raw_out = out_mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING | DISABLE_NEWLINE_AUTO_RETURN;

            SetConsoleMode(h_in, raw_in) != 0 && SetConsoleMode(h_out, raw_out) != 0
        } else if let Some((orig_in, orig_out)) = saved.take() {
            SetConsoleMode(h_in, orig_in) != 0 && SetConsoleMode(h_out, orig_out) != 0
        } else {
            true
        }
    }
}

#[cfg(not(windows))]
pub fn win_set_raw_mode(_enable: bool) -> bool { false }

#[cfg(windows)]
static SAVED_WIN_MOUSE_MODE: std::sync::Mutex<Option<u32>> = std::sync::Mutex::new(None);

#[cfg(windows)]
pub fn win_set_mouse_mode(enable: bool) -> bool {
    unsafe {
        let h_in = GetStdHandle(-10);
        if h_in.is_null() {
            return false;
        }

        let mut in_mode: u32 = 0;
        if GetConsoleMode(h_in, &mut in_mode) == 0 {
            return false;
        }

        let mut saved = match SAVED_WIN_MOUSE_MODE.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        const ENABLE_MOUSE_INPUT: u32 = 0x0010;
        const ENABLE_QUICK_EDIT_MODE: u32 = 0x0040;
        const ENABLE_EXTENDED_FLAGS: u32 = 0x0080;

        if enable {
            *saved = Some(in_mode);
            let mouse_in = (in_mode & !ENABLE_QUICK_EDIT_MODE) | ENABLE_MOUSE_INPUT | ENABLE_EXTENDED_FLAGS;
            SetConsoleMode(h_in, mouse_in) != 0
        } else if let Some(orig_in) = saved.take() {
            SetConsoleMode(h_in, orig_in | ENABLE_EXTENDED_FLAGS) != 0
        } else {
            true
        }
    }
}

#[cfg(not(windows))]
pub fn win_set_mouse_mode(_enable: bool) -> bool { false }

#[cfg(windows)]
pub fn win_terminal_size(out_cols: *mut u32, out_rows: *mut u32) -> bool {
    unsafe {
        let h_out = GetStdHandle(-11);
        if !h_out.is_null() {
            let mut csbi = std::mem::zeroed::<ConsoleScreenBufferInfo>();
            if GetConsoleScreenBufferInfo(h_out, &mut csbi) != 0 {
                let cols = (csbi.sr_window.right - csbi.sr_window.left + 1) as u32;
                let rows = (csbi.sr_window.bottom - csbi.sr_window.top + 1) as u32;
                if cols > 0 && rows > 0 {
                    if !out_cols.is_null() { *out_cols = cols; }
                    if !out_rows.is_null() { *out_rows = rows; }
                    return true;
                }
            }
        }
        if !out_cols.is_null() { *out_cols = 80; }
        if !out_rows.is_null() { *out_rows = 24; }
        false
    }
}

#[cfg(not(windows))]
pub fn win_terminal_size(_out_cols: *mut u32, _out_rows: *mut u32) -> bool { false }

#[cfg(windows)]
pub fn win_poll_stdin(timeout_ms: i64) -> bool {
    unsafe {
        let h_in = GetStdHandle(-10);
        if h_in.is_null() { return false; }
        let ms = if timeout_ms < 0 { 0xFFFFFFFF } else { timeout_ms as u32 };
        WaitForSingleObject(h_in, ms) == 0
    }
}

#[cfg(not(windows))]
pub fn win_poll_stdin(_timeout_ms: i64) -> bool { false }

#[cfg(windows)]
pub fn win_read_stdin_byte() -> i32 {
    unsafe {
        let h_in = GetStdHandle(-10);
        if h_in.is_null() { return -1; }
        let mut b = 0u8;
        let mut read_bytes = 0u32;
        let res = ReadFile(h_in, &mut b, 1, &mut read_bytes, std::ptr::null_mut());
        if res != 0 && read_bytes == 1 {
            b as i32
        } else {
            -1
        }
    }
}

#[cfg(not(windows))]
pub fn win_read_stdin_byte() -> i32 { -1 }

#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_win_get_std_handle(n: i32) -> i64 {
    unsafe {
        let h = GetStdHandle(n);
        h as i64
    }
}

#[cfg(not(windows))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_win_get_std_handle(_n: i32) -> i64 { 0 }

#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_win_is_console(handle: i64) -> bool {
    let mut mode: u32 = 0;
    unsafe { GetConsoleMode(handle as *mut _, &mut mode) != 0 }
}

#[cfg(not(windows))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_win_is_console(_handle: i64) -> bool { false }

#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_win_get_console_mode(handle: i64) -> i32 {
    let mut mode: u32 = 0;
    let res = unsafe { GetConsoleMode(handle as *mut _, &mut mode) };
    if res != 0 { mode as i32} else { -1 }
}

#[cfg(not(windows))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_win_get_console_mode(_handle: i64) -> i32 { -1 }

#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_win_set_console_mode(handle: i64, mode: i32) -> bool {
    unsafe { SetConsoleMode(handle as *mut _, mode as u32) != 0 }
}

#[cfg(not(windows))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_win_set_console_mode(_handle: i64, _mode: i32) -> bool { false }

#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_win_create_pipe(read_h: *mut i64, write_h: *mut i64) -> bool {
    let mut r: *mut std::ffi::c_void = std::ptr::null_mut();
    let mut w: *mut std::ffi::c_void = std::ptr::null_mut();
    let res = unsafe { CreatePipe(&mut r, &mut w, std::ptr::null_mut(), 0) };
    if res != 0 {
        if !read_h.is_null() { unsafe { *read_h = r as i64; } }
        if !write_h.is_null() { unsafe { *write_h = w as i64; } }
        true
    } else { false }
}

#[cfg(not(windows))]
#[unsafe(no_mangle)]
pub extern "C" fn yukistd_io_win_create_pipe(_read_h: *mut i64, _write_h: *mut i64) -> bool { false }
