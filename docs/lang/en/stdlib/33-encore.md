# Module std.encore

The `std.encore` module provides a comprehensive suite of tools for low-level and systems Unicode processing: UTF-8 byte sequence validation, extraction and encoding of Unicode scalar values (codepoints), extended grapheme cluster segmentation complying with UAX #29 (including combining accents and zero-width joiner sequences), as well as bidirectional UTF-16 conversions with surrogate pair support for direct Win32 API (`wchar_t*`, `LPCWSTR`) and JVM (`jchar[]`) runtime interoperability.

---

## 1. Import: load std.encore

Importing the entire module:

```yukino
load std.encore
```

All public components of `std.encore` are brought into the current scope:

- **Error Structures:**
  - `UtfError` — error triggered by invalid UTF-8 byte sequences, out-of-range Unicode scalar codepoints, or broken UTF-16 surrogate pairs (codes `101`, `102`, `103`).
- **Data Structures:**
  - `CodepointEntry` — descriptor for an individual Unicode codepoint with its scalar value, byte offset, and UTF-8 byte length;
  - `GraphemeCluster` — descriptor for a grapheme cluster with its text substring, byte offset, and byte length.
- **UTF-8 Functions:**
  - *Validation and conversion:* `is_valid_utf8`, `is_valid_utf8_raw`, `validate_utf8`, `from_utf8_lossy`.
- **Unicode Codepoint Functions:**
  - *Iteration and count:* `count_codepoints`, `decode_codepoints`, `codepoints`;
  - *Encoding:* `encode_codepoint`, `encode_codepoints`.
- **Grapheme Cluster Functions (UAX #29):**
  - *Segmentation and lookup:* `count_graphemes`, `graphemes`, `grapheme_clusters`, `grapheme_at`.
- **UTF-16 Functions (Win32 / JVM Interop):**
  - *UTF-16 encoding:* `to_utf16`, `to_utf16_null_terminated`;
  - *UTF-16 decoding:* `from_utf16`, `from_utf16_lossy`, `from_utf16_null_terminated`;
  - *Surrogate inspection:* `is_high_surrogate`, `is_low_surrogate`, `is_surrogate`.

All listed entities are also automatically available when loading the entire standard library via `load std`.

---

## 2. Constants

No global constants are declared in `std.encore`. All operations are performed using functions, data structures, and error types.

---

## 3. Interfaces

No interfaces are declared in `std.encore`.

---

## 4. Data and Panic Structures

### 4.1 UtfError

An error structure passed to `panic` when invalid UTF-8 byte sequences, illegal Unicode codepoints, or malformed UTF-16 surrogate pairs are encountered during strict validation.

```yukino
public struct UtfError {
    message: string
    code: int32
}
```

- `message` — human-readable error description;
- `code` — numeric error code:
  - `101` — invalid UTF-8 byte sequence during strict validation;
  - `102` — invalid Unicode scalar value (outside `0x0000..0x10FFFF` or in surrogate range `0xD800..0xDFFF`);
  - `103` — invalid UTF-16 surrogate sequence during strict decoding.

### 4.2 CodepointEntry

Represents an individual Unicode codepoint within a string, specifying its value and byte bounds.

```yukino
public struct CodepointEntry {
    codepoint: uint32
    byte_offset: int64
    byte_len: int32
}
```

- `codepoint` — 32-bit unsigned integer scalar value of the Unicode character;
- `byte_offset` — starting byte offset of the codepoint within the string;
- `byte_len` — byte length of the codepoint in UTF-8 (from 1 to 4).

### 4.3 GraphemeCluster

Represents an indivisible user-perceived character (grapheme cluster) according to Unicode standard UAX #29.

```yukino
public struct GraphemeCluster {
    text: string
    byte_offset: int64
    byte_len: int32
}
```

- `text` — substring holding the complete grapheme cluster;
- `byte_offset` — starting byte offset of the cluster in the original text;
- `byte_len` — total byte length of the grapheme cluster.

---

## 5. Classes

No classes are declared in `std.encore`.

---

## 6. Function Reference

### 6.1 is_valid_utf8

Checks whether the provided `bytes` slice forms a well-formed UTF-8 sequence.

```yukino
public fun is_valid_utf8(ref bytes: bytes) -> bool
```

Returns boolean `true` if all bytes constitute valid UTF-8, or `false` otherwise. Not nullable.

This function never triggers a panic.

```yukino
load std.encore: is_valid_utf8
load std.io: println

set raw: bytes = cast("Yukino text", "bytes")
if is_valid_utf8(ref raw) {
    println("Valid UTF-8 sequence")
}
```

### 6.2 is_valid_utf8_raw

Validates a UTF-8 memory buffer by raw pointer `ptr<uint8>` and byte length `len`. Useful in low-level C ABI and FFI interactions.

```yukino
public fun is_valid_utf8_raw(p: ptr<uint8>, len: int64) -> bool
```

Returns `true` if the memory range contains valid UTF-8, otherwise `false`. If `null` is passed, returns `true` only when `len == 0`. Not nullable.

This function never triggers a panic.

```yukino
load std.encore: is_valid_utf8_raw
load std.io: println

set text = "Safety check"
set ok = is_valid_utf8_raw(text.as_ptr(), text.len())
println("Pointer valid: " + cast(ok, "string"))
```

### 6.3 validate_utf8

Strictly validates the given `bytes` slice and returns it as a `string` without redundant memory allocations.

```yukino
public fun validate_utf8(ref bytes: bytes) -> string
```

Returns a validated string of type `string`. Not nullable.

Panics with `UtfError` (code `101`) if the byte sequence is not valid UTF-8.

```yukino
load std.encore: validate_utf8
load std.io: println

set raw: bytes = cast("Hello world", "bytes")
set str = validate_utf8(ref raw)
println("Validated text: " + str)
```

### 6.4 from_utf8_lossy

Performs lossy decoding of a `bytes` slice into a `string`. Any invalid or incomplete byte sequences are replaced with the standard Unicode replacement character `U+FFFD`.

```yukino
public fun from_utf8_lossy(ref bytes: bytes) -> string
```

Returns a string of type `string`. Not nullable.

This function never triggers a panic.

```yukino
load std.encore: from_utf8_lossy
load std.io: println

set raw_buf: uint8[] = []
raw_buf.push(0xFFu8)
raw_buf.push(0xFEu8)
set raw_bytes: bytes = from_raw_parts(raw_buf.as_ptr(), 2, 2)

set safe_str = from_utf8_lossy(ref raw_bytes)
println("Lossy result length: " + cast(safe_str.len(), "string"))
```

### 6.5 codepoints

Splits the string `s` into an array of detailed `CodepointEntry` descriptors containing codepoint values, byte offsets, and byte lengths.

```yukino
public fun codepoints(s: string) -> CodepointEntry[]
```

Returns an array of `CodepointEntry[]`. Returns an empty slice if the string is empty. Not nullable.

This function never triggers a panic.

```yukino
load std.encore: codepoints
load std.io: println

set entries = codepoints("A-B")
for i in 0..entries.len() {
    set e = entries[i]
    println("Codepoint: " + cast(e.codepoint, "string") + " at offset: " + cast(e.byte_offset, "string"))
}
```

### 6.6 decode_codepoints

Extracts a flat array of 32-bit Unicode scalar values (`uint32[]`) from string `s`.

```yukino
public fun decode_codepoints(s: string) -> uint32[]
```

Returns an array of scalar values `uint32[]`. Not nullable.

This function never triggers a panic.

```yukino
load std.encore: decode_codepoints
load std.io: println

set codes = decode_codepoints("Yukino")
for i in 0..codes.len() {
    println("Char code: " + cast(codes[i], "string"))
}
```

### 6.7 encode_codepoint

Encodes an individual 32-bit Unicode codepoint `cp` into a 1 to 4-byte UTF-8 string.

```yukino
public fun encode_codepoint(cp: uint32) -> string
```

Returns a single-character string of type `string`. Not nullable.

Panics with `UtfError` (code `102`) if `cp` is greater than `0x10FFFF` or lies within the surrogate range `0xD800..0xDFFF`.

```yukino
load std.encore: encode_codepoint
load std.io: println

set letter_a = encode_codepoint(65u32)
println("Encoded: " + letter_a)
```

### 6.8 encode_codepoints

Encodes an array of 32-bit scalar values `cps` into a unified UTF-8 string. Any invalid codepoint values are safely replaced with `U+FFFD`.

```yukino
public fun encode_codepoints(ref cps: uint32[]) -> string
```

Returns the encoded string of type `string`. Returns an empty string `""` if the array is empty. Not nullable.

This function never triggers a panic.

```yukino
load std.encore: encode_codepoints
load std.io: println

set list: uint32[] = [72u32, 105u32, 33u32]
set res = encode_codepoints(ref list)
println("Word: " + res)
```

### 6.9 count_codepoints

Returns the total number of Unicode scalar codepoints in string `s`.

```yukino
public fun count_codepoints(s: string) -> int64
```

Returns the count as an `int64`. Not nullable.

This function never triggers a panic.

```yukino
load std.encore: count_codepoints
load std.io: println

set count = count_codepoints("Hello")
println("Codepoints count: " + cast(count, "string"))
```

### 6.10 graphemes

Segments string `s` into user-perceived grapheme clusters according to the Unicode UAX #29 standard, returning an array of string slices.

```yukino
public fun graphemes(s: string) -> string[]
```

Returns an array of strings `string[]`, where each element represents one grapheme cluster. Not nullable.

This function never triggers a panic.

```yukino
load std.encore: graphemes
load std.io: println

set text = "e\u{0301}"
set list = graphemes(text)
println("Graphemes count: " + cast(list.len(), "string"))
```

### 6.11 grapheme_clusters

Segments string `s` and returns detailed `GraphemeCluster[]` descriptors containing the text, starting byte offset, and byte length for each cluster.

```yukino
public fun grapheme_clusters(s: string) -> GraphemeCluster[]
```

Returns an array of `GraphemeCluster[]`. Not nullable.

This function never triggers a panic.

```yukino
load std.encore: grapheme_clusters
load std.io: println

set clusters = grapheme_clusters("Yukino")
for i in 0..clusters.len() {
    set c = clusters[i]
    println("Grapheme: " + c.text + ", bytes: " + cast(c.byte_len, "string"))
}
```

### 6.12 count_graphemes

Calculates the number of indivisible grapheme clusters in string `s`. Correctly handles combining diacritics, emoji sequences, and regional flags.

```yukino
public fun count_graphemes(s: string) -> int64
```

Returns the cluster count as an `int64`. Not nullable.

This function never triggers a panic.

```yukino
load std.encore: count_graphemes
load std.io: println

set total = count_graphemes("cafe\u{0301}")
println("Grapheme count: " + cast(total, "string"))
```

### 6.13 grapheme_at

Retrieves the grapheme cluster at the specified 0-based logical `index`.

```yukino
public fun grapheme_at(s: string, index: int64) -> string?
```

Returns a string of type `string` containing the requested cluster, or `null` if the index is negative or out of bounds. Nullable.

This function never triggers a panic.

```yukino
load std.encore: grapheme_at
load std.io: println

set char_opt = grapheme_at("Hello", 0)
if char_opt != null {
    println("First grapheme: " + (char_opt ?? ""))
}
```

### 6.14 to_utf16

Encodes string `s` into an array of 16-bit code units `uint16[]` (UTF-16). Characters outside the BMP are encoded as surrogate pairs.

```yukino
public fun to_utf16(s: string) -> uint16[]
```

Returns an array of code units `uint16[]`. Not nullable.

This function never triggers a panic.

```yukino
load std.encore: to_utf16
load std.io: println

set u16 = to_utf16("JVM String")
println("UTF-16 units: " + cast(u16.len(), "string"))
```

### 6.15 to_utf16_null_terminated

Encodes string `s` into UTF-16 with an appended null terminator `0u16`. Useful for passing strings directly to Windows Win32 APIs accepting `LPCWSTR` / `wchar_t*`.

```yukino
public fun to_utf16_null_terminated(s: string) -> uint16[]
```

Returns an array of `uint16[]` ending with `0u16`. Array length equals unit count plus one. Not nullable.

This function never triggers a panic.

```yukino
load std.encore: to_utf16_null_terminated
load std.io: println

set wstr = to_utf16_null_terminated("MessageBoxW title")
println("Last zero unit: " + cast(wstr[wstr.len() - 1], "string"))
```

### 6.16 from_utf16

Strictly decodes an array of 16-bit code units `units` into a UTF-8 string.

```yukino
public fun from_utf16(ref units: uint16[]) -> string
```

Returns the decoded string of type `string`. Not nullable.

Panics with `UtfError` (code `103`) if any invalid or incomplete surrogate pairs are encountered.

```yukino
load std.encore: to_utf16, from_utf16
load std.io: println

set original = "Testing UTF-16"
set units = to_utf16(original)
set restored = from_utf16(ref units)
println("Restored: " + restored)
```

### 6.17 from_utf16_lossy

Lossy decodes an array of 16-bit code units `units` into a UTF-8 string. Broken or unmatched surrogate units are replaced with `U+FFFD`.

```yukino
public fun from_utf16_lossy(ref units: uint16[]) -> string
```

Returns a safe string of type `string`. Not nullable.

This function never triggers a panic.

```yukino
load std.encore: from_utf16_lossy
load std.io: println

set bad_units: uint16[] = [0xD800u16, 65u16]
set safe_text = from_utf16_lossy(ref bad_units)
println("Safe text length: " + cast(safe_text.len(), "string"))
```

### 6.18 from_utf16_null_terminated

Decodes a null-terminated UTF-16 string from a raw pointer `ptr<uint16>` (`LPCWSTR` / `wchar_t*`) up to the first null terminator `0u16`.

```yukino
public fun from_utf16_null_terminated(p: ptr<uint16>) -> string
```

Returns a string of type `string`. If `null` is passed, returns an empty string `""`. Not nullable.

This function never triggers a panic.

```yukino
load std.encore: to_utf16_null_terminated, from_utf16_null_terminated
load std.io: println

set win_buffer = to_utf16_null_terminated("Win32 API Output")
set result = from_utf16_null_terminated(win_buffer.as_ptr())
println("Win32 text: " + result)
```

### 6.19 is_high_surrogate

Checks whether a 16-bit code unit `unit` is a leading (high) UTF-16 surrogate in range `0xD800..0xDBFF`.

```yukino
public fun is_high_surrogate(unit: uint16) -> bool
```

Returns boolean `true` if unit is in the high surrogate range, otherwise `false`. Not nullable.

This function never triggers a panic.

```yukino
load std.encore: is_high_surrogate
load std.io: println

set is_high = is_high_surrogate(0xD83Du16)
println("Is high: " + cast(is_high, "string"))
```

### 6.20 is_low_surrogate

Checks whether a 16-bit code unit `unit` is a trailing (low) UTF-16 surrogate in range `0xDC00..0xDFFF`.

```yukino
public fun is_low_surrogate(unit: uint16) -> bool
```

Returns boolean `true` if unit is in the low surrogate range, otherwise `false`. Not nullable.

This function never triggers a panic.

```yukino
load std.encore: is_low_surrogate
load std.io: println

set is_low = is_low_surrogate(0xDE00u16)
println("Is low: " + cast(is_low, "string"))
```

### 6.21 is_surrogate

Checks whether a 16-bit code unit `unit` is any UTF-16 surrogate code unit in range `0xD800..0xDFFF`.

```yukino
public fun is_surrogate(unit: uint16) -> bool
```

Returns boolean `true` if the unit is a surrogate, otherwise `false`. Not nullable.

This function never triggers a panic.

```yukino
load std.encore: is_surrogate
load std.io: println

set check = is_surrogate(65u16)
println("Is surrogate: " + cast(check, "string"))
```

---

[← Previous: Module std.sqlite](32-sqlite.md) | [Next: Third-Party Libraries Catalog →](../../libs-en.md)
