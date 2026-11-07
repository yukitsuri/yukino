use std::cell::RefCell;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct YukinoString {
    pub ptr: *const u8,
    pub len: i64,
}

thread_local! {
    static LOCALE_STR_RING: RefCell<(usize, Vec<Vec<u8>>)> = const { RefCell::new((0, Vec::new())) };
    static PREFERRED_ENTRIES: RefCell<Vec<YukinoString>> = const { RefCell::new(Vec::new()) };
    static PREFERRED_POOL: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn set_str_result(bytes: &[u8], out_len: *mut i64) -> *const u8 {
    LOCALE_STR_RING.with(|cell| {
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
            unsafe {
                *out_len = buf.len() as i64;
            }
        }
        buf.as_ptr()
    })
}

fn normalize_locale(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() || s == "C" || s == "POSIX" {
        return "en_US".to_string();
    }
    let s = match s.split_once('@') {
        Some((b, _)) => b,
        None => s,
    };
    let s = match s.split_once('.') {
        Some((b, _)) => b,
        None => s,
    };
    let s = s.replace('-', "_");
    if s.is_empty() {
        "en_US".to_string()
    } else {
        s
    }
}

fn extract_language_code(tag: &str) -> String {
    let s = tag.trim();
    let s = match s.split_once('@') {
        Some((b, _)) => b,
        None => s,
    };
    let s = match s.split_once('.') {
        Some((b, _)) => b,
        None => s,
    };
    let lang = match s.split_once('_') {
        Some((b, _)) => b,
        None => match s.split_once('-') {
            Some((b, _)) => b,
            None => s,
        },
    };
    lang.trim().to_ascii_lowercase()
}

#[cfg(unix)]
fn detect_locale() -> String {
    for var in &["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            let val = val.trim();
            if !val.is_empty() && val != "C" && val != "POSIX" {
                return normalize_locale(val);
            }
        }
    }
    if let Ok(content) = std::fs::read_to_string("/etc/locale.conf") {
        for line in content.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("LANG=") {
                let clean = rest.trim_matches('"').trim_matches('\'').trim();
                if !clean.is_empty() && clean != "C" && clean != "POSIX" {
                    return normalize_locale(clean);
                }
            }
        }
    }
    if let Ok(content) = std::fs::read_to_string("/etc/default/locale") {
        for line in content.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("LANG=") {
                let clean = rest.trim_matches('"').trim_matches('\'').trim();
                if !clean.is_empty() && clean != "C" && clean != "POSIX" {
                    return normalize_locale(clean);
                }
            }
        }
    }
    "en_US".to_string()
}

#[cfg(windows)]
fn detect_locale() -> String {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    unsafe extern "system" {
        fn GetUserDefaultLocaleName(lpLocaleName: *mut u16, cchLocaleName: i32) -> i32;
    }

    let mut buf = [0u16; 85];
    let len = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    if len > 1 {
        let os_str = OsString::from_wide(&buf[..(len as usize - 1)]);
        if let Ok(s) = os_str.into_string() {
            return normalize_locale(&s);
        }
    }
    for var in &["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            let val = val.trim();
            if !val.is_empty() && val != "C" && val != "POSIX" {
                return normalize_locale(val);
            }
        }
    }
    "en_US".to_string()
}

#[cfg(not(any(unix, windows)))]
fn detect_locale() -> String {
    "en_US".to_string()
}

#[cfg(unix)]
fn detect_timezone() -> String {
    if let Ok(tz) = std::env::var("TZ") {
        let tz = tz.trim().trim_start_matches(':');
        if !tz.is_empty() {
            return tz.to_string();
        }
    }
    if let Ok(target) = std::fs::canonicalize("/etc/localtime") {
        let path_str = target.to_string_lossy();
        if let Some(idx) = path_str.find("/zoneinfo/") {
            let zone = &path_str[idx + "/zoneinfo/".len()..];
            if !zone.is_empty() {
                return zone.to_string();
            }
        }
    }
    if let Ok(content) = std::fs::read_to_string("/etc/timezone") {
        let tz = content.trim();
        if !tz.is_empty() {
            return tz.to_string();
        }
    }
    if let Ok(content) = std::fs::read_to_string("/etc/sysconfig/clock") {
        for line in content.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("ZONE=") {
                let clean = rest.trim_matches('"').trim_matches('\'').trim();
                if !clean.is_empty() {
                    return clean.to_string();
                }
            }
        }
    }
    "UTC".to_string()
}

#[cfg(windows)]
fn map_windows_tz_to_iana(name: &str) -> String {
    match name {
        "Russian Standard Time" => "Europe/Moscow".to_string(),
        "W. Europe Standard Time" => "Europe/Berlin".to_string(),
        "GMT Standard Time" => "Europe/London".to_string(),
        "Eastern Standard Time" => "America/New_York".to_string(),
        "Central Standard Time" => "America/Chicago".to_string(),
        "Mountain Standard Time" => "America/Denver".to_string(),
        "Pacific Standard Time" => "America/Los_Angeles".to_string(),
        "Tokyo Standard Time" => "Asia/Tokyo".to_string(),
        "China Standard Time" => "Asia/Shanghai".to_string(),
        "India Standard Time" => "Asia/Kolkata".to_string(),
        "UTC" => "UTC".to_string(),
        other if !other.is_empty() => other.to_string(),
        _ => "UTC".to_string(),
    }
}

#[cfg(windows)]
fn detect_timezone() -> String {
    if let Ok(tz) = std::env::var("TZ") {
        let tz = tz.trim().trim_start_matches(':');
        if !tz.is_empty() {
            return tz.to_string();
        }
    }
    #[repr(C)]
    struct DynamicTimeZoneInformation {
        bias: i32,
        standard_name: [u16; 32],
        standard_date: [u16; 8],
        standard_bias: i32,
        daylight_name: [u16; 32],
        daylight_date: [u16; 8],
        daylight_bias: i32,
        time_zone_key_name: [u16; 128],
        dynamic_daylight_time_disabled: u8,
    }
    unsafe extern "system" {
        fn GetDynamicTimeZoneInformation(info: *mut DynamicTimeZoneInformation) -> u32;
    }
    unsafe {
        let mut tz_info: DynamicTimeZoneInformation = std::mem::zeroed();
        let res = GetDynamicTimeZoneInformation(&mut tz_info);
        if res != 0xFFFFFFFF {
            let key_len = tz_info
                .time_zone_key_name
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(128);
            if key_len > 0 {
                use std::os::windows::ffi::OsStringExt;
                let os_str = std::ffi::OsString::from_wide(&tz_info.time_zone_key_name[..key_len]);
                if let Ok(name) = os_str.into_string() {
                    return map_windows_tz_to_iana(&name);
                }
            }
        }
    }
    "UTC".to_string()
}

#[cfg(not(any(unix, windows)))]
fn detect_timezone() -> String {
    "UTC".to_string()
}

#[cfg(unix)]
fn detect_preferred_languages() -> Vec<String> {
    let mut result = Vec::new();
    if let Ok(val) = std::env::var("LANGUAGE") {
        for item in val.split(':') {
            let item = item.trim();
            if !item.is_empty() {
                let norm = extract_language_code(item);
                if !norm.is_empty() && !result.contains(&norm) {
                    result.push(norm);
                }
            }
        }
    }
    let cur = extract_language_code(&detect_locale());
    if !cur.is_empty() && !result.contains(&cur) {
        result.push(cur);
    }
    if result.is_empty() {
        result.push("en".to_string());
    }
    result
}

#[cfg(windows)]
fn detect_preferred_languages() -> Vec<String> {
    let mut result = Vec::new();
    unsafe extern "system" {
        fn GetUserPreferredUILanguages(
            dwFlags: u32,
            pulNumLanguages: *mut u32,
            pwszLanguagesBuffer: *mut u16,
            pcchLanguagesBuffer: *mut u32,
        ) -> i32;
    }
    const MUI_LANGUAGE_NAME: u32 = 0x8;
    let mut count = 0u32;
    let mut buf_size = 0u32;
    unsafe {
        if GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut count,
            std::ptr::null_mut(),
            &mut buf_size,
        ) != 0
            && buf_size > 0
        {
            let mut buf = vec![0u16; buf_size as usize];
            if GetUserPreferredUILanguages(
                MUI_LANGUAGE_NAME,
                &mut count,
                buf.as_mut_ptr(),
                &mut buf_size,
            ) != 0
            {
                use std::os::windows::ffi::OsStringExt;
                let mut start = 0;
                for i in 0..buf.len() {
                    if buf[i] == 0 {
                        if i > start {
                            let os_str = std::ffi::OsString::from_wide(&buf[start..i]);
                            if let Ok(s) = os_str.into_string() {
                                let lang = extract_language_code(&s);
                                if !lang.is_empty() && !result.contains(&lang) {
                                    result.push(lang);
                                }
                            }
                        }
                        start = i + 1;
                    }
                }
            }
        }
    }
    if result.is_empty() {
        let cur = extract_language_code(&detect_locale());
        if !cur.is_empty() {
            result.push(cur);
        } else {
            result.push("en".to_string());
        }
    }
    result
}

#[cfg(not(any(unix, windows)))]
fn detect_preferred_languages() -> Vec<String> {
    vec!["en".to_string()]
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_locale_current_locale(out_len: *mut i64) -> *const u8 {
    let loc = detect_locale();
    set_str_result(loc.as_bytes(), out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_locale_system_timezone(out_len: *mut i64) -> *const u8 {
    let tz = detect_timezone();
    set_str_result(tz.as_bytes(), out_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_locale_preferred_languages(out_count: *mut i64) -> *const YukinoString {
    let langs = detect_preferred_languages();
    PREFERRED_POOL.with(|pool_cell| {
        PREFERRED_ENTRIES.with(|entries_cell| {
            let mut pool = pool_cell.borrow_mut();
            let mut entries = entries_cell.borrow_mut();
            pool.clear();
            entries.clear();

            for s in &langs {
                let bytes = s.as_bytes();
                let start_idx = pool.len();
                pool.extend_from_slice(bytes);
                let len = bytes.len() as i64;
                let ptr = unsafe { pool.as_ptr().add(start_idx) };
                entries.push(YukinoString { ptr, len });
            }

            let base_ptr = pool.as_ptr();
            let mut offset = 0;
            for item in entries.iter_mut() {
                item.ptr = unsafe { base_ptr.add(offset) };
                offset += item.len as usize;
            }

            if !out_count.is_null() {
                unsafe { *out_count = entries.len() as i64; }
            }

            entries.as_ptr()
        })
    })
}
