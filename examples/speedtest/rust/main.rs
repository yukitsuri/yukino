mod bf;
mod bigfact;
mod chacha;
mod common;
mod hash;
mod hashmap;
mod mandel;
mod matrix;
mod mem;
mod numeric;
mod out;
mod sort;

use std::process;
use std::time::Instant;

const EXIT_USAGE: i32 = 64;
const EXIT_UNKNOWN: i32 = 65;

struct Test {
    name: &'static str,
    run: fn() -> u64,
}

fn test_nop() -> u64 {
    0
}

const TESTS: [Test; 22] = [
    Test { name: "nop", run: test_nop },
    Test { name: "sieve", run: numeric::sieve },
    Test { name: "collatz", run: numeric::collatz },
    Test { name: "fib", run: numeric::fib_test },
    Test { name: "queens", run: numeric::queens },
    Test { name: "leibniz", run: numeric::leibniz },
    Test { name: "matmul", run: matrix::matmul },
    Test { name: "spectral", run: matrix::spectral },
    Test { name: "mandel", run: mandel::mandel },
    Test { name: "sort", run: sort::sort },
    Test { name: "hashmap", run: hashmap::hashmap },
    Test { name: "sha256", run: hash::sha256 },
    Test { name: "crc32", run: hash::crc32 },
    Test { name: "fnv", run: hash::fnv },
    Test { name: "chacha", run: chacha::chacha },
    Test { name: "bigfact", run: bigfact::bigfact },
    Test { name: "bf", run: bf::bf },
    Test { name: "memseq", run: mem::mem_seq },
    Test { name: "memrand", run: mem::mem_rand },
    Test { name: "memtree", run: mem::mem_tree },
    Test { name: "outbuf", run: out::out_buf },
    Test { name: "outline", run: out::out_line },
];

fn main() {
    let name = match std::env::args().nth(1) {
        Some(n) => n,
        None => process::exit(EXIT_USAGE),
    };
    for t in TESTS.iter() {
        if t.name != name {
            continue;
        }
        let start = Instant::now();
        let sum = (t.run)();
        common::report(sum, start.elapsed().as_nanos() as u64);
        return;
    }
    process::exit(EXIT_UNKNOWN);
}
