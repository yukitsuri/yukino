const common = @import("common.zig");

const hashBufBytes: u32 = 1 << 22;
const shaPasses: u64 = 16;
const crcPasses = 32;
const fnvPasses = 64;
const shaBlockBytes = 64;
const shaRounds = 64;
const crcPoly: u32 = 0xedb88320;
const shaInit = [8]u32{ 0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19 };
const shaK = [shaRounds]u32{
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
};

var hashBuf: [hashBufBytes]u8 = undefined;

fn fillBuf() void {
    for (0..hashBufBytes) |i| hashBuf[i] = common.fillByte(@intCast(i));
}

fn ror(x: u32, comptime n: u32) u32 {
    return (x >> n) | (x << (32 - n));
}

fn shaBlock(st: *[8]u32, p: *const [shaBlockBytes]u8) void {
    var w: [shaRounds]u32 = undefined;
    for (0..16) |i| {
        w[i] = (@as(u32, p[4 * i]) << 24) | (@as(u32, p[4 * i + 1]) << 16) | (@as(u32, p[4 * i + 2]) << 8) | @as(u32, p[4 * i + 3]);
    }
    for (16..shaRounds) |i| {
        const s0 = ror(w[i - 15], 7) ^ ror(w[i - 15], 18) ^ (w[i - 15] >> 3);
        const s1 = ror(w[i - 2], 17) ^ ror(w[i - 2], 19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16] +% s0 +% w[i - 7] +% s1;
    }
    var a = st[0];
    var b = st[1];
    var c = st[2];
    var d = st[3];
    var e = st[4];
    var f = st[5];
    var g = st[6];
    var h = st[7];
    for (0..shaRounds) |i| {
        const t1 = h +% (ror(e, 6) ^ ror(e, 11) ^ ror(e, 25)) +% ((e & f) ^ (~e & g)) +% shaK[i] +% w[i];
        const t2 = (ror(a, 2) ^ ror(a, 13) ^ ror(a, 22)) +% ((a & b) ^ (a & c) ^ (b & c));
        h = g;
        g = f;
        f = e;
        e = d +% t1;
        d = c;
        c = b;
        b = a;
        a = t1 +% t2;
    }
    st[0] +%= a;
    st[1] +%= b;
    st[2] +%= c;
    st[3] +%= d;
    st[4] +%= e;
    st[5] +%= f;
    st[6] +%= g;
    st[7] +%= h;
}

pub fn sha256() u64 {
    fillBuf();
    var st = shaInit;
    for (0..shaPasses) |_| {
        var off: usize = 0;
        while (off < hashBufBytes) : (off += shaBlockBytes) shaBlock(&st, hashBuf[off..][0..shaBlockBytes]);
    }
    var last: [shaBlockBytes]u8 = @splat(0);
    const bits: u64 = shaPasses * hashBufBytes * 8;
    last[0] = 0x80;
    for (0..8) |k| last[shaBlockBytes - 1 - k] = @truncate(bits >> @intCast(8 * k));
    shaBlock(&st, &last);
    var h = common.mixSeed;
    for (0..8) |i| h = common.mix(h, st[i]);
    return h;
}

pub fn crc32() u64 {
    fillBuf();
    var table: [256]u32 = undefined;
    for (0..256) |n| {
        var c: u32 = @intCast(n);
        for (0..8) |_| c = if ((c & 1) == 1) (c >> 1) ^ crcPoly else c >> 1;
        table[n] = c;
    }
    var crc: u32 = 0xffffffff;
    for (0..crcPasses) |_| {
        for (0..hashBufBytes) |i| crc = table[(crc ^ hashBuf[i]) & 0xff] ^ (crc >> 8);
    }
    return common.mix(common.mixSeed, ~crc);
}

pub fn fnv() u64 {
    fillBuf();
    var h = common.mixSeed;
    for (0..fnvPasses) |_| {
        for (0..hashBufBytes) |i| h = common.mix(h, hashBuf[i]);
    }
    return h;
}
