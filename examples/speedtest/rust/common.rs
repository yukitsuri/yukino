use std::io::Write;
use std::process;

pub const MIX_SEED: u64 = 0xcbf29ce484222325;
pub const RNG_SEED: u64 = 0x9e3779b97f4a7c15;
const MIX_PRIME: u64 = 0x100000001b3;
const RNG_MUL: u64 = 0x2545f4914f6cdd1d;
const RNG_SHIFT_A: u32 = 12;
const RNG_SHIFT_B: u32 = 25;
const RNG_SHIFT_C: u32 = 27;
const KEY31_SHIFT: u32 = 33;
const FILL_MUL: u32 = 167;
const FILL_SHIFT: u32 = 9;
const DECIMAL_BASE: u64 = 10;
const MAX_DIGITS: usize = 20;
const REPORT_BYTES: usize = 64;
const EXIT_WRITE_FAILED: i32 = 2;

pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng { state: seed }
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> RNG_SHIFT_A;
        x ^= x << RNG_SHIFT_B;
        x ^= x >> RNG_SHIFT_C;
        self.state = x;
        x.wrapping_mul(RNG_MUL)
    }

    pub fn key31(&mut self) -> u32 {
        (self.next() >> KEY31_SHIFT) as u32
    }

    pub fn below(&mut self, n: u64) -> u64 {
        (self.next() >> 1) % n
    }
}

pub fn mix(h: u64, v: u64) -> u64 {
    (h ^ v).wrapping_mul(MIX_PRIME)
}

pub fn fill_byte(i: u32) -> u8 {
    i.wrapping_mul(FILL_MUL).wrapping_add(i >> FILL_SHIFT) as u8
}

pub fn write_out(buf: &[u8]) {
    if std::io::stdout().write_all(buf).is_err() {
        process::exit(EXIT_WRITE_FAILED);
    }
}

pub fn fmt_u64(mut v: u64, out: &mut [u8]) -> usize {
    let mut tmp = [0u8; MAX_DIGITS];
    let mut n = 0;
    loop {
        tmp[n] = b'0' + (v % DECIMAL_BASE) as u8;
        n += 1;
        v /= DECIMAL_BASE;
        if v == 0 {
            break;
        }
    }
    for i in 0..n {
        out[i] = tmp[n - 1 - i];
    }
    n
}

pub fn report(checksum: u64, ns: u64) {
    let mut line = [0u8; REPORT_BYTES];
    let mut n = 0;
    line[n] = b'R';
    line[n + 1] = b' ';
    n += 2;
    n += fmt_u64(checksum, &mut line[n..]);
    line[n] = b' ';
    n += 1;
    n += fmt_u64(ns, &mut line[n..]);
    line[n] = b'\n';
    n += 1;
    if std::io::stderr().write_all(&line[..n]).is_err() {
        process::exit(EXIT_WRITE_FAILED);
    }
}
