use crate::common::{mix, MIX_SEED};

const SIEVE_LIMIT: usize = 40_000_000;
const COLLATZ_LIMIT: u64 = 1_500_000;
const FIB_N: u64 = 42;
const QUEENS_N: u32 = 14;
const LEIBNIZ_TERMS: u64 = 150_000_000;
const LEIBNIZ_SCALE: f64 = 4.0e9;

pub fn sieve() -> u64 {
    let mut composite = vec![0u8; SIEVE_LIMIT + 1];
    let mut i = 2;
    while i * i <= SIEVE_LIMIT {
        if composite[i] == 0 {
            let mut j = i * i;
            while j <= SIEVE_LIMIT {
                composite[j] = 1;
                j += i;
            }
        }
        i += 1;
    }
    let mut count: u64 = 0;
    for i in 2..=SIEVE_LIMIT {
        count += (composite[i] == 0) as u64;
    }
    mix(MIX_SEED, count)
}

pub fn collatz() -> u64 {
    let mut best_start: u64 = 0;
    let mut best_steps: u64 = 0;
    for start in 1..=COLLATZ_LIMIT {
        let mut n = start;
        let mut steps: u64 = 0;
        while n != 1 {
            n = if n & 1 == 1 { 3 * n + 1 } else { n >> 1 };
            steps += 1;
        }
        if steps > best_steps {
            best_steps = steps;
            best_start = start;
        }
    }
    mix(mix(MIX_SEED, best_start), best_steps)
}

fn fib(n: u64) -> u64 {
    if n < 2 {
        n
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

pub fn fib_test() -> u64 {
    mix(MIX_SEED, fib(FIB_N))
}

fn solve_queens(cols: u32, diag_l: u32, diag_r: u32, full: u32) -> u64 {
    if cols == full {
        return 1;
    }
    let mut count: u64 = 0;
    let mut avail = full & !(cols | diag_l | diag_r);
    while avail != 0 {
        let bit = avail & (!avail).wrapping_add(1);
        avail ^= bit;
        count += solve_queens(cols | bit, ((diag_l | bit) << 1) & full, (diag_r | bit) >> 1, full);
    }
    count
}

pub fn queens() -> u64 {
    let full: u32 = (1u32 << QUEENS_N) - 1;
    mix(MIX_SEED, solve_queens(0, 0, 0, full))
}

pub fn leibniz() -> u64 {
    let mut sum: f64 = 0.0;
    let mut sign: f64 = 1.0;
    for k in 0..LEIBNIZ_TERMS {
        sum += sign / ((2 * k + 1) as f64);
        sign = -sign;
    }
    mix(MIX_SEED, (sum * LEIBNIZ_SCALE) as u64)
}
