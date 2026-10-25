#include "tests.h"

enum { mapBits = 22, mapSize = 1 << mapBits, mapInserts = 2000000, mapMisses = 1000000 };

static const u64 mapHashMul = 0x9e3779b97f4a7c15ULL;
static const u64 mapMissing = ~0ULL;
static const u64 missSeed = 0x2545f4914f6cdd1dULL;
static u64 mapKeys[mapSize];
static u64 mapVals[mapSize];

static u32 slotOf(u64 key) {
    return (u32)((key * mapHashMul) >> (64 - mapBits));
}

static void put(u64 key, u64 val) {
    u32 s = slotOf(key);
    while (mapKeys[s] != 0 && mapKeys[s] != key) s = (s + 1) & (mapSize - 1);
    mapKeys[s] = key;
    mapVals[s] = val;
}

static u64 get(u64 key) {
    u32 s = slotOf(key);
    while (mapKeys[s] != 0) {
        if (mapKeys[s] == key) return mapVals[s];
        s = (s + 1) & (mapSize - 1);
    }
    return mapMissing;
}

u64 testHashmap(void) {
    Rng r;
    rngInit(&r, rngSeed);
    for (u64 i = 0; i < mapInserts; i++) put(rngNext(&r) | 1, i);
    rngInit(&r, rngSeed);
    u64 sum = 0;
    for (u64 i = 0; i < mapInserts; i++) sum += get(rngNext(&r) | 1);
    rngInit(&r, missSeed);
    u64 misses = 0;
    for (u64 i = 0; i < mapMisses; i++) {
        u64 v = get(rngNext(&r) | 1);
        if (v == mapMissing) misses++;
        else sum += v;
    }
    return mix(mix(mixSeed, sum), misses);
}
