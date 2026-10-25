#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_to_bits_f64(x: f64) -> u64 {
    x.to_bits()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_from_bits_f64(b: u64) -> f64 {
    f64::from_bits(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_to_bits_f32(x: f32) -> u32 {
    x.to_bits()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_from_bits_f32(b: u32) -> f32 {
    f32::from_bits(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_sqrt(x: f64) -> f64 {
    x.sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_fma(a: f64, b: f64, c: f64) -> f64 {
    a.mul_add(b, c)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_abs(x: f64) -> f64 {
    x.abs()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_copysign(x: f64, sign: f64) -> f64 {
    x.copysign(sign)
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_floor(x: f64) -> f64 {
    x.floor()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_ceil(x: f64) -> f64 {
    x.ceil()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_trunc(x: f64) -> f64 {
    x.trunc()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_round(x: f64) -> f64 {
    x.round()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_is_nan(x: f64) -> bool {
    x.is_nan()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_is_infinite(x: f64) -> bool {
    x.is_infinite()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_is_finite(x: f64) -> bool {
    x.is_finite()
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_clz_u64(x: u64) -> i32 {
    x.leading_zeros() as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_ctz_u64(x: u64) -> i32 {
    x.trailing_zeros() as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_popcount_u64(x: u64) -> i32 {
    x.count_ones() as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_clz_u32(x: u32) -> i32 {
    x.leading_zeros() as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_ctz_u32(x: u32) -> i32 {
    x.trailing_zeros() as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_popcount_u32(x: u32) -> i32 {
    x.count_ones() as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_mul_wide_u64(a: u64, b: u64, out_hi: *mut u64) -> u64 {
    let wide = (a as u128) * (b as u128);
    if !out_hi.is_null() {
        unsafe {
            *out_hi = (wide >> 64) as u64;
        }
    }
    wide as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_add_carry_u64(
    a: u64,
    b: u64,
    carry_in: u8,
    out_carry: *mut u8,
) -> u64 {
    let (sum1, c1) = a.overflowing_add(b);
    let (sum2, c2) = sum1.overflowing_add((carry_in != 0) as u64);
    let carry_out = (c1 || c2) as u8;
    if !out_carry.is_null() {
        unsafe {
            *out_carry = carry_out;
        }
    }
    sum2
}

#[unsafe(no_mangle)]
pub extern "C" fn yukistd_math_sub_borrow_u64(
    a: u64,
    b: u64,
    borrow_in: u8,
    out_borrow: *mut u8,
) -> u64 {
    let (diff1, b1) = a.overflowing_sub(b);
    let (diff2, b2) = diff1.overflowing_sub((borrow_in != 0) as u64);
    let borrow_out = (b1 || b2) as u8;
    if !out_borrow.is_null() {
        unsafe {
            *out_borrow = borrow_out;
        }
    }
    diff2
}
