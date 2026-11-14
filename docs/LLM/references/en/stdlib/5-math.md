# Module std.math

The `std.math` module provides a minimal orthogonal basis of low-level mathematical primitives: IEEE 754 bitwise conversions, hardware rounding and square root instructions, processor bit counters, extended-width arithmetic with carry/borrow flags, and fundamental floating-point constants.

---

## 1. Import: load std.math

Importing the entire package:

```yukino
load std.math
```

All public elements of the `std.math` module are imported into the current scope:

- **IEEE 754 constants:**
  - `NAN_BITS`, `INFINITY_BITS`, `NEG_INFINITY_BITS`;
  - `EPSILON`, `MIN_POSITIVE`, `MAX`, `MIN`;
  - `MANTISSA_BITS`, `EXP_BIAS`.
- **Bitwise conversions (Float Bitcasts):**
  - `to_bits`, `from_bits` (for `float64` / `uint64`);
  - `to_bits32`, `from_bits32` (for `float32` / `uint32`).
- **Special values and classification:**
  - `nan`, `infinity`, `neg_infinity`;
  - `is_nan`, `is_infinite`, `is_finite`.
- **Hardware mathematical operations:**
  - `sqrt`, `fma`, `abs`, `copysign`;
  - `floor`, `ceil`, `trunc`, `round`;
  - `abs_int`, `min`, `max`, `min_int`, `max_int`.
- **Low-level bit counters (Bit Twiddling):**
  - `clz`, `ctz`, `popcount` (for `uint64`);
  - `clz32`, `ctz32`, `popcount32` (for `uint32`).
- **Extended-precision arithmetic (Big-number arithmetic):**
  - `mul_wide` — multiplies two 64-bit numbers returning a pair `(hi, lo)`;
  - `add_carry` — adds two 64-bit numbers with a carry flag `(sum, carry_out)`;
  - `sub_borrow` — subtracts two 64-bit numbers with a borrow flag `(diff, borrow_out)`.

All listed entities are also automatically available when importing the entire standard library: `load std`.

---

## 2. Constants

### 2.1 Bitmask constants

```yukino
public static NAN_BITS: uint64 = 0x7ff8000000000000u64
public static INFINITY_BITS: uint64 = 0x7ff0000000000000u64
public static NEG_INFINITY_BITS: uint64 = 0xfff0000000000000u64
```

- `NAN_BITS` — the canonical quiet NaN (Quiet NaN) bit pattern of the IEEE 754 double precision format.
- `INFINITY_BITS` — the bit representation of positive infinity ($+\infty$).
- `NEG_INFINITY_BITS` — the bit representation of negative infinity ($-\infty$).

### 2.2 Numeric parameters of the IEEE 754 format (float64)

```yukino
public static EPSILON: float64 = 2.2204460492503131e-16
public static MIN_POSITIVE: float64 = 2.2250738585072014e-308
public static MAX: float64 = 1.7976931348623157e+308
public static MIN: float64 = -1.7976931348623157e+308
public static MANTISSA_BITS: int32 = 52
public static EXP_BIAS: int32 = 1023
```

- `EPSILON` — machine epsilon: the difference between $1.0$ and the next representable `float64` number.
- `MIN_POSITIVE` — the smallest positive normalized `float64` number.
- `MAX` — the largest finite positive `float64` number.
- `MIN` — the smallest (largest in absolute value negative) finite `float64` number.
- `MANTISSA_BITS` — the number of fractional part (mantissa) bits in the `float64` format.
- `EXP_BIAS` — the exponent bias of the IEEE 754 format.

---

## 3. Errors and Panics

The `std.math` module does not define error structures. The functions of the module conform to the IEEE 754 specification: mathematically undefined operations (for example, extracting the square root of a negative number) return `NaN`, and division by zero returns `Infinity` or `-Infinity` without interrupting the program with a panic.

---

## 4. Function Reference

### 4.1 to_bits

Performs a bitwise reinterpretation (bitcast) between a 64-bit floating-point number `float64` and a 64-bit unsigned integer `uint64` without changing the memory contents.

```yukino
public fun to_bits(x: float64) -> uint64
```

Returns the raw 64 bits of the floating-point number as `uint64`.

This function never panics.

```yukino
load std.math: to_bits
load std.io: println

set bits = to_bits(1.0)
println("Bits of 1.0: ${bits}") // 0x3ff0000000000000
```

### 4.2 from_bits

Converts a 64-bit integer into a floating-point number `float64` by bitwise reinterpretation without arithmetic conversion.

```yukino
public fun from_bits(b: uint64) -> float64
```

Returns a `float64` number with the bit content of `b`.

This function never panics.

```yukino
load std.math: from_bits
load std.io: println

set val = from_bits(0x3ff0000000000000u64)
println("Value: ${val}") // 1.0
```

### 4.3 to_bits32

Performs a bitwise reinterpretation between a 32-bit floating-point number `float32` and a 32-bit unsigned integer `uint32`.

```yukino
public fun to_bits32(x: float32) -> uint32
```

Returns the bits of the number as `uint32`.

This function never panics.

```yukino
load std.math: to_bits32
load std.io: println

set b = to_bits32(cast(1.0, "float32"))
println("float32 bits: ${b}")
```

### 4.4 from_bits32

Converts a 32-bit integer `uint32` into `float32` by bitwise reinterpretation.

```yukino
public fun from_bits32(b: uint32) -> float32
```

Returns a `float32` number.

This function never panics.

```yukino
load std.math: from_bits32
load std.io: println

set f = from_bits32(0x3f800000u32)
println("Value: ${f}") // 1.0
```

### 4.5 nan

Returns the canonical special Not-a-Number value (Quiet NaN) of the IEEE 754 double precision standard.

```yukino
public fun nan() -> float64
```

Returns the special floating-point value `NaN`.

This function never panics.

```yukino
load std.math: nan, is_nan
load std.io: println

set n = nan()
println("Is NaN: ${is_nan(n)}") // true
```

### 4.6 infinity

Returns the special positive infinity value ($+\infty$) of the IEEE 754 format.

```yukino
public fun infinity() -> float64
```

Returns the positive infinity value.

This function never panics.

```yukino
load std.math: infinity, is_infinite
load std.io: println

set inf = infinity()
println("Infinity: ${is_infinite(inf)}") // true
```

### 4.7 neg_infinity

Returns the special negative infinity value ($-\infty$) of the IEEE 754 format.

```yukino
public fun neg_infinity() -> float64
```

Returns the negative infinity value.

This function never panics.

```yukino
load std.math: neg_infinity, is_infinite
load std.io: println

set neg_inf = neg_infinity()
println("Negative infinity: ${is_infinite(neg_inf)}") // true
```

### 4.8 is_nan

Checks whether the floating-point number is the `NaN` value (Not a Number).

```yukino
public fun is_nan(x: float64) -> bool
```

Returns `true` if the number `x` is `NaN`, otherwise `false`.

This function never panics.

```yukino
load std.math: is_nan, nan
load std.io: println

println("NaN check: ${is_nan(nan())}") // true
```

### 4.9 is_infinite

Checks whether the floating-point number is positive or negative infinity.

```yukino
public fun is_infinite(x: float64) -> bool
```

Returns `true` if the number equals $+\infty$ or $-\infty$, otherwise `false`.

This function never panics.

```yukino
load std.math: is_infinite, infinity
load std.io: println

println("Infinity check: ${is_infinite(infinity())}") // true
```

### 4.10 is_finite

Checks whether the floating-point number is finite (is neither `NaN` nor infinity).

```yukino
public fun is_finite(x: float64) -> bool
```

Returns `true` if the number is finite, otherwise `false`.

This function never panics.

```yukino
load std.math: is_finite
load std.io: println

println("Number is finite: ${is_finite(3.14)}") // true
```

### 4.11 sqrt

Computes the square root of a floating-point number using the processor hardware instruction.

```yukino
public fun sqrt(x: float64) -> float64
```

Returns the square root of `x`. If $x < 0$, returns `NaN`.

This function never panics.

```yukino
load std.math: sqrt
load std.io: println

set root = sqrt(16.0)
println("Square root: ${root}") // 4.0
```

### 4.12 fma

Performs the hardware fused multiply-add operation (Fused Multiply-Add: $a \times b + c$) with a single intermediate rounding.

```yukino
public fun fma(a: float64, b: float64, c: float64) -> float64
```

Returns the exact result of computing $a \times b + c$ without intermediate loss of digits.

This function never panics.

```yukino
load std.math: fma
load std.io: println

set res = fma(2.0, 3.0, 4.0)
println("fma result: ${res}") // 10.0
```


### 4.13 abs

Returns the absolute value (magnitude) of a floating-point number `float64`.

```yukino
public fun abs(x: float64) -> float64
```

Returns a non-negative `float64` number by resetting the sign bit to 0.

This function never panics.

```yukino
load std.math: abs
load std.io: println

set val = abs(-3.14)
println("Absolute value: ${val}") // 3.14
```

### 4.14 abs_int

Returns the absolute value (magnitude) of a 64-bit signed integer `int64`.

```yukino
public fun abs_int(x: int64) -> int64
```

Returns a non-negative integer `int64`.

This function never panics.

```yukino
load std.math: abs_int
load std.io: println

set val = abs_int(-42)
println("Absolute int value: ${val}") // 42
```

### 4.15 copysign

Returns the magnitude of the number `x` with the sign borrowed from the number `sign`.

```yukino
public fun copysign(x: float64, sign: float64) -> float64
```

Returns a floating-point number `float64`.

This function never panics.

```yukino
load std.math: copysign
load std.io: println

set res = copysign(10.0, -1.0)
println("Number with sign: ${res}") // -10.0
```

### 4.16 floor

Rounds a floating-point number down to the nearest integer not exceeding the argument.

```yukino
public fun floor(x: float64) -> float64
```

Returns the rounded-down `float64` number.

This function never panics.

```yukino
load std.math: floor
load std.io: println

set res = floor(3.7)
println("Rounded down: ${res}") // 3.0
```

### 4.17 ceil

Rounds a floating-point number up to the nearest integer not less than the argument.

```yukino
public fun ceil(x: float64) -> float64
```

Returns the rounded-up `float64` number.

This function never panics.

```yukino
load std.math: ceil
load std.io: println

set res = ceil(3.2)
println("Rounded up: ${res}") // 4.0
```

### 4.18 trunc

Discards the fractional part of a floating-point number (rounding toward zero).

```yukino
public fun trunc(x: float64) -> float64
```

Returns the truncated `float64` number.

This function never panics.

```yukino
load std.math: trunc
load std.io: println

set res = trunc(-3.7)
println("Truncated: ${res}") // -3.0
```

### 4.19 round

Rounds a floating-point number to the nearest integer (half-integers are rounded away from zero).

```yukino
public fun round(x: float64) -> float64
```

Returns the rounded `float64` number.

This function never panics.

```yukino
load std.math: round
load std.io: println

set res = round(3.5)
println("Rounded: ${res}") // 4.0
```


### 4.20 min

Returns the smaller of two floating-point numbers `float64`.

```yukino
public fun min(a: float64, b: float64) -> float64
```

Returns the smaller of the two numbers.

This function never panics.

```yukino
load std.math: min
load std.io: println

set m = min(1.5, 2.8)
println("Minimum: ${m}") // 1.5
```

### 4.21 max

Returns the larger of two floating-point numbers `float64`.

```yukino
public fun max(a: float64, b: float64) -> float64
```

Returns the larger of the two numbers.

This function never panics.

```yukino
load std.math: max
load std.io: println

set m = max(1.5, 2.8)
println("Maximum: ${m}") // 2.8
```

### 4.22 min_int

Returns the smaller of two 64-bit signed integers `int64`.

```yukino
public fun min_int(a: int64, b: int64) -> int64
```

Returns the smaller integer `int64`.

This function never panics.

```yukino
load std.math: min_int
load std.io: println

set m = min_int(10, 20)
println("Minimum int: ${m}") // 10
```

### 4.23 max_int

Returns the larger of two 64-bit signed integers `int64`.

```yukino
public fun max_int(a: int64, b: int64) -> int64
```

Returns the larger integer `int64`.

This function never panics.

```yukino
load std.math: max_int
load std.io: println

set m = max_int(10, 20)
println("Maximum int: ${m}") // 20
```

### 4.24 clz

Count Leading Zeros — counts the number of leading zero bits in a 64-bit number `uint64`.

```yukino
public fun clz(x: uint64) -> int32
```

Returns the number of leading zeros from 0 to 64 (returns 64 for zero).

This function never panics.

```yukino
load std.math: clz
load std.io: println

set count = clz(1u64)
println("Leading zeros: ${count}") // 63
```

### 4.25 clz32

Counts the number of leading zero bits in a 32-bit number `uint32`.

```yukino
public fun clz32(x: uint32) -> int32
```

Returns the number of leading zeros from 0 to 32.

This function never panics.

```yukino
load std.math: clz32
load std.io: println

set count = clz32(1u32)
println("Leading zeros: ${count}") // 31
```

### 4.26 ctz

Count Trailing Zeros — counts the number of trailing zero bits in a 64-bit number `uint64`.

```yukino
public fun ctz(x: uint64) -> int32
```

Returns the number of trailing zeros from 0 to 64.

This function never panics.

```yukino
load std.math: ctz
load std.io: println

set count = ctz(8u64)
println("Trailing zeros: ${count}") // 3
```

### 4.27 ctz32

Counts the number of trailing zero bits in a 32-bit number `uint32`.

```yukino
public fun ctz32(x: uint32) -> int32
```

Returns the number of trailing zeros from 0 to 32.

This function never panics.

```yukino
load std.math: ctz32
load std.io: println

set count = ctz32(16u32)
println("Trailing zeros: ${count}") // 4
```

### 4.28 popcount

Population Count — counts the total number of one bits (Hamming weight) in a 64-bit number `uint64`.

```yukino
public fun popcount(x: uint64) -> int32
```

Returns the number of one bits from 0 to 64.

This function never panics.

```yukino
load std.math: popcount
load std.io: println

set ones = popcount(7u64)
println("One bits: ${ones}") // 3
```

### 4.29 popcount32

Counts the total number of one bits in a 32-bit number `uint32`.

```yukino
public fun popcount32(x: uint32) -> int32
```

Returns the number of one bits from 0 to 32.

This function never panics.

```yukino
load std.math: popcount32
load std.io: println

set ones = popcount32(15u32)
println("One bits: ${ones}") // 4
```

### 4.30 mul_wide

Multiplies two 64-bit unsigned integers preserving the full 128-bit result.

```yukino
public fun mul_wide(a: uint64, b: uint64) -> (uint64, uint64)
```

Returns a pair `(hi, lo)`, where `hi` is the high 64 bits of the product and `lo` is the low 64 bits.

This function never panics.

```yukino
load std.math: mul_wide
load std.io: println

set (hi, lo) = mul_wide(0xffffffffffffffffu64, 2u64)
println("hi: ${hi}, lo: ${lo}")
```

### 4.31 add_carry

Adds two 64-bit numbers taking the incoming carry bit `carry_in` into account.

```yukino
public fun add_carry(a: uint64, b: uint64, carry_in: uint8 = 0) -> (uint64, uint8)
```

Returns a pair `(sum, carry_out)`, where `sum` is the low 64 bits of the sum modulo $2^{64}$, and `carry_out` is the outgoing carry bit (1 on overflow, otherwise 0).

This function never panics.

```yukino
load std.math: add_carry
load std.io: println

set (sum, c_out) = add_carry(0xffffffffffffffffu64, 1u64, 0u8)
println("Sum: ${sum}, carry: ${c_out}")
```

### 4.32 sub_borrow

Subtracts two 64-bit unsigned integers taking the incoming borrow bit `borrow_in` into account.

```yukino
public fun sub_borrow(a: uint64, b: uint64, borrow_in: uint8 = 0) -> (uint64, uint8)
```

Returns a pair `(diff, borrow_out)`, where `diff` is the difference modulo $2^{64}$, and `borrow_out` is the borrow flag (1 on borrow, otherwise 0).

This function never panics.

```yukino
load std.math: sub_borrow
load std.io: println

set (diff, b_out) = sub_borrow(0u64, 1u64, 0u8)
println("Difference: ${diff}, borrow: ${b_out}")
```

---

[← Back: Module std.platformio](4-platformio.md) | [Next: Module std.datetime →](6-datetime.md)
