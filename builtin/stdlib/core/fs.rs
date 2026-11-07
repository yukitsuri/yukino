use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct YukinoString {
    pub ptr: *const u8,
    pub len: i64,
}

thread_local! {
    static LAST_ERROR: RefCell<(i32, String)> = const { RefCell::new((0, String::new())) };
    static FS_STR_RING: RefCell<(usize, Vec<Vec<u8>>)> = const { RefCell::new((0, Vec::new())) };
    static FS_ENTRIES_BUF: RefCell<Vec<YukinoString>> = const { RefCell::new(Vec::new()) };
    static FS_NAMES_POOL: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

static NEXT_FD: AtomicI32 = AtomicI32::new(10);
static OPEN_FILES: Mutex<Option<HashMap<i32, File>>> = Mutex::new(None);

fn set_last_error(code: i32, msg: String) {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = (code, msg);
    });
}

fn clear_last_error() {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = (0, String::new());
    });
}

fn record_io_error(e: std::io::Error) -> i32 {
    let code = match e.kind() {
        std::io::ErrorKind::NotFound => 1,
        std::io::ErrorKind::PermissionDenied => 2,
        std::io::ErrorKind::AlreadyExists => 3,
        std::io::ErrorKind::DirectoryNotEmpty => 5,
        std::io::ErrorKind::InvalidInput | std::io::ErrorKind::InvalidData => 7,
        std::io::ErrorKind::Unsupported => 8,
        _ => 6,
    };
    let msg = e.to_string();
    set_last_error(code, msg);
    code
}

fn set_fs_str_result(bytes: &[u8], out_len: *mut i64) -> *const u8 {
    FS_STR_RING.with(|cell| {
        let mut ring = cell.borrow_mut();
        if ring.1.len() < 256 {
            ring.1.resize_with(256, Vec::new);
        }
        let idx = ring.0 % 256;
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

unsafe fn path_from_raw<'a>(ptr: *const u8, len: i64) -> Option<&'a Path> {
    if ptr.is_null() || len < 0 {
        return None;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    std::str::from_utf8(slice).ok().map(Path::new)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_last_error_code() -> i32 {
    LAST_ERROR.with(|cell| cell.borrow().0)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_last_error_message(out_len: *mut i64) -> *const u8 {
    LAST_ERROR.with(|cell| {
        let err = cell.borrow();
        let bytes = err.1.as_bytes();
        set_fs_str_result(bytes, out_len)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_canonicalize(path_p: *const u8, path_len: i64, out_len: *mut i64) -> *const u8 {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => {
            set_last_error(7, "invalid path pointer or length".to_string());
            if !out_len.is_null() { unsafe { *out_len = -1; } }
            return std::ptr::null();
        }
    };
    match std::fs::canonicalize(p) {
        Ok(canon) => {
            let s = canon.to_string_lossy();
            set_fs_str_result(s.as_bytes(), out_len)
        }
        Err(e) => {
            record_io_error(e);
            if !out_len.is_null() { unsafe { *out_len = -1; } }
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_exists(path_p: *const u8, path_len: i64) -> bool {
    clear_last_error();
    match unsafe { path_from_raw(path_p, path_len) } {
        Some(p) => p.exists(),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_is_file(path_p: *const u8, path_len: i64) -> bool {
    clear_last_error();
    match unsafe { path_from_raw(path_p, path_len) } {
        Some(p) => p.is_file(),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_is_dir(path_p: *const u8, path_len: i64) -> bool {
    clear_last_error();
    match unsafe { path_from_raw(path_p, path_len) } {
        Some(p) => p.is_dir(),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_is_symlink(path_p: *const u8, path_len: i64) -> bool {
    clear_last_error();
    match unsafe { path_from_raw(path_p, path_len) } {
        Some(p) => p.is_symlink(),
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_read_text(path_p: *const u8, path_len: i64, out_len: *mut i64) -> *const u8 {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => {
            set_last_error(7, "invalid path".to_string());
            if !out_len.is_null() { unsafe { *out_len = -1; } }
            return std::ptr::null();
        }
    };
    match std::fs::read(p) {
        Ok(data) => set_fs_str_result(&data, out_len),
        Err(e) => {
            record_io_error(e);
            if !out_len.is_null() { unsafe { *out_len = -1; } }
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_write_text(path_p: *const u8, path_len: i64, data_p: *const u8, data_len: i64) -> bool {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => {
            set_last_error(7, "invalid path".to_string());
            return false;
        }
    };
    let data = if data_p.is_null() || data_len <= 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(data_p, data_len as usize) }
    };
    match std::fs::write(p, data) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_append_text(path_p: *const u8, path_len: i64, data_p: *const u8, data_len: i64) -> bool {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => {
            set_last_error(7, "invalid path".to_string());
            return false;
        }
    };
    let data = if data_p.is_null() || data_len <= 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(data_p, data_len as usize) }
    };
    let mut file = match OpenOptions::new().create(true).append(true).open(p) {
        Ok(f) => f,
        Err(e) => {
            record_io_error(e);
            return false;
        }
    };
    match file.write_all(data) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_read_bytes(path_p: *const u8, path_len: i64, buf: *mut u8, max_len: i64) -> i64 {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => {
            set_last_error(7, "invalid path".to_string());
            return -1;
        }
    };
    if buf.is_null() || max_len <= 0 {
        return 0;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(buf, max_len as usize) };
    let mut file = match File::open(p) {
        Ok(f) => f,
        Err(e) => {
            record_io_error(e);
            return -1;
        }
    };
    match file.read(slice) {
        Ok(n) => n as i64,
        Err(e) => {
            record_io_error(e);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_size(path_p: *const u8, path_len: i64) -> i64 {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => {
            set_last_error(7, "invalid path".to_string());
            return -1;
        }
    };
    match std::fs::metadata(p) {
        Ok(m) => m.len() as i64,
        Err(e) => {
            record_io_error(e);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_remove(path_p: *const u8, path_len: i64) -> bool {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => {
            set_last_error(7, "invalid path".to_string());
            return false;
        }
    };
    match std::fs::remove_file(p) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_rename(from_p: *const u8, from_len: i64, to_p: *const u8, to_len: i64) -> bool {
    clear_last_error();
    let from = match unsafe { path_from_raw(from_p, from_len) } {
        Some(p) => p,
        None => return false,
    };
    let to = match unsafe { path_from_raw(to_p, to_len) } {
        Some(p) => p,
        None => return false,
    };
    match std::fs::rename(from, to) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_copy(from_p: *const u8, from_len: i64, to_p: *const u8, to_len: i64) -> i64 {
    clear_last_error();
    let from = match unsafe { path_from_raw(from_p, from_len) } {
        Some(p) => p,
        None => return -1,
    };
    let to = match unsafe { path_from_raw(to_p, to_len) } {
        Some(p) => p,
        None => return -1,
    };
    match std::fs::copy(from, to) {
        Ok(bytes) => bytes as i64,
        Err(e) => {
            record_io_error(e);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_hard_link(orig_p: *const u8, orig_len: i64, link_p: *const u8, link_len: i64) -> bool {
    clear_last_error();
    let orig = match unsafe { path_from_raw(orig_p, orig_len) } {
        Some(p) => p,
        None => return false,
    };
    let link = match unsafe { path_from_raw(link_p, link_len) } {
        Some(p) => p,
        None => return false,
    };
    match std::fs::hard_link(orig, link) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_symlink(orig_p: *const u8, orig_len: i64, link_p: *const u8, link_len: i64) -> bool {
    clear_last_error();
    let orig = match unsafe { path_from_raw(orig_p, orig_len) } {
        Some(p) => p,
        None => return false,
    };
    let link = match unsafe { path_from_raw(link_p, link_len) } {
        Some(p) => p,
        None => return false,
    };
    #[cfg(unix)]
    let res = std::os::unix::fs::symlink(orig, link);
    #[cfg(windows)]
    let res = if orig.is_dir() {
        std::os::windows::fs::symlink_dir(orig, link)
    } else {
        std::os::windows::fs::symlink_file(orig, link)
    };
    match res {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_touch(path_p: *const u8, path_len: i64) -> bool {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => return false,
    };
    if p.exists() {
        match OpenOptions::new().write(true).open(p) {
            Ok(file) => {
                let _ = file.set_modified(std::time::SystemTime::now());
                true
            }
            Err(e) => {
                record_io_error(e);
                false
            }
        }
    } else {
        match File::create(p) {
            Ok(_) => true,
            Err(e) => {
                record_io_error(e);
                false
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_set_readonly(path_p: *const u8, path_len: i64, readonly: bool) -> bool {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => return false,
    };
    match std::fs::metadata(p) {
        Ok(m) => {
            let mut perms = m.permissions();
            perms.set_readonly(readonly);
            match std::fs::set_permissions(p, perms) {
                Ok(()) => true,
                Err(e) => {
                    record_io_error(e);
                    false
                }
            }
        }
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_set_permissions(path_p: *const u8, path_len: i64, mode: u32) -> bool {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => return false,
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(mode);
        match std::fs::set_permissions(p, perms) {
            Ok(()) => true,
            Err(e) => {
                record_io_error(e);
                false
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = mode;
        let _ = p;
        true
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_metadata(
    path_p: *const u8,
    path_len: i64,
    out_size: *mut i64,
    out_type: *mut i32,
    out_readonly: *mut bool,
    out_modified: *mut i64,
    out_created: *mut i64,
    out_accessed: *mut i64,
    out_mode: *mut u32,
) -> bool {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => return false,
    };
    let meta = match std::fs::symlink_metadata(p) {
        Ok(m) => m,
        Err(e) => {
            record_io_error(e);
            return false;
        }
    };

    if !out_size.is_null() {
        unsafe { *out_size = meta.len() as i64; }
    }
    if !out_type.is_null() {
        let ftype = if meta.is_symlink() {
            3
        } else if meta.is_dir() {
            2
        } else if meta.is_file() {
            1
        } else {
            0
        };
        unsafe { *out_type = ftype; }
    }
    if !out_readonly.is_null() {
        unsafe { *out_readonly = meta.permissions().readonly(); }
    }
    let to_nanos = |st: Result<std::time::SystemTime, std::io::Error>| -> i64 {
        match st {
            Ok(time) => time
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as i64)
                .unwrap_or(0),
            Err(_) => 0,
        }
    };
    if !out_modified.is_null() {
        unsafe { *out_modified = to_nanos(meta.modified()); }
    }
    if !out_created.is_null() {
        unsafe { *out_created = to_nanos(meta.created()); }
    }
    if !out_accessed.is_null() {
        unsafe { *out_accessed = to_nanos(meta.accessed()); }
    }
    if !out_mode.is_null() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            unsafe { *out_mode = meta.permissions().mode(); }
        }
        #[cfg(not(unix))]
        {
            unsafe { *out_mode = 0; }
        }
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_open(path_p: *const u8, path_len: i64, mode: i32) -> i32 {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => return -1,
    };
    let mut opts = OpenOptions::new();
    match mode {
        1 => { opts.read(true); }                     // Read
        2 => { opts.write(true).create(true).truncate(true); } // Write (truncate)
        3 => { opts.append(true).create(true); }      // Append
        4 => { opts.write(true).create_new(true); }   // Create new exclusive
        _ => {
            set_last_error(7, "unsupported open mode".to_string());
            return -1;
        }
    };
    let file = match opts.open(p) {
        Ok(f) => f,
        Err(e) => {
            record_io_error(e);
            return -1;
        }
    };
    let fd = NEXT_FD.fetch_add(1, Ordering::Relaxed);
    let mut guard = OPEN_FILES.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    map.insert(fd, file);
    fd
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_close(fd: i32) -> bool {
    let mut guard = OPEN_FILES.lock().unwrap();
    if let Some(map) = guard.as_mut() {
        map.remove(&fd).is_some()
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_read_fd(fd: i32, buf: *mut u8, max_len: i64) -> i64 {
    clear_last_error();
    if buf.is_null() || max_len <= 0 {
        return 0;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(buf, max_len as usize) };
    let mut guard = OPEN_FILES.lock().unwrap();
    if let Some(map) = guard.as_mut() {
        if let Some(file) = map.get_mut(&fd) {
            match file.read(slice) {
                Ok(n) => return n as i64,
                Err(e) => {
                    record_io_error(e);
                    return -1;
                }
            }
        }
    }
    set_last_error(6, "invalid file descriptor".to_string());
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_write_fd(fd: i32, buf: *const u8, len: i64) -> i64 {
    clear_last_error();
    if buf.is_null() || len <= 0 {
        return 0;
    }
    let slice = unsafe { std::slice::from_raw_parts(buf, len as usize) };
    let mut guard = OPEN_FILES.lock().unwrap();
    if let Some(map) = guard.as_mut() {
        if let Some(file) = map.get_mut(&fd) {
            match file.write(slice) {
                Ok(n) => return n as i64,
                Err(e) => {
                    record_io_error(e);
                    return -1;
                }
            }
        }
    }
    set_last_error(6, "invalid file descriptor".to_string());
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_flush_fd(fd: i32) -> bool {
    clear_last_error();
    let mut guard = OPEN_FILES.lock().unwrap();
    if let Some(map) = guard.as_mut() {
        if let Some(file) = map.get_mut(&fd) {
            match file.flush() {
                Ok(()) => return true,
                Err(e) => {
                    record_io_error(e);
                    return false;
                }
            }
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_seek_fd(fd: i32, offset: i64, whence: i32) -> i64 {
    clear_last_error();
    let pos = match whence {
        0 => SeekFrom::Start(offset as u64),
        1 => SeekFrom::Current(offset),
        2 => SeekFrom::End(offset),
        _ => {
            set_last_error(7, "invalid seek whence".to_string());
            return -1;
        }
    };
    let mut guard = OPEN_FILES.lock().unwrap();
    if let Some(map) = guard.as_mut() {
        if let Some(file) = map.get_mut(&fd) {
            match file.seek(pos) {
                Ok(n) => return n as i64,
                Err(e) => {
                    record_io_error(e);
                    return -1;
                }
            }
        }
    }
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_file_tell_fd(fd: i32) -> i64 {
    yukistd_fs_file_seek_fd(fd, 0, 1)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_create(path_p: *const u8, path_len: i64) -> bool {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => return false,
    };
    match std::fs::create_dir(p) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_create_all(path_p: *const u8, path_len: i64) -> bool {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => return false,
    };
    match std::fs::create_dir_all(p) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_remove(path_p: *const u8, path_len: i64) -> bool {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => return false,
    };
    match std::fs::remove_dir(p) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_remove_all(path_p: *const u8, path_len: i64) -> bool {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => return false,
    };
    match std::fs::remove_dir_all(p) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_current(out_len: *mut i64) -> *const u8 {
    clear_last_error();
    match std::env::current_dir() {
        Ok(dir) => {
            let s = dir.to_string_lossy();
            set_fs_str_result(s.as_bytes(), out_len)
        }
        Err(e) => {
            record_io_error(e);
            if !out_len.is_null() { unsafe { *out_len = -1; } }
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_set_current(path_p: *const u8, path_len: i64) -> bool {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => return false,
    };
    match std::env::set_current_dir(p) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_is_empty(path_p: *const u8, path_len: i64) -> bool {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => return false,
    };
    match std::fs::read_dir(p) {
        Ok(mut rd) => rd.next().is_none(),
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_list(path_p: *const u8, path_len: i64, out_count: *mut i64) -> *const YukinoString {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => {
            if !out_count.is_null() { unsafe { *out_count = -1; } }
            return std::ptr::null();
        }
    };
    let read_dir = match std::fs::read_dir(p) {
        Ok(rd) => rd,
        Err(e) => {
            record_io_error(e);
            if !out_count.is_null() { unsafe { *out_count = -1; } }
            return std::ptr::null();
        }
    };

    FS_NAMES_POOL.with(|pool_cell| {
        FS_ENTRIES_BUF.with(|entries_cell| {
            let mut pool = pool_cell.borrow_mut();
            let mut entries = entries_cell.borrow_mut();
            pool.clear();
            entries.clear();

            for entry_res in read_dir {
                if let Ok(entry) = entry_res {
                    let name = entry.file_name();
                    let bytes = name.as_encoded_bytes();
                    let start_idx = pool.len();
                    pool.extend_from_slice(bytes);
                    let len = bytes.len() as i64;
                    let ptr = unsafe { pool.as_ptr().add(start_idx) };
                    entries.push(YukinoString { ptr, len });
                }
            }

            let base_ptr = pool.as_ptr();
            let mut curr_offset = 0;
            for item in entries.iter_mut() {
                item.ptr = unsafe { base_ptr.add(curr_offset) };
                curr_offset += item.len as usize;
            }

            if !out_count.is_null() {
                unsafe { *out_count = entries.len() as i64; }
            }
            entries.as_ptr()
        })
    })
}

fn walk_dir_recursive(dir: &Path, list: &mut Vec<PathBuf>) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry_res in rd {
            if let Ok(entry) = entry_res {
                let p = entry.path();
                list.push(p.clone());
                if p.is_dir() {
                    walk_dir_recursive(&p, list);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_walk(path_p: *const u8, path_len: i64, out_count: *mut i64) -> *const YukinoString {
    clear_last_error();
    let p = match unsafe { path_from_raw(path_p, path_len) } {
        Some(path) => path,
        None => {
            if !out_count.is_null() { unsafe { *out_count = -1; } }
            return std::ptr::null();
        }
    };

    let mut paths = Vec::new();
    walk_dir_recursive(p, &mut paths);

    FS_NAMES_POOL.with(|pool_cell| {
        FS_ENTRIES_BUF.with(|entries_cell| {
            let mut pool = pool_cell.borrow_mut();
            let mut entries = entries_cell.borrow_mut();
            pool.clear();
            entries.clear();

            for path in &paths {
                let s = path.to_string_lossy();
                let bytes = s.as_bytes();
                let start_idx = pool.len();
                pool.extend_from_slice(bytes);
                let len = bytes.len() as i64;
                let ptr = unsafe { pool.as_ptr().add(start_idx) };
                entries.push(YukinoString { ptr, len });
            }

            let base_ptr = pool.as_ptr();
            let mut curr_offset = 0;
            for item in entries.iter_mut() {
                item.ptr = unsafe { base_ptr.add(curr_offset) };
                curr_offset += item.len as usize;
            }

            if !out_count.is_null() {
                unsafe { *out_count = entries.len() as i64; }
            }
            entries.as_ptr()
        })
    })
}

fn copy_dir_all_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_child = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all_recursive(&entry.path(), &dest_child)?;
        } else {
            std::fs::copy(entry.path(), dest_child)?;
        }
    }
    Ok(())
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_copy_all(from_p: *const u8, from_len: i64, to_p: *const u8, to_len: i64) -> bool {
    clear_last_error();
    let from = match unsafe { path_from_raw(from_p, from_len) } {
        Some(p) => p,
        None => return false,
    };
    let to = match unsafe { path_from_raw(to_p, to_len) } {
        Some(p) => p,
        None => return false,
    };
    match copy_dir_all_recursive(from, to) {
        Ok(()) => true,
        Err(e) => {
            record_io_error(e);
            false
        }
    }
}

fn get_home_dir() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() {
                return Some(PathBuf::from(home));
            }
        }
        None
    }
    #[cfg(windows)]
    {
        if let Ok(prof) = std::env::var("USERPROFILE") {
            if !prof.is_empty() {
                return Some(PathBuf::from(prof));
            }
        }
        if let (Ok(drive), Ok(path)) = (std::env::var("HOMEDRIVE"), std::env::var("HOMEPATH")) {
            let combined = format!("{drive}{path}");
            if !combined.is_empty() {
                return Some(PathBuf::from(combined));
            }
        }
        None
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_temp(out_len: *mut i64) -> *const u8 {
    clear_last_error();
    let p = std::env::temp_dir();
    let s = p.to_string_lossy();
    set_fs_str_result(s.as_bytes(), out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_home(out_len: *mut i64) -> *const u8 {
    clear_last_error();
    match get_home_dir() {
        Some(p) => {
            let s = p.to_string_lossy();
            set_fs_str_result(s.as_bytes(), out_len)
        }
        None => {
            if !out_len.is_null() { unsafe { *out_len = 0; } }
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_config(out_len: *mut i64) -> *const u8 {
    clear_last_error();
    #[cfg(unix)]
    {
        if let Ok(val) = std::env::var("XDG_CONFIG_HOME") {
            if !val.is_empty() {
                return set_fs_str_result(val.as_bytes(), out_len);
            }
        }
        if let Some(mut home) = get_home_dir() {
            home.push(".config");
            let s = home.to_string_lossy();
            return set_fs_str_result(s.as_bytes(), out_len);
        }
    }
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.is_empty() {
                return set_fs_str_result(appdata.as_bytes(), out_len);
            }
        }
    }
    let p = std::env::temp_dir();
    let s = p.to_string_lossy();
    set_fs_str_result(s.as_bytes(), out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_data(out_len: *mut i64) -> *const u8 {
    clear_last_error();
    #[cfg(unix)]
    {
        if let Ok(val) = std::env::var("XDG_DATA_HOME") {
            if !val.is_empty() {
                return set_fs_str_result(val.as_bytes(), out_len);
            }
        }
        if let Some(mut home) = get_home_dir() {
            home.push(".local");
            home.push("share");
            let s = home.to_string_lossy();
            return set_fs_str_result(s.as_bytes(), out_len);
        }
    }
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
            if !appdata.is_empty() {
                return set_fs_str_result(appdata.as_bytes(), out_len);
            }
        }
    }
    let p = std::env::temp_dir();
    let s = p.to_string_lossy();
    set_fs_str_result(s.as_bytes(), out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_data_local(out_len: *mut i64) -> *const u8 {
    yukistd_fs_dir_data(out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_cache(out_len: *mut i64) -> *const u8 {
    clear_last_error();
    #[cfg(unix)]
    {
        if let Ok(val) = std::env::var("XDG_CACHE_HOME") {
            if !val.is_empty() {
                return set_fs_str_result(val.as_bytes(), out_len);
            }
        }
        if let Some(mut home) = get_home_dir() {
            home.push(".cache");
            let s = home.to_string_lossy();
            return set_fs_str_result(s.as_bytes(), out_len);
        }
    }
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
            if !appdata.is_empty() {
                let mut path = PathBuf::from(appdata);
                path.push("cache");
                let s = path.to_string_lossy();
                return set_fs_str_result(s.as_bytes(), out_len);
            }
        }
    }
    let p = std::env::temp_dir();
    let s = p.to_string_lossy();
    set_fs_str_result(s.as_bytes(), out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_state(out_len: *mut i64) -> *const u8 {
    clear_last_error();
    #[cfg(unix)]
    {
        if let Ok(val) = std::env::var("XDG_STATE_HOME") {
            if !val.is_empty() {
                return set_fs_str_result(val.as_bytes(), out_len);
            }
        }
        if let Some(mut home) = get_home_dir() {
            home.push(".local");
            home.push("state");
            let s = home.to_string_lossy();
            return set_fs_str_result(s.as_bytes(), out_len);
        }
    }
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
            if !appdata.is_empty() {
                return set_fs_str_result(appdata.as_bytes(), out_len);
            }
        }
    }
    let p = std::env::temp_dir();
    let s = p.to_string_lossy();
    set_fs_str_result(s.as_bytes(), out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_runtime(out_len: *mut i64) -> *const u8 {
    clear_last_error();
    #[cfg(unix)]
    {
        if let Ok(val) = std::env::var("XDG_RUNTIME_DIR") {
            if !val.is_empty() {
                return set_fs_str_result(val.as_bytes(), out_len);
            }
        }
    }
    let p = std::env::temp_dir();
    let s = p.to_string_lossy();
    set_fs_str_result(s.as_bytes(), out_len)
}

fn user_subdir(sub: &str, out_len: *mut i64) -> *const u8 {
    if let Some(mut home) = get_home_dir() {
        home.push(sub);
        let s = home.to_string_lossy();
        set_fs_str_result(s.as_bytes(), out_len)
    } else {
        if !out_len.is_null() { unsafe { *out_len = 0; } }
        std::ptr::null()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_desktop(out_len: *mut i64) -> *const u8 {
    user_subdir("Desktop", out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_documents(out_len: *mut i64) -> *const u8 {
    user_subdir("Documents", out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_downloads(out_len: *mut i64) -> *const u8 {
    user_subdir("Downloads", out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_music(out_len: *mut i64) -> *const u8 {
    user_subdir("Music", out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_pictures(out_len: *mut i64) -> *const u8 {
    user_subdir("Pictures", out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_dir_videos(out_len: *mut i64) -> *const u8 {
    user_subdir("Videos", out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_fs_exe_path(out_len: *mut i64) -> *const u8 {
    clear_last_error();
    match std::env::current_exe() {
        Ok(exe) => {
            let s = exe.to_string_lossy();
            set_fs_str_result(s.as_bytes(), out_len)
        }
        Err(e) => {
            record_io_error(e);
            if !out_len.is_null() { unsafe { *out_len = -1; } }
            std::ptr::null()
        }
    }
}
