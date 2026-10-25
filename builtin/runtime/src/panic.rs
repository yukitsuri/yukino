use crate::rt_cast::YukinoString;

pub type UnwindReasonCode = i32;
pub type UnwindAction = i32;

#[repr(C)]
pub struct UnwindException {
    pub exception_class: u64,
    pub exception_cleanup: Option<unsafe extern "C" fn(UnwindReasonCode, *mut UnwindException)>,
    pub private_1: usize,
    pub private_2: usize,
}

pub const YUKINO_EXCEPTION_CLASS: u64 = u64::from_ne_bytes(*b"YUKINO\0\0");

#[repr(C)]
pub struct YukinoException {
    pub unwind_header: UnwindException,
    pub message_ptr: *const u8,
    pub message_len: usize,
    pub kind_ptr: *const u8,
    pub kind_len: usize,
    pub has_code: i32,
    pub code: i32,
    pub payload_ptr: *mut u8,
    pub payload_len: usize,
}

thread_local! { static IN_PANIC: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

#[cfg(not(all(windows, target_env = "msvc")))]
unsafe extern "C" {
    fn _Unwind_RaiseException(exception_object: *mut UnwindException) -> UnwindReasonCode;
}

unsafe extern "C" fn yukino_exception_cleanup(
    _reason: UnwindReasonCode,
    exc: *mut UnwindException,
) {
    if !exc.is_null() {
        let yukino_exc = exc.cast::<YukinoException>();
        let payload_ptr = unsafe { (*yukino_exc).payload_ptr };
        let payload_len = unsafe { (*yukino_exc).payload_len };
        if !payload_ptr.is_null() && payload_len > 0 {
            if let Ok(layout) = std::alloc::Layout::from_size_align(payload_len, 8) {
                unsafe { std::alloc::dealloc(payload_ptr, layout); }
            }
        }
        drop(unsafe { Box::from_raw(yukino_exc) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_exception_get_message(
    exc: *const UnwindException,
) -> YukinoString {
    if exc.is_null() {
        return YukinoString {
            ptr: std::ptr::null(),
            len: 0,
        };
    }
    unsafe {
        if (*exc).exception_class == YUKINO_EXCEPTION_CLASS {
            let yukino_exc = exc.cast::<YukinoException>();
            YukinoString {
                ptr: (*yukino_exc).message_ptr,
                len: (*yukino_exc).message_len,
            }
        } else {
            YukinoString {
                ptr: std::ptr::null(),
                len: 0,
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_exception_get_kind(
    exc: *const UnwindException,
) -> YukinoString {
    if exc.is_null() {
        return YukinoString {
            ptr: std::ptr::null(),
            len: 0,
        };
    }
    unsafe {
        if (*exc).exception_class == YUKINO_EXCEPTION_CLASS {
            let yukino_exc = exc.cast::<YukinoException>();
            YukinoString {
                ptr: (*yukino_exc).kind_ptr,
                len: (*yukino_exc).kind_len,
            }
        } else {
            YukinoString {
                ptr: std::ptr::null(),
                len: 0,
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_exception_has_code(
    exc: *const UnwindException,
) -> i32 {
    if exc.is_null() {
        return 0;
    }
    unsafe {
        if (*exc).exception_class == YUKINO_EXCEPTION_CLASS {
            let yukino_exc = exc.cast::<YukinoException>();
            (*yukino_exc).has_code
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_exception_get_code(
    exc: *const UnwindException,
) -> i32 {
    if exc.is_null() {
        return 0;
    }
    unsafe {
        if (*exc).exception_class == YUKINO_EXCEPTION_CLASS {
            let yukino_exc = exc.cast::<YukinoException>();
            (*yukino_exc).code
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_exception_get_payload(
    exc: *const UnwindException,
) -> *const u8 {
    if exc.is_null() {
        return std::ptr::null();
    }
    unsafe {
        if (*exc).exception_class == YUKINO_EXCEPTION_CLASS {
            let yukino_exc = exc.cast::<YukinoException>();
            (*yukino_exc).payload_ptr
        } else {
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_panic_handled() {
    IN_PANIC.with(|p| p.set(false));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_raise_panic(msg_ptr: *const u8, msg_len: usize) -> ! {
    let default_kind = b"Panic";
    unsafe {
        yukino_raise_typed_panic(
            msg_ptr,
            msg_len,
            default_kind.as_ptr(),
            default_kind.len(),
            0,
            0,
            std::ptr::null(),
            0,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_raise_typed_panic(
    msg_ptr: *const u8,
    msg_len: usize,
    kind_ptr: *const u8,
    kind_len: usize,
    has_code: i32,
    code: i32,
    payload_ptr: *const u8,
    payload_len: usize,
) -> ! {
    let msg_str = if !msg_ptr.is_null() && msg_len > 0 {
        let slice = unsafe { std::slice::from_raw_parts(msg_ptr, msg_len) };
        std::str::from_utf8(slice).unwrap_or("<invalid utf8 panic message>")
    } else {
        "<empty panic message>"
    };

    let kind_str = if !kind_ptr.is_null() && kind_len > 0 {
        let slice = unsafe { std::slice::from_raw_parts(kind_ptr, kind_len) };
        std::str::from_utf8(slice).unwrap_or("Panic")
    } else {
        "Panic"
    };

    let already_panicking = IN_PANIC.with(|p| p.get());
    if already_panicking {
        eprintln!("fatal runtime error: panic during unwinding: [{kind_str}] {msg_str}");
        std::process::abort();
    }
    IN_PANIC.with(|p| p.set(true));

    let (copied_payload_ptr, copied_payload_len) = if !payload_ptr.is_null() && payload_len > 0 {
        if let Ok(layout) = std::alloc::Layout::from_size_align(payload_len, 8) {
            let dest = unsafe { std::alloc::alloc(layout) };
            if !dest.is_null() {
                unsafe { std::ptr::copy_nonoverlapping(payload_ptr, dest, payload_len); }
                (dest, payload_len)
            } else {
                (std::ptr::null_mut(), 0)
            }
        } else {
            (std::ptr::null_mut(), 0)
        }
    } else {
        (std::ptr::null_mut(), 0)
    };

    #[cfg(not(all(windows, target_env = "msvc")))]
    {
        let boxed_exc = Box::new(YukinoException {
            unwind_header: UnwindException {
                exception_class: YUKINO_EXCEPTION_CLASS,
                exception_cleanup: Some(yukino_exception_cleanup),
                private_1: 0,
                private_2: 0,
            },
            message_ptr: msg_ptr,
            message_len: msg_len,
            kind_ptr,
            kind_len,
            has_code,
            code,
            payload_ptr: copied_payload_ptr,
            payload_len: copied_payload_len,
        });

        let raw_exc = Box::into_raw(boxed_exc);
        unsafe {
            let reason = _Unwind_RaiseException(&raw mut (*raw_exc).unwind_header);
            if has_code != 0 {
                eprintln!("fatal: unhandled panic (reason code {reason}): [{kind_str} #{code}] {msg_str}");
            } else {
                eprintln!("fatal: unhandled panic (reason code {reason}): [{kind_str}] {msg_str}");
            }
            drop(Box::from_raw(raw_exc));
        }
    }

    std::process::abort();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_panic_abort(
    msg_ptr: *const u8,
    msg_len: usize,
    kind_ptr: *const u8,
    kind_len: usize,
    has_code: i32,
    code: i32,
) -> ! {
    let msg_str = if !msg_ptr.is_null() && msg_len > 0 {
        let slice = unsafe { std::slice::from_raw_parts(msg_ptr, msg_len) };
        std::str::from_utf8(slice).unwrap_or("<invalid utf8 panic message>")
    } else {
        "<empty panic message>"
    };

    let kind_str = if !kind_ptr.is_null() && kind_len > 0 {
        let slice = unsafe { std::slice::from_raw_parts(kind_ptr, kind_len) };
        std::str::from_utf8(slice).unwrap_or("Panic")
    } else {
        "Panic"
    };

    if has_code != 0 {
        eprintln!("fatal panic (abort): [{kind_str} #{code}] {msg_str}");
    } else {
        eprintln!("fatal panic (abort): [{kind_str}] {msg_str}");
    }

    std::process::abort();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yukino_raise_overflow() -> ! {
    let msg = b"integer overflow (R0101)";
    unsafe { yukino_raise_panic(msg.as_ptr(), msg.len()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exception_structure() {
        let msg = b"test error";
        let kind = b"CustomError";
        let boxed = Box::new(YukinoException {
            unwind_header: UnwindException {
                exception_class: YUKINO_EXCEPTION_CLASS,
                exception_cleanup: Some(yukino_exception_cleanup),
                private_1: 0,
                private_2: 0,
            },
            message_ptr: msg.as_ptr(),
            message_len: msg.len(),
            kind_ptr: kind.as_ptr(),
            kind_len: kind.len(),
            has_code: 1,
            code: 404,
            payload_ptr: std::ptr::null_mut(),
            payload_len: 0,
        });
        let raw = Box::into_raw(boxed);
        unsafe {
            let s = yukino_exception_get_message(&raw const (*raw).unwind_header);
            assert_eq!(s.len, msg.len());
            assert_eq!(s.ptr, msg.as_ptr());
            let k = yukino_exception_get_kind(&raw const (*raw).unwind_header);
            assert_eq!(k.len, kind.len());
            assert_eq!(k.ptr, kind.as_ptr());
            assert_eq!(yukino_exception_has_code(&raw const (*raw).unwind_header), 1);
            assert_eq!(yukino_exception_get_code(&raw const (*raw).unwind_header), 404);
            assert!(yukino_exception_get_payload(&raw const (*raw).unwind_header).is_null());
            let cleanup = (*raw).unwind_header.exception_cleanup.unwrap();
            cleanup(0, &raw mut (*raw).unwind_header);
        }
    }

    #[test]
    fn test_foreign_exception_message() {
        let mut foreign = UnwindException {
            exception_class: 0x12345678,
            exception_cleanup: None,
            private_1: 0,
            private_2: 0,
        };
        unsafe {
            let s = yukino_exception_get_message(&raw mut foreign);
            assert!(s.ptr.is_null());
            assert_eq!(s.len, 0);
            let k = yukino_exception_get_kind(&raw mut foreign);
            assert!(k.ptr.is_null());
            assert_eq!(k.len, 0);
            assert_eq!(yukino_exception_has_code(&raw mut foreign), 0);
            assert_eq!(yukino_exception_get_code(&raw mut foreign), 0);
            assert!(yukino_exception_get_payload(&raw mut foreign).is_null());
        }
    }
}
