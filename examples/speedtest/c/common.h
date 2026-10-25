#ifndef COMMON_H
#define COMMON_H

#include <stddef.h>
#include <stdint.h>

typedef uint8_t u8;
typedef uint32_t u32;
typedef uint64_t u64;
typedef int64_t i64;

static const u64 mixSeed = 0xcbf29ce484222325ULL;
static const u64 mixPrime = 0x100000001b3ULL;
static const u64 rngSeed = 0x9e3779b97f4a7c15ULL;
static const u64 rngMul = 0x2545f4914f6cdd1dULL;
static const u32 rngShiftA = 12;
static const u32 rngShiftB = 25;
static const u32 rngShiftC = 27;
static const u32 key31Shift = 33;
static const u32 fillMul = 167;
static const u32 fillShift = 9;

typedef struct {
    u64 state;
} Rng;

static inline void rngInit(Rng *r, u64 seed) {
    r->state = seed;
}

static inline u64 rngNext(Rng *r) {
    u64 x = r->state;
    x ^= x >> rngShiftA;
    x ^= x << rngShiftB;
    x ^= x >> rngShiftC;
    r->state = x;
    return x * rngMul;
}

static inline u32 rngKey31(Rng *r) {
    return (u32)(rngNext(r) >> key31Shift);
}

static inline u64 rngBelow(Rng *r, u64 n) {
    return (rngNext(r) >> 1) % n;
}

static inline u64 mix(u64 h, u64 v) {
    return (h ^ v) * mixPrime;
}

static inline u8 fillByte(u32 i) {
    return (u8)(i * fillMul + (i >> fillShift));
}

u64 nowNs(void);
void writeOut(const u8 *buf, size_t len);
size_t fmtU64(u64 v, u8 *out);
void report(u64 checksum, u64 ns);

#endif
