use crate::common::{mix, MIX_SEED};

const FACT_N: u64 = 20_000;
const LIMB_CAP: usize = 10_000;
const LIMB_BASE: u64 = 1_000_000_000;

pub fn bigfact() -> u64 {
    let mut limbs = vec![0u32; LIMB_CAP];
    let mut used: usize = 1;
    limbs[0] = 1;
    for m in 2..=FACT_N {
        let mut carry: u64 = 0;
        for i in 0..used {
            let t = limbs[i] as u64 * m + carry;
            limbs[i] = (t % LIMB_BASE) as u32;
            carry = t / LIMB_BASE;
        }
        while carry != 0 {
            limbs[used] = (carry % LIMB_BASE) as u32;
            used += 1;
            carry /= LIMB_BASE;
        }
    }
    let mut h = MIX_SEED;
    for i in 0..used {
        h = mix(h, limbs[i] as u64);
    }
    mix(h, used as u64)
}
