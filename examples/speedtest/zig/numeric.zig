const common = @import("common.zig");

const mix = common.mix;
const mixSeed = common.mixSeed;

const sieveLimit: usize = 40000000;
const collatzLimit: u64 = 1500000;
const fibN: u64 = 42;
const queensN = 14;
const leibnizTerms: u64 = 150000000;
const leibnizScale: f64 = 4.0e9;

var composite: [sieveLimit + 1]u8 = @splat(0);

pub fn sieve() u64 {
    var i: usize = 2;
    while (i * i <= sieveLimit) : (i += 1) {
        if (composite[i] != 0) continue;
        var j = i * i;
        while (j <= sieveLimit) : (j += i) composite[j] = 1;
    }
    var count: u64 = 0;
    i = 2;
    while (i <= sieveLimit) : (i += 1) count += @intFromBool(composite[i] == 0);
    return mix(mixSeed, count);
}

pub fn collatz() u64 {
    var bestStart: u64 = 0;
    var bestSteps: u64 = 0;
    var start: u64 = 1;
    while (start <= collatzLimit) : (start += 1) {
        var n = start;
        var steps: u64 = 0;
        while (n != 1) {
            n = if ((n & 1) == 1) 3 * n + 1 else n >> 1;
            steps += 1;
        }
        if (steps > bestSteps) {
            bestSteps = steps;
            bestStart = start;
        }
    }
    return mix(mix(mixSeed, bestStart), bestSteps);
}

fn fib(n: u64) u64 {
    return if (n < 2) n else fib(n - 1) + fib(n - 2);
}

pub fn fibTest() u64 {
    return mix(mixSeed, fib(fibN));
}

fn solveQueens(cols: u32, diagL: u32, diagR: u32, full: u32) u64 {
    if (cols == full) return 1;
    var count: u64 = 0;
    var avail = full & ~(cols | diagL | diagR);
    while (avail != 0) {
        const bit = avail & (~avail +% 1);
        avail ^= bit;
        count += solveQueens(cols | bit, ((diagL | bit) << 1) & full, (diagR | bit) >> 1, full);
    }
    return count;
}

pub fn queens() u64 {
    const full: u32 = (1 << queensN) - 1;
    return mix(mixSeed, solveQueens(0, 0, 0, full));
}

pub fn leibniz() u64 {
    var sum: f64 = 0.0;
    var sign: f64 = 1.0;
    var k: u64 = 0;
    while (k < leibnizTerms) : (k += 1) {
        sum += sign / @as(f64, @floatFromInt(2 * k + 1));
        sign = -sign;
    }
    return mix(mixSeed, @intFromFloat(sum * leibnizScale));
}
