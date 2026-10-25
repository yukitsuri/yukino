const common = @import("common.zig");

const mandelSize: i64 = 1000;
const mandelMaxIter: u64 = 500;
const mandelFrac = 24;

fn iterations(cr: i64, ci: i64) u64 {
    const limit: i64 = 4 << mandelFrac;
    var zr: i64 = 0;
    var zi: i64 = 0;
    var n: u64 = 0;
    while (n < mandelMaxIter) {
        const zr2 = (zr * zr) >> mandelFrac;
        const zi2 = (zi * zi) >> mandelFrac;
        if (zr2 + zi2 > limit) break;
        zi = ((2 * zr * zi) >> mandelFrac) + ci;
        zr = zr2 - zi2 + cr;
        n += 1;
    }
    return n;
}

pub fn mandel() u64 {
    const one: i64 = 1 << mandelFrac;
    const x0: i64 = -2 * one;
    const y0: i64 = @divExact(-3 * one, 2);
    const step: i64 = @divTrunc(3 * one, mandelSize);
    var total: u64 = 0;
    var py: i64 = 0;
    while (py < mandelSize) : (py += 1) {
        var px: i64 = 0;
        while (px < mandelSize) : (px += 1) total += iterations(x0 + px * step, y0 + py * step);
    }
    return common.mix(common.mixSeed, total);
}
