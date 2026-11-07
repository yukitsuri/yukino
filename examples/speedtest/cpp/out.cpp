#include "tests.hpp"

namespace {

constexpr std::size_t outBufSize = 65536;
constexpr u64 outLines = 3000000;
constexpr u64 outSingleLines = 300000;
constexpr std::size_t maxLineBytes = 64;

u8 outBuf[outBufSize];

std::size_t fmtLine(u8 *dst, u64 index, u64 value) {
    std::size_t n = fmtU64(index, dst);
    dst[n++] = ' ';
    n += fmtU64(value, dst + n);
    dst[n++] = '\n';
    return n;
}

}

u64 testOutBuf() {
    Rng r(rngSeed);
    std::size_t used = 0;
    u64 total = 0;
    for (u64 i = 0; i < outLines; i++) {
        if (used + maxLineBytes > outBufSize) {
            writeOut(outBuf, used);
            total += used;
            used = 0;
        }
        used += fmtLine(outBuf + used, i, r.next());
    }
    writeOut(outBuf, used);
    return mix(mixSeed, total + used);
}

u64 testOutLine() {
    Rng r(rngSeed);
    u8 line[maxLineBytes];
    u64 total = 0;
    for (u64 i = 0; i < outSingleLines; i++) {
        std::size_t n = fmtLine(line, i, r.next());
        writeOut(line, n);
        total += n;
    }
    return mix(mixSeed, total);
}
