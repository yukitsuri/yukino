# Module std.random

The `std.random` module provides pseudorandom number generators (PRNG), the SplitMix64 algorithm, deterministic isolated generators with a given seed, global random value generators, sampling, Gaussian distributions, and UUID version 4 generation.

---

## 1. Import: load std.random

Importing the entire package:

```yukino
load std.random
```

The following are imported into the current scope:

- **Classes and structures:**
  - `class Rng` — an object-oriented generator based on SplitMix64;
  - `struct RandomError` — the error structure raised on invalid generation arguments.
- **Rng instance factories:**
  - `rng_new` — creates a generator initialized with system entropy;
  - `rng_with_seed` — creates a deterministic generator with a given fixed `uint64` seed.
- **Global generation functions:**
  - Primitives: `rand_u64`, `rand_u32`, `rand_i64`, `rand_i32`, `rand_int`, `rand_bool`, `rand_float`, `rand_float32`;
  - Ranges: `rand_range`, `rand_int`;
  - Utilities: `rand_choice_char`, `rand_uuid_v4`, `rand_gaussian`;
  - System entropy: `rand_bytes`, `rand_seed`.

All listed entities are also automatically available when loading the entire standard library: `load std`.

---

## 2. Structures and Classes

### 2.1 RandomError

The error structure raised via `panic` when invalid arguments are passed to generation functions.

```yukino
public struct RandomError {
    message: string
    code: int32
}
```

- `code = 1` — invalid range in `rand_int` and in the `range_int` method of the `Rng` class (`min > max`);
- `code = 2` — invalid range in `rand_range` and in the `range_float` method of the `Rng` class (`min > max`);
- `code = 3` — empty string of options in `rand_choice_char`.

### 2.2 class Rng

An instance-based pseudorandom number generator. Implements the SplitMix64 algorithm with a period of $2^{64}$. Maintains its own isolated state, providing full determinism and reproducibility.

```yukino
public class Rng {
    state: uint64

    fun init(seed: uint64)
    fun next_u64() -> uint64
    fun next_u32() -> uint32
    fun next_i64() -> int64
    fun next_i32() -> int32
    fun next_bool() -> bool
    fun next_float() -> float64
    fun next_float32() -> float32
    fun range_int(min: int64, max: int64) -> int64
    fun range_float(min: float64, max: float64) -> float64
    fun fill_bytes(buf: ptr<uint8>, len: int64)
}
```

The constructor `init(seed: uint64)` sets the initial state of the generator; the seed `0` is automatically replaced with the fixed default value `0x853c49e6748fea9b`.

---

## 3. Rng Class Methods

### 3.1 next_u64

Generates the next 64-bit unsigned pseudorandom number.

```yukino
fun next_u64() -> uint64
```

Returns a pseudorandom value of type `uint64`.

This function never panics.

```yukino
load std.random: rng_new
load std.io: println

set rng = rng_new()
set val = rng.next_u64()
println("Random uint64: ${val}")
```

### 3.2 next_u32

Generates the next 32-bit unsigned number (the high 32 bits of the `next_u64` result).

```yukino
fun next_u32() -> uint32
```

Returns a pseudorandom value of type `uint32`.

This function never panics.

```yukino
load std.random: rng_new
load std.io: println

set rng = rng_new()
set val = rng.next_u32()
println("Random uint32: ${val}")
```

### 3.3 next_i64

Generates the next 64-bit signed pseudorandom number.

```yukino
fun next_i64() -> int64
```

Returns a pseudorandom value of type `int64`.

This function never panics.

```yukino
load std.random: rng_new
load std.io: println

set rng = rng_new()
set val = rng.next_i64()
println("Random int64: ${val}")
```

### 3.4 next_i32

Generates the next 32-bit signed pseudorandom number.

```yukino
fun next_i32() -> int32
```

Returns a pseudorandom value of type `int32`.

This function never panics.

```yukino
load std.random: rng_new
load std.io: println

set rng = rng_new()
set val = rng.next_i32()
println("Random int32: ${val}")
```

### 3.5 next_bool

Generates a pseudorandom boolean value with 50% probability.

```yukino
fun next_bool() -> bool
```

Returns a value of type `bool` (`true` or `false`).

This function never panics.

```yukino
load std.random: rng_new
load std.io: println

set rng = rng_new()
set flag = rng.next_bool()
println("Boolean value: ${flag}")
```

### 3.6 next_float

Generates a floating-point number in the half-open interval `[0.0, 1.0)`.

```yukino
fun next_float() -> float64
```

Returns a number of type `float64` in the range from 0.0 (inclusive) to 1.0 (exclusive).

This function never panics.

```yukino
load std.random: rng_new
load std.io: println

set rng = rng_new()
set prob = rng.next_float()
println("Probability: ${prob}")
```

### 3.7 next_float32

Generates a 32-bit floating-point number in the half-open interval `[0.0, 1.0)`.

```yukino
fun next_float32() -> float32
```

Returns a number of type `float32` in the range from 0.0 (inclusive) to 1.0 (exclusive).

This function never panics.

```yukino
load std.random: rng_new
load std.io: println

set rng = rng_new()
set val = rng.next_float32()
println("float32: ${val}")
```

### 3.8 range_int

Generates a uniformly distributed integer in the closed interval `[min, max]`.

```yukino
fun range_int(min: int64, max: int64) -> int64
```

Returns a random integer `int64` in the range from `min` to `max` inclusive.

If `min > max`, raises a panic with the `RandomError` structure (`code = 1`).

```yukino
load std.random: rng_new, RandomError
load std.io: println

set rng = rng_new()
sus {
    set roll = rng.range_int(1, 6)
    println("Dice roll: ${roll}")
} catch err: RandomError {
    println("Range error: ${err.message}")
}
```

### 3.9 range_float

Generates a uniformly distributed floating-point number in the half-open interval `[min, max)`.

```yukino
fun range_float(min: float64, max: float64) -> float64
```

Returns a number `float64` in the half-open interval `[min, max)`.

If `min > max`, raises a panic with the `RandomError` structure (`code = 2`).

```yukino
load std.random: rng_new, RandomError
load std.io: println

set rng = rng_new()
sus {
    set temp = rng.range_float(15.0, 25.0)
    println("Temperature: ${temp}")
} catch err: RandomError {
    println("Range error: ${err.message}")
}
```

### 3.10 fill_bytes

Fills the passed memory buffer with bytes of a pseudorandom sequence.

```yukino
fun fill_bytes(buf: ptr<uint8>, len: int64)
```

Fills `len` bytes at the pointer `buf` and returns no values.

This function never panics.

```yukino
load std.random: rng_new
load std.io: println

set rng = rng_new()
set raw_mem: uint8[] = []
for i in 0..32 {
    raw_mem.push(0)
}
rng.fill_bytes(raw_mem.as_ptr(), 32)
println("32 bytes filled")
```

---

## 4. Global Module Functions

### 4.1 rng_new

Creates a new isolated `Rng` generator seeded with the system entropy of the operating system (on Unix — from `/dev/urandom`; if the source is unavailable, a software fallback entropy is used).

```yukino
public fun rng_new() -> Rng
```

Returns a new instance of the `Rng` class.

This function never panics.

```yukino
load std.random: rng_new, Rng
load std.io: println

set rng = rng_new()
println("Random number: ${rng.next_u64()}")
```

### 4.2 rng_with_seed

Creates a deterministic `Rng` generator with the given fixed initial seed.

```yukino
public fun rng_with_seed(seed: uint64) -> Rng
```

Returns an `Rng` instance that reproducibly yields an identical sequence for the same `seed`.

This function never panics.

```yukino
load std.random: rng_with_seed, Rng
load std.io: println

set rng = rng_with_seed(42u64)
set first = rng.next_i64()
println("Deterministic value: ${first}")
```

### 4.3 rand_u64

Returns a random 64-bit unsigned number from the global thread-safe generator.

```yukino
public fun rand_u64() -> uint64
```

Returns a random `uint64` value.

This function never panics.

```yukino
load std.random: rand_u64
load std.io: println

set val = rand_u64()
println("Global u64: ${val}")
```

### 4.4 rand_u32

Returns a random 32-bit unsigned number from the global generator.

```yukino
public fun rand_u32() -> uint32
```

Returns a random `uint32` value.

This function never panics.

```yukino
load std.random: rand_u32
load std.io: println

set val = rand_u32()
println("Global u32: ${val}")
```

### 4.5 rand_int

Returns a uniformly distributed integer in the closed interval `[min, max]` from the global generator.

```yukino
public fun rand_int(min: int64, max: int64) -> int64
```

Returns a random integer `int64` between `min` and `max` inclusive.

If `min > max`, raises a panic with the `RandomError` structure (`code = 1`).

```yukino
load std.random: rand_int, RandomError
load std.io: println

sus {
    set n = rand_int(1, 100)
    println("Number: ${n}")
} catch err: RandomError {
    println("Range error: ${err.message}")
}
```

### 4.6 rand_bool

Returns a random boolean value with 50% probability from the global generator.

```yukino
public fun rand_bool() -> bool
```

Returns `true` or `false`.

This function never panics.

```yukino
load std.random: rand_bool
load std.io: println

set flag = rand_bool()
println("Flag: ${flag}")
```

### 4.7 rand_float

Returns a floating-point number in the half-open interval `[0.0, 1.0)` from the global generator.

```yukino
public fun rand_float() -> float64
```

Returns a floating-point number `float64` in the range from 0.0 (inclusive) to 1.0 (exclusive).

This function never panics.

```yukino
load std.random: rand_float
load std.io: println

set f = rand_float()
println("Random float64: ${f}")
```

### 4.8 rand_float32

Returns a 32-bit floating-point number in the half-open interval `[0.0, 1.0)` from the global generator.

```yukino
public fun rand_float32() -> float32
```

Returns a floating-point number `float32`.

This function never panics.

```yukino
load std.random: rand_float32
load std.io: println

set f32 = rand_float32()
println("Random float32: ${f32}")
```

### 4.9 rand_range

Returns a random floating-point number in the half-open interval `[min, max)` from the global generator.

```yukino
public fun rand_range(min: float64, max: float64) -> float64
```

Returns a floating-point number `float64` in the range `[min, max)`.

If `min > max`, raises a panic with the `RandomError` structure (`code = 2`).

```yukino
load std.random: rand_range, RandomError
load std.io: println

sus {
    set speed = rand_range(0.0, 120.0)
    println("Speed: ${speed}")
} catch err: RandomError {
    println("Error: ${err.message}")
}
```

### 4.10 rand_choice_char

Randomly selects one character from the passed string of options.

```yukino
public fun rand_choice_char(chars: string) -> char
```

Returns the selected `char` character.

If the passed string is empty, raises a panic with the `RandomError` structure (`code = 3`).

```yukino
load std.random: rand_choice_char, RandomError
load std.io: println

sus {
    set c = rand_choice_char("0123456789ABCDEF")
    println("Character: ${c}")
} catch err: RandomError {
    println("Error: ${err.message}")
}
```

### 4.11 rand_uuid_v4

Generates a canonical version 4 UUID (universally unique identifier) according to the RFC 4122 standard.

```yukino
public fun rand_uuid_v4() -> string
```

Returns a string of the format `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx`.

This function never panics.

```yukino
load std.random: rand_uuid_v4
load std.io: println

set session_id = rand_uuid_v4()
println("Session ID: ${session_id}")
```

### 4.12 rand_gaussian

Generates a random floating-point number approximately following the normal (Gaussian) distribution with the given mathematical expectation and standard deviation. The value is computed as the sum of 12 uniformly distributed values from `[0.0, 1.0)` minus 6, so the result always lies within `mean ± 6 * std_dev`.

```yukino
public fun rand_gaussian(mean: float64 = 0.0, std_dev: float64 = 1.0) -> float64
```

Returns the generated `float64` value.

This function never panics.

```yukino
load std.random: rand_gaussian
load std.io: println

set sample = rand_gaussian(20.0, 2.5)
println("Gaussian value: ${sample}")
```

### 4.13 rand_bytes

Fills the passed memory buffer with random bytes of system entropy. On Unix, bytes are read from `/dev/urandom`; if the source is unavailable (including on platforms without `/dev/urandom`), bytes are generated by a software generator and are not cryptographically secure. If `buf` is null or `len <= 0`, the function does nothing.

```yukino
public fun rand_bytes(buf: ptr<uint8>, len: int64)
```

Fills `len` bytes at the pointer `buf` and returns no values.

This function never panics.

```yukino
load std.random: rand_bytes
load std.io: println

set key_buf: uint8[] = []
for i in 0..32 {
    key_buf.push(0)
}
rand_bytes(key_buf.as_ptr(), 32)
println("Entropy buffer received")
```

### 4.14 rand_seed

Forcibly sets the initial seed value for the global generator.

```yukino
public fun rand_seed(seed: uint64)
```

Sets the internal state of the global generator and returns no values. The seed `0` is automatically replaced with a fixed default value.

This function never panics.

```yukino
load std.random: rand_seed, rand_u64
load std.io: println

rand_seed(123456789u64)
set a = rand_u64()
println("Seeded number: ${a}")
```

### 4.15 rand_i64

Returns a random 64-bit signed number from the global generator: the result of `rand_u64()` reinterpreted as `int64`.

```yukino
public fun rand_i64() -> int64
```

Returns a pseudorandom value of type `int64`.

This function never panics.

```yukino
load std.random: rand_i64
load std.io: println

set val = rand_i64()
println("Global int64: ${val}")
```

### 4.16 rand_i32

Returns a random 32-bit signed number from the global generator: the result of `rand_u32()` reinterpreted as `int32`.

```yukino
public fun rand_i32() -> int32
```

Returns a pseudorandom value of type `int32`.

This function never panics.

```yukino
load std.random: rand_i32
load std.io: println

set val = rand_i32()
println("Global int32: ${val}")
```

---

[← Back: Module std.datetime](6-datetime.md) | [Next: Module std.fs →](8-fs.md)
