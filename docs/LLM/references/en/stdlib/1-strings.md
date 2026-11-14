# Module std.strings

The `std.strings` module provides a comprehensive, high-performance suite of functions and error structures for string operations: substring manipulation, search, comparison, case conversions, formatting, padding, splitting, joining, and primitive type parsing. All inspection, transformation, and search functions take strings by value (`s: string`), as strings in Yukino are lightweight 16-byte Copy descriptors `{ ptr, len }` passed directly in CPU registers without heap allocations.

---

## 1. Import: load std.strings

Importing the entire package:

```yukino
load std.strings
```

All public elements of the `std.strings` module are imported into the current scope:

- **Error structures:**
  - `StringIndexError` — index out of bounds error (code `1`);
  - `StringParseError` — primitive parsing error (codes `10`, `11`, `12`).
- **String functions:**
  - *Length and empty checks:* `utflen`, `is_empty`
  - *Index access:* `byte_at`, `char_at`
  - *Slices and substrings:* `slice`
  - *Comparison:* `eq`, `ne`, `compare`
  - *Search and containment:* `starts_with`, `ends_with`, `contains`, `index_of`, `index_of_from`, `last_index_of`, `count`
  - *Trimming and padding:* `trim`, `trim_start`, `trim_end`, `pad_start`, `pad_end`
  - *Casing and reversal:* `upper`, `lower`, `reverse`
  - *Replacement, repetition, and joining:* `replace`, `repeat`, `split`, `lines`, `str_join`
  - *Safe parsing (returns nullable):* `parse_int`, `parse_float`
  - *Strict parsing (panics on invalid format):* `to_int64`, `to_float64`, `to_bool`, `to_string`

All listed entities are also automatically available when loading the entire standard library: `load std`.

---

## 2. Constants

The `std.strings` module does not declare global constants. All operations are performed via functions and typed error structures.

---

## 3. Data Structures and Panic Errors

The module defines two typed error structures that can be raised via `panic` when exceptional conditions occur.

### 3.1 StringIndexError

Raised when attempting to access a string index outside valid boundaries.

```yukino
public struct StringIndexError {
    message: string
    code: int32
}
```

- `message` — human-readable description (`"string index out of bounds"`).
- `code` — numeric error code: `1`.

### 3.2 StringParseError

Raised when a string cannot be converted into the requested primitive type during strict parsing.

```yukino
public struct StringParseError {
    message: string
    code: int32
}
```

- `message` — human-readable error description (such as `"cannot parse string as int64"`).
- `code` — numeric error code:
  - `10` — `int64` parsing failure;
  - `11` — `float64` parsing failure;
  - `12` — `bool` parsing failure.

---

## 4. Function Reference

### 4.1 utflen

Returns the number of Unicode codepoints in the UTF-8 string `s`. To query the raw byte length, use the built-in `.len()` method.

```yukino
public fun utflen(s: string) -> int64
```

Returns the number of Unicode codepoints as `int64`. Not nullable.

This function never panics.

```yukino
load std.strings: utflen
load std.io: println

set text = "Hello, world!"
set count = utflen(text)
println("Characters: ${count}")
```

### 4.2 is_empty

Checks whether the string `s` is empty (its byte length is zero).

```yukino
public fun is_empty(s: string) -> bool
```

Returns a boolean value of type `bool`. Not nullable.

This function never panics.

```yukino
load std.strings: is_empty
load std.io: println

set text = ""
set empty = is_empty(text)
println("Is empty: ${empty}")
```

### 4.3 byte_at

Returns the byte value at zero-based `index` in string `s`.

```yukino
public fun byte_at(s: string, index: int64) -> uint8
```

Returns a byte numeric value of type `uint8`. Not nullable.

If `index < 0` or `index >= s.len()`, panics with a `StringIndexError` structure (code `1`).

```yukino
load std.strings: byte_at, StringIndexError
load std.io: println

set text = "Yukino"
set code = byte_at(text, 0)
println("Byte: ${code}")
```

### 4.4 char_at

Extracts the byte at zero-based `index` and returns it as a `char` character.

```yukino
public fun char_at(s: string, index: int64) -> char
```

Returns a character value of type `char`. Not nullable.

If `index` is outside `[0, s.len() - 1]`, panics with a `StringIndexError` structure (code `1`).

```yukino
load std.strings: char_at
load std.io: println

set text = "Hello"
set ch = char_at(text, 0)
println("First character: ${ch}")
```

### 4.5 slice

Returns a substring slice from `start` (inclusive) to `end` (exclusive). Index boundaries are automatically clamped to the valid string range: `start` is clamped to `[0, len]`, and `end` to `[start, len]`.

```yukino
public fun slice(s: string, start: int64, end: int64) -> string
```

Returns a substring of type `string`. If the slice length is non-positive, returns an empty string `""`. Not nullable.

This function never panics.

```yukino
load std.strings: slice
load std.io: println

set text = "compiler"
set sub = slice(text, 0, 4)
println("Slice: ${sub}")
```

### 4.6 eq

Performs a byte-by-byte equality check between two strings `a` and `b`.

```yukino
public fun eq(a: string, b: string) -> bool
```

Returns `true` if both strings have equal byte lengths and identical contents, otherwise `false`. Not nullable.

This function never panics.

```yukino
load std.strings: eq
load std.io: println

set s1 = "status"
set s2 = "status"
set match = eq(s1, s2)
println("Equal: ${match}")
```

### 4.7 ne

Performs a byte-by-byte inequality check between two strings `a` and `b`.

```yukino
public fun ne(a: string, b: string) -> bool
```

Returns `true` if strings differ in length or byte contents, otherwise `false`. Not nullable.

This function never panics.

```yukino
load std.strings: ne
load std.io: println

set s1 = "draft"
set s2 = "published"
set diff = ne(s1, s2)
println("Different: ${diff}")
```

### 4.8 compare

Performs byte-wise lexicographical comparison of strings `a` and `b`.

```yukino
public fun compare(a: string, b: string) -> int32
```

Returns `-1` if `a < b`, `0` if `a == b`, and `1` if `a > b`. Not nullable.

This function never panics.

```yukino
load std.strings: compare
load std.io: println

set s1 = "apple"
set s2 = "banana"
set cmp = compare(s1, s2)
println("Comparison: ${cmp}")
```

### 4.9 starts_with

Checks whether string `s` begins with prefix `prefix`. If `prefix` is empty `""`, returns `null`. If the prefix is longer than the string, returns `false`.

```yukino
public fun starts_with(s: string, prefix: string) -> bool?
```

Returns `true` if the prefix matches, `false` if it does not match, or `null` if the prefix is empty `""`. Nullable (`bool?`).

This function never panics.

```yukino
load std.strings: starts_with
load std.io: println

set text = "yukic-driver"
set ok = starts_with(text, "yukic")
println("Prefix matched: ${ok}")
```

### 4.10 ends_with

Checks whether string `s` terminates with suffix `suffix`. If `suffix` is empty `""`, returns `null`. If the suffix is longer than the string, returns `false`.

```yukino
public fun ends_with(s: string, suffix: string) -> bool?
```

Returns `true` if the suffix matches, `false` if it does not match, or `null` if the suffix is empty `""`. Nullable (`bool?`).

This function never panics.

```yukino
load std.strings: ends_with
load std.io: println

set text = "main.ykn"
set ok = ends_with(text, ".ykn")
println("Suffix matched: ${ok}")
```

### 4.11 contains

Checks whether substring `sub` is present within string `s`.

```yukino
public fun contains(s: string, sub: string) -> bool
```

Returns `true` if substring is found, otherwise `false`. Not nullable.

This function never panics.

```yukino
load std.strings: contains
load std.io: println

set text = "standard library"
set has = contains(text, "lib")
println("Contains: ${has}")
```

### 4.12 index_of

Returns the zero-based byte index of the first occurrence of `sub` in string `s`, or `null` if the substring is not found.

```yukino
public fun index_of(s: string, sub: string) -> int64?
```

Returns the zero-based index as `int64`, or `null` if `sub` is not found within `s`. An empty `sub` returns `0`. Nullable (`int64?`).

This function never panics.

```yukino
load std.strings: index_of
load std.io: println

set text = "banana"
set idx = index_of(text, "na")
println("Index: ${idx}")
```

### 4.13 index_of_from

Returns the zero-based byte index of the first occurrence of `sub` in string `s`, starting the search at `from_idx`, or `null` if not found.

```yukino
public fun index_of_from(s: string, sub: string, from_idx: int64) -> int64?
```

Returns the zero-based index as `int64`, or `null` if `sub` is not found or `from_idx >= s.len()`. If `from_idx < 0`, the search begins at index `0`. Nullable (`int64?`).

This function never panics.

```yukino
load std.strings: index_of_from
load std.io: println

set text = "banana"
set idx = index_of_from(text, "na", 3)
println("Index from 3: ${idx}")
```

### 4.14 last_index_of

Returns the zero-based byte index of the last occurrence of `sub` in string `s`, or `null` if the substring is not found.

```yukino
public fun last_index_of(s: string, sub: string) -> int64?
```

Returns the zero-based index as `int64`, or `null` if `sub` is not found. An empty `sub` returns `s.len()`. Nullable (`int64?`).

This function never panics.

```yukino
load std.strings: last_index_of
load std.io: println

set path = "src/driver/main.ykn"
set last = last_index_of(path, "/")
println("Last slash: ${last}")
```

### 4.15 count

Counts the number of non-overlapping occurrences of `sub` in string `s`.

```yukino
public fun count(s: string, sub: string) -> int64
```

Returns the occurrence count as `int64`. If `s` or `sub` is empty, or `sub` is longer than `s`, returns `0`. Not nullable.

This function never panics.

```yukino
load std.strings: count
load std.io: println

set text = "ab_ab_ab"
set n = count(text, "ab")
println("Count: ${n}")
```

### 4.16 trim

Strips all leading and trailing whitespace characters (spaces, tabs, newlines, carriage returns, and other Unicode whitespace) from both ends of `s`.

```yukino
public fun trim(s: string) -> string
```

Returns a trimmed string of type `string`. Not nullable.

This function never panics.

```yukino
load std.strings: trim
load std.io: println

set raw = "  hello world  \n"
set clean = trim(raw)
println("Result: '${clean}'")
```

### 4.17 trim_start

Removes whitespace characters only from the start of string `s`.

```yukino
public fun trim_start(s: string) -> string
```

Returns a string of type `string` with leading whitespace removed. Not nullable.

This function never panics.

```yukino
load std.strings: trim_start
load std.io: println

set raw = "   start_only   "
set clean = trim_start(raw)
println("Result: '${clean}'")
```

### 4.18 trim_end

Removes whitespace characters only from the end of string `s`.

```yukino
public fun trim_end(s: string) -> string
```

Returns a string of type `string` with trailing whitespace removed. Not nullable.

This function never panics.

```yukino
load std.strings: trim_end
load std.io: println

set raw = "   end_only   "
set clean = trim_end(raw)
println("Result: '${clean}'")
```

### 4.19 upper

Converts all characters of string `s` to uppercase following UTF-8 rules.

```yukino
public fun upper(s: string) -> string
```

Returns an uppercase string of type `string`. Not nullable.

This function never panics.

```yukino
load std.strings: upper
load std.io: println

set text = "yukino"
set s = upper(text)
println("Upper: ${s}")
```

### 4.20 lower

Converts all characters of string `s` to lowercase following UTF-8 rules.

```yukino
public fun lower(s: string) -> string
```

Returns a lowercase string of type `string`. Not nullable.

This function never panics.

```yukino
load std.strings: lower
load std.io: println

set text = "YUKINO"
set s = lower(text)
println("Lower: ${s}")
```

### 4.21 replace

Replaces all non-overlapping occurrences of `from_sub` with `to_sub` in string `s`. If `from_sub` is not found, returns the original string. An empty `from_sub` matches every position of the string: `to_sub` is inserted at the beginning and at the end of the string, as well as between every pair of adjacent characters.

```yukino
public fun replace(s: string, from_sub: string, to_sub: string) -> string
```

Returns the transformed string of type `string`. Not nullable.

This function never panics.

```yukino
load std.strings: replace
load std.io: println

set text = "one apple, two apples"
set res = replace(text, "apple", "orange")
println("Result: ${res}")
```

### 4.22 repeat

Constructs a new string by repeating string `s` a specified number of times `count_num`. If `count_num <= 0` or the string is empty, returns an empty string `""`.

```yukino
public fun repeat(s: string, count_num: int64) -> string
```

Returns the repeated string of type `string`. Not nullable.

This function never panics.

```yukino
load std.strings: repeat
load std.io: println

set dash = "-"
set border = repeat(dash, 5)
println("Border: ${border}")
```

### 4.23 reverse

Returns a string with its characters reversed in order while preserving valid Unicode sequence boundaries.

```yukino
public fun reverse(s: string) -> string
```

Returns the reversed string of type `string`. Not nullable.

This function never panics.

```yukino
load std.strings: reverse
load std.io: println

set text = "hello"
set rev = reverse(text)
println("Reversed: ${rev}")
```

### 4.24 pad_start

Pads string `s` on the left with `pad` until it reaches `target_len`. The `pad` parameter defaults to `" "` (single space). If string length is already greater than or equal to `target_len`, or the padding string is empty, returns `s` unmodified.

```yukino
public fun pad_start(s: string, target_len: int64, pad: string = " ") -> string
```

Returns the padded string of type `string`. Not nullable.

This function never panics.

```yukino
load std.strings: pad_start
load std.io: println

set text = "42"
set num = pad_start(text, 5, "0")
println("Result: ${num}")
```

### 4.25 pad_end

Pads string `s` on the right with `pad` until it reaches `target_len`. The `pad` parameter defaults to `" "` (single space). If string length is already greater than or equal to `target_len`, or the padding string is empty, returns `s` unmodified.

```yukino
public fun pad_end(s: string, target_len: int64, pad: string = " ") -> string
```

Returns the padded string of type `string`. Not nullable.

This function never panics.

```yukino
load std.strings: pad_end
load std.io: println

set text = "abc"
set padded = pad_end(text, 6)
println("Result: '${padded}'")
```

### 4.26 split

Splits string `s` into an array of substrings `string[]` by delimiter `delimiter`. If `s` is empty, returns an empty array. If `delimiter` is empty, returns a single-element array containing `s`.

```yukino
public fun split(s: string, delimiter: string) -> string[]
```

Returns an array of strings `string[]`. Not nullable.

This function never panics.

```yukino
load std.strings: split
load std.io: println

set fruits = "apple,orange,banana"
set tokens = split(fruits, ",")
println("Count: ${tokens.len()}")
```

### 4.27 lines

Splits multiline text `s` into an array of individual lines `string[]`. Standard line breaks (LF `\n` and CRLF `\r\n`) are recognized and excluded from resulting elements.

```yukino
public fun lines(s: string) -> string[]
```

Returns an array of lines `string[]`. Not nullable.

This function never panics.

```yukino
load std.strings: lines
load std.io: println

set poem = "line one\nline two\nline three"
set rows = lines(poem)
println("Lines: ${rows.len()}")
```

### 4.28 str_join

Joins all elements in array `items` into a single string using delimiter `delimiter`. Defaults to `""`. If the array is empty, returns an empty string `""`.

```yukino
public fun str_join(items: string[], delimiter: string = "") -> string
```

Returns the joined string of type `string`. Not nullable.

This function never panics.

```yukino
load std.strings: str_join
load std.io: println

set elements = ["yukic", "span", "parser"]
set joined = str_join(elements, " -> ")
println("Joined: ${joined}")
```

### 4.29 parse_int

Attempts to parse string `s` into a 64-bit signed integer. Leading and trailing whitespace characters are trimmed.

```yukino
public fun parse_int(s: string) -> int64?
```

Returns the parsed integer of type `int64` or `null` if the string contains an invalid integer format or overflows.

This function never panics.

```yukino
load std.strings: parse_int
load std.io: println

set text = "12345"
set num = parse_int(text)
if num != null {
    println("Number: ${num ?? 0}")
}
```

### 4.30 parse_float

Attempts to parse string `s` into a 64-bit double precision float. Leading and trailing whitespace characters are trimmed.

```yukino
public fun parse_float(s: string) -> float64?
```

Returns the parsed float value of type `float64` or `null` if the format is invalid.

This function never panics.

```yukino
load std.strings: parse_float
load std.io: println

set text = "3.1415"
set val = parse_float(text)
if val != null {
    println("Value: ${val ?? 0.0}")
}
```

### 4.31 to_int64

Strictly parses string `s` into an `int64` integer. Leading and trailing whitespace is trimmed.

```yukino
public fun to_int64(s: string) -> int64
```

Returns a parsed integer of type `int64`. Not nullable.

If string `s` is not a valid integer representation or exceeds `int64` range, panics with a `StringParseError` structure (code `10`).

```yukino
load std.strings: to_int64, StringParseError
load std.io: println

sus {
    set text = "   42000   "
    set number = to_int64(text)
    println("Number: ${number}")
} catch e: StringParseError {
    println("Error: ${e.message}")
}
```

### 4.32 to_float64

Strictly parses string `s` into a double-precision floating-point number of type `float64`. Leading and trailing whitespace is trimmed.

```yukino
public fun to_float64(s: string) -> float64
```

Returns a parsed float of type `float64`. Not nullable.

If format is invalid, panics with a `StringParseError` structure (code `11`).

```yukino
load std.strings: to_float64, StringParseError
load std.io: println

sus {
    set text = "3.14159265"
    set val = to_float64(text)
    println("Value: ${val}")
} catch e: StringParseError {
    println("Error: ${e.message}")
}
```

### 4.33 to_bool

Strictly parses a boolean value from string `s` (after automatic whitespace trimming). Recognized values `"true"`, `"1"`, `"yes"`, `"on"` return `true`. Values `"false"`, `"0"`, `"no"`, `"off"` return `false`.

```yukino
public fun to_bool(s: string) -> bool
```

Returns a boolean value of type `bool`. Not nullable.

For all other string values, panics with a `StringParseError` structure (code `12`).

```yukino
load std.strings: to_bool, StringParseError
load std.io: println

sus {
    set text = "yes"
    set flag = to_bool(text)
    if flag {
        println("Flag is true")
    }
} catch e: StringParseError {
    println("Error: ${e.message}")
}
```

### 4.34 to_string

Identity string conversion: takes string `s` by value and returns its value. Useful for callback handlers or type coercion boundaries.

```yukino
public fun to_string(s: string) -> string
```

Returns the string value of type `string`. Not nullable.

This function never panics.

```yukino
load std.strings: to_string
load std.io: println

set text = "hello"
set s = to_string(text)
println("String: ${s}")
```

---

[← Previous: Standard Library Overview](0-overview.md) | [Next: Module std.io →](2-io.md)
