# Module std.sharedc

The `std.sharedc` (Shared C) module provides a cross-platform interface for loading dynamic shared libraries (`.so` on Linux/Unix, `.dll` on Windows, `.dylib` on macOS), resolving exported C symbols into pointers, and calling native C functions via `cfun` pointer casts. The module supports standard POSIX dynamic linking flags (`RTLD_NOW`, `RTLD_LAZY`, `RTLD_GLOBAL`, `RTLD_LOCAL`), process self-namespace resolution, safe descriptor closing, and platform-specific library filename formatting.

---

## 1. Import: load std.sharedc

Importing the entire package:

```yukino
load std.sharedc
```

When imported, the following exported entities are available:

- **Constants:** `RTLD_LAZY`, `RTLD_NOW`, `RTLD_LOCAL`, `RTLD_GLOBAL`.
- **Error Structs:** `SharedError`.
- **Classes:** `SharedLibrary`.
- **Functions:** `open`, `open_flags`, `try_open`, `try_open_flags`, `open_self`, `open_self_flags`, `filename`, `last_error`.

All listed entities are also automatically available when loading the entire standard library: `load std`.

---

## 2. Constants

The module declares standard symbol resolution and binding flags:

```yukino
public static RTLD_LAZY: int32 = 1
public static RTLD_NOW: int32 = 2
public static RTLD_LOCAL: int32 = 4
public static RTLD_GLOBAL: int32 = 8
```

- `RTLD_LAZY` — Lazy binding; symbols are resolved only as the code referencing them is executed.
- `RTLD_NOW` — Immediate binding; all undefined symbols are resolved when the library is loaded (default).
- `RTLD_LOCAL` — Symbols from this library are not made available to resolve references in subsequently loaded libraries.
- `RTLD_GLOBAL` — Symbols defined by this library will be made available for symbol resolution of subsequently loaded libraries.

On Windows, these flags are ignored: loading is performed via `LoadLibraryW`, which does not provide binding mode control.

---

## 3. Interfaces

Module `std.sharedc` does not declare any interfaces.

---

## 4. Data Structures and Panic Errors

### 4.1 SharedError

Error structure used when raising a panic during library loading or symbol resolution:

```yukino
public struct SharedError {
    message: string
    code: int32
}
```

The `message` field contains a detailed system error description returned by the OS (`dlerror` on Unix or `FormatMessageW` on Windows). The `code` field categorizes the failure:
- `1` — Failed to open the shared library file at the specified path or failed to open the current process namespace.
- `2` — The requested symbol was not found in the shared library export table.
- `3` — Attempted to look up a symbol on a library instance that has already been closed.

---

## 5. Classes

### 5.1 SharedLibrary

The `SharedLibrary` class encapsulates an active native handle to an opened shared library or the process namespace.

```yukino
public class SharedLibrary {
    public handle: ptr<void>
    public path: string
    public is_closed: bool

    public fun init(handle: ptr<void>, path: string)
    public fun symbol(name: string) -> ptr<void>
    public fun symbol_or_panic(name: string) -> ptr<void>
    public fun close() -> bool
}
```

The `handle` field contains the raw OS pointer to the loaded module. The `path` field stores the original file path (or `"<self>"` for the process namespace). The `is_closed` field tracks whether the descriptor has already been released.

#### 5.1.1 symbol

Resolves and returns a raw pointer to an exported symbol by name. Returns `null` if the symbol cannot be found or if the library has already been closed.

```yukino
public fun symbol(name: string) -> ptr<void>
```

Returns a pointer of type `ptr<void>`. The return value is nullable (can be `null`).

The function never panics.

```yukino
load std.sharedc: open
load std.io: println

fun main() {
    set lib = open("./libmath.so")
    set sym = lib.symbol("calculate")
    if sym != null {
        set calc = cast(sym, "cfun(int32) -> int32")
        println(calc(10))
    }
    lib.close()
}
```

#### 5.1.2 symbol_or_panic

Resolves and returns a raw pointer to an exported symbol by name. If the symbol does not exist or the library is already closed, raises a panic with the underlying OS error message.

```yukino
public fun symbol_or_panic(name: string) -> ptr<void>
```

Returns a pointer of type `ptr<void>`. The return value is non-nullable (guaranteed not to be `null`).

Panics with `SharedError` (code `3` if the library is closed, code `2` if the symbol was not found).

```yukino
load std.sharedc: open, SharedError
load std.io: println

fun main() {
    set lib = open("./libmath.so")
    sus {
        set sym = lib.symbol_or_panic("add_numbers")
        set add = cast(sym, "cfun(int32, int32) -> int32")
        println(add(5, 7))
    } catch err: SharedError {
        println("Symbol lookup failed: ${err.message}")
    }
    lib.close()
}
```

#### 5.1.3 close

Unloads the dynamic library and frees the associated OS handle. Safe to call multiple times on the same object (subsequent calls return `true` without error).

```yukino
public fun close() -> bool
```

Returns a boolean value of type `bool`: `true` on successful closure (or if already closed), or `false` on OS failure. The return value is non-nullable.

The function never panics.

```yukino
load std.sharedc: open
load std.io: println

fun main() {
    set lib = open("./libmath.so")
    set ok = lib.close()
    println("Library closed: ${ok}")
}
```

---

## 6. Function Reference

### 6.1 open

Loads a dynamic shared library from the given path using the default `RTLD_NOW` binding flag. Panics if the library cannot be loaded.

```yukino
public fun open(path: string) -> SharedLibrary
```

Returns an instance of `SharedLibrary`. The return value is non-nullable.

Panics with `SharedError` (code `1`) if the library cannot be found or loaded.

```yukino
load std.sharedc: open
load std.io: println

fun main() {
    set lib = open("./libtest.so")
    println("Loaded library: ${lib.path}")
    lib.close()
}
```

### 6.2 open_flags

Loads a dynamic shared library from the given path using explicit custom binding flags `flags` (e.g., `RTLD_LAZY | RTLD_GLOBAL`).

```yukino
public fun open_flags(path: string, flags: int32) -> SharedLibrary
```

Returns an instance of `SharedLibrary`. The return value is non-nullable.

Panics with `SharedError` (code `1`) if loading fails.

```yukino
load std.sharedc: open_flags, RTLD_LAZY, RTLD_GLOBAL
load std.io: println

fun main() {
    set lib = open_flags("./libplugin.so", RTLD_LAZY | RTLD_GLOBAL)
    println("Plugin loaded: ${lib.path}")
    lib.close()
}
```

### 6.3 try_open

Attempts to load a dynamic shared library from the given path using `RTLD_NOW`. Unlike `open`, returns `null` instead of panicking on failure.

```yukino
public fun try_open(path: string) -> SharedLibrary?
```

Returns an instance of `SharedLibrary` on success, or `null` if loading fails. The return value is nullable.

The function never panics.

```yukino
load std.sharedc: try_open
load std.io: println

fun main() {
    set lib = try_open("./optional_accelerator.so")
    if lib == null {
        println("Optional module not found, using fallback")
    } else {
        println("Accelerator attached")
        lib.close()
    }
}
```

### 6.4 try_open_flags

Attempts to load a dynamic shared library using explicit binding flags `flags`. Returns `null` on failure without panicking.

```yukino
public fun try_open_flags(path: string, flags: int32) -> SharedLibrary?
```

Returns an instance of `SharedLibrary` on success, or `null` on failure. The return value is nullable.

The function never panics.

```yukino
load std.sharedc: try_open_flags, RTLD_LAZY
load std.io: println

fun main() {
    set lib = try_open_flags("./optional_module.so", RTLD_LAZY)
    if lib != null {
        println("Module opened successfully")
        lib.close()
    }
}
```

### 6.5 open_self

Opens a handle to the global symbol namespace of the current running process using `RTLD_NOW`. Allows looking up symbols exported by the main executable or linked C runtime (such as `puts`, `malloc`, `getpid`).

```yukino
public fun open_self() -> SharedLibrary
```

Returns an instance of `SharedLibrary` with path `"<self>"`. The return value is non-nullable.

Panics with `SharedError` (code `1`) if opening fails.

```yukino
load std.sharedc: open_self
load std.io: println

fun main() {
    set proc = open_self()
    set puts_ptr = proc.symbol("puts")
    if puts_ptr != null {
        println("Symbol puts found in process namespace")
    }
    proc.close()
}
```

### 6.6 open_self_flags

Opens a handle to the global symbol namespace of the current process with explicit binding flags `flags`.

```yukino
public fun open_self_flags(flags: int32) -> SharedLibrary
```

Returns an instance of `SharedLibrary`. The return value is non-nullable.

Panics with `SharedError` (code `1`) if opening fails.

```yukino
load std.sharedc: open_self_flags, RTLD_GLOBAL
load std.io: println

fun main() {
    set proc = open_self_flags(RTLD_GLOBAL)
    println("Process namespace opened with RTLD_GLOBAL")
    proc.close()
}
```

### 6.7 filename

Formats a library stem `name` into the platform-standard shared library filename. For example, `"crypto"` becomes `"libcrypto.so"` on Linux/Unix, `"crypto.dll"` on Windows, and `"libcrypto.dylib"` on macOS. The extension is always appended: existing suffixes (`.so`, `.dll`, `.dylib`) in the input string are not detected, so `"mylib.so"` produces `"libmylib.so.so"` on Linux.

```yukino
public fun filename(name: string) -> string
```

Returns a formatted filename string of type `string`. The return value is non-nullable.

The function never panics.

```yukino
load std.sharedc: filename
load std.io: println

fun main() {
    println(filename("sqlite3")) // "libsqlite3.so" on Linux, "sqlite3.dll" on Windows
    println(filename("mylib.so")) // "libmylib.so.so" on Linux, "mylib.so.dll" on Windows
}
```

### 6.8 last_error

Returns the message of the last dynamic loader error as a `string`, or `null` if no error has been recorded. The message is captured at the moment an operation fails (from `dlerror()` on Unix, or from the `GetLastError()` code formatted via `FormatMessageW` on Windows) and is stored until the next loading or symbol lookup operation.

```yukino
public fun last_error() -> string?
```

Returns an error description string of type `string` or `null`. The return value is nullable.

The function never panics.

```yukino
load std.sharedc: last_error, try_open
load std.io: println

fun main() {
    set lib = try_open("./missing.so")
    if lib == null {
        set err = last_error()
        if err != null {
            println("Failure reason: ${err}")
        }
    }
}
```

---

[← Previous: Module std.numf](18-numf.md) | [Next: Module std.json →](20-json.md)
