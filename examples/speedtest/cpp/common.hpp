#ifndef COMMON_HPP
#define COMMON_HPP

#include <cstddef>
#include <cstdint>

using u8 = std::uint8_t;
using u32 = std::uint32_t;
using u64 = std::uint64_t;
using i64 = std::int64_t;

constexpr u64 mixSeed = 0xcbf29ce484222325ULL;
constexpr u64 mixPrime = 0x100000001b3ULL;
constexpr u64 rngSeed = 0x9e3779b97f4a7c15ULL;
constexpr u64 rngMul = 0x2545f4914f6cdd1dULL;
constexpr u32 rngShiftA = 12;
constexpr u32 rngShiftB = 25;
constexpr u32 rngShiftC = 27;
constexpr u32 key31Shift = 33;
constexpr u32 fillMul = 167;
constexpr u32 fillShift = 9;

struct Rng {
    u64 state;

    explicit Rng(u64 seed) : state(seed) {}

    u64 next() {
        u64 x = state;
        x ^= x >> rngShiftA;
        x ^= x << rngShiftB;
        x ^= x >> rngShiftC;
        state = x;
        return x * rngMul;
    }

    u32 key31() {
        return static_cast<u32>(next() >> key31Shift);
    }

    u64 below(u64 n) {
        return (next() >> 1) % n;
    }
};

inline u64 mix(u64 h, u64 v) {
    return (h ^ v) * mixPrime;
}

inline u8 fillByte(u32 i) {
    return static_cast<u8>(i * fillMul + (i >> fillShift));
}

u64 nowNs();
void writeOut(const u8 *buf, std::size_t len);
std::size_t fmtU64(u64 v, u8 *out);
void report(u64 checksum, u64 ns);

#endif
