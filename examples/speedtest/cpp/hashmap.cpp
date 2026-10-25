#include "tests.hpp"

namespace {

constexpr u32 mapBits = 22;
constexpr u32 mapSize = u32{1} << mapBits;
constexpr u64 mapInserts = 2000000;
constexpr u64 mapMisses = 1000000;
constexpr u64 mapHashMul = 0x9e3779b97f4a7c15ULL;
constexpr u64 mapMissing = ~0ULL;
constexpr u64 missSeed = 0x2545f4914f6cdd1dULL;

u64 mapKeys[mapSize];
u64 mapVals[mapSize];

u32 slotOf(u64 key) {
    return static_cast<u32>((key * mapHashMul) >> (64 - mapBits));
}

void put(u64 key, u64 val) {
    u32 s = slotOf(key);
    while (mapKeys[s] != 0 && mapKeys[s] != key) s = (s + 1) & (mapSize - 1);
    mapKeys[s] = key;
    mapVals[s] = val;
}

u64 get(u64 key) {
    u32 s = slotOf(key);
    while (mapKeys[s] != 0) {
        if (mapKeys[s] == key) return mapVals[s];
        s = (s + 1) & (mapSize - 1);
    }
    return mapMissing;
}

}

u64 testHashmap() {
    Rng r(rngSeed);
    for (u64 i = 0; i < mapInserts; i++) put(r.next() | 1, i);
    Rng again(rngSeed);
    u64 sum = 0;
    for (u64 i = 0; i < mapInserts; i++) sum += get(again.next() | 1);
    Rng miss(missSeed);
    u64 misses = 0;
    for (u64 i = 0; i < mapMisses; i++) {
        u64 v = get(miss.next() | 1);
        if (v == mapMissing) misses++;
        else sum += v;
    }
    return mix(mix(mixSeed, sum), misses);
}
