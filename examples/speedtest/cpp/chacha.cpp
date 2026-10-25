#include "tests.hpp"

namespace {

constexpr u32 chachaBlocks = 2000000;
constexpr u32 chachaWords = 16;
constexpr u32 chachaDoubleRounds = 10;
constexpr u32 chachaConst[4] = {0x61707865u, 0x3320646eu, 0x79622d32u, 0x6b206574u};

inline u32 rol(u32 x, u32 n) {
    return (x << n) | (x >> (32 - n));
}

inline void quarter(u32 *s, u32 a, u32 b, u32 c, u32 d) {
    s[a] += s[b];
    s[d] = rol(s[d] ^ s[a], 16);
    s[c] += s[d];
    s[b] = rol(s[b] ^ s[c], 12);
    s[a] += s[b];
    s[d] = rol(s[d] ^ s[a], 8);
    s[c] += s[d];
    s[b] = rol(s[b] ^ s[c], 7);
}

void chachaBlock(const u32 *in, u32 *out) {
    for (u32 i = 0; i < chachaWords; i++) out[i] = in[i];
    for (u32 r = 0; r < chachaDoubleRounds; r++) {
        quarter(out, 0, 4, 8, 12);
        quarter(out, 1, 5, 9, 13);
        quarter(out, 2, 6, 10, 14);
        quarter(out, 3, 7, 11, 15);
        quarter(out, 0, 5, 10, 15);
        quarter(out, 1, 6, 11, 12);
        quarter(out, 2, 7, 8, 13);
        quarter(out, 3, 4, 9, 14);
    }
    for (u32 i = 0; i < chachaWords; i++) out[i] += in[i];
}

}

u64 testChacha() {
    u32 st[chachaWords] = {};
    u32 out[chachaWords];
    Rng r(rngSeed);
    for (u32 i = 0; i < 4; i++) st[i] = chachaConst[i];
    for (u32 i = 4; i < chachaWords; i++) st[i] = static_cast<u32>(r.next());
    u32 acc = 0;
    for (u32 c = 0; c < chachaBlocks; c++) {
        st[12] = c;
        chachaBlock(st, out);
        for (u32 i = 0; i < chachaWords; i++) acc ^= out[i];
    }
    return mix(mixSeed, acc);
}
