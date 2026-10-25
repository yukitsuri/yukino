const common = @import("common.zig");

const mapBits = 22;
const mapSize: u32 = 1 << mapBits;
const mapInserts: u64 = 2000000;
const mapMisses: u64 = 1000000;
const mapHashMul: u64 = 0x9e3779b97f4a7c15;
const mapMissing: u64 = ~@as(u64, 0);
const missSeed: u64 = 0x2545f4914f6cdd1d;

var mapKeys: [mapSize]u64 = @splat(0);
var mapVals: [mapSize]u64 = undefined;

fn slotOf(key: u64) u32 {
    return @intCast((key *% mapHashMul) >> (64 - mapBits));
}

fn put(key: u64, val: u64) void {
    var s = slotOf(key);
    while (mapKeys[s] != 0 and mapKeys[s] != key) s = (s + 1) & (mapSize - 1);
    mapKeys[s] = key;
    mapVals[s] = val;
}

fn get(key: u64) u64 {
    var s = slotOf(key);
    while (mapKeys[s] != 0) {
        if (mapKeys[s] == key) return mapVals[s];
        s = (s + 1) & (mapSize - 1);
    }
    return mapMissing;
}

pub fn hashmap() u64 {
    var r = common.Rng.init(common.rngSeed);
    var i: u64 = 0;
    while (i < mapInserts) : (i += 1) put(r.next() | 1, i);
    var again = common.Rng.init(common.rngSeed);
    var sum: u64 = 0;
    i = 0;
    while (i < mapInserts) : (i += 1) sum +%= get(again.next() | 1);
    var miss = common.Rng.init(missSeed);
    var misses: u64 = 0;
    i = 0;
    while (i < mapMisses) : (i += 1) {
        const v = get(miss.next() | 1);
        if (v == mapMissing) {
            misses += 1;
        } else {
            sum +%= v;
        }
    }
    return common.mix(common.mix(common.mixSeed, sum), misses);
}
