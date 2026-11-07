#include "tests.h"

enum { memSeqCount = 12000000, memRandCount = 8000000, memRandSteps = 3000000, memTreeNodes = (1 << 22) - 1 };

static const u32 noChild = 0xffffffffu;
static const u32 treeValMul = 2654435761u;
static const u64 memSeqMul = 0x9e3779b97f4a7c15ULL;
static u64 seqBuf[memSeqCount];
static u32 perm[memRandCount];
static u32 treeLeft[memTreeNodes];
static u32 treeRight[memTreeNodes];
static u32 treeVal[memTreeNodes];

u64 testMemSeq(void) {
    for (u64 i = 0; i < memSeqCount; i++) seqBuf[i] = i * memSeqMul;
    u64 sum = 0;
    for (u64 i = 0; i < memSeqCount; i++) sum += seqBuf[i] ^ i;
    return mix(mixSeed, sum);
}

u64 testMemRand(void) {
    Rng r;
    rngInit(&r, rngSeed);
    for (u32 i = 0; i < memRandCount; i++) perm[i] = i;
    // sattolo shuffle gives one cycle so every step is a new cache miss
    for (u32 i = memRandCount - 1; i > 0; i--) {
        u32 j = (u32)rngBelow(&r, i);
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

static u64 treeSum(u32 node) {
    if (node == noChild) return 0;
    return treeVal[node] + treeSum(treeLeft[node]) + treeSum(treeRight[node]);
}

u64 testMemTree(void) {
    for (u32 i = 0; i < memTreeNodes; i++) {
        u32 l = 2 * i + 1;
        u32 r = 2 * i + 2;
        treeLeft[i] = l < memTreeNodes ? l : noChild;
        treeRight[i] = r < memTreeNodes ? r : noChild;
        treeVal[i] = i * treeValMul;
    }
    return mix(mixSeed, treeSum(0));
}
