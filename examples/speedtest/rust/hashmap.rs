use crate::common::{mix, Rng, MIX_SEED, RNG_SEED};

const MAP_BITS: u32 = 22;
const MAP_SIZE: usize = 1 << MAP_BITS;
const MAP_INSERTS: u64 = 2_000_000;
const MAP_MISSES: u64 = 1_000_000;
const MAP_HASH_MUL: u64 = 0x9e3779b97f4a7c15;
const MAP_MISSING: u64 = u64::MAX;
const MISS_SEED: u64 = 0x2545f4914f6cdd1d;

struct Table {
    keys: Vec<u64>,
    vals: Vec<u64>,
}

impl Table {
    fn new() -> Table {
        Table { keys: vec![0u64; MAP_SIZE], vals: vec![0u64; MAP_SIZE] }
    }

    fn slot_of(key: u64) -> usize {
        (key.wrapping_mul(MAP_HASH_MUL) >> (64 - MAP_BITS)) as usize
    }

    fn put(&mut self, key: u64, val: u64) {
        let mut s = Table::slot_of(key);
        while self.keys[s] != 0 && self.keys[s] != key {
            s = (s + 1) & (MAP_SIZE - 1);
        }
        self.keys[s] = key;
        self.vals[s] = val;
    }

    fn get(&self, key: u64) -> u64 {
        let mut s = Table::slot_of(key);
        while self.keys[s] != 0 {
            if self.keys[s] == key {
                return self.vals[s];
            }
            s = (s + 1) & (MAP_SIZE - 1);
        }
        MAP_MISSING
    }
}

pub fn hashmap() -> u64 {
    let mut table = Table::new();
    let mut r = Rng::new(RNG_SEED);
    for i in 0..MAP_INSERTS {
        table.put(r.next() | 1, i);
    }
    let mut again = Rng::new(RNG_SEED);
    let mut sum: u64 = 0;
    for _ in 0..MAP_INSERTS {
        sum = sum.wrapping_add(table.get(again.next() | 1));
    }
    let mut miss = Rng::new(MISS_SEED);
    let mut misses: u64 = 0;
    for _ in 0..MAP_MISSES {
        let v = table.get(miss.next() | 1);
        if v == MAP_MISSING {
            misses += 1;
        } else {
            sum = sum.wrapping_add(v);
        }
    }
    mix(mix(MIX_SEED, sum), misses)
}
