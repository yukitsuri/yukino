const common = @import("common.zig");

const chachaBlocks: u32 = 2000000;
const chachaWords = 16;
const chachaDoubleRounds = 10;
const chachaConst = [4]u32{ 0x61707865, 0x3320646e, 0x79622d32, 0x6b206574 };

fn rol(x: u32, comptime n: u32) u32 {
    return (x << n) | (x >> (32 - n));
}

fn quarter(s: *[chachaWords]u32, a: usize, b: usize, c: usize, d: usize) void {
    s[a] +%= s[b];
    s[d] = rol(s[d] ^ s[a], 16);
    s[c] +%= s[d];
    s[b] = rol(s[b] ^ s[c], 12);
    s[a] +%= s[b];
    s[d] = rol(s[d] ^ s[a], 8);
    s[c] +%= s[d];
    s[b] = rol(s[b] ^ s[c], 7);
}

fn chachaBlock(in: *const [chachaWords]u32, out: *[chachaWords]u32) void {
    out.* = in.*;
    for (0..chachaDoubleRounds) |_| {
        quarter(out, 0, 4, 8, 12);
        quarter(out, 1, 5, 9, 13);
        quarter(out, 2, 6, 10, 14);
        quarter(out, 3, 7, 11, 15);
        quarter(out, 0, 5, 10, 15);
        quarter(out, 1, 6, 11, 12);
        quarter(out, 2, 7, 8, 13);
        quarter(out, 3, 4, 9, 14);
    }
    for (0..chachaWords) |i| out[i] +%= in[i];
}

pub fn chacha() u64 {
    var st: [chachaWords]u32 = @splat(0);
    var out: [chachaWords]u32 = undefined;
    var r = common.Rng.init(common.rngSeed);
    for (0..4) |i| st[i] = chachaConst[i];
    for (4..chachaWords) |i| st[i] = @truncate(r.next());
    var acc: u32 = 0;
    var c: u32 = 0;
    while (c < chachaBlocks) : (c += 1) {
        st[12] = c;
        chachaBlock(&st, &out);
        for (0..chachaWords) |i| acc ^= out[i];
    }
    return common.mix(common.mixSeed, acc);
}
