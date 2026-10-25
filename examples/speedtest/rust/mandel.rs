use crate::common::{mix, MIX_SEED};

const MANDEL_SIZE: i64 = 1000;
const MANDEL_MAX_ITER: u64 = 500;
const MANDEL_FRAC: u32 = 24;

fn iterations(cr: i64, ci: i64) -> u64 {
    let limit: i64 = 4 << MANDEL_FRAC;
    let mut zr: i64 = 0;
    let mut zi: i64 = 0;
    let mut n: u64 = 0;
    while n < MANDEL_MAX_ITER {
        let zr2 = (zr * zr) >> MANDEL_FRAC;
        let zi2 = (zi * zi) >> MANDEL_FRAC;
        if zr2 + zi2 > limit {
            break;
        }
        zi = ((2 * zr * zi) >> MANDEL_FRAC) + ci;
        zr = zr2 - zi2 + cr;
        n += 1;
    }
    n
}

pub fn mandel() -> u64 {
    let one: i64 = 1 << MANDEL_FRAC;
    let x0 = -2 * one;
    let y0 = -3 * one / 2;
    let step = 3 * one / MANDEL_SIZE;
    let mut total: u64 = 0;
    for py in 0..MANDEL_SIZE {
        for px in 0..MANDEL_SIZE {
            total += iterations(x0 + px * step, y0 + py * step);
        }
    }
    mix(MIX_SEED, total)
}
