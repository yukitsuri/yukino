const common = @import("common.zig");

const outBufSize = 65536;
const outLines: u64 = 3000000;
const outSingleLines: u64 = 300000;
const maxLineBytes = 64;

var outBufData: [outBufSize]u8 = undefined;

fn fmtLine(dst: []u8, index: u64, value: u64) usize {
    var n = common.fmtU64(index, dst);
    dst[n] = ' ';
    n += 1;
    n += common.fmtU64(value, dst[n..]);
    dst[n] = '\n';
    return n + 1;
}

pub fn outBuf() u64 {
    var r = common.Rng.init(common.rngSeed);
    var used: usize = 0;
    var total: u64 = 0;
    var i: u64 = 0;
    while (i < outLines) : (i += 1) {
        if (used + maxLineBytes > outBufSize) {
            common.writeOut(outBufData[0..used]);
            total += used;
            used = 0;
        }
        used += fmtLine(outBufData[used..], i, r.next());
    }
    common.writeOut(outBufData[0..used]);
    return common.mix(common.mixSeed, total + used);
}

pub fn outLine() u64 {
    var r = common.Rng.init(common.rngSeed);
    var line: [maxLineBytes]u8 = undefined;
    var total: u64 = 0;
    var i: u64 = 0;
    while (i < outSingleLines) : (i += 1) {
        const n = fmtLine(&line, i, r.next());
        common.writeOut(line[0..n]);
        total += n;
    }
    return common.mix(common.mixSeed, total);
}
