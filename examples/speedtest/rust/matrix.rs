use crate::common::{mix, Rng, MIX_SEED, RNG_SEED};

const MAT_N: usize = 900;
const MAT_CELLS: usize = MAT_N * MAT_N;
const SPECTRAL_N: usize = 1500;
const SPECTRAL_ITERS: u32 = 10;
const SPECTRAL_SCALE: f64 = 1.0e6;

fn multiply(a: &[u32], b: &[u32], c: &mut [u32]) {
    for i in 0..MAT_N {
        for k in 0..MAT_N {
            let aik = a[i * MAT_N + k];
            let brow = &b[k * MAT_N..(k + 1) * MAT_N];
            let crow = &mut c[i * MAT_N..(i + 1) * MAT_N];
            for j in 0..MAT_N {
                crow[j] = crow[j].wrapping_add(aik.wrapping_mul(brow[j]));
            }
        }
    }
}

pub fn matmul() -> u64 {
    let mut a = vec![0u32; MAT_CELLS];
    let mut b = vec![0u32; MAT_CELLS];
    let mut c = vec![0u32; MAT_CELLS];
    let mut r = Rng::new(RNG_SEED);
    for i in 0..MAT_CELLS {
        a[i] = r.key31();
    }
    for i in 0..MAT_CELLS {
        b[i] = r.key31();
    }
    multiply(&a, &b, &mut c);
    let mut h = MIX_SEED;
    for i in 0..MAT_CELLS {
        h = mix(h, c[i] as u64);
    }
    h
}

fn coef(i: u32, j: u32) -> f64 {
    1.0 / (((i + j) * (i + j + 1) / 2 + i + 1) as f64)
}

fn mul_av(v: &[f64], out: &mut [f64]) {
    for i in 0..SPECTRAL_N {
        let mut s = 0.0;
        for j in 0..SPECTRAL_N {
            s += coef(i as u32, j as u32) * v[j];
        }
        out[i] = s;
    }
}

fn mul_atv(v: &[f64], out: &mut [f64]) {
    for i in 0..SPECTRAL_N {
        let mut s = 0.0;
        for j in 0..SPECTRAL_N {
            s += coef(j as u32, i as u32) * v[j];
        }
        out[i] = s;
    }
}

fn mul_atav(v: &[f64], out: &mut [f64], tmp: &mut [f64]) {
    mul_av(v, tmp);
    mul_atv(tmp, out);
}

pub fn spectral() -> u64 {
    let mut u = vec![1.0f64; SPECTRAL_N];
    let mut v = vec![0.0f64; SPECTRAL_N];
    let mut t = vec![0.0f64; SPECTRAL_N];
    for _ in 0..SPECTRAL_ITERS {
        mul_atav(&u, &mut v, &mut t);
        mul_atav(&v, &mut u, &mut t);
    }
    let mut vbv = 0.0;
    let mut vv = 0.0;
    for i in 0..SPECTRAL_N {
        vbv += u[i] * v[i];
        vv += v[i] * v[i];
    }
    mix(mix(MIX_SEED, (vbv * SPECTRAL_SCALE) as u64), (vv * SPECTRAL_SCALE) as u64)
}
