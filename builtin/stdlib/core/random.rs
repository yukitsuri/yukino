use std::fs::File;
use std::io::Read;
use std::sync::atomic::{AtomicU64, Ordering};

static FALLBACK_COUNTER: AtomicU64 = AtomicU64::new(0xda942042e4dd58b5);
static GLOBAL_RNG_STATE: AtomicU64 = AtomicU64::new(0);

fn fallback_entropy() -> u64 {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0) as u64;
    let c = FALLBACK_COUNTER.fetch_add(0x9e3779b97f4a7c15, Ordering::Relaxed);
    let mut x = t ^ c ^ (&c as *const _ as usize as u64);
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58476d1ce4e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d049bb133111eb);
    x ^= x >> 31;
    x
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_random_system_entropy() -> u64 {
    #[cfg(unix)]
    {
        let mut buf = [0u8; 8];
        if let Ok(mut f) = File::open("/dev/urandom") {
            if f.read_exact(&mut buf).is_ok() {
                return u64::from_ne_bytes(buf);
            }
        }
    }
    fallback_entropy()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_random_system_bytes(buf: *mut u8, len: i64) {
    if buf.is_null() || len <= 0 {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(buf, len as usize) };
    #[cfg(unix)]
    {
        if let Ok(mut f) = File::open("/dev/urandom") {
            if f.read_exact(slice).is_ok() {
                return;
            }
        }
    }
    let mut s = fallback_entropy();
    for byte in slice.iter_mut() {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *byte = (s >> 32) as u8;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_random_global_u64() -> u64 {
    let mut s = GLOBAL_RNG_STATE.load(Ordering::Relaxed);
    if s == 0 {
        s = yukistd_random_system_entropy();
        if s == 0 {
            s = 0x853c49e6748fea9b;
        }
        let _ = GLOBAL_RNG_STATE.compare_exchange(0, s, Ordering::Relaxed, Ordering::Relaxed);
    }
    let prev = GLOBAL_RNG_STATE.fetch_add(0x9e3779b97f4a7c15, Ordering::Relaxed);
    let mut z = prev.wrapping_add(0x9e3779b97f4a7c15);
    z ^= z >> 30;
    z = z.wrapping_mul(0xbf58476d1ce4e5b9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_random_global_seed(seed: u64) {
    let actual_seed = if seed == 0 { 0x853c49e6748fea9b } else { seed };
    GLOBAL_RNG_STATE.store(actual_seed, Ordering::Relaxed);
}
