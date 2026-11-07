#include "tests.h"

enum { hashBufBytes = 1 << 22, shaPasses = 16, crcPasses = 32, fnvPasses = 64, shaBlockBytes = 64, shaRounds = 64 };

static const u32 crcPoly = 0xedb88320u;
static const u32 shaInit[8] = {0x6a09e667u, 0xbb67ae85u, 0x3c6ef372u, 0xa54ff53au, 0x510e527fu, 0x9b05688cu, 0x1f83d9abu, 0x5be0cd19u};
static const u32 shaK[shaRounds] = {
    0x428a2f98u, 0x71374491u, 0xb5c0fbcfu, 0xe9b5dba5u, 0x3956c25bu, 0x59f111f1u, 0x923f82a4u, 0xab1c5ed5u,
    0xd807aa98u, 0x12835b01u, 0x243185beu, 0x550c7dc3u, 0x72be5d74u, 0x80deb1feu, 0x9bdc06a7u, 0xc19bf174u,
    0xe49b69c1u, 0xefbe4786u, 0x0fc19dc6u, 0x240ca1ccu, 0x2de92c6fu, 0x4a7484aau, 0x5cb0a9dcu, 0x76f988dau,
    0x983e5152u, 0xa831c66du, 0xb00327c8u, 0xbf597fc7u, 0xc6e00bf3u, 0xd5a79147u, 0x06ca6351u, 0x14292967u,
    0x27b70a85u, 0x2e1b2138u, 0x4d2c6dfcu, 0x53380d13u, 0x650a7354u, 0x766a0abbu, 0x81c2c92eu, 0x92722c85u,
    0xa2bfe8a1u, 0xa81a664bu, 0xc24b8b70u, 0xc76c51a3u, 0xd192e819u, 0xd6990624u, 0xf40e3585u, 0x106aa070u,
    0x19a4c116u, 0x1e376c08u, 0x2748774cu, 0x34b0bcb5u, 0x391c0cb3u, 0x4ed8aa4au, 0x5b9cca4fu, 0x682e6ff3u,
    0x748f82eeu, 0x78a5636fu, 0x84c87814u, 0x8cc70208u, 0x90befffau, 0xa4506cebu, 0xbef9a3f7u, 0xc67178f2u};

static u8 hashBuf[hashBufBytes];
static u32 crcTable[256];

static void fillBuf(void) {
    for (u32 i = 0; i < hashBufBytes; i++) hashBuf[i] = fillByte(i);
}

static inline u32 ror(u32 x, u32 n) {
    return (x >> n) | (x << (32 - n));
}

static void shaBlock(u32 *st, const u8 *p) {
    u32 w[shaRounds];
    for (u32 i = 0; i < 16; i++) {
        w[i] = ((u32)p[4 * i] << 24) | ((u32)p[4 * i + 1] << 16) | ((u32)p[4 * i + 2] << 8) | (u32)p[4 * i + 3];
    }
    for (u32 i = 16; i < shaRounds; i++) {
        u32 s0 = ror(w[i - 15], 7) ^ ror(w[i - 15], 18) ^ (w[i - 15] >> 3);
        u32 s1 = ror(w[i - 2], 17) ^ ror(w[i - 2], 19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16] + s0 + w[i - 7] + s1;
    }
    u32 a = st[0], b = st[1], c = st[2], d = st[3], e = st[4], f = st[5], g = st[6], h = st[7];
    for (u32 i = 0; i < shaRounds; i++) {
        u32 t1 = h + (ror(e, 6) ^ ror(e, 11) ^ ror(e, 25)) + ((e & f) ^ (~e & g)) + shaK[i] + w[i];
        u32 t2 = (ror(a, 2) ^ ror(a, 13) ^ ror(a, 22)) + ((a & b) ^ (a & c) ^ (b & c));
        h = g;
        g = f;
        f = e;
        e = d + t1;
        d = c;
        c = b;
        b = a;
        a = t1 + t2;
    }
    st[0] += a;
    st[1] += b;
    st[2] += c;
    st[3] += d;
    st[4] += e;
    st[5] += f;
    st[6] += g;
    st[7] += h;
}

u64 testSha256(void) {
    fillBuf();
    u32 st[8];
    for (u32 i = 0; i < 8; i++) st[i] = shaInit[i];
    for (u32 pass = 0; pass < shaPasses; pass++) {
        for (u32 off = 0; off < hashBufBytes; off += shaBlockBytes) shaBlock(st, hashBuf + off);
    }
    u8 last[shaBlockBytes] = {0};
    u64 bits = (u64)shaPasses * hashBufBytes * 8;
    last[0] = 0x80;
    for (u32 k = 0; k < 8; k++) last[shaBlockBytes - 1 - k] = (u8)(bits >> (8 * k));
    shaBlock(st, last);
    u64 h = mixSeed;
    for (u32 i = 0; i < 8; i++) h = mix(h, st[i]);
    return h;
}

u64 testCrc32(void) {
    fillBuf();
    for (u32 n = 0; n < 256; n++) {
        u32 c = n;
        for (u32 k = 0; k < 8; k++) c = (c & 1) ? (c >> 1) ^ crcPoly : c >> 1;
        crcTable[n] = c;
    }
    u32 crc = 0xffffffffu;
    for (u32 pass = 0; pass < crcPasses; pass++) {
        for (u32 i = 0; i < hashBufBytes; i++) crc = crcTable[(crc ^ hashBuf[i]) & 0xff] ^ (crc >> 8);
    }
    return mix(mixSeed, ~crc);
}

u64 testFnv(void) {
    fillBuf();
    u64 h = mixSeed;
    for (u32 pass = 0; pass < fnvPasses; pass++) {
        for (u32 i = 0; i < hashBufBytes; i++) h = mix(h, hashBuf[i]);
    }
    return h;
}
