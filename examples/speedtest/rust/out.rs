use crate::common::{fmt_u64, mix, write_out, Rng, MIX_SEED, RNG_SEED};

const OUT_BUF_SIZE: usize = 65536;
const OUT_LINES: u64 = 3_000_000;
const OUT_SINGLE_LINES: u64 = 300_000;
const MAX_LINE_BYTES: usize = 64;

fn fmt_line(dst: &mut [u8], index: u64, value: u64) -> usize {
    let mut n = fmt_u64(index, dst);
    dst[n] = b' ';
    n += 1;
    n += fmt_u64(value, &mut dst[n..]);
    dst[n] = b'\n';
    n + 1
}

pub fn out_buf() -> u64 {
    let mut r = Rng::new(RNG_SEED);
    let mut buf = vec![0u8; OUT_BUF_SIZE];
    let mut used: usize = 0;
    let mut total: u64 = 0;
    for i in 0..OUT_LINES {
        if used + MAX_LINE_BYTES > OUT_BUF_SIZE {
            write_out(&buf[..used]);
            total += used as u64;
            used = 0;
        }
        used += fmt_line(&mut buf[used..], i, r.next());
    }
    write_out(&buf[..used]);
    mix(MIX_SEED, total + used as u64)
}

pub fn out_line() -> u64 {
    let mut r = Rng::new(RNG_SEED);
    let mut line = [0u8; MAX_LINE_BYTES];
    let mut total: u64 = 0;
    for i in 0..OUT_SINGLE_LINES {
        let n = fmt_line(&mut line, i, r.next());
        write_out(&line[..n]);
        total += n as u64;
    }
    mix(MIX_SEED, total)
}
