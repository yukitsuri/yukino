use std::cell::RefCell;
use std::fs;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct YukinoString {
    pub ptr: *const u8,
    pub len: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct YukinoNetworkInterfaceInfo {
    pub name: YukinoString,
    pub mac_address: YukinoString,
    pub is_up: bool,
    pub is_loopback: bool,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct YukinoThermalSensorInfo {
    pub label: YukinoString,
    pub temperature_celsius: f32,
    pub has_critical: bool,
    pub critical_temp: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct YukinoProcessInfo {
    pub pid: u32,
    pub parent_pid: u32,
    pub name: YukinoString,
    pub cmdline: YukinoString,
    pub cpu_usage: f32,
    pub memory_rss: u64,
    pub memory_virtual: u64,
}

thread_local! {
    static NET_STR_POOL: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
    static NET_INFO_BUF: RefCell<Vec<YukinoNetworkInterfaceInfo>> = const { RefCell::new(Vec::new()) };

    static THERMAL_STR_POOL: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
    static THERMAL_INFO_BUF: RefCell<Vec<YukinoThermalSensorInfo>> = const { RefCell::new(Vec::new()) };

    static PROC_STR_POOL: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
    static PROC_INFO_BUF: RefCell<Vec<YukinoProcessInfo>> = const { RefCell::new(Vec::new()) };
}

fn alloc_pool_str(pool: &mut Vec<Vec<u8>>, text: &str) -> YukinoString {
    pool.push(text.as_bytes().to_vec());
    let last = pool.last().unwrap();
    YukinoString {
        ptr: last.as_ptr(),
        len: last.len() as i64,
    }
}

#[cfg(not(windows))]
fn read_linux_iface_mac(ifname: &str) -> String {
    let path = format!("/sys/class/net/{}/address", ifname);
    fs::read_to_string(path).map(|s| s.trim().to_string()).unwrap_or_default()
}

#[cfg(not(windows))]
fn read_linux_iface_flags(ifname: &str) -> (bool, bool) {
    let flags_path = format!("/sys/class/net/{}/flags", ifname);
    if let Ok(raw) = fs::read_to_string(flags_path) {
        let clean = raw.trim().trim_start_matches("0x");
        if let Ok(flags) = u32::from_str_radix(clean, 16) {
            return ((flags & 0x1) != 0, (flags & 0x8) != 0);
        }
    }
    let oper_path = format!("/sys/class/net/{}/operstate", ifname);
    let is_up = fs::read_to_string(oper_path).map(|s| s.trim() == "up").unwrap_or(false);
    (is_up, ifname == "lo")
}

#[cfg(not(windows))]
fn collect_linux_networks(
    pool: &mut Vec<Vec<u8>>,
    buf: &mut Vec<YukinoNetworkInterfaceInfo>,
) {
    let Ok(content) = fs::read_to_string("/proc/net/dev") else { return };
    for line in content.lines().skip(2) {
        let Some((name_part, stats_part)) = line.split_once(':') else { continue };
        let ifname = name_part.trim();
        if ifname.is_empty() { continue; }
        let stats: Vec<&str> = stats_part.split_whitespace().collect();
        if stats.len() < 16 { continue; }

        let rx_bytes = stats[0].parse::<u64>().unwrap_or(0);
        let rx_packets = stats[1].parse::<u64>().unwrap_or(0);
        let rx_errors = stats[2].parse::<u64>().unwrap_or(0);
        let tx_bytes = stats[8].parse::<u64>().unwrap_or(0);
        let tx_packets = stats[9].parse::<u64>().unwrap_or(0);
        let tx_errors = stats[10].parse::<u64>().unwrap_or(0);

        let mac = read_linux_iface_mac(ifname);
        let (is_up, is_loopback) = read_linux_iface_flags(ifname);

        let name_str = alloc_pool_str(pool, ifname);
        let mac_str = alloc_pool_str(pool, &mac);

        buf.push(YukinoNetworkInterfaceInfo {
            name: name_str,
            mac_address: mac_str,
            is_up,
            is_loopback,
            rx_bytes,
            tx_bytes,
            rx_packets,
            tx_packets,
            rx_errors,
            tx_errors,
        });
    }
}

#[cfg(windows)]
fn collect_windows_networks(
    _pool: &mut Vec<Vec<u8>>,
    _buf: &mut Vec<YukinoNetworkInterfaceInfo>,
) {
    // windows network collection placeholder
}

#[cfg(not(windows))]
fn collect_linux_thermal_zones(
    pool: &mut Vec<Vec<u8>>,
    buf: &mut Vec<YukinoThermalSensorInfo>,
) {
    let Ok(entries) = fs::read_dir("/sys/class/thermal") else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("thermal_zone") { continue; }
        let path = entry.path();
        let type_label = fs::read_to_string(path.join("type"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| name.clone());
        let Ok(temp_str) = fs::read_to_string(path.join("temp")) else { continue };
        let Ok(temp_raw) = temp_str.trim().parse::<f32>() else { continue };
        let temp_c = temp_raw / 1000.0;

        let crit_opt = fs::read_to_string(path.join("trip_point_0_temp"))
            .or_else(|_| fs::read_to_string(path.join("trip_point_1_temp")))
            .ok()
            .and_then(|s| s.trim().parse::<f32>().ok())
            .map(|raw| raw / 1000.0);

        let label_str = alloc_pool_str(pool, &type_label);
        buf.push(YukinoThermalSensorInfo {
            label: label_str,
            temperature_celsius: temp_c,
            has_critical: crit_opt.is_some(),
            critical_temp: crit_opt.unwrap_or(0.0),
        });
    }
}

#[cfg(not(windows))]
fn collect_linux_hwmon(
    pool: &mut Vec<Vec<u8>>,
    buf: &mut Vec<YukinoThermalSensorInfo>,
) {
    let Ok(entries) = fs::read_dir("/sys/class/hwmon") else { return };
    for entry in entries.flatten() {
        let hwmon_path = entry.path();
        let base_name = fs::read_to_string(hwmon_path.join("name"))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let Ok(sub_entries) = fs::read_dir(&hwmon_path) else { continue };

        for sub in sub_entries.flatten() {
            let fname = sub.file_name().to_string_lossy().to_string();
            if !fname.starts_with("temp") || !fname.ends_with("_input") {
                continue;
            }
            let prefix = &fname[..fname.len() - 6];
            let Ok(temp_str) = fs::read_to_string(sub.path()) else { continue };
            let Ok(raw_temp) = temp_str.trim().parse::<f32>() else { continue };
            let temp_c = raw_temp / 1000.0;

            let label_file = hwmon_path.join(format!("{prefix}_label"));
            let label = fs::read_to_string(label_file)
                .map(|s| format!("{} {}", base_name, s.trim()).trim().to_string())
                .unwrap_or_else(|_| format!("{base_name} {prefix}").trim().to_string());

            let crit_file = hwmon_path.join(format!("{prefix}_crit"));
            let max_file = hwmon_path.join(format!("{prefix}_max"));
            let crit_opt = fs::read_to_string(crit_file)
                .or_else(|_| fs::read_to_string(max_file))
                .ok()
                .and_then(|s| s.trim().parse::<f32>().ok())
                .map(|raw| raw / 1000.0);

            let label_str = alloc_pool_str(pool, &label);
            buf.push(YukinoThermalSensorInfo {
                label: label_str,
                temperature_celsius: temp_c,
                has_critical: crit_opt.is_some(),
                critical_temp: crit_opt.unwrap_or(0.0),
            });
        }
    }
}

#[cfg(not(windows))]
fn parse_linux_proc_stat(stat_content: &str) -> Option<(String, u32, u64, u64)> {
    let open_paren = stat_content.find('(')?;
    let close_paren = stat_content.rfind(')')?;
    if close_paren <= open_paren {
        return None;
    }
    let name = stat_content[open_paren + 1..close_paren].to_string();
    let rest = &stat_content[close_paren + 1..];
    let fields: Vec<&str> = rest.split_whitespace().collect();
    if fields.len() < 22 {
        return None;
    }
    let ppid = fields[1].parse::<u32>().unwrap_or(0);
    let vsize = fields[20].parse::<u64>().unwrap_or(0);
    let rss_pages = fields[21].parse::<u64>().unwrap_or(0);
    let rss = rss_pages.saturating_mul(4096);
    Some((name, ppid, vsize, rss))
}

#[cfg(not(windows))]
fn collect_linux_processes(
    pool: &mut Vec<Vec<u8>>,
    buf: &mut Vec<YukinoProcessInfo>,
) {
    let Ok(entries) = fs::read_dir("/proc") else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(pid) = name.parse::<u32>() else { continue };
        let proc_path = entry.path();

        let stat_file = proc_path.join("stat");
        let Ok(stat_content) = fs::read_to_string(stat_file) else { continue };
        let Some((proc_name, ppid, vsize, rss)) = parse_linux_proc_stat(&stat_content) else {
            continue;
        };

        let cmdline_file = proc_path.join("cmdline");
        let cmdline = match fs::read(cmdline_file) {
            Ok(bytes) if !bytes.is_empty() => {
                let replaced: Vec<u8> = bytes
                    .into_iter()
                    .map(|b| if b == 0 { b' ' } else { b })
                    .collect();
                String::from_utf8_lossy(&replaced).trim().to_string()
            }
            _ => proc_name.clone(),
        };

        let name_str = alloc_pool_str(pool, &proc_name);
        let cmdline_str = alloc_pool_str(pool, &cmdline);

        buf.push(YukinoProcessInfo {
            pid,
            parent_pid: ppid,
            name: name_str,
            cmdline: cmdline_str,
            cpu_usage: 0.0,
            memory_rss: rss,
            memory_virtual: vsize,
        });
    }
}

#[cfg(windows)]
unsafe extern "system" {
    fn CreateToolhelp32Snapshot(dw_flags: u32, th32_process_id: u32) -> *mut std::ffi::c_void;
    fn Process32FirstW(h_snapshot: *mut std::ffi::c_void, lppe: *mut ProcessEntry32W) -> i32;
    fn Process32NextW(h_snapshot: *mut std::ffi::c_void, lppe: *mut ProcessEntry32W) -> i32;
    fn CloseHandle(h_object: *mut std::ffi::c_void) -> i32;
}

#[cfg(windows)]
#[repr(C)]
struct ProcessEntry32W {
    dw_size: u32,
    cnt_usage: u32,
    th32_process_id: u32,
    th32_default_heap_id: usize,
    th32_module_id: u32,
    cnt_threads: u32,
    th32_parent_process_id: u32,
    pc_pri_class_base: i32,
    dw_flags: u32,
    sz_exe_file: [u16; 260],
}

#[cfg(windows)]
fn collect_windows_processes(
    pool: &mut Vec<Vec<u8>>,
    buf: &mut Vec<YukinoProcessInfo>,
) {
    let snapshot = unsafe { CreateToolhelp32Snapshot(0x00000002, 0) };
    if snapshot.is_null() || snapshot == (!0usize) as *mut std::ffi::c_void {
        return;
    }
    let mut entry = ProcessEntry32W {
        dw_size: std::mem::size_of::<ProcessEntry32W>() as u32,
        cnt_usage: 0,
        th32_process_id: 0,
        th32_default_heap_id: 0,
        th32_module_id: 0,
        cnt_threads: 0,
        th32_parent_process_id: 0,
        pc_pri_class_base: 0,
        dw_flags: 0,
        sz_exe_file: [0u16; 260],
    };
    if unsafe { Process32FirstW(snapshot, &mut entry) } != 0 {
        loop {
            let pid = entry.th32_process_id;
            let ppid = entry.th32_parent_process_id;
            let name_len = entry.sz_exe_file.iter().position(|&c| c == 0).unwrap_or(260);
            let exe_name = String::from_utf16_lossy(&entry.sz_exe_file[..name_len]);

            let name_str = alloc_pool_str(pool, &exe_name);
            let cmdline_str = alloc_pool_str(pool, &exe_name);

            buf.push(YukinoProcessInfo {
                pid,
                parent_pid: ppid,
                name: name_str,
                cmdline: cmdline_str,
                cpu_usage: 0.0,
                memory_rss: 0,
                memory_virtual: 0,
            });

            if unsafe { Process32NextW(snapshot, &mut entry) } == 0 {
                break;
            }
        }
    }
    unsafe { CloseHandle(snapshot); }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_sysinfox_network_interfaces(
    out_count: *mut i64,
) -> *const YukinoNetworkInterfaceInfo {
    NET_STR_POOL.with(|pool_cell| {
        NET_INFO_BUF.with(|buf_cell| {
            let mut pool = pool_cell.borrow_mut();
            let mut buf = buf_cell.borrow_mut();
            pool.clear();
            buf.clear();

            #[cfg(windows)]
            collect_windows_networks(&mut pool, &mut buf);

            #[cfg(not(windows))]
            collect_linux_networks(&mut pool, &mut buf);

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
pub extern "C" fn yukistd_sysinfox_thermal_sensors(
    out_count: *mut i64,
) -> *const YukinoThermalSensorInfo {
    THERMAL_STR_POOL.with(|pool_cell| {
        THERMAL_INFO_BUF.with(|buf_cell| {
            let mut pool = pool_cell.borrow_mut();
            let mut buf = buf_cell.borrow_mut();
            pool.clear();
            buf.clear();

            #[cfg(not(windows))]
            {
                collect_linux_thermal_zones(&mut pool, &mut buf);
                collect_linux_hwmon(&mut pool, &mut buf);
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
pub extern "C" fn yukistd_sysinfox_process_list(
    out_count: *mut i64,
) -> *const YukinoProcessInfo {
    PROC_STR_POOL.with(|pool_cell| {
        PROC_INFO_BUF.with(|buf_cell| {
            let mut pool = pool_cell.borrow_mut();
            let mut buf = buf_cell.borrow_mut();
            pool.clear();
            buf.clear();

            #[cfg(windows)]
            collect_windows_processes(&mut pool, &mut buf);

            #[cfg(not(windows))]
            collect_linux_processes(&mut pool, &mut buf);

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
