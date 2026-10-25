# Compiler Intrinsics

Compiler intrinsics are functions implemented directly by the LLVM code generator and the Yukino runtime rather than by standard library code. They are available globally in any part of a program without using the `load` directive.

> [!NOTE]
> All compiler intrinsics can be invoked either directly by name (`cast(...)`, `exit(...)`, `typeof!(...)`) or using the optional built-in `rt::` namespace prefix (`rt::cast(...)`, `rt::exit(...)`, `rt::typeof!(...)`). Using `rt::` prevents naming collisions with user-defined functions.

---

## Contents

1. [Type introspection: typeof!](#1-type-introspection-typeof)  
   1.1 [Signature and purpose](#11-signature-and-purpose)  
   1.2 [Using typeof! in cast operations](#12-using-typeof-in-cast-operations)
2. [Type casting: cast](#2-type-casting-cast)  
   2.1 [Signature and call syntax](#21-signature-and-call-syntax)  
   2.2 [Matrix of allowed conversions](#22-matrix-of-allowed-conversions)  
   2.3 [Numeric conversions (Numeric <-> Numeric)](#23-numeric-conversions-numeric---numeric)  
   2.4 [Character conversions (char)](#24-character-conversions-char)  
   2.5 [Numbers and strings (Numeric <-> string)](#25-numbers-and-strings-numeric---string)  
   2.6 [Boolean type (bool)](#26-boolean-type-bool)  
   2.7 [Raw pointers and functions (ptr, cfun, null)](#27-raw-pointers-and-functions-ptr-cfun-null)  
   2.8 [Bytes and strings (bytes <-> string)](#28-bytes-and-strings-bytes---string)  
   2.9 [Static validation of constants and error diagnostics](#29-static-validation-of-constants-and-error-diagnostics)  
   2.10 [Null return and panics in the cast function](#210-null-return-and-panics-in-the-cast-function)
3. [Command-line arguments: args and raw_args](#3-command-line-arguments-args-and-raw_args)  
   3.1 [Argument list: args](#31-argument-list-args)  
   3.2 [Raw platform argument string: raw_args](#32-raw-platform-argument-string-raw_args)  
   3.3 [Error behavior: args and raw_args](#33-error-behavior-args-and-raw_args)
4. [Process control and error handling](#4-process-control-and-error-handling)  
   4.1 [Process termination: exit](#41-process-termination-exit)  
   4.2 [Abnormal termination: panic](#42-abnormal-termination-panic)  
   4.3 [Built-in checks: assert, assert_eq, assert_ne](#43-built-in-checks-assert-assert_eq-assert_ne)
5. [Low-level memory operations](#5-low-level-memory-operations)  
   5.1 [The ptr_of operator](#51-the-ptr_of-operator)  
   5.2 [Constructing from raw parts: from_raw_parts](#52-constructing-from-raw-parts-from_raw_parts)  
   5.3 [Null return and behavior on errors](#53-null-return-and-behavior-on-errors)
6. [Built-in methods for slices, arrays, strings, and bytes](#6-built-in-methods-for-slices-arrays-strings-and-bytes)
7. [Summary table of intrinsics: return types, null, and panics](#7-summary-table-of-intrinsics-return-types-null-and-panics)

---

## 1. Type introspection: typeof!

### 1.1 Signature and purpose

```yukino
typeof!(expr: T) -> string
```

- **Applicable to:** expressions of any type `T` (primitives, compound types, structs, slices, functions, and pointers).
- **Return type:** `string` — the canonical string name of the type (for example, `"int32"`, `"float64"`, `"string"`, `"int32[]"`, `"ptr<char>"`).

The `typeof!(expr)` macro is evaluated entirely at compile time. The compiler determines the static type of the expression `expr` and substitutes a string literal describing it. All generated type strings are deduplicated in the `rodata` section of the binary, so the overhead of calling `typeof!` is zero.

```yukino
set count = 42
set count_type = typeof!(count)       // "int32"

set ratio = 3.14
set ratio_type = typeof!(ratio)       // "float64"

set items = [1, 2, 3]
set items_type = typeof!(items)       // "int32[]"
```

### 1.2 Using typeof! in cast operations

`typeof!` can be passed as the second argument to the built-in `cast` function, allowing a value to be cast to the type of an existing variable without manually duplicating the type name:

```yukino
set sample: int32 = 0
set big_num: int64 = 1000

// Casts big_num to the type of the sample variable (int32)
set matched_val = cast(big_num, typeof!(sample))
```

---

## 2. Type casting: cast

Yukino has no implicit type conversions: the compiler never automatically widens or narrows numbers, converts pointers to addresses, or auto-parses strings. Every explicit conversion is performed through the `cast` intrinsic.

### 2.1 Signature and call syntax

```yukino
cast(value: T, target: "target_type" | typeof!(expr)) -> U
```

- **Parameter 1 (`value`):** the source value of type `T`.
- **Parameter 2 (`target`):** the target type `U`. The second argument **must** be either a quoted string literal with a type name (for example, `"int32"`, `"uint8"`, `"ptr<char>"`) or a `typeof!(...)` call. Passing a dynamic string variable is forbidden and aborts compilation with error `E0431`.
- **Return type:** the converted value of the target type `U`.

> [!NOTE]
> An alternative angle-bracket syntax also exists: `cast<T>(value)` (mentioned in the [Basic Syntax](3-basics.md) document). It performs a direct bit-level LLVM coercion of the value to the target type and does **not** go through the full `cast` conversion matrix of this section: runtime functions (number formatting, string parsing, panics) are not invoked for it. For safe conversions involving strings and validation use the `cast(value, "type")` form.

### 2.2 Matrix of allowed conversions

The table below lists all type categories supported by the compiler for the `cast` function:

| Source type `T` | Target type `U` | Compiler behavior and semantics |
| :--- | :--- | :--- |
| `T` | `T` (identity) | Returns the value unchanged (no-op). |
| `int8..int64`, `uint8..uint64` | `int8..int64`, `uint8..uint64` | Signed (`sext`) or unsigned (`zext`) extension, or bit truncation (`trunc`). Literal ranges are checked at compile time. |
| `float32`, `float64` | `float32`, `float64` | Floating-point precision change (`fpext` / `fptrunc`). |
| `int*`, `uint*` | `float32`, `float64` | Integer-to-floating-point conversion (`sitofp` / `uitofp`). |
| `float32`, `float64` | `int*`, `uint*` | Fractional part truncation (`fptosi` / `fptoui`). |
| `char` | `int*`, `uint*` | Obtaining the 32-bit integer Unicode code of the character. |
| `int*`, `uint*` | `char` | Converting an integer code point to a character. |
| `char` | `string` | Creating a single-character string through the runtime. |
| `string` | `char` | Extracting the first character of the string. |
| Any number (`int*`, `uint*`, `float*`) | `string` | Formatting a number into a string representation in the current memory region. |
| `string` | Any number (`int*`, `uint*`, `float*`) | Parsing a string into a numeric value of the corresponding type. Leading and trailing whitespace is trimmed automatically. |
| `bool` | `int*`, `uint*` | Converting a boolean to a number (`false -> 0`, `true -> 1`). |
| `int*`, `uint*` | `bool` | Comparison with zero (`0 -> false`, any nonzero number -> `true`). |
| `ptr<T>` | `ptr<U>` | Casting a raw pointer between incompatible data types. |
| `ptr<T>`, `cfun(...)`, `null` | `ptr<U>`, `cfun(...)` | Casting between raw pointers and C function pointers. |
| `ptr<T>`, `cfun(...)` | `uint64`, `int64` | Converting a pointer to an integer memory address (`ptrtoint`). |
| `uint64`, `int64` | `ptr<T>`, `cfun(...)` | Converting a numeric address to a pointer (`inttoptr`). |
| `ptr<T>`, `cfun(...)` | `float64` | Converting a pointer address to a real number and back. |
| `ptr<T>`, `cfun(...)` | `string` | Formatting a pointer address into a hexadecimal string. |
| `string` | `ptr<T>`, `cfun(...)` | Parsing a string memory address into a raw pointer. |
| `bytes` | `string` | Interpreting a byte slice as a string without copying the buffer. |
| `string` | `bytes` | Interpreting a string as a byte slice without copying the buffer. |
| `bool` | `string` | Formatting a boolean value into a string (`"true"` or `"false"`). |
| `string` | `bool` | Parsing a string into a boolean (`"true"`/`"1"` -> `true`, `"false"`/`"0"` -> `false`). |
| `string` | `char[]` | Converting a string into a slice of Unicode characters (`char[]`) with allocation in the current region. |
| `char[]` | `string` | Constructing a UTF-8 string from a `char[]` slice with allocation in the current region. |

### 2.3 Numeric conversions (Numeric <-> Numeric)

Conversion between any pair of built-in Yukino numeric types is supported (`int8`, `int16`, `int32`, `int64`, `uint8`, `uint16`, `uint32`, `uint64`, `float32`, `float64`):

```yukino
set a: int64 = 1024

// Changing the bit width of integers
set b: int32 = cast(a, "int32")
set c: uint8 = cast(a, "uint8")

// Converting between integers and floating-point numbers
set pi: float64 = 3.14159
set pi_int: int32 = cast(pi, "int32") // 3 (fractional part discarded)
set back_to_float: float32 = cast(pi_int, "float32") // 3.0
```

### 2.4 Character conversions (char)

The `char` type represents a Unicode character. `cast` freely converts it to a numeric code and to a string:

```yukino
set ch: char = 'A'

// Character to integer code
set code: int32 = cast(ch, "int32") // 65

// Integer code to character
set ch_restored: char = cast(66, "char") // 'B'

// Character to string
set str_val: string = cast(ch, "string") // "A"

// String to first character
set first_char: char = cast("Yukino", "char") // 'Y'

// String to slice of Unicode characters (char[])
set letters: char[] = cast("Yukino", "char[]")

// Slice of Unicode characters back to string
set reconstructed: string = cast(letters, "string") // "Yukino"
```

### 2.5 Numbers and strings (Numeric <-> string)

Casting a number to a string automatically formats its value and allocates the result in the active memory region. The reverse cast performs parsing:

```yukino
// Formatting a number into a string
set score: int32 = 4200
set text: string = cast(score, "string") // "4200"

// Parsing a string into a number
set parsed_int: int32 = cast("12345", "int32")
set parsed_float: float64 = cast("3.1415", "float64")
```

### 2.6 Boolean type (bool)

Boolean values convert to integers and back:

```yukino
set is_active: bool = true

// bool to integer (0 or 1)
set flag_num: int32 = cast(is_active, "int32") // 1

// Integer to bool (0 -> false, any nonzero -> true)
set flag_bool: bool = cast(0, "bool") // false
set flag_non_zero: bool = cast(42, "bool") // true

// bool to string ("true" or "false")
set str_true: string = cast(true, "string") // "true"
set str_false: string = cast(false, "string") // "false"

// String to bool ("true"/"1" -> true, "false"/"0" -> false)
set b1: bool = cast("true", "bool") // true
set b2: bool = cast("false", "bool") // false
set b3: bool = cast("1", "bool") // true
set b4: bool = cast("0", "bool") // false
```

Casting a string to `bool` recognizes `"true"` and `"1"` as `true`, and `"false"` and `"0"` as `false` (leading and trailing whitespace is trimmed automatically). If the string is empty or points to `null`, `false` is returned. If an invalid string format is provided, an untyped panic `invalid boolean syntax in string cast to bool` is raised.

### 2.7 Raw pointers and functions (ptr, cfun, null)

For interacting with the system C ABI, FFI, and low-level memory work, `cast` supports pointer and address conversion:

```yukino
set value: int64 = 100
set p_val = ptr_of(value) // ptr<int64>

// Casting between different pointer types
set p_bytes: ptr<uint8> = cast(p_val, "ptr<uint8>")

// Obtaining the numeric memory address
set address: uint64 = cast(p_val, "uint64")

// Restoring a pointer from a numeric address
set p_restored: ptr<int64> = cast(address, "ptr<int64>")

// String to C pointer and formatting the address as a hexadecimal string
set msg = "Buffer\0"
set c_ptr: ptr<char> = cast(msg, "ptr<char>")
set addr_text: string = cast(c_ptr, "string") // e.g. "0x7f3a10"
```

### 2.8 Bytes and strings (bytes <-> string)

The `bytes` type (byte slice `[]uint8`) and the `string` type have compatible buffer descriptor layouts. `cast` performs a fast reinterpretation of memory without extra allocation and copying:

```yukino
set original = "hello payload"

// Viewing a string as a byte slice
set raw_bytes: bytes = cast(original, "bytes")

// Interpreting the byte slice back as a string
set restored_text: string = cast(raw_bytes, "string")
```

### 2.9 Static validation of constants and error diagnostics

The compiler strictly verifies `cast` arguments during static analysis:

1. **Non-constant target type (`E0431`):** the target type must be a string literal or `typeof!(...)`.
2. **Incompatible type pair (`E0430`):** an attempt to cast incompatible structures (for example, a struct to `int32` or a slice to a number) triggers the compile-time error `invalid type cast from 'T' to 'U'`.
3. **Constant literal overflow (`E0430`):** when trying to narrow a numeric literal that does not fit into the bit width of the target type, the compiler halts the build:

```yukino
// Compilation error E0430: value `1000` exceeds 8 bits for target type `uint8`
set bad = cast(1000, "uint8")
```

### 2.10 Null return and panics in the cast function

The behavior of `cast` when handling invalid or boundary values is strictly specified:

#### When cast returns null
When casting to raw pointer types (`ptr<T>`) or C function pointer types (`cfun(...)`):
- `cast(0, "ptr<T>")` returns the null pointer `null`.
- `cast(null, "ptr<T>")` returns `null`.
- `cast("", "ptr<T>")` returns `null`.
- `cast("invalid_hex", "ptr<T>")`: if the string is not a valid hexadecimal memory address (for example, the hexadecimal number cannot be parsed), the function returns `null`.

#### When cast panics
If a value cannot be correctly converted to the requested type, the Yukino runtime throws an **untyped panic**: the exception receives the kind `"Panic"` without a numeric code, and the message describes the failure reason. The typed `StringParseError` structure is not used for `cast` errors — it is only applied by functions of the `std.strings` module.

| Conversion | Panic condition | Panic message |
| :--- | :--- | :--- |
| `string -> char` | The string is empty (`""`) or `null` | `cannot cast empty or null string to char` |
| `string -> char` | The string contains invalid UTF-8 | `invalid utf-8 in string cast to char` |
| `string -> int*` | The string is empty (`""`) or `null` | `cannot cast empty or null string to int64` |
| `string -> int*` | Invalid integer syntax (letters, special characters, overflow) | `invalid integer syntax in string cast to int64` |
| `string -> uint*` | The string is empty (`""`) or `null` | `cannot cast empty or null string to uint64` |
| `string -> uint*` | A negative number or invalid syntax | `invalid integer syntax in string cast to uint64` |
| `string -> float*` | The string is empty (`""`) or `null` | `cannot cast empty or null string to float64` |
| `string -> float*` | Invalid real number syntax | `invalid float syntax in string cast to float64` |

```yukino
// Example of catching an invalid parse error via sus/catch:
sus {
    set num: int32 = cast("not_a_number", "int32")
} catch {
    println("Cast error: " + errkind)
}
```

---

## 3. Command-line arguments: args and raw_args

### 3.1 Argument list: args

```yukino
args() -> string[]
```

- **Applicable to:** called with no arguments `()`.
- **Return type:** `string[]` — a slice of strings containing the user command-line arguments of the process (the argument at index 0, the executable name, is stripped automatically).

```yukino
fun main() {
    set cli_args = args()
    set total: int64 = cli_args.len()
    
    for i in 0..cli_args.len() {
        set arg_val = cli_args[i]
        // Process the arguments
    }
}
```

### 3.2 Raw platform argument string: raw_args

```yukino
raw_args() -> string
```

- **Applicable to:** called with no arguments `()`.
- **Return type:** `string` — the raw contiguous string of system arguments of the process for direct interaction with platform APIs and the C ABI.

### 3.3 Error behavior: args and raw_args

- **Null return:** the `args()` and `raw_args()` functions **never return null** and **never panic**.
  - If the process is started without arguments, `args()` returns a valid empty `string[]` slice of zero length (`args().len() == 0`).
  - `raw_args()` returns a valid empty string `""` of zero length.
  - Without an active memory region both functions also return an empty slice or empty string instead of panicking.
- `raw_args()` joins the user arguments of the process into a single string separated by single spaces.

---

## 4. Process control and error handling

### 4.1 Process termination: exit

```yukino
exit(code: int32) -> never
```

- **Parameter (`code`):** strictly an integer exit code of type `int32`.
- **Return type:** `never` — the function does not return control to the call site.

```yukino
if critical_config_missing {
    exit(1)
}

// Successful exit
exit(0)
```

### 4.2 Abnormal termination: panic and error handling rules

```yukino
panic(payload: S) -> never
```

- **Parameter:** a user-defined error struct `S` (the struct's first string field containing the error message is extracted by the runtime for console output).
- **Return type:** `never` — immediately initiates abnormal process termination or stack unwinding according to the `panic_strategy` setting in `yuki.toml`.

#### Error handling rules in the project:

1. **Missing external value:** if the logic is correct but a value is missing from the outside (for example, a JSON key or an optional parameter) — the function returns `null`.
2. **Runtime error:** a `panic` is thrown. The Yukino compiler strictly requires passing an error struct into `panic(...)` (passing strings or numbers is rejected). Always create a typed error structure with context fields so that the calling code can catch it in a `catch` block:
3. **Completion status:** if a function logically returns nothing, it returns a boolean (`bool`) `true`/`false` so that the calling code can understand the result of the operation.

```yukino
// 1. Attempting to pass a raw string is rejected by the compiler during type checking:
// panic("division by zero attempted") // Compile error

// 2. Error structure with typed context (CORRECT)
struct MathError {
    message: string
    code: int32
}

if divisor == 0 {
    panic(MathError {
        message: "division by zero attempted",
        code: 101,
    })
}
```

> [!TIP]
> A detailed guide to error handling, the `errkind` variable, catch clause forms `catch TempName: Struct`, `catch Struct`, `catch`, and best practices for designing error types is provided in the document: **[14. Error Handling and Exceptions](14-errors.md)**.


### 4.3 Built-in checks: assert, assert_eq, assert_ne

```yukino
assert(condition: bool, [message: string]) -> void
assert_eq(actual: T, expected: T, [message: string]) -> void
assert_ne(actual: T, expected: T, [message: string]) -> void
```

- **Parameters:**
  - `assert`: the `condition: bool` condition and an optional message `message: string`. In the current implementation the compiler only checks the condition: the `message` is not used during code generation.
  - `assert_eq` / `assert_ne`: the `actual` and `expected` values. The comparison is only performed for integer values; values of other types are not compared, and the check automatically passes. The optional `message` is also ignored.
- **Return type:** `void`.
- **Behavior on a failed check:** when a check fails, the runtime throws an **untyped panic** (kind `"Panic"`, no numeric code) with an empty message. No dedicated panic structure for checks exists in the current compiler implementation.

```yukino
set balance = 100

// Basic boolean condition check (throws an untyped panic when violated)
assert(balance > 0)

// Equality check of integer values
assert_eq(balance, 100)

// Inequality check
assert_ne(balance, 0)
```

---

## 5. Low-level memory operations

### 5.1 The ptr_of operator

```yukino
ptr_of(variable: T) -> ptr<T>
```

- **Parameter (`variable`):** the identifier of a local variable (lvalue) of any type `T`.
- **Return type:** a typed raw pointer `ptr<T>`.

```yukino
set counter = 42
set p_counter: ptr<int32> = ptr_of(counter)
```

### 5.2 Constructing from raw parts: from_raw_parts

```yukino
// 1. Constructing a string from a pointer to bytes and a length (2 arguments)
string::from_raw_parts(raw_ptr: ptr<uint8>, length: int64 | int32) -> string

// 2. Constructing a slice from a pointer, a length, and a capacity (3 arguments)
slice::from_raw_parts(raw_ptr: ptr<T>, length: int64 | int32, capacity: int64 | int32) -> T[]
```

- **Parameters:**
  - `string::from_raw_parts` takes a pointer to the start of a byte array `ptr<uint8>` and an integer length.
  - `slice::from_raw_parts` takes a raw pointer `ptr<T>` to an arbitrary element type `T`, a length, and a capacity. The capacity is mandatory: a two-argument call always constructs a string, not a slice.
- Neither function validates the passed values: the descriptor is built directly from the pointer, length, and capacity. There is no `from_utf8` intrinsic in the compiler — UTF-8 validity checking is performed by standard library functions.

```yukino
// Recreating a string from an external buffer
set text = string::from_raw_parts(raw_ptr, length)

// Recreating a slice from a raw buffer
set slice_view = slice::from_raw_parts(raw_items_ptr, 10, 16)
```

### 5.3 Null return and behavior on errors

- **`string::from_raw_parts(raw_ptr, length)`**:
  - Passing `raw_ptr == null` with a zero length produces a correct empty string descriptor `{ ptr: null, len: 0 }`.
  - Passing `raw_ptr == null` with a nonzero length `length > 0` produces a corrupted descriptor; any subsequent read or indexing operation leads to undefined behavior.
  - The function never panics and performs no validation.

- **`slice::from_raw_parts(raw_ptr, length, capacity)`**:
  - Similarly constructs the slice descriptor without validation: the `capacity < length` check and pointer validity are not controlled, and the function never panics.

---

## 6. Built-in methods for slices, arrays, strings, and bytes

The compiler provides a set of built-in methods for collection types (`T[]`, `[N]T`), strings (`string`), and byte slices (`bytes`).

| Method | Allowed receiver types | Return type | Description |
| :--- | :--- | :--- | :--- |
| `.len()` | `string`, `bytes`, `T[]`, `[N]T` | `int64` | The length of a string/bytes in bytes, or the number of elements in a slice or array |
| `.cap()` | `bytes`, `T[]` | `int64` | The currently allocated capacity of a slice or byte buffer |
| `.is_empty()` | `string`, `bytes`, `T[]` | `bool` | Checks for the absence of elements (`.len() == 0`) |
| `.as_ptr()` | `string`, `bytes`, `T[]` | `ptr<uint8>` (for string/bytes), `ptr<T>` (for slices) | A raw pointer to the contiguous element buffer in memory |
| `.push(elem)` | `T[]`, `bytes` | `void` | Appends an element to the end of a slice with dynamic capacity growth in the region |
| `.pop()` | `T[]`, `bytes` | `T?` (for `bytes` — `uint8?`) | Removes and returns the last element of a slice (returns `null` if the slice is empty) |
| `.del(index)` | `T[]`, `bytes` | `void` | Deletes an element by index, shifting subsequent elements to the left |
| `.clear()` | `T[]`, `bytes` | `void` | Resets the slice length to 0 (`len = 0`) without freeing the buffer |
| `.reserve(additional)` | `T[]`, `bytes` | `void` | Reserves additional capacity in advance in the active memory region |
| `.cp()` | `T[]` | `T[]` | Deep copy (clone) of slice elements into the current memory region |

Usage example:

```yukino
set buffer: int64[] = [10, 20, 30]

set length: int64 = buffer.len()     // 3
set capacity: int64 = buffer.cap()   // 3 or more
set empty: bool = buffer.is_empty()  // false
set raw_ptr: ptr<int64> = buffer.as_ptr()

// Dynamically appending elements to the slice
buffer.push(40)
buffer.push(50)

// Popping the last element with a nullable return
set popped = buffer.pop() // popped: int64? (equals 50)
if popped != null {
    println("Popped element: " + cast(popped ?? 0, "string"))
}

// Deleting an element by index, shifting elements
buffer.del(1) // removes the element with the value 20

// Reserving capacity in advance for new elements
buffer.reserve(50)

// Resetting the slice length to 0
buffer.clear()
assert(buffer.is_empty())

// Cloning a slice
set numbers = [1, 2, 3]
set numbers_copy = numbers.cp()

// String length in bytes, emptiness check, and pointer to the bytes
set greeting = "Yukino"
set greeting_len: int64 = greeting.len()
set greeting_empty: bool = greeting.is_empty() // false
set greeting_ptr: ptr<uint8> = greeting.as_ptr()
```

### 6.2 Method safety, null returns, and panics

- **Method `.as_ptr()`**:
  - **Null return:** returns `null` (`nullptr`) if the slice or string is empty (`.len() == 0`) or has no allocated buffer.
  - **Panic:** never panics.

- **Methods `.len()` and `.cap()`**:
  - **Null return:** return a numeric `int64` value (`0` for empty or null structures). Never return `null`.
  - **Panic:** never panic.

- **Method `.is_empty()`**:
  - **Null return:** never returns `null` (always returns `bool`).
  - **Panic:** never panics.

- **Method `.push(elem)`**:
  - **Null return:** returns no value (`void`).
  - **Panic:** never panics. If memory cannot be allocated for the grown buffer (region exhaustion), the element is silently not added.

- **Method `.pop()`**:
  - **Null return:** returns `null` if the slice is empty (`.len() == 0`). For a non-empty slice it returns the last value of `T` and decreases `len` by 1.
  - **Panic:** never panics.

- **Method `.del(index)`**:
  - **Null return:** returns no value (`void`).
  - **Panic:** when the index is out of bounds (`index >= collection.len()`), the runtime throws an **untyped panic** (kind `"Panic"`, no numeric code) with the message `slice index out of bounds in slice.del`.

- **Method `.clear()`**:
  - **Null return:** returns no value (`void`).
  - **Panic:** never panics.

- **Method `.reserve(additional)`**:
  - **Null return:** returns no value (`void`).
  - **Panic:** never panics. If additional memory cannot be reserved, the reservation is silently skipped.

- **Method `.cp()`**:
  - Creates a deep copy of the slice in the active memory region. Requires an active region: without one, compilation fails with the error `array cloning requires an active region`.

- **Indexing and slicing operations (`collection[i]`, `collection[start..end]`):**
  - Indexing and subrange extraction are compiled **without bounds checks**: an index outside `0..len-1` or a violated range `0 <= start <= end <= len` does not trigger a panic and leads to undefined behavior (reading arbitrary memory).
  - For checked access use standard library functions: `std.strings.byte_at` and `std.strings.char_at` (they throw a typed `StringIndexError` panic when out of bounds), while `std.strings.slice` and `std.strings.substring` safely clamp the range to the string bounds.

```yukino
// Example: direct indexing is performed without bounds checks.
// For checked access to string bytes use std.strings.byte_at:
load std.strings: byte_at

set text = "Yukino"
sus {
    set byte_val = byte_at(ref text, 100)
} catch e: StringIndexError {
    println("Index out of range: " + e.message)
}
```

---

## 7. Summary table of intrinsics: return types, null, and panics

Below is the complete reference matrix of built-in compiler functions and operations:

| Intrinsic / Operation | Signature | Can return `null`? (Conditions) | Behavior on error |
| :--- | :--- | :--- | :--- |
| `typeof!(expr)` | `typeof!(T) -> string` | **No** (always a type name) | **Never panics** (evaluated at compile time) |
| `cast(val, "ptr<T>")` | `T -> ptr<T>` | **Yes:** if `val` equals `0`, `null`, `""`, or is not a valid hex address | **Never panics** (an invalid address returns `null`) |
| `cast(str, "char")` | `string -> char` | **No** | Untyped `"Panic"` panic (empty string, `null`, corrupted UTF-8) |
| `cast(str, "int*")` | `string -> int*` | **No** | Untyped `"Panic"` panic (non-numeric characters, empty string, overflow) |
| `cast(str, "uint*")` | `string -> uint*` | **No** | Untyped `"Panic"` panic (negative number, non-numeric characters) |
| `cast(str, "float*")` | `string -> float*` | **No** | Untyped `"Panic"` panic (invalid floating-point number format) |
| `cast(str, "bool")` | `string -> bool` | **No** (an empty string or `null` returns `false`) | Untyped `"Panic"` panic with message `invalid boolean syntax in string cast to bool` |
| `cast(str, "char[]")` | `string -> char[]` | **No** (an empty string returns an empty slice) | Untyped `"Panic"` panic with message `invalid utf-8 in string cast to char[]` |
| `cast(chars, "string")` | `char[] -> string` | **No** (an empty slice returns an empty string) | Untyped `"Panic"` panic with message `invalid unicode codepoint in char[] cast to string` |
| `cast(val, "string")` | `T -> string` | **No** (empty string on zero length) | **Never panics** (for numbers, `bool`, `char`, pointers) |
| `args()` | `() -> string[]` | **No** (an empty slice when there are no arguments or no region) | **Never panics** |
| `raw_args()` | `() -> string` | **No** (the empty string `""` when absent) | **Never panics** |
| `exit(code)` | `int32 -> never` | **No** (does not return control) | **Never panics** (terminates the process) |
| `panic(payload)` | `S -> never` | **No** (does not return control) | The `payload` structure (a developer-defined type, e.g. `MathError`) |
| `assert(cond, [msg])` | `bool, [string] -> void` | **No** | Untyped `"Panic"` panic with an empty message (when `cond == false`) |
| `assert_eq(a, b, [msg])` | `T, T, [string] -> void` | **No** | Untyped `"Panic"` panic with an empty message (when `a != b`; only integers are compared) |
| `assert_ne(a, b, [msg])` | `T, T, [string] -> void` | **No** | Untyped `"Panic"` panic with an empty message (when `a == b`; only integers are compared) |
| `ptr_of(var)` | `T -> ptr<T>` | **No** (a variable's address is nonzero) | **Never panics** |
| `string::from_raw_parts` | `ptr<uint8>, int -> string` | **Yes (inside the descriptor):** if a `null` pointer is passed with length 0 | **Never panics** (no validation; a corrupted descriptor leads to undefined behavior) |
| `slice::from_raw_parts` | `ptr<T>, int, int -> T[]` | **Yes (inside the descriptor):** if a `null` pointer is passed with length 0 | **Never panics** (no validation) |
| `.as_ptr()` | `collection -> ptr<T>` | **Yes:** if the collection is empty or has no buffer | **Never panics** |
| `.len()`, `.cap()` | `collection -> int64` | **No** (returns `0`) | **Never panics** |
| `.is_empty()` | `collection -> bool` | **No** (always `true` or `false`) | **Never panics** |
| `.push(elem)` | `T[] -> void` | **No** | **Never panics** (on region exhaustion the element is silently not added) |
| `.pop()` | `T[] -> T?` | **Yes:** returns `null` if the slice is empty | **Never panics** |
| `.del(index)` | `T[] -> void` | **No** | Untyped `"Panic"` panic with the message `slice index out of bounds in slice.del` (when `index >= len`) |
| `.clear()` | `T[] -> void` | **No** | **Never panics** |
| `.reserve(add)` | `T[] -> void` | **No** | **Never panics** (on region exhaustion the reservation is silently skipped) |
| `.cp()` | `T[] -> T[]` | **No** | **Never panics** (requires an active region, otherwise a compile error) |
| `collection[i]` | Indexing | **No** | **No bounds check** (going outside `0..len-1` does not panic, undefined behavior) |
| `collection[s..e]` | Subrange slicing | **No** | **No bounds check** (a violated range does not panic, undefined behavior) |

---

[← Back: System C ABI and FFI](11-cabi.md) | [Next: Compiler Attributes →](13-attributes.md)
