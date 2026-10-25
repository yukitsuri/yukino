const common = @import("common.zig");

const mix = common.mix;
const mixSeed = common.mixSeed;

const matN: usize = 900;
const matCells = matN * matN;
const spectralN: u32 = 1500;
const spectralIters = 10;
const spectralScale: f64 = 1.0e6;

var matA: [matCells]u32 = undefined;
var matB: [matCells]u32 = undefined;
var matC: [matCells]u32 = @splat(0);
var specU: [spectralN]f64 = undefined;
var specV: [spectralN]f64 = undefined;
var specT: [spectralN]f64 = undefined;

fn multiply() void {
    for (0..matN) |i| {
        for (0..matN) |k| {
            const aik = matA[i * matN + k];
            for (0..matN) |j| matC[i * matN + j] +%= aik *% matB[k * matN + j];
        }
    }
}

pub fn matmul() u64 {
    var r = common.Rng.init(common.rngSeed);
    for (0..matCells) |i| matA[i] = r.key31();
    for (0..matCells) |i| matB[i] = r.key31();
    multiply();
    var h = mixSeed;
    for (0..matCells) |i| h = mix(h, matC[i]);
    return h;
}

fn coef(i: u32, j: u32) f64 {
    return 1.0 / @as(f64, @floatFromInt((i + j) * (i + j + 1) / 2 + i + 1));
}

fn mulAv(v: *const [spectralN]f64, out: *[spectralN]f64) void {
    for (0..spectralN) |i| {
        var s: f64 = 0.0;
        for (0..spectralN) |j| s += coef(@intCast(i), @intCast(j)) * v[j];
        out[i] = s;
    }
}

fn mulAtv(v: *const [spectralN]f64, out: *[spectralN]f64) void {
    for (0..spectralN) |i| {
        var s: f64 = 0.0;
        for (0..spectralN) |j| s += coef(@intCast(j), @intCast(i)) * v[j];
        out[i] = s;
    }
}

fn mulAtAv(v: *const [spectralN]f64, out: *[spectralN]f64) void {
    mulAv(v, &specT);
    mulAtv(&specT, out);
}

pub fn spectral() u64 {
    for (0..spectralN) |i| specU[i] = 1.0;
    for (0..spectralIters) |_| {
        mulAtAv(&specU, &specV);
        mulAtAv(&specV, &specU);
    }
    var vbv: f64 = 0.0;
    var vv: f64 = 0.0;
    for (0..spectralN) |i| {
        vbv += specU[i] * specV[i];
        vv += specV[i] * specV[i];
    }
    return mix(mix(mixSeed, @intFromFloat(vbv * spectralScale)), @intFromFloat(vv * spectralScale));
}
