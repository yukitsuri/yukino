use crate::common::{mix, Rng, MIX_SEED, RNG_SEED};

const MEM_SEQ_COUNT: usize = 12_000_000;
const MEM_RAND_COUNT: usize = 8_000_000;
const MEM_RAND_STEPS: usize = 3_000_000;
const MEM_TREE_NODES: usize = (1 << 22) - 1;
const NO_CHILD: u32 = u32::MAX;
const TREE_VAL_MUL: u32 = 2654435761;
const MEM_SEQ_MUL: u64 = 0x9e3779b97f4a7c15;

pub fn mem_seq() -> u64 {
    let mut buf = vec![0u64; MEM_SEQ_COUNT];
    for i in 0..MEM_SEQ_COUNT {
        buf[i] = (i as u64).wrapping_mul(MEM_SEQ_MUL);
    }
    let mut sum: u64 = 0;
    for i in 0..MEM_SEQ_COUNT {
        sum = sum.wrapping_add(buf[i] ^ i as u64);
    }
    mix(MIX_SEED, sum)
}

pub fn mem_rand() -> u64 {
    let mut perm = vec![0u32; MEM_RAND_COUNT];
    for i in 0..MEM_RAND_COUNT {
        perm[i] = i as u32;
    }
    let mut r = Rng::new(RNG_SEED);
    // sattolo shuffle gives one cycle so every step is a new cache miss
    for i in (1..MEM_RAND_COUNT).rev() {
        let j = r.below(i as u64) as usize;
        let t = perm[i];
        perm[i] = perm[j];
        perm[j] = t;
    }
    let mut p: usize = 0;
    let mut sum: u64 = 0;
    for _ in 0..MEM_RAND_STEPS {
        p = perm[p] as usize;
        sum += p as u64;
    }
    mix(MIX_SEED, sum)
}

struct Tree {
    left: Vec<u32>,
    right: Vec<u32>,
    val: Vec<u32>,
}

impl Tree {
    fn sum(&self, node: u32) -> u64 {
        if node == NO_CHILD {
            return 0;
        }
        let n = node as usize;
        self.val[n] as u64 + self.sum(self.left[n]) + self.sum(self.right[n])
    }
}

pub fn mem_tree() -> u64 {
    let mut tree = Tree {
        left: vec![0u32; MEM_TREE_NODES],
        right: vec![0u32; MEM_TREE_NODES],
        val: vec![0u32; MEM_TREE_NODES],
    };
    for i in 0..MEM_TREE_NODES {
        let l = 2 * i + 1;
        let r = 2 * i + 2;
        tree.left[i] = if l < MEM_TREE_NODES { l as u32 } else { NO_CHILD };
        tree.right[i] = if r < MEM_TREE_NODES { r as u32 } else { NO_CHILD };
        tree.val[i] = (i as u32).wrapping_mul(TREE_VAL_MUL);
    }
    mix(MIX_SEED, tree.sum(0))
}
