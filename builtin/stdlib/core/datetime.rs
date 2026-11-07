use std::sync::LazyLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

static STEADY_START: LazyLock<Instant> = LazyLock::new(Instant::now);

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_datetime_now_epoch_nanos() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_nanos() as i64,
        Err(e) => -(e.duration().as_nanos() as i64),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_datetime_steady_nanos() -> i64 {
    STEADY_START.elapsed().as_nanos() as i64
}

#[cfg(unix)]
unsafe extern "C" {
    fn time(timer: *mut i64) -> i64;
    fn localtime_r(timep: *const i64, result: *mut Tm) -> *mut Tm;
}

#[cfg(unix)]
#[repr(C)]
struct Tm {
    tm_sec: i32,
    tm_min: i32,
    tm_hour: i32,
    tm_mday: i32,
    tm_mon: i32,
    tm_year: i32,
    tm_wday: i32,
    tm_yday: i32,
    tm_isdst: i32,
    tm_gmtoff: i64,
    tm_zone: *const u8,
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_datetime_local_offset_secs() -> i32 {
    #[cfg(unix)]
    unsafe {
        let mut t: i64 = 0;
        time(&raw mut t);
        let mut out: Tm = std::mem::zeroed();
        if !localtime_r(&raw const t, &raw mut out).is_null() {
            return out.tm_gmtoff as i32;
        }
        0
    }
    #[cfg(windows)]
    unsafe {
        unsafe extern "C" {
            fn _get_timezone(seconds: *mut i64) -> i32;
            fn _get_daylight(hours: *mut i32) -> i32;
        }
        let mut tz: i64 = 0;
        let mut dl: i32 = 0;
        _get_timezone(&raw mut tz);
        _get_daylight(&raw mut dl);
        let mut offset = -tz;
        if dl != 0 {
            offset += 3600;
        }
        offset as i32
    }
    #[cfg(not(any(unix, windows)))]
    {
        0
    }
}
