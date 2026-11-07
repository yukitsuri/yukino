use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::process::{Child, Command, Stdio};
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
    static PROCESS_STR_RING: RefCell<(usize, Vec<Vec<u8>>)> = const { RefCell::new((0, Vec::new())) };
    static STDOUT_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static STDERR_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static ENVS_POOL: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static ENVS_ENTRIES: RefCell<Vec<YukinoString>> = const { RefCell::new(Vec::new()) };
}

static NEXT_CHILD_ID: AtomicI32 = AtomicI32::new(1);
static CHILDREN: Mutex<Option<HashMap<i32, Child>>> = Mutex::new(None);

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

fn set_str_result(bytes: &[u8], out_len: *mut i64) -> *const u8 {
    PROCESS_STR_RING.with(|cell| {
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

unsafe fn str_from_raw<'a>(ptr: *const u8, len: i64) -> &'a str {
    if ptr.is_null() || len <= 0 {
        return "";
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    std::str::from_utf8(slice).unwrap_or("")
}

unsafe fn parse_string_array(items_p: *const YukinoString, items_len: i64) -> Vec<String> {
    if items_p.is_null() || items_len <= 0 {
        return Vec::new();
    }
    let items = unsafe { std::slice::from_raw_parts(items_p, items_len as usize) };
    let mut result = Vec::with_capacity(items.len());
    for item in items {
        if item.ptr.is_null() || item.len <= 0 {
            result.push(String::new());
        } else {
            let slice = unsafe { std::slice::from_raw_parts(item.ptr, item.len as usize) };
            result.push(std::str::from_utf8(slice).unwrap_or("").to_string());
        }
    }
    result
}

fn split_launcher(launcher: &str) -> (String, Vec<String>) {
    let parts: Vec<&str> = launcher.split_whitespace().collect();
    if parts.is_empty() {
        return (String::new(), Vec::new());
    }
    let prog = parts[0].to_string();
    let args = parts[1..].iter().map(|s| s.to_string()).collect();
    (prog, args)
}

fn map_stdio(mode: i32) -> Stdio {
    match mode {
        1 => Stdio::piped(),
        2 => Stdio::null(),
        _ => Stdio::inherit(),
    }
}

fn extract_status(status: std::process::ExitStatus) -> (i32, i32) {
    let code = status.code().unwrap_or(-1);
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        let sig = status.signal().unwrap_or(0);
        (code, sig)
    }
    #[cfg(not(unix))]
    {
        (code, 0)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_last_error_code() -> i32 {
    LAST_ERROR.with(|cell| cell.borrow().0)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_last_error_message(out_len: *mut i64) -> *const u8 {
    LAST_ERROR.with(|cell| {
        let err = cell.borrow();
        set_str_result(err.1.as_bytes(), out_len)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_default_shell(out_len: *mut i64) -> *const u8 {
    #[cfg(windows)]
    let shell = "powershell -Command";
    #[cfg(not(windows))]
    let shell = "sh -c";
    set_str_result(shell.as_bytes(), out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_exec(
    launcher_p: *const u8,
    launcher_len: i64,
    script_p: *const u8,
    script_len: i64,
) -> i32 {
    clear_last_error();
    let launcher_str = unsafe { str_from_raw(launcher_p, launcher_len) };
    let script_str = unsafe { str_from_raw(script_p, script_len) };
    let (prog, mut args) = split_launcher(launcher_str);
    if prog.is_empty() {
        set_last_error(1, "empty launcher command".to_string());
        return -1;
    }
    args.push(script_str.to_string());

    let mut cmd = Command::new(&prog);
    cmd.args(&args);
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    match cmd.status() {
        Ok(s) => extract_status(s).0,
        Err(e) => {
            set_last_error(2, e.to_string());
            -1
        }
    }
}

fn set_output_buffers(
    out: std::process::Output,
    out_status: *mut i32,
    out_signal: *mut i32,
    out_stdout: *mut *const u8,
    out_stdout_len: *mut i64,
    out_stderr: *mut *const u8,
    out_stderr_len: *mut i64,
) {
    let (code, sig) = extract_status(out.status);
    if !out_status.is_null() { unsafe { *out_status = code; } }
    if !out_signal.is_null() { unsafe { *out_signal = sig; } }

    STDOUT_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        *buf = out.stdout;
        if !out_stdout_len.is_null() { unsafe { *out_stdout_len = buf.len() as i64; } }
        if !out_stdout.is_null() { unsafe { *out_stdout = buf.as_ptr(); } }
    });

    STDERR_BUF.with(|cell| {
        let mut buf = cell.borrow_mut();
        *buf = out.stderr;
        if !out_stderr_len.is_null() { unsafe { *out_stderr_len = buf.len() as i64; } }
        if !out_stderr.is_null() { unsafe { *out_stderr = buf.as_ptr(); } }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_run(
    prog_p: *const u8,
    prog_len: i64,
    args_p: *const YukinoString,
    args_len: i64,
    out_status: *mut i32,
    out_signal: *mut i32,
    out_stdout: *mut *const u8,
    out_stdout_len: *mut i64,
    out_stderr: *mut *const u8,
    out_stderr_len: *mut i64,
) -> bool {
    clear_last_error();
    let prog = unsafe { str_from_raw(prog_p, prog_len) };
    if prog.is_empty() {
        set_last_error(1, "empty program name".to_string());
        return false;
    }
    let args = unsafe { parse_string_array(args_p, args_len) };

    let mut cmd = Command::new(prog);
    cmd.args(&args);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    match cmd.output() {
        Ok(out) => {
            set_output_buffers(out, out_status, out_signal, out_stdout, out_stdout_len, out_stderr, out_stderr_len);
            true
        }
        Err(e) => {
            set_last_error(3, e.to_string());
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_run_shell(
    launcher_p: *const u8,
    launcher_len: i64,
    script_p: *const u8,
    script_len: i64,
    out_status: *mut i32,
    out_signal: *mut i32,
    out_stdout: *mut *const u8,
    out_stdout_len: *mut i64,
    out_stderr: *mut *const u8,
    out_stderr_len: *mut i64,
) -> bool {
    clear_last_error();
    let launcher_str = unsafe { str_from_raw(launcher_p, launcher_len) };
    let script_str = unsafe { str_from_raw(script_p, script_len) };
    let (prog, mut args) = split_launcher(launcher_str);
    if prog.is_empty() {
        set_last_error(1, "empty launcher command".to_string());
        return false;
    }
    args.push(script_str.to_string());

    let mut cmd = Command::new(&prog);
    cmd.args(&args);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    match cmd.output() {
        Ok(out) => {
            set_output_buffers(out, out_status, out_signal, out_stdout, out_stdout_len, out_stderr, out_stderr_len);
            true
        }
        Err(e) => {
            set_last_error(3, e.to_string());
            false
        }
    }
}

fn configure_command(
    prog: &str,
    args: &[String],
    cwd: &str,
    envs_k: &[String],
    envs_v: &[String],
    clear_env: bool,
    stdin_mode: i32,
    stdout_mode: i32,
    stderr_mode: i32,
) -> Command {
    let mut cmd = Command::new(prog);
    cmd.args(args);
    if !cwd.is_empty() {
        cmd.current_dir(cwd);
    }
    if clear_env {
        cmd.env_clear();
    }
    for (k, v) in envs_k.iter().zip(envs_v.iter()) {
        cmd.env(k, v);
    }
    cmd.stdin(map_stdio(stdin_mode));
    cmd.stdout(map_stdio(stdout_mode));
    cmd.stderr(map_stdio(stderr_mode));
    cmd
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_command_output(
    prog_p: *const u8,
    prog_len: i64,
    args_p: *const YukinoString,
    args_len: i64,
    cwd_p: *const u8,
    cwd_len: i64,
    envs_k_p: *const YukinoString,
    envs_v_p: *const YukinoString,
    envs_len: i64,
    clear_env: bool,
    out_status: *mut i32,
    out_signal: *mut i32,
    out_stdout: *mut *const u8,
    out_stdout_len: *mut i64,
    out_stderr: *mut *const u8,
    out_stderr_len: *mut i64,
) -> bool {
    clear_last_error();
    let prog = unsafe { str_from_raw(prog_p, prog_len) };
    let cwd = unsafe { str_from_raw(cwd_p, cwd_len) };
    let args = unsafe { parse_string_array(args_p, args_len) };
    let envs_k = unsafe { parse_string_array(envs_k_p, envs_len) };
    let envs_v = unsafe { parse_string_array(envs_v_p, envs_len) };

    let mut cmd = configure_command(prog, &args, cwd, &envs_k, &envs_v, clear_env, 2, 1, 1);
    match cmd.output() {
        Ok(out) => {
            set_output_buffers(out, out_status, out_signal, out_stdout, out_stdout_len, out_stderr, out_stderr_len);
            true
        }
        Err(e) => {
            set_last_error(4, e.to_string());
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_command_status(
    prog_p: *const u8,
    prog_len: i64,
    args_p: *const YukinoString,
    args_len: i64,
    cwd_p: *const u8,
    cwd_len: i64,
    envs_k_p: *const YukinoString,
    envs_v_p: *const YukinoString,
    envs_len: i64,
    clear_env: bool,
    stdin_mode: i32,
    stdout_mode: i32,
    stderr_mode: i32,
    out_status: *mut i32,
    out_signal: *mut i32,
) -> bool {
    clear_last_error();
    let prog = unsafe { str_from_raw(prog_p, prog_len) };
    let cwd = unsafe { str_from_raw(cwd_p, cwd_len) };
    let args = unsafe { parse_string_array(args_p, args_len) };
    let envs_k = unsafe { parse_string_array(envs_k_p, envs_len) };
    let envs_v = unsafe { parse_string_array(envs_v_p, envs_len) };

    let mut cmd = configure_command(prog, &args, cwd, &envs_k, &envs_v, clear_env, stdin_mode, stdout_mode, stderr_mode);
    match cmd.status() {
        Ok(s) => {
            let (code, sig) = extract_status(s);
            if !out_status.is_null() { unsafe { *out_status = code; } }
            if !out_signal.is_null() { unsafe { *out_signal = sig; } }
            true
        }
        Err(e) => {
            set_last_error(4, e.to_string());
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_command_spawn(
    prog_p: *const u8,
    prog_len: i64,
    args_p: *const YukinoString,
    args_len: i64,
    cwd_p: *const u8,
    cwd_len: i64,
    envs_k_p: *const YukinoString,
    envs_v_p: *const YukinoString,
    envs_len: i64,
    clear_env: bool,
    stdin_mode: i32,
    stdout_mode: i32,
    stderr_mode: i32,
    out_pid: *mut i32,
) -> i32 {
    clear_last_error();
    let prog = unsafe { str_from_raw(prog_p, prog_len) };
    let cwd = unsafe { str_from_raw(cwd_p, cwd_len) };
    let args = unsafe { parse_string_array(args_p, args_len) };
    let envs_k = unsafe { parse_string_array(envs_k_p, envs_len) };
    let envs_v = unsafe { parse_string_array(envs_v_p, envs_len) };

    let mut cmd = configure_command(prog, &args, cwd, &envs_k, &envs_v, clear_env, stdin_mode, stdout_mode, stderr_mode);
    match cmd.spawn() {
        Ok(child) => {
            let id = NEXT_CHILD_ID.fetch_add(1, Ordering::Relaxed);
            let pid = child.id() as i32;
            if !out_pid.is_null() {
                unsafe { *out_pid = pid; }
            }
            let mut guard = match CHILDREN.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            guard.get_or_insert_with(HashMap::new).insert(id, child);
            id
        }
        Err(e) => {
            set_last_error(5, e.to_string());
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_child_id(handle: i32) -> i32 {
    let guard = match CHILDREN.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    guard.as_ref().and_then(|m| m.get(&handle)).map(|c| c.id() as i32).unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_child_wait(
    handle: i32,
    out_status: *mut i32,
    out_signal: *mut i32,
) -> bool {
    let mut child = {
        let mut guard = match CHILDREN.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        match guard.as_mut().and_then(|m| m.remove(&handle)) {
            Some(c) => c,
            None => {
                set_last_error(6, "invalid child process handle".to_string());
                return false;
            }
        }
    };

    match child.wait() {
        Ok(s) => {
            let (code, sig) = extract_status(s);
            if !out_status.is_null() { unsafe { *out_status = code; } }
            if !out_signal.is_null() { unsafe { *out_signal = sig; } }
            true
        }
        Err(e) => {
            set_last_error(7, e.to_string());
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_child_try_wait(
    handle: i32,
    out_has_exited: *mut bool,
    out_status: *mut i32,
    out_signal: *mut i32,
) -> bool {
    let mut guard = match CHILDREN.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let child = match guard.as_mut().and_then(|m| m.get_mut(&handle)) {
        Some(c) => c,
        None => {
            set_last_error(6, "invalid child process handle".to_string());
            return false;
        }
    };

    match child.try_wait() {
        Ok(Some(s)) => {
            if !out_has_exited.is_null() { unsafe { *out_has_exited = true; } }
            let (code, sig) = extract_status(s);
            if !out_status.is_null() { unsafe { *out_status = code; } }
            if !out_signal.is_null() { unsafe { *out_signal = sig; } }
            true
        }
        Ok(None) => {
            if !out_has_exited.is_null() { unsafe { *out_has_exited = false; } }
            true
        }
        Err(e) => {
            set_last_error(7, e.to_string());
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_child_kill(handle: i32) -> bool {
    let mut guard = match CHILDREN.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let child = match guard.as_mut().and_then(|m| m.get_mut(&handle)) {
        Some(c) => c,
        None => {
            set_last_error(6, "invalid child process handle".to_string());
            return false;
        }
    };
    child.kill().is_ok()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_child_terminate(handle: i32) -> bool {
    #[cfg(unix)]
    {
        let pid = yukistd_process_child_id(handle);
        if pid <= 0 { return false; }
        unsafe { libc_kill(pid, 15) == 0 }
    }
    #[cfg(not(unix))]
    {
        yukistd_process_child_kill(handle)
    }
}

#[cfg(unix)]
unsafe fn libc_kill(pid: i32, sig: i32) -> i32 {
    unsafe extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    unsafe { kill(pid, sig) }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_child_wait_with_output(
    handle: i32,
    out_status: *mut i32,
    out_signal: *mut i32,
    out_stdout: *mut *const u8,
    out_stdout_len: *mut i64,
    out_stderr: *mut *const u8,
    out_stderr_len: *mut i64,
) -> bool {
    let child = {
        let mut guard = match CHILDREN.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        match guard.as_mut().and_then(|m| m.remove(&handle)) {
            Some(c) => c,
            None => {
                set_last_error(6, "invalid child process handle".to_string());
                return false;
            }
        }
    };

    match child.wait_with_output() {
        Ok(out) => {
            set_output_buffers(out, out_status, out_signal, out_stdout, out_stdout_len, out_stderr, out_stderr_len);
            true
        }
        Err(e) => {
            set_last_error(8, e.to_string());
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_child_write_stdin(
    handle: i32,
    data_p: *const u8,
    data_len: i64,
    close_after: bool,
) -> bool {
    let mut guard = match CHILDREN.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let child = match guard.as_mut().and_then(|m| m.get_mut(&handle)) {
        Some(c) => c,
        None => return false,
    };
    if let Some(stdin) = child.stdin.as_mut() {
        let bytes = if data_p.is_null() || data_len <= 0 {
            &[][..]
        } else {
            unsafe { std::slice::from_raw_parts(data_p, data_len as usize) }
        };
        if stdin.write_all(bytes).is_err() {
            return false;
        }
        let _ = stdin.flush();
    }
    if close_after {
        child.stdin.take();
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_child_drop(handle: i32) {
    let mut guard = match CHILDREN.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(map) = guard.as_mut() {
        map.remove(&handle);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_current_pid() -> i32 {
    std::process::id() as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_parent_pid() -> i32 {
    #[cfg(unix)]
    {
        unsafe extern "C" { fn getppid() -> i32; }
        unsafe { getppid() }
    }
    #[cfg(not(unix))]
    {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_current_exe(out_len: *mut i64) -> *const u8 {
    let exe = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    set_str_result(exe.as_bytes(), out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_current_dir(out_len: *mut i64) -> *const u8 {
    let cwd = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    set_str_result(cwd.as_bytes(), out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_set_current_dir(path_p: *const u8, path_len: i64) -> bool {
    let p = unsafe { str_from_raw(path_p, path_len) };
    std::env::set_current_dir(p).is_ok()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_get_env(name_p: *const u8, name_len: i64, out_len: *mut i64) -> *const u8 {
    let name = unsafe { str_from_raw(name_p, name_len) };
    if let Ok(val) = std::env::var(name) {
        set_str_result(val.as_bytes(), out_len)
    } else {
        if !out_len.is_null() { unsafe { *out_len = -1; } }
        std::ptr::null()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_set_env(name_p: *const u8, name_len: i64, val_p: *const u8, val_len: i64) -> bool {
    let name = unsafe { str_from_raw(name_p, name_len) };
    let val = unsafe { str_from_raw(val_p, val_len) };
    if name.is_empty() { return false; }
    unsafe { std::env::set_var(name, val); }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_remove_env(name_p: *const u8, name_len: i64) -> bool {
    let name = unsafe { str_from_raw(name_p, name_len) };
    if name.is_empty() { return false; }
    unsafe { std::env::remove_var(name); }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_all_envs(out_count: *mut i64) -> *const YukinoString {
    ENVS_POOL.with(|pool_cell| {
        ENVS_ENTRIES.with(|entries_cell| {
            let mut pool = pool_cell.borrow_mut();
            let mut entries = entries_cell.borrow_mut();
            pool.clear();
            entries.clear();

            for (k, v) in std::env::vars() {
                let s = format!("{}={}", k, v);
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

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_process_abort() {
    std::process::abort();
}
