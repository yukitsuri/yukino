#include "tests.h"

enum { sieveLimit = 40000000, collatzLimit = 1500000, fibN = 42, queensN = 14, leibnizTerms = 150000000 };

static const double leibnizScale = 4.0e9;
static u8 composite[sieveLimit + 1];

u64 testSieve(void) {
    u64 count = 0;
    for (u64 i = 2; i * i <= sieveLimit; i++) {
        if (composite[i]) continue;
        for (u64 j = i * i; j <= sieveLimit; j += i) composite[j] = 1;
    }
    for (u64 i = 2; i <= sieveLimit; i++) count += !composite[i];
    return mix(mixSeed, count);
}

u64 testCollatz(void) {
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

static u64 fib(u64 n) {
    return n < 2 ? n : fib(n - 1) + fib(n - 2);
}

u64 testFib(void) {
    return mix(mixSeed, fib(fibN));
}

static u64 solveQueens(u32 cols, u32 diagL, u32 diagR, u32 full) {
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

u64 testQueens(void) {
    u32 full = ((u32)1 << queensN) - 1;
    return mix(mixSeed, solveQueens(0, 0, 0, full));
}

u64 testLeibniz(void) {
    double sum = 0.0;
    double sign = 1.0;
    for (u64 k = 0; k < leibnizTerms; k++) {
        sum += sign / (double)(2 * k + 1);
        sign = -sign;
    }
    return mix(mixSeed, (u64)(sum * leibnizScale));
}
