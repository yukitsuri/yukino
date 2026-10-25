# Strings and Bytes

In Yukino, strings are a fundamental built-in type. The compiler provides a rich set of string operations directly in the language core, without the need to connect external modules.

## Contents

1. [Internal string representation and semantics](#1-internal-string-representation-and-semantics)  
   1.1 [What a string is under the hood (The Descriptor)](#11-what-a-string-is-under-the-hood-the-descriptor)  
   1.2 [String as a Copy type: why strings are no longer affine](#12-string-as-a-copy-type-why-strings-are-no-longer-affine)  
   1.3 [Literals in .rodata vs dynamic runtime strings in RAM](#13-literals-in-rodata-vs-dynamic-runtime-strings-in-ram)  
   1.4 [What happens during reassignment and concatenation](#14-what-happens-during-reassignment-and-concatenation)  
   1.5 [What happens during a .cp() call](#15-what-happens-during-a-cp-call)  
   1.6 [How string conversion via cast(x, "string") works](#16-how-string-conversion-via-castx-string-works)
2. [String and byte literals](#2-string-and-byte-literals)  
   2.1 [Ordinary and multiline strings](#21-ordinary-and-multiline-strings)  
   2.2 [Escape sequences](#22-escape-sequences)  
   2.3 [Data of the bytes type](#23-data-of-the-bytes-type)
3. [String interpolation: ${...}](#3-string-interpolation-)
4. [Built-in compiler core operations](#4-built-in-compiler-core-operations)  
   4.1 [Length, emptiness, and raw pointer (.len, .is_empty, .as_ptr)](#41-length-emptiness-and-raw-pointer-len-is_empty-as_ptr)  
   4.2 [Byte indexing](#42-byte-indexing)  
   4.3 [Substring slices: s[start..end]](#43-substring-slices-sstartend)  
   4.4 [Concatenation and comparison](#44-concatenation-and-comparison)  
   4.5 [Copying strings (.cp)](#45-copying-strings-cp)
5. [Interop with C strings](#5-interop-with-c-strings)  
   5.1 [Passing strings to C via .as_ptr()](#51-passing-strings-to-c-via-as_ptr)
   5.2 [cast between a string and a pointer](#52-cast-between-a-string-and-a-pointer)

---

## 1. Internal string representation and semantics

### 1.1 What a string is under the hood (The Descriptor)
A `string` in Yukino is not a separate heap object with headers, capacity fields, or reference counters. It is a compact **descriptor** exactly 16 bytes in size on a 64-bit system (in LLVM IR: `{ ptr, i64 }`):

```text
String: { ptr: pointer to bytes (8 bytes), len: byte length (8 bytes) }
```

The descriptor operates as follows:
- `ptr` is an absolute 64-bit memory address pointing to the first byte of text.
- `len` is the number of bytes in the string.
- The end of the string in memory is computed directly as `ptr + len`.

Because the length is known up front, `.len()` queries and substring slicing run in instantaneous $O(1)$ time without scanning for a null terminator byte `\0`. Strings can safely contain embedded null bytes inside the text.

### 1.2 String as a Copy type: why strings are no longer affine
In earlier versions of Yukino, `string` was treated as an affine type with move semantics. Because of that, passing a string to a function (`println(s)`) or assigning it to another variable consumed the owner variable, forcing developers to write `ref s: string` and pass `ref s` everywhere. That artificial friction was removed: strings were converted into a fully first-class **Copy type**.

Because strings in Yukino are **strictly immutable**:
- Passing a string to a function or performing `set b = a` copies only the **16-byte descriptor** in CPU registers.
- The original variable remains completely valid and is not invalidated.
- Reading strings never requires writing `ref`.

### 1.3 Literals in `.rodata` vs dynamic runtime strings in RAM
String bytes reside in two distinct memory regions, but they are consumed identically through the 16-byte descriptor:

1. **String literals (`"constants"` and `static` declarations):**  
   The compiler places these bytes directly into the read-only data section (`.rodata`) of the binary executable. The operating system maps this section into RAM as **Read-Only**. They persist throughout the entire process lifetime.
2. **Dynamic runtime strings (`read_line()`, `cast(...)`):**  
   These are created during program execution. Their bytes are allocated in RAM within the active **regional bump allocator (Region)** of the current function. Allocation takes only 1 CPU cycle (a pointer bump). When the function or enclosing `region { ... }` block finishes, all regional memory is reclaimed instantaneously in $O(1)$.

### 1.4 What happens during reassignment and concatenation
- **Concatenation (`a + b`):**  
  The `+` operator computes the sum of lengths `a.len() + b.len()`, allocates a new contiguous memory buffer in the current RAM region, copies the bytes of string `a`, then the bytes of string `b`, and returns a new 16-byte descriptor.
- **Reassignment (`rewrite s = s + "!"`):**  
  Reassignment does not mutate or free the existing buffer in memory (it remains safely in the region until scope exit). It mutates **the local variable `s` on the stack itself**: its `ptr` and `len` fields are overwritten with the address and length of the new buffer.

### 1.5 What happens during a .cp() call
Calling `.cp()` on a string returns **the existing descriptor without allocating new memory or copying bytes**.  
Because strings are immutable, two descriptors can safely reference the exact same memory buffer. Physical allocation of a new independent buffer is only performed by `.cp()` for mutable slices (`T[]` and `bytes`).

### 1.6 How string conversion via cast(x, "string") works
The compiler intrinsic `cast(value, "string")` converts values into text:
- **Integers (`int8`..`int64`, `uint8`..`uint64`):** formats the number as decimal text, allocates memory in the current RAM region, and returns the descriptor.
- **Floating-point numbers (`float32`, `float64`):** formats the float into textual form.
- **Characters (`char`):** encodes a 32-bit Unicode code point into a 1-to-4 byte UTF-8 sequence in the current region.
- **Byte slices (`bytes`):** performs a Zero-Copy rewrap: takes the `ptr` and `len` from the slice and constructs a `string` descriptor without reallocating (UTF-8 validation is not performed).
- **Pointers (`ptr<T>`):** formats the address into a hexadecimal string like `"0x7ffd1234"`.

---

## 2. String and byte literals

### 2.1 Ordinary and multiline strings

```yukino
// Single-line string
set greeting = "Hello, Yukino!"

// Multiline string via triple quotes (preserves line breaks and formatting)
set query = """
SELECT id, username, email
FROM users
WHERE active = true
ORDER BY id DESC
"""
```

### 2.2 Escape sequences

The following escape sequences are supported:
- `\n` — newline;
- `\t` — tab;
- `\r` — carriage return;
- `\\` — backslash;
- `\"` — double quote;
- `\'` — single quote;
- `\0` — null byte;
- `\$` — dollar sign (escapes the start of interpolation `${`);
- `\xHH` — 8-bit hexadecimal byte (exactly two hexadecimal digits);
- `\u{H...}` — Unicode code point (1 to 6 hexadecimal digits in braces).

Any other sequence after `\` triggers compilation error E0108.

### 2.3 Data of the bytes type

There is no separate byte-literal syntax of the `b"..."` form in the language. A value of type `bytes` is obtained in one of the following ways:

```yukino
// Converting a string to bytes (the buffer is not copied, cap = len)
set header: bytes = cast("PNG\r\n\x1a\n", "bytes")

// Embedding an entire binary file at compile time
static FONT_DATA: bytes = include_bytes!("assets/font.ttf")
```

The reverse conversion (`cast(data, "string")`) performs no UTF-8 validation.

---

## 3. String interpolation: ${...}

Yukino supports interpolation of arbitrary expressions directly inside string literals via `${expr}`:

```yukino
set name = "Yukino Yukinoshita"
set age = 17

// Built-in interpolation of variables and computations
set message = "Hello, ${name}! In 5 years you will be ${age + 5}."

set couple = { x: "Hikigaya", y: "Yukino" }
set result = "Result: ${couple.x} ${couple.y}"
```

The compiler automatically converts interpolated values to strings without involving external libraries. Integers, floating-point numbers, `char`, and pointers (formatted as a hexadecimal address like `0x...`) are supported. A `bool` value inside interpolation is not supported and results in the compilation error `unsupported bool cast between \`bool\` and \`string\`` — convert it beforehand via `cast(flag, "int32")`.

---

## 4. Built-in compiler core operations

All the operations listed below work at the compiler level itself and are available in any file without importing the standard library.

### 4.1 Length, emptiness, and raw pointer (.len, .is_empty, .as_ptr)

```yukino
set s = "Yukino"

// The number of bytes in O(1)
set byte_count: int64 = s.len()

// Check for an empty string (len == 0)
set nothing: bool = s.is_empty()

// Pointer to the first byte of the buffer in memory (type ptr<uint8>)
set raw_ptr = s.as_ptr()
```

These three methods are the only built-in string methods. Methods such as `.cap()`, `.push()`, and `.pop()` exist only for `T[]` slices and `bytes`; they are not defined for strings.

### 4.2 Byte indexing

String indexing with square brackets `s[i]` returns the byte at the offset, with type `uint8`:

```yukino
set word = "ABC"
set first_byte: uint8 = word[0] // 65 (the code of 'A')
```

> [!WARNING]
> Direct indexing `s[i]` compiles **without bounds checks**: the compiler emits an offset access without comparing against `len`. An out-of-range index does not trigger a panic; instead it leads to undefined behavior (reading foreign memory). For checked access use the `byte_at` function from `std.strings` (or check the length using len).

### 4.3 Substring slices: s[start..end]

A string slice returns a substring of type `string` pointing to a range of bytes of the original string. Open, half-open, and closed ranges are supported:

```yukino
set text = "Hello, world!"

set greeting = text[0..5]   // "Hello" (indices 0 through 4)
set exact    = text[0..=4]  // "Hello" (including index 4)
set rest     = text[7..]    // "world!" (from index 7 to the end)
set prefix   = text[..5]    // "Hello" (from the beginning to 4)
set whole    = text[..]     // "Hello, world!" (the entire string)
```

> [!WARNING]
> Like direct indexing, slices `s[start..end]` compile **without bounds checks**: the result length is computed as `end - start` and the pointer as an offset from the buffer start. Invalid bounds (for example `end > len`) do not trigger a panic; they lead to undefined behavior. For checked slicing use the `slice`/`substring` functions from `std.strings`.

### 4.4 Concatenation and comparison

The `+` operator concatenates strings, allocating a new string in the current region:

```yukino
set part1 = "Yukino "
set part2 = "Yukinoshita"

set full_name = part1 + part2 // "Yukino Yukinoshita"
```

The `==` and `!=` operators compare strings:

```yukino
if full_name == "Yukino Yukinoshita" {
    // the strings are equal
}

if part1 != part2 {
    // the strings are not equal
}
```

Comparison is optimized: the string lengths are checked for equality first ($O(1)$) along with a buffer identity check; byte-by-byte content comparison is invoked only when the lengths match.

### 4.5 Copying strings (.cp)

Since a string in Yukino is a lightweight `{ ptr, len }` descriptor, a plain assignment `set b = a` does not duplicate the data but merely copies the descriptor, referencing the same immutable buffer in memory.

For strings, the `.cp()` method likewise returns the original descriptor without copying: since strings are immutable, physically duplicating the buffer is unnecessary. Real deep copying into a new buffer of the current memory region is performed by `.cp()` only for `T[]` slices and `bytes`:

```yukino
set original = "Yukino"
set clone = original.cp() // the same immutable buffer; no copy needed for strings

set data: bytes = cast("raw", "bytes")
set data_copy = data.cp() // a new independent memory buffer with a full copy of the bytes
```

> [!TIP]
> For more details on the internals of `.cp()`, as well as working with capacity and raw slice pointers, see [Compiler Intrinsics and Core Methods (12-intrinsics.md)](12-intrinsics.md#6-built-in-methods-for-slices-arrays-strings-and-bytes).

---

## 5. Interop with C strings

C libraries expect strings of type `ptr<char>` terminated with a null byte (`\0`). Yukino string literal buffers are already null-terminated, so passing a literal to a C function only requires the `.as_ptr()` method.

### 5.1 Passing strings to C via .as_ptr()

```yukino
externc {
    |cdecl|
    cfun write(fd: int32, buf: ptr<uint8>, count: int64) -> int64;
}

fun main() {
    set text = "Hello, C!\n" // the literal is terminated with \0 in memory
    write(1, text.as_ptr(), text.len())
}
```

`.as_ptr()` returns a pointer of type `ptr<uint8>`; if needed, it can be converted to `ptr<char>` via `cast` (conversion between pointer types is a no-op reinterpretation).

> [!WARNING]
> A guaranteed terminating null byte exists only for literals. Strings assembled at runtime (for example, by concatenation) may not contain a terminating `\0` — pass them to C together with their length (`ptr`, `len`), as in the example above.

### 5.2 cast between a string and a pointer

The `cast` intrinsic between `string` and `ptr<T>` **does not perform C-string conversion**. Its actual semantics is converting an address to text and back:

```yukino
// The string is interpreted as the textual representation of an address:
// an optional "0x" prefix and surrounding whitespace are allowed;
// an empty or invalid string yields a null pointer
set p: ptr<uint8> = cast("0x7ffd1234", "ptr<uint8>")

// A pointer is formatted into a string like "0x7ffd1234"
set text: string = cast(p, "string")
```

---

[← Back: Structures, Classes, and Interfaces](7-oop.md) | [Next: Modules, Imports, and Visibility →](9-modules.md)