#[cfg(windows)]
unsafe extern "system" {
    fn TerminateProcess(hProcess: *mut std::ffi::c_void, uExitCode: u32) -> i32;
    fn GetCurrentProcessId() -> u32;
    fn GetProcessId(Process: *mut std::ffi::c_void) -> u32;
    fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
    fn WaitForSingleObject(hHandle: *mut std::ffi::c_void, dwMilliseconds: u32) -> u32;
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_win_terminate_process(handle: i64, exit_code: u32) -> bool {
    #[cfg(windows)]
    unsafe { TerminateProcess(handle as *mut _, exit_code) != 0 }
    #[cfg(not(windows))]
    {
        let _ = (handle, exit_code);
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_win_get_current_process_id() -> u32 {
    #[cfg(windows)]
    unsafe { GetCurrentProcessId() }
    #[cfg(not(windows))]
    { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_win_get_process_id(handle: i64) -> u32 {
    #[cfg(windows)]
    unsafe { GetProcessId(handle as *mut _) }
    #[cfg(not(windows))]
    {
        let _ = handle;
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_win_close_handle(handle: i64) -> bool {
    #[cfg(windows)]
    unsafe { CloseHandle(handle as *mut _) != 0 }
    #[cfg(not(windows))]
    {
        let _ = handle;
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_os_win_wait_for_single_object(handle: i64, timeout_ms: u32) -> u32 {
    #[cfg(windows)]
    unsafe { WaitForSingleObject(handle as *mut _, timeout_ms) }
    #[cfg(not(windows))]
    {
        let _ = (handle, timeout_ms);
        0xFFFFFFFF
    }
}
