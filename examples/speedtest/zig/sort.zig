const common = @import("common.zig");

const sortCount: i64 = 3000000;
const insertionCutoff: i64 = 16;
const sortStride: usize = 1009;

var keys: [sortCount]u32 = undefined;

fn insertionSort(a: []u32, lo: i64, hi: i64) void {
    var i = lo + 1;
    while (i <= hi) : (i += 1) {
        const v = a[@intCast(i)];
        var j = i - 1;
        while (j >= lo and a[@intCast(j)] > v) : (j -= 1) a[@intCast(j + 1)] = a[@intCast(j)];
        a[@intCast(j + 1)] = v;
    }
}

fn quickSort(a: []u32, loStart: i64, hiStart: i64) void {
    var lo = loStart;
    var hi = hiStart;
    while (hi - lo >= insertionCutoff) {
        const pivot = a[@intCast(lo + @divTrunc(hi - lo, 2))];
        var i = lo;
        var j = hi;
        while (i <= j) {
            while (a[@intCast(i)] < pivot) i += 1;
            while (a[@intCast(j)] > pivot) j -= 1;
            if (i <= j) {
                const t = a[@intCast(i)];
                a[@intCast(i)] = a[@intCast(j)];
                a[@intCast(j)] = t;
                i += 1;
                j -= 1;
            }
        }
        // recurse into the smaller part so stack depth stays logarithmic
        if (j - lo < hi - i) {
            quickSort(a, lo, j);
            lo = i;
        } else {
            quickSort(a, i, hi);
            hi = j;
        }
    }
    insertionSort(a, lo, hi);
}

pub fn sort() u64 {
    var r = common.Rng.init(common.rngSeed);
    for (0..sortCount) |i| keys[i] = r.key31();
    quickSort(&keys, 0, sortCount - 1);
    var ordered: u64 = 1;
    for (1..sortCount) |i| {
        if (keys[i - 1] > keys[i]) ordered = 0;
    }
    var h = common.mixSeed;
    var i: usize = 0;
    while (i < sortCount) : (i += sortStride) h = common.mix(h, keys[i]);
    return common.mix(h, ordered);
}
