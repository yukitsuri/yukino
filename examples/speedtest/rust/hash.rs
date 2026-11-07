use crate::common::{fill_byte, mix, MIX_SEED};

const HASH_BUF_BYTES: usize = 1 << 22;
const SHA_PASSES: usize = 16;
const CRC_PASSES: usize = 32;
const FNV_PASSES: usize = 64;
const SHA_BLOCK_BYTES: usize = 64;
const SHA_ROUNDS: usize = 64;
const CRC_POLY: u32 = 0xedb88320;
const SHA_INIT: [u32; 8] = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
const SHA_K: [u32; SHA_ROUNDS] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn make_buf() -> Vec<u8> {
    let mut buf = vec![0u8; HASH_BUF_BYTES];
    for i in 0..HASH_BUF_BYTES {
        buf[i] = fill_byte(i as u32);
    }
    buf
}

fn ror(x: u32, n: u32) -> u32 {
    (x >> n) | (x << (32 - n))
}

fn sha_block(st: &mut [u32; 8], p: &[u8]) {
    let mut w = [0u32; SHA_ROUNDS];
    for i in 0..16 {
        w[i] = ((p[4 * i] as u32) << 24) | ((p[4 * i + 1] as u32) << 16) | ((p[4 * i + 2] as u32) << 8) | (p[4 * i + 3] as u32);
    }
    for i in 16..SHA_ROUNDS {
        let s0 = ror(w[i - 15], 7) ^ ror(w[i - 15], 18) ^ (w[i - 15] >> 3);
        let s1 = ror(w[i - 2], 17) ^ ror(w[i - 2], 19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
    }
    let mut a = st[0];
    let mut b = st[1];
    let mut c = st[2];
    let mut d = st[3];
    let mut e = st[4];
    let mut f = st[5];
    let mut g = st[6];
    let mut h = st[7];
    for i in 0..SHA_ROUNDS {
        let t1 = h
            .wrapping_add(ror(e, 6) ^ ror(e, 11) ^ ror(e, 25))
            .wrapping_add((e & f) ^ (!e & g))
            .wrapping_add(SHA_K[i])
            .wrapping_add(w[i]);
        let t2 = (ror(a, 2) ^ ror(a, 13) ^ ror(a, 22)).wrapping_add((a & b) ^ (a & c) ^ (b & c));
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    st[0] = st[0].wrapping_add(a);
    st[1] = st[1].wrapping_add(b);
    st[2] = st[2].wrapping_add(c);
    st[3] = st[3].wrapping_add(d);
    st[4] = st[4].wrapping_add(e);
    st[5] = st[5].wrapping_add(f);
    st[6] = st[6].wrapping_add(g);
    st[7] = st[7].wrapping_add(h);
}

pub fn sha256() -> u64 {
    let buf = make_buf();
    let mut st = SHA_INIT;
    for _ in 0..SHA_PASSES {
        let mut off = 0;
        while off < HASH_BUF_BYTES {
            sha_block(&mut st, &buf[off..off + SHA_BLOCK_BYTES]);
            off += SHA_BLOCK_BYTES;
        }
    }
    let mut last = [0u8; SHA_BLOCK_BYTES];
    let bits = (SHA_PASSES * HASH_BUF_BYTES * 8) as u64;
    last[0] = 0x80;
    for k in 0..8 {
        last[SHA_BLOCK_BYTES - 1 - k] = (bits >> (8 * k)) as u8;
    }
    sha_block(&mut st, &last);
    let mut h = MIX_SEED;
    for i in 0..8 {
        h = mix(h, st[i] as u64);
    }
    h
}

pub fn crc32() -> u64 {
    let buf = make_buf();
    let mut table = [0u32; 256];
    for n in 0..256 {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 { (c >> 1) ^ CRC_POLY } else { c >> 1 };
        }
        table[n] = c;
    }
    let mut crc: u32 = 0xffffffff;
    for _ in 0..CRC_PASSES {
        for i in 0..HASH_BUF_BYTES {
            crc = table[((crc ^ buf[i] as u32) & 0xff) as usize] ^ (crc >> 8);
        }
    }
    mix(MIX_SEED, (!crc) as u64)
}

pub fn fnv() -> u64 {
    let buf = make_buf();
    let mut h = MIX_SEED;
    for _ in 0..FNV_PASSES {
        for i in 0..HASH_BUF_BYTES {
            h = mix(h, buf[i] as u64);
        }
    }
    h
}
