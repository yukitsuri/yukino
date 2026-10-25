#include "tests.h"

enum { outBufSize = 65536, outLines = 3000000, outSingleLines = 300000, maxLineBytes = 64 };

static u8 outBuf[outBufSize];

static size_t fmtLine(u8 *dst, u64 index, u64 value) {
    size_t n = fmtU64(index, dst);
    dst[n++] = ' ';
    n += fmtU64(value, dst + n);
    dst[n++] = '\n';
    return n;
}

u64 testOutBuf(void) {
    Rng r;
    rngInit(&r, rngSeed);
    size_t used = 0;
    u64 total = 0;
    for (u64 i = 0; i < outLines; i++) {
        if (used + maxLineBytes > outBufSize) {
            writeOut(outBuf, used);
            total += used;
            used = 0;
        }
        used += fmtLine(outBuf + used, i, rngNext(&r));
    }
    writeOut(outBuf, used);
    return mix(mixSeed, total + used);
}

u64 testOutLine(void) {
    Rng r;
    rngInit(&r, rngSeed);
    u8 line[maxLineBytes];
    u64 total = 0;
    for (u64 i = 0; i < outSingleLines; i++) {
        size_t n = fmtLine(line, i, rngNext(&r));
        writeOut(line, n);
        total += n;
    }
    return mix(mixSeed, total);
}
