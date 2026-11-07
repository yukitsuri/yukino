const builtin = @import("builtin");

const Timespec = extern struct { sec: i64, nsec: i64 };

extern "c" fn write(fd: c_int, buf: [*]const u8, n: usize) isize;
extern "c" fn clock_gettime(id: c_int, ts: *Timespec) c_int;
extern "c" fn _exit(code: c_int) noreturn;

pub const mixSeed: u64 = 0xcbf29ce484222325;
pub const rngSeed: u64 = 0x9e3779b97f4a7c15;
const mixPrime: u64 = 0x100000001b3;
const rngMul: u64 = 0x2545f4914f6cdd1d;
const rngShiftA = 12;
const rngShiftB = 25;
const rngShiftC = 27;
const key31Shift = 33;
const fillMul: u32 = 167;
const fillShift = 9;
const decimalBase: u64 = 10;
const maxDigits = 20;
const reportBytes = 64;
const exitWriteFailed = 2;
const nsPerSec: u64 = 1000000000;
const clockMonotonic: c_int = if (builtin.os.tag == .macos) 6 else 1;
const stdoutFd = 1;
const stderrFd = 2;

pub const Rng = struct {
    state: u64,

    pub fn init(seed: u64) Rng {
        return .{ .state = seed };
    }

    pub fn next(self: *Rng) u64 {
        var x = self.state;
        x ^= x >> rngShiftA;
        x ^= x << rngShiftB;
        x ^= x >> rngShiftC;
        self.state = x;
        return x *% rngMul;
    }

    pub fn key31(self: *Rng) u32 {
        return @intCast(self.next() >> key31Shift);
    }

    pub fn below(self: *Rng, n: u64) u64 {
        return (self.next() >> 1) % n;
    }
};

pub fn mix(h: u64, v: u64) u64 {
    return (h ^ v) *% mixPrime;
}

pub fn fillByte(i: u32) u8 {
    return @truncate(i *% fillMul +% (i >> fillShift));
}

pub fn nowNs() u64 {
    var ts: Timespec = undefined;
    _ = clock_gettime(clockMonotonic, &ts);
    return @as(u64, @intCast(ts.sec)) * nsPerSec + @as(u64, @intCast(ts.nsec));
}

fn writeFd(fd: c_int, buf: []const u8) void {
    var done: usize = 0;
    while (done < buf.len) {
        const n = write(fd, buf.ptr + done, buf.len - done);
        if (n <= 0) _exit(exitWriteFailed);
        done += @intCast(n);
    }
}

pub fn writeOut(buf: []const u8) void {
    writeFd(stdoutFd, buf);
}

pub fn fmtU64(value: u64, out: []u8) usize {
    var v = value;
    var tmp: [maxDigits]u8 = undefined;
    var n: usize = 0;
    while (true) {
        tmp[n] = '0' + @as(u8, @intCast(v % decimalBase));
        n += 1;
        v /= decimalBase;
        if (v == 0) break;
    }
    for (0..n) |i| out[i] = tmp[n - 1 - i];
    return n;
}

pub fn report(checksum: u64, ns: u64) void {
    var line: [reportBytes]u8 = undefined;
    line[0] = 'R';
    line[1] = ' ';
    var n: usize = 2;
    n += fmtU64(checksum, line[n..]);
    line[n] = ' ';
    n += 1;
    n += fmtU64(ns, line[n..]);
    line[n] = '\n';
    n += 1;
    writeFd(stderrFd, line[0..n]);
}
