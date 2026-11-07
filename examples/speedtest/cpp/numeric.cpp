#include "tests.hpp"

namespace {

constexpr u64 sieveLimit = 40000000;
constexpr u64 collatzLimit = 1500000;
constexpr u64 fibN = 42;
constexpr u32 queensN = 14;
constexpr u64 leibnizTerms = 150000000;
constexpr double leibnizScale = 4.0e9;

u8 composite[sieveLimit + 1];

u64 fib(u64 n) {
    return n < 2 ? n : fib(n - 1) + fib(n - 2);
}

u64 solveQueens(u32 cols, u32 diagL, u32 diagR, u32 full) {
    if (cols == full) return 1;
    u64 count = 0;
    u32 avail = full & ~(cols | diagL | diagR);
    while (avail != 0) {
        u32 bit = avail & (~avail + 1);
        avail ^= bit;
        count += solveQueens(cols | bit, ((diagL | bit) << 1) & full, (diagR | bit) >> 1, full);
    }
    return count;
}

}

u64 testSieve() {
    u64 count = 0;
    for (u64 i = 2; i * i <= sieveLimit; i++) {
        if (composite[i]) continue;
        for (u64 j = i * i; j <= sieveLimit; j += i) composite[j] = 1;
    }
    for (u64 i = 2; i <= sieveLimit; i++) count += composite[i] == 0;
    return mix(mixSeed, count);
}

u64 testCollatz() {
    u64 bestStart = 0;
    u64 bestSteps = 0;
    for (u64 start = 1; start <= collatzLimit; start++) {
        u64 n = start;
        u64 steps = 0;
        while (n != 1) {
            n = (n & 1) ? 3 * n + 1 : n >> 1;
            steps++;
        }
        if (steps > bestSteps) {
            bestSteps = steps;
            bestStart = start;
        }
    }
    return mix(mix(mixSeed, bestStart), bestSteps);
}

u64 testFib() {
    return mix(mixSeed, fib(fibN));
}

u64 testQueens() {
    u32 full = (u32{1} << queensN) - 1;
    return mix(mixSeed, solveQueens(0, 0, 0, full));
}

u64 testLeibniz() {
    double sum = 0.0;
    double sign = 1.0;
    for (u64 k = 0; k < leibnizTerms; k++) {
        sum += sign / static_cast<double>(2 * k + 1);
        sign = -sign;
    }
    return mix(mixSeed, static_cast<u64>(sum * leibnizScale));
}
