const common = @import("common.zig");

const memSeqCount: u64 = 12000000;
const memRandCount: u32 = 8000000;
const memRandSteps: u32 = 3000000;
const memTreeNodes: u32 = (1 << 22) - 1;
const noChild: u32 = 0xffffffff;
const treeValMul: u32 = 2654435761;
const memSeqMul: u64 = 0x9e3779b97f4a7c15;

var seqBuf: [memSeqCount]u64 = undefined;
var perm: [memRandCount]u32 = undefined;
var treeLeft: [memTreeNodes]u32 = undefined;
var treeRight: [memTreeNodes]u32 = undefined;
var treeVal: [memTreeNodes]u32 = undefined;

pub fn memSeq() u64 {
    for (0..memSeqCount) |i| seqBuf[i] = @as(u64, i) *% memSeqMul;
    var sum: u64 = 0;
    for (0..memSeqCount) |i| sum +%= seqBuf[i] ^ @as(u64, i);
    return common.mix(common.mixSeed, sum);
}

pub fn memRand() u64 {
    for (0..memRandCount) |i| perm[i] = @intCast(i);
    var r = common.Rng.init(common.rngSeed);
    // sattolo shuffle gives one cycle so every step is a new cache miss
    var i: u32 = memRandCount - 1;
    while (i > 0) : (i -= 1) {
        const j: u32 = @intCast(r.below(i));
        const t = perm[i];
        perm[i] = perm[j];
        perm[j] = t;
    }
    var p: u32 = 0;
    var sum: u64 = 0;
    for (0..memRandSteps) |_| {
        p = perm[p];
        sum += p;
    }
    return common.mix(common.mixSeed, sum);
}

fn treeSum(node: u32) u64 {
    if (node == noChild) return 0;
    return @as(u64, treeVal[node]) + treeSum(treeLeft[node]) + treeSum(treeRight[node]);
}

pub fn memTree() u64 {
    for (0..memTreeNodes) |i| {
        const l = 2 * i + 1;
        const r = 2 * i + 2;
        treeLeft[i] = if (l < memTreeNodes) @intCast(l) else noChild;
        treeRight[i] = if (r < memTreeNodes) @intCast(r) else noChild;
        treeVal[i] = @as(u32, @intCast(i)) *% treeValMul;
    }
    return common.mix(common.mixSeed, treeSum(0));
}
