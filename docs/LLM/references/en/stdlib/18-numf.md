# Module std.numf

The `std.numf` (number format) module provides utilities for formatting, base conversion, and human-readable representation of numeric values in Yukino. The module includes arbitrary radix conversions (binary, hexadecimal, octal), string parsing with support for prefixes and separators, grouping by thousands, zero-padded left alignment, byte size conversions (IEC and SI), and metric suffixes.

---

## 1. Import: load std.numf

Importing the entire package:

```yukino
load std.numf
```

When imported, the following exported entities are available:

- **Error Structs:** `NumfError`.
- **Radix Conversion Functions:** `to_base`, `to_hex`, `to_hex_upper`, `to_bin`, `to_oct`, `parse_base`, `parse_hex`, `parse_bin`, `parse_oct`.
- **Formatting and Display Functions:** `pretty_int`, `pretty`, `pretty_float`, `to_fixed`, `pad_left`, `human_bytes`, `human_metric`, `percent`.

All listed entities are also automatically available when loading the entire standard library: `load std`.

---

## 2. Constants

Module `std.numf` does not declare any global constants.

---

## 3. Interfaces

Module `std.numf` does not declare any interfaces.

---

## 4. Data Structures and Panic Errors

### 4.1 NumfError

Error structure used when raising a panic due to invalid arguments passed to functions of this module:

```yukino
public struct NumfError {
    message: string
    code: int32
}
```

The `message` field contains an explanatory description, and `code` represents the error category:
- `101` — Invalid radix (`radix < 2` or `radix > 36`).
- `102` — Negative precision (`precision < 0`) or negative alignment width (`width < 0`).

---

## 5. Classes

Module `std.numf` does not declare any classes.

---

## 6. Function Reference

### 6.1 to_base

Converts a 64-bit signed integer into a string representation in the specified positional base `radix` (from 2 to 36 inclusive). For digits greater than 9, lowercase Latin letters `'a'` through `'z'` are used. For negative numbers, a leading `"-"` sign is prepended.

```yukino
public fun to_base(val: int64, radix: int32) -> string
```

Returns a string value of type `string`. The return value is not nullable.

Raises a panic with structure `NumfError` (code 101) if `radix < 2` or `radix > 36`.

```yukino
load std.numf: to_base
load std.io: println

set b2 = to_base(10, 2)
set b16 = to_base(255, 16)
set b36 = to_base(35, 36)

println(b2)  // "1010"
println(b16) // "ff"
println(b36) // "z"
```

### 6.2 to_hex

Converts a 64-bit signed integer into a lowercase hexadecimal string. If `prefix` is `true`, prepends `"0x"` (or `"-0x"` for negative values).

```yukino
public fun to_hex(val: int64, prefix: bool = false) -> string
```

Returns a string value of type `string`. The return value is not nullable.

The function never raises a panic.

```yukino
load std.numf: to_hex
load std.io: println

println(to_hex(255, false)) // "ff"
println(to_hex(255, true))  // "0xff"
println(to_hex(-255, true)) // "-0xff"
```

### 6.3 to_hex_upper

Converts a 64-bit signed integer into an uppercase hexadecimal string with digits `'A'` through `'F'`. If `prefix` is `true`, prepends `"0x"` (or `"-0x"` for negative values).

```yukino
public fun to_hex_upper(val: int64, prefix: bool = false) -> string
```

Returns a string value of type `string`. The return value is not nullable.

The function never raises a panic.

```yukino
load std.numf: to_hex_upper
load std.io: println

println(to_hex_upper(255, false)) // "FF"
println(to_hex_upper(255, true))  // "0xFF"
println(to_hex_upper(-255, true)) // "-0xFF"
```

### 6.4 to_bin

Converts a 64-bit signed integer into a binary string. If `prefix` is `true`, prepends `"0b"` (or `"-0b"` for negative values).

```yukino
public fun to_bin(val: int64, prefix: bool = false) -> string
```

Returns a string value of type `string`. The return value is not nullable.

The function never raises a panic.

```yukino
load std.numf: to_bin
load std.io: println

println(to_bin(42, false)) // "101010"
println(to_bin(42, true))  // "0b101010"
println(to_bin(-5, true))  // "-0b101"
```

### 6.5 to_oct

Converts a 64-bit signed integer into an octal string. If `prefix` is `true`, prepends `"0o"` (or `"-0o"` for negative values).

```yukino
public fun to_oct(val: int64, prefix: bool = false) -> string
```

Returns a string value of type `string`. The return value is not nullable.

The function never raises a panic.

```yukino
load std.numf: to_oct
load std.io: println

println(to_oct(493, false)) // "755"
println(to_oct(493, true))  // "0o755"
```

### 6.6 parse_base

Parses a string `s` in the positional base `radix` (2 to 36). Supports optional leading `'+'` or `'-'` signs, underscores `'_'` between digits, and standard base prefixes (`"0x"`/`"0X"` for base 16, `"0b"`/`"0B"` for base 2, `"0o"`/`"0O"` for base 8).

```yukino
public fun parse_base(s: string, radix: int32) -> int64?
```

Returns a parsed 64-bit signed integer of type `int64?` or `null` if the string contains invalid characters, is empty, or overflows the `int64` bounds.

Raises a panic with structure `NumfError` (code 101) if `radix < 2` or `radix > 36`.

```yukino
load std.numf: parse_base
load std.io: println

set val = parse_base("1010", 2)
set hex_val = parse_base("0xDEAD_BEEF", 16)

println(val ?? 0)     // 10
println(hex_val ?? 0) // 3735928559
```

### 6.7 parse_hex

Parses a hexadecimal string. Supports lowercase and uppercase digits `'a'` through `'f'`, optional `"0x"` or `"0X"` prefixes, sign prefixes, and underscore separators `'_'`.

```yukino
public fun parse_hex(s: string) -> int64?
```

Returns a parsed integer of type `int64?` or `null` if the string is not a valid hexadecimal number or overflows.

The function never raises a panic.

```yukino
load std.numf: parse_hex
load std.io: println

set val = parse_hex("0xff")
println(val ?? 0) // 255
```

### 6.8 parse_bin

Parses a binary string. Supports `'0'` and `'1'`, optional `"0b"` or `"0B"` prefixes, sign prefixes, and underscore separators `'_'`.

```yukino
public fun parse_bin(s: string) -> int64?
```

Returns a parsed integer of type `int64?` or `null` if the string contains invalid characters or overflows.

The function never raises a panic.

```yukino
load std.numf: parse_bin
load std.io: println

set val = parse_bin("0b1111_0000")
println(val ?? 0) // 240
```

### 6.9 parse_oct

Parses an octal string. Supports digits `'0'` through `'7'`, optional `"0o"` or `"0O"` prefixes, sign prefixes, and underscore separators `'_'`.

```yukino
public fun parse_oct(s: string) -> int64?
```

Returns a parsed integer of type `int64?` or `null` if the string contains invalid digits or overflows.

The function never raises a panic.

```yukino
load std.numf: parse_oct
load std.io: println

set val = parse_oct("0o755")
println(val ?? 0) // 493
```

### 6.10 pretty_int

Formats an integer with thousands grouped by the specified separator string (defaults to space `" "`).

```yukino
public fun pretty_int(val: int64, separator: string = " ") -> string
```

Returns a formatted string of type `string`. The return value is not nullable.

The function never raises a panic.

```yukino
load std.numf: pretty_int
load std.io: println

println(pretty_int(1234567, " ")) // "1 234 567"
println(pretty_int(1234567, ",")) // "1,234,567"
println(pretty_int(-1000000))     // "-1 000 000"
```

### 6.11 pretty

Convenience alias for `pretty_int(val, " ")`. Groups thousands using space separators.

```yukino
public fun pretty(val: int64) -> string
```

Returns a formatted string of type `string`. The return value is not nullable.

The function never raises a panic.

```yukino
load std.numf: pretty
load std.io: println

println(pretty(1000000)) // "1 000 000"
```

### 6.12 to_fixed

Formats a 64-bit floating-point number `val` into a string representation with a fixed number of decimal places specified by `precision`. Rounding follows the standard IEEE 754 round-half-to-even rule (banker's rounding).

```yukino
public fun to_fixed(val: float64, precision: int32) -> string
```

Returns a formatted string of type `string`. The return value is not nullable. Special values return `"NaN"`, `"Infinity"`, or `"-Infinity"`.

Raises a panic with structure `NumfError` (code 102) if `precision < 0`.

```yukino
load std.numf: to_fixed
load std.io: println

println(to_fixed(3.14159, 2)) // "3.14"
println(to_fixed(3.0, 4))     // "3.0000"
println(to_fixed(12.5, 0))    // "12"
println(to_fixed(13.5, 0))    // "14"
```

### 6.13 pretty_float

Formats a floating-point number with a fixed number of decimal digits `precision` and thousands grouped by `separator` in the integer portion.

```yukino
public fun pretty_float(val: float64, precision: int32 = 2, separator: string = " ") -> string
```

Returns a formatted string of type `string`. The return value is not nullable.

Raises a panic with structure `NumfError` (code 102) if `precision < 0`.

```yukino
load std.numf: pretty_float
load std.io: println

println(pretty_float(1234567.8901, 2, " ")) // "1 234 567.89"
println(pretty_float(1234567.8901, 2, ",")) // "1,234,567.89"
```

### 6.14 pad_left

Pads the string representation of an integer `val` on the left using `pad_char` (defaults to `"0"`) until the total length reaches `width`. For negative numbers padded with `"0"`, the leading `"-"` sign is preserved at the beginning (e.g. `"-0042"`).

```yukino
public fun pad_left(val: int64, width: int32, pad_char: string = "0") -> string
```

Returns an aligned string of type `string`. The return value is not nullable.

Raises a panic with structure `NumfError` (code 102) if `width < 0`.

```yukino
load std.numf: pad_left
load std.io: println

println(pad_left(42, 5, "0")) // "00042"
println(pad_left(-42, 5, "0")) // "-0042"
println(pad_left(42, 5, " ")) // "   42"
```

### 6.15 human_bytes

Formats a byte count into a human-readable size string with appropriate unit suffixes. By default, uses binary IEC prefixes with a multiplier of 1024 (`"B"`, `"KiB"`, `"MiB"`, `"GiB"`, `"TiB"`, `"PiB"`, `"EiB"`). If `use_si` is `true`, uses decimal SI prefixes with a multiplier of 1000 (`"B"`, `"KB"`, `"MB"`, `"GB"`, `"TB"`, `"PB"`, `"EB"`).

```yukino
public fun human_bytes(bytes: int64, use_si: bool = false, precision: int32 = 1) -> string
```

Returns a formatted string of type `string`. The return value is not nullable.

Raises a panic with structure `NumfError` (code 102) if `precision < 0`.

```yukino
load std.numf: human_bytes
load std.io: println

println(human_bytes(500))         // "500 B"
println(human_bytes(1024))        // "1.0 KiB"
println(human_bytes(1536))        // "1.5 KiB"
println(human_bytes(1048576))     // "1.0 MiB"
println(human_bytes(1000, true))  // "1.0 KB"
```

### 6.16 human_metric

Formats a floating-point number into a compact metric representation with standard scale suffixes (`"K"`, `"M"`, `"B"`, `"T"`). Numbers with magnitude below 1000 do not receive a suffix.

```yukino
public fun human_metric(val: float64, precision: int32 = 1) -> string
```

Returns a compact string of type `string`. The return value is not nullable. Special values return `"NaN"`, `"Infinity"`, or `"-Infinity"`.

Raises a panic with structure `NumfError` (code 102) if `precision < 0`.

```yukino
load std.numf: human_metric
load std.io: println

println(human_metric(42.0))            // "42"
println(human_metric(1500.0, 1))       // "1.5K"
println(human_metric(2500000.0, 1))    // "2.5M"
println(human_metric(1000000000.0, 1)) // "1.0B"
println(human_metric(-1500.0, 1))      // "-1.5K"
```

### 6.17 percent

Formats a numeric ratio `val` (where `1.0` equals 100%) as a percentage string with a `"%" ` suffix.

```yukino
public fun percent(val: float64, precision: int32 = 1) -> string
```

Returns a percentage string of type `string`. The return value is not nullable. Special values return `"NaN"`, `"Infinity"`, or `"-Infinity"`.

Raises a panic with structure `NumfError` (code 102) if `precision < 0`.

```yukino
load std.numf: percent
load std.io: println

println(percent(0.75, 1))   // "75.0%"
println(percent(0.75, 0))   // "75%"
println(percent(0.1234, 2)) // "12.34%"
println(percent(-0.05, 1))  // "-5.0%"
```

---

[← Back: Module std.locale](17-locale.md) | [Next: Module std.sharedc →](19-sharedc.md)
