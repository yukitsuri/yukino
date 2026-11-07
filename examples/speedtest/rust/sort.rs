use crate::common::{mix, Rng, MIX_SEED, RNG_SEED};

const SORT_COUNT: usize = 3_000_000;
const INSERTION_CUTOFF: i64 = 16;
const SORT_STRIDE: usize = 1009;

fn insertion_sort(a: &mut [u32], lo: i64, hi: i64) {
    for i in (lo + 1)..=hi {
        let v = a[i as usize];
        let mut j = i - 1;
        while j >= lo && a[j as usize] > v {
            a[(j + 1) as usize] = a[j as usize];
            j -= 1;
        }
        a[(j + 1) as usize] = v;
    }
}

fn quick_sort(a: &mut [u32], mut lo: i64, mut hi: i64) {
    while hi - lo >= INSERTION_CUTOFF {
        let pivot = a[(lo + (hi - lo) / 2) as usize];
        let mut i = lo;
        let mut j = hi;
        while i <= j {
            while a[i as usize] < pivot {
                i += 1;
            }
            while a[j as usize] > pivot {
                j -= 1;
            }
            if i <= j {
                let t = a[i as usize];
                a[i as usize] = a[j as usize];
                a[j as usize] = t;
                i += 1;
                j -= 1;
            }
        }
        // recurse into the smaller part so stack depth stays logarithmic
        if j - lo < hi - i {
            quick_sort(a, lo, j);
            lo = i;
        } else {
            quick_sort(a, i, hi);
            hi = j;
        }
    }
    insertion_sort(a, lo, hi);
}

pub fn sort() -> u64 {
    let mut keys = vec![0u32; SORT_COUNT];
    let mut r = Rng::new(RNG_SEED);
    for i in 0..SORT_COUNT {
        keys[i] = r.key31();
    }
    quick_sort(&mut keys, 0, SORT_COUNT as i64 - 1);
    let mut ordered: u64 = 1;
    for i in 1..SORT_COUNT {
        if keys[i - 1] > keys[i] {
            ordered = 0;
        }
    }
    let mut h = MIX_SEED;
    let mut i = 0;
    while i < SORT_COUNT {
        h = mix(h, keys[i] as u64);
        i += SORT_STRIDE;
    }
    mix(h, ordered)
}
