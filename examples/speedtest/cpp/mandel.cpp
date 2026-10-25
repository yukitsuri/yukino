#include "tests.hpp"

namespace {

constexpr i64 mandelSize = 1000;
constexpr u64 mandelMaxIter = 500;
constexpr u32 mandelFrac = 24;

u64 iterations(i64 cr, i64 ci) {
    constexpr i64 limit = i64{4} << mandelFrac;
    i64 zr = 0;
    i64 zi = 0;
    u64 n = 0;
    while (n < mandelMaxIter) {
        i64 zr2 = (zr * zr) >> mandelFrac;
        i64 zi2 = (zi * zi) >> mandelFrac;
        if (zr2 + zi2 > limit) break;
        zi = ((2 * zr * zi) >> mandelFrac) + ci;
        zr = zr2 - zi2 + cr;
        n++;
    }
    return n;
}

}

u64 testMandel() {
    constexpr i64 one = i64{1} << mandelFrac;
    constexpr i64 x0 = -2 * one;
    constexpr i64 y0 = -3 * one / 2;
    constexpr i64 step = 3 * one / mandelSize;
    u64 total = 0;
    for (i64 py = 0; py < mandelSize; py++) {
        for (i64 px = 0; px < mandelSize; px++) total += iterations(x0 + px * step, y0 + py * step);
    }
    return mix(mixSeed, total);
}
