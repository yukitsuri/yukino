#include "tests.hpp"

namespace {

constexpr u64 memSeqCount = 12000000;
constexpr u32 memRandCount = 8000000;
constexpr u32 memRandSteps = 3000000;
constexpr u32 memTreeNodes = (1u << 22) - 1;
constexpr u32 noChild = 0xffffffffu;
constexpr u32 treeValMul = 2654435761u;
constexpr u64 memSeqMul = 0x9e3779b97f4a7c15ULL;

u64 seqBuf[memSeqCount];
u32 perm[memRandCount];
u32 treeLeft[memTreeNodes];
u32 treeRight[memTreeNodes];
u32 treeVal[memTreeNodes];

u64 treeSum(u32 node) {
    if (node == noChild) return 0;
    return treeVal[node] + treeSum(treeLeft[node]) + treeSum(treeRight[node]);
}

}

u64 testMemSeq() {
    for (u64 i = 0; i < memSeqCount; i++) seqBuf[i] = i * memSeqMul;
    u64 sum = 0;
    for (u64 i = 0; i < memSeqCount; i++) sum += seqBuf[i] ^ i;
    return mix(mixSeed, sum);
}

u64 testMemRand() {
    Rng r(rngSeed);
    for (u32 i = 0; i < memRandCount; i++) perm[i] = i;
    // sattolo shuffle gives one cycle so every step is a new cache miss
    for (u32 i = memRandCount - 1; i > 0; i--) {
        u32 j = static_cast<u32>(r.below(i));
        u32 t = perm[i];
        perm[i] = perm[j];
        perm[j] = t;
    }
    u32 p = 0;
    u64 sum = 0;
    for (u32 i = 0; i < memRandSteps; i++) {
        p = perm[p];
        sum += p;
    }
    return mix(mixSeed, sum);
}

u64 testMemTree() {
    for (u32 i = 0; i < memTreeNodes; i++) {
        u32 l = 2 * i + 1;
        u32 r = 2 * i + 2;
        treeLeft[i] = l < memTreeNodes ? l : noChild;
        treeRight[i] = r < memTreeNodes ? r : noChild;
        treeVal[i] = i * treeValMul;
    }
    return mix(mixSeed, treeSum(0));
}
