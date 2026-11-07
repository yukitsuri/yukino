use std::cell::RefCell;
use std::ffi::CString;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct YukinoString {
    pub ptr: *const u8,
    pub len: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct YukinoDiskInfo {
    pub name: YukinoString,
    pub mount_point: YukinoString,
    pub file_system: YukinoString,
    pub total_space: u64,
    pub available_space: u64,
    pub is_removable: bool,
    pub is_read_only: bool,
}

thread_local! {
    static SYSINFO_STR_RING: RefCell<(usize, Vec<Vec<u8>>)> = const { RefCell::new((0, Vec::new())) };
    static DISKS_INFO_BUF: RefCell<Vec<YukinoDiskInfo>> = const { RefCell::new(Vec::new()) };
    static DISKS_STR_POOL: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
}

fn set_str_result(bytes: &[u8], out_len: *mut i64) -> *const u8 {
    SYSINFO_STR_RING.with(|cell| {
        let mut ring = cell.borrow_mut();
        if ring.1.len() < 64 {
            ring.1.resize_with(64, Vec::new);
        }
        let idx = ring.0 % 64;
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

#[cfg(unix)]
unsafe fn unix_gethostname() -> Option<String> {
    unsafe extern "C" {
        fn gethostname(name: *mut std::ffi::c_char, len: usize) -> std::ffi::c_int;
    }
    let mut buf = [0u8; 256];
    if unsafe { gethostname(buf.as_mut_ptr() as *mut std::ffi::c_char, buf.len()) } == 0 {
        let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        std::str::from_utf8(&buf[..len]).ok().map(|s| s.to_string())
    } else {
        None
    }
}

#[cfg(unix)]
unsafe fn unix_get_euid() -> u32 {
    unsafe extern "C" {
        fn geteuid() -> std::ffi::c_uint;
    }
    unsafe { geteuid() as u32 }
}

#[cfg(unix)]
unsafe fn unix_statvfs(path: &str) -> Option<(u64, u64, bool)> {
    #[repr(C)]
    struct Statvfs {
        f_bsize: std::ffi::c_ulong,
        f_frsize: std::ffi::c_ulong,
        f_blocks: u64,
        f_bfree: u64,
        f_bavail: u64,
        f_files: u64,
        f_ffree: u64,
        f_favail: u64,
        f_fsid: std::ffi::c_ulong,
        f_flag: std::ffi::c_ulong,
        f_namemax: std::ffi::c_ulong,
        __f_spare: [std::ffi::c_int; 6],
    }
    unsafe extern "C" {
        fn statvfs(path: *const std::ffi::c_char, buf: *mut Statvfs) -> std::ffi::c_int;
    }
    let c_path = CString::new(path).ok()?;
    let mut st: Statvfs = unsafe { std::mem::zeroed() };
    if unsafe { statvfs(c_path.as_ptr(), &mut st) } == 0 {
        let frsize = if st.f_frsize > 0 { st.f_frsize as u64 } else { st.f_bsize as u64 };
        let total = st.f_blocks.saturating_mul(frsize);
        let avail = st.f_bavail.saturating_mul(frsize);
        let ro = (st.f_flag & 1) != 0; // ST_RDONLY = 1
        Some((total, avail, ro))
    } else {
        None
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_host_name(out_len: *mut i64) -> *const u8 {
    if let Ok(name) = fs::read_to_string("/proc/sys/kernel/hostname") {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            return set_str_result(trimmed.as_bytes(), out_len);
        }
    }
    #[cfg(unix)]
    if let Some(host) = unsafe { unix_gethostname() } {
        let trimmed = host.trim();
        if !trimmed.is_empty() {
            return set_str_result(trimmed.as_bytes(), out_len);
        }
    }
    if let Ok(h) = std::env::var("HOSTNAME").or_else(|_| std::env::var("COMPUTERNAME")) {
        let trimmed = h.trim();
        if !trimmed.is_empty() {
            return set_str_result(trimmed.as_bytes(), out_len);
        }
    }
    if !out_len.is_null() {
        unsafe { *out_len = 0; }
    }
    std::ptr::null()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_os_name(out_len: *mut i64) -> *const u8 {
    let name = match std::env::consts::OS {
        "linux" => "Linux",
        "windows" => "Windows",
        "macos" => "macOS",
        "android" => "Android",
        "freebsd" => "FreeBSD",
        "netbsd" => "NetBSD",
        "openbsd" => "OpenBSD",
        other => other,
    };
    set_str_result(name.as_bytes(), out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_os_version(out_len: *mut i64) -> *const u8 {
    if let Ok(content) = fs::read_to_string("/etc/os-release").or_else(|_| fs::read_to_string("/usr/lib/os-release")) {
        for line in content.lines() {
            if let Some(val) = line.strip_prefix("VERSION_ID=") {
                let trimmed = val.trim().trim_matches('"');
                if !trimmed.is_empty() {
                    return set_str_result(trimmed.as_bytes(), out_len);
                }
            } else if let Some(val) = line.strip_prefix("BUILD_ID=") {
                let trimmed = val.trim().trim_matches('"');
                if !trimmed.is_empty() {
                    return set_str_result(trimmed.as_bytes(), out_len);
                }
            }
        }
    }
    if let Ok(rel) = fs::read_to_string("/proc/sys/kernel/osrelease") {
        let trimmed = rel.trim();
        if !trimmed.is_empty() {
            return set_str_result(trimmed.as_bytes(), out_len);
        }
    }
    if !out_len.is_null() {
        unsafe { *out_len = 0; }
    }
    std::ptr::null()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_kernel_version(out_len: *mut i64) -> *const u8 {
    if let Ok(rel) = fs::read_to_string("/proc/sys/kernel/osrelease") {
        let trimmed = rel.trim();
        if !trimmed.is_empty() {
            return set_str_result(trimmed.as_bytes(), out_len);
        }
    }
    if !out_len.is_null() {
        unsafe { *out_len = 0; }
    }
    std::ptr::null()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_distribution_id(out_len: *mut i64) -> *const u8 {
    if let Ok(content) = fs::read_to_string("/etc/os-release").or_else(|_| fs::read_to_string("/usr/lib/os-release")) {
        for line in content.lines() {
            if let Some(val) = line.strip_prefix("ID=") {
                let trimmed = val.trim().trim_matches('"');
                if !trimmed.is_empty() {
                    return set_str_result(trimmed.as_bytes(), out_len);
                }
            }
        }
    }
    set_str_result(std::env::consts::OS.as_bytes(), out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_cpu_arch(out_len: *mut i64) -> *const u8 {
    set_str_result(std::env::consts::ARCH.as_bytes(), out_len)
}

#[cfg(windows)]
unsafe extern "system" {
    fn GetTickCount64() -> u64;
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_uptime() -> u64 {
    #[cfg(windows)]
    {
        // GetTickCount64 returns milliseconds elapsed since system boot.
        unsafe { GetTickCount64() / 1000 }
    }
    #[cfg(not(windows))]
    {
        if let Ok(content) = fs::read_to_string("/proc/uptime") {
            if let Some(first) = content.split_whitespace().next() {
                if let Ok(f) = first.parse::<f64>() {
                    return f.max(0.0) as u64;
                }
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_boot_time() -> u64 {
    if let Ok(content) = fs::read_to_string("/proc/stat") {
        for line in content.lines() {
            if let Some(val) = line.strip_prefix("btime ") {
                if let Ok(t) = val.trim().parse::<u64>() {
                    return t;
                }
            }
        }
    }
    let up = yukistd_sysinfo_uptime();
    if up > 0 {
        if let Ok(d) = SystemTime::now().duration_since(UNIX_EPOCH) {
            return d.as_secs().saturating_sub(up);
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_current_user_name(out_len: *mut i64) -> *const u8 {
    if let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")).or_else(|_| std::env::var("LOGNAME")) {
        let trimmed = user.trim();
        if !trimmed.is_empty() {
            return set_str_result(trimmed.as_bytes(), out_len);
        }
    }
    if !out_len.is_null() {
        unsafe { *out_len = 0; }
    }
    std::ptr::null()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_current_user_id(out_len: *mut i64) -> *const u8 {
    #[cfg(unix)]
    {
        let uid = unsafe { unix_get_euid() };
        let s = uid.to_string();
        return set_str_result(s.as_bytes(), out_len);
    }
    #[cfg(not(unix))]
    {
        if let Ok(user) = std::env::var("USERNAME") {
            return set_str_result(user.as_bytes(), out_len);
        }
        if !out_len.is_null() {
            unsafe { *out_len = 0; }
        }
        std::ptr::null()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_is_elevated() -> bool {
    #[cfg(unix)]
    {
        unsafe { unix_get_euid() == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_logical_core_count() -> u32 {
    std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_physical_core_count(out_val: *mut u32) -> bool {
    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        let mut cores = std::collections::HashSet::new();
        let mut current_phys: Option<String> = None;
        let mut current_core: Option<String> = None;

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("physical id") {
                if let Some((_, v)) = line.split_once(':') {
                    current_phys = Some(v.trim().to_string());
                }
            } else if line.starts_with("core id") {
                if let Some((_, v)) = line.split_once(':') {
                    current_core = Some(v.trim().to_string());
                }
            } else if line.is_empty() {
                if let (Some(p), Some(c)) = (current_phys.take(), current_core.take()) {
                    cores.insert((p, c));
                }
            }
        }
        if let (Some(p), Some(c)) = (current_phys, current_core) {
            cores.insert((p, c));
        }

        if !cores.is_empty() {
            if !out_val.is_null() {
                unsafe { *out_val = cores.len() as u32; }
            }
            return true;
        }
    }
    let logical = yukistd_sysinfo_logical_core_count();
    if !out_val.is_null() {
        unsafe { *out_val = logical; }
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_cpu_brand(out_len: *mut i64) -> *const u8 {
    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("model name") || line.starts_with("Processor") || line.starts_with("Hardware") {
                if let Some((_, v)) = line.split_once(':') {
                    let brand = v.trim();
                    if !brand.is_empty() {
                        return set_str_result(brand.as_bytes(), out_len);
                    }
                }
            }
        }
    }
    if !out_len.is_null() {
        unsafe { *out_len = 0; }
    }
    std::ptr::null()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_cpu_frequency() -> u64 {
    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("cpu MHz") {
                if let Some((_, v)) = line.split_once(':') {
                    if let Ok(mhz) = v.trim().parse::<f64>() {
                        return mhz.max(0.0) as u64;
                    }
                }
            }
        }
    }
    if let Ok(freq_str) = fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq") {
        if let Ok(khz) = freq_str.trim().parse::<u64>() {
            return khz / 1000;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_load_average(out_one: *mut f64, out_five: *mut f64, out_fifteen: *mut f64) -> bool {
    if let Ok(content) = fs::read_to_string("/proc/loadavg") {
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 3 {
            if let (Ok(o), Ok(f), Ok(ff)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>(), parts[2].parse::<f64>()) {
                if !out_one.is_null() { unsafe { *out_one = o; } }
                if !out_five.is_null() { unsafe { *out_five = f; } }
                if !out_fifteen.is_null() { unsafe { *out_fifteen = ff; } }
                return true;
            }
        }
    }
    #[cfg(unix)]
    {
        unsafe extern "C" {
            fn getloadavg(loadavg: *mut f64, nelem: std::ffi::c_int) -> std::ffi::c_int;
        }
        let mut arr = [0.0f64; 3];
        if unsafe { getloadavg(arr.as_mut_ptr(), 3) == 3 } {
            if !out_one.is_null() { unsafe { *out_one = arr[0]; } }
            if !out_five.is_null() { unsafe { *out_five = arr[1]; } }
            if !out_fifteen.is_null() { unsafe { *out_fifteen = arr[2]; } }
            return true;
        }
    }
    false
}

struct MemStats {
    total: u64,
    free: u64,
    available: u64,
    swap_total: u64,
    swap_free: u64,
}

fn parse_meminfo() -> Option<MemStats> {
    #[cfg(windows)]
    {
        windows_mem_stats()
    }
    #[cfg(not(windows))]
    {
        proc_meminfo_stats()
    }
}

#[cfg(windows)]
#[repr(C)]
struct MemoryStatusEx {
    dw_length: u32,
    dw_memory_load: u32,
    ull_total_phys: u64,
    ull_avail_phys: u64,
    ull_total_page_file: u64,
    ull_avail_page_file: u64,
    ull_total_virtual: u64,
    ull_avail_virtual: u64,
    ull_avail_extended_virtual: u64,
}

#[cfg(windows)]
unsafe extern "system" {
    fn GlobalMemoryStatusEx(lp_buffer: *mut MemoryStatusEx) -> i32;
}

#[cfg(windows)]
fn windows_mem_stats() -> Option<MemStats> {
    let mut status = MemoryStatusEx {
        dw_length: std::mem::size_of::<MemoryStatusEx>() as u32,
        dw_memory_load: 0,
        ull_total_phys: 0,
        ull_avail_phys: 0,
        ull_total_page_file: 0,
        ull_avail_page_file: 0,
        ull_total_virtual: 0,
        ull_avail_virtual: 0,
        ull_avail_extended_virtual: 0,
    };
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        return None;
    }
    // page file counters include RAM; swap is the commit limit beyond physical memory
    Some(MemStats {
        total: status.ull_total_phys,
        free: status.ull_avail_phys,
        available: status.ull_avail_phys,
        swap_total: status.ull_total_page_file.saturating_sub(status.ull_total_phys),
        swap_free: status.ull_avail_page_file.saturating_sub(status.ull_avail_phys),
    })
}

#[cfg(not(windows))]
fn proc_meminfo_stats() -> Option<MemStats> {
    let content = fs::read_to_string("/proc/meminfo").ok()?;
    let mut stats = MemStats {
        total: 0,
        free: 0,
        available: 0,
        swap_total: 0,
        swap_free: 0,
    };
    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let key = parts.next()?;
        let val_str = parts.next()?;
        let val_kb = val_str.parse::<u64>().unwrap_or(0);
        let bytes = val_kb.saturating_mul(1024);
        match key {
            "MemTotal:" => stats.total = bytes,
            "MemFree:" => stats.free = bytes,
            "MemAvailable:" => stats.available = bytes,
            "SwapTotal:" => stats.swap_total = bytes,
            "SwapFree:" => stats.swap_free = bytes,
            _ => {}
        }
    }
    if stats.available == 0 {
        stats.available = stats.free;
    }
    Some(stats)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_total_memory() -> u64 {
    parse_meminfo().map(|s| s.total).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_free_memory() -> u64 {
    parse_meminfo().map(|s| s.free).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_available_memory() -> u64 {
    parse_meminfo().map(|s| s.available).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_used_memory() -> u64 {
    parse_meminfo().map(|s| s.total.saturating_sub(s.available)).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_total_swap() -> u64 {
    parse_meminfo().map(|s| s.swap_total).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_free_swap() -> u64 {
    parse_meminfo().map(|s| s.swap_free).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_used_swap() -> u64 {
    parse_meminfo().map(|s| s.swap_total.saturating_sub(s.swap_free)).unwrap_or(0)
}

#[cfg(windows)]
unsafe extern "system" {
    fn GetDiskFreeSpaceExW(
        lp_directory_name: *const u16,
        lp_free_bytes_available_to_caller: *mut u64,
        lp_total_number_of_bytes: *mut u64,
        lp_total_number_of_free_bytes: *mut u64,
    ) -> i32;
    fn GetLogicalDrives() -> u32;
    fn GetDriveTypeW(lp_root_path_name: *const u16) -> u32;
    fn GetVolumeInformationW(
        lp_root_path_name: *const u16,
        lp_volume_name_buffer: *mut u16,
        n_volume_name_size: u32,
        lp_volume_serial_number: *mut u32,
        lp_maximum_component_length: *mut u32,
        lp_file_system_flags: *mut u32,
        lp_file_system_name_buffer: *mut u16,
        n_file_system_name_size: u32,
    ) -> i32;
}

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn win_disk_space(path: &str) -> Option<(u64, u64)> {
    let wide = to_wide(path);
    let mut avail: u64 = 0;
    let mut total: u64 = 0;
    let mut free: u64 = 0;
    if unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut avail, &mut total, &mut free) } != 0 {
        Some((total, avail))
    } else {
        None
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_disk_space(path_p: *const u8, path_len: i64, out_total: *mut u64, out_avail: *mut u64) -> bool {
    let path = unsafe { str_from_raw(path_p, path_len) };
    if path.is_empty() {
        return false;
    }
    #[cfg(windows)]
    if let Some((total, avail)) = win_disk_space(path) {
        if !out_total.is_null() { unsafe { *out_total = total; } }
        if !out_avail.is_null() { unsafe { *out_avail = avail; } }
        return true;
    }
    #[cfg(unix)]
    if let Some((total, avail, _)) = unsafe { unix_statvfs(path) } {
        if !out_total.is_null() { unsafe { *out_total = total; } }
        if !out_avail.is_null() { unsafe { *out_avail = avail; } }
        return true;
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_disks(out_count: *mut i64) -> *const YukinoDiskInfo {
    DISKS_STR_POOL.with(|pool_cell| {
        DISKS_INFO_BUF.with(|buf_cell| {
            let mut pool = pool_cell.borrow_mut();
            let mut buf = buf_cell.borrow_mut();
            pool.clear();
            buf.clear();

            #[cfg(windows)]
            {
                let mask = unsafe { GetLogicalDrives() };
                for i in 0..26u32 {
                    if mask & (1u32 << i) == 0 {
                        continue;
                    }
                    let letter = (b'A' + i as u8) as char;
                    let root = format!("{letter}:\\");
                    // drive types: 0 unknown, 1 no root, 2 removable, 3 fixed, 4 remote, 5 cdrom, 6 ramdisk
                    let drive_type = unsafe { GetDriveTypeW(to_wide(&root).as_ptr()) };
                    if drive_type <= 1 || drive_type == 5 {
                        continue;
                    }
                    let Some((total, avail)) = win_disk_space(&root) else { continue };
                    if total == 0 {
                        continue;
                    }

                    let mut fs_name = [0u16; 32];
                    let mut vol_name = [0u16; 64];
                    let mut serial = 0u32;
                    let mut max_comp = 0u32;
                    let mut flags = 0u32;
                    let fs_str = if unsafe {
                        GetVolumeInformationW(
                            to_wide(&root).as_ptr(),
                            vol_name.as_mut_ptr(),
                            vol_name.len() as u32,
                            &mut serial,
                            &mut max_comp,
                            &mut flags,
                            fs_name.as_mut_ptr(),
                            fs_name.len() as u32,
                        )
                    } != 0
                    {
                        let end = fs_name.iter().position(|&c| c == 0).unwrap_or(fs_name.len());
                        String::from_utf16_lossy(&fs_name[..end])
                    } else {
                        String::new()
                    };

                    let dev = format!("{letter}:");
                    let is_remov = drive_type == 2;
                    // FILE_READ_ONLY_VOLUME
                    let is_ro = flags & 0x0000_0008 != 0;

                    pool.push(dev.as_bytes().to_vec());
                    let name_p = pool.last().unwrap().as_ptr();
                    let name_len = dev.len() as i64;

                    pool.push(root.as_bytes().to_vec());
                    let mount_p = pool.last().unwrap().as_ptr();
                    let mount_len = root.len() as i64;

                    pool.push(fs_str.as_bytes().to_vec());
                    let fs_p = pool.last().unwrap().as_ptr();
                    let fs_len = fs_str.len() as i64;

                    buf.push(YukinoDiskInfo {
                        name: YukinoString { ptr: name_p, len: name_len },
                        mount_point: YukinoString { ptr: mount_p, len: mount_len },
                        file_system: YukinoString { ptr: fs_p, len: fs_len },
                        total_space: total,
                        available_space: avail,
                        is_removable: is_remov,
                        is_read_only: is_ro,
                    });
                }
            }

            #[cfg(not(windows))]
            if let Ok(mounts) = fs::read_to_string("/proc/mounts") {
                let skip_types = [
                    "proc", "sysfs", "devpts", "cgroup", "cgroup2", "tmpfs", "devtmpfs",
                    "securityfs", "pstore", "bpf", "autofs", "tracefs", "debugfs", "configfs",
                    "ramfs", "fusectl", "mqueue", "hugetlbfs", "binfmt_misc"
                ];

                for line in mounts.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() < 4 {
                        continue;
                    }
                    let dev = parts[0];
                    let mount = parts[1];
                    let fstype = parts[2];
                    let opts = parts[3];

                    if !dev.starts_with('/') {
                        continue;
                    }
                    if skip_types.contains(&fstype) {
                        continue;
                    }

                    #[cfg(unix)]
                    if let Some((total, avail, ro)) = unsafe { unix_statvfs(mount) } {
                        if total == 0 {
                            continue;
                        }
                        let is_ro = ro || opts.split(',').any(|o| o == "ro");
                        let is_remov = mount.starts_with("/media") || mount.starts_with("/run/media") || opts.contains("removable");

                        pool.push(dev.as_bytes().to_vec());
                        let name_p = pool.last().unwrap().as_ptr();
                        let name_len = dev.len() as i64;

                        pool.push(mount.as_bytes().to_vec());
                        let mount_p = pool.last().unwrap().as_ptr();
                        let mount_len = mount.len() as i64;

                        pool.push(fstype.as_bytes().to_vec());
                        let fs_p = pool.last().unwrap().as_ptr();
                        let fs_len = fstype.len() as i64;

                        buf.push(YukinoDiskInfo {
                            name: YukinoString { ptr: name_p, len: name_len },
                            mount_point: YukinoString { ptr: mount_p, len: mount_len },
                            file_system: YukinoString { ptr: fs_p, len: fs_len },
                            total_space: total,
                            available_space: avail,
                            is_removable: is_remov,
                            is_read_only: is_ro,
                        });
                    }
                }
            }

            if !out_count.is_null() {
                unsafe { *out_count = buf.len() as i64; }
            }
            if buf.is_empty() {
                std::ptr::null()
            } else {
                buf.as_ptr()
            }
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_battery_percent(out_val: *mut f32) -> bool {
    let power_path = Path::new("/sys/class/power_supply");
    if let Ok(entries) = fs::read_dir(power_path) {
        for entry_res in entries {
            if let Ok(entry) = entry_res {
                let name = entry.file_name().to_string_lossy().into_owned();
                let is_bat = name.starts_with("BAT") || {
                    let type_path = entry.path().join("type");
                    fs::read_to_string(type_path).map(|t| t.trim() == "Battery").unwrap_or(false)
                };
                if is_bat {
                    let cap_path = entry.path().join("capacity");
                    if let Ok(cap_str) = fs::read_to_string(cap_path) {
                        if let Ok(cap) = cap_str.trim().parse::<f32>() {
                            if !out_val.is_null() {
                                unsafe { *out_val = cap.clamp(0.0, 100.0); }
                            }
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfo_is_battery_charging(out_val: *mut bool) -> bool {
    let power_path = Path::new("/sys/class/power_supply");
    if let Ok(entries) = fs::read_dir(power_path) {
        for entry_res in entries {
            if let Ok(entry) = entry_res {
                let name = entry.file_name().to_string_lossy().into_owned();
                let is_bat = name.starts_with("BAT") || {
                    let type_path = entry.path().join("type");
                    fs::read_to_string(type_path).map(|t| t.trim() == "Battery").unwrap_or(false)
                };
                if is_bat {
                    let status_path = entry.path().join("status");
                    if let Ok(status_str) = fs::read_to_string(status_path) {
                        let st = status_str.trim();
                        let is_charging = st.eq_ignore_ascii_case("Charging");
                        if !out_val.is_null() {
                            unsafe { *out_val = is_charging; }
                        }
                        return true;
                    }
                }
            }
        }
    }
    false
}
