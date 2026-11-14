# Core Library: core

The `core` package is the fundamental low-level library of the Yukino programming language. It contains runtime primitives, type conversion utilities, low-level memory operations, panic handling, process management, and compiler-integrated macros.

The `core` package is automatically linked by the compiler when listed in the manifest `includes = ["core", "stdlib"]`, and its symbols are imported using the standard `load` directive.

---

## Table of Contents

1. [Architecture and Structure of core](#1-architecture-and-structure-of-core)
2. [Module core.utils: Type Coercion and Assertions](#2-module-coreutils-type-coercion-and-assertions)  
   2.1 [Type Casting: cast](#21-type-casting-cast)  
   2.2 [Conversion Matrix for cast](#22-conversion-matrix-for-cast)  
   2.3 [Assertions: assert, assert_eq, assert_ne](#23-assertions-assert-assert_eq-assert_ne)
3. [Module core.panic: Panic Handling](#3-module-corepanic-panic-handling)
4. [Module core.process: Process Control and Arguments](#4-module-coreprocess-process-control-and-arguments)  
   4.1 [Process Termination: exit](#41-process-termination-exit)  
   4.2 [Command-Line Arguments: args and raw_args](#42-command-line-arguments-args-and-raw_args)
5. [Module core.mem: Low-Level Memory and Arenas](#5-module-coremem-low-level-memory-and-arenas)  
   5.1 [Address-of Operator: ptr_of](#51-address-of-operator-ptr_of)  
   5.2 [Constructing Slices: from_raw_parts](#52-constructing-slices-from_raw_parts)  
   5.3 [Regional Memory Introspection: region_used, region_cap, region_reset](#53-regional-memory-introspection-region_used-region_cap-region_reset)  
   5.4 [Export and Long-Lived Objects: export, persistent](#54-export-and-long-lived-objects-export-persistent)  
   5.5 [First-Class Arenas: class Arena](#55-first-class-arenas-class-arena)
6. [Collection Descriptors: core.slices, core.strings, core.bytes](#6-collection-descriptors-coreslices-corestrings-corebytes)  
   6.1 [Slice Structure and Methods](#61-slice-structure-and-methods)  
   6.2 [String Structure and String Immutability](#62-string-structure-and-string-immutability)  
   6.3 [Bytes Structure](#63-bytes-structure)
7. [Module core.macro: Compiler Macros](#7-module-coremacro-compiler-macros)  
   7.1 [Type Introspection: typeof!](#71-type-introspection-typeof)  
   7.2 [Source Code Macros: file!, line!, col!](#72-source-code-macros-file-line-col)  
   7.3 [Conditional Compilation: compile_print!, compile_error!, cfg!](#73-conditional-compilation-compile_print-compile_error-cfg)  
   7.4 [Resource Embedding: include_str!, include_bytes!](#74-resource-embedding-include_str-include_bytes)  
   7.5 [Build Environment Metadata Macros](#75-build-environment-metadata-macros)
8. [Summary Reference Table](#8-summary-reference-table)

---

## 1. Architecture and Structure of core

The `core` package is partitioned into cohesive modules with distinct responsibilities:

| Module | Exported Symbols | Purpose |
| :--- | :--- | :--- |
| `core.utils` | `cast`, `assert`, `assert_eq`, `assert_ne` | Explicit type casting and invariant assertions |
| `core.panic` | `panic` | Unwind panic invocation and error propagation |
| `core.process` | `exit`, `args`, `raw_args` | Process termination codes and CLI arguments |
| `core.mem` | `ptr_of`, `from_raw_parts`, `region_used`, `region_cap`, `region_reset`, `export`, `persistent`, `Arena` | Pointers, first-class memory arenas, and regional metrics |
| `core.slices` | `Slice` | Low-level slice header layout for `T[]` |
| `core.strings` | `String` | Low-level header layout for immutable `string` |
| `core.bytes` | `Bytes` | Low-level header layout for mutable `bytes` buffer |
| `core.macro` | `typeof`, `file`, `line`, `col`, `cfg`, `compile_print`, `compile_error`, `include_str`, `include_bytes`, `hostenv`, `yukienv`, `is_lib_connected`, `get_features`, `get_profileconfig`, `get_target`, `get_yukic_version` | Built-in compile-time expansion macros |

---

## 2. Module core.utils: Type Coercion and Assertions

Import:
```yukino
load core.utils: cast, assert, assert_eq, assert_ne
```

### 2.1 Type Casting: cast

Yukino employs strict static typing and prohibits implicit type coercions: numeric types do not silently widen or narrow, and pointers do not decay. All explicit type conversions are performed via the `cast` function:

```yukino
cast(value: T, target: "target_type" | typeof!(expr)) -> U
```

- **Argument 1 (`value`):** source value of any valid type `T`.
- **Argument 2 (`target`):** string literal with the target type name (e.g. `"int32"`, `"float64"`, `"string"`, `"ptr<char>"`) or a compile-time `typeof!(expr)` expression.
- **Return value:** the coerced value of target type `U`.

Usage example:
```yukino
load core.utils: cast

set count: int64 = 42
set small: int32 = cast(count, "int32")
set str_val: string = cast(small, "string")
```

### 2.2 Conversion Matrix for cast

| Source Type `T` | Target Type `U` | Semantics |
| :--- | :--- | :--- |
| `int*` / `uint*` | `int*` / `uint*` | Signed or unsigned truncation (`trunc`) or extension (`zext`/`sext`) |
| `int*` / `uint*` | `float*` | Conversion to IEEE-754 floating-point representation |
| `float*` | `int*` / `uint*` | Fractional truncation toward zero |
| `char` | `int*` / `uint*` | Extraction of 32-bit Unicode code point |
| `int*` / `uint*` | `char` | Code point to `char` conversion (invalid code points yield U+FFFD) |
| `int*` / `uint*` | `string` | Base-10 string formatting into active region |
| `float*` | `string` | Decimal floating-point formatting into active region |
| `bool` | `string` | Returns literal `"true"` or `"false"` |
| `string` | `int*` / `uint*` | String parsing; returns 0 on parse failure |
| `string` | `float*` | String parsing to floating-point number |
| `string` | `bool` | `"true"` returns `true`, all other inputs return `false` |
| `bool` | `int*` / `uint*` | `true` -> 1, `false` -> 0 |
| `int*` / `uint*` | `bool` | `0` -> `false`, any non-zero -> `true` |
| `ptr<T>` | `ptr<U>` | Bitwise pointer address reinterpretation |
| `ptr<T>` | `int64` / `uint64` | Raw address to machine-width integer conversion |
| `cfun(...)` | `ptr<void>` | Conversion of C ABI function pointer to generic pointer |
| `bytes` | `string` | UTF-8 validated string creation from byte buffer |
| `string` | `bytes` | Byte slice view of string buffer without copying |
| `bytes` | `uint8[]` | Seamless zero-cost slice descriptor `{ ptr, len, cap }` reinterpretation without allocation or data copy (O(1)) |
| `uint8[]` | `bytes` | Zero-cost conversion of dynamic `uint8[]` array into `bytes` buffer (O(1)) |

```yukino
// Conversion between bytes and uint8[]:
set raw_bytes: bytes = cast("Hello", "bytes")
set u8_slice: uint8[] = cast(raw_bytes, "uint8[]")
set restored_bytes: bytes = cast(u8_slice, "bytes")
```

### 2.3 Assertions: assert, assert_eq, assert_ne

Runtime assertions verify preconditions and postconditions:

```yukino
assert(condition: bool, message?: string) -> void
assert_eq<T>(left: T, right: T, message?: string) -> void
assert_ne<T>(left: T, right: T, message?: string) -> void
```

When condition evaluation fails, the program terminates with the provided diagnostic message.

```yukino
load core.utils: assert, assert_eq

fun test_logic() {
    assert(2 + 2 == 4, "arithmetic invariant failed")
    assert_eq(10 * 10, 100, "multiplication invariant failed")
}
```

---

## 3. Module core.panic: Panic Handling

Import:
```yukino
load core.panic: panic
```

### 3.1 Function panic

The `panic` function triggers a controlled panic on the current thread:

```yukino
panic(payload: any) -> void
```

- Accepts error structs, strings, or primitive values.
- Initiates stack unwinding, which can be trapped via `sus { ... } catch e { ... }`.
- If uncaught, prints the error trace and terminates the process with a non-zero exit code.

```yukino
load core.panic: panic

struct FileError {
    code: int32
    message: string
}

fun open_config(path: string) {
    if path.len() == 0 {
        panic(FileError {
            code: 404,
            message: "file path must not be empty",
        })
    }
}
```

---

## 4. Module core.process: Process Control and Arguments

Import:
```yukino
load core.process: exit, args, raw_args
```

### 4.1 Process Termination: exit

```yukino
exit(code: int32) -> void
```

Terminates the program immediately, returning `code` to the operating system after executing pending `defer` statements in scope.

```yukino
load core.process: exit

fun main() {
    exit(0)
}
```

### 4.2 Command-Line Arguments: args and raw_args

```yukino
args() -> string[]
raw_args() -> string
```

- `args()`: returns an array of individual CLI arguments (argument 0 is the executable path).
- `raw_args()`: returns the raw unparsed command-line string from the operating system.

---

## 5. Module core.mem: Low-Level Memory and Arenas

Import:
```yukino
load core.mem: ptr_of, from_raw_parts, region_used, region_cap, region_reset, export, persistent, Arena
```

### 5.1 Address-of Operator: ptr_of

```yukino
ptr_of<T>(variable: T) -> ptr<T>
```

Returns the raw memory pointer to a variable. Primarily used for C ABI interop (FFI).

### 5.2 Constructing Slices: from_raw_parts

```yukino
from_raw_parts<T>(ptr: ptr<T>, len: int64, cap: int64) -> T[]
```

Constructs a slice `T[]` directly from raw buffer pointer `ptr`, length `len`, and capacity `cap` without allocating new memory.

### 5.3 Regional Memory Introspection: region_used, region_cap, region_reset

- `region_used() -> int64`: bytes currently allocated in the active region.
- `region_cap() -> int64`: total capacity of system chunks reserved for the current region.
- `region_reset() -> void`: resets the region bump-pointer back to the beginning in O(1).

```yukino
load core.mem: region_used, region_cap, region_reset

fun heavy_loop() {
    set i = 0
    while i < 1000 {
        set temp = "iteration data " + cast(i, "string")
        region_reset()
        rewrite i = i + 1
    }
}
```

### 5.4 Export and Long-Lived Objects: export, persistent

- `export<T>(val: T) -> T`: moves `val` from an inner `region` block to the enclosing caller region.
- `persistent<T>(val: T) -> T`: moves `val` into the process-level persistent region surviving thread and stack lifetimes.

### 5.5 First-Class Arenas: class Arena

The `Arena` class provides standalone arenas with explicit lifecycle management:

```yukino
load core.mem: Arena

class Arena {
    public fun init(cap: int64)
    public fun cap() -> int64
    public fun used() -> int64
    public fun reset() -> void
    public fun destroy() -> void
    public fun as_ptr() -> ptr<void>
}
```

Usage example:
```yukino
load core.mem: Arena

fun run_tasks() {
    set pool = init Arena(65536)
    defer {
        pool.destroy()
    }

    region(pool) {
        set packet = "network payload"
        println("Used in pool: ${pool.used()} / ${pool.cap()}")
    }

    pool.reset()
}
```

---

## 6. Collection Descriptors: core.slices, core.strings, core.bytes

### 6.1 Slice Structure and Methods

The slice `T[]` layout:
```yukino
struct Slice {
    ptr: ptr<void>
    len: int64
    cap: int64
}
```

Supported slice methods:
- `.len() -> int64`: element count.
- `.cap() -> int64`: allocated capacity.
- `.as_ptr() -> ptr<T>`: raw pointer to first element.
- `.cp() -> T[]`: shallow copy.
- `.push(item: T) -> void`: append element.
- `.pop() -> T?`: remove and return last element.
- `.del(index: int64) -> void`: remove element by index.
- `.clear() -> void`: set length to 0.
- `.reserve(additional: int64) -> void`: preallocate capacity.

### 6.2 String Structure and String Immutability

The `string` layout:
```yukino
struct String {
    ptr: ptr<uint8>
    len: int64
}
```

Strings in Yukino are strictly **immutable**. Read methods `.len()`, `.as_ptr()`, `.cp()` are supported. Mutating operations (`.push`, `.del`, `.clear`) are rejected by the compiler. For mutable text buffers, use `bytes` or `std.strings`.

### 6.3 Bytes Structure

The `bytes` layout:
```yukino
struct Bytes {
    ptr: ptr<uint8>
    len: int64
    cap: int64
}
```

---

## 7. Module core.macro: Compiler Macros

Built-in macros evaluated at compile time.

### 7.1 Type Introspection: typeof!

```yukino
typeof!(expr) -> string
```

Resolves the static type of `expr` at compile time and emits a deduplicated `rodata` string literal:

```yukino
set count = 42
set t = typeof!(count) // "int32"
```

### 7.2 Source Code Macros: file!, line!, col!

- `file!() -> string`: current source file path.
- `line!() -> int32`: current 1-based source line.
- `col!() -> int32`: current 1-based column.

### 7.3 Conditional Compilation: compile_print!, compile_error!, cfg!

- `compile_print!(msg)`: prints message during compilation.
- `compile_error!(msg)`: aborts compilation with custom diagnostic.
- `cfg!(predicate) -> bool`: evaluates build environment predicate (e.g. `cfg!(target_os = "linux")`).

### 7.4 Resource Embedding: include_str!, include_bytes!

- `include_str!(path) -> string`: embeds UTF-8 file content as a string literal.
- `include_bytes!(path) -> bytes`: embeds binary file content as a byte buffer.

### 7.5 Build Environment Metadata Macros

- `hostenv!(var) -> string`: environment variable value from build host.
- `yukienv!(key, scope) -> string`: reads configuration values from `yuki.toml`.
- `is_lib_connected!(lib, scope) -> bool`: checks if a library dependency is connected.
- `get_features!() -> string[]`: returns list of active build features.
- `get_profileconfig!() -> string`: returns active build profile (`"debug"` or `"release"`).
- `get_target!() -> string`: returns target triple (e.g. `"x86_64-unknown-linux-gnu"`).
- `get_yukic_version!() -> string`: returns `yukic` compiler version.

---

## 8. Summary Reference Table

| Module | Symbol | Signature | Returns | Failure / Panic |
| :--- | :--- | :--- | :--- | :--- |
| `core.utils` | `cast` | `cast(val, "type")` | Coerced value | None (returns default/null on parse error) |
| `core.utils` | `assert` | `assert(cond, msg?)` | `void` | Panics if condition is false |
| `core.utils` | `assert_eq` | `assert_eq(a, b, msg?)` | `void` | Panics if operands differ |
| `core.utils` | `assert_ne` | `assert_ne(a, b, msg?)` | `void` | Panics if operands are equal |
| `core.panic` | `panic` | `panic(payload)` | `void` | Triggers controlled panic / stack unwind |
| `core.process` | `exit` | `exit(code)` | `void` | Immediate process exit |
| `core.process` | `args` | `args()` | `string[]` | Command line arguments list |
| `core.process` | `raw_args` | `raw_args()` | `string` | Unparsed command line string |
| `core.mem` | `ptr_of` | `ptr_of(variable)` | `ptr<T>` | Pointer to variable storage |
| `core.mem` | `from_raw_parts` | `from_raw_parts(p, len, cap)` | `T[]` | Slice wrapping raw buffer |
| `core.mem` | `region_used` | `region_used()` | `int64` | Bytes currently used in local region |
| `core.mem` | `region_cap` | `region_cap()` | `int64` | Total capacity of local region chunks |
| `core.mem` | `region_reset` | `region_reset()` | `void` | Resets bump pointer in O(1) |
| `core.mem` | `export` | `export(val)` | `T` | Promotes value to parent region |
| `core.mem` | `persistent` | `persistent(val)` | `T` | Promotes value to process region |
| `core.mem` | `Arena` | `class Arena` | Arena instance | Methods `.cap()`, `.used()`, `.reset()`, `.destroy()`, `.as_ptr()` |

---

[← Back: C ABI and FFI](11-cabi.md) | [Next: Compiler Attributes →](13-attributes.md)
