const common = @import("common.zig");

const factN: u64 = 20000;
const limbCap = 10000;
const limbBase: u64 = 1000000000;

var limbs: [limbCap]u32 = @splat(0);

pub fn bigfact() u64 {
    var used: usize = 1;
    limbs[0] = 1;
    var m: u64 = 2;
    while (m <= factN) : (m += 1) {
        var carry: u64 = 0;
        for (0..used) |i| {
            const t = @as(u64, limbs[i]) * m + carry;
            limbs[i] = @intCast(t % limbBase);
            carry = t / limbBase;
        }
        while (carry != 0) {
            limbs[used] = @intCast(carry % limbBase);
            used += 1;
            carry /= limbBase;
        }
    }
    var h = common.mixSeed;
    for (0..used) |i| h = common.mix(h, limbs[i]);
    return common.mix(h, used);
}
