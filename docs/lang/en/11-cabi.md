# System C ABI and C FFI

Yukino is designed for seamless interoperability with C, C++, Rust, and operating system libraries. The language supports direct declaration of external C functions, C structures with a controlled memory layout, custom calling conventions, and C ABI function exports.

---

## Contents

1. [External C Declarations and the externc Block](#1-external-c-declarations-and-the-externc-block)  
   1.1 [Top-Level Standalone Declarations (without externc)](#11-top-level-standalone-declarations-without-externc)  
   1.2 [The Grouping externc Block](#12-the-grouping-externc-block)  
   1.3 [Declaring C Functions: cfun](#13-declaring-c-functions-cfun)  
   1.4 [C Attribute Syntax: \|attr1, attr2\|](#14-c-attribute-syntax-attr1-attr2)  
   1.5 [Calling Conventions](#15-calling-conventions)  
   1.6 [Linkage and Optimization C Attributes](#16-linkage-and-optimization-c-attributes)  
   1.7 [Termination Syntax: the Semicolon (;)](#17-termination-syntax-the-semicolon-)
2. [C Structures: cstruct and Memory Layout](#2-c-structures-cstruct-and-memory-layout)  
   2.1 [Declaration and Field Access](#21-declaration-and-field-access)  
   2.2 [The packed and align Attributes](#22-the-packed-and-align-attributes)
3. [Exporting Yukino Functions to the C ABI](#3-exporting-yukino-functions-to-the-c-abi)
4. [Raw Pointers: ptr\<T\> and the ptr_of Operator](#4-raw-pointers-ptrt-and-the-ptr_of-operator)
5. [Integrating C Files into the Build Process](#5-integrating-c-files-into-the-build-process)
6. [Comprehensive Example: Different Types in C and Yukino](#6-comprehensive-example-different-types-in-c-and-yukino)

---

## 1. External C Declarations and the externc Block

To import functions and structures from external object files (`.o`), static libraries (`.a`), shared libraries (`.so`/`.dll`), or the C system runtime into Yukino, the `cfun` and `cstruct` keywords are used.

> [!NOTE]
> The `externc { ... }` block **is not mandatory**. `cfun` functions and `cstruct` structures are full-fledged top-level module elements and can be declared either standalone or grouped inside an `externc` block.

> [!TIP]
> **Invoking via the `extern::` namespace:**
> Declared C functions can be called either directly by name (`puts(ptr)`) or with the `extern::` namespace prefix (`extern::puts(ptr)`). Using `extern::` guarantees that the call resolves to the C ABI function and prevents collisions if a Yukino function with the same name is declared in the project.

### 1.1 Top-Level Standalone Declarations (without externc)

If you need to import one or two external functions or define a C-compatible structure, you can declare them directly in the file's global scope:

```yukino
// C function declaration without externc:
|cdecl|
cfun puts(s: ptr<c_char>) -> c_int;

|cdecl|
cfun abs(x: int32) -> int32;

|noreturn, cold|
cfun exit(code: int32) -> void;

// Top-level C structure declaration:
|packed|
cstruct PacketHeader {
    magic: uint16,
    length: uint32,
}
```

### 1.2 The Grouping externc Block

The `externc` block is convenient for logically organizing multiple external declarations, as well as for explicitly binding to a C header during integration:

```yukino
externc {
    // Importing functions from the Libc standard library
    |cdecl|
    cfun free(p: ptr<void>) -> void;

    |cdecl, returns_nonnull|
    cfun malloc(size: int64) -> ptr<void>;
}

// Specifying the header file name (for meta-information and C translators):
externc "math.h" {
    cfun sin(x: float64) -> float64;
    cfun cos(x: float64) -> float64;
}
```

### 1.3 Declaring C Functions: cfun

Unlike standard `fun` functions, the `cfun` keyword is used for C functions. This signals the compiler to use the system C ABI without Yukino's internal mangling.

Both standard Yukino types (`int32`, `int64`, `float64`) and platform C types are used for parameter and return types:
- `c_char`, `c_schar`, `c_uchar`
- `c_short`, `c_ushort`
- `c_int`, `c_uint`
- `c_long`, `c_ulong`, `c_longlong`, `c_ulonglong`
- `c_size_t`, `c_ssize_t`
- `ptr<void>`, `ptr<c_char>`

### 1.4 C Attribute Syntax: |attr1, attr2|

C attributes are enclosed in vertical bars without extra spaces: `|attr1, attr2="value"|`. Multiple attributes can be listed inside a single pair of bars, separated by commas:

```yukino
// A single attribute
|cdecl|
cfun basic_func() -> void;

// Multiple attributes separated by commas
|cold, inline_never|
cfun slow_error_handler(code: int32) -> void;

// An attribute with a string value
|section="hot_code", inline_always|
cfun fast_worker(data: int32) -> int32;
```

### 1.5 Calling Conventions

Yukino supports all standard LLVM calling conventions:

| Attribute | Name | Purpose |
| :--- | :--- | :--- |
| `\|cdecl\|` | C Declaration (Default) | The platform's standard C calling convention |
| `\|stdcall\|` | Standard Call | Win32 API functions (callee cleans the stack) |
| `\|fastcall\|` | Fast Call | Passes the first arguments through CPU registers |
| `\|thiscall\|` | This Call | Calling C++ class methods on x86 |
| `\|sysv64\|` | System V AMD64 ABI | Explicit POSIX/Linux x86_64 convention |
| `\|win64\|` | Microsoft x64 ABI | Explicit Windows x86_64 convention |

Practical usage examples:

```yukino
externc {
    // Calling a WinAPI system function
    |stdcall|
    cfun GetSystemMetrics(n_index: int32) -> int32;

    // High-performance internal C worker
    |fastcall|
    cfun process_chunk(src: ptr<void>, len: int64) -> int32;
}
```

### 1.6 Linkage and Optimization C Attributes

| C Attribute | LLVM Equivalent | Purpose |
| :--- | :--- | :--- |
| `\|weak\|` | `Linkage::ExternalWeak` | Weak linkage (the symbol may be missing at link time) |
| `\|noreturn\|` | `Attribute::NoReturn` | The function never returns control (`exit`, `abort`) |
| `\|cold\|` | `Attribute::Cold` | Rarely executed code (moved out of the hot instruction cache) |
| `\|hot\|` | `Attribute::Hot` | Speed-critical function (inlining priority) |
| `\|inline_always\|` | `Attribute::AlwaysInline` | Forced inlining of the function body |
| `\|inline_never\|` | `Attribute::NoInline` | Prohibits inlining of the function |
| `\|returns_nonnull\|` | `Attribute::NonNull` | The returned pointer is guaranteed not to be `NULL` |
| `\|section="name"\|` | `.text.name` | Places the function in a specialized ELF/PE section |

Practical combinations:

```yukino
externc {
    // 1. Emergency system halt (cold path, never returns)
    |noreturn, cold|
    cfun fatal_abort(msg: ptr<c_char>) -> void;

    // 2. System memory allocator (always returns a non-null pointer)
    |cdecl, returns_nonnull|
    cfun custom_malloc(size: int64) -> ptr<void>;

    // 3. Optional plugin (weak linkage, can be overridden)
    |weak, cdecl|
    cfun optional_plugin_hook() -> int32;
}
```

### 1.7 Termination Syntax: the Semicolon (;)

Bodyless `cfun` signatures are declarations and are conventionally terminated with a semicolon `;`:

```yukino
// Correct:
cfun exit(code: int32) -> void;

// The semicolon is not required — this is also valid syntax
// (the next declaration or the block's `}` terminates the current one):
cfun abs(x: int32) -> int32
```

The parser accepts both forms equally, both at the top level and inside an `externc` block; the semicolon serves as a visual separator in declaration lists.

---

## 2. C Structures: cstruct and Memory Layout

Unlike standard Yukino `struct` structures, the `cstruct` keyword is used for C structures. They follow the size, field offsets, and ABI of the target platform: fields are laid out according to the natural alignment of their types, and the `packed` attribute disables padding.

### 2.1 Declaration and Field Access

```yukino
externc {
    cstruct NativePoint {
        x: int32,
        y: int32,
    }

    cstruct TimeSpec {
        tv_sec: int64,
        tv_nsec: int64,
    }
}

fun inspect_point() {
    set pt = NativePoint { x: 10, y: 20 }
    
    // When reading cstruct fields, the compiler requires an explicit type annotation:
    set pt_x: int32 = pt.x
    set pt_y: int32 = pt.y
}
```

### 2.2 The packed and align Attributes

For working with network protocols or hardware headers, you can pack a structure without padding using the `packed` attribute:

```yukino
externc {
    // Packed network header without padding bytes
    |packed|
    cstruct EthernetHeader {
        dst_mac: uint64,
        src_mac: uint64,
        ethertype: uint16,
    }
}
```

Without `packed`, fields are laid out according to the natural alignment of their types, as in C.

> [!WARNING]
> The `align="N"` attribute (the value is a string or an integer) is accepted by the parser and stored in the structure registry, but is currently **not applied by the code generator**: the actual alignment is determined by the natural field layout (or by `packed`). Do not rely on `align` when reproducing structures with non-standard alignment.

---

## 3. Exporting Yukino Functions to the C ABI

Yukino lets you write libraries that are called from C, C++, Python (`ctypes`), Rust, or C# (`P/Invoke`). For this, the function is declared with the `cfun` keyword.

> [!IMPORTANT]
> At the top level of a module, C attributes `|...|` **are always placed before the `public` keyword**:

```yukino
// 1. Exporting a function with the cdecl convention and the hot hint
|cdecl, hot|
public cfun yukino_add(a: int32, b: int32) -> int32 {
    return a + b
}

struct HostExitError {
    message: string
    code: int32
}

// 2. Exporting an emergency exit function with the noreturn and cold hints
|cold, noreturn|
public cfun yukino_panic_exit(code: int32) -> void {
    panic(HostExitError {
        message: "fatal exit requested by host",
        code: code,
    })
}
```

The compiled library exports these functions with clean C names (unmangled symbols).

---

## 4. Raw Pointers: ptr\<T\> and the ptr_of Operator

Yukino supports raw typed pointers `ptr<T>`:

- `ptr<c_char>`: a raw pointer to a C string.
- `ptr<void>`: a generic opaque pointer (`void*`).
- `ptr<int32>`: a pointer to a specific data type.

### The ptr_of Operator

The `ptr_of(variable)` operator returns a raw pointer to a local variable:

```yukino
set val = 123
set raw_address = ptr_of(val) // ptr<int64>: an unsuffixed integer literal defaults to int64
```

### Passing strings to C functions

`cast` between `string` and `ptr<T>` **is not a C-string conversion**: it parses the string's text as a hexadecimal address (or formats a pointer into a `0x...` string). To pass strings to C functions, use the `.as_ptr()` method — string literal buffers are null-terminated:

```yukino
externc {
    |cdecl|
    cfun write(fd: int32, buf: ptr<uint8>, count: int64) -> int64;
}

fun main() {
    set greeting = "Hello, C!\n"
    write(1, greeting.as_ptr(), greeting.len())
}
```

If needed, `ptr<uint8>` can be renamed to `ptr<char>` via `cast` (conversion between pointer types generates no code). See the [section 5 of the strings document](8-strings.md#5-interop-with-c-strings) for details.

---

## 5. Integrating C Files into the Build Process

To build projects containing both Yukino and C code, the `yuki.toml` file is used:

### Step 1: Compiling the C Code

Compile your `.c` file into a `.o` object file with the `gcc` or `clang` compiler:

```sh
gcc -c src/native_helper.c -o src/native_helper.o
```

### Step 2: Registering It in the yuki.toml Manifest

Add the compiled object file to the `[libs.static]` section:

```toml
[libs.static]
objects = ["src/native_helper.o"]
system = ["m", "pthread"]
```

### Step 3: Declaring and Calling It from Yukino Code

```yukino
externc {
    cfun custom_c_multiply(a: int32, b: int32) -> int32;
}

fun main() {
    set res = custom_c_multiply(6, 7) // 42
}
```

The `yukino run` command automatically links the `native_helper.o` object file and the system libraries into the final binary.

---

## 6. Comprehensive Example: Different Types in C and Yukino

Below is a complete example of interoperability between a C codebase and the Yukino language, covering primitives, floating-point numbers, boolean flags, C strings, structures with the C ABI field layout (`cstruct`), output parameters passed by pointer (`ptr<T>`), and callbacks (`cfun` callbacks).

### C Source Code (sensor_native.h and sensor_native.c)

```c
// sensor_native.h
#include <stdint.h>
#include <stdbool.h>

typedef struct {
    int32_t sensor_id;
    double temperature;
    float humidity;
    bool is_active;
    const char* label;
} SensorReport;

typedef int32_t (*FilterCallback)(int32_t value, int32_t threshold);

// Process sensor data: takes scalars, a string, and a structure
double process_sensor(
    const SensorReport* report,
    double calibration_offset,
    int32_t* out_status_code
);

// Apply a callback to an array of integers
int32_t filter_samples(
    const int32_t* samples,
    int32_t count,
    int32_t threshold,
    FilterCallback callback
);
```

```c
// sensor_native.c
#include "sensor_native.h"
#include <string.h>

double process_sensor(
    const SensorReport* report,
    double calibration_offset,
    int32_t* out_status_code
) {
    if (!report || !report->is_active) {
        if (out_status_code) *out_status_code = -1;
        return 0.0;
    }
    if (out_status_code) *out_status_code = 0;
    return report->temperature + calibration_offset;
}

int32_t filter_samples(
    const int32_t* samples,
    int32_t count,
    int32_t threshold,
    FilterCallback callback
) {
    int32_t passed = 0;
    for (int32_t i = 0; i < count; i++) {
        if (callback(samples[i], threshold) > 0) {
            passed++;
        }
    }
    return passed;
}
```

### Yukino Code (main.ykn)

```yukino
// 1. Description of the external data structure with the C ABI field layout
externc {
    cstruct SensorReport {
        sensor_id: int32,
        temperature: float64,
        humidity: float32,
        is_active: bool,
        label: ptr<char>,
    };

    // 2. Declarations of external functions with various data types
    |cdecl|
    cfun process_sensor(
        report: ptr<SensorReport>,
        calibration_offset: float64,
        out_status_code: ptr<int32>
    ) -> float64;

    |cdecl|
    cfun filter_samples(
        samples: ptr<int32>,
        count: int32,
        threshold: int32,
        callback: cfun(int32, int32) -> int32
    ) -> int32;
}

// 3. A callback function written in Yukino and exported for C
|cdecl|
public cfun threshold_filter(value: int32, threshold: int32) -> int32 {
    if value >= threshold {
        return 1
    }
    return 0
}

fun main() {
    // Preparing a C string: the literal is terminated with \0 in memory,
    // and the ptr<uint8> pointer is renamed to ptr<char> without any code
    set label_text = "Living Room Sensor\0"
    set c_label = cast(label_text.as_ptr(), "ptr<char>")

    // Creating the C structure
    set report = SensorReport {
        sensor_id: 101,
        temperature: 23.5,
        humidity: 48.0,
        is_active: true,
        label: c_label,
    }

    // Status output parameter via pointer
    set status_code: int32 = 0

    // Calling the C function, passing a pointer to the structure and an output pointer
    set calibrated_temp = process_sensor(
        ptr_of(report),
        -0.5,
        ptr_of(status_code)
    )

    if status_code == 0 {
        // Successful processing
    }

    // Passing the array and the callback to the C function
    set samples: int32[] = [12, 45, 68, 90, 22]
    set passed_count = filter_samples(
        samples.as_ptr(),
        cast(samples.len(), "int32"),
        40,
        threshold_filter
    )
    // passed_count == 3 (45, 68, 90)
}
```

---

[← Back: The define Macro System](10-macros.md) | [Next: Compiler Intrinsics →](12-intrinsics.md)
