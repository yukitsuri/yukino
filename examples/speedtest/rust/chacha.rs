use crate::common::{mix, Rng, MIX_SEED, RNG_SEED};

const CHACHA_BLOCKS: u32 = 2_000_000;
const CHACHA_WORDS: usize = 16;
const CHACHA_DOUBLE_ROUNDS: u32 = 10;
const CHACHA_CONST: [u32; 4] = [0x61707865, 0x3320646e, 0x79622d32, 0x6b206574];

fn rol(x: u32, n: u32) -> u32 {
    (x << n) | (x >> (32 - n))
}

fn quarter(s: &mut [u32; CHACHA_WORDS], a: usize, b: usize, c: usize, d: usize) {
    s[a] = s[a].wrapping_add(s[b]);
    s[d] = rol(s[d] ^ s[a], 16);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = rol(s[b] ^ s[c], 12);
    s[a] = s[a].wrapping_add(s[b]);
    s[d] = rol(s[d] ^ s[a], 8);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = rol(s[b] ^ s[c], 7);
}

fn chacha_block(input: &[u32; CHACHA_WORDS], out: &mut [u32; CHACHA_WORDS]) {
    *out = *input;
    for _ in 0..CHACHA_DOUBLE_ROUNDS {
        quarter(out, 0, 4, 8, 12);
        quarter(out, 1, 5, 9, 13);
        quarter(out, 2, 6, 10, 14);
        quarter(out, 3, 7, 11, 15);
        quarter(out, 0, 5, 10, 15);
        quarter(out, 1, 6, 11, 12);
        quarter(out, 2, 7, 8, 13);
        quarter(out, 3, 4, 9, 14);
    }
    for i in 0..CHACHA_WORDS {
        out[i] = out[i].wrapping_add(input[i]);
    }
}

pub fn chacha() -> u64 {
    let mut st = [0u32; CHACHA_WORDS];
    let mut out = [0u32; CHACHA_WORDS];
    let mut r = Rng::new(RNG_SEED);
    for i in 0..4 {
        st[i] = CHACHA_CONST[i];
    }
    for i in 4..CHACHA_WORDS {
        st[i] = r.next() as u32;
    }
    let mut acc: u32 = 0;
    for c in 0..CHACHA_BLOCKS {
        st[12] = c;
        chacha_block(&st, &mut out);
        for i in 0..CHACHA_WORDS {
            acc ^= out[i];
        }
    }
    mix(MIX_SEED, acc as u64)
}
