const std = @import("std");
const common = @import("common.zig");
const bf = @import("bf.zig");
const bigfact = @import("bigfact.zig");
const chacha = @import("chacha.zig");
const hash = @import("hash.zig");
const hashmap = @import("hashmap.zig");
const mandel = @import("mandel.zig");
const matrix = @import("matrix.zig");
const mem = @import("mem.zig");
const numeric = @import("numeric.zig");
const out = @import("out.zig");
const sort = @import("sort.zig");

const exitUsage = 64;
const exitUnknown = 65;

const Test = struct {
    name: []const u8,
    run: *const fn () u64,
};

fn testNop() u64 {
    return 0;
}

const tests = [_]Test{
    .{ .name = "nop", .run = &testNop },
    .{ .name = "sieve", .run = &numeric.sieve },
    .{ .name = "collatz", .run = &numeric.collatz },
    .{ .name = "fib", .run = &numeric.fibTest },
    .{ .name = "queens", .run = &numeric.queens },
    .{ .name = "leibniz", .run = &numeric.leibniz },
    .{ .name = "matmul", .run = &matrix.matmul },
    .{ .name = "spectral", .run = &matrix.spectral },
    .{ .name = "mandel", .run = &mandel.mandel },
    .{ .name = "sort", .run = &sort.sort },
    .{ .name = "hashmap", .run = &hashmap.hashmap },
    .{ .name = "sha256", .run = &hash.sha256 },
    .{ .name = "crc32", .run = &hash.crc32 },
    .{ .name = "fnv", .run = &hash.fnv },
    .{ .name = "chacha", .run = &chacha.chacha },
    .{ .name = "bigfact", .run = &bigfact.bigfact },
    .{ .name = "bf", .run = &bf.bf },
    .{ .name = "memseq", .run = &mem.memSeq },
    .{ .name = "memrand", .run = &mem.memRand },
    .{ .name = "memtree", .run = &mem.memTree },
    .{ .name = "outbuf", .run = &out.outBuf },
    .{ .name = "outline", .run = &out.outLine },
};

fn strEq(a: []const u8, b: [*:0]const u8) bool {
    for (a, 0..) |c, i| {
        if (c != b[i]) return false;
    }
    return b[a.len] == 0;
}

pub fn main(init: std.process.Init.Minimal) u8 {
    const argv = init.args.vector;
    if (argv.len != 2) return exitUsage;
    for (tests) |t| {
        if (!strEq(t.name, argv[1])) continue;
        const start = common.nowNs();
        const sum = t.run();
        common.report(sum, common.nowNs() - start);
        return 0;
    }
    return exitUnknown;
}
