use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use libsqlite3_sys as ffi;

thread_local! {
    static SQLITE_STR_RING: RefCell<(usize, Vec<Vec<u8>>)> = const { RefCell::new((0, Vec::new())) };
}

fn set_str_result(bytes: &[u8], out_len: *mut i64) -> *const u8 {
    SQLITE_STR_RING.with(|cell| {
        let mut ring = cell.borrow_mut();
        if ring.1.len() < 128 {
            ring.1.resize_with(128, Vec::new);
        }
        let idx = ring.0 % 128;
        ring.0 = ring.0.wrapping_add(1);
        let buf = &mut ring.1[idx];
        buf.clear();
        buf.extend_from_slice(bytes);
        if !out_len.is_null() {
            unsafe {
                *out_len = buf.len() as i64;
            }
        }
        buf.as_ptr()
    })
}

unsafe fn str_to_cstring(ptr: *const u8, len: i64) -> Option<CString> {
    if ptr.is_null() || len < 0 {
        return None;
    }
    if len == 0 {
        return CString::new("").ok();
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    CString::new(slice).ok()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_open(
    path_p: *const u8,
    path_len: i64,
    flags: i32,
    out_db: *mut *mut c_void,
) -> i32 {
    if out_db.is_null() {
        return ffi::SQLITE_MISUSE;
    }
    let c_path = match unsafe { str_to_cstring(path_p, path_len) } {
        Some(s) => s,
        None => return ffi::SQLITE_MISUSE,
    };

    let actual_flags = if flags == 0 {
        ffi::SQLITE_OPEN_READWRITE | ffi::SQLITE_OPEN_CREATE
    } else {
        flags as c_int
    };

    let mut db: *mut ffi::sqlite3 = ptr::null_mut();
    let rc = unsafe {
        ffi::sqlite3_open_v2(
            c_path.as_ptr(),
            &mut db,
            actual_flags,
            ptr::null(),
        )
    };

    unsafe {
        *out_db = db as *mut c_void;
    }
    rc
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_close(db: *mut c_void) -> i32 {
    if db.is_null() {
        return ffi::SQLITE_OK;
    }
    unsafe { ffi::sqlite3_close(db as *mut ffi::sqlite3) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_prepare(
    db: *mut c_void,
    sql_p: *const u8,
    sql_len: i64,
    out_stmt: *mut *mut c_void,
) -> i32 {
    if db.is_null() || out_stmt.is_null() || sql_p.is_null() || sql_len < 0 {
        return ffi::SQLITE_MISUSE;
    }
    let mut stmt: *mut ffi::sqlite3_stmt = ptr::null_mut();
    let rc = unsafe {
        ffi::sqlite3_prepare_v2(
            db as *mut ffi::sqlite3,
            sql_p as *const c_char,
            sql_len as c_int,
            &mut stmt,
            ptr::null_mut(),
        )
    };
    unsafe {
        *out_stmt = stmt as *mut c_void;
    }
    rc
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_step(stmt: *mut c_void) -> i32 {
    if stmt.is_null() {
        return ffi::SQLITE_MISUSE;
    }
    unsafe { ffi::sqlite3_step(stmt as *mut ffi::sqlite3_stmt) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_reset(stmt: *mut c_void) -> i32 {
    if stmt.is_null() {
        return ffi::SQLITE_MISUSE;
    }
    unsafe { ffi::sqlite3_reset(stmt as *mut ffi::sqlite3_stmt) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_finalize(stmt: *mut c_void) -> i32 {
    if stmt.is_null() {
        return ffi::SQLITE_OK;
    }
    unsafe { ffi::sqlite3_finalize(stmt as *mut ffi::sqlite3_stmt) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_bind_null(stmt: *mut c_void, index: i32) -> i32 {
    if stmt.is_null() {
        return ffi::SQLITE_MISUSE;
    }
    unsafe { ffi::sqlite3_bind_null(stmt as *mut ffi::sqlite3_stmt, index as c_int) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_bind_int64(
    stmt: *mut c_void,
    index: i32,
    val: i64,
) -> i32 {
    if stmt.is_null() {
        return ffi::SQLITE_MISUSE;
    }
    unsafe { ffi::sqlite3_bind_int64(stmt as *mut ffi::sqlite3_stmt, index as c_int, val) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_bind_double(
    stmt: *mut c_void,
    index: i32,
    val: f64,
) -> i32 {
    if stmt.is_null() {
        return ffi::SQLITE_MISUSE;
    }
    unsafe { ffi::sqlite3_bind_double(stmt as *mut ffi::sqlite3_stmt, index as c_int, val) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_bind_text(
    stmt: *mut c_void,
    index: i32,
    text_p: *const u8,
    text_len: i64,
) -> i32 {
    if stmt.is_null() {
        return ffi::SQLITE_MISUSE;
    }
    if text_p.is_null() || text_len < 0 {
        return unsafe { ffi::sqlite3_bind_null(stmt as *mut ffi::sqlite3_stmt, index as c_int) };
    }
    unsafe {
        ffi::sqlite3_bind_text(
            stmt as *mut ffi::sqlite3_stmt,
            index as c_int,
            text_p as *const c_char,
            text_len as c_int,
            ffi::SQLITE_TRANSIENT(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_bind_blob(
    stmt: *mut c_void,
    index: i32,
    blob_p: *const u8,
    blob_len: i64,
) -> i32 {
    if stmt.is_null() {
        return ffi::SQLITE_MISUSE;
    }
    if blob_p.is_null() || blob_len < 0 {
        return unsafe { ffi::sqlite3_bind_null(stmt as *mut ffi::sqlite3_stmt, index as c_int) };
    }
    unsafe {
        ffi::sqlite3_bind_blob(
            stmt as *mut ffi::sqlite3_stmt,
            index as c_int,
            blob_p as *const c_void,
            blob_len as c_int,
            ffi::SQLITE_TRANSIENT(),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_column_count(stmt: *mut c_void) -> i32 {
    if stmt.is_null() {
        return 0;
    }
    unsafe { ffi::sqlite3_column_count(stmt as *mut ffi::sqlite3_stmt) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_column_name(
    stmt: *mut c_void,
    col: i32,
    out_len: *mut i64,
) -> *const u8 {
    if stmt.is_null() {
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        return ptr::null();
    }
    let name_ptr = unsafe { ffi::sqlite3_column_name(stmt as *mut ffi::sqlite3_stmt, col as c_int) };
    if name_ptr.is_null() {
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        return ptr::null();
    }
    let c_str = unsafe { CStr::from_ptr(name_ptr) };
    set_str_result(c_str.to_bytes(), out_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_column_type(stmt: *mut c_void, col: i32) -> i32 {
    if stmt.is_null() {
        return ffi::SQLITE_NULL;
    }
    unsafe { ffi::sqlite3_column_type(stmt as *mut ffi::sqlite3_stmt, col as c_int) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_column_int64(stmt: *mut c_void, col: i32) -> i64 {
    if stmt.is_null() {
        return 0;
    }
    unsafe { ffi::sqlite3_column_int64(stmt as *mut ffi::sqlite3_stmt, col as c_int) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_column_double(stmt: *mut c_void, col: i32) -> f64 {
    if stmt.is_null() {
        return 0.0;
    }
    unsafe { ffi::sqlite3_column_double(stmt as *mut ffi::sqlite3_stmt, col as c_int) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_column_text(
    stmt: *mut c_void,
    col: i32,
    out_len: *mut i64,
) -> *const u8 {
    if stmt.is_null() {
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        return ptr::null();
    }
    let p = unsafe { ffi::sqlite3_column_text(stmt as *mut ffi::sqlite3_stmt, col as c_int) };
    if p.is_null() {
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        return ptr::null();
    }
    let bytes_count = unsafe { ffi::sqlite3_column_bytes(stmt as *mut ffi::sqlite3_stmt, col as c_int) } as usize;
    let slice = unsafe { std::slice::from_raw_parts(p, bytes_count) };
    set_str_result(slice, out_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_column_blob(
    stmt: *mut c_void,
    col: i32,
    out_len: *mut i64,
) -> *const u8 {
    if stmt.is_null() {
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        return ptr::null();
    }
    let p = unsafe { ffi::sqlite3_column_blob(stmt as *mut ffi::sqlite3_stmt, col as c_int) };
    if p.is_null() {
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        return ptr::null();
    }
    let bytes_count = unsafe { ffi::sqlite3_column_bytes(stmt as *mut ffi::sqlite3_stmt, col as c_int) } as usize;
    let slice = unsafe { std::slice::from_raw_parts(p as *const u8, bytes_count) };
    set_str_result(slice, out_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_changes(db: *mut c_void) -> i64 {
    if db.is_null() {
        return 0;
    }
    unsafe { ffi::sqlite3_changes64(db as *mut ffi::sqlite3) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_last_insert_rowid(db: *mut c_void) -> i64 {
    if db.is_null() {
        return 0;
    }
    unsafe { ffi::sqlite3_last_insert_rowid(db as *mut ffi::sqlite3) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_errmsg(db: *mut c_void, out_len: *mut i64) -> *const u8 {
    if db.is_null() {
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        return ptr::null();
    }
    let msg_ptr = unsafe { ffi::sqlite3_errmsg(db as *mut ffi::sqlite3) };
    if msg_ptr.is_null() {
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        return ptr::null();
    }
    let c_str = unsafe { CStr::from_ptr(msg_ptr) };
    set_str_result(c_str.to_bytes(), out_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_errcode(db: *mut c_void) -> i32 {
    if db.is_null() {
        return ffi::SQLITE_MISUSE;
    }
    unsafe { ffi::sqlite3_errcode(db as *mut ffi::sqlite3) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukistd_sqlite_version(out_len: *mut i64) -> *const u8 {
    let ver_ptr = unsafe { ffi::sqlite3_libversion() };
    if ver_ptr.is_null() {
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        return ptr::null();
    }
    let c_str = unsafe { CStr::from_ptr(ver_ptr) };
    set_str_result(c_str.to_bytes(), out_len)
}
